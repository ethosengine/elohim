"""RAM hook attribution delegates to the stdlib shim without starting the evaluator."""
import importlib.util
from pathlib import Path
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('ram_hook', ROOT / '.claude/hooks/ram-guard.py')
hook = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hook)


class RamSessionTest(unittest.TestCase):
    def test_owner_and_session_use_one_shim_without_subprocess(self):
        identity = dict(ownerPid=1234, session='codex-1234-7', runtime='codex')
        with patch.object(hook, '_session_identity', return_value=identity), patch.object(hook.subprocess, 'run', side_effect=AssertionError('no subprocess')):
            self.assertEqual(1234, hook.owner_pid())
            self.assertEqual('codex-1234-7', hook.session_key({}))
            self.assertEqual('hook-session', hook.session_key({'session_id': 'hook-session'}))

    def test_unobserved_owner_preserves_parent_and_default_fallbacks(self):
        with patch.object(hook, '_session_identity', return_value={'ownerPid': None, 'session': None}), patch.object(hook.os, 'getppid', return_value=321):
            self.assertEqual(321, hook.owner_pid())
            self.assertEqual('default', hook.session_key({}))

    def test_fallback_is_fast_and_stdlib(self):
        with patch.object(hook.subprocess, 'run', side_effect=AssertionError('no subprocess')):
            start = time.monotonic()
            for _ in range(20):
                hook._session_identity()
            self.assertLess(time.monotonic() - start, 1)


if __name__ == '__main__':
    unittest.main()
