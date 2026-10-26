"""Gate priority over flycheck, using fake /proc and no real signals.

Run: python3 -m unittest discover -s .claude/hooks/tests -p 'test_cargo_disk_guard*.py'
"""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
import tempfile
import unittest
from unittest import mock


HOOK = Path(__file__).resolve().parents[1] / 'cargo-disk-guard.py'
spec = importlib.util.spec_from_file_location('cargo_disk_guard', HOOK)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


class YieldTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.proc = Path(tmp.name)
        for patch in (
            mock.patch.object(guard, 'PROC_DIR', str(self.proc)),
            mock.patch.object(guard, 'disk_pct', return_value=50),
            mock.patch.object(guard, 'read_policy', return_value=(85, 75)),
            mock.patch.object(guard, 'host_psi_io', return_value=None),
            mock.patch.object(guard, 'read_io_policy', return_value=dict(guard.IO_DEFAULTS)),
            mock.patch.dict(os.environ, {'CARGO_BUILD_JOBS': '1'}),
        ):
            patch.start()
            self.addCleanup(patch.stop)
        patch = mock.patch.object(guard.os, 'kill', side_effect=self.terminate)
        self.kill = patch.start()
        self.addCleanup(patch.stop)

    def process(self, pid, argv, parent=1, marked=False, state='S'):
        directory = self.proc / str(pid)
        directory.mkdir(exist_ok=True)
        (directory / 'cmdline').write_bytes(b'\0'.join(a.encode() for a in argv) + b'\0')
        (directory / 'environ').write_bytes(b'ELOHIM_LSP_FLYCHECK=1\0' if marked else b'PATH=/bin\0')
        # Fields 3 (state), 4 (ppid), ... 22 (start time), with a tricky comm.
        (directory / 'stat').write_text(
            f'{pid} (test ) process) {state} {parent} ' + '0 ' * 17 + f'{pid * 100}\n')

    def flycheck(self, base=100):
        self.process(base, ['rust-analyzer'])
        self.process(base + 1, ['bash', '/repo/genesis/agentic/bin/lsp-flycheck', 'rust', 'src/lib.rs'], base)
        self.process(base + 2, ['/toolchain/bin/cargo', 'check'], base + 1, marked=True)
        self.process(base + 3, ['rustc', '--crate-name', 'demo'], base + 2, marked=True)

    def terminate(self, pid, sig):
        self.assertEqual(sig, signal.SIGTERM)
        # A zombie is exited, even though its parent has not reaped it yet.
        path = self.proc / str(pid) / 'stat'
        path.write_text(path.read_text().replace(') S ', ') Z '))

    def run_hook(self, command='just gate eprfs'):
        output = io.StringIO()
        data = {'tool_name': 'Bash', 'cwd': '/scratch', 'tool_input': {'command': command}}
        with mock.patch.object(guard.sys, 'stdin', io.StringIO(json.dumps(data))), contextlib.redirect_stdout(output):
            try:
                guard.main()
            except SystemExit as error:
                self.assertEqual(error.code, 0)
        lines = output.getvalue().splitlines()
        self.assertLessEqual(len(lines), 1, output.getvalue())
        return json.loads(lines[0])['hookSpecificOutput'] if lines else {}

    def test_yieldable_only_allows_cargo_and_just_gate_with_context(self):
        for command in ('cargo check -j1', 'just gate eprfs'):
            with self.subTest(command=command):
                self.flycheck()
                self.kill.reset_mock()
                result = self.run_hook(command)
                self.assertNotIn('permissionDecision', result)
                self.assertEqual(result['additionalContext'],
                                 'yielded: stopped lsp flycheck pid 101 for this build')
                self.kill.assert_called_once_with(101, signal.SIGTERM)

    def test_non_yieldable_at_max_denies(self):
        self.process(200, ['cargo', 'build'])
        result = self.run_hook()
        self.assertEqual(result['permissionDecision'], 'deny')
        self.assertIn('pid 200', result['permissionDecisionReason'])
        self.kill.assert_not_called()

    def test_node_gate_runner_is_heavy(self):
        for node in ('/usr/bin/node', '/usr/bin/nodejs'):
            for options in ([], ['--some-flag', '--another-flag']):
                for marked in (False, True):
                    with self.subTest(node=node, options=options, marked=marked):
                        self.process(200, [node, *options, '/repo/genesis/orchestrator/gate-runner.mjs', 'eprfs'], marked=marked)
                        self.assertEqual(guard.running_heavy(), [(200, 'gate-runner.mjs', marked)])

    def test_bare_gate_runner_is_heavy(self):
        for head in ('gate-runner.mjs', '/repo/genesis/orchestrator/gate-runner.mjs'):
            for marked in (False, True):
                with self.subTest(head=head, marked=marked):
                    self.process(200, [head, 'eprfs'], marked=marked)
                    self.assertEqual(guard.running_heavy(), [(200, 'gate-runner.mjs', marked)])

    def test_node_only_matches_first_non_option_argument(self):
        for argv in (
            ['node', '--some-flag', 'other.mjs'],
            ['node', '--some-flag', 'other.mjs', '/repo/gate-runner.mjs'],
            ['node', '--some-flag'],
        ):
            with self.subTest(argv=argv):
                self.process(200, argv)
                self.assertEqual(guard.running_heavy(), [])

    def test_node_gate_runner_and_cargo_at_max_deny(self):
        self.process(200, ['node', '/repo/genesis/orchestrator/gate-runner.mjs', 'eprfs'])
        self.process(201, ['cargo', 'build'])
        with mock.patch.object(guard, 'read_io_policy', return_value={**guard.IO_DEFAULTS, 'max_concurrent_heavy': 2}):
            result = self.run_hook()
        self.assertEqual(result['permissionDecision'], 'deny')
        self.assertIn('ONE BUILD AT A TIME', result['permissionDecisionReason'])
        self.assertIn('200', result['permissionDecisionReason'])
        self.assertIn('201', result['permissionDecisionReason'])
        self.kill.assert_not_called()

    def test_mixed_counts_only_non_yieldable_and_signals_only_lsp_root(self):
        self.flycheck()
        self.process(200, ['cargo', 'build'])
        with mock.patch.object(guard, 'read_io_policy', return_value={**guard.IO_DEFAULTS, 'max_concurrent_heavy': 2}):
            result = self.run_hook()
        self.assertNotIn('permissionDecision', result)
        self.kill.assert_called_once_with(101, signal.SIGTERM)

    def test_mixed_at_max_denies_without_cancelling_an_edit_for_a_blocked_gate(self):
        self.flycheck()
        self.process(200, ['rustc'])
        self.assertEqual(self.run_hook()['permissionDecision'], 'deny')
        self.kill.assert_not_called()

    def test_highest_marked_ancestor_is_the_only_signal_target(self):
        self.process(301, ['bash', '/repo/genesis/agentic/bin/lsp-flycheck'])
        self.process(302, ['intermediate'], 301, marked=True)
        self.process(303, ['cargo', 'check'], 302, marked=True)
        self.run_hook()
        self.kill.assert_called_once_with(301, signal.SIGTERM)

    def test_unmarked_parent_with_flycheck_in_arguments_is_not_a_wrapper(self):
        self.process(301, ['bash', '-c', 'echo lsp-flycheck'])
        self.process(302, ['cargo', 'check'], 301, marked=True)
        self.run_hook()
        self.kill.assert_called_once_with(302, signal.SIGTERM)

    def test_two_flychecks_each_yield_once(self):
        self.flycheck(100)
        self.flycheck(300)
        result = self.run_hook()
        self.assertEqual(self.kill.call_args_list,
                         [mock.call(101, signal.SIGTERM), mock.call(301, signal.SIGTERM)])
        self.assertEqual(len(result['additionalContext'].splitlines()), 2)

    def test_missing_or_unreadable_environ_is_not_yieldable(self):
        self.flycheck()
        real_open = open

        def unreadable(path, *args, **kwargs):
            if str(path).endswith('/environ'):
                raise PermissionError('not ours')
            return real_open(path, *args, **kwargs)

        with mock.patch('builtins.open', side_effect=unreadable):
            self.assertEqual(self.run_hook()['permissionDecision'], 'deny')
        for pid in (102, 103):
            (self.proc / str(pid) / 'environ').unlink()
        self.assertEqual(self.run_hook()['permissionDecision'], 'deny')
        self.kill.assert_not_called()

    def test_marker_must_be_an_exact_environment_entry(self):
        self.process(200, ['cargo', 'test'])
        for env in (b'ELOHIM_LSP_FLYCHECK=10\0', b'OTHER=ELOHIM_LSP_FLYCHECK=1\0'):
            (self.proc / '200/environ').write_bytes(env)
            self.assertEqual(self.run_hook()['permissionDecision'], 'deny')
        self.kill.assert_not_called()

    def test_no_heavy_command_never_signals(self):
        self.flycheck()
        for command in ('cargo metadata', 'just --list', 'echo "cargo build"'):
            self.assertEqual(self.run_hook(command), {})
        self.kill.assert_not_called()

    def test_other_deny_rules_preserve_flychecks(self):
        self.flycheck()
        with mock.patch.object(guard, 'disk_pct', return_value=90):
            self.assertIn('DISK HARD CEILING', self.run_hook()['permissionDecisionReason'])
        with mock.patch.object(guard, 'host_psi_io', return_value={'full_avg10': 30}):
            self.assertIn('HOST I/O PRESSURE', self.run_hook()['permissionDecisionReason'])
        with mock.patch.dict(os.environ, {'CARGO_BUILD_JOBS': ''}):
            self.assertIn('JOBS CAP', self.run_hook()['permissionDecisionReason'])
        self.kill.assert_not_called()

    def test_soft_watermark_and_yield_share_one_context_response(self):
        self.flycheck()
        with mock.patch.object(guard, 'disk_pct', return_value=80):
            result = self.run_hook()
        self.assertIn('yielded: stopped lsp flycheck pid 101', result['additionalContext'])
        self.assertIn('DISK SOFT WATERMARK', result['additionalContext'])

    def test_wait_is_bounded_and_does_not_claim_a_still_running_process_stopped(self):
        self.flycheck()
        self.kill.side_effect = None
        now = [0.0]
        with mock.patch.object(guard.time, 'monotonic', side_effect=lambda: now[0]), \
                mock.patch.object(guard.time, 'sleep', side_effect=lambda delay: now.__setitem__(0, now[0] + delay)):
            result = self.run_hook()
        self.assertAlmostEqual(now[0], 3)
        self.assertNotIn('permissionDecision', result)
        self.assertIn('still stopping after 3 s', result['additionalContext'])
        self.assertNotIn('yielded: stopped', result['additionalContext'])

    def test_changed_identity_is_not_signalled(self):
        self.flycheck()

        def gone(pid):
            self.process(101, ['other-build'])
            return 101

        with mock.patch.object(guard, 'flycheck_root', side_effect=gone):
            self.assertEqual(self.run_hook(), {})
        self.kill.assert_not_called()


if __name__ == '__main__':
    unittest.main()
