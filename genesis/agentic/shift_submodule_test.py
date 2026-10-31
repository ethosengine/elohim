"""A source-clean revision binds dependency commits even when their local dirt is ignored."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent))
import shift_ops as ops


class SubmoduleRevisionTests(unittest.TestCase):
    def test_gitlink_drift_refuses_readiness_and_delivery(self):
        with tempfile.TemporaryDirectory(prefix='shift-submodule-') as scratch:
            root = Path(scratch) / 'root'
            upstream = Path(scratch) / 'upstream'
            def git(cwd, *args):
                return subprocess.check_output(['git', '-c', 'protocol.file.allow=always',
                    '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
                    *args], cwd=cwd, text=True, stderr=subprocess.DEVNULL).strip()
            for directory in [root, upstream]:
                directory.mkdir()
                git(directory, 'init', '-q')
            (upstream / 'code').write_text('first\n')
            git(upstream, 'add', 'code')
            git(upstream, 'commit', '-qm', 'first')
            first = git(upstream, 'rev-parse', 'HEAD')
            git(upstream, 'commit', '--allow-empty', '-qm', 'second')
            second = git(upstream, 'rev-parse', 'HEAD')
            git(root, 'submodule', 'add', '-q', str(upstream), 'dependency')
            dependency = root / 'dependency'
            git(dependency, 'checkout', '-q', first)
            git(root, 'add', '.gitmodules', 'dependency')
            git(root, 'commit', '-qm', 'pinned dependency')
            module = Path(__file__).with_name('readiness.mjs').as_uri()
            def readiness():
                script = f'import {{checkGitClean}} from {json.dumps(module)}; console.log(JSON.stringify(await checkGitClean({{cwd:process.argv[1]}})));'
                return json.loads(subprocess.check_output(['node', '--input-type=module', '-e', script, str(root)], text=True))['ok']
            self.assertTrue(ops.clean(root))
            self.assertTrue(readiness())
            (dependency / 'code').write_text('local uncommitted work\n')
            self.assertTrue(ops.clean(root))
            self.assertTrue(readiness())
            git(dependency, 'checkout', '-q', second)
            self.assertFalse(ops.clean(root))
            self.assertFalse(readiness())
