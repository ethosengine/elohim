import { strict as assert } from 'node:assert';
import { test } from 'node:test';

import { featureSummary } from '../jenkins-feature-summary.js';
import { HOUSEHOLD_ONLY_HOLD_REASON } from '../lib/household-only.js';

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

void test('counts skipped household-only scenarios separately without changing verdict counts', () => {
  const report = JSON.stringify([
    {
      uri: 'features/dataplane/epr-app-deliverability.feature',
      elements: [
        {
          type: 'scenario',
          name: 'held household',
          before: [
            {
              result: { status: 'skipped' },
              embeddings: [
                {
                  data: Buffer.from(HOUSEHOLD_ONLY_HOLD_REASON).toString('base64'),
                  mime_type: 'text/plain',
                },
              ],
            },
          ],
          steps: [{ result: { status: 'skipped' } }],
        },
        { type: 'scenario', name: 'failed household', steps: [{ result: { status: 'failed' } }] },
      ],
    },
    {
      uri: 'features/dataplane/doorway-fixture-readiness.feature',
      elements: [
        { type: 'scenario', name: 'generic hold', steps: [{ result: { status: 'skipped' } }] },
      ],
    },
  ]);
  const summary = featureSummary(lane => (lane === 'api' ? report : undefined));
  assert.match(
    summary,
    /^api: 0\/3 scenarios passed, 1 failed, 2 skipped \(1 household-only\), 0 pending\/undefined/
  );
});
