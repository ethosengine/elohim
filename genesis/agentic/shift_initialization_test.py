"""Readiness retry keeps the original initialization authority and every attempt."""
import datetime as dt
import json
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
import shift
import shift_state as state


class Initialization(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='shift-initialization-test-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.directory = state.home(self.root, 'initialization-test')
        self.directory.mkdir(parents=True)
        self.objective = self.root / 'objective-input.json'
        state.atomic(self.objective, {'budget': {'wall_clock_min': 2400}, 'scope': {'paths': ['source.py']}})
        self.deadline = dt.datetime.fromtimestamp(time.time() + 3600, dt.timezone.utc).isoformat()
        self.authority = self.directory / 'authority.json'
        state.atomic(self.authority, {'build': True, 'landing': False,
                                     'source': 'original test authorization', 'deadline': self.deadline})
        state.atomic(self.directory / 'revocation.json', {'revoked': False})
        self.declaration = self.root / 'declaration.json'
        self.spec = {
            'id': 'initialization-test', 'worktree': str(self.root), 'objective_path': self.objective.name,
            'deadline': self.deadline, 'mode': 'rehearsal', 'allowed_tools': ['Bash(pwd)'],
            'intent': 'original-intent', 'commitment': 'original-commitment', 'scope_cid': 'original-scope',
            'habit': 'dataplane-convergence', 'verification': ['elohim-storage'],
            'authority': {'path': str(self.authority.relative_to(self.root)),
                          'revocation': str((self.directory / 'revocation.json').relative_to(self.root))},
            'steps': [{'id': 'verify', 'kind': 'agent', 'prompt': 'Verify the original task.'}],
        }
        state.atomic(self.declaration, self.spec)
        self.ready = False
        self.attempts = []

        def read_input(argv, _root, **_kwargs):
            # Parsing/schema behavior has its own Node tests. This suite isolates
            # initialization persistence from installed runtime authentication.
            if argv[0] == 'node':
                return json.dumps(state.read(Path(argv[-1])))
            self.fail(f'unexpected initialization command: {argv}')

        def preflight(spec, directory, *, checks):
            self.attempts.append(checks)
            (checks / 'claude-canary.jsonl').write_text(f'canary attempt {len(self.attempts)}\n')
            state.atomic(checks / 'baseline.json', {'ready': self.ready, 'baseline': 0})
            if not self.ready:
                raise state.Refused('fixture readiness failed')
            return {'checks': str(checks), 'measured': 0, 'revision': 'b' * 40}

        for name, replacement in [('run', read_input), ('preflight', preflight),
                                   ('launch', lambda directory, ownership: state.read(directory / 'contract.json'))]:
            patcher = patch.object(shift, name, replacement)
            patcher.start()
            self.addCleanup(patcher.stop)

    def fail_readiness(self):
        with self.assertRaisesRegex(state.Refused, 'fixture readiness failed'):
            shift.start(self.declaration, 'codex')

    def test_matching_retry_preserves_original_contract_time_and_attempt_evidence(self):
        self.fail_readiness()
        initialized = (self.directory / 'initialization.json').read_bytes()
        frozen = (self.directory / 'objective.json').read_bytes()
        first = {p.name: p.read_bytes() for p in self.attempts[0].iterdir()}
        self.ready = True
        contract = shift.start(self.declaration, 'claude')
        self.assertEqual(len(self.attempts), 2)
        self.assertNotEqual(self.attempts[0], self.attempts[1])
        self.assertEqual(first, {p.name: p.read_bytes() for p in self.attempts[0].iterdir()})
        self.assertEqual((self.directory / 'initialization.json').read_bytes(), initialized)
        self.assertEqual((self.directory / 'objective.json').read_bytes(), frozen)
        self.assertEqual(contract['started_at'], json.loads(initialized)['spec']['started_at'])
        self.assertEqual(contract['deadline'], self.deadline)
        self.assertEqual(contract['scope_cid'], 'original-scope')
        self.assertEqual(contract['baseline']['checks'], str(self.attempts[1]))

    def test_changed_scope_authority_or_deadline_cannot_restart_readiness(self):
        self.fail_readiness()
        declaration = self.declaration.read_bytes()
        authorization = self.authority.read_bytes()
        original = self.objective.read_bytes()
        for change in ['scope', 'authority', 'deadline', 'objective', 'formatting']:
            with self.subTest(change=change):
                self.declaration.write_bytes(declaration)
                self.authority.write_bytes(authorization)
                self.objective.write_bytes(original)
                if change == 'scope':
                    altered = state.read(self.declaration)
                    altered['scope_cid'] = 'different-scope'
                    state.atomic(self.declaration, altered)
                elif change == 'authority':
                    altered = state.read(self.authority)
                    altered['landing'] = True
                    state.atomic(self.authority, altered)
                elif change == 'deadline':
                    altered = state.read(self.declaration)
                    altered['deadline'] = dt.datetime.fromtimestamp(time.time() + 4000, dt.timezone.utc).isoformat()
                    state.atomic(self.declaration, altered)
                elif change == 'objective':
                    altered = state.read(self.objective)
                    altered['scope']['paths'] = ['**']
                    state.atomic(self.objective, altered)
                else:
                    self.declaration.write_bytes(declaration + b'\n')
                with self.assertRaises(state.Refused):
                    shift.start(self.declaration, 'codex')
                self.assertEqual(len(self.attempts), 1)

    def test_missing_or_corrupt_initialization_and_authority_are_not_recreated(self):
        self.fail_readiness()
        for name in ['initialization.json', 'initialization-checkpoint.json', 'objective.json',
                     'authority.json', 'revocation.json']:
            target = self.directory / name
            original = target.read_bytes()
            for corruption in ['missing', 'corrupt']:
                with self.subTest(name=name, corruption=corruption):
                    if corruption == 'missing':
                        target.unlink()
                    else:
                        target.write_text('{broken')
                    with self.assertRaises((state.Refused, OSError)):
                        shift.start(self.declaration, 'codex')
                    self.assertEqual(len(self.attempts), 1)
                    target.write_bytes(original)

    def test_clock_regression_refuses_without_lowering_the_saved_floor(self):
        now = time.time()
        with patch.object(shift.time, 'time', return_value=now + 60):
            self.fail_readiness()
        checkpoint = self.directory / 'initialization-checkpoint.json'
        floor = state.read(checkpoint)['last_wall']
        with patch.object(shift.time, 'time', return_value=now + 30):
            with self.assertRaisesRegex(state.Refused, 'clock regression'):
                shift.start(self.declaration, 'codex')
        self.assertEqual(state.read(checkpoint)['last_wall'], floor)
        self.assertEqual(len(self.attempts), 1)

    def test_revocation_stop_and_expiry_refuse_new_readiness_attempts(self):
        self.fail_readiness()
        state.atomic(self.directory / 'revocation.json', {'revoked': True})
        with self.assertRaisesRegex(state.Refused, 'revoked'):
            shift.start(self.declaration, 'codex')
        state.atomic(self.directory / 'revocation.json', {'revoked': False})
        state.atomic(self.directory / 'stop.json', {'reason': 'operator stop'})
        with self.assertRaisesRegex(state.Refused, 'stop marker'):
            shift.start(self.declaration, 'codex')
        (self.directory / 'stop.json').unlink()
        with patch.object(shift.time, 'time', return_value=time.time() + 4001):
            with self.assertRaisesRegex(state.Refused, 'deadline expired'):
                shift.start(self.declaration, 'codex')
        self.assertEqual(len(self.attempts), 1)

    def test_published_execution_is_never_rebuilt_from_initialization(self):
        self.ready = True
        shift.start(self.declaration, 'codex')
        for name in ['contract.json', 'checkpoint.json']:
            (self.directory / name).unlink()
        with self.assertRaisesRegex(state.Refused, 'publication already began'):
            shift.start(self.declaration, 'codex')
        self.assertEqual(len(self.attempts), 1)

    def test_partial_or_corrupt_execution_publication_requires_resume_not_start(self):
        self.fail_readiness()
        for name in ['contract.json', 'checkpoint.json']:
            path = self.directory / name
            path.write_text('{broken')
            with self.assertRaisesRegex(state.Refused, 'already has execution state'):
                shift.start(self.declaration, 'codex')
            path.unlink()
        self.assertEqual(len(self.attempts), 1)

    def test_unknown_legacy_objective_cannot_be_blessed_as_a_new_initialization(self):
        state.atomic(self.directory / 'objective.json', state.read(self.objective))
        with self.assertRaisesRegex(state.Refused, 'inspection only'):
            shift.start(self.declaration, 'codex')
        self.assertEqual(self.attempts, [])


if __name__ == '__main__':
    unittest.main()
