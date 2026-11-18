"""A worktree keeps its cargo-pool family across a branch rename (backlog row 18).

pool-lib.sh detect_family and its Python mirror in .claude/hooks/cargo-disk-guard.py
record the first family a worktree derives from a real branch in
<git-dir>/cargo-pool-family and return it afterwards, so a rename, a hand-over
under another prefix or a detached head in the same worktree stays warm.

Run: python3 -m unittest genesis.agentic.pool_family_test
"""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
POOL_LIB = HERE / 'bin' / 'pool-lib.sh'
HOOK = HERE.parents[1] / '.claude' / 'hooks' / 'cargo-disk-guard.py'
RECORD = 'cargo-pool-family'


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
        with mock.patch.dict(os.environ, {**self.env, **extra}, clear=True):
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

    def test_unwritable_record_still_derives(self):
        # A directory in the record's place can be neither read nor written as a
        # file, which stands in for a read-only git dir even when tests run as root.
        self.record(self.wt).mkdir()
        self.assertEqual(self.bash_family(self.wt), 'steward')
        self.assertEqual(self.py_family(self.wt), 'steward')
        self.git(self.wt, 'branch', '-m', 'claude/x')
        self.assertEqual(self.bash_family(self.wt), 'claude')


if __name__ == '__main__':
    unittest.main()
