/**
 * delivery-series — the push-delivers-within-budget habit's check.
 * Fixtures mirror orchestrator/dev #1891 (timed out with app ABORTED) and a
 * delivered app-only run. Run: node --test genesis/orchestrator/delivery-series.test.mjs
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { classifyRun, summarize, verdict } from './delivery-series.mjs';

const run1891 = classifyRun(
  { number: 1891, result: 'ABORTED', durationMs: 244.5 * 60_000 },
  { pipelines: ['elohim', 'elohim-edge', 'elohim-holochain', 'elohim-genesis'] },
  {
    executionOrder: ['elohim', 'elohim-edge', 'elohim-holochain', 'elohim-genesis'],
    results: {
      'elohim-holochain': { result: 'SUCCESS' },
      'elohim-edge': { result: 'SUCCESS' },
      elohim: { result: 'ABORTED', error: 'Timeout has been exceeded' },
    },
    abortedBeforeStart: ['elohim-genesis'],
  }
);

const delivered = n =>
  classifyRun(
    { number: n, result: 'SUCCESS', durationMs: 40 * 60_000 },
    null,
    { executionOrder: ['elohim'], results: { elohim: { result: 'UNSTABLE' } } }
  );

test('a run whose planned pipeline was cut off is not delivered, and is a timeout', () => {
  assert.equal(run1891.delivered, false);
  assert.equal(run1891.timedOut, true);
  assert.equal(run1891.outcome['elohim-genesis'], 'NOT_DISPATCHED');
  assert.equal(run1891.minutes, 244.5);
});

test('UNSTABLE counts as delivered; no-op runs say nothing', () => {
  assert.equal(delivered(1).delivered, true);
  assert.equal(classifyRun({ number: 2, result: 'SUCCESS', durationMs: 30_000 }, { pipelines: [] }, null), null);
});

test('a fire-and-forget pipeline the orchestrator never awaited does not block delivery', () => {
  const r = classifyRun({ number: 3, result: 'SUCCESS', durationMs: 60_000 }, null, {
    executionOrder: ['elohim', 'elohim-storybook'],
    results: { elohim: { result: 'SUCCESS' }, 'elohim-storybook': { result: 'DISPATCHED' } },
  });
  assert.equal(r.delivered, true);
});

test('an orchestrator aborted before archiving its graph is an undelivered timeout', () => {
  const r = classifyRun({ number: 1884, result: 'ABORTED', durationMs: 251.7 * 60_000 }, { pipelines: ['elohim-edge'] }, null);
  assert.equal(r.delivered, false);
  assert.equal(r.timedOut, true);
});

test('verdict: the 09-19..09-22 series is outside bounds; a delivering series is within', () => {
  const starved = summarize(Array.from({ length: 8 }, () => run1891), { window: 10 });
  assert.equal(starved.delivered, 0);
  assert.equal(verdict(starved), 1);

  const healthy = summarize([delivered(10), delivered(9), delivered(8), run1891, delivered(7)], { window: 5 });
  assert.equal(healthy.rate, 0.8);
  assert.equal(healthy.p90DeliveredMin, 40);
  assert.equal(verdict(healthy), 0);
});

test('too little evidence is never read as health', () => {
  assert.equal(verdict(summarize([delivered(1)], { window: 10 })), 2);
  assert.equal(verdict(summarize([null, null], { window: 10 })), 2);
});
