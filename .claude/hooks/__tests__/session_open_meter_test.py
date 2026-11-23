#!/usr/bin/env python3
"""session-open-context-bytes@1 is the DECLARED SUM of CLAUDE.md + the private memory index
as loaded (first 200 lines) + the hook's own output; the spawn is once-per-session and silent."""
import importlib.util
import os
import re
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

HOOKS = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('load_project_context', HOOKS / 'load-project-context.py')
lpc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lpc)


class SessionOpenSum(unittest.TestCase):
    def fixture(self, claude_md: bytes, index: bytes):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        proj = Path(tmp.name) / 'proj'
        proj.mkdir()
        (proj / 'CLAUDE.md').write_bytes(claude_md)
        slug = re.sub(r'[^A-Za-z0-9]', '-', os.path.realpath(proj))
        mem = Path(tmp.name) / 'cfg' / 'projects' / slug / 'memory'
        mem.mkdir(parents=True)
        (mem / 'MEMORY.md').write_bytes(index)
        return str(proj), str(Path(tmp.name) / 'cfg')

    def test_total_is_the_sum_of_three_named_surfaces(self):
        proj, cfg = self.fixture(b'x' * 1000, b'y' * 300)
        r = lpc.session_open_bytes(proj, 77, config_dir=cfg)
        self.assertEqual(r['surfaces'], {'claude_md': 1000, 'memory_index': 300, 'hook_output': 77})
        self.assertEqual(r['total'], 1377)

    def test_memory_index_counts_only_the_first_200_lines(self):
        proj, cfg = self.fixture(b'', b'abcd\n' * 250)
        r = lpc.session_open_bytes(proj, 0, config_dir=cfg)
        self.assertEqual(r['surfaces']['memory_index'], 5 * 200)

    def test_multibyte_counts_bytes_not_characters(self):
        proj, cfg = self.fixture('é'.encode() * 10, b'')
        self.assertEqual(lpc.session_open_bytes(proj, 0, config_dir=cfg)['surfaces']['claude_md'], 20)

    def test_missing_surfaces_read_zero_never_raise(self):
        with tempfile.TemporaryDirectory() as d:
            r = lpc.session_open_bytes(d, 5, config_dir=d + '/none')
        self.assertEqual(r['total'], 5)

    def test_spawn_is_once_per_session_and_skips_without_session(self):
        with tempfile.TemporaryDirectory() as d, patch('tempfile.gettempdir', return_value=d), \
                patch('subprocess.Popen') as popen:
            lpc._spawn_meter('/p', '', 10)
            self.assertEqual(popen.call_count, 0)
            lpc._spawn_meter('/p', 'sess-a', 10)
            lpc._spawn_meter('/p', 'sess-a', 10)
            lpc._spawn_meter('/p', 'sess-b', 10)
            self.assertEqual(popen.call_count, 2)

    def test_spawn_swallows_popen_failure(self):
        with tempfile.TemporaryDirectory() as d, patch('tempfile.gettempdir', return_value=d), \
                patch('subprocess.Popen', side_effect=OSError('no')):
            lpc._spawn_meter('/p', 'sess-c', 10)  # must not raise

    def test_worktree_resolves_to_the_main_checkouts_memory_slug(self):
        with tempfile.TemporaryDirectory() as d:
            main = Path(d) / 'main'
            (main / '.git' / 'worktrees' / 'x').mkdir(parents=True)
            (main / '.git' / 'worktrees' / 'x' / 'commondir').write_text('../..\n')
            wt = Path(d) / 'wt'
            wt.mkdir()
            (wt / '.git').write_text(f"gitdir: {main}/.git/worktrees/x\n")
            (wt / 'CLAUDE.md').write_bytes(b'z' * 10)
            slug = re.sub(r'[^A-Za-z0-9]', '-', os.path.realpath(main))
            mem = Path(d) / 'cfg' / 'projects' / slug / 'memory'
            mem.mkdir(parents=True)
            (mem / 'MEMORY.md').write_bytes(b'm' * 42)
            r = lpc.session_open_bytes(str(wt), 0, config_dir=str(Path(d) / 'cfg'))
        self.assertEqual(r['surfaces']['memory_index'], 42)
        self.assertEqual(r['surfaces']['claude_md'], 10)

    def test_raising_spawn_leaves_stdout_and_exit_intact(self):
        import io
        import json
        import sys
        with tempfile.TemporaryDirectory() as d:
            out = io.StringIO()
            env = {'CLAUDE_PROJECT_DIR': d}
            with patch.dict(os.environ, env), patch.object(sys, 'stdin', io.StringIO('{}')), \
                    patch.object(sys, 'stdout', out), \
                    patch.object(lpc, '_cached_context', return_value='ctx'), \
                    patch.object(lpc, '_epr_bootstrap_block', return_value='b'), \
                    patch.object(lpc, 'seed_memory_injection_flag'), \
                    patch.object(lpc, '_spawn_meter', side_effect=RuntimeError('boom')):
                lpc.main()  # must not raise or sys.exit
        payload = json.loads(out.getvalue())
        self.assertIn('ctx', payload['hookSpecificOutput']['additionalContext'])


if __name__ == '__main__':
    unittest.main()
