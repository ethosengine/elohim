"""Rule 4 counts BUILDS (process trees), not heavy processes, against
io.max_concurrent_heavy, using a fake /proc.

Run: python3 -m unittest discover -s .claude/hooks/tests -p 'test_cargo_disk_guard*.py'
"""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock


HOOK = Path(__file__).resolve().parents[1] / 'cargo-disk-guard.py'
spec = importlib.util.spec_from_file_location('cargo_disk_guard_seats', HOOK)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


class SeatsTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.proc = Path(tmp.name)
        self.limit = 2
        for patch in (
            mock.patch.object(guard, 'PROC_DIR', str(self.proc)),
            mock.patch.object(guard, 'disk_pct', return_value=50),
            mock.patch.object(guard, 'read_policy', return_value=(85, 75)),
            mock.patch.object(guard, 'host_psi_io', return_value=None),
            mock.patch.object(guard, 'read_io_policy', side_effect=lambda: {
                **guard.IO_DEFAULTS, 'max_concurrent_heavy': self.limit}),
            mock.patch.dict(os.environ, {'CARGO_BUILD_JOBS': '1'}),
            # PROC_DIR isolates what the hook READS; a signal would still reach a real pid.
            mock.patch.object(guard.os, 'kill'),
        ):
            patch.start()
            self.addCleanup(patch.stop)

    def process(self, pid, argv, parent=1, marked=False, comm='test'):
        d = self.proc / str(pid)
        d.mkdir(exist_ok=True)
        (d / 'cmdline').write_bytes(b'\0'.join(a.encode() for a in argv) + b'\0')
        (d / 'environ').write_bytes(b'ELOHIM_LSP_FLYCHECK=1\0' if marked else b'PATH=/bin\0')
        (d / 'stat').write_text(f'{pid} ({comm}) S {parent} ' + '0 ' * 17 + f'{pid * 100}\n')

    def gate(self, base):
        """gate-runner -> bash -> cargo test -> six rustc + rust-lld."""
        self.process(base, ['node', '/repo/genesis/orchestrator/gate-runner.mjs', 'x'])
        self.process(base + 1, ['bash', 'run-local-gate.sh'], base)
        self.process(base + 2, ['cargo', 'test'], base + 1)
        for i in range(6):
            self.process(base + 3 + i, ['rustc', '--crate-name', f'c{i}'], base + 2)
        self.process(base + 9, ['rust-lld'], base + 3)

    def run_hook(self, command='just gate eprfs'):
        out = io.StringIO()
        data = {'tool_name': 'Bash', 'cwd': '/scratch', 'tool_input': {'command': command}}
        with mock.patch.object(guard.sys, 'stdin', io.StringIO(json.dumps(data))), contextlib.redirect_stdout(out):
            try:
                guard.main()
            except SystemExit as e:
                self.assertEqual(e.code, 0)
        lines = out.getvalue().splitlines()
        return json.loads(lines[0])['hookSpecificOutput'] if lines else {}

    def test_one_gate_is_one_build_and_second_is_admitted_at_limit_2(self):
        self.gate(1000)
        self.assertGreater(len(guard.running_heavy()), 5)
        live = [(p, c) for p, c, y in guard.running_heavy() if not y]
        self.assertEqual(guard.group_builds(live), [(1002, 'cargo test', 1000, 'gate-runner.mjs')])
        self.assertEqual(self.run_hook(), {})

    def test_two_builds_at_limit_2_denied_naming_both_roots(self):
        self.gate(1000)
        self.gate(2000)
        r = self.run_hook()
        self.assertEqual(r['permissionDecision'], 'deny')
        reason = r['permissionDecisionReason']
        self.assertIn('2 of 2', reason)
        self.assertIn('pid 1000', reason)
        self.assertIn('pid 2000', reason)
        self.assertIn('fewer than 2', reason)
        self.assertIn('[ -d /proc/1000 ] && [ -d /proc/2000 ]', reason)

    def test_limit_1_with_one_build_denied(self):
        self.limit = 1
        self.gate(1000)
        r = self.run_hook()
        self.assertEqual(r['permissionDecision'], 'deny')
        self.assertIn('1 of 1', r['permissionDecisionReason'])

    def test_yieldable_is_not_a_build(self):
        self.process(100, ['bash', 'lsp-flycheck'])
        self.process(101, ['cargo', 'check'], 100, marked=True)
        self.process(102, ['rustc'], 101, marked=True)
        self.limit = 1
        r = self.run_hook()
        self.assertNotIn('permissionDecision', r)

    def test_orphan_rustc_is_its_own_build(self):
        self.gate(1000)
        self.process(500, ['rustc'], parent=1)
        live = [(p, c) for p, c, y in guard.running_heavy() if not y]
        self.assertEqual([b[0] for b in guard.group_builds(live)], [500, 1002])
        self.assertEqual(self.run_hook()['permissionDecision'], 'deny')

    def test_vanished_parent_malformed_stat_odd_comm_and_loop_do_not_raise(self):
        self.process(10, ['rustc'], parent=999)  # parent absent
        self.process(11, ['rustc'], parent=1)
        (self.proc / '11' / 'stat').write_text('garbage')
        self.process(12, ['rustc'], parent=1, comm='a b) (c')
        self.process(13, ['cargo', 'build'], parent=14)  # loop 13 <-> 14
        self.process(14, ['cargo', 'build'], parent=13)
        live = [(p, c) for p, c, y in guard.running_heavy() if not y]
        builds = {b[0] for b in guard.group_builds(live)}
        # a loop is impossible in a real /proc; it must not raise, hang, or hide a build
        self.assertEqual(builds, {10, 11, 12, 13, 14})

    def test_comm_with_parens_still_finds_the_parent(self):
        self.process(20, ['cargo', 'build'])
        self.process(21, ['rustc'], parent=20, comm='x) S 1 (y')
        live = [(p, c) for p, c, y in guard.running_heavy() if not y]
        self.assertEqual([b[0] for b in guard.group_builds(live)], [20])

    def test_deep_chain_is_bounded(self):
        self.process(30, ['cargo', 'build'])
        prev = 30
        for pid in range(31, 31 + guard.MAX_ANCESTRY_DEPTH + 5):
            self.process(pid, ['sh'], prev)
            prev = pid
        self.process(900, ['rustc'], prev)
        live = [(p, c) for p, c, y in guard.running_heavy() if not y]
        self.assertEqual(len(guard.group_builds(live)), 2)

    def builds(self):
        live = [(p, c) for p, c, y in guard.running_heavy() if not y]
        return guard.group_builds(live)

    def test_two_cargos_side_by_side_under_one_gate_runner_are_two_builds(self):
        # Ancestry alone does not make concurrent compiles one seat.
        self.process(3000, ['node', '/repo/genesis/orchestrator/gate-runner.mjs', 'x'])
        self.process(3001, ['cargo', 'build'], 3000)
        self.process(3002, ['rustc'], 3001)
        self.process(3003, ['cargo', 'test'], 3000)
        self.process(3004, ['rustc'], 3003)
        self.assertEqual([(b[0], b[2]) for b in self.builds()], [(3001, 3000), (3003, 3000)])
        r = self.run_hook()
        self.assertEqual(r['permissionDecision'], 'deny')
        self.assertIn('2 of 2', r['permissionDecisionReason'])

    def test_a_cargo_test_whose_tests_start_two_cargos_is_two_builds(self):
        self.process(3100, ['cargo', 'test'])
        self.process(3101, ['/target/debug/deps/artifact-abc'], 3100)
        self.process(3102, ['cargo', 'build'], 3101)
        self.process(3103, ['cargo', 'build'], 3101)
        self.assertEqual([b[0] for b in self.builds()], [3102, 3103])

    def test_a_nested_cargo_that_runs_alone_is_one_build(self):
        self.process(3200, ['cargo', 'test'])
        self.process(3201, ['/target/debug/deps/artifact-abc'], 3200)
        self.process(3202, ['cargo', 'build'], 3201)
        self.process(3203, ['rustc'], 3202)
        self.assertEqual(self.builds(), [(3202, 'cargo build', 3200, 'cargo test')])

    def test_two_gate_runners_under_one_shell_are_two_builds(self):
        self.process(3300, ['bash', 'pre-push.bash'])
        for base in (3310, 3320):
            self.process(base, ['node', '/repo/genesis/orchestrator/gate-runner.mjs', 'x'], 3300)
            self.process(base + 1, ['cargo', 'test'], base)
        self.assertEqual([b[0] for b in self.builds()], [3311, 3321])

    def test_a_gate_runner_between_cargo_steps_still_holds_its_seat(self):
        self.process(3400, ['node', '/repo/genesis/orchestrator/gate-runner.mjs', 'x'])
        self.process(3401, ['bash', 'gate-steps.sh'], 3400)
        self.assertEqual(self.builds(), [(3400, 'gate-runner.mjs', 3400, 'gate-runner.mjs')])
        self.limit = 1
        self.assertEqual(self.run_hook()['permissionDecision'], 'deny')

    def test_a_zombie_and_a_nonpositive_parent_change_nothing(self):
        self.process(3500, ['cargo', 'build'], parent=0)
        self.process(3501, ['rustc'], parent=-7)
        d = self.proc / '3502'
        d.mkdir()
        (d / 'cmdline').write_bytes(b'')  # a zombie has an empty cmdline and is not listed
        (d / 'environ').write_bytes(b'')
        (d / 'stat').write_text('3502 (rustc) Z 3500 ' + '0 ' * 17 + '1\n')
        self.assertEqual([b[0] for b in self.builds()], [3500, 3501])

    def test_an_outer_cargo_still_compiling_beside_its_child_cargo_is_two_builds(self):
        self.process(3600, ['cargo', 'build'])
        self.process(3601, ['rustc', '--crate-name', 'outer'], 3600)
        self.process(3602, ['/target/debug/build/x/build-script-build'], 3600)
        self.process(3603, ['cargo', 'build'], 3602)
        self.process(3604, ['rustc', '--crate-name', 'inner'], 3603)
        self.assertEqual([(b[0], b[2]) for b in self.builds()], [(3600, 3600), (3603, 3600)])
        self.assertEqual(self.run_hook()['permissionDecision'], 'deny')

    def test_one_gate_runner_running_projects_in_turn_is_one_build(self):
        # `just gate a b`: one runner, one cargo at a time.
        self.process(3700, ['node', '/repo/genesis/orchestrator/gate-runner.mjs', 'a', 'b'])
        self.process(3701, ['bash', 'run-local-gate.sh'], 3700)
        self.process(3702, ['cargo', 'clippy'], 3701)
        self.process(3703, ['rustc'], 3702)
        self.assertEqual([(b[0], b[2]) for b in self.builds()], [(3702, 3700)])
        self.assertEqual(self.run_hook(), {})

    def test_a_ring_of_drivers_longer_than_the_depth_bound_hides_none(self):
        # Impossible in a real /proc; the count must still never fall to zero.
        n = guard.MAX_ANCESTRY_DEPTH + 2
        for i in range(n):
            self.process(4000 + i, ['cargo', 'build'], parent=4000 + (i + 1) % n)
        self.assertEqual(len(self.builds()), n)

    def test_a_cargo_with_options_before_its_verb_is_listed(self):
        self.process(3800, ['cargo', '--color=always', 'build'])
        self.process(3801, ['cargo', '+1.100.0', '--color', 'always', '-Z', 'unstable-options', 'test'])
        self.process(3802, ['cargo', '--config', 'build', 'metadata'])  # `build` here is a value
        self.process(3803, ['cargo', '--version'])
        self.assertEqual(sorted(p for p, _, _ in guard.running_heavy()), [3800, 3801])

    def test_two_cargos_of_an_unknown_subcommand_under_one_gate_runner_are_two_builds(self):
        # running_heavy does not list them; their compilers still give each a seat.
        self.process(3900, ['node', '/repo/genesis/orchestrator/gate-runner.mjs', 'x'])
        for base in (3910, 3920):
            self.process(base, ['cargo', 'llvm-cov'], 3900)
            self.process(base + 1, ['rustc'], base)
        self.assertEqual(sorted(p for p, _, _ in guard.running_heavy()), [3900, 3911, 3921])
        self.assertEqual([(b[0], b[1], b[2]) for b in self.builds()],
                         [(3910, 'cargo llvm-cov', 3900), (3920, 'cargo llvm-cov', 3900)])
        self.assertEqual(self.run_hook()['permissionDecision'], 'deny')

    def test_one_cargo_of_an_unknown_subcommand_under_a_gate_runner_is_one_build(self):
        self.process(3950, ['node', '/repo/genesis/orchestrator/gate-runner.mjs', 'x'])
        self.process(3951, ['cargo', 'llvm-cov'], 3950)
        self.process(3952, ['rustc'], 3951)
        self.assertEqual([(b[0], b[2]) for b in self.builds()], [(3951, 3950)])
        self.assertEqual(self.run_hook(), {})

    def test_two_doc_builds_by_alias_under_one_gate_runner_are_two_builds(self):
        self.process(4100, ['node', '/repo/genesis/orchestrator/gate-runner.mjs', 'x'])
        for base in (4110, 4120):
            self.process(base, ['cargo', 'd'], 4100)
            self.process(base + 1, ['rustdoc', '--crate-name', 'x'], base)
        self.assertEqual([b[0] for b in self.builds()], [4110, 4120])

    def test_a_cargo_clippy_still_linting_beside_its_child_cargo_is_two_builds(self):
        self.process(4200, ['cargo', 'clippy'])
        self.process(4201, ['clippy-driver', '--crate-name', 'outer'], 4200)
        self.process(4202, ['/target/debug/build/x/build-script-build'], 4200)
        self.process(4203, ['cargo', 'build'], 4202)
        self.process(4204, ['cc1plus'], 4203)
        self.assertEqual([b[0] for b in self.builds()], [4200, 4203])

    def test_a_grouping_failure_counts_every_process_and_refuses(self):
        # An error inside the grouping must refuse work, never admit it: one gate
        # then reads as many builds, which is the rule this change replaced.
        self.gate(1000)
        live = [(p, c) for p, c, y in guard.running_heavy() if not y]
        with mock.patch.object(guard, 'proc_parent', side_effect=RuntimeError('unreadable')):
            self.assertEqual(guard.group_builds(live), sorted((p, c, p, c) for p, c in set(live)))
            self.assertEqual(self.run_hook()['permissionDecision'], 'deny')

    def test_running_heavy_rows_unchanged(self):
        self.process(200, ['/usr/bin/cargo', '+stable', 'build', '-p', 'x', 'y'])
        self.process(201, ['rustc'], 200, marked=True)
        self.process(202, ['bash'], 200)
        self.assertEqual(sorted(guard.running_heavy()),
                         [(200, '/usr/bin/cargo +stable build -p', False), (201, 'rustc', True)])


if __name__ == '__main__':
    unittest.main()
