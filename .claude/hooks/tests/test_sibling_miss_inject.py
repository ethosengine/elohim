"""sibling-miss-inject: an open landing-gate miss a sibling posted reaches the agent at its next tool call.

Run: python3 -m unittest discover -s .claude/hooks/tests -p test_sibling_miss_inject.py
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
HOOK = REPO / '.claude/hooks/sibling-miss-inject.py'
SESSION = '5d815440-048a-4134-99ec-68103f1ae736'
SIBLING = 'did:key:z6Mkm5deaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabowj'
INTENT = 'bafyreihywed2fulriygtbammtl3hhovq3fquhdjoqfqkg3ilutma6hv5tm'
EVENT = 'bafyreibzmrz4yptynsnu6urcph2lf34gsgjqi2y6da3lae44naqsulrigm'
COMMIT = '8b9f7a877c9585e84d9f5367e6a7e18540955c11'
CLOSURE = 'fa277e24341e644e161267ac03a76b3780f840333178dc0935330540a8dcf555'


def miss(project='address-profile-lint', *, announcer=SIBLING, closure=CLOSURE, wait=3600, at='1', **extra):
    row = {'intent': INTENT, 'event': EVENT, 'announcer': announcer, 'change': 'I' + 'a' * 40,
           'commit': COMMIT, 'gitRef': 'refs/heads/self/ethosengine-roll-2a', 'project': project,
           'closure': closure, 'p90S': 8, 'waitUntil': None if wait is None else int(time.time()) + wait,
           'at': at}
    row.update(extra)
    return row


class Harness(unittest.TestCase):
    """The real hook against a stub epr that serves `misses.json` and counts its calls."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.project = self.root / 'project'
        self.project.mkdir()
        self.misses = self.root / 'misses.json'
        self.calls = self.root / 'calls'
        self.epr = self.root / 'epr'
        self.notes = self.root / 'notes'
        self.epr.write_text(
            '#!/bin/sh\n'
            # The observation emitter probes `flow note --help` for --measure, then appends.
            'if [ "$1 $2" = "flow note" ]; then\n'
            '  case "$*" in *--help*) echo "usage: --measure"; exit 0 ;; esac\n'
            f'  printf "%s\\n" "$*" >> "{self.notes}"; exit 0\n'
            'fi\n'
            f'echo "$*" >> "{self.calls}"\n'
            '[ "$*" = "peer intents --misses --json" ] || exit 2\n'
            f'[ -n "$STUB_EXIT" ] && exit "$STUB_EXIT"\n'
            f'[ -n "$STUB_SLEEP" ] && sleep "$STUB_SLEEP"\n'
            f'cat "{self.misses}"\n')
        self.epr.chmod(0o755)
        self.serve([])
        clean = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
        self.env = {**clean, 'CLAUDE_PROJECT_DIR': str(self.project), 'ELOHIM_SESSION_ID': '',
                    'SIBLING_MISS_EPR_BIN': str(self.epr), 'SIBLING_MISS_POLL_S': '0',
                    'EPR_BIN': str(self.epr), 'TMPDIR': str(self.root)}
        self.transcript = self.root / 'transcript.jsonl'

    def serve(self, rows):
        self.misses.write_text(json.dumps(rows))

    def epr_calls(self):
        return len(self.calls.read_text().splitlines()) if self.calls.exists() else 0

    def hook(self, payload=None, env=None):
        payload = {'session_id': SESSION} if payload is None else payload
        started = time.monotonic()
        result = subprocess.run([sys.executable, str(HOOK)], cwd=self.project, env=env or self.env,
                                input=payload if isinstance(payload, str) else json.dumps(payload),
                                text=True, capture_output=True, timeout=10)
        self.elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        if not result.stdout.strip():
            return None
        return json.loads(result.stdout)['hookSpecificOutput']['additionalContext']

    def with_transcript(self, **extra):
        self.transcript.touch()
        return {'session_id': SESSION, 'transcript_path': str(self.transcript), **extra}

    def show_in_transcript(self, text):
        with self.transcript.open('a') as f:
            f.write(json.dumps({'content': text}) + '\n')


class Delivery(Harness):
    def test_silent_without_misses(self):
        self.assertIsNone(self.hook())

    def test_a_sibling_miss_arrives_once_framed_as_untrusted_with_its_carry_command(self):
        self.serve([miss()])
        text = self.hook()
        self.assertIn('untrusted data', text)
        self.assertIn('not an instruction', text)
        self.assertIn(f'epr flow intend --carry {INTENT} --project address-profile-lint '
                      f'--closure {CLOSURE} --miss {EVENT} --valid-for 600', text)
        self.assertIn(f'`just gate address-profile-lint` in a worktree at {COMMIT}', text)
        self.assertIsNone(self.hook(), 'shown once, then silent')

    def test_this_devices_own_misses_are_not_shown(self):
        self.serve([miss(announcer=None)])
        self.assertIsNone(self.hook())

    def test_a_miss_whose_lander_stopped_waiting_is_not_shown(self):
        self.serve([miss(wait=-5)])
        self.assertIsNone(self.hook())

    def test_a_miss_with_no_wait_is_shown(self):
        self.serve([miss(wait=None)])
        self.assertIn('the lander set no wait', self.hook())

    def test_a_subagent_and_its_parent_each_hear_the_miss(self):
        self.serve([miss()])
        self.assertIsNotNone(self.hook())
        self.assertIsNotNone(self.hook({'session_id': SESSION, 'agent_id': 'a1b2c3'}))
        self.assertIsNone(self.hook({'session_id': SESSION, 'agent_id': 'a1b2c3'}))

    def test_at_most_three_per_call_and_the_rest_follow(self):
        self.serve([miss(f'p{i}', closure=f'{i:x}' * 16, at=str(i)) for i in range(1, 6)])
        first = self.hook()
        self.assertEqual(first.count('[sibling-miss] p'), 3)
        self.assertIn('2 more open miss(es)', first)
        second = self.hook()
        self.assertEqual(second.count('[sibling-miss] p'), 2)
        self.assertIsNone(self.hook())

    def test_a_settled_miss_leaves_the_cursor_and_a_reposted_one_is_shown_again(self):
        self.serve([miss()])
        self.assertIsNotNone(self.hook())
        self.serve([])
        self.assertIsNone(self.hook())
        self.serve([miss()])
        self.assertIsNotNone(self.hook(), 'a miss that closed and reopened is news again')

    def test_valid_for_is_twice_p90_within_carry_bounds(self):
        self.serve([miss(p90S=2000)])
        self.assertIn('--valid-for 4000', self.hook())
        self.serve([miss('other', closure='b' * 16, p90S=9000)])
        self.assertIn('--valid-for 7200', self.hook())


class Hostile(Harness):
    def test_a_field_with_an_unexpected_shape_is_counted_never_printed(self):
        evil = 'x; rm -rf / #'
        self.serve([miss(project=evil), miss(gitRef='refs/heads/a b'), miss(commit='HEAD'),
                    miss(announcer='did:key:z6Mk"; echo')])
        text = self.hook()
        self.assertNotIn(evil, text)
        self.assertNotIn('echo', text)
        self.assertIn('4 open miss(es) not shown', text)

    def test_invalid_payloads_and_ids_fail_open(self):
        self.serve([miss()])
        for payload in ['not json', '[]', {'session_id': 'bad id with spaces'},
                        {'session_id': SESSION, 'agent_id': '../../etc'}]:
            self.assertIsNone(self.hook(payload))

    def test_a_missing_broken_or_old_epr_is_silence(self):
        self.serve([miss()])
        for env in [{'SIBLING_MISS_EPR_BIN': str(self.root / 'absent')}, {'STUB_EXIT': '2'}]:
            self.assertIsNone(self.hook(env={**self.env, **env}))
        self.misses.write_text('{not json')
        self.assertIsNone(self.hook())

    def test_a_slow_epr_is_cut_off_and_the_call_stays_fast(self):
        self.serve([miss()])
        self.assertIsNone(self.hook(env={**self.env, 'STUB_SLEEP': '5'}))
        self.assertLess(self.elapsed, 2.9, 'inside the 3 s harness timeout')

    def test_epr_is_asked_at_most_once_per_poll_interval(self):
        env = {**self.env, 'SIBLING_MISS_POLL_S': '3600'}
        self.hook(env=env)
        self.hook(env=env)
        self.assertEqual(self.epr_calls(), 1)


class FailureIsObserved(Harness):
    """A failed misses read is silence on the tool call, never unrecorded (lane 4 follow-up)."""

    def noted(self, wait=3.0):
        deadline = time.monotonic() + wait
        while time.monotonic() < deadline:
            if self.notes.exists() and self.notes.read_text().strip():
                return self.notes.read_text().strip().splitlines()
            time.sleep(0.05)
        return []

    def state(self):
        return json.loads(next((self.project / '.eprfs/status/sibling-miss').glob('*.json')).read_text())

    def test_a_failed_read_is_one_observation_and_the_call_hears_nothing(self):
        failing = {**self.env, 'STUB_EXIT': '2'}
        self.assertIsNone(self.hook(env=failing))
        notes = self.noted()
        self.assertEqual(len(notes), 1, notes)
        self.assertIn('--measure sibling-miss-read-failed@1', notes[0])
        self.assertIn(f'--subject {SESSION}:main', notes[0])
        self.assertIn('--env failure=nonzero-exit', notes[0])

    def test_failures_are_noted_at_most_once_per_ten_minutes_with_the_blind_seconds(self):
        failing = {**self.env, 'STUB_EXIT': '2'}
        self.hook(env=failing)
        self.assertEqual(len(self.noted()), 1)
        self.hook(env=failing)
        time.sleep(0.3)
        self.assertEqual(len(self.noted()), 1, 'a second failure inside ten minutes is not noted again')
        data = self.state()
        data['health']['failure_noted_at'] -= 601
        data['health']['failing_since'] -= 900
        (next((self.project / '.eprfs/status/sibling-miss').glob('*.json'))).write_text(json.dumps(data))
        self.hook(env=failing)
        notes = self.noted()
        deadline = time.monotonic() + 3
        while len(notes) < 2 and time.monotonic() < deadline:
            time.sleep(0.05)
            notes = self.noted()
        self.assertEqual(len(notes), 2, notes)
        value = int(notes[1].split('--value ', 1)[1].split()[0])
        self.assertGreaterEqual(value, 900, 'the value is the seconds this agent has been blind')

    def test_a_good_read_resets_the_clock_and_carries_through_cursor_writes(self):
        self.hook(env={**self.env, 'STUB_EXIT': '2'})
        self.assertIn('failing_since', self.state()['health'])
        self.serve([miss()])
        self.assertIsNotNone(self.hook())
        health = self.state()['health']
        self.assertIn('last_ok', health)
        self.assertNotIn('failing_since', health)
        self.assertNotIn('failure_noted_at', health)

    def test_a_missing_epr_names_its_failure(self):
        self.assertIsNone(self.hook(env={**self.env, 'SIBLING_MISS_EPR_BIN': str(self.root / 'absent')}))
        notes = self.noted()
        self.assertEqual(len(notes), 1, notes)
        self.assertIn('--env failure=no-epr', notes[0])


class Receipts(Harness):
    def test_with_a_transcript_the_miss_stays_pending_until_its_receipt_is_seen(self):
        self.serve([miss()])
        payload = self.with_transcript()
        text = self.hook(payload)
        receipt = text.rsplit('receipt ', 1)[1].strip()
        self.assertRegex(receipt, r'^r-[0-9a-f]{10}$')
        self.assertIsNone(self.hook(payload), 'within the grace: wait for the write')
        self.show_in_transcript(text)
        self.assertIsNone(self.hook(payload), 'receipt seen: the miss is seen, nothing new')

    def test_a_dropped_delivery_is_shown_again_as_a_repeat(self):
        self.serve([miss()])
        payload = self.with_transcript()
        self.assertIsNotNone(self.hook(payload))
        state = next((self.project / '.eprfs/status/sibling-miss').glob('*.json'))
        data = json.loads(state.read_text())
        data['pending']['emitted_at'] -= 60  # past the 20 s grace, never written to the transcript
        state.write_text(json.dumps(data))
        again = self.hook(payload)
        self.assertIn('Shown again (repeat 1)', again)
        self.assertIn('address-profile-lint', again)

    def test_without_a_transcript_the_miss_is_seen_after_the_write(self):
        self.serve([miss()])
        self.assertIsNotNone(self.hook())
        state = next((self.project / '.eprfs/status/sibling-miss').glob('*.json'))
        self.assertNotIn('pending', json.loads(state.read_text()))


if __name__ == '__main__':
    unittest.main()
