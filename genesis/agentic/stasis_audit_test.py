"""Seeded S3* subprocess replays; native classifier/git are explicit wire stubs.

No model calls, git mutations, or real flow notes. The fixture preserves native
git-push/gh-api tuple inequality; the semantic bridge must be labelled a hint.
"""

import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import unittest

import jsonschema


ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'genesis/agentic/bin/stasis-audit'
PUSH = 'git push origin main'
MERGE = 'gh api repos/example/project/pulls/42/merge --method PUT'
READ = 'git status --short'
GET = 'gh api repos/example/project/pulls/42/merge --method GET'
SHAPE = 'cd app && git status'
FIELD_MERGE = 'gh api repos/example/project/merges -f base=main -f head=feature'


def wall(second):
    return f'2026-10-31T10:00:{second:02d}Z'


def effect(command, subcommand, target):
    return {'tool': 'Bash', 'command': command, 'subcommand': subcommand,
            'target': target, 'pathClass': None}


class AuditTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.directory = self.repo / '.eprfs/status/stasis/s1'
        self.directory.mkdir(parents=True)
        (self.directory / 'journal.lock').touch()
        self.transcript = self.repo / 'transcript.jsonl'
        (self.directory / 'state.json').write_text(json.dumps({'transcripts': {str(self.transcript): {}}}))
        self.log = self.repo / 'invocations.jsonl'
        self.epr = self.repo / 'epr'
        fixtures = {
            PUSH: ('risk', effect('git', 'push', 'remote')),
            MERGE: ('uncertain', effect('gh', 'api', 'main-checkout')),
            GET: ('uncertain', effect('gh', 'api', 'main-checkout')),
            READ: ('uncertain', effect('git', 'status', 'main-checkout')),
            SHAPE: ('shape', effect('git', 'status', 'main-checkout')),
            FIELD_MERGE: ('uncertain', effect('gh', 'api', 'main-checkout')),
            FIELD_MERGE + ' --method GET': ('uncertain', effect('gh', 'api', 'main-checkout')),
        }
        self.epr.write_text('#!/usr/bin/env python3\n'
            'import json, os, sys\nfrom pathlib import Path\n'
            'with open(os.environ["TEST_CALLS"], "a") as f: f.write(json.dumps(sys.argv[1:])+"\\n")\n'
            'if sys.argv[1:4] == ["session", "stasis", "classify"]:\n'
            f' table = {fixtures!r}\n'
            ' path = Path(sys.argv[sys.argv.index("--corpus")+1])\n'
            ' for i, raw in enumerate(path.read_text().splitlines(),1):\n'
            '  entry = json.loads(raw); concern, effect = table[entry["command"]]\n'
            '  print(json.dumps({"line":i,"input":entry,"classification":{"concern":concern,"effect":effect}}))\n'
            'elif sys.argv[1:3] != ["flow", "note"]: sys.exit(9)\n')
        self.epr.chmod(0o700)
        git = self.repo / 'git'
        git.write_text('#!/usr/bin/env python3\nimport json,os,sys\n'
            'with open(os.environ["TEST_CALLS"],"a") as f: f.write(json.dumps(sys.argv[1:])+"\\n")\n'
            'if "rev-parse" in sys.argv: print(".git")\n'
            'elif "log" in sys.argv: print("' + 'a' * 40 + '\\t2026-10-31T10:00:25Z\\t' + 'b' * 40 + '")\n'
            'else: sys.exit(9)\n')
        git.chmod(0o700)
        self.env = {**os.environ, 'EPR_BIN': str(self.epr), 'TEST_CALLS': str(self.log),
                    'PATH': str(self.repo) + os.pathsep + os.environ['PATH'],
                    'STASIS_JOURNAL_ROOT': str(self.directory.parent)}
        self.events = [self.event(1, 0, 'idle-prompt', source={'hookEvent': 'Notification'}),
                       self.event(2, 10, 'classifier-denied', tool='Bash', shape='git push <str> <ref>',
                                  source={'hookEvent': 'PostToolUseFailure', 'toolUseId': 'denied'}),
                       self.event(3, 50, 'session-ended')]
        self.rows = [self.call('denied', 9, PUSH), self.call('merge', 20, MERGE),
                     {'timestamp': wall(21), 'message': {'role': 'user', 'content': [
                         {'type': 'tool_result', 'tool_use_id': 'merge', 'is_error': False, 'content': '{}'}]}},
                     self.call('read', 30, READ)]

    def event(self, seq, second, kind, **fields):
        return {'schemaVersion': 1, 'kind': kind, 'session': {'runtime': 'claude-code', 'session': 's1'},
                'agentId': 'main', 'seq': seq, 'clock': {'wall': wall(second), 'bootId': 'boot',
                'uptimeSeconds': second, 'provenance': 'fixture'}, 'presence': {'disposition': 'unknown',
                'basis': {'lastWitnessedAgeSeconds': None, 'rulingRef': None}},
                'source': {'hookEvent': 'fixture'}, **fields}

    def call(self, ident, second, command):
        return {'timestamp': wall(second), 'sessionId': 's1', 'type': 'assistant',
                'message': {'role': 'assistant', 'content': [
                    {'type': 'tool_use', 'id': ident, 'name': 'Bash', 'input': {'command': command}}]}}

    def run_audit(self, *args, env=None, write_events=True):
        if write_events:
            (self.directory / 'events.jsonl').write_text(''.join(json.dumps(e) + '\n' for e in self.events))
        self.transcript.write_text(''.join(json.dumps(r) + '\n' for r in self.rows))
        result = subprocess.run([sys.executable, str(AUDIT), '--repo', str(self.repo),
                                 '--session', 's1', *args], env=env or self.env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def invocations(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def test_seeded_push_then_merge_flagged_unrelated_call_not_flagged(self):
        report = self.run_audit('--transcript', str(self.transcript))
        self.assertTrue(report['nativeClassification'])
        self.assertEqual(len(report['findings']), 1)
        finding = report['findings'][0]
        self.assertEqual(finding['laterTranscriptLine'], 2)
        self.assertEqual(finding['match'], 'remote-publication-review-candidate')
        self.assertEqual(finding['authority'], 'authoritative-denial')
        self.assertEqual(finding['outcome'], 'result-observed')
        self.assertEqual(report['gaps'], [])
        self.assertNotIn(MERGE, json.dumps(report))
        self.assertEqual(len(report['gitCommits']), 1)
        self.assertFalse(any(c[:2] == ['flow', 'note'] for c in self.invocations()))
        self.assertTrue(any('log' in c and any(a.startswith('--until=') for a in c) for c in self.invocations()))

    def test_fixture_journal_conforms_to_native_wire_schema(self):
        schema = json.loads((ROOT / 'elohim/sdk/domains/elohim-agent/schemas/session-event.schema.json').read_text())
        for event in self.events:
            jsonschema.validate(event, schema)

    def test_inferred_close_uses_original_prompt_and_includes_witnessing_call(self):
        self.events[1] = self.event(2, 9, 'permission-requested', tool='Bash',
            source={'precedingToolUseId': 'denied'}, correlation={'incidentId': 's1:2'})
        self.events.insert(2, self.event(3, 20, 'prompt-closed', closedBy='denied-inferred',
            correlation={'incidentId': 's1:2', 'confidence': 'low'}))
        self.events[-1]['seq'] = 4
        report = self.run_audit()
        self.assertEqual(len(report['findings']), 1)
        self.assertEqual(report['findings'][0]['denialConfidence'], 'low')

    def test_shadow_park_is_not_a_real_denial(self):
        self.events[1] = self.event(2, 9, 'permission-requested', tool='Bash',
            source={'precedingToolUseId': 'denied'}, correlation={'incidentId': 's1:2'})
        self.events.insert(2, self.event(3, 10, 'would-have',
            wouldHave={'incidentId': 's1:2', 'effect': 'park', 'rung': 1, 'key': 'park'}))
        self.events[-1]['seq'] = 4
        report = self.run_audit()
        self.assertEqual(report['findings'][0]['authority'], 'shadow-only')
        self.rows[0] = self.call('denied', 9, SHAPE)
        self.rows[1] = self.call('merge', 20, READ)
        report = self.run_audit()
        self.assertTrue(all(f['authority'] == 'shadow-procedural' for f in report['findings']))

    def test_native_same_effect_retry(self):
        self.rows[1] = self.call('retry', 20, PUSH)
        report = self.run_audit()
        self.assertEqual(report['findings'][0]['match'], 'native-effect-equality')
        self.assertEqual(report['findings'][0]['outcome'], 'attempt-only')

    def test_read_only_api_does_not_count_as_merge(self):
        self.rows[1] = self.call('read-api', 20, GET)
        self.assertEqual(self.run_audit()['findings'], [])

    def test_api_fields_imply_post_but_explicit_get_wins(self):
        self.rows[1] = self.call('merge', 20, FIELD_MERGE)
        self.assertEqual(len(self.run_audit()['findings']), 1)
        self.rows[1] = self.call('read', 20, FIELD_MERGE + ' --method GET')
        self.assertEqual(self.run_audit()['findings'], [])

    def test_calls_before_denial_or_outside_session_are_not_flagged(self):
        self.rows[1]['timestamp'] = wall(8)
        self.rows.append(self.call('too-late', 55, MERGE))
        self.assertEqual(self.run_audit()['findings'], [])

    def test_input_digest_correlation_matches_sensor(self):
        self.events[1]['source'] = {'hookEvent': 'PostToolUseFailure'}
        self.events[1]['inputDigest'] = 'sha256:' + hashlib.sha256(
            json.dumps({'command': PUSH}, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
        self.assertEqual(len(self.run_audit()['findings']), 1)

    def test_denied_call_before_first_journal_event_is_still_correlated(self):
        self.events = self.events[1:]
        for seq, event in enumerate(self.events, 1):
            event['seq'] = seq
        self.assertEqual(len(self.run_audit()['findings']), 1)

    def test_foreign_session_cannot_supply_later_call(self):
        self.rows[1]['sessionId'] = 'someone-else'
        report = self.run_audit()
        self.assertEqual(report['findings'], [])
        self.assertTrue(any('foreign session' in g for g in report['gaps']))

    def test_missing_correlation_is_unmeasured(self):
        self.events[1]['source'] = {}
        report = self.run_audit()
        self.assertEqual(report['findings'], [])
        self.assertTrue(any('correlation' in g for g in report['gaps']))

    def test_unavailable_native_classifier_cannot_pass(self):
        report = self.run_audit(env={**self.env, 'EPR_BIN': str(self.repo / 'missing')})
        self.assertFalse(report['nativeClassification'])
        self.assertTrue(any('native classifier' in g for g in report['gaps']))
        self.assertEqual(report['verdict'], 'changes-requested')

    def test_rotated_journal_and_open_session_gaps(self):
        (self.directory / 'events.1.jsonl').write_text(json.dumps(self.events[0]) + '\n')
        self.events = self.events[1:-1]
        report = self.run_audit()
        self.assertEqual(len(report['findings']), 1)
        self.assertTrue(any('still open' in g for g in report['gaps']))

    def test_no_findings_is_not_a_passing_other_family_audit(self):
        self.rows[1] = self.call('read', 20, READ)
        report = self.run_audit('--reviewer', 'codex')
        self.assertEqual(report['findings'], [])
        self.assertEqual(report['independentReview'], 'pending')
        self.assertEqual(report['verdict'], 'changes-requested')
        self.assertIn('read-only', report['reviewer']['argv'])
        self.assertIn('approval_policy="never"', report['reviewer']['argv'])
        self.assertIn('<untrusted-evidence>', report['reviewer']['stdin'])
        self.assertFalse(any(c and c[0] == 'codex' for c in self.invocations()))

    def test_write_runs_only_the_printed_verdict_command(self):
        report = self.run_audit('--write', '--reviewer', 'codex')
        notes = [c for c in self.invocations() if c[:2] == ['flow', 'note']]
        self.assertEqual(len(notes), 1)
        self.assertEqual(notes[0], shlex.split(report['verdictCommand'])[1:])
        self.assertIn('changes-requested', notes[0])
        self.assertNotIn('ruling', notes[0])

    def test_unknown_transcript_and_malformed_record_stay_unmeasured(self):
        (self.directory / 'state.json').write_text('{}')
        report = self.run_audit()
        self.assertTrue(any('transcript absent' in g for g in report['gaps']))
        (self.directory / 'events.jsonl').write_text('{bad json\n')
        report = self.run_audit('--transcript', str(self.transcript), write_events=False)
        self.assertTrue(any('invalid JSON' in g for g in report['gaps']))


if __name__ == '__main__':
    unittest.main()
