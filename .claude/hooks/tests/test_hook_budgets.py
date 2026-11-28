"""
epr-meta-resolver ends inside its 3 s harness budget when `epr` is slow (observed 2026-11-25: PreToolUse:Write
3088 ms), and `epr_client` still loads by filename the way stasis_control.worker() loads it.
"""
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import threading
import time
import unittest

REPO = Path(__file__).resolve().parents[3]
RESOLVER = REPO / '.claude/hooks/epr-meta-resolver.py'
SENSE = REPO / '.claude/hooks/dialogue-stasis-sense.py'
BUDGET = 3.0  # the harness's timeout for both hooks


class ResolverBudgetTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)

    def run_hook(self, hook, event, env):
        started = time.monotonic()
        result = subprocess.run([sys.executable, str(hook)], input=json.dumps(event), text=True,
                                capture_output=True, env=env, timeout=20)
        return result, time.monotonic() - started

    def test_resolver_skips_a_slow_epr_inside_the_budget(self):
        stubs = self.root / 'bin'
        stubs.mkdir()
        epr = stubs / 'epr'
        epr.write_text('#!/bin/sh\nsleep 10\n')
        epr.chmod(epr.stat().st_mode | stat.S_IXUSR)
        env = {k: v for k, v in os.environ.items() if k not in {'EPR_BIN', 'EPR_SESSION_RESOLVING'}}
        env.update(PATH=f'{stubs}:{os.environ["PATH"]}', CLAUDE_PROJECT_DIR=str(REPO),
                   EPR_CLIENT_CACHE_TTL='0')
        target = REPO / 'genesis/docs/hook-budget-probe.md'
        event = {'session_id': 'hook-budget', 'hook_event_name': 'PreToolUse', 'tool_name': 'Write',
                 'tool_input': {'file_path': str(target), 'content': 'probe\n'}, 'cwd': str(REPO)}
        result, seconds = self.run_hook(RESOLVER, event, env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertLess(seconds, BUDGET, f'{seconds:.2f}s')
        self.assertIn('epr-meta-resolver: skipped', result.stderr)
        self.assertEqual(len([l for l in result.stderr.splitlines() if 'skipped' in l]), 1, result.stderr)

    def test_epr_client_loads_by_filename_as_the_native_worker_does(self):
        spec = importlib.util.spec_from_file_location('stasis_epr_client', REPO / '.claude/scripts/_lib/epr_client.py')
        client = importlib.util.module_from_spec(spec)
        code = ('import importlib.util,sys\n'
                f'spec = importlib.util.spec_from_file_location("stasis_epr_client", {str(spec.origin)!r})\n'
                'm = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)\n'
                'assert callable(m.resolve_binary) and m.deadline.cap(2, "x") == 2\n')
        result = subprocess.run([sys.executable, '-c', code], cwd=self.root, capture_output=True, text=True,
                                env={k: v for k, v in os.environ.items() if k != 'PYTHONPATH'})
        self.assertEqual((result.returncode, result.stderr), (0, ''))

    def test_dialogue_sensor_fails_open_before_a_slow_child_reaches_the_harness_limit(self):
        child = self.root / 'slow-child'
        child.write_text('#!/bin/sh\nsleep 10\n')
        child.chmod(child.stat().st_mode | stat.S_IXUSR)
        env = {k: v for k, v in os.environ.items() if not k.startswith('STASIS_')}
        env.update(CLAUDE_PROJECT_DIR=str(self.root),
                   STASIS_JOURNAL_ROOT=str(self.root / 'journal'),
                   STASIS_COORDINATOR_BIN=str(child), EPR_BIN=str(child),
                   STASIS_SENSE_DEADLINE_S='0.15')
        event = {'session_id': 'hook-budget', 'hook_event_name': 'SessionStart'}
        result, seconds = self.run_hook(SENSE, event, env)
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, '', ''))
        self.assertLess(seconds, BUDGET, f'{seconds:.2f}s')
        self.assertFalse((self.root / 'journal').exists(), 'deadline path writes no partial journal')


    def test_dialogue_sensor_cut_short_by_its_deadline_still_advances_its_transcript_cursor(self):
        # Row 108: a scan stopped by the inner deadline must still commit its cursor, or a loaded
        # device rescans the same stretch on every call and never progresses.
        quick = self.root / 'quick-child'
        quick.write_text('#!/bin/sh\nexit 0\n')
        quick.chmod(quick.stat().st_mode | stat.S_IXUSR)
        transcript = self.root / 'transcript.jsonl'
        line = json.dumps({'type': 'assistant', 'message': {'content': 'x' * 200}}) + '\n'
        transcript.write_text(line * 400_000)
        env = {k: v for k, v in os.environ.items() if not k.startswith('STASIS_')}
        env.update(CLAUDE_PROJECT_DIR=str(self.root),
                   STASIS_JOURNAL_ROOT=str(self.root / 'journal'),
                   STASIS_COORDINATOR_BIN=str(quick), EPR_BIN=str(quick),
                   STASIS_SENSE_DEADLINE_S='1.0')
        event = {'session_id': 'hook-progress', 'hook_event_name': 'PostToolUse',
                 'transcript_path': str(transcript), 'tool_name': 'Bash',
                 'tool_input': {'command': 'true'}, 'tool_response': {}}
        offsets = []
        for _ in range(2):
            result, seconds = self.run_hook(SENSE, event, env)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertLess(seconds, BUDGET, f'{seconds:.2f}s')
            state = json.loads((self.root / 'journal' / 'hook-progress' / 'state.json').read_text())
            offsets.append(max(c['offset'] for c in state['transcripts'].values()))
        self.assertGreater(offsets[0], 0)
        self.assertGreater(offsets[1], offsets[0], offsets)
        self.assertLess(offsets[1], transcript.stat().st_size, 'the fixture must outlast two scans')


if __name__ == '__main__':
    unittest.main()
