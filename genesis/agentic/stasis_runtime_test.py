"""Runtime boundary replays; native authority and keeper processes are fixtures."""
import json
import subprocess
import sys
import unittest

from genesis.agentic import stasis_control_test as base
from genesis.agentic.stasis_coordinator_test import coordinator, EVENT_VALIDATOR


class RuntimeTest(unittest.TestCase):
    setUp = base.ControlTest.setUp
    guide = base.ControlTest.guide
    rows = base.ControlTest.rows

    def invoke(self, event, extra=(), **fields):
        fields.setdefault('turn_id', 'turn-1')
        return base.ControlTest.invoke(self, event, extra=('--runtime=codex', *extra), **fields)

    def events(self):
        path = self.directory / 'events.jsonl'
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        for row in rows:
            EVENT_VALIDATOR.validate(row)
            self.assertEqual(row['session']['runtime'], 'codex')
        return rows

    def test_codex_correlation_is_scoped_to_turn_and_redacted(self):
        fields = dict(tool_name='Bash', tool_input={'command': 'git show private-secret-ref'},
                      tool_use_id='call-1')
        self.invoke('PreToolUse', **fields)
        self.request_with_fields(fields)
        opening = next(e for e in self.events() if e['kind'] == 'permission-requested')
        self.assertEqual(opening['source']['promptId'], 'turn-1')
        self.assertEqual(opening['source']['precedingToolUseId'], 'call-1')
        self.assertNotIn('private-secret-ref', (self.directory / 'events.jsonl').read_text())
        self.assertEqual(opening['presence']['disposition'], 'unknown')

    def request_with_fields(self, fields):
        result = self.invoke('PermissionRequest', **fields)
        # Unprobed Codex control must not deny or approve a prompt.
        self.assertEqual(result.stdout, '')

    def test_codex_does_not_infer_human_denial_or_presence(self):
        fields = dict(tool_name='Bash', tool_input={'command': 'git status'}, tool_use_id='c1')
        self.invoke('PreToolUse', **fields)
        self.request_with_fields(fields)
        self.invoke('UserPromptSubmit', prompt='A system continuation can look like a user prompt')
        closed = [r for r in self.events() if r['kind'] == 'prompt-closed']
        self.assertEqual(closed[-1]['closedBy'], 'unknown')
        self.assertTrue(all(e['presence']['disposition'] == 'unknown' for e in self.events()))

    def test_codex_guidance_delivered_once_to_matching_reader(self):
        self.guide()
        self.guide('child')
        result = self.invoke('PostToolUse', tool_name='Bash', tool_use_id='c1')
        output = json.loads(result.stdout)['hookSpecificOutput']
        self.assertEqual(output['hookEventName'], 'PostToolUse')
        self.assertIn('advice, never authority', output['additionalContext'])
        self.assertEqual(self.invoke('PostToolUse').stdout, '')
        self.assertEqual(len(self.rows('receipt')), 1)
        self.assertIn('guidance', self.invoke('PostToolUse', agent_id='child').stdout)
        self.assertEqual(len(self.rows('receipt')), 2)

    def test_codex_policy_mismatch_preserves_undelivered_advice(self):
        self.guide()
        self.statusfile.write_text(json.dumps(base.status(policy='mismatch')))
        self.assertEqual(self.invoke('PostToolUse').stdout, '')
        self.assertEqual(self.rows('receipt'), [])

    def test_codex_recursion_guard_runs_before_reading_input(self):
        result = subprocess.run([sys.executable, str(base.HOOK), '--runtime=codex'],
                                input='invalid json', capture_output=True, text=True,
                                env={**self.env, 'STASIS_KEEPER': '1'}, timeout=2)
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, '', ''))
        self.assertFalse(self.directory.exists())

    def test_codex_stop_and_session_end_are_foldable(self):
        self.invoke('Stop', last_assistant_message='Which branch should receive this change?')
        self.invoke('SessionEnd')
        self.assertTrue(any(e['kind'] == 'turn-ended' for e in self.events()))
        self.assertEqual(self.events()[-1]['kind'], 'session-ended')

    def test_codex_usage_counts_input_and_output_without_double_counting_cache(self):
        raw = json.dumps({'type': 'turn.completed', 'usage': {
            'input_tokens': 100, 'cached_input_tokens': 80, 'output_tokens': 20}})
        self.assertEqual(coordinator.codex_tokens(raw), 120)
        self.assertIsNone(coordinator.codex_tokens('{"type":"turn.failed"}'))
        self.assertIsNone(coordinator.codex_tokens('{"type":"turn.completed","usage":{}}'))
        self.assertIsNone(coordinator.codex_tokens('not json'))
        self.assertIsNone(coordinator.codex_tokens('{"type":"turn.completed","usage":null}'))

    def test_codex_cannot_borrow_claude_idle_wake_capability(self):
        self.guide()
        self.assertEqual(self.invoke('Stop', extra=('--rewake',)).stdout, '')
        self.assertEqual(self.rows('wake'), [])

    def test_codex_notification_names_dialogue_delivery(self):
        self.guide(kind='PushNotification')
        output = self.invoke('PostToolUse').stdout
        self.assertIn('notify the steward in the session dialogue', output)
        self.assertNotIn('request PushNotification', output)


if __name__ == '__main__':
    unittest.main()
