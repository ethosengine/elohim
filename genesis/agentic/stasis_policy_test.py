"""Slice 6 default parity, schema and steward-only pin preparation contracts."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest

import jsonschema


ROOT = Path(__file__).resolve().parents[2]
POLICY = Path('.epr-meta/elohim/policies/dialogue-stasis-policy.json')
SCHEMA = ROOT / 'elohim/sdk/domains/elohim-agent/schemas/dialogue-stasis-policy.schema.json'
SPEC = ROOT / 'genesis/docs/superpowers/specs/2026-10-30-dialogue-stasis-loop-design.md'
RUST = ROOT / 'elohim/eprfs/device-runtime/src/session_stasis.rs'
CLASSIFIER = RUST.with_suffix('') / 'classify.rs'
PIN = ROOT / 'genesis/agentic/bin/stasis-policy-pin'


class PolicyTests(unittest.TestCase):
    def setUp(self):
        self.policy = json.loads((ROOT / POLICY).read_text())

    def test_schema_and_relative_reference(self):
        schema = json.loads(SCHEMA.read_text())
        jsonschema.Draft202012Validator.check_schema(schema)
        jsonschema.validate(self.policy, schema)
        self.assertEqual(((ROOT / POLICY).parent / self.policy['$schema']).resolve(), SCHEMA)
        for key, bad in [('dwellS', -1), ('budgets', {}), ('unrecognized', True)]:
            with self.subTest(key=key), self.assertRaises(jsonschema.ValidationError):
                jsonschema.validate({**self.policy, key: bad}, schema)

    def test_every_native_numeric_default_from_rust_source(self):
        # Deliberately narrow source-contract parser: a refactor fails loudly,
        # never silently falls back to numbers copied into this test.
        body = RUST.read_text().split('pub fn builtin() -> Self {', 1)[1].split('pub fn hysteresis', 1)[0]
        fields = {'incident_keeper_calls': 'incidentKeeperCalls', 'incident_elapsed_s': 'incidentElapsedS',
                  'incident_tokens': 'incidentTokens', 'shift_keeper_s': 'shiftKeeperS',
                  'shift_tokens': 'shiftTokens'}
        for rust, wire in fields.items():
            value = re.findall(rf'\b{rust}: ([\d_.]+),', body)
            self.assertEqual(len(value), 1, rust)
            self.assertEqual(self.policy['budgets'][wire], float(value[0].replace('_', '')))
        for rust, wire in [('collaborative_window_s', 'collaborativeWindowS'), ('dwell_s', 'dwellS')]:
            value = re.findall(rf'\b{rust}: ([\d_.]+),', body)
            self.assertEqual(len(value), 1, rust)
            self.assertEqual(self.policy[wire], float(value[0].replace('_', '')))
        for rust, wire in [('stall', 'stall'), ('shift_ceiling', 'shiftCeiling')]:
            matches = re.findall(rf'{rust}: Bound::new\(([\d.]+), "([^"]+)"\.into\(\), ([\d.]+)\)', body)
            self.assertEqual(len(matches), 1, rust)
            limit, unit, threshold = matches[0]
            self.assertEqual(self.policy[wire], {'limit': float(limit), 'unit': unit, 'thresholdPct': float(threshold)})
        self.assertEqual(self.policy['stall']['limit'] * self.policy['stall']['thresholdPct'] / 100, 180)

    def test_never_list_verbatim_and_promotion_from_spec(self):
        table = SPEC.read_text().split('## Policy boundary and learning', 1)[1].split('**Live operation**', 1)[0]
        invariants = re.findall(r'^\| (Never [^|]+?) \|', table, re.M)
        self.assertEqual([row['invariant'] for row in self.policy['never']], invariants)
        self.assertEqual(len({row['id'] for row in self.policy['never']}), 7)
        promotion = self.policy['promotionCriteria']
        self.assertEqual(promotion['cleanProceduralReshapeResolutions'], 2)
        self.assertEqual(promotion['trigger'], 'propose')
        self.assertEqual(promotion['activationRequires'],
                         ['completed-passing-other-family-audit', 'steward-standing-ruling'])

    def test_classifier_rule_set_from_rust_source(self):
        source = CLASSIFIER.read_text()
        shapes = source.split('pub fn shape_patterns(', 1)[1].split('    found\n}', 1)[0]
        self.assertEqual(self.policy['ruleSet']['shapePatterns'], re.findall(r'found.push\("([^"]+)"\)', shapes))
        self.assertIn('[("deny", RuleKind::Deny), ("ask", RuleKind::Ask)]', source)
        self.assertEqual(self.policy['ruleSet']['riskRules'], ['deny', 'ask'])
        self.assertEqual(self.policy['ruleSet']['uncertainAs'], 'risk')

    def test_pin_only_prints_steward_command(self):
        with tempfile.TemporaryDirectory() as scratch:
            repo = Path(scratch)
            path = repo / POLICY
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps(self.policy))
            fake = repo / 'native-cid'
            cid = 'bafkrei' + 'a' * 52
            fake.write_text('#!/usr/bin/env python3\nimport sys\n'
                            'assert sys.argv[1] == "cid" and len(sys.argv) == 3\n'
                            f'print({cid!r})\n')
            fake.chmod(0o700)
            env = {**os.environ, 'EPRFS_BIN': str(fake), 'EPR_BIN': '/not-executed/epr'}
            result = subprocess.run([sys.executable, str(PIN), '--repo', str(repo)], env=env,
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn(cid, result.stdout)
            self.assertIn('--kind ruling', result.stdout)
            self.assertIn('policyCid', result.stdout)
            self.assertFalse((repo / '.eprfs').exists())
            env['EPRFS_BIN'] = str(repo / 'missing')
            missing = subprocess.run([sys.executable, str(PIN), '--repo', str(repo)], env=env,
                                     capture_output=True, text=True)
            self.assertEqual(missing.returncode, 2)
            self.assertNotIn('--kind ruling', missing.stdout)


if __name__ == '__main__':
    unittest.main()
