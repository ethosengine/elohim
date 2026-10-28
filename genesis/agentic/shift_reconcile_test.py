"""Crash-boundary recovery and bounded retry tests in disposable repositories."""
import json
from pathlib import Path
import subprocess
import sys
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
import shift as launcher
import shift_ops as ops
import shift_reconcile as reconcile
import shift_state as state
import shift_supervisor as supervisor
import shift_test as fixture_module


class RecoveryTests(unittest.TestCase):
    def setUp(self):
        fixture = fixture_module.ContractFixture(methodName='runTest')
        fixture.setUp()
        self.addCleanup(fixture.tearDown)
        self.fixture = fixture
        self.root, self.directory, self.contract = fixture.root, fixture.directory, fixture.contract
        self.git('config', 'user.name', 'Fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')

    def git(self, *args):
        return state.run(['git', *args], self.root)

    def configure(self, step):
        self.step = step
        self.contract['steps'] = [step]
        self.fixture.freeze()
        self.checkpoint = self.fixture.checkpoint
        self.operation = ops.op_dir(self.directory, step)

    def dispatch(self, argv=None, binding=None):
        pending = ops.prepare(self.directory, self.contract, self.checkpoint, self.step)
        argv = argv or self.step.get('argv', ['fixture-command'])
        state.atomic(self.operation / f"dispatch-{pending['attempt']}.json", {
            'operation': self.step['id'], 'contract_sha256': pending['contract_sha256'],
            'attempt': pending['attempt'], 'revision': pending['revision'],
            'argv': argv, 'native_binding': binding,
        }, exclusive=True)
        (self.operation / pending['log']).write_text('original operation output\n')
        return pending

    def finish(self, pending, code=0, session=None):
        log = self.operation / pending['log']
        result = {'operation': self.step['id'], 'kind': self.step['kind'],
                  'contract_sha256': pending['contract_sha256'], 'attempt': pending['attempt'],
                  'exit': code, 'clean': True, 'revision': state.revision(self.root), 'session': session,
                  'evidence': [{'path': str(log.relative_to(self.root)), 'sha256': state.digest(log)}]}
        ops.save_receipt(self.operation, result)
        return result

    def next(self):
        return ops.next_operation(self.directory, self.contract, self.checkpoint)

    def commit_effect(self, message=None):
        self.configure({'id': 'commit', 'kind': 'commit', 'message': 'delivered fixture'})
        (self.root / 'source.txt').write_text('fixture change\n')
        pending = ops.prepare(self.directory, self.contract, self.checkpoint, self.step)
        argv = ops.host_argv(self.directory, self.contract, self.checkpoint, self.step)
        self.dispatch(argv)
        if message:
            argv = ['git', 'commit', '-m', message]
        subprocess.run(argv, cwd=self.root, check=True, capture_output=True)
        return pending

    def test_unique_commit_reconciles_without_repeating_commit_or_changing_original_evidence(self):
        self.commit_effect()
        head = state.revision(self.root)
        log = (self.operation / 'host-1.log').read_bytes()
        self.assertFalse((self.operation / 'receipt.json').exists())
        self.assertIsNone(self.next())
        receipt = state.read(self.operation / 'receipt.json')
        self.assertTrue(receipt['recovered'])
        self.assertEqual(receipt['revision'], head)
        original = (self.operation / 'receipt.json').read_bytes()
        self.assertIsNone(self.next())
        self.assertEqual(state.revision(self.root), head)
        self.assertEqual((self.operation / 'receipt.json').read_bytes(), original)
        self.assertEqual((self.operation / 'host-1.log').read_bytes(), log)

    def test_substring_and_duplicate_commit_trailers_never_establish_unique_effect(self):
        self.commit_effect('prose mentions Shift-Operation: test-shift/commit without a trailer')
        with self.assertRaisesRegex(state.Refused, 'unique operation commit'):
            self.next()
        self.git('commit', '--allow-empty', '-m', 'first', '-m', 'Shift-Operation: test-shift/commit')
        self.git('commit', '--allow-empty', '-m', 'second', '-m', 'Shift-Operation: test-shift/commit')
        with self.assertRaisesRegex(state.Refused, 'unique operation commit'):
            self.next()
        self.assertFalse((self.operation / 'receipt.json').exists())

    def publication(self, kind):
        grant = state.read(self.directory / 'authority.json')
        grant['landing'] = True
        state.atomic(self.directory / 'authority.json', grant)
        self.contract['authority']['sha256'] = state.digest(self.directory / 'authority.json')
        landing = self.root.parent / (self.root.name + '-landing')
        self.addCleanup(lambda: subprocess.run(['git', '-C', str(self.root), 'worktree', 'remove', '--force', str(landing)], capture_output=True))
        self.git('worktree', 'add', '-q', '-b', 'dev', str(landing))
        self.contract['landing_worktree'] = str(landing)
        self.configure({'id': kind, 'kind': kind})
        if kind == 'push':
            remote = self.directory / 'remote.git'
            self.git('init', '--bare', '-q', str(remote))
            self.git('remote', 'add', 'origin', str(remote))
            state.run(['git', 'push', 'origin', 'dev:dev'], landing)
        with patch.object(ops, 'require_delivery_evidence'):
            argv = ops.host_argv(self.directory, self.contract, self.checkpoint, self.step)
        self.dispatch(argv)

    def test_land_and_push_observations_require_live_exact_revision_acceptance(self):
        for kind in ['land', 'push']:
            with self.subTest(kind=kind):
                # A separate disposable repository per publication boundary.
                fixture = RecoveryTests(methodName='runTest')
                fixture.setUp()
                try:
                    fixture.publication(kind)
                    delivered = state.revision(fixture.root)
                    with patch.object(ops, 'require_delivery_evidence') as accepted:
                        self.assertIsNone(fixture.next())
                        accepted.assert_called_once_with(fixture.directory, fixture.contract, delivered)
                    self.assertTrue(state.read(fixture.operation / 'receipt.json')['recovered'])
                finally:
                    fixture.doCleanups()

    def test_observed_publication_never_overrides_revoked_or_changed_acceptance(self):
        self.publication('land')
        with patch.object(ops, 'require_delivery_evidence', side_effect=state.Refused('native evidence changed')):
            with self.assertRaisesRegex(state.Refused, 'native evidence changed'):
                self.next()
        state.atomic(self.directory / 'revocation.json', {'revoked': True})
        with self.assertRaisesRegex(state.Refused, 'revoked'):
            self.next()
        self.assertFalse((self.operation / 'receipt.json').exists())

    def native_fixture(self):
        self.report = self.directory / 'original-report.md'
        self.report.write_bytes(b'Original fulfillment report.\nKeep these exact bytes.\n')
        argv = ['epr', 'flow', 'fulfill', '--on', self.contract['commitment'], '--report',
                str(self.report.relative_to(self.root)), '--status', 'DONE', '--as', 'agent:fixture@model',
                '--commit', state.revision(self.root)]
        self.configure({'id': 'fulfill', 'kind': 'native', 'argv': argv})
        self.native = ({'fixture': 'native-context'}, {'fulfillments': ['event-original']})
        self.identity = {'path': str(self.report.relative_to(self.root)), 'sha256': state.digest(self.report),
                         'cid': 'native-body-cid'}
        identity_patch = patch.object(reconcile, 'report_identity', side_effect=lambda contract, path: {
            **self.identity, 'sha256': state.digest(state.under(contract['worktree'], path))})
        native_patch = patch.object(reconcile, 'native_context', return_value=self.native)
        identity_patch.start()
        native_patch.start()
        self.addCleanup(identity_patch.stop)
        self.addCleanup(native_patch.stop)
        binding = reconcile.dispatch_binding(self.contract, self.step, argv)
        self.dispatch(argv, binding)
        self.row = {'cid': 'event-original', 'record': {
            'kind': 'event', 'action': 'produce', 'provider': 'agent:fixture@model',
            'resource': self.identity['cid'], 'inScopeOf': self.contract['scope_cid'],
            'fulfills': [self.contract['commitment']],
            'quantity': {'Count': {'value': 1.0, 'unit': 'task-report'}},
            'classifiedAs': ['report:DONE', 'evidence:' + self.identity['cid'],
                             'commit:' + state.revision(self.root)],
        }}
        self.flows = self.root / '.eprfs/status/flows.jsonl'
        self.flows.parent.mkdir(parents=True)
        self.flows.write_text(json.dumps(self.row) + '\n')

    def test_crash_after_native_fulfillment_before_receipt_observes_record_without_refulfilling(self):
        self.native_fixture()
        report_bytes, flow_bytes = self.report.read_bytes(), self.flows.read_bytes()
        self.assertFalse((self.operation / 'receipt.json').exists())
        self.assertIsNone(self.next())
        receipt_bytes = (self.operation / 'receipt.json').read_bytes()
        self.assertTrue(state.read(self.operation / 'receipt.json')['recovered'])
        self.assertEqual(state.read(self.operation / 'recovery-1.json')['effect']['record'], self.row)
        self.assertIsNone(self.next())
        self.assertEqual((self.operation / 'receipt.json').read_bytes(), receipt_bytes)
        self.assertEqual(self.report.read_bytes(), report_bytes)
        self.assertEqual(self.flows.read_bytes(), flow_bytes)

    def test_native_commit_revision_is_persisted_as_exact_dispatch_sha(self):
        self.native_fixture()
        delivered = state.revision(self.root)
        self.step['argv'] = self.step['argv'][:-2]
        self.step['commit_revision'] = True
        # Re-freeze this disposable declaration before its modeled dispatch.
        self.fixture.freeze()
        self.checkpoint = self.fixture.checkpoint
        for name in ['pending.json', 'dispatch-1.json']:
            (self.operation / name).unlink()
        argv = ops.host_argv(self.directory, self.contract, self.checkpoint, self.step)
        self.assertEqual(argv[-2:], ['--commit', delivered])
        self.dispatch(argv, reconcile.dispatch_binding(self.contract, self.step, argv))
        self.assertIsNone(self.next())
        witness = state.read(self.operation / 'recovery-1.json')
        self.assertIn('commit:' + delivered, witness['effect']['record']['record']['classifiedAs'])
        self.assertNotIn('--commit', self.step['argv'])

    def test_native_recovery_missing_changed_or_conflicting_original_evidence_refuses(self):
        self.native_fixture()
        for change in [{'provider': 'agent:other@model'}, {'fulfills': ['other-commitment']},
                       {'resource': 'other-report'}, {'inScopeOf': 'other-scope'},
                       {'classifiedAs': ['report:DONE', 'evidence:native-body-cid', 'commit:other']}]:
            with self.subTest(change=change):
                self.flows.write_text(json.dumps({**self.row, 'record': {**self.row['record'], **change}}) + '\n')
                with self.assertRaisesRegex(state.Refused, 'missing or ambiguous'):
                    self.next()
        self.flows.write_text(json.dumps(self.row) + '\n')
        self.native[1]['fulfillments'].append('event-duplicate')
        with self.flows.open('a') as stream:
            stream.write(json.dumps({**self.row, 'cid': 'event-duplicate'}) + '\n')
        with self.assertRaisesRegex(state.Refused, 'missing or ambiguous'):
            self.next()
        self.report.write_text('rewritten fulfillment\n')
        with self.assertRaisesRegex(state.Refused, 'original report changed'):
            self.next()
        self.assertFalse((self.operation / 'receipt.json').exists())

    def execute_native_fixture(self):
        # This fixture already models the native event. A real short child gives
        # execute its completed command; native record inspection remains required.
        (self.operation / 'dispatch-1.json').unlink()
        def spawn(argv, **kwargs):
            return subprocess.Popen([sys.executable, '-c', "print('native command returned')"], **kwargs)
        with patch.object(ops, 'load', return_value=(self.contract, self.checkpoint)), \
                patch.object(ops, 'host_argv', return_value=self.step['argv']), \
                patch.object(supervisor, 'spawn_owned', side_effect=spawn):
            ops.execute(self.directory, self.step['id'])

    def test_normal_native_completion_pins_original_report_in_receipt(self):
        self.native_fixture()
        self.execute_native_fixture()
        self.assertEqual(ops.receipts(self.directory, self.contract)[0]['exit'], 0)
        original = (self.operation / 'receipt.json').read_bytes()
        self.report.write_text('rewritten after fulfillment')
        with self.assertRaisesRegex(state.Refused, 'changed receipt evidence'):
            ops.receipts(self.directory, self.contract)
        self.assertEqual((self.operation / 'receipt.json').read_bytes(), original)

    def test_native_exit_zero_with_wrong_fulfillment_cannot_authorize_retry_or_progress(self):
        self.native_fixture()
        wrong = {**self.row, 'record': {**self.row['record'], 'provider': 'agent:other@model'}}
        self.flows.write_text(json.dumps(wrong) + '\n')
        self.execute_native_fixture()
        self.assertEqual(ops.receipts(self.directory, self.contract)[0]['exit'], 125)
        with self.assertRaisesRegex(state.Refused, 'side effects require independent inspection'):
            self.next()

    def test_native_report_identity_uses_native_body_cid_and_exact_original_bytes(self):
        report = self.directory / 'report.md'
        report.write_text('---\nstatus: DONE\n---\nOriginal body\n')
        with patch.object(reconcile, 'run', return_value=json.dumps({'identity': {'cid': 'native-computed-cid'}})) as native:
            identity = reconcile.report_identity(self.contract, str(report))
        native.assert_called_once_with(['epr', 'flow', 'context', str(report), '--json'], str(self.root))
        self.assertEqual(identity['cid'], 'native-computed-cid')
        self.assertEqual(identity['sha256'], state.digest(report))

    def test_known_agent_failure_resumes_only_saved_session_in_both_runtimes(self):
        for runtime in ['claude', 'codex']:
            with self.subTest(runtime=runtime):
                self.configure({'id': runtime, 'kind': 'agent', 'prompt': 'fixture'})
                self.checkpoint['runtime'] = runtime
                pending = self.dispatch()
                session = pending.get('session') or '11111111-1111-4111-8111-111111111111'
                event = {'type': 'system', 'session_id': session} if runtime == 'claude' else {'type': 'thread.started', 'thread_id': session}
                (self.operation / pending['log']).write_text(json.dumps(event) + '\n')
                self.finish(pending, 1, session)
                original = (self.operation / 'receipt.json').read_bytes()
                self.assertEqual(self.next(), self.step)
                retry = ops.prepare(self.directory, self.contract, self.checkpoint, self.step)
                self.assertEqual(retry['resume_session'], session)
                self.assertEqual(retry['attempt'], 2)
                self.assertEqual((self.operation / 'receipt.json').read_bytes(), original)

    def test_gate_failure_and_stale_success_retry_with_immutable_history_and_iteration_ceiling(self):
        self.configure({'id': 'gate', 'kind': 'gate', 'project': 'elohim-storage'})
        pending = self.dispatch()
        self.finish(pending, 1)
        original = (self.operation / 'receipt.json').read_bytes()
        self.assertEqual(self.next(), self.step)
        second = self.dispatch()
        self.finish(second)
        self.assertIsNone(self.next())
        self.git('commit', '--allow-empty', '-m', 'scoped repair')
        self.assertEqual(self.next(), self.step)
        third = self.dispatch()
        self.finish(third, 1)
        with self.assertRaisesRegex(state.Refused, 'iteration budget'):
            self.next()
        self.assertEqual((self.operation / 'receipt.json').read_bytes(), original)
        self.assertTrue((self.operation / 'receipt-2.json').exists())
        self.assertTrue((self.operation / 'receipt-3.json').exists())
        (self.operation / pending['log']).write_text('changed historical log')
        with self.assertRaisesRegex(state.Refused, 'changed receipt evidence'):
            ops.receipts(self.directory, self.contract)

    def test_only_explicit_readonly_measure_failure_is_retriable(self):
        for kind, readonly in [('measure', False), ('native', False), ('lifecycle', False),
                               ('household', False), ('ci', False), ('measure', True)]:
            with self.subTest(kind=kind, readonly=readonly):
                self.configure({'id': f'{kind}-{readonly}', 'kind': kind, 'readonly': readonly,
                                'argv': ['true']})
                self.finish(self.dispatch(), 1)
                if readonly:
                    self.assertEqual(self.next(), self.step)
                else:
                    with self.assertRaisesRegex(state.Refused, 'side effects require independent inspection'):
                        self.next()

    def test_objective_retry_keeps_failure_in_stability_history_and_collects_fresh_sample(self):
        command = "python3 -c \"from pathlib import Path; print(Path('.claude/shifts/test-shift/value').read_text())\""
        objective = {'measure': {'run': command}, 'scope': {'paths': ['**']},
                     'budget': {'iterations': 3}, 'baseline': {'predicate': '<=', 'value': 1},
                     'target': {'predicate': '==', 'value': 0, 'stability': {'consecutive': 2}}}
        state.atomic(self.directory / 'objective.json', objective)
        self.contract.update(objective=objective, objective_sha256=state.digest(self.directory / 'objective.json'))
        first = {'id': 'sample-a', 'kind': 'measure', 'objective': True, 'readonly': True}
        second = {**first, 'id': 'sample-b'}
        self.contract['steps'] = [first, second]
        self.fixture.freeze()
        self.checkpoint = self.fixture.checkpoint
        def measure(step, value):
            (self.directory / 'value').write_text(str(value))
            ops.prepare(self.directory, self.contract, self.checkpoint, step)
            ops.execute(self.directory, step['id'])
        measure(first, 0)
        self.assertEqual(self.next(), second)
        measure(second, 1)
        original = (ops.op_dir(self.directory, second) / 'receipt.json').read_bytes()
        self.assertEqual(self.next(), second)
        measure(second, 0)
        history = ops.objective_history(self.directory, self.contract)
        self.assertEqual(len(history), 3)
        with self.assertRaisesRegex(state.Refused, 'consecutive measurements'):
            ops.require_measurements(self.contract, history, state.revision(self.root))
        self.assertEqual(self.next(), first)
        measure(first, 0)
        self.assertIsNone(self.next())
        ops.require_measurements(self.contract, ops.objective_history(self.directory, self.contract), state.revision(self.root))
        self.assertEqual((ops.op_dir(self.directory, second) / 'receipt.json').read_bytes(), original)

    def test_uncertain_retry_cannot_hide_behind_previous_failed_gate_receipt(self):
        self.configure({'id': 'gate', 'kind': 'gate', 'project': 'elohim-storage'})
        self.finish(self.dispatch(), 1)
        self.dispatch()  # Attempt two reached dispatch but never wrote its receipt.
        with self.assertRaisesRegex(state.Refused, 'unresolved side effect'):
            self.next()

    def test_crash_after_recovery_witness_before_receipt_reuses_observation(self):
        self.commit_effect()
        with patch.object(ops, 'save_receipt', side_effect=RuntimeError('simulated crash')):
            with self.assertRaisesRegex(RuntimeError, 'simulated crash'):
                self.next()
        witness = (self.operation / 'recovery-1.json').read_bytes()
        self.assertIsNone(self.next())
        self.assertEqual((self.operation / 'recovery-1.json').read_bytes(), witness)


class ReadinessResourceTests(unittest.TestCase):
    def test_baseline_lease_and_declared_environment_release_even_on_failure(self):
        import tempfile
        with tempfile.TemporaryDirectory(prefix='shift-readiness-') as temporary:
            root = Path(temporary)
            (root / '.git').write_text('fixture worktree marker')
            ark = root / 'ark'
            ark.touch()
            directory = root / '.claude/shifts/fixture'
            spec = {'id': 'fixture', 'worktree': str(root), 'ark': str(ark), 'allowed_tools': [],
                    'verification': ['storage'], 'steps': [{'id': 'gate', 'kind': 'gate', 'project': 'storage'}],
                    'deadline': 'unchanged-original-deadline', 'measurement_resource': 'cargo',
                    'workload_env': {'CARGO_TARGET_POOL_FAMILY': 'fixture-pool'}}
            calls = []
            def bounded(argv, cwd, timeout, log, text=None, *, env=None, check=None):
                calls.append((argv, env))
                if 'genesis/agentic/readiness.mjs' in argv:
                    self.assertEqual(lease.call_args_list, [
                        unittest.mock.call(spec, ['cargo']),
                        unittest.mock.call(spec, ['cargo'], release=True),
                        unittest.mock.call(spec, 'cargo'),
                    ])
                    raise state.Refused('baseline failed')
            def run(argv, root):
                return 'storage\tfixture' if '--list' in argv else 'null'
            with patch.object(launcher.shutil, 'which', return_value='/fixture/tool'), \
                    patch.object(launcher, 'initialization_remaining', return_value=30), \
                    patch.object(launcher, 'session_from', return_value='fixture-session'), \
                    patch.object(launcher, 'completed', return_value=True), \
                    patch.object(launcher, 'native_context'), \
                    patch.object(launcher, 'bounded', side_effect=bounded), \
                    patch.object(launcher, 'run', side_effect=run), \
                    patch.object(launcher, 'lease') as lease:
                with self.assertRaisesRegex(state.Refused, 'baseline failed'):
                    launcher.preflight(spec, directory)
                self.assertEqual(lease.call_args_list, [
                    unittest.mock.call(spec, ['cargo']),
                    unittest.mock.call(spec, ['cargo'], release=True),
                    unittest.mock.call(spec, 'cargo'),
                    unittest.mock.call(spec, 'cargo', release=True),
                ])
            self.assertTrue(all(env == spec['workload_env'] for _, env in calls))
            self.assertEqual(spec['deadline'], 'unchanged-original-deadline')

    def test_invalid_readiness_resource_and_unrestricted_environment_refuse(self):
        for fields in [{'measurement_resource': 'all-resources'}, {'workload_env': {'PATH': '/replacement'}}]:
            with self.subTest(fields=fields), self.assertRaises(state.Refused):
                ops.validate_steps({'steps': [{'id': 'measure', 'kind': 'measure', 'argv': ['true']}], **fields})


if __name__ == '__main__':
    unittest.main()
