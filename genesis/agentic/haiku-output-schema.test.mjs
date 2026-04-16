import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import Ajv from 'ajv';

const schema = JSON.parse(
  readFileSync('.claude/schemas/haiku-output.schema.json', 'utf8'),
);
const validate = new Ajv({ allErrors: true, strict: false }).compile(schema);

test('valid Haiku finding passes', () => {
  const out = {
    iteration: 4,
    measurement: { value: 0.18, delta: 0.03, baseline: 0.05, target: 0.2 },
    context: {
      build_id: 1247,
      status: 'failed',
      first_failing_stage: 'alpha-deploy',
    },
    primary_failure: {
      error_class: 'StatefulSet field forbidden',
      evidence: 'forbidden: updates to .spec.volumeClaimTemplates are forbidden',
      files_mentioned: ['genesis/orchestrator/manifests/doorway/alpha.yaml'],
    },
    observed_anti_patterns: [
      {
        pattern: 'full kubectl describe dump',
        evidence: '3200 lines; only 8 matter',
      },
    ],
    confidence: 'medium',
  };
  assert.equal(validate(out), true, JSON.stringify(validate.errors));
});

test('rejects confidence outside enum', () => {
  const out = {
    iteration: 1,
    measurement: { value: 0, delta: 0, baseline: 0, target: 1 },
    context: { build_id: 1, status: 'passed', first_failing_stage: null },
    primary_failure: null,
    observed_anti_patterns: [],
    confidence: 'certain',
  };
  assert.equal(validate(out), false);
});

test('primary_failure nullable when status passed', () => {
  const out = {
    iteration: 1,
    measurement: { value: 1, delta: 0, baseline: 0, target: 1 },
    context: { build_id: 1, status: 'passed', first_failing_stage: null },
    primary_failure: null,
    observed_anti_patterns: [],
    confidence: 'high',
  };
  assert.equal(validate(out), true, JSON.stringify(validate.errors));
});

test('observed_anti_patterns may be empty array', () => {
  const out = {
    iteration: 1,
    measurement: { value: 0, delta: 0, baseline: 0, target: 1 },
    context: { build_id: 1, status: 'failed', first_failing_stage: 'x' },
    primary_failure: {
      error_class: 'x',
      evidence: 'x',
      files_mentioned: [],
    },
    observed_anti_patterns: [],
    confidence: 'high',
  };
  assert.equal(validate(out), true, JSON.stringify(validate.errors));
});
