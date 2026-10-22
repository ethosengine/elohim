"""Delivery evidence regressions; fixture receipts confer no native acceptance."""
import copy
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
import shift_evidence as evidence
import shift_ops as ops
import shift_state as state


class EvidenceFixture(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='elohim-shift-evidence-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        (self.root / '.gitignore').write_text('.claude/\n.eprfs/\n')
        subprocess.run(['git', '-C', str(self.root), 'add', '.gitignore'], check=True)
        subprocess.run(['git', '-C', str(self.root), '-c', 'user.name=Fixture', '-c',
                        'user.email=fixture@example.invalid', 'commit', '-qm', 'baseline'], check=True)
        self.delivered = state.revision(self.root)
        self.directory = state.home(self.root, 'evidence-fixture')
        self.directory.mkdir(parents=True)
        self.pid_file = self.directory / 'storage.pid'
        handle = state.incarnation(os.getpid())
        self.pid_file.write_text(f"{handle['pid']} {handle['start']}\n")
        self.household = {
            'id': 'household', 'kind': 'household', 'oracle': '@sync-document-id-roundtrip',
            'proof': '.claude/shifts/evidence-fixture/oracle.json',
            'binary': {'build_operation': 'build', 'pid_file': str(self.pid_file)},
        }
        self.ci = {'id': 'ci', 'kind': 'ci', 'proof': '.claude/shifts/evidence-fixture/ci.json'}
        self.contract = {
            'worktree': str(self.root), 'intent': 'fixture-intent',
            'downstream': ['storage', 'orchestration'],
            'objective': {'target': {'predicate': '>=', 'value': 1,
                                     'stability': {'consecutive': 2}}},
            'steps': [self.household, self.ci],
        }
        state.atomic(self.directory / 'contract.json', self.contract)
        self.pending = {'contract_sha256': state.digest(self.directory / 'contract.json'),
                        'attempt': 2, 'prepared_at': time.time() - 5}
        self.raw = self.directory / 'observations.log'
        self.raw.write_text('fixture scenario passed\n')
        self.proof = {
            **self.pending, 'operation': 'household', 'revision': self.delivered,
            'success': True, 'observed_at': time.time(), 'observations': ['fixture only'],
            'evidence': [{'path': str(self.raw.relative_to(self.root)), 'sha256': state.digest(self.raw)}],
            'oracle': self.household['oracle'], 'scenarios': {'passed': 1, 'failed': 0, 'skipped': 0},
        }
        self.build = {
            'operation': 'build', 'contract_sha256': self.pending['contract_sha256'],
            'revision': self.delivered, 'exit': 0, 'clean': True,
            'artifact': {'path': sys.executable, 'sha256': state.digest('/proc/self/exe')},
        }
        self.build_path = self.directory / 'operations/build/receipt.json'
        state.atomic(self.build_path, self.build)

    def check_proof(self, proof=None, step=None, pending=None):
        step = step or self.household
        proof = proof if proof is not None else self.proof
        state.atomic(self.root / step['proof'], proof)
        return evidence.oracle_proof(self.contract, step, self.delivered, pending or self.pending)

    def observe(self, position):
        return evidence.binary_observation(self.directory, self.contract, self.household, position)


class OracleProofTests(EvidenceFixture):
    def test_current_household_proof_binds_exact_dispatch_and_raw_evidence(self):
        self.assertEqual(self.check_proof(), self.proof)

    def test_superseded_dispatch_and_other_contract_or_operation_refuse(self):
        for field, value in [('attempt', 1), ('contract_sha256', 'old-contract'),
                             ('operation', 'another-household')]:
            with self.subTest(field=field):
                proof = {**self.proof, field: value}
                with self.assertRaisesRegex(state.Refused, 'another dispatch'):
                    self.check_proof(proof)

    def test_stale_future_and_missing_observation_time_refuse(self):
        for observed in [self.pending['prepared_at'] - 1, time.time() + 3600, None, float('nan')]:
            with self.subTest(observed=observed):
                with self.assertRaisesRegex(state.Refused, 'stale or future-dated'):
                    self.check_proof({**self.proof, 'observed_at': observed})

    def test_other_revision_unsuccessful_or_incomplete_oracle_refuses(self):
        variants = [
            {'revision': 'another-revision'}, {'success': False}, {'observations': []},
            {'evidence': []}, {'oracle': '@different-scenario'},
            {'scenarios': {'passed': 0, 'failed': 0, 'skipped': 0}},
            {'scenarios': {'passed': 1, 'failed': 1, 'skipped': 0}},
            {'scenarios': {'passed': 1, 'failed': 0, 'skipped': 1}},
        ]
        for changed in variants:
            with self.subTest(changed=changed), self.assertRaises(state.Refused):
                self.check_proof({**self.proof, **changed})

    def test_changed_raw_oracle_evidence_refuses(self):
        self.check_proof()
        self.raw.write_text('named seed prerequisite missing\n')
        with self.assertRaisesRegex(state.Refused, 'raw evidence changed'):
            self.check_proof()

    def ci_proof(self):
        return {**self.proof, 'operation': self.ci['id'], 'builds': [
            {'pipeline': name, 'revision': self.delivered, 'result': 'SUCCESS',
             'number': number, 'url': f'https://ci.example.invalid/{name}/{number}/'}
            for number, name in enumerate(self.contract['downstream'], 1)
        ]}

    def test_ci_requires_one_successful_exact_revision_build_per_selected_pipeline(self):
        proof = self.ci_proof()
        self.assertEqual(self.check_proof(proof, self.ci), proof)
        for changed in [{'revision': 'other'}, {'result': 'FAILURE'}, {'result': 'NOT_BUILT'},
                        {'result': 'ABORTED'}, {'url': ''}, {'number': None}]:
            with self.subTest(changed=changed):
                invalid = copy.deepcopy(proof)
                invalid['builds'][0].update(changed)
                with self.assertRaisesRegex(state.Refused, 'downstream success'):
                    self.check_proof(invalid, self.ci)

    def test_ci_missing_duplicate_and_conflicting_selected_builds_refuse(self):
        proof = self.ci_proof()
        for builds in [proof['builds'][:1],
                       proof['builds'] + [proof['builds'][0]],
                       proof['builds'] + [{**proof['builds'][0], 'result': 'FAILURE', 'number': 99}]]:
            with self.subTest(builds=builds):
                with self.assertRaisesRegex(state.Refused, 'missing or conflicting'):
                    self.check_proof({**proof, 'builds': builds}, self.ci)

    def test_receipt_snapshots_survive_new_observation_and_reject_changed_pinned_bytes(self):
        operation = ops.op_dir(self.directory, self.household)
        pending = {**self.pending, 'operation': self.household['id'], 'log': 'host-2.log'}
        state.atomic(operation / 'pending.json', pending)
        # Run a real child writing the declared proof from the actual dispatch env.
        # Only authority setup is mocked; this test exercises execute's proof capture.
        script = '\n'.join([
            'import json, os, pathlib, time',
            f'proof = {self.proof!r}',
            "proof.update(contract_sha256=os.environ['SHIFT_CONTRACT_SHA256'],",
            "    operation=os.environ['SHIFT_OPERATION'], attempt=int(os.environ['SHIFT_ATTEMPT']),",
            "    revision=os.environ['SHIFT_REVISION'], observed_at=time.time())",
            "pathlib.Path(os.environ['SHIFT_PROOF_PATH']).write_text(json.dumps(proof))",
            "print('fixture oracle passed')",
        ])
        with patch.object(ops, 'load', return_value=(self.contract, {})), \
                patch.object(ops, 'host_argv', return_value=[sys.executable, '-c', script]):
            ops.execute(self.directory, self.household['id'])
        receipt_path = operation / 'receipt.json'
        original = receipt_path.read_bytes()
        receipt = ops.receipts(self.directory, self.contract)[0]
        self.assertEqual(receipt['exit'], 0)
        snapshot = operation / 'proof-2.json'
        original_proof = snapshot.read_bytes()
        self.check_proof({**self.proof, 'attempt': 3}, pending={**self.pending, 'attempt': 3})
        self.assertEqual(ops.receipts(self.directory, self.contract)[0], receipt)
        self.assertEqual(receipt_path.read_bytes(), original)
        self.assertEqual(snapshot.read_bytes(), original_proof)
        self.raw.write_text('later verification contradicts old observation\n')
        with self.assertRaisesRegex(state.Refused, 'changed receipt evidence'):
            ops.receipts(self.directory, self.contract)
        self.assertEqual(receipt_path.read_bytes(), original)


class BinaryEvidenceTests(EvidenceFixture):
    def test_real_binary_before_after_and_recovery_preserve_original_receipt(self):
        before = self.observe('before')
        operation = ops.op_dir(self.directory, self.household)
        receipt = {'operation': 'household', 'exit': 0, 'revision': self.delivered,
                   'contract_sha256': self.pending['contract_sha256'], 'evidence': []}
        state.atomic(operation / 'receipt.json', receipt, exclusive=True)
        original = (operation / 'receipt.json').read_bytes()
        # Simulate controller loss after worker receipt but before guardian observation.
        self.assertFalse((operation / 'binary-after.json').exists())
        after = self.observe('after')
        evidence.require_binary_evidence(self.directory, self.household, self.delivered)
        self.assertEqual(before['sha256'], state.digest('/proc/self/exe'))
        self.assertEqual(before['process'], state.incarnation(os.getpid()))
        self.assertEqual(after['process'], before['process'])
        original_after = (operation / 'binary-after.json').read_bytes()
        self.assertEqual(self.observe('after'), after)
        self.assertEqual((operation / 'binary-after.json').read_bytes(), original_after)
        self.assertEqual((operation / 'receipt.json').read_bytes(), original)

    def test_build_wrong_revision_contract_exit_cleanliness_or_binary_refuses(self):
        for changed in [{'revision': 'other'}, {'contract_sha256': 'other'}, {'exit': 1},
                        {'clean': False}, {'artifact': {}}, {'artifact': {'sha256': 'wrong-binary'}}]:
            with self.subTest(changed=changed):
                state.atomic(self.build_path, {**self.build, **changed})
                with self.assertRaises(state.Refused):
                    self.observe('before')
                self.assertFalse((ops.op_dir(self.directory, self.household) / 'binary-before.json').exists())

    def test_pid_only_and_stale_start_ticks_refuse_even_for_matching_executable(self):
        for record in [str(os.getpid()), f'{os.getpid()} stale-start-ticks']:
            with self.subTest(record=record):
                self.pid_file.write_text(record)
                with self.assertRaisesRegex(state.Refused, 'PID record'):
                    self.observe('before')

    def test_pid_incarnation_change_during_executable_read_refuses(self):
        original = state.incarnation(os.getpid())
        with patch.object(evidence, 'incarnation', side_effect=[original, {**original, 'start': 'reused'}]):
            with self.assertRaisesRegex(state.Refused, 'exact built storage binary'):
                self.observe('before')

    def test_identical_executable_in_replacement_process_cannot_complete_original_oracle(self):
        self.observe('before')
        child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(30)'])
        try:
            handle = state.incarnation(child.pid)
            self.pid_file.write_text(f"{handle['pid']} {handle['start']}\n")
            self.assertEqual(state.digest(f'/proc/{child.pid}/exe'), self.build['artifact']['sha256'])
            self.observe('after')
            with self.assertRaisesRegex(state.Refused, 'binary/process changed'):
                evidence.require_binary_evidence(self.directory, self.household, self.delivered)
            with self.assertRaisesRegex(state.Refused, 'conflicts with current process'):
                self.observe('before')
        finally:
            child.terminate()
            child.wait(timeout=5)

    def test_missing_changed_stale_and_clock_regressed_binary_witnesses_refuse(self):
        before = self.observe('before')
        with self.assertRaisesRegex(state.Refused, 'missing/corrupt state'):
            evidence.require_binary_evidence(self.directory, self.household, self.delivered)
        after = self.observe('after')
        path = ops.op_dir(self.directory, self.household) / 'binary-after.json'
        for changed in [{'sha256': 'different'}, {'build_operation': 'different'},
                        {'revision': 'different'}, {'observed_at': before['observed_at'] - 1}]:
            with self.subTest(changed=changed):
                state.atomic(path, {**after, **changed})
                with self.assertRaises(state.Refused):
                    evidence.require_binary_evidence(self.directory, self.household, self.delivered)
        state.atomic(path, after)
        with self.assertRaisesRegex(state.Refused, 'stale'):
            evidence.require_binary_evidence(self.directory, self.household, 'another-revision')


class ObjectiveMeasurementTests(unittest.TestCase):
    def setUp(self):
        self.contract = {'objective': {'target': {'predicate': '>=', 'value': 1,
                                                 'stability': {'consecutive': 2}}}}

    @staticmethod
    def measured(operation, value=1, revision='delivered', clean=True):
        return {'operation': operation, 'measurement': value, 'revision': revision, 'clean': clean,
                'exit': 0, 'finished': time.time()}

    def check(self, rows):
        evidence.require_measurements(self.contract, rows, 'delivered')

    def test_requires_declared_count_of_clean_observations_at_exact_revision(self):
        valid = [self.measured('one'), self.measured('two')]
        self.check(valid)
        for rows in [[], valid[:1], [valid[0], self.measured('two', revision='old')],
                     [valid[0], self.measured('two', clean=False)]]:
            with self.subTest(rows=rows), self.assertRaisesRegex(state.Refused, 'consecutive measurements'):
                self.check(rows)

    def test_latest_consecutive_results_must_meet_target_and_be_finite(self):
        for value in [0, float('nan'), float('inf'), float('-inf')]:
            with self.subTest(value=value), self.assertRaises(state.Refused):
                self.check([self.measured('one'), self.measured('two'), self.measured('three', value)])
        self.check([self.measured('old-red', 0), self.measured('one'), self.measured('two')])

    def test_failed_or_invalid_attempt_resets_stability_in_observation_order(self):
        first = self.measured('a')
        failed = {**self.measured('b', 0), 'exit': 126}
        retried = self.measured('b')
        with self.assertRaises(state.Refused):
            self.check([retried, first, failed])
        invalid = {**failed, 'objective': True}
        del invalid['measurement']
        with self.assertRaises(state.Refused):
            self.check([first, invalid, retried])
        self.check([first, failed, retried, self.measured('a')])


if __name__ == '__main__':
    unittest.main()
