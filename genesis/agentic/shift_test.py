"""Execution-contract regressions. Process tests use real PID namespaces and ark."""
import datetime as dt
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
import shift_ops as ops
import shift_runtime as runtime
import shift_state as state
import shift_supervisor as supervisor


def iso(seconds):
    return dt.datetime.fromtimestamp(seconds, dt.timezone.utc).isoformat()


class ContractFixture(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='elohim-shift-test-')
        self.root = Path(self.temp.name)
        self.directory = state.home(self.root, 'test-shift')
        self.directory.mkdir(parents=True)
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        subprocess.run(['git', '-C', str(self.root), '-c', 'user.name=Fixture', '-c',
                        'user.email=fixture@example.invalid', 'commit', '--allow-empty', '-qm', 'baseline'], check=True)
        (self.root / '.gitignore').write_text('.claude/\n')
        subprocess.run(['git', '-C', str(self.root), 'add', '.gitignore'], check=True)
        subprocess.run(['git', '-C', str(self.root), '-c', 'user.name=Fixture', '-c',
                        'user.email=fixture@example.invalid', 'commit', '-qm', 'private state'], check=True)
        self.deadline = time.time() + 60
        grant = {'build': True, 'landing': False, 'source': 'test authorization', 'deadline': iso(self.deadline)}
        state.atomic(self.directory / 'authority.json', grant)
        state.atomic(self.directory / 'revocation.json', {'revoked': False})
        state.atomic(self.directory / 'objective.json', {'scope': {'paths': ['**']}, 'budget': {'iterations': 3}})
        self.contract = {
            'version': 1, 'id': 'test-shift', 'worktree': str(self.root), 'started_at': iso(time.time()-1),
            'deadline': iso(self.deadline), 'mode': 'rehearsal', 'baseline_revision': state.revision(self.root),
            'objective_sha256': state.digest(self.directory / 'objective.json'),
            'objective': state.read(self.directory / 'objective.json'), 'allowed_tools': ['Bash(pwd)'],
            'intent': 'intent-existing', 'commitment': 'promise-existing', 'scope_cid': 'scope-existing', 'habit': 'dataplane-convergence',
            'verification': ['elohim-storage'],
            'authority': {'path': '.claude/shifts/test-shift/authority.json',
                          'sha256': state.digest(self.directory / 'authority.json'),
                          'revocation': '.claude/shifts/test-shift/revocation.json'},
            'steps': [{'id': 'implement', 'kind': 'agent', 'prompt': 'fixture'}],
        }
        self.freeze()

    def freeze(self):
        state.atomic(self.directory / 'contract.json', self.contract)
        self.checkpoint = {'version': 1, 'contract_sha256': state.digest(self.directory / 'contract.json'),
                           'last_wall': time.time()-0.1, 'runtime': 'codex', 'phase': 'ready', 'runs': []}
        state.atomic(self.directory / 'checkpoint.json', self.checkpoint)

    def tearDown(self):
        self.temp.cleanup()

    def test_original_scope_survives_checkpoint_and_runtime_switch(self):
        state.checkpoint(self.directory, runtime='claude')
        contract, checkpoint = state.load(self.directory)
        self.assertEqual(contract['scope_cid'], 'scope-existing')
        self.assertEqual(checkpoint['runtime'], 'claude')

    def test_missing_corrupt_or_changed_execution_authority_refuses(self):
        for path in ['checkpoint.json', 'contract.json', 'objective.json', 'authority.json', 'revocation.json']:
            with self.subTest(path=path):
                target = self.directory / path
                original = target.read_bytes()
                target.write_text('{broken')
                with self.assertRaises((state.Refused, KeyError)):
                    state.load(self.directory)
                target.write_bytes(original)
        grant = state.read(self.directory / 'authority.json')
        grant['build'] = False
        state.atomic(self.directory / 'authority.json', grant)
        with self.assertRaisesRegex(state.Refused, 'authorization.*changed'):
            state.load(self.directory)

    def test_stop_and_revocation_are_not_cleared_by_recovery(self):
        state.atomic(self.directory / 'revocation.json', {'revoked': True})
        with self.assertRaisesRegex(state.Refused, 'revoked'):
            state.load(self.directory)
        state.atomic(self.directory / 'revocation.json', {'revoked': False})
        state.atomic(self.directory / 'stop.json', {'reason': 'operator'})
        with self.assertRaisesRegex(state.Refused, 'stop marker'):
            state.checkpoint(self.directory, phase='running')

    def test_deadline_and_clock_regression_refuse(self):
        with self.assertRaisesRegex(state.Refused, 'deadline expired'):
            state.check_clock(self.contract, self.checkpoint, self.deadline + 1)
        with self.assertRaisesRegex(state.Refused, 'clock regression'):
            state.check_clock(self.contract, self.checkpoint, self.checkpoint['last_wall'] - 1)

    def test_publication_requires_separate_original_authority(self):
        with self.assertRaisesRegex(state.Refused, 'landing/push'):
            state.authority(self.directory, self.contract, self.checkpoint, publication=True)

    def test_exclusive_ownership_and_receipt_immutability(self):
        with state.lock(self.directory / 'execution.lock'):
            with self.assertRaisesRegex(state.Refused, 'already owned'):
                with state.lock(self.directory / 'execution.lock', blocking=False):
                    pass
        target = self.directory / 'fulfillment.json'
        state.atomic(target, {'original': True}, exclusive=True)
        original = target.read_bytes()
        with self.assertRaises(FileExistsError):
            state.atomic(target, {'original': False}, exclusive=True)
        self.assertEqual(target.read_bytes(), original)

    def test_stale_process_incarnation_never_signals(self):
        current = state.incarnation(os.getpid())
        current['start'] = 'impossible'
        self.assertFalse(state.signal_handle(current, signal.SIGKILL))

    def test_split_socket_frame_is_received_as_one_message(self):
        import socket
        import threading
        left, right = socket.socketpair()
        def send():
            with left:
                left.sendall(b'{"token":')
                time.sleep(.02)
                left.sendall(b'"abc"}\n')
        thread = threading.Thread(target=send)
        thread.start()
        with right:
            self.assertEqual(supervisor.read_message(right), {'token': 'abc'})
        thread.join()

    def test_only_unstaged_pinned_observers_can_coexist_with_clean_source(self):
        path = self.root / 'observer.json'
        path.write_text('{}')
        subprocess.run(['git', '-C', str(self.root), 'add', 'observer.json'], check=True)
        subprocess.run(['git', '-C', str(self.root), '-c', 'user.name=Fixture', '-c',
                        'user.email=fixture@example.invalid', 'commit', '-qm', 'observer fixture'], check=True)
        path.write_text('{"next":1}')
        self.contract['observer_paths'] = ['observer.json']
        self.assertTrue(ops.clean(self.root, self.contract))
        self.assertNotIn('observer.json', ops.scoped_changes(self.contract))
        subprocess.run(['git', '-C', str(self.root), 'add', 'observer.json'], check=True)
        self.assertFalse(ops.clean(self.root, self.contract))
        with self.assertRaisesRegex(state.Refused, 'observer state is staged'):
            ops.scoped_changes(self.contract)

    def test_canary_requires_successful_tool_and_terminal_runtime_evidence(self):
        log = self.directory / 'canary.jsonl'
        for name in ['codex', 'claude']:
            terminal = {'type': 'turn.completed'} if name == 'codex' else {'type': 'result', 'is_error': False, 'result': 'SHIFT_CANARY_OK'}
            log.write_text(json.dumps(terminal)+'\n')
            self.assertTrue(runtime.completed(name, log))
            self.assertFalse(runtime.completed(name, log, canary_root=self.root))
            log.write_text('{"type":"turn.started"}\n')
            self.assertFalse(runtime.completed(name, log))

    def test_saved_session_recovers_before_checkpoint_for_both_adapters(self):
        session = '11111111-1111-4111-8111-111111111111'
        for name in ['codex', 'claude']:
            with self.subTest(runtime=name):
                operation = ops.op_dir(self.directory, self.contract['steps'][0])
                if operation.exists():
                    import shutil
                    shutil.rmtree(operation)
                self.checkpoint['runtime'] = name
                pending = ops.prepare(self.directory, self.contract, self.checkpoint, self.contract['steps'][0])
                # Real runtimes write their session id before doing work. No checkpoint is updated here.
                event = {'type': 'thread.started', 'thread_id': session} if name == 'codex' else {'type': 'system', 'session_id': pending['session']}
                (operation / pending['log']).write_text(json.dumps(event)+'\n{"partial')
                state.atomic(operation / 'dispatch-1.json', {'started': True})
                resumed = ops.prepare(self.directory, self.contract, self.checkpoint, self.contract['steps'][0])
                expected = session if name == 'codex' else pending['session']
                self.assertEqual(resumed['resume_session'], expected)
                argv = runtime.command(name, session=expected)
                self.assertIn(expected, argv)
                self.assertNotIn('--last', argv)
                self.assertNotIn('--continue', argv)
                self.assertFalse(any('bypass' in arg or 'skip-permissions' in arg for arg in argv))

    def test_runtime_switch_uses_new_conversation_same_task(self):
        step = self.contract['steps'][0]
        pending = ops.prepare(self.directory, self.contract, self.checkpoint, step)
        operation = ops.op_dir(self.directory, step)
        (operation / pending['log']).write_text(json.dumps({'type': 'thread.started', 'thread_id': '11111111-1111-4111-8111-111111111111'}))
        state.atomic(operation / 'dispatch-1.json', {'started': True})
        self.checkpoint['runtime'] = 'claude'
        switched = ops.prepare(self.directory, self.contract, self.checkpoint, step)
        self.assertIsNone(switched['resume_session'])
        self.assertIsNotNone(switched['session'])
        self.assertEqual(self.contract['scope_cid'], 'scope-existing')

    def test_uncertain_dispatch_without_session_is_not_repeated(self):
        step = self.contract['steps'][0]
        ops.prepare(self.directory, self.contract, self.checkpoint, step)
        state.atomic(ops.op_dir(self.directory, step) / 'dispatch-1.json', {'started': True})
        with self.assertRaisesRegex(state.Refused, 'uncertain'):
            ops.prepare(self.directory, self.contract, self.checkpoint, step)

    def test_arbitrary_command_is_not_replayed_after_unknown_side_effect(self):
        step = {'id': 'measure', 'kind': 'measure', 'argv': ['true']}
        ops.prepare(self.directory, self.contract, self.checkpoint, step)
        state.atomic(ops.op_dir(self.directory, step) / 'dispatch-1.json', {'started': True})
        with self.assertRaisesRegex(state.Refused, 'unresolved side effect'):
            ops.prepare(self.directory, self.contract, self.checkpoint, step)

    def test_controller_death_before_dispatch_does_not_resume_nonexistent_session(self):
        for name in ['codex', 'claude']:
            step = {'id': f'prepared-{name}', 'kind': 'agent', 'prompt': 'fixture'}
            self.checkpoint['runtime'] = name
            first = ops.prepare(self.directory, self.contract, self.checkpoint, step)
            second = ops.prepare(self.directory, self.contract, self.checkpoint, step)
            self.assertEqual(second, first)
            self.assertIsNone(second['resume_session'])

    def test_superseded_acceptance_cannot_authorize_an_old_revision(self):
        status_dir = self.root / '.eprfs/status'
        status_dir.mkdir(parents=True)
        for name, revision in [('old', 'revision-a'), ('new', 'revision-b')]:
            state.atomic(self.directory / f'{name}.json', {'intent': self.contract['intent'], 'revision': revision})
        rows = [
            {'cid': 'old', 'record': {'classifiedAs': ['verdict:approved', 'report-path:.claude/shifts/test-shift/old.json']}},
            {'cid': 'new', 'record': {'classifiedAs': ['verdict:approved', 'supersedes:old', 'report-path:.claude/shifts/test-shift/new.json']}},
        ]
        (status_dir / 'flows.jsonl').write_text('\n'.join(json.dumps(row) for row in rows))
        native = ({}, {'acceptance': 'accepted', 'issues': [], 'acceptance_records': ['old', 'new']})
        with patch.object(state, 'native_context', return_value=native):
            with self.assertRaisesRegex(state.Refused, 'exact delivered revision'):
                state.require_acceptance(self.contract, 'revision-a')
            state.require_acceptance(self.contract, 'revision-b')

    def test_current_native_acceptance_is_required_despite_checkpoint_done(self):
        for verdict in ['acceptance-unestablished', 'revalidation-required', 'contested', 'changes-requested']:
            with self.subTest(verdict=verdict), patch.object(state, 'native_context', return_value=({}, {'acceptance': verdict, 'issues': []})):
                with self.assertRaisesRegex(state.Refused, 'current native acceptance'):
                    state.require_acceptance(self.contract, state.revision(self.root))

    def test_missing_gate_or_household_receipt_cannot_be_accepted(self):
        with patch.object(ops, 'require_acceptance', return_value={}):
            with self.assertRaisesRegex(state.Refused, 'gate receipts'):
                ops.require_delivery_evidence(self.directory, self.contract, state.revision(self.root))

    def test_evidence_changed_after_measurement_refuses(self):
        operation = ops.op_dir(self.directory, self.contract['steps'][0])
        operation.mkdir(parents=True)
        evidence = operation / 'stdout.log'
        evidence.write_text('passed')
        state.atomic(operation / 'receipt.json', {'operation': 'implement',
                     'contract_sha256': state.digest(self.directory / 'contract.json'),
                     'evidence': [{'path': str(evidence.relative_to(self.root)), 'sha256': state.digest(evidence)}]})
        self.assertEqual(len(ops.receipts(self.directory, self.contract)), 1)
        evidence.write_text('failed')
        with self.assertRaisesRegex(state.Refused, 'changed receipt evidence'):
            ops.receipts(self.directory, self.contract)


if __name__ == '__main__':
    unittest.main()
