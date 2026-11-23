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
worktrees_newer_than_marker() { [ -z "${NEW_WTS:-}" ] || printf '%s\\n' "$NEW_WTS"; }
disk_budget_for_worktree() { printf '30\\t12\\t%s\\n' "${BUDGET_VERDICT:-ok}"; }
'''
        (self.bin / 'pool-lib.sh').write_text(library)
        df = self.bin / 'df'  # the host's real disk must never decide a test
        df.write_text('#!/bin/sh\necho "Filesystem 1K-blocks Used Available Capacity Mounted on"\n'
                      'echo "fake 100 ${DF_PCT:-40} $((100 - ${DF_PCT:-40})) ${DF_PCT:-40}% /"\n')
        df.chmod(0o755)
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
            self.assertNotIn('stewardship', text)
        self.wait_report()
        warm = self.invoke()
        self.assertIn('Worktree stewardship (pass ', warm)
        self.assertIn('1 removed (merged-clean), 0 logged orphan/dirty', warm)
        self.assertEqual(self.log.read_text().splitlines(), ['steward:0'])

    def test_brief_is_at_most_three_lines_and_prints_no_slot_table(self):
        self.invoke()
        self.wait_report()
        text = self.invoke(CARGO_TARGET_POOL_ROOT=str(self.pool))
        lines = text.splitlines()
        self.assertLessEqual(len(lines), 4, text)  # pool line, stewardship (non-zero), pointer
        self.assertIn('family fixture', lines[0])
        self.assertIn('% used', lines[0])
        self.assertIn('cargo-pool key', text)
        self.assertIn('cargo-pool status', text)
        for gone in ('/family/', 'CARGO_TARGET_DIR=', 'Operator commands', 'release',
                     'DNA / WASM', 'Legacy target/', 'Pool size'):
            self.assertNotIn(gone, text)

    def test_quiet_pass_prints_no_stewardship_line(self):
        lib = (self.bin / 'pool-lib.sh').read_text().replace(
            "printf 'cleaned\\tremoved\\tfixture\\tbranch\\n'", "printf 'x\\tleft\\tf\\tb\\n'")
        (self.bin / 'pool-lib.sh').write_text(lib)
        self.invoke()
        self.wait_report()
        text = self.invoke()
        self.assertNotIn('stewardship', text)
        self.assertEqual(len(text.splitlines()), 2, text)

    def test_an_old_format_cache_line_is_never_printed(self):
        self.invoke()
        report = self.wait_report()
        report.write_text('Worktree stewardship: 5 removed (merged-clean), 2 logged orphan/dirty, 1 left untouched.\\n')
        text = self.invoke()
        self.assertNotIn('stewardship', text)
        self.assertNotIn('5 removed', text)

    def test_a_short_new_worktree_prints_one_budget_line_and_others_print_nothing(self):
        wt = self.root / 'wt-new'
        wt.mkdir()
        self.invoke(NEW_WTS=str(wt), BUDGET_VERDICT='short')
        self.wait_report()
        text = self.invoke()
        budget = [l for l in text.splitlines() if 'Budget SHORT' in l]
        self.assertEqual(len(budget), 1, text)
        for needle in ('wt-new', '~30G', 'free 12G', 'cargo-pool estimate'):
            self.assertIn(needle, budget[0])
        (self.root / 'again').mkdir()
        for stale in self.pool.glob('.preflight-*.txt'):
            os.utime(stale, (time.time() - 300,) * 2)
        self.invoke(NEW_WTS=str(wt), BUDGET_VERDICT='tight')
        deadline = time.monotonic() + 5
        while 'Budget SHORT' in self.invoke() and time.monotonic() < deadline:
            time.sleep(.1)
        self.assertNotIn('Budget SHORT', self.invoke())

    def test_a_worktree_that_is_not_short_prints_no_budget_line(self):
        wt = self.root / 'wt-new'
        wt.mkdir()
        self.invoke(NEW_WTS=str(wt), BUDGET_VERDICT='tight')
        self.wait_report()
        self.assertNotIn('Budget', self.invoke())

    def test_warn_banner_at_85_percent_names_enforce(self):
        text = self.invoke(DF_PCT='85')
        lines = text.splitlines()
        self.assertIn('85% used', lines[0])
        self.assertTrue(lines[1].startswith('!  Disk pressure'), text)
        self.assertIn('cargo-pool enforce --yes', text)
        self.assertNotIn('CRITICAL', text)

    def test_critical_banner_at_92_percent_says_heavy_cargo_is_denied(self):
        text = self.invoke(DF_PCT='92')
        self.assertTrue(text.splitlines()[1].startswith('!! CRITICAL DISK PRESSURE'), text)
        self.assertIn('heavy cargo is DENIED at the hard ceiling', text)
        self.assertIn('cargo-pool enforce --yes', text)

    def test_healthy_disk_prints_no_banner(self):
        text = self.invoke()
        self.assertIn('volume 40% used', text.splitlines()[0])
        self.assertNotIn('pressure', text.lower())

    def test_stale_report_survives_failed_refresh_and_keeps_age(self):
        self.invoke()
        report = self.wait_report()
        old = report.read_bytes()
        stamp = time.time() - 180
        os.utime(report, (stamp, stamp))
        text = self.invoke(FAIL_SCAN='1')
        self.assertIn('1 removed', text)
        deadline = time.monotonic() + 5
        while self.log.read_text().count('steward:') < 2 and time.monotonic() < deadline:
            time.sleep(.05)
        self.assertEqual(self.log.read_text().count('steward:'), 2)
        self.assertEqual(report.read_bytes(), old)
        self.assertEqual(report.stat().st_mtime, stamp)

    def test_busy_lock_returns_pending_without_launching_a_second_pass(self):
        holder = subprocess.Popen(['bash', '-c',
                                   'exec 9>"$1"; flock -x 9; echo ready; read -r _',
                                   '_', str(self.pool / '.preflight.lock')],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        try:
            self.assertEqual(holder.stdout.readline().strip(), 'ready')
            self.assertNotIn('stewardship', self.invoke())
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
        self.assertIn('(dry-run plan, pass ', text)
        self.assertIn('1 would remove', text)
        self.assertNotIn('1 removed', text)
        self.assertEqual(self.log.read_text().splitlines(), ['steward:1'])



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
