import { strict as assert } from 'node:assert';
import { test } from 'node:test';

import { featureSummary } from '../jenkins-feature-summary.js';

void test('preserves separate lane results and missing evidence', () => {
  const report = JSON.stringify([
    {
      uri: 'features/mesh.feature',
      elements: [
        { type: 'scenario', name: 'ran', steps: [{ result: { status: 'passed' } }] },
        { type: 'scenario', name: 'held', steps: [{ result: { status: 'skipped' } }] },
      ],
    },
  ]);
  const summary = featureSummary(lane => (lane === 'api' ? report : undefined));
  assert.equal(
    summary,
    'api: 1/2 scenarios passed, 0 failed, 1 skipped, 0 pending/undefined | browser: not measured (no report) | peer-stage: not measured (no report)'
  );
});

void test('empty, malformed, and unreadable reports never claim a pass', () => {
  const summary = featureSummary(lane => {
    if (lane === 'api') return '[]';
    if (lane === 'browser') return 'not json';
    throw new Error('read refused');
  });
  assert.equal(
    summary,
    'api: not measured (empty report) | browser: not measured (unreadable report) | peer-stage: not measured (unreadable report)'
  );
});
