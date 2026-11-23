"""Rule 2 (pool discipline) names the slot, or how to get it, in its denial.

Run: python3 -m unittest discover -s .claude/hooks/tests -p 'test_cargo_disk_guard*.py'
"""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import unittest
from unittest import mock


HOOK = Path(__file__).resolve().parents[1] / 'cargo-disk-guard.py'
spec = importlib.util.spec_from_file_location('cargo_disk_guard_pool', HOOK)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)

COMMAND = 'cd elohim/elohim-storage && cargo build -j1'


class PoolDisciplineTest(unittest.TestCase):
    def setUp(self):
        for patch in (
            mock.patch.object(guard, 'disk_pct', return_value=50),
            mock.patch.object(guard, 'read_policy', return_value=(85, 75)),
            mock.patch.dict(os.environ, {'CARGO_BUILD_JOBS': '1'}),
        ):
            patch.start()
            self.addCleanup(patch.stop)

    def denial(self, slot):
        out = io.StringIO()
        data = {'tool_name': 'Bash', 'cwd': '/scratch', 'tool_input': {'command': COMMAND}}
        with mock.patch.object(guard, 'slot_for', return_value=slot), \
                mock.patch.object(guard.sys, 'stdin', io.StringIO(json.dumps(data))), \
                contextlib.redirect_stdout(out):
            with self.assertRaises(SystemExit):
                guard.main()
        out_hook = json.loads(out.getvalue().splitlines()[0])['hookSpecificOutput']
        self.assertEqual(out_hook['permissionDecision'], 'deny')
        return out_hook['permissionDecisionReason']

    def test_no_slot_points_at_cargo_pool_key_not_the_session_start_block(self):
        text = self.denial(None)
        self.assertIn('POOL DISCIPLINE', text)
        self.assertIn('cargo-pool key', text)
        self.assertIn('from the workspace directory', text)
        self.assertNotIn('SessionStart', text)
        self.assertNotIn('Slot map', text)

    def test_a_known_slot_is_named_and_the_trailing_pointer_is_the_same(self):
        text = self.denial('/pool/family/f/elohim_elohim-storage/dev')
        self.assertIn('CARGO_TARGET_DIR=/pool/family/f/elohim_elohim-storage/dev ', text)
        self.assertIn('cargo-pool key', text)
        self.assertIn('cargo-pool status', text)
        self.assertNotIn('SessionStart', text)
        self.assertNotIn('Slot map', text)


if __name__ == '__main__':
    unittest.main()
