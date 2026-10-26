"""Flycheck uses real berth admission and a stub cargo; no Rust compilation.

Run: python3 -m unittest genesis.agentic.lsp_flycheck_test
"""
import json
import os
from pathlib import Path
import selectors
import shutil
import signal
import subprocess
import tempfile
import time
import unittest


REPO = Path(__file__).resolve().parents[2]
BIN = REPO / 'genesis/agentic/bin'


class FlycheckTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        policy_dir = self.root / 'genesis/agentic'
        policy_dir.mkdir(parents=True)
        self.policy = json.loads((REPO / 'genesis/agentic/pool-policy.json').read_text())
        (policy_dir / 'pool-policy.json').write_text(json.dumps(self.policy))
        stubs = self.root / 'stubs'
        stubs.mkdir()
        epr = stubs / 'epr'
        epr.write_text('''#!/usr/bin/env python3
import json, os, sys
if sys.argv[1:] != ['session', 'id', '--json']:
    sys.exit(1)
session = None if os.environ.get('NO_SESSION') else 'flycheck-session'
print(json.dumps(dict(runtime='declared', session=session,
                     ownerPid=int(os.environ['TEST_OWNER_PID']), ownerStartTime='1',
                     source='declared' if session else 'none')))
''')
        cargo = stubs / 'cargo'
        cargo.write_text('''#!/usr/bin/env python3
import json, os, pathlib, subprocess, sys, time
child = subprocess.Popen(['sleep', '30']) if os.environ.get('CARGO_CHILD') else None
lease_file = pathlib.Path(os.environ['BERTH_DIR']) / 'leases.json'
row = dict(argv=sys.argv[1:], cwd=os.getcwd(), pid=os.getpid(), child=child.pid if child else None,
           env={k: os.environ.get(k) for k in ('RUSTUP_TOOLCHAIN', 'CARGO_BUILD_JOBS', 'CARGO_TARGET_DIR', 'RUSTFLAGS', 'ELOHIM_LSP_FLYCHECK')},
           leases=json.loads(lease_file.read_text()) if lease_file.exists() else {})
pathlib.Path(os.environ['CARGO_CAPTURE']).write_text(json.dumps(row))
print(json.dumps({'reason': 'build-finished', 'success': True}), flush=True)
time.sleep(float(os.environ.get('CARGO_SLEEP', '0')))
sys.exit(int(os.environ.get('CARGO_EXIT', '0')))
''')
        epr.chmod(0o755)
        cargo.chmod(0o755)
        self.capture = self.root / 'cargo.json'
        self.proc_dir = self.root / 'proc'
        self.proc_dir.mkdir()
        self.env = {**os.environ, 'PATH': f'{stubs}:{os.environ["PATH"]}',
                    'EPR_BIN': str(epr), 'BERTH_DIR': str(self.root / 'berth'),
                    'BERTH_UNDER_TEST': '1', 'CLAUDE_PROJECT_DIR': str(self.root),
                    'ELOHIM_DEVICE_KEY_FILE': str(self.root / 'no-device/key'),
                    'XDG_CONFIG_HOME': str(self.root / 'config'),
                    'TEST_OWNER_PID': str(os.getpid()), 'CARGO_CAPTURE': str(self.capture),
                    'CARGO_TARGET_POOL_ROOT': str(self.root / 'pool'),
                    'CARGO_TARGET_POOL_FAMILY': 'test-family',
                    'CARGO_GUARD_PROC_DIR': str(self.proc_dir),
                    'RUSTUP_TOOLCHAIN': 'inherited-pin', 'RUSTFLAGS': '--cfg unwanted-native-flag'}
        self.saved = self.package('crates/demo')
        self.flycheck = BIN / 'lsp-flycheck'

    def observe_contention(self):
        """Observe the real lease when the process scan runs, without compiling Rust."""
        scripts = self.root / 'genesis/agentic/bin'
        scripts.mkdir()
        for name in ('pool-lib.sh', 'berth'):
            (scripts / name).symlink_to(BIN / name)
        self.flycheck = scripts / 'lsp-flycheck'
        shutil.copy2(BIN / 'lsp-flycheck', self.flycheck)
        guard = self.root / '.claude/hooks/cargo-disk-guard.py'
        guard.parent.mkdir(parents=True)
        guard.write_text(f'''
import json, os, pathlib, runpy, time
guard = runpy.run_path({str(REPO / '.claude/hooks/cargo-disk-guard.py')!r})
def running_heavy():
    leases = pathlib.Path(os.environ['BERTH_DIR']) / 'leases.json'
    pathlib.Path(os.environ['CARGO_CAPTURE'] + '.scan').write_text(
        leases.read_text() if leases.exists() else '{{}}')
    time.sleep(float(os.environ.get('SCAN_SLEEP', '0')))
    return guard['running_heavy']()
''')

    def assert_claimed_before_scan_and_released(self):
        leases = json.loads(Path(str(self.capture) + '.scan').read_text())
        rows = leases.get('cargo/slots') or [leases['cargo']]
        own = [row for row in rows if row['holder'] == 'flycheck-session']
        self.assertEqual(len(own), 1, rows)
        self.assertEqual(own[0]['class'], 'lsp')
        ledger = self.root / 'berth/ledger.jsonl'
        events = [json.loads(line) for line in ledger.read_text().splitlines()]
        self.assertEqual([row['kind'] for row in events
                          if row.get('resource') == 'cargo'
                          and row.get('session') == 'flycheck-session'], ['claim', 'release'])

    def package(self, workspace):
        root = self.root / workspace
        (root / 'member/src').mkdir(parents=True)
        (root / 'Cargo.toml').write_text('[workspace]\nmembers = ["member"]\n')
        (root / 'member/Cargo.toml').write_text('[package]\nname = "example"\nversion = "0.1.0"\n')
        saved = root / 'member/src/lib.rs'
        saved.write_text('')
        return saved

    def berth(self, *args):
        return subprocess.run([str(BIN / 'berth'), *args], cwd=self.root,
                              env=self.env, text=True, capture_output=True, timeout=10)

    def run_check(self, saved=None, **env):
        return subprocess.run([str(self.flycheck), 'rust', str(saved or self.saved)],
                              cwd=self.root, env={**self.env, **env}, text=True,
                              capture_output=True, timeout=10)

    def holders(self):
        result = self.berth('who', 'cargo')
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = json.loads(result.stdout)
        return ([rows] if isinstance(rows, dict) else rows) or []

    def run_build_scripts(self, **env):
        return subprocess.run([str(self.flycheck), 'rust', '--build-scripts'],
                              cwd=self.saved.parents[2], env={**self.env, **env},
                              text=True, capture_output=True, timeout=10)

    def test_build_scripts_waits_for_lease_then_runs_once_from_workspace_root(self):
        result = self.berth('claim', 'cargo', '--session', 'gate-session')
        self.assertEqual(result.returncode, 0, result.stderr)
        before = self.holders()
        proc = subprocess.Popen([str(BIN / 'lsp-flycheck'), 'rust', '--build-scripts'],
                                cwd=self.saved.parents[2],
                                env={**self.env, 'LSP_FLYCHECK_POLL_S': '0.1',
                                     'LSP_FLYCHECK_WAIT_MAX_S': '8'},
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(proc.stderr, selectors.EVENT_READ)
                self.assertTrue(selector.select(timeout=5), 'no waiting message')
                waiting = proc.stderr.readline()
            # Claim-first admission releases its own lease before announcing a wait.
            while waiting and 'lsp build-scripts waiting:' not in waiting:
                self.assertIn('cargo: released by flycheck-session', waiting)
                waiting = proc.stderr.readline()
            self.assertIn('lsp build-scripts waiting:', waiting)
            self.assertIn('gate-session', waiting)
            self.assertIsNone(proc.poll())
            self.assertFalse(self.capture.exists())
            self.assertEqual([row for row in self.holders() if row['holder'] == 'gate-session'],
                             before)
            self.assertEqual(self.berth('release', 'cargo', '--session', 'gate-session').returncode, 0)
            stdout, stderr = proc.communicate(timeout=5)
            self.assertEqual(proc.returncode, 0, stderr)
            self.assertEqual([json.loads(line) for line in stdout.splitlines()],
                             [{'reason': 'build-finished', 'success': True}])
        finally:
            if proc.poll() is None:
                proc.terminate()
            proc.communicate(timeout=5)
        row = json.loads(self.capture.read_text())
        self.assertEqual(row['argv'], ['check', '--workspace', '--message-format=json', '--all-targets'])
        self.assertEqual(row['cwd'], str(self.saved.parents[2]))
        self.assertEqual(row['env'], {
            'RUSTUP_TOOLCHAIN': '1.96.1', 'CARGO_BUILD_JOBS': '1', 'RUSTFLAGS': '',
            'ELOHIM_LSP_FLYCHECK': '1',
            'CARGO_TARGET_DIR': str(self.root / 'pool/family/test-family/crates__demo/lsp')})
        leases = row['leases'].get('cargo/slots') or [row['leases']['cargo']]
        self.assertEqual(len(leases), 1)
        self.assertEqual(leases[0]['class'], 'lsp')
        self.assertEqual(leases[0]['ttl_s'], 600)
        self.assertEqual(self.holders(), [])

    def test_build_scripts_timeout_is_nonzero_without_compiling_or_releasing_holder(self):
        result = self.berth('claim', 'cargo', '--session', 'other-lsp', '--class', 'lsp')
        self.assertEqual(result.returncode, 0, result.stderr)
        before = self.holders()
        result = self.run_build_scripts(LSP_FLYCHECK_POLL_S='0.1', LSP_FLYCHECK_WAIT_MAX_S='0.3')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('other-lsp', result.stderr)
        self.assertIn('timed out', result.stderr)
        self.assertEqual(result.stdout, '')
        self.assertFalse(self.capture.exists())
        self.assertEqual(self.holders(), before)

    def test_build_scripts_waits_for_heavy_process_without_a_lease(self):
        process = self.proc_dir / '4242'
        process.mkdir()
        (process / 'cmdline').write_bytes(b'/bin/rustc\0--crate-name\0demo\0')
        (process / 'environ').write_bytes(b'')
        result = self.run_build_scripts(LSP_FLYCHECK_POLL_S='0.1', LSP_FLYCHECK_WAIT_MAX_S='0.3')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('pid 4242', result.stderr)
        self.assertIn('timed out', result.stderr)
        self.assertEqual(result.stdout, '')
        self.assertFalse(self.capture.exists())
        self.assertEqual(self.holders(), [])

    def test_build_scripts_releases_lease_after_cargo_failure(self):
        result = self.run_build_scripts(CARGO_EXIT='101')
        self.assertEqual(result.returncode, 101, result.stderr)
        self.assertTrue(self.capture.exists())
        self.assertEqual(self.holders(), [])

    def test_held_capacity_emits_one_deferral_and_does_not_run_cargo(self):
        for n in range(self.policy['berth']['resources']['cargo']):
            result = self.berth('claim', 'cargo', '--session', f'build-{n}')
            self.assertEqual(result.returncode, 0, result.stderr)
        before = self.holders()
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(result.stdout.splitlines()), 1)
        row = json.loads(result.stdout)
        self.assertEqual(row['reason'], 'compiler-message')
        self.assertEqual(row['package_id'], 'example')
        self.assertEqual(row['manifest_path'], str(self.saved.parents[1] / 'Cargo.toml'))
        self.assertEqual(row['target'], {'kind': ['lib'], 'name': 'example', 'src_path': str(self.saved)})
        warning = row['message']
        self.assertTrue(warning['message'].startswith('flycheck deferred: build slot held by '))
        for lease in before:
            self.assertIn(lease['holder'], warning['message'])
        self.assertEqual(warning['rendered'], warning['message'] + '\n')
        self.assertEqual((warning['level'], warning['spans'], warning['children'], warning['code']),
                         ('warning', [], [], None))
        self.assertFalse(self.capture.exists())
        self.assertEqual(self.holders(), before)

    def test_package_scope_native_pin_slot_and_release_after_success_or_failure(self):
        for code in (0, 101):
            with self.subTest(exit_code=code):
                result = self.run_check(CARGO_EXIT=str(code))
                self.assertEqual(result.returncode, code, result.stderr)
                row = json.loads(self.capture.read_text())
                self.assertEqual(row['argv'], ['check', '--message-format=json', '-p', 'example', '--all-targets'])
                self.assertEqual(row['cwd'], str(self.saved.parents[1]))
                self.assertEqual(row['env'], {
                    'RUSTUP_TOOLCHAIN': '1.96.1', 'CARGO_BUILD_JOBS': '1', 'RUSTFLAGS': '',
                    'ELOHIM_LSP_FLYCHECK': '1',
                    'CARGO_TARGET_DIR': str(self.root / 'pool/family/test-family/crates__demo/lsp')})
                leases = row['leases'].get('cargo/slots') or [row['leases']['cargo']]
                self.assertEqual(leases[0]['class'], 'lsp')
                self.assertEqual(leases[0]['ttl_s'], 600)
                self.assertEqual(self.holders(), [])
                self.assertEqual(json.loads(result.stdout)['reason'], 'build-finished')

    def test_one_foreign_lease_defers_even_with_spare_capacity(self):
        self.observe_contention()
        self.assertGreaterEqual(self.policy['berth']['resources']['cargo'], 2)
        result = self.berth('claim', 'cargo', '--session', 'gate-session')
        self.assertEqual(result.returncode, 0, result.stderr)
        before = self.holders()
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('gate-session', json.loads(result.stdout)['message']['message'])
        self.assertFalse(self.capture.exists())
        self.assertEqual(self.holders(), before)
        self.assert_claimed_before_scan_and_released()

    def test_claim_precedes_contention_scan_and_own_lease_does_not_defer(self):
        self.observe_contention()
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.capture.exists())
        self.assert_claimed_before_scan_and_released()
        self.assertEqual(self.holders(), [])

    def test_heavy_process_found_under_lease_releases_and_defers(self):
        self.observe_contention()
        process = self.proc_dir / '4242'
        process.mkdir()
        (process / 'cmdline').write_bytes(b'/bin/rustc\0--crate-name\0demo\0')
        (process / 'environ').write_bytes(b'')
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('pid 4242', json.loads(result.stdout)['message']['message'])
        self.assertFalse(self.capture.exists())
        self.assert_claimed_before_scan_and_released()
        self.assertEqual(self.holders(), [])

    def test_spent_admission_budget_never_starts_cargo(self):
        self.observe_contention()
        result = self.run_check(LSP_FLYCHECK_TTL_S='13', LSP_FLYCHECK_MARGIN_S='12',
                                SCAN_SLEEP='1')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(self.capture.exists())
        self.assertEqual(json.loads(result.stdout)['message']['message'],
                         'flycheck exceeded its lease; stopped')
        self.assert_claimed_before_scan_and_released()
        self.assertEqual(self.holders(), [])

    def test_timeout_stops_cargo_and_children_and_releases_for_both_modes(self):
        for run in (self.run_check, self.run_build_scripts):
            with self.subTest(mode=run.__name__):
                result = run(LSP_FLYCHECK_TTL_S='14', LSP_FLYCHECK_MARGIN_S='12',
                             CARGO_SLEEP='30', CARGO_CHILD='1')
                self.assertEqual(result.returncode, 0, result.stderr)
                row = json.loads(self.capture.read_text())
                leases = row['leases'].get('cargo/slots') or [row['leases']['cargo']]
                self.assertEqual(leases[0]['ttl_s'], 14)
                warnings = [json.loads(line) for line in result.stdout.splitlines()
                            if json.loads(line)['reason'] == 'compiler-message']
                self.assertEqual(len(warnings), 1, result.stdout)
                self.assertEqual(warnings[0]['message']['message'],
                                 'flycheck exceeded its lease; stopped')
                self.assertEqual(warnings[0]['message']['level'], 'warning')
                self.assertEqual(self.holders(), [])
                for pid in (row['pid'], row['child']):
                    stat = Path(f'/proc/{pid}/stat')
                    self.assertTrue(not stat.exists() or
                                    stat.read_text().rsplit(')', 1)[1].split()[0] == 'Z')

    def test_non_lsp_heavy_process_defers_without_a_lease(self):
        process = self.proc_dir / '4242'
        process.mkdir()
        (process / 'environ').write_bytes(b'')
        for command in (b'/bin/cargo\0+1.96.1\0check\0', b'/bin/rustc\0--crate-name\0demo\0'):
            with self.subTest(command=command):
                (process / 'cmdline').write_bytes(command)
                result = self.run_check()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn('pid 4242', json.loads(result.stdout)['message']['message'])
                self.assertFalse(self.capture.exists())
                self.assertEqual(self.holders(), [])

    def test_lsp_process_does_not_count_as_a_gate(self):
        process = self.proc_dir / '4242'
        process.mkdir()
        (process / 'cmdline').write_bytes(b'/bin/cargo\0check\0')
        (process / 'environ').write_bytes(b'ELOHIM_LSP_FLYCHECK=1\0')
        result = self.run_check()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.capture.exists())

    def test_wasm_unsets_pin(self):
        saved = self.package('elohim/holochain/dna/example')
        result = self.run_check(saved)
        self.assertEqual(result.returncode, 0, result.stderr)
        env = json.loads(self.capture.read_text())['env']
        self.assertIsNone(env['RUSTUP_TOOLCHAIN'])
        self.assertEqual(env['CARGO_BUILD_JOBS'], '1')
        self.assertTrue(env['CARGO_TARGET_DIR'].endswith('/elohim__holochain__dna__example/lsp'))
        self.assertEqual(self.holders(), [])

    def test_no_session_runs_without_a_lease(self):
        result = self.run_check(NO_SESSION='1')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(self.capture.read_text())['leases'], {})
        self.assertEqual(self.holders(), [])

    def test_signal_releases_lease(self):
        proc = subprocess.Popen([str(BIN / 'lsp-flycheck'), 'rust', str(self.saved)],
                                cwd=self.root, env={**self.env, 'CARGO_SLEEP': '30', 'CARGO_CHILD': '1'},
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.addCleanup(lambda: proc.poll() is None and proc.kill())
        deadline = time.monotonic() + 8
        while not self.capture.exists() and proc.poll() is None and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertTrue(self.capture.exists())
        row = json.loads(self.capture.read_text())
        def reap_stubs():
            for pid in (row['pid'], row['child']):
                try:
                    os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        self.addCleanup(reap_stubs)
        proc.send_signal(signal.SIGTERM)
        _, stderr = proc.communicate(timeout=5)
        self.assertEqual(proc.returncode, 143, stderr)
        self.assertEqual(self.holders(), [])
        for pid in (row['pid'], row['child']):
            stat = Path(f'/proc/{pid}/stat')
            self.assertTrue(not stat.exists() or stat.read_text().rsplit(')', 1)[1].split()[0] == 'Z')

    def test_classless_cargo_keeps_default_free_and_explicit_ttl_semantics(self):
        for ttl in (None, 1200):
            args = [] if ttl is None else ['--ttl', str(ttl)]
            result = self.berth('claim', 'cargo', '--session', 'legacy', *args)
            self.assertEqual(result.returncode, 0, result.stderr)
            lease = self.holders()[0]
            self.assertEqual(lease['class'], 'verify')
            self.assertEqual(lease['ttl_s'], ttl)
            renewed = self.berth('claim', 'cargo', '--session', 'legacy')
            self.assertEqual(renewed.returncode, 0, renewed.stderr)
            self.assertEqual(self.holders()[0]['ttl_s'], ttl)
            self.assertEqual(self.berth('release', 'cargo', '--session', 'legacy').returncode, 0)


if __name__ == '__main__':
    unittest.main()
