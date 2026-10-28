"""Native attribution eligibility refuses before paid readiness or new execution."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import shift as launcher
import shift_state as state


class AttributionReadiness(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='shift-attribution-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / '.git').touch()
        (self.root / 'ark').touch()
        self.spec = {'id': 'fixture', 'worktree': str(self.root), 'ark': str(self.root / 'ark'),
                     'allowed_tools': [], 'verification': ['fixture'], 'scope_cid': 'scope',
                     'intent': 'intent', 'commitment': 'commitment',
                     'steps': [{'id': 'gate', 'kind': 'gate', 'project': 'fixture'}]}
        self.promise = {'commitment': 'commitment', 'attribution': {
            'eligible': True, 'actor_claim': 'exact-registered-claim', 'witness': None, 'issues': []}}

    def payload(self):
        return json.dumps({'reconciliation': {'scope_cid': 'scope', 'assertions': [
            {'intent': 'intent', 'commitments': [self.promise]}]}})

    def test_exact_native_direct_or_witnessed_eligibility_is_returned_without_inference(self):
        for witness in [None, 'exact-recorded-binding']:
            self.promise['attribution']['witness'] = witness
            with self.subTest(witness=witness), patch.object(state, 'run', return_value=self.payload()):
                _, observed = state.native_context(self.spec)
                self.assertEqual(observed, self.promise)

    def test_missing_old_native_field_and_invalid_eligibility_refuse(self):
        candidates = [None, {}, {'eligible': True},
                      {'eligible': False, 'actor_claim': None, 'witness': None,
                       'issues': ['implementer has no exact registered actor claim']},
                      {'eligible': True, 'actor_claim': 'actor', 'witness': 'witness',
                       'issues': ['conflicting attribution witness']},
                      {'eligible': 'true', 'actor_claim': 'actor', 'witness': None, 'issues': []},
                      {'eligible': True, 'actor_claim': None, 'witness': None, 'issues': []}]
        for attribution in candidates:
            with self.subTest(attribution=attribution):
                if attribution is None:
                    self.promise.pop('attribution', None)
                else:
                    self.promise['attribution'] = attribution
                with patch.object(state, 'run', return_value=self.payload()):
                    with self.assertRaisesRegex(state.Refused, 'native .*attribution'):
                        state.native_context(self.spec)

    def test_ineligible_commitment_stops_before_canaries_leases_and_baseline(self):
        self.promise['attribution'] = {'eligible': False, 'actor_claim': None, 'witness': None,
                                      'issues': ['implementer has no exact registered actor claim']}
        with patch.object(launcher.shutil, 'which', return_value='/fixture/tool'), \
                patch.object(launcher, 'initialization_remaining', return_value=100), \
                patch.object(state, 'run', return_value=self.payload()), \
                patch.object(launcher, 'bounded') as execute, \
                patch.object(launcher, 'lease') as claim:
            with self.assertRaisesRegex(state.Refused, 'not acceptance-eligible'):
                launcher.preflight(self.spec, self.root / '.claude/shifts/fixture')
            execute.assert_not_called()
            claim.assert_not_called()
        self.assertFalse((self.root / '.claude').exists())


if __name__ == '__main__':
    unittest.main()
