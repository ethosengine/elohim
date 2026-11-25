"""A worktree keeps its cargo-pool family across a branch rename (backlog row 18).

pool-lib.sh detect_family and its Python mirror in .claude/hooks/cargo-disk-guard.py
record the first family a worktree derives from a real branch in
<git-dir>/cargo-pool-family and return it afterwards, so a rename, a hand-over
under another prefix or a detached head in the same worktree stays warm.

Run: python3 -m unittest genesis.agentic.pool_family_test
"""
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
POOL_LIB = HERE / 'bin' / 'pool-lib.sh'
HOOK = HERE.parents[1] / '.claude' / 'hooks' / 'cargo-disk-guard.py'
RECORD = 'cargo-pool-family'
# Every family and second-level workspace name under /projects/.cargo-target-pool on 2026-11-18.
EXISTING = [
    'berth', 'brit', 'brit-memory', 'brit-noise-20261112', 'claude', 'deps', 'deps-zip', 'dev',
    'epr-1100-f664a1bfb', 'epr-a0bec2549', 'feedback', 'follow', 'follow-outcome', 'gate',
    'hc-fork-071', 'hc-k6v2', 'hostcompat', 'k12', 'k13', 'k9', 'land', 'qualification', 'regen2',
    'restart', 'retire', 'retire-1961', 'rust1100', 'steward', 'steward-arrival',
    'elohim__elohim-storage', 'genesis__orchestrator__rust-style', 'k9-8b273f826__elohim__eprfs',
    'projects__elohim__.worktrees__brit-memory-advisories-20261114', 'xfetch', 'dev', 'release',
]
# Values that must never become a pool path component.
HOSTILE = ['..', '.', '../escape', 'a/b', '/abs', 'a b', 'a\tb', 'a\nb', '-rf', 'x' * 201,
           'ok$(id)', 'a;b', 'café', '  ', '.hidden']


def load_guard():
    spec = importlib.util.spec_from_file_location('cargo_disk_guard_family', HOOK)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PoolFamilyTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.env = {k: v for k, v in os.environ.items()
                    if not k.startswith('GIT_') and k != 'CARGO_TARGET_POOL_FAMILY'}
        self.env.update({'CARGO_TARGET_POOL_ROOT': str(self.root / 'pool'),
                         'GIT_CONFIG_GLOBAL': os.devnull, 'GIT_CONFIG_NOSYSTEM': '1',
                         'EPR_BIN': '/nonexistent'})
        self.repo = self.root / 'repo'
        self.repo.mkdir()
        self.git(self.repo, 'init', '-q', '-b', 'dev')
        self.git(self.repo, '-c', 'user.name=t', '-c', 'user.email=t@example.invalid',
                 'commit', '-q', '--allow-empty', '-m', 'root')
        self.wt = self.root / 'wt'
        self.git(self.repo, 'worktree', 'add', '-q', '-b', 'feat/steward-arrival', str(self.wt))
        self.guard = load_guard()

    def git(self, cwd, *args):
        return subprocess.run(['git', '-C', str(cwd), *args], env=self.env, check=True,
                              capture_output=True, text=True).stdout.strip()

    def bash_family(self, wt, **extra):
        out = subprocess.run(
            ['bash', '-c', 'source "$1" && detect_family "$2"', '_', str(POOL_LIB), str(wt)],
            env={**self.env, **extra}, capture_output=True, text=True, timeout=20)
        self.assertEqual(out.returncode, 0, out.stderr)
        return out.stdout.strip()

    def py_family(self, wt, **extra):
        self.py_stderr = io.StringIO()
        with mock.patch.dict(os.environ, {**self.env, **extra}, clear=True), \
                contextlib.redirect_stderr(self.py_stderr):
            return self.guard.family_for(str(wt))

    def record(self, wt):
        return Path(self.git(wt, 'rev-parse', '--absolute-git-dir')) / RECORD

    def test_rename_prefix_change_and_detached_head_keep_the_recorded_family(self):
        self.assertEqual(self.bash_family(self.wt), 'steward')
        self.assertEqual(self.record(self.wt).read_text().strip(), 'steward')
        self.git(self.wt, 'branch', '-m', 'claude/steward-arrival')
        self.assertEqual(self.bash_family(self.wt), 'steward')
        self.assertEqual(self.py_family(self.wt), 'steward')
        self.git(self.wt, 'switch', '-q', '-c', 'self/handover')
        self.assertEqual(self.bash_family(self.wt), 'steward')
        self.git(self.wt, 'switch', '-q', '--detach')
        self.assertEqual(self.bash_family(self.wt), 'steward')
        self.assertEqual(self.py_family(self.wt), 'steward')

    def test_unrecorded_worktree_derives_as_before_so_existing_families_resolve(self):
        other = self.root / 'other'
        self.git(self.repo, 'worktree', 'add', '-q', '-b', 'claude/nt-pool-prep', str(other))
        self.assertFalse(self.record(other).exists())
        self.assertEqual(self.py_family(other), 'claude')
        self.assertEqual(self.bash_family(other), 'claude')
        # Records are per worktree: the main checkout and the first worktree are untouched.
        self.assertFalse(self.record(self.repo).exists())
        self.assertFalse(self.record(self.wt).exists())
        self.assertEqual(self.bash_family(self.repo), 'dev')

    def test_python_mirror_writes_the_same_record(self):
        self.assertEqual(self.py_family(self.wt), 'steward')
        self.git(self.wt, 'branch', '-m', 'claude/x')
        self.assertEqual(self.bash_family(self.wt), 'steward')

    def test_detached_head_without_a_record_writes_none(self):
        self.git(self.wt, 'switch', '-q', '--detach')
        self.assertEqual(self.bash_family(self.wt), 'wt')
        self.assertIsNone(self.py_family(self.wt))
        self.assertFalse(self.record(self.wt).exists())

    def test_explicit_signals_outrank_the_record_and_forgetting_follows_the_branch(self):
        self.bash_family(self.wt)
        self.assertEqual(self.bash_family(self.wt, CARGO_TARGET_POOL_FAMILY='env'), 'env')
        (self.wt / '.family').write_text('pinned\n')
        self.assertEqual(self.bash_family(self.wt), 'pinned')
        self.assertEqual(self.py_family(self.wt), 'pinned')
        (self.wt / '.family').unlink()
        self.git(self.wt, 'branch', '-m', 'claude/x')
        self.record(self.wt).unlink()
        self.assertEqual(self.bash_family(self.wt), 'claude')

    def bash_run(self, script, *args, **extra):
        return subprocess.run(['bash', '-c', f'source "$1" && shift && {script}', '_', str(POOL_LIB), *args],
                              env={**self.env, **extra}, capture_output=True, text=True, timeout=20)

    def test_validators_agree_and_keep_every_existing_family(self):
        values = EXISTING + HOSTILE + ['sprint', 'as-integrate', 'v1.2', '_x', 'A-Z']
        out = self.bash_run('for v in "$@"; do pool_component_ok "$v" && echo 1 || echo 0; done', *values)
        self.assertEqual(out.returncode, 0, out.stderr)
        bash = [line == '1' for line in out.stdout.splitlines()]
        python = [self.guard.pool_component_ok(v) for v in values]
        self.assertEqual(bash, python)
        self.assertTrue(all(python[:len(EXISTING)]), 'every family in the live pool still resolves')
        self.assertFalse(any(python[len(EXISTING):len(EXISTING) + len(HOSTILE)]))

    def test_hostile_values_from_every_source_are_ignored_identically(self):
        for value in HOSTILE:
            for source in ('env', '.family', 'record'):
                with self.subTest(source=source, value=value):
                    extra = {}
                    record = self.record(self.wt)
                    record.unlink(missing_ok=True)
                    (self.wt / '.family').unlink(missing_ok=True)
                    if source == 'env':
                        extra['CARGO_TARGET_POOL_FAMILY'] = value
                    elif source == '.family':
                        (self.wt / '.family').write_text(value)
                    else:
                        record.write_text(value)
                    out = subprocess.run(
                        ['bash', '-c', 'source "$1" && detect_family "$2"', '_', str(POOL_LIB), str(self.wt)],
                        env={**self.env, **extra}, capture_output=True, text=True, timeout=20)
                    bash = out.stdout.strip()
                    python = self.py_family(self.wt, **extra)
                    self.assertEqual(bash, 'steward', 'falls through to the branch family')
                    self.assertEqual(python, bash)
                    if value.strip() or source == 'env':
                        self.assertIn('cargo-pool: ignoring family', out.stderr)
                        self.assertIn('cargo-pool: ignoring family', self.py_stderr.getvalue())
                    if source == 'record':
                        self.assertEqual(record.read_text(), value, 'an invalid record is never replaced')
        (self.wt / '.family').unlink(missing_ok=True)

    def test_whitespace_around_a_valid_value_is_trimmed_by_both(self):
        (self.wt / '.family').write_text('  sprint \n')
        self.assertEqual(self.bash_family(self.wt), 'sprint')
        self.assertEqual(self.py_family(self.wt), 'sprint')
        self.assertEqual(self.bash_family(self.wt, CARGO_TARGET_POOL_FAMILY=' as-x '), 'as-x')
        self.assertEqual(self.py_family(self.wt, CARGO_TARGET_POOL_FAMILY=' as-x '), 'as-x')

    def test_a_symlinked_or_fifo_record_is_neither_read_nor_written(self):
        record = self.record(self.wt)
        target = self.root / 'outside'
        target.write_text('evil\n')
        record.symlink_to(target)
        for family in (self.bash_family, self.py_family):
            self.assertEqual(family(self.wt), 'steward')
        self.assertEqual(target.read_text(), 'evil\n', 'the link target is never truncated')
        self.assertTrue(record.is_symlink())
        record.unlink()
        record.symlink_to(self.root / 'dangling')
        for family in (self.bash_family, self.py_family):
            self.assertEqual(family(self.wt), 'steward')
        self.assertFalse((self.root / 'dangling').exists(), 'a dangling link is never followed to create')
        record.unlink()
        os.mkfifo(record)
        for family in (self.bash_family, self.py_family):
            self.assertEqual(family(self.wt), 'steward')  # a write or read here would hang to the timeout
        self.assertFalse(record.is_file())

    def test_slot_construction_refuses_anything_but_one_component_per_level(self):
        ok = self.bash_run('slot_path "$@"', 'steward', 'elohim/elohim-storage', 'dev')
        self.assertEqual(ok.returncode, 0)
        self.assertEqual(ok.stdout.strip(), f"{self.root / 'pool'}/family/steward/elohim__elohim-storage/dev")
        for args in (('..', 'elohim', 'dev'), ('a/b', 'elohim', 'dev'), ('steward', '..', 'dev'),
                     ('steward', 'elohim', '../x'), ('', 'elohim', 'dev'), ('a b', 'elohim', 'dev')):
            with self.subTest(args=args):
                out = self.bash_run('slot_path "$@"', *args)
                self.assertEqual(out.returncode, 1)
                self.assertEqual(out.stdout, '')
                self.assertIn('refusing slot outside the pool', out.stderr)
        with mock.patch.object(self.guard, 'family_for', return_value='..'):
            self.assertIsNone(self.guard.slot_for('elohim', str(self.wt), False))
        with mock.patch.object(self.guard, 'family_for', return_value='steward'):
            self.assertIsNone(self.guard.slot_for('..', str(self.wt), False))
            slot = self.guard.slot_for('elohim/elohim-storage', str(self.wt), False)
            self.assertTrue(slot.endswith('/family/steward/elohim__elohim-storage/dev-wt-wt'), slot)

    def test_unwritable_record_still_derives(self):
        # A directory in the record's place can be neither read nor written as a
        # file, which stands in for a read-only git dir even when tests run as root.
        self.record(self.wt).mkdir()
        self.assertEqual(self.bash_family(self.wt), 'steward')
        self.assertEqual(self.py_family(self.wt), 'steward')
        self.git(self.wt, 'branch', '-m', 'claude/x')
        self.assertEqual(self.bash_family(self.wt), 'claude')

    def locked_pool_slot(self, family='evictable'):
        slot = self.root / 'pool' / 'family' / family / 'workspace' / 'dev'
        slot.mkdir(parents=True)
        (slot / 'artifact').write_text('built')
        lock = Path(f'{slot}.gate.lock')
        locker = subprocess.Popen(['flock', '-x', str(lock), 'sleep', '30'])
        self.addCleanup(lambda: locker.poll() is None and locker.kill())
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            probe = subprocess.run(['flock', '-n', str(lock), 'true'])
            if probe.returncode != 0:
                break
            time.sleep(0.01)
        self.assertNotEqual(probe.returncode, 0, 'fixture did not acquire the slot lock')
        return slot, lock

    def assert_busy_slot_delete_is_skipped(self, label):
        slot, lock = self.locked_pool_slot(label)
        out = self.bash_run('delete_slot_with_gate_lock "$1"; echo status=$?', str(slot))
        self.assertEqual(out.returncode, 0, out.stderr)
        self.assertIn('status=75', out.stdout)
        self.assertTrue(slot.is_dir())
        self.assertTrue(lock.exists(), 'a held stable lock inode is never unlinked')

    def assert_busy_family_delete_is_skipped(self, label):
        slot, lock = self.locked_pool_slot(label)
        out = self.bash_run('delete_family_with_gate_locks "$1"; echo status=$?', label)
        self.assertEqual(out.returncode, 0, out.stderr)
        self.assertIn('status=75', out.stdout)
        self.assertTrue(slot.is_dir())
        self.assertTrue(lock.exists(), 'family eviction never unlinks a held slot lock')

    def test_evict_on_merge_path_skips_a_busy_slot_and_preserves_its_lock_inode(self):
        self.assert_busy_family_delete_is_skipped('evict-on-merge')
        self.assertGreaterEqual(POOL_LIB.read_text().count('delete_family_with_gate_locks "$fam"'), 2)

    def test_ttl_path_skips_a_busy_slot_and_preserves_its_lock_inode(self):
        self.assert_busy_family_delete_is_skipped('ttl')
        self.assertIn('enforce_log ttl-evict', POOL_LIB.read_text())

    def test_lru_path_skips_a_busy_slot(self):
        self.assert_busy_slot_delete_is_skipped('lru')
        self.assertIn('delete_slot_with_gate_lock "$victim"', POOL_LIB.read_text())

    def test_keep_warm_trim_path_skips_a_busy_slot(self):
        self.assert_busy_slot_delete_is_skipped('keep-warm')
        self.assertIn('delete_slot_with_gate_lock "$slot"', POOL_LIB.read_text())


if __name__ == '__main__':
    unittest.main()
