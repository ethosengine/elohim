"""Subprocess replay of the v1 observe-only adapter and its context-only relay."""
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

try:
    import jsonschema
except ImportError:
    jsonschema = None

REPO = Path(__file__).resolve().parents[3]
HOOK = REPO / '.claude/hooks/dialogue-stasis-sense.py'
SCHEMA = json.loads((REPO / 'elohim/sdk/domains/elohim-agent/schemas/session-event.schema.json').read_text())
VALIDATOR = jsonschema.Draft202012Validator(SCHEMA, format_checker=jsonschema.FormatChecker()) if jsonschema else None


class DialogueStasisSenseTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.transcript = self.root / 'transcript.jsonl'
        self.transcript.touch()
        self.journal = self.root / 'journal'
        self.directory = self.journal / 'session-1'
        clean = {k: v for k, v in os.environ.items() if not k.startswith('STASIS_')}
        self.env = {**clean, 'CLAUDE_PROJECT_DIR': str(self.root), 'STASIS_JOURNAL_ROOT': str(self.journal)}
        self.addCleanup(self.validate_events)

    def invoke(self, event, output=False, env=None, **fields):
        payload = dict(session_id='session-1', hook_event_name=event, prompt_id='prompt-1',
                       transcript_path=str(self.transcript))
        payload.update(fields)
        result = subprocess.run([sys.executable, str(HOOK)], input=json.dumps(payload), text=True,
                                capture_output=True, env={**self.env, **(env or {})}, timeout=5)
        self.assertEqual((result.returncode, result.stderr), (0, ''), result.stderr)
        if not output:
            self.assertEqual(result.stdout, '')
        return result.stdout

    def events(self, kind=None):
        paths = sorted(self.directory.glob('events.*.jsonl'), key=lambda p: int(p.name.split('.')[1]))
        if (self.directory / 'events.jsonl').exists():
            paths.append(self.directory / 'events.jsonl')
        events = [json.loads(line) for p in paths for line in p.read_text().splitlines() if line.strip()]
        return [e for e in events if kind is None or e['kind'] == kind]

    def validate_events(self):
        for event in self.events():
            if VALIDATOR:
                VALIDATOR.validate(event)
            else:
                self.assertEqual(set(SCHEMA['required']) - event.keys(), set())
                self.assertEqual(event['schemaVersion'], 1)
                self.assertIn(event['kind'], SCHEMA['properties']['kind']['enum'])
                self.assertTrue(set(event) <= SCHEMA['properties'].keys())
                for rule in SCHEMA['allOf']:
                    selector = rule['if']['properties']['kind']
                    if event['kind'] in selector.get('enum', [selector.get('const')]):
                        self.assertTrue(set(rule['then']['required']) <= event.keys())

    def state(self):
        return json.loads((self.directory / 'state.json').read_text())

    def write_state(self, state):
        (self.directory / 'state.json').write_text(json.dumps(state))

    def pre(self, tool_id='call-1', command='git status', **fields):
        self.invoke('PreToolUse', tool_name='Bash', tool_use_id=tool_id,
                    tool_input={'command': command}, **fields)

    def request(self, command='git status', **fields):
        self.invoke('PermissionRequest', tool_name='Bash', tool_input={'command': command}, **fields)

    def cancelled(self, count=1, **fields):
        attachment = dict(type='hook_cancelled', hookName='native-seam-guidance', command='python3 /private/hook.py --token=SECRET',
                          durationMs=2100, timeoutMs=2000, timedOut=True)
        attachment.update(fields)
        with self.transcript.open('a') as stream:
            for _ in range(count):
                stream.write(json.dumps({'type': 'attachment', 'attachment': attachment}) + '\n')

    def context(self, event='PostToolUse'):
        out = self.invoke(event, output=True)
        if not out:
            return ''
        if event == 'UserPromptSubmit':
            return out.strip()
        envelope = json.loads(out)
        self.assertEqual(set(envelope), {'hookSpecificOutput'})
        self.assertEqual(envelope['hookSpecificOutput']['hookEventName'], event)
        self.assertEqual(set(envelope['hookSpecificOutput']), {'hookEventName', 'additionalContext'})
        return envelope['hookSpecificOutput']['additionalContext']

    def test_approve_serial_and_canonical_digest(self):
        self.pre(command='curl -s https://secret.example/x')
        self.request(command='curl -s https://secret.example/x')
        event = self.events('permission-requested')[0]
        self.assertEqual(event['source']['precedingToolUseId'], 'call-1')
        self.assertEqual(event['correlation']['confidence'], 'high')
        expected = 'sha256:' + hashlib.sha256(json.dumps({'command': 'curl -s https://secret.example/x'}, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        self.assertEqual(event['inputDigest'], expected)
        self.assertEqual(event['shape'], 'curl -s <url>')
        self.invoke('PostToolUse', tool_use_id='call-1', duration_ms=750)
        close = self.events('prompt-closed')[0]
        self.assertEqual(close['closedBy'], 'approved')
        self.assertEqual(close['durationMs'], 750)
        self.assertEqual(close['correlation']['incidentId'], event['correlation']['incidentId'])
        self.assertEqual(close['presence']['disposition'], 'collaborative')
        self.assertEqual(self.state()['openPrompts']['main'], [])

    def test_inferred_deny_at_each_next_event(self):
        for next_event in ('PreToolUse', 'Stop', 'UserPromptSubmit'):
            with self.subTest(next_event=next_event):
                self.invoke('UserPromptSubmit', prompt='New turn')
                self.pre()
                self.request()
                self.invoke(next_event, tool_name='Bash', tool_use_id='next', prompt='continue')
                close = self.events('prompt-closed')[-1]
                self.assertEqual(close['closedBy'], 'denied-inferred')
                self.assertEqual(close['correlation']['confidence'], 'low')
                self.assertEqual(close['correlation']['method'], 'inferred-next-event')

    def test_subagent_correlation_and_parent_does_not_close_it(self):
        self.pre(agent_id='child')
        self.request(agent_id='child')
        self.invoke('Stop', last_assistant_message='Parent done.')
        self.assertEqual(self.events('prompt-closed'), [])
        self.invoke('PostToolUse', agent_id='child', tool_use_id='call-1', duration_ms=5)
        self.assertEqual(self.events('prompt-closed')[0]['agentId'], 'child')

    def test_parallel_batch_has_no_guessed_id(self):
        self.pre('a', 'git status')
        self.pre('b', 'git diff')
        self.request('git status')
        self.request('git diff')
        for event in self.events('permission-requested'):
            self.assertIsNone(event['source']['precedingToolUseId'])
            self.assertEqual(event['correlation']['confidence'], 'low')
        self.invoke('PostToolUse', tool_use_id='a', duration_ms=2)
        self.assertEqual(self.events('prompt-closed'), [])
        self.invoke('Stop')
        self.assertTrue(all(e['closedBy'] == 'unknown' for e in self.events('prompt-closed')))
        self.assertIn('unknown-close', [e['gap']['reason'] for e in self.events('observation-gap')])

    def test_parallel_ambiguity_survives_one_completion(self):
        self.pre('a', 'git status')
        self.pre('b', 'git diff')
        self.invoke('PostToolUse', tool_use_id='b')
        self.request('git status')
        self.assertIsNone(self.events('permission-requested')[0]['source']['precedingToolUseId'])

    def test_identical_inputs_never_choose_a_call(self):
        self.pre('a')
        self.pre('b')
        self.request()
        self.request()
        for event in self.events('permission-requested'):
            self.assertIsNone(event['source']['precedingToolUseId'])
            self.assertEqual(event['correlation']['confidence'], 'low')
        self.invoke('PostToolUse', tool_use_id='a')
        self.assertEqual(self.events('prompt-closed'), [])

    def test_repeated_request_invalidates_cached_correlation(self):
        self.pre()
        self.request()
        self.request()
        self.assertIsNone(self.events('permission-requested')[-1]['source']['precedingToolUseId'])
        self.invoke('PostToolUse', tool_use_id='call-1')
        self.assertEqual(self.events('prompt-closed'), [])

    def test_wrong_prompt_does_not_match(self):
        self.pre(prompt_id='one')
        self.request(prompt_id='two')
        self.assertIsNone(self.events('permission-requested')[0]['source']['precedingToolUseId'])

    def test_notification_permission_prompt(self):
        self.pre()
        self.request()
        self.invoke('Notification', notification_type='permission_prompt')
        self.assertEqual(self.events('dialog-displayed')[0]['correlation']['incidentId'],
                         self.events('permission-requested')[0]['correlation']['incidentId'])

    def test_notification_without_open_prompt_records_gap(self):
        self.invoke('Notification', notification_type='permission_prompt')
        self.assertEqual(self.events('dialog-displayed')[0]['correlation']['confidence'], 'unknown')
        self.assertTrue(self.events('observation-gap'))

    def test_matching_tool_failure_is_approved(self):
        self.pre()
        self.request()
        self.invoke('PostToolUseFailure', tool_use_id='call-1', duration_ms=20, error='SECRET')
        self.assertEqual(self.events('prompt-closed')[0]['closedBy'], 'approved')
        self.assertEqual(self.events('classifier-denied'), [])

    def test_classifier_denied_and_session_end(self):
        self.invoke('PermissionDenied', tool_name='Bash', tool_input={'command': 'git status'})
        self.assertEqual(len(self.events('classifier-denied')), 1)
        self.pre()
        self.request()
        self.invoke('SessionEnd')
        self.assertEqual(self.events('prompt-closed')[0]['closedBy'], 'unknown')
        self.assertEqual(len(self.events('session-ended')), 1)

    def test_presence_and_system_notification(self):
        self.invoke('Stop')
        self.assertEqual(self.events()[-1]['presence']['disposition'], 'unknown')
        self.invoke('UserPromptSubmit', prompt='stuck? private words')
        witness = self.state()['presence']
        self.assertEqual(self.events('steward-ping')[0]['presence']['disposition'], 'collaborative')
        self.invoke('UserPromptSubmit', prompt='<task-notification>stuck?</task-notification>')
        self.assertEqual(self.state()['presence'], witness)
        self.assertEqual(len(self.events('steward-ping')), 1)
        self.invoke('UserPromptSubmit', prompt='Please continue')
        self.invoke('Stop', last_assistant_message='Done.')
        self.assertEqual(self.events('turn-ended')[-1]['presence']['disposition'], 'collaborative')

    def test_task_notification_alone_does_not_invent_presence(self):
        self.invoke('UserPromptSubmit', prompt='<task-notification>wake</task-notification>')
        self.invoke('Stop')
        self.assertEqual(self.events()[-1]['presence']['disposition'], 'unknown')

    def test_passive_presence_window_expires(self):
        self.invoke('UserPromptSubmit', prompt='Hello')
        state = self.state()
        state['presence']['lastWitnessUptimeSeconds'] -= 601
        self.write_state(state)
        self.invoke('Stop')
        self.assertEqual(self.events()[-1]['presence']['disposition'], 'automated')

    def test_question_classification_never_keeps_dialogue(self):
        for message, classification in [('Should I continue?', 'yes'), ('Done.', 'no'), ('', 'uncertain'), (None, 'uncertain')]:
            self.invoke('Stop', last_assistant_message=message)
            self.assertEqual(self.events('turn-ended')[-1]['question'], classification)

    def test_boundary_examples_and_no_raw_arguments_in_state_or_journal(self):
        cases = [
            ('git -C /projects/elohim/.worktrees/x rebase --onto main feat', 'git -C <path> rebase --onto <ref> <ref>'),
            ('cd app && pnpm exec vitest run foo.spec.ts', 'cd <path> && pnpm exec vitest run <path>'),
            ('curl -s https://doorway-alpha.elohim.host/p2p/status', 'curl -s <url>'),
            ('bash -c "echo SECRET"', 'bash -c <str>'),
            ('python3 -c "print(SECRET)"', 'python3 -c <str>'),
            ('cat <<EOF\nSECRET\nEOF', 'cat << <str>'),
            ('curl --token=SECRET -H "Authorization: SECRET" 42 "*.py"', 'curl --token=<str> -H <str> <num> <glob>'),
            ('echo SECRET\ncat /private/SECRET', 'echo <str> \n cat <path>'),
        ]
        for command, expected in cases:
            self.invoke('PermissionDenied', tool_name='Bash', tool_input={'command': command})
            self.assertEqual(self.events('classifier-denied')[-1]['shape'], expected)
        self.assertNotIn('SECRET', json.dumps(self.events()) + json.dumps(self.state()))
        self.assertTrue(all('concernClass' not in e and 'effectClass' not in e for e in self.events()))

    def test_sequence_discontinuity_is_explicit(self):
        self.invoke('Stop')
        state = self.state()
        state['nextSeq'] += 4
        self.write_state(state)
        self.invoke('Stop')
        self.assertIn('sequence-discontinuity', [e['gap']['reason'] for e in self.events('observation-gap')])
        seqs = [e['seq'] for e in self.events()]
        self.assertEqual(seqs, sorted(set(seqs)))

    def test_state_loss_recovers_prompt_without_reusing_sequence(self):
        self.pre()
        self.request()
        (self.directory / 'state.json').unlink()
        self.invoke('Stop')
        self.assertEqual(self.events('prompt-closed')[0]['correlation']['incidentId'], self.events('permission-requested')[0]['correlation']['incidentId'])
        seqs = [e['seq'] for e in self.events()]
        self.assertEqual(seqs, sorted(set(seqs)))

    def test_internal_error_is_gap_and_fail_open(self):
        self.invoke('PreToolUse', tool_name='Bash', tool_use_id=['invalid'])
        self.assertEqual(self.events('observation-gap')[0]['gap']['reason'], 'adapter-error')

    def test_keeper_is_silent_and_writes_nothing(self):
        self.invoke('UserPromptSubmit', prompt='stuck?', env={'STASIS_KEEPER': '1'})
        self.assertFalse(self.journal.exists())

    def test_sensing_off_is_silent_and_writes_nothing(self):
        self.invoke('Stop', env={'STASIS_SENSE': 'off'})
        self.assertFalse(self.journal.exists())

    def test_idle_prompt_is_not_journalled(self):
        self.cancelled(3)
        self.invoke('Notification', notification_type='idle_prompt')
        self.assertFalse(self.journal.exists())

    def test_segment_rotation_preserves_order(self):
        for _ in range(5):
            self.invoke('Stop', env={'STASIS_SEGMENT_BYTES': '900'})
        self.assertGreater(len(list(self.directory.glob('events.*.jsonl'))), 1)
        self.assertEqual([e['seq'] for e in self.events()], list(range(1, 6)))
        self.assertEqual(self.state()['nextSeq'], 6)

    def test_retention_emits_gap_before_unknown_reference_closes(self):
        self.pre()
        self.request()
        # Rotate the permission request out; leave the prompt unresolved.
        self.invoke('PermissionDenied', env={'STASIS_SEGMENT_BYTES': '1'})
        segment = next(self.directory.glob('events.*.jsonl'))
        os.utime(segment, (time.time() - 31 * 86400,) * 2)
        self.invoke('Stop')
        self.assertFalse(segment.exists())
        self.assertIn('retention', [e['gap']['reason'] for e in self.events('observation-gap')])
        self.assertTrue(self.events('prompt-closed'))

    def test_concurrent_invocations_allocate_unique_sequences(self):
        with ThreadPoolExecutor(max_workers=6) as pool:
            list(pool.map(lambda _: self.invoke('Stop'), range(12)))
        self.assertEqual([e['seq'] for e in self.events()], list(range(1, 13)))

    def test_three_cancellations_coalesce_and_notice_once(self):
        self.invoke('PostToolUse')
        self.cancelled(3)
        context = self.context()
        expected = 'harness exception: `native-seam-guidance` timed out (`2100` ms against `2000` ms) ×3 since the last notice. Investigate and fix it now, or dispatch a subagent to.'
        self.assertEqual(context, expected)
        self.assertEqual([e['exception']['count'] for e in self.events('harness-exception')], [3])
        self.assertEqual(self.context(), '')
        self.assertNotIn('SECRET', json.dumps(self.state()) + json.dumps(self.events()))

    def test_doubled_count_reemits_but_partial_growth_is_silent(self):
        self.invoke('PostToolUse')
        self.cancelled(3)
        self.assertTrue(self.context())
        self.cancelled(1)
        self.assertEqual(self.context(), '')
        self.cancelled(2)
        self.assertIn('×3 since the last notice', self.context())
        self.assertEqual(sum(e['exception']['count'] for e in self.events('harness-exception')), 6)

    def test_rate_limit_expiry_emits_only_new_exceptions(self):
        self.invoke('PostToolUse')
        self.cancelled(3)
        self.context()
        state = self.state()
        for entry in state['exceptions'].values():
            entry['noticeAt'] -= 1801
        self.write_state(state)
        self.assertEqual(self.context(), '')
        self.cancelled(1)
        self.assertIn('×1 since the last notice', self.context())

    def test_queue_until_context_hook_and_event_specific_emission(self):
        self.invoke('PostToolUse')
        self.cancelled(3)
        self.invoke('Stop')
        self.assertIn('×3', self.context('PreToolUse'))
        self.cancelled(3)
        self.assertIn('×3', self.context('UserPromptSubmit'))
        self.assertEqual(self.context(), '')

    def test_first_cursor_journals_historical_exceptions_without_notifying(self):
        self.cancelled(3)
        self.invoke('PostToolUse')
        exceptions = self.events('harness-exception')
        self.assertEqual(len(exceptions), 1)
        self.assertEqual(exceptions[0]['exception']['count'], 3)
        self.assertIs(exceptions[0]['exception']['baseline'], True)
        entry = next(iter(self.state()['exceptions'].values()))
        self.assertEqual(entry['count'], 3)
        self.assertEqual(entry['notifiedCount'], 3)
        self.assertEqual(self.context(), '')

    def test_baseline_key_notifies_and_doubles_using_only_new_occurrences(self):
        self.cancelled(3)
        self.invoke('PostToolUse')
        self.cancelled()
        lines = self.context().splitlines()
        self.assertEqual(len(lines), 1)
        self.assertIn('×1 since the last notice', lines[0])
        self.assertFalse(self.events('harness-exception')[-1]['exception'].get('baseline', False))
        self.cancelled()
        self.assertIn('×1 since the last notice', self.context())
        self.cancelled()
        self.assertEqual(self.context(), '')
        self.cancelled()
        self.assertIn('×2 since the last notice', self.context())

    def test_new_key_after_baseline_notifies_once(self):
        self.cancelled(3)
        self.invoke('PostToolUse')
        self.cancelled(hookName='new-hook')
        lines = self.context().splitlines()
        self.assertEqual(len(lines), 1)
        self.assertIn('`new-hook`', lines[0])
        self.assertIn('×1 since the last notice', lines[0])
        self.assertEqual(self.context(), '')

    def test_lost_cursor_reestablishes_a_silent_baseline(self):
        self.cancelled(3)
        self.invoke('PostToolUse')
        (self.directory / 'state.json').unlink()
        self.invoke('PostToolUse')
        self.assertIs(self.events('harness-exception')[-1]['exception']['baseline'], True)
        self.assertEqual(self.events('harness-exception')[-1]['exception']['count'], 3)
        self.assertEqual(len(self.events('harness-exception')), 2)
        self.assertIn('transcript-cursor-lost', [e['gap']['reason'] for e in self.events('observation-gap')])
        self.cancelled()
        self.assertIn('×1 since the last notice', self.context())

    def test_replaced_cursor_reestablishes_a_silent_baseline(self):
        self.cancelled(3)
        self.invoke('PostToolUse')
        self.transcript.rename(self.root / 'old-transcript.jsonl')
        self.cancelled(3)
        self.invoke('PostToolUse')
        self.assertEqual(len(self.events('harness-exception')), 2)
        self.assertIs(self.events('harness-exception')[-1]['exception']['baseline'], True)
        self.assertEqual(self.events('harness-exception')[-1]['exception']['count'], 3)
        self.assertIn('transcript-discontinuity', [e['gap']['reason'] for e in self.events('observation-gap')])
        self.cancelled()
        self.assertIn('×1 since the last notice', self.context())

    def test_truncated_cursor_reestablishes_a_silent_baseline(self):
        self.cancelled(6)
        self.invoke('PostToolUse')
        self.transcript.write_text('')
        self.cancelled(3)
        self.invoke('PostToolUse')
        self.assertEqual(len(self.events('harness-exception')), 2)
        self.assertIs(self.events('harness-exception')[-1]['exception']['baseline'], True)
        self.assertEqual(self.events('harness-exception')[-1]['exception']['count'], 3)
        self.assertIn('transcript-discontinuity', [e['gap']['reason'] for e in self.events('observation-gap')])
        self.cancelled()
        self.assertIn('×1 since the last notice', self.context())

    def test_absent_transcript_path_is_gap_never_zero(self):
        self.invoke('Stop', transcript_path=None)
        self.assertIn('transcript-path-absent', [e['gap']['reason'] for e in self.events('observation-gap')])
        self.assertEqual(self.events('harness-exception'), [])
        self.assertEqual(self.state()['exceptions'], {})

    def test_unreadable_transcript_is_gap(self):
        self.invoke('Stop', transcript_path=str(self.root / 'missing'))
        self.assertIn('transcript-unreadable', [e['gap']['reason'] for e in self.events('observation-gap')])

    def test_cursor_does_not_rescan_an_existing_prefix(self):
        self.cancelled(3)
        self.context()
        cursor = next(iter(self.state()['transcripts'].values()))
        self.assertEqual(cursor['offset'], self.transcript.stat().st_size)
        # Same-length changed prefix would yield new keys if the adapter rescanned.
        original = self.transcript.read_text()
        changed = original.replace('native-seam-guidance', 'different-hook-nameX')
        self.assertEqual(len(changed), len(original))
        self.transcript.write_text(changed)
        self.assertEqual(self.context(), '')
        self.assertEqual(len(self.events('harness-exception')), 1)

    def test_transcript_truncation_and_partial_records_are_gaps(self):
        self.cancelled(3)
        self.context()
        self.transcript.write_text('')
        self.assertEqual(self.context(), '')
        self.transcript.write_text('{"attachment":')
        self.assertEqual(self.context(), '')
        reasons = [e['gap']['reason'] for e in self.events('observation-gap')]
        self.assertIn('transcript-discontinuity', reasons)
        self.assertIn('transcript-partial-record', reasons)

    def test_hook_error_and_stop_failure_records(self):
        self.invoke('PostToolUse')
        with self.transcript.open('a') as stream:
            for kind in ('hook_error', 'StopFailure'):
                stream.write(json.dumps({'type': kind, 'hookName': 'test-hook', 'command': 'python3 /private/x.py'}) + '\n')
        self.invoke('Stop')
        self.assertEqual({e['exception']['source'] for e in self.events('harness-exception')}, {'hook_error', 'stop_failure'})
        self.assertEqual(len([e for e in self.events('observation-gap') if e['gap']['reason'] == 'hook-error']), 2)
        self.assertEqual(len(self.context().splitlines()), 2)

    def test_redacted_same_shape_keeps_distinct_command_keys(self):
        self.invoke('PostToolUse')
        self.cancelled(command='python3 /one/hook.py')
        self.cancelled(command='python3 /two/hook.py')
        self.assertEqual(len(self.context().splitlines()), 2)
        self.assertEqual(len(self.events('harness-exception')), 2)

    def test_default_root_uses_project_directory(self):
        self.invoke('Stop', env={'STASIS_JOURNAL_ROOT': ''})
        self.assertTrue((self.root / '.eprfs/status/stasis/session-1/events.jsonl').exists())

    def test_invalid_payloads_and_session_paths_fail_open(self):
        for raw in ('null', '[]', 'invalid', '{}'):
            result = subprocess.run([sys.executable, str(HOOK)], input=raw, text=True, capture_output=True, env=self.env)
            self.assertEqual((result.returncode, result.stdout, result.stderr), (0, '', ''))
        self.invoke('Stop', session_id='../escape')
        self.assertFalse(self.journal.exists())

    def test_package_source_is_identical_and_registration_is_unwired(self):
        pkg = json.loads((REPO / '.epr-meta/elohim/packages/hooks/dialogue-stasis-sense.json').read_text())
        self.assertEqual(pkg['source']['body'].encode(), HOOK.read_bytes())
        self.assertIsNone(pkg['registration'])
        self.assertEqual(len(pkg['metadata']['intendedRegistrations']), 9)
        if jsonschema:
            jsonschema.validate(pkg, json.loads((REPO / 'elohim/sdk/domains/elohim-agent/schemas/hook-package.schema.json').read_text()))


if __name__ == '__main__':
    print('Event validation: jsonschema Draft 2020-12' if VALIDATOR else 'Event validation: minimal structural fallback')
    unittest.main()
