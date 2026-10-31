"""Slice 9 replay: native authority, keepers and daemon startup are all stubs."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from genesis.agentic import stasis_control as control
from genesis.agentic import stasis_coordinator_test as base

ROOT = control.REPO
HOOK = ROOT / '.claude/hooks/dialogue-stasis-sense.py'
CID = 'bafkreipinnedfixture'
STUB = '''#!/usr/bin/env python3
import json, os, sys, time, subprocess
from pathlib import Path
args = sys.argv[1:]
with open(os.environ['NATIVE_CALLS'], 'a') as stream:
    stream.write(json.dumps(args) + '\\n')
verb = args[2]
if os.environ.get('NATIVE_HANG') == verb:
    subprocess.Popen([sys.executable, '-c', "import time; from pathlib import Path; time.sleep(2.3); Path(" + repr(os.environ['ESCAPED']) + ").touch()"])
    time.sleep(10)
if verb == 'status':
    if os.environ.get('STATUS_EXIT'):
        sys.exit(int(os.environ['STATUS_EXIT']))
    print(Path(os.environ['STATUS_FIXTURE']).read_text())
elif verb == 'decide':
    payload = json.load(sys.stdin)
    answer = json.loads(os.environ.get('DECISION', '{}'))
    answer.setdefault('incidentId', payload.get('correlation', {}).get('incidentId'))
    print(json.dumps(answer))
    sys.exit(int(os.environ.get('DECIDE_EXIT', '0')))
elif verb == 'classify':
    json.load(sys.stdin)
    print(os.environ['CLASSIFICATION'])
elif verb == 'fold':
    path = os.environ['SHIFT_FIXTURE'] if '--shift-current' in args else os.environ['FOLD_FIXTURE']
    print(Path(path).read_text())
else:
    sys.exit(1)
'''


def status(disposition='automated', policy='pinned-match', incidents=None):
    return {'policyState': policy, 'policyCid': CID,
            'sessions': [{'session': 's1', 'openIncidents': incidents or [],
                          'presence': {'disposition': disposition,
                                       'lastWitnessedAgeSeconds': 700 if disposition == 'automated' else 0,
                                       'rulingRef': None}}]}


class ControlTest(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix='stasis-control-')
        self.addCleanup(temp.cleanup)
        self.repo = Path(temp.name)
        self.root = self.repo / 'journals'
        self.directory = self.root / 's1'
        self.transcript = self.repo / 'transcript.jsonl'
        self.transcript.touch()
        self.policy = self.repo / control.POLICY
        self.policy.parent.mkdir(parents=True)
        self.policy.write_bytes((ROOT / control.POLICY).read_bytes())
        self.epr = self.repo / 'epr'
        self.epr.write_text(STUB)
        self.epr.chmod(0o700)
        self.daemon = self.repo / 'coordinator'
        self.daemon.write_text('#!/usr/bin/env python3\nimport os,time,sys\n'
                               'time.sleep(float(os.environ.get("START_SLEEP", "0")))\n'
                               'sys.exit(int(os.environ.get("START_EXIT", "0")))\n')
        self.daemon.chmod(0o700)
        self.statusfile = self.repo / 'status.json'
        self.statusfile.write_text(json.dumps(status()))
        clean = {k: v for k, v in os.environ.items() if not k.startswith('STASIS_')}
        self.env = {**clean, 'CLAUDE_PROJECT_DIR': str(self.repo), 'EPR_BIN': str(self.epr),
                    'STASIS_JOURNAL_ROOT': str(self.root), 'STASIS_LADDER': 'live',
                    'STASIS_COORDINATOR_BIN': str(self.daemon), 'STATUS_FIXTURE': str(self.statusfile),
                    'NATIVE_CALLS': str(self.repo / 'calls'), 'ESCAPED': str(self.repo / 'escaped'),
                    'DECISION': json.dumps({'effect': 'deny', 'message': 'park', 'policyCid': CID, 'shadow': False})}

    def invoke(self, event, extra=(), **fields):
        payload = {'session_id': 's1', 'hook_event_name': event,
                   'transcript_path': str(self.transcript), **fields}
        started = time.monotonic()
        result = subprocess.run([sys.executable, str(HOOK), *extra], input=json.dumps(payload),
                                env=self.env, capture_output=True, text=True, timeout=4)
        self.elapsed = time.monotonic() - started
        self.assertIn(result.returncode, [0, 2], result.stderr)
        return result

    def request(self):
        return self.invoke('PermissionRequest', tool_name='Bash', tool_input={'command': 'git status'})

    def rows(self, kind=None):
        with control.delivery_lock(self.directory) as journal:
            return [r for r in journal.rows if not kind or r['kind'] == kind]

    def guide(self, reader='main', kind='guidance'):
        self.directory.mkdir(parents=True, exist_ok=True)
        with control.delivery_lock(self.directory) as journal:
            journal.add(kind, kind + reader, incidentId='s1:1', policyCid=CID, agentId=reader,
                        message='Continue independent authorized work.')

    def fixture_git(self, *args, input=None, author_time=None):
        # Plumbing builds disposable fixtures without add/commit/checkout commands.
        env = {k: v for k, v in self.env.items() if not k.startswith('GIT_')}
        env.update(GIT_AUTHOR_NAME='Fixture', GIT_AUTHOR_EMAIL='fixture@example.invalid',
                   GIT_COMMITTER_NAME='Fixture', GIT_COMMITTER_EMAIL='fixture@example.invalid',
                   GIT_AUTHOR_DATE=f'{author_time or int(time.time()) + 2} +0000',
                   GIT_COMMITTER_DATE=f'{int(time.time()) + 2} +0000')
        result = subprocess.run(['git', *args], cwd=self.checkout, env=env, input=input,
                                text=True, capture_output=True, check=True)
        return result.stdout.strip()

    def completion_fixture(self, author_time=None):
        self.checkout = self.repo / 'session-checkout'
        self.checkout.mkdir()
        self.fixture_git('init', '-q')
        (self.checkout / 'work.py').write_text('done\n')
        blob = self.fixture_git('hash-object', '-w', '--stdin', input='done\n')
        tree = self.fixture_git('mktree', input=f'100644 blob {blob}\twork.py\n')
        self.invoke('UserPromptSubmit', prompt='status?')  # First journal event precedes commit.
        self.head = self.fixture_git('commit-tree', tree, input='fixture\n', author_time=author_time)
        self.fixture_git('update-ref', 'HEAD', self.head)
        self.fixture_git('read-tree', self.head)
        self.gatefile = self.write_gate()

    def write_gate(self, exit=0, name='first', at='2026-10-31T00:00:00Z',
                   sha=None, closed=True, deferred=False):
        runs = self.checkout / '.eprfs/status/gate-live/runs'
        runs.mkdir(parents=True, exist_ok=True)
        rows = [{'kind': 'run-open', 'at': at, 'candidate': {'sha': sha or self.head}},
                {'kind': 'gate-verdict', 'project': 'fixture', 'verdict': {'exit': exit, 'deferred': deferred}}]
        if closed:
            rows.append({'kind': 'run-close', 'outcome': 'passed' if exit == 0 else 'failed'})
        path = runs / (name + '.jsonl')
        path.write_text(''.join(json.dumps(row) + '\n' for row in rows))
        return path

    def stop_events(self, kind='turn-ended'):
        events = [json.loads(line) for line in (self.directory / 'events.jsonl').read_text().splitlines()]
        for event in events:
            base.EVENT_VALIDATOR.validate(event)
        return [event for event in events if event['kind'] == kind]

    def stop(self, message='Done.', **fields):
        return self.invoke('Stop', cwd=str(self.checkout), last_assistant_message=message, **fields)

    def test_accomplished_real_checkout_in_shadow_and_live(self):
        self.completion_fixture()
        for ladder in ['shadow', 'live']:
            self.env['STASIS_LADDER'] = ladder
            self.assertEqual(self.stop().stdout, '')
            event = self.stop_events()[-1]
            self.assertEqual(event['outcome'], 'accomplished')
            self.assertEqual(event['correlation']['confidence'], 'high')
            self.assertLess(self.elapsed, 3)

    def test_dirty_files_block_once_then_refer_across_hook_restarts(self):
        self.completion_fixture()
        (self.checkout / 'work.py').write_text('unfinished\n')
        first = json.loads(self.stop().stdout)
        self.assertEqual(first['decision'], 'block')
        self.assertIn('uncommitted: 1 files: work.py', first['reason'])
        self.assertIn('Next authorized step', first['reason'])
        self.assertEqual(self.stop(stop_hook_active=True).stdout, '')
        self.assertEqual(self.stop().stdout, '')
        self.assertEqual(len(self.rows('Stop-block')), 1)
        self.assertTrue(all(e['outcome'] == 'stalled' for e in self.stop_events()))
        self.assertTrue(all(e['wouldHave']['effect'] == 'Refer' for e in self.stop_events('would-have')))
        self.assertEqual(self.stop_events('would-have')[-1]['wouldHave']['incidentId'],
                         f"s1:{self.stop_events()[-1]['seq']}")

    def test_shadow_dirty_only_journals_and_collaborative_does_not_block(self):
        self.completion_fixture()
        (self.checkout / 'work.py').write_text('unfinished\n')
        for ladder, disposition in [('shadow', 'automated'), ('live', 'collaborative'), ('live', 'unknown')]:
            self.env['STASIS_LADDER'] = ladder
            self.statusfile.write_text(json.dumps(status(disposition)))
            self.assertEqual(self.stop().stdout, '')
            self.assertEqual(self.stop_events()[-1]['outcome'], 'stalled')
        self.assertFalse(self.rows('Stop-block'))

    def test_newest_red_gate_vetoes_older_green_and_names_project(self):
        self.completion_fixture()
        self.write_gate(exit=1, name='newer', at='2026-10-31T00:01:00Z')
        self.assertIn('gate: red on fixture', self.stop().stdout)
        self.assertEqual(self.stop_events()[-1]['outcome'], 'stalled')

    def test_linked_checkout_uses_common_gate_root_and_own_dirty_files(self):
        self.completion_fixture()
        main = self.checkout
        linked = self.repo / 'linked-checkout'
        linked.mkdir()
        admin = main / '.git/worktrees/fixture'
        admin.mkdir(parents=True)
        # Linked-worktree metadata fixture, without a checkout command.
        (linked / '.git').write_text(f'gitdir: {admin}\n')
        (admin / 'commondir').write_text('../..\n')
        (admin / 'gitdir').write_text(str(linked / '.git') + '\n')
        (admin / 'HEAD').write_text(self.head + '\n')
        (admin / 'index').write_bytes((main / '.git/index').read_bytes())
        (linked / 'work.py').write_bytes((main / 'work.py').read_bytes())
        self.checkout = linked
        self.assertEqual(self.fixture_git('rev-parse', '--show-toplevel'), str(linked))
        (main / 'work.py').write_text('other session work\n')
        self.assertEqual(self.stop().stdout, '')
        self.assertEqual(self.stop_events()[-1]['outcome'], 'accomplished')
        (linked / 'work.py').write_text('this session work\n')
        self.assertIn('work.py', self.stop().stdout)
        self.assertEqual(self.stop_events()[-1]['outcome'], 'stalled')

    def test_newest_unfinished_deferred_or_wrong_head_is_not_green(self):
        self.completion_fixture()
        self.env['STASIS_LADDER'] = 'shadow'
        for options in [{'closed': False}, {'deferred': True}]:
            self.write_gate(name='newer', at='2026-10-31T00:01:00Z', **options)
            self.stop()
            self.assertEqual(self.stop_events()[-1]['outcome'], 'stalled')
        (self.gatefile.parent / 'newer.jsonl').unlink()
        self.write_gate(sha='0' * 40)
        self.stop()
        self.assertIn('no run for', self.stop_events()[-1]['stallReason'])

    def test_unreadable_gate_never_accomplished(self):
        self.completion_fixture()
        self.gatefile.unlink()
        self.gatefile.mkdir()  # EISDIR is deterministic even under root.
        self.stop()
        event = self.stop_events()[-1]
        self.assertEqual(event['outcome'], 'stalled')
        self.assertEqual(event['correlation']['confidence'], 'low')
        self.assertIn('gate: unreadable', event['stallReason'])

    def test_missing_session_commit_is_stalled_even_when_clean_and_green(self):
        self.completion_fixture(author_time=1_700_000_000)
        self.stop()
        self.assertIn('no session commit', self.stop_events()[-1]['stallReason'])

    def test_recorded_commit_is_evidence_but_prose_is_not(self):
        self.completion_fixture(author_time=1_700_000_000)
        spec = importlib.util.spec_from_file_location('stop_sensor', HOOK)
        sensor = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(sensor)
        with control.delivery_lock(self.directory):
            journal = sensor.Journal(self.directory, {'session_id': 's1'})
            journal.append('would-have', wouldHave={'effect': 'commit-observed', 'incidentId': None,
                           'rung': None, 'key': 'commit-observed', 'commit': self.head})
            journal.save()
        self.assertEqual(self.stop().stdout, '')
        self.assertEqual(self.stop_events()[-1]['outcome'], 'accomplished')

    def test_question_and_pending_human_ask_pass_through(self):
        self.completion_fixture()
        (self.checkout / 'work.py').write_text('unfinished\n')
        self.assertEqual(self.stop('Which option?').stdout, '')
        self.assertEqual(self.stop_events()[-1]['outcome'], 'question')
        with control.delivery_lock(self.directory) as delivery:
            delivery.park('s1:human', CID)
            delivery.add('human', 'human:s1:human', incidentId='s1:human', policyCid=CID)
        self.assertEqual(self.stop().stdout, '')
        self.assertEqual(self.stop_events()[-1]['outcome'], 'exception')

    def test_rung_five_question_passes_through(self):
        self.completion_fixture()
        self.stop('Which option?')
        previous = self.stop_events()[-1]
        spec = importlib.util.spec_from_file_location('stop_sensor', HOOK)
        sensor = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(sensor)
        with control.delivery_lock(self.directory):
            journal = sensor.Journal(self.directory, {'session_id': 's1'})
            journal.append('would-have', wouldHave={'effect': 'incident-advancement',
                           'incidentId': f"s1:{previous['seq']}", 'rung': 5, 'key': 'rung5'})
            journal.save()
        self.assertEqual(self.stop().stdout, '')
        self.assertEqual(self.stop_events()[-1]['outcome'], 'exception')
        self.assertEqual(self.stop().stdout, '')
        self.assertEqual(self.stop_events()[-1]['outcome'], 'exception')
        self.invoke('UserPromptSubmit', prompt='Use the first option.')
        (self.checkout / 'work.py').write_text('still unfinished\n')
        self.stop()
        self.assertEqual(self.stop_events()[-1]['outcome'], 'stalled')

    def test_stop_git_budget_is_shared_and_timeout_is_low_confidence(self):
        self.completion_fixture()
        calls = []
        def slow_git(args, **kwargs):
            calls.append(kwargs['timeout'])
            time.sleep(.3)
            if 'rev-parse' in args:
                return 0, f'{self.checkout}\n{self.checkout / ".git"}\n{self.head}\n'
            return None, ''
        started = time.monotonic()
        with patch.object(control, 'bounded', side_effect=slow_git):
            reasons, low = control.checkout_evidence(str(self.checkout), [], {'wall': '2026-10-31T00:00:00Z'})
        self.assertTrue(low)
        self.assertIn('git: unavailable', reasons[-1])
        self.assertLess(calls[1], calls[0] - .25)
        self.assertLess(time.monotonic() - started, 1)

    def test_tooling_ledgers_ignored_but_renaming_work_into_them_is_dirty(self):
        self.completion_fixture()
        for path in ['.claude/data/fixture.json', '.eprfs/status/fixture.json']:
            target = self.checkout / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text('original\n')
            blob = self.fixture_git('hash-object', '-w', '--stdin', input='original\n')
            self.fixture_git('update-index', '--add', '--cacheinfo', f'100644,{blob},{path}')
        tree = self.fixture_git('write-tree')
        self.head = self.fixture_git('commit-tree', tree, '-p', self.head, input='ledger fixtures\n')
        self.fixture_git('update-ref', 'HEAD', self.head)
        self.write_gate()
        for path in ['.claude/data/fixture.json', '.eprfs/status/fixture.json']:
            (self.checkout / path).write_text('changed\n')
        self.assertEqual(self.stop().stdout, '')
        self.assertEqual(self.stop_events()[-1]['outcome'], 'accomplished')
        self.fixture_git('mv', 'work.py', '.claude/data/work.py')
        self.assertIn('work.py', self.stop().stdout)
        self.assertEqual(self.stop_events()[-1]['outcome'], 'stalled')

    def test_head_can_carry_ancestor_session_commit(self):
        self.completion_fixture()
        tree = self.fixture_git('rev-parse', 'HEAD^{tree}')
        self.head = self.fixture_git('commit-tree', tree, '-p', self.head, input='older author time\n',
                                     author_time=1_700_000_000)
        self.fixture_git('update-ref', 'HEAD', self.head)
        self.write_gate()
        self.assertEqual(self.stop().stdout, '')
        self.assertEqual(self.stop_events()[-1]['outcome'], 'accomplished')

    def test_git_failure_real_stop_path_times_out_low(self):
        self.completion_fixture()
        bin_dir = self.repo / 'slow-bin'
        bin_dir.mkdir()
        git = bin_dir / 'git'
        git.write_text('#!/usr/bin/env python3\nimport time\ntime.sleep(10)\n')
        git.chmod(0o700)
        self.env['PATH'] = str(bin_dir) + os.pathsep + self.env['PATH']
        self.stop()
        event = self.stop_events()[-1]
        self.assertEqual(event['outcome'], 'stalled')
        self.assertEqual(event['correlation']['confidence'], 'low')
        self.assertIn('git: unavailable', event['stallReason'])
        self.assertLess(self.elapsed, 3)

    def test_stop_status_timeout_fits_registered_timeout(self):
        self.completion_fixture()
        (self.checkout / 'work.py').write_text('unfinished\n')
        self.env['NATIVE_HANG'] = 'status'
        self.assertEqual(self.stop().stdout, '')
        self.assertLess(self.elapsed, 3)

    def test_automated_fixed_deny_and_separate_park_lifecycle(self):
        result = self.request()
        out = json.loads(result.stdout)['hookSpecificOutput']
        self.assertEqual(out, {'hookEventName': 'PermissionRequest', 'decision': {
            'behavior': 'deny', 'message': control.PARK.format('s1:1')}})
        self.assertEqual(len(self.rows('park')), 1)
        events = [json.loads(x) for x in (self.directory / 'events.jsonl').read_text().splitlines()]
        self.assertEqual(self.rows('stall-closed')[0]['closedBy'], 'parked')
        self.assertEqual(events[-1]['gap']['reason'], 'native-park-fold-pending')
        for event in events:
            base.EVENT_VALIDATOR.validate(event)
        self.assertEqual(self.invoke('Stop', last_assistant_message='All independent work is done.').returncode, 0)
        with control.delivery_lock(self.directory) as journal:
            journal.add('park-state', 'answer:s1:1', incidentId='s1:1', state='answered')
            self.assertFalse(journal.pending())
        # Answering retires the ask, but supplies no completion evidence or execution grant.
        result = self.invoke('Stop', last_assistant_message='Done.')
        self.assertEqual(json.loads(result.stdout)['decision'], 'block')
        self.assertEqual(self.stop_events()[-1]['outcome'], 'stalled')

    def test_collaborative_unknown_and_away_use_native_presence(self):
        for disposition in ['collaborative', 'unknown', 'automated']:
            with self.subTest(disposition=disposition):
                s = status(disposition)
                if disposition == 'automated':
                    s['sessions'][0]['presence'].update(lastWitnessedAgeSeconds=0, rulingRef='away-ruling')
                self.statusfile.write_text(json.dumps(s))
                self.assertEqual(bool(self.request().stdout), disposition == 'automated')

    def test_inert_missing_mismatched_nonzero_and_non_deny(self):
        for code in ['3', '1', '17']:
            self.env['DECIDE_EXIT'] = code
            self.assertEqual(self.request().stdout, '')
        self.env.pop('DECIDE_EXIT')
        for state in ['absent', 'unpinned', 'pinned-mismatch', 'invalid']:
            self.statusfile.write_text(json.dumps(status(policy=state)))
            self.assertEqual(self.request().stdout, '')
        self.statusfile.write_text(json.dumps(status()))
        for answer in [{'effect': 'none'}, {'effect': 'context', 'message': 'try'},
                       {'effect': 'deny', 'message': 'park', 'shadow': True},
                       {'effect': 'deny', 'message': '', 'shadow': False},
                       {'effect': 'deny', 'message': 'park', 'shadow': False, 'policyCid': 'wrong'}]:
            self.env['DECISION'] = json.dumps(answer)
            self.assertEqual(self.request().stdout, '')
        self.assertFalse((self.directory / 'delivery.jsonl').exists())

    def test_decide_deadline_kills_descendants(self):
        self.env['NATIVE_HANG'] = 'decide'
        self.assertEqual(self.request().stdout, '')
        self.assertLess(self.elapsed, 2.2)
        time.sleep(.5)
        self.assertFalse((self.repo / 'escaped').exists())

    def test_rung_zero_only_audited_shape_and_never_ask_or_risk(self):
        policy = json.loads(self.policy.read_text())
        effect = {'tool': 'Bash', 'command': 'git', 'subcommand': 'status', 'target': 'worktree'}
        policy['learnedForms'] = [{'name': 'git-C', 'tool': 'Bash', 'shape': 'cd <path> && git status',
                                  'effectClass': effect, 'legibleForm': 'git -C <path> status',
                                  'status': 'active', 'auditVerdict': 'approved',
                                  'auditRef': 'audit-1', 'rulingRef': 'ruling-1'}]
        self.policy.write_text(json.dumps(policy))
        for concern, rule in [('risk', 'ask'), ('uncertain', 'unparseable'), ('shape', 'ask'), ('shape', 'none')]:
            self.env['CLASSIFICATION'] = json.dumps({'classification': {
                'concern': concern, 'ruleMatch': rule, 'effect': effect}})
            out = self.invoke('PreToolUse', tool_name='Bash', tool_input={'command': 'cd app && git status'}).stdout
            self.assertEqual(bool(out), concern == 'shape' and rule == 'none')
            if out:
                self.assertEqual(json.loads(out)['hookSpecificOutput']['permissionDecision'], 'deny')
                self.assertIn('git-C', out)
        policy['learnedForms'][0]['status'] = 'proposed'
        self.policy.write_text(json.dumps(policy))
        self.assertEqual(self.invoke('PreToolUse', tool_name='Bash', tool_input={'command': 'cd app && git status'}).stdout, '')

    def test_guidance_is_per_reader_and_mismatch_never_consumes_it(self):
        self.guide()
        self.guide('child')
        self.env['STASIS_LADDER'] = 'shadow'
        self.assertEqual(self.invoke('PostToolUse').stdout, '')
        self.env['STASIS_LADDER'] = 'live'
        self.statusfile.write_text(json.dumps(status(policy='pinned-mismatch')))
        self.assertEqual(self.invoke('PostToolUse').stdout, '')
        self.assertFalse(self.rows('receipt'))
        self.statusfile.write_text(json.dumps(status()))
        self.assertIn('advice, never authority', self.invoke('PostToolUse').stdout)
        self.assertEqual(self.invoke('PostToolUse').stdout, '')
        self.assertIn('advice, never authority', self.invoke('UserPromptSubmit', agent_id='child', prompt='<task-notification>x').stdout)
        self.assertEqual(len(self.rows('receipt')), 2)

    def test_batch_once_after_restart_and_never_recursive(self):
        self.request()
        self.request()
        first = self.invoke('Stop', last_assistant_message='Independent work complete.')
        self.assertEqual(json.loads(first.stdout)['decision'], 'block')
        self.assertIn('ONE AskUserQuestion', first.stdout)
        self.assertEqual(len(self.rows('batch')), 1)
        self.assertEqual(len(self.rows('batch')[0]['incidents']), 2)
        self.assertEqual(self.invoke('Stop', last_assistant_message='Independent work complete.').stdout, '')
        self.request()
        self.assertEqual(self.invoke('Stop', stop_hook_active=True, last_assistant_message='Done.').stdout, '')
        self.assertEqual(self.invoke('Stop').stdout, '')  # uncertain question never blocks
        self.assertEqual(self.invoke('Stop', last_assistant_message='Done.').stdout, '')
        self.assertEqual(self.stop_events()[-1]['outcome'], 'exception')

    def test_answer_closes_only_exact_park_and_never_authorizes_it(self):
        self.request()
        with control.delivery_lock(self.directory) as journal:
            journal.park('s1:10', CID)
        question = {'questions': [{'question': 'Resolve s1:10?'}]}
        self.invoke('PostToolUse', tool_name='AskUserQuestion', tool_input=question,
                    tool_response={'error': 'cancelled'})
        self.assertFalse(self.rows('park-state'))
        self.invoke('PostToolUse', tool_name='AskUserQuestion', tool_input=question,
                    tool_response={'answers': {'Resolve s1:10?': 'Keep parked'}})
        self.assertEqual([r['incidentId'] for r in self.rows('park-state')], ['s1:10'])
        with control.delivery_lock(self.directory) as journal:
            self.assertEqual([r['incidentId'] for r in journal.pending()], ['s1:1'])

    def test_contended_journal_fails_open_without_native_call(self):
        self.directory.mkdir(parents=True)
        with control.delivery_lock(self.directory):
            self.assertEqual(self.request().stdout, '')
            self.assertLess(self.elapsed, .5)
        self.assertFalse((self.repo / 'calls').exists())

    def test_sensing_off_has_no_io_even_with_bad_input(self):
        self.env['STASIS_SENSE'] = 'off'
        result = subprocess.run([sys.executable, str(HOOK)], input='bad input', env=self.env,
                                capture_output=True, text=True, timeout=1)
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, '', ''))
        self.assertFalse(self.root.exists())

    def test_ladder_off_byte_identical_slice3_golden(self):
        fixture = json.loads((ROOT / 'genesis/agentic/fixtures/stasis-slice3-output.json').read_text())
        self.env['STASIS_LADDER'] = 'off'
        for step in fixture['steps']:
            if step.get('exceptions'):
                with self.transcript.open('a') as stream:
                    for _ in range(step['exceptions']):
                        stream.write(json.dumps({'attachment': {'type': 'hook_cancelled', 'hookName': 'fixture-hook',
                                                                'durationMs': 2100, 'timeoutMs': 2000}}) + '\n')
            result = self.invoke(step['event'], prompt_id='p1', **step.get('fields', {}))
            self.assertEqual(result.stdout.encode(), step['stdout'].encode(), step['event'])
        self.assertTrue((self.directory / 'events.jsonl').is_file())
        self.assertFalse((self.repo / 'calls').exists())

    def test_wiring_patch_preserves_all_existing_settings(self):
        settings = json.loads((ROOT / '.claude/settings.json').read_text())
        original = copy.deepcopy(settings)
        patch_rows = json.loads((ROOT / 'genesis/agentic/stasis-settings.patch.json').read_text())
        regs = json.loads((ROOT / '.epr-meta/elohim/packages/hooks/dialogue-stasis-sense.json').read_text())['metadata']['intendedRegistrations']
        for op in patch_rows:
            self.assertEqual(op['op'], 'add')
            path = op['path'].split('/')[1:]
            self.assertEqual(path[0], 'hooks')
            if path[-1] == '-':
                settings['hooks'][path[1]].append(op['value'])
            else:
                self.assertNotIn(path[1], settings['hooks'])
                settings['hooks'][path[1]] = op['value']
        for event, entries in original['hooks'].items():
            self.assertEqual(settings['hooks'][event][:len(entries)], entries)
        self.assertEqual(settings['permissions'], original['permissions'])
        additions = [entry for event, entries in settings['hooks'].items()
                     for entry in entries[len(original['hooks'].get(event, [])):]]
        self.assertEqual(len(additions), len(regs))
        self.assertEqual(len([entry for entry in additions if entry['hooks'][0].get('asyncRewake')]), 1)

    def test_session_start_states_and_deadline_fail_open(self):
        for ladder, pin, expected in [('off', 'absent', 'off'), ('shadow', 'unpinned', 'shadow'),
                                      ('live', 'pinned-match', 'live'), ('live', 'pinned-mismatch', 'shadow')]:
            self.env['STASIS_LADDER'] = ladder
            self.statusfile.write_text(json.dumps(status(policy=pin, incidents=['one', 'two'])))
            line = json.loads(self.invoke('SessionStart').stdout)['hookSpecificOutput']['additionalContext']
            self.assertTrue(line.startswith('dialogue-stasis: ' + expected), line)
            self.assertIn('coordinator alive · 2 open incidents', line)
            self.assertEqual(len(line.splitlines()), 1)
            self.assertLess(self.elapsed, 1)
        self.env.update(START_SLEEP='5', NATIVE_HANG='status')
        line = self.invoke('SessionStart').stdout
        self.assertIn('coordinator unknown', line)
        self.assertIn('unknown open incidents', line)
        self.assertLess(self.elapsed, 1)

    def test_idle_wakes_once_and_does_not_consume_context(self):
        self.env['STASIS_REWAKE_SECONDS'] = '.3'
        self.invoke('Stop', last_assistant_message='Waiting?')
        self.guide()
        wake = self.invoke('Stop', extra=('--rewake',), last_assistant_message='Waiting?')
        self.assertEqual(wake.returncode, 2)
        self.assertIn('Stasis guidance is ready', wake.stderr)
        self.assertFalse(self.rows('receipt'))
        self.assertEqual(self.invoke('Stop', extra=('--rewake',), last_assistant_message='Waiting?').returncode, 0)
        self.assertIn('advice, never authority', self.invoke('PostToolUse').stdout)

    def test_wake_unsupported_and_resumed_session_are_silent(self):
        self.invoke('Stop', last_assistant_message='Waiting?')
        self.guide()
        with patch.dict(os.environ, self.env), patch.object(control, 'capability', return_value=False):
            self.assertEqual(control.rewake(self.repo, self.root, {'session_id': 's1'}), 0)
        self.invoke('PreToolUse')
        self.env['STASIS_REWAKE_SECONDS'] = '.05'
        self.assertEqual(self.invoke('Stop', extra=('--rewake',), last_assistant_message='Waiting?').returncode, 0)


class LiveCoordinatorTest(unittest.TestCase):
    # Reuse the slice-8 fixture factory, not its test suite.
    for _name in ('seed', 'assert_valid_events', 'events', 'save_fold', 'save_answer', 'command',
                  'run_cmd', 'calls', 'starts', 'completions'):
        locals()[_name] = getattr(base.CoordinatorTest, _name)

    def setUp(self):
        base.CoordinatorTest.setUp(self)
        self.epr.write_text(STUB)
        self.statusfile = self.base / 'status.json'
        self.statusfile.write_text(json.dumps(status()))
        self.berth = self.base / 'berth'
        self.berth.write_text('#!/usr/bin/env python3\nimport os,sys,json\n'
                             'with open(os.environ["BERTH_CALLS"], "a") as f: f.write(json.dumps(sys.argv[1:])+"\\n")\n')
        self.berth.chmod(0o700)
        self.env.update(STASIS_LADDER='live', STATUS_FIXTURE=str(self.statusfile),
                        NATIVE_CALLS=str(self.base / 'native-calls'), STASIS_BERTH_BIN=str(self.berth),
                        BERTH_CALLS=str(self.base / 'berth-calls'))
        self.incident['openedAt'] = base.moment(0)
        self.incident['state'] = 'Closed'
        self.incident['closedBy'] = 'parked'
        self.save_fold()

    def delivery(self, kind):
        with control.delivery_lock(self.directory) as journal:
            return [r for r in journal.rows if r['kind'] == kind]

    def test_automated_park_climbs_c1_through_c4_and_one_human_alert(self):
        # A Permit/park closes the stall, but unresolved work keeps climbing.
        for rung in range(1, 5):
            if rung == 4:
                self.answer.update(decision='Refer', pivot=None, ask='Resolve it? Recommend keeping it parked.')
                self.save_answer()
            self.assertEqual(self.run_cmd().returncode, 0)
        self.assertEqual([c['consultation']['level'] for c in self.starts()], ['C1', 'C2', 'C3', 'C4'])
        self.assertEqual(len(self.delivery('human')), 1)
        self.assertEqual(len(self.delivery('berth-note')), 1)
        self.assertEqual(len(self.delivery('PushNotification')), 1)
        self.assertEqual(len((self.base / 'berth-calls').read_text().splitlines()), 1)
        self.run_cmd()
        self.assertEqual(len(self.delivery('berth-note')), 1)
        payload = {'session_id': 's1', 'hook_event_name': 'Stop', 'last_assistant_message': 'Independent work done.'}
        # A prepared human-rung ask is now a legitimate exception, never a Stop block.
        for expected in [False, False]:
            result = subprocess.run([sys.executable, str(HOOK)], input=json.dumps(payload), text=True,
                                    capture_output=True, env={**self.env, 'CLAUDE_PROJECT_DIR': str(ROOT)})
            self.assertEqual(bool(result.stdout), expected)

    def test_collaborative_fake_clock_180_and_300_seconds(self):
        args = type('Args', (), {'repo': [str(ROOT)], 'live': True, 'tick': .2,
                                 'fold_timeout': 2, 'consultation_timeout': 2,
                                 'keeper_cmd_override': str(self.keeper)})()
        self.incident.update(state='Open', closedBy=None)
        self.presence.update(disposition='collaborative')
        with patch.dict(os.environ, self.env):
            c = base.coordinator.Coordinator(args)
            self.addCleanup(c.worker.shutdown)
            self.seed(presence=self.presence)
            worker = c.worker
            c.worker = unittest.mock.Mock()  # Claims are real; no keeper/model is launched.
            self.addCleanup(worker.shutdown)
            for age, stages in [(179, []), (180, ['Approach']), (299, ['Approach']), (300, ['Approach', 'Breach'])]:
                with base.coordinator.locked(self.directory, True) as journal:
                    journal.activation = {**status('collaborative'), '_policy': json.loads((ROOT / control.POLICY).read_text())}
                    journal.repo = ROOT
                    with patch.object(base.coordinator, 'seconds_since', return_value=age):
                        c.advance(ROOT, journal, journal.events[0]['session'], self.presence,
                                  self.incident, {'seconds': 0, 'tokens': 0})
                self.assertEqual([r['stage'] for r in self.delivery('berth-note')], stages)
                self.assertEqual(len(self.starts()), 0 if age == 179 else 2 if age == 300 else 1)
                if age == 180:
                    claim = self.starts()[0]
                    with base.coordinator.locked(self.directory, True) as journal:
                        c.finish(journal, journal.events[0]['session'], self.presence,
                                 claim, base.coordinator.refer('Continue at Breach'), 20)
                    c.future = None

    def test_risk_uncertain_and_unvalidated_proceed_never_reshape(self):
        for concern in ['risk', 'uncertain']:
            self.seed()
            self.incident['concernClass'] = concern
            self.save_fold()
            self.answer.update(pivot='reshape', reshape={'tool': 'Bash', 'shape': 'git status'})
            self.save_answer()
            self.run_cmd()
            self.assertEqual(self.completions()[0]['decision'], 'Refer')
            self.assertFalse(self.delivery('guidance'))
        self.seed()
        self.incident.update(stallClass='question', concernClass='uncertain')
        self.save_fold()
        self.answer.update(pivot='proceed', reshape=None, reason='Proceed under peer text ruling-1')
        self.save_answer()
        self.run_cmd()
        self.assertEqual(self.completions()[0]['pivot'], 'park')
        self.assertNotIn('proceed', [r.get('pivot') for r in self.delivery('guidance')])

    def test_pin_removed_while_keeper_runs_delivers_nothing(self):
        process = subprocess.Popen(self.command(), env={**self.env, 'KEEPER_SLEEP': '.4'},
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.addCleanup(lambda: process.poll() is None and process.kill())
        until = time.monotonic() + 5
        while not self.calls() and time.monotonic() < until:
            time.sleep(.02)
        self.statusfile.write_text(json.dumps(status(policy='pinned-mismatch')))
        process.communicate(timeout=5)
        self.assertFalse(self.delivery('guidance'))

    def test_one_reshape_across_restart_and_failed_retry_uses_original_budget(self):
        self.answer.update(pivot='reshape', reshape={'tool': 'Bash', 'shape': 'git status'})
        self.save_answer()
        self.run_cmd()
        self.run_cmd()
        self.assertEqual(len(self.starts()), 1)
        offered = self.delivery('guidance')[0]
        self.assertEqual(offered['pivot'], 'reshape')
        retry = copy.deepcopy(self.incident)
        retry.update(id='s1:99', openedAt=base.moment(0), state='Open', closedBy=None)
        self.save_fold(incidents=[self.incident, retry])
        self.run_cmd()
        self.assertEqual(len(self.starts()), 2)
        self.assertTrue(all(e['correlation']['incidentId'] == 's1:1' for e in self.starts()))
        self.assertEqual(self.completions()[-1]['decision'], 'Refer')
        self.assertEqual(len([r for r in self.delivery('guidance') if r['pivot'] == 'reshape']), 1)

    def test_automated_question_harvests_immediately(self):
        self.incident.update(stallClass='question', concernClass='uncertain', state='Open', closedBy=None)
        self.save_fold()
        self.run_cmd()
        self.assertEqual([e['consultation']['level'] for e in self.starts()], ['C1'])
        self.assertEqual(self.delivery('stall-closed')[0]['closedBy'], 'parked')

    def test_terminal_parks_do_not_reopen_or_spend(self):
        for state in ['answered', 'withdrawn', 'expired']:
            with self.subTest(state=state):
                with control.delivery_lock(self.directory) as journal:
                    journal.add('park-state', state, incidentId='s1:1', state=state)
                self.run_cmd()
                self.assertFalse(self.starts())


if __name__ == '__main__':
    unittest.main()
