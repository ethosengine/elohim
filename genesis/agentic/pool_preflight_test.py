"""Pool preflight serves completed evidence while slow walks run single-flight."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import unittest
from concurrent.futures import ThreadPoolExecutor

BIN = Path(__file__).resolve().parent / 'bin'


class PoolPreflightTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.bin = self.root / 'bin'
        self.bin.mkdir()
        shutil.copy2(BIN / 'pool-preflight.sh', self.bin)
        library = (BIN / 'pool-lib.sh').read_text() + '''
steward_all_worktrees() {
  echo "steward:$1" >>"$CALL_LOG"
  while [ -f "${REFRESH_HOLD:-}" ]; do sleep .02; done
  sleep .6
  printf 'cleaned\\tremoved\\tfixture\\tbranch\\n'
}
pool_disk_bytes() {
  echo size >>"$CALL_LOG"
  [ "${FAIL_SCAN:-0}" = 0 ] || return 1
  echo 10737418240
}
legacy_native_bytes() { echo 1073741824; }
stale_incremental_bytes() { echo 2147483648; }
node_modules_stale_bytes() { echo 3221225472; }
angular_cache_stale_bytes() { echo 1073741824; }
worktrees_newer_than_marker() { return 0; }
'''
        (self.bin / 'pool-lib.sh').write_text(library)
        git = self.bin / 'git'
        git.write_text('#!/bin/sh\ncase "$*" in *rev-parse*) echo fix/fixture;; *) exit 99;; esac\n')
        git.chmod(0o755)
        self.repo = self.root / 'repo'
        (self.repo / '.git').mkdir(parents=True)
        (self.repo / '.family').write_text('fixture')
        (self.repo / 'elohim/elohim-storage').mkdir(parents=True)
        self.pool = self.root / 'pool'
        self.pool.mkdir()
        self.log = self.root / 'calls'
        self.env = {**os.environ, 'PATH': str(self.bin) + ':' + os.environ['PATH'],
                    'CLAUDE_PROJECT_DIR': str(self.repo), 'POOL_PARENT_REPO': str(self.repo),
                    'POOL_WORKTREES_DIR': str(self.root / 'worktrees'),
                    'CARGO_TARGET_POOL_ROOT': str(self.pool), 'CALL_LOG': str(self.log),
                    'CARGO_TARGET_POOL_PREFLIGHT_DRY': '0'}
        self.env.pop('CARGO_TARGET_POOL_FAMILY', None)

    def invoke(self, _=None, **env):
        started = time.monotonic()
        result = subprocess.run(['bash', str(self.bin / 'pool-preflight.sh')],
                                input='{}', capture_output=True, text=True,
                                env={**self.env, **env}, timeout=4)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertLess(time.monotonic() - started, 7.5)
        output = json.loads(result.stdout)['hookSpecificOutput']
        self.assertEqual(output['hookEventName'], 'SessionStart')
        return output['additionalContext']

    def wait_report(self):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            reports = list(self.pool.glob('.preflight-*.txt'))
            if reports:
                return reports[0]
            time.sleep(.05)
        self.fail('detached report did not complete')

    def test_cold_single_flight_then_all_stewardship_and_pressure_lines_land(self):
        hold = self.root / 'hold-refresh'
        hold.touch()
        self.env['REFRESH_HOLD'] = str(hold)
        try:
            with ThreadPoolExecutor(max_workers=4) as workers:
                cold = list(workers.map(self.invoke, range(4)))
        finally:
            hold.unlink()
        for text in cold:
            self.assertIn('refresh pending (no completed pass)', text)
            self.assertNotIn('0 removed', text)
            self.assertIn('/family/fixture/elohim__elohim-storage/dev', text)
        self.wait_report()
        warm = self.invoke()
        for line in ('last completed pass', '1 removed', '10.0G', 'Legacy target/',
                     'Stale incremental', 'Stale node_modules', 'Old .angular caches'):
            self.assertIn(line, warm)
        self.assertEqual(self.log.read_text().splitlines(), ['steward:0', 'size'])

    def test_stale_report_survives_failed_refresh_and_keeps_age(self):
        self.invoke()
        report = self.wait_report()
        old = report.read_bytes()
        stamp = time.time() - 180
        os.utime(report, (stamp, stamp))
        text = self.invoke(FAIL_SCAN='1')
        self.assertRegex(text, r'refresh pending \(last pass 18\d+s ago\)')
        self.assertIn('1 removed', text)
        deadline = time.monotonic() + 5
        while self.log.read_text().count('size') < 2 and time.monotonic() < deadline:
            time.sleep(.05)
        self.assertEqual(self.log.read_text().count('size'), 2)
        self.assertEqual(report.read_bytes(), old)
        self.assertEqual(report.stat().st_mtime, stamp)

    def test_busy_lock_returns_pending_without_launching_a_second_pass(self):
        holder = subprocess.Popen(['bash', '-c',
                                   'exec 9>"$1"; flock -x 9; echo ready; read -r _',
                                   '_', str(self.pool / '.preflight.lock')],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        try:
            self.assertEqual(holder.stdout.readline().strip(), 'ready')
            self.assertIn('refresh pending', self.invoke())
            # Wait for a direct contender too; it must return without calling stewardship.
            result = subprocess.run(['bash', str(self.bin / 'pool-preflight.sh'), '--refresh'],
                                    input='', capture_output=True, text=True, env=self.env, timeout=2)
            self.assertEqual(result.returncode, 0)
            self.assertFalse(self.log.exists())
        finally:
            holder.communicate('\n', timeout=2)

    def test_dry_run_report_never_claims_applied_removals(self):
        self.invoke(CARGO_TARGET_POOL_PREFLIGHT_DRY='1')
        self.wait_report()
        text = self.invoke(CARGO_TARGET_POOL_PREFLIGHT_DRY='1')
        self.assertIn('(dry-run plan): 1 would remove', text)
        self.assertNotIn('1 removed', text)
        self.assertEqual(self.log.read_text().splitlines(), ['steward:1', 'size'])



class PoolRootResolutionTest(unittest.TestCase):
    """The pool root: the variable, else the container pool where it exists, else the saved layout."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.asked = self.root / 'asked'
        self.epr = self.root / 'epr'

    def fake_epr(self, body):
        self.epr.write_text(f'#!/bin/sh\necho asked >>"{self.asked}"\n{body}\n')
        self.epr.chmod(0o755)

    def pool_root(self, **env):
        base = {'PATH': os.environ['PATH'], 'HOME': str(self.root), 'EPR_BIN': str(self.epr)}
        out = subprocess.run(['bash', '-c', f'. "{BIN / "pool-lib.sh"}" && pool_root'],
                             env={**base, **env}, capture_output=True, text=True, timeout=20)
        self.assertEqual(out.returncode, 0, out.stderr)
        return out.stdout.strip()

    def test_the_variable_wins_and_epr_is_not_asked(self):
        self.fake_epr('exit 0')
        self.assertEqual(self.pool_root(CARGO_TARGET_POOL_ROOT='/named/pool',
                                        POOL_HOST_DEFAULT=str(self.root / 'absent')), '/named/pool')
        self.assertFalse(self.asked.exists())

    def test_the_container_pool_is_used_where_it_exists_without_asking(self):
        self.fake_epr('exit 0')
        container = self.root / 'container-pool'
        container.mkdir()
        self.assertEqual(self.pool_root(POOL_HOST_DEFAULT=str(container)), str(container))
        self.assertFalse(self.asked.exists())

    def test_a_device_with_no_container_pool_uses_the_place_it_saved(self):
        saved = json.dumps({'cargoPool': {'path': '/home/person/.cache/pool', 'origin': {'kind': 'saved'}}})
        self.fake_epr(f"echo '{saved}'")
        self.assertEqual(self.pool_root(POOL_HOST_DEFAULT=str(self.root / 'absent')),
                         '/home/person/.cache/pool')
        self.assertTrue(self.asked.exists())

    def test_no_saved_place_or_no_epr_keeps_the_old_default(self):
        absent = str(self.root / 'absent')
        self.fake_epr("echo '{\"cargoPool\": null}'")
        self.assertEqual(self.pool_root(POOL_HOST_DEFAULT=absent), absent)
        self.fake_epr('exit 2')
        self.assertEqual(self.pool_root(POOL_HOST_DEFAULT=absent), absent)
        self.assertEqual(self.pool_root(POOL_HOST_DEFAULT=absent, EPR_BIN=str(self.root / 'no-epr')), absent)


if __name__ == '__main__':
    unittest.main()
