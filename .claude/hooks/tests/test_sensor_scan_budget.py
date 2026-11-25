"""The transcript scan of dialogue-stasis-sense stops at a time budget and resumes from its cursor.

Observed 2026-11-25: UserPromptSubmit timed out (3020 and 3061 ms) on sessions with very large
transcripts; a first scan of a 64 MB transcript measured 2.4-3.5 s, every other phase under 0.1 s.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

REPO = Path(__file__).resolve().parents[3]
SENSE = REPO / '.claude/hooks/dialogue-stasis-sense.py'
OLD = json.dumps({'type': 'hook_cancelled', 'hookName': 'old-hook', 'command': 'x', 'durationMs': 3100, 'timeoutMs': 3000})
FILLER = json.dumps({'type': 'user'})


class ScanBudgetTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.transcript = self.root / 'transcript.jsonl'

    def call(self, **extra_env):
        env = {k: v for k, v in os.environ.items() if not k.startswith('STASIS_')}
        env.update(CLAUDE_PROJECT_DIR=str(self.root), STASIS_JOURNAL_ROOT=str(self.root / 'journal'), **extra_env)
        event = {'session_id': 'session-1', 'hook_event_name': 'UserPromptSubmit', 'prompt_id': 'p1',
                 'transcript_path': str(self.transcript)}
        started = time.monotonic()
        result = subprocess.run([sys.executable, str(SENSE)], input=json.dumps(event), text=True,
                                capture_output=True, env=env, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result, time.monotonic() - started

    def state(self):
        return json.loads((self.root / 'journal/session-1/state.json').read_text())

    def offset(self):
        return next(iter(self.state()['transcripts'].values()))['offset']

    def test_a_scan_that_cannot_finish_returns_in_budget_and_resumes_without_re_alerting(self):
        self.transcript.write_text(''.join([FILLER + '\n'] * 200000 + [OLD + '\n'] + [FILLER + '\n'] * 200000))
        size = self.transcript.stat().st_size
        outputs, offsets = [], []
        for _ in range(40):
            result, seconds = self.call(STASIS_SCAN_UNTIL_S='0.25')
            self.assertLess(seconds, 3.0, f'{seconds:.2f}s')
            outputs.append(result.stdout)
            offsets.append(self.offset())
            if offsets[-1] == size:
                break
        self.assertEqual(offsets[-1], size)
        self.assertGreater(len(set(offsets)), 2, 'the scan must have been split')
        self.assertEqual(offsets, sorted(offsets))
        self.assertNotIn('old-hook', ''.join(outputs))  # history never alerts, split or not
        state = self.state()
        entry = next(iter(state['exceptions'].values()))
        self.assertEqual(entry['count'], entry['baselineCount'])
        self.assertEqual(next(iter(state['transcripts'].values()))['baselineEnd'], size)

    def test_a_new_exception_after_the_baseline_still_alerts(self):
        self.transcript.write_text(''.join([FILLER + '\n'] * 100 + [OLD + '\n']))
        self.call()
        with self.transcript.open('a') as stream:
            stream.write(OLD + '\n')
        result, _ = self.call()
        self.assertIn('old-hook', result.stdout)

    def test_a_small_transcript_is_scanned_whole_with_no_notice(self):
        self.transcript.write_text(''.join([FILLER + '\n'] * 10 + [OLD + '\n']))
        result, _ = self.call(STASIS_SCAN_UNTIL_S='0')
        self.assertEqual((result.stdout, result.stderr), ('', ''))
        self.assertEqual(self.offset(), self.transcript.stat().st_size)
        events = [json.loads(l) for l in (self.root / 'journal/session-1/events.jsonl').read_text().splitlines()]
        self.assertEqual([e['kind'] for e in events if e['kind'] == 'harness-exception'], ['harness-exception'])
        self.assertTrue(next(e for e in events if e['kind'] == 'harness-exception')['exception']['baseline'])


if __name__ == '__main__':
    unittest.main()
