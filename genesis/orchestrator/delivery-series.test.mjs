/**
 * delivery-series — the push-delivers-within-budget habit's check.
 * Fixtures mirror orchestrator/dev #1891 (timed out with app ABORTED) and a
 * delivered app-only run. Run: node --test genesis/orchestrator/delivery-series.test.mjs
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  classifyRun,
  normalizeStages,
  phaseCases,
  STAGE_BOUNDS,
  stageRun,
  summarize,
  summarizeStages,
  stagesVerdict,
  verdict,
} from './delivery-series.mjs';

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

// ── --stages: per-stage wall clock and cost per delivered bundle (Lane D1) ────────────────
// Fixtures mirror elohim/dev #1725 (the 7200 s readiness wait inside "Publish and Verify App
// Delivery": 122.5 min, FAILED, 0 delivered) and a phased build shaped the way Lane A records
// concern-scoped phases in the junit report (classname elohim-app.deploy.<env>).
const describe1725 = {
  stages: [
    { name: 'Build App', status: 'SUCCESS', durationMillis: 54_000 },
    { name: 'Unit Test', status: 'SUCCESS', durationMillis: 120_000 },
    { name: 'Publish and Verify App Delivery', status: 'FAILED', durationMillis: 122.5 * 60_000 },
    { name: 'Build Image', status: 'NOT_EXECUTED', durationMillis: 0 },
    { name: 'Running', status: 'IN_PROGRESS', durationMillis: 5 },
  ],
};

test('normalizeStages speaks the pipeline-trajectory vocabulary', () => {
  const s = normalizeStages(describe1725);
  assert.deepEqual(s.map(x => x.result), ['SUCCESS', 'SUCCESS', 'FAILURE', 'NOT_BUILT', null]);
  assert.equal(s[2].durationMs, 122.5 * 60_000);
  assert.deepEqual(normalizeStages(null), []);
});

const run1725 = stageRun({ number: 1725, result: 'FAILURE', durationMs: 129.5 * 60_000 }, normalizeStages(describe1725), []);
const deliveredRun = n =>
  stageRun(
    { number: n, result: 'SUCCESS', durationMs: 30 * 60_000 },
    normalizeStages({ stages: [{ name: 'Publish and Verify App Delivery', status: 'SUCCESS', durationMillis: 8 * 60_000 }] }),
    []
  );

test('an unphased build falls back to the stage: the readiness wait is not split out', () => {
  assert.equal(run1725.split, 'stage');
  assert.equal(run1725.publishVerifyMin, 122.5);
  assert.equal(run1725.phases.readiness, null);
  assert.equal(run1725.delivered, 0);
  assert.equal(deliveredRun(1).delivered, 1);
});

const report = {
  suites: [
    {
      cases: [
        { className: 'elohim-app.deploy.dev', name: 'readiness doorway-alpha.elohim.host', status: 'PASSED', duration: 4 },
        { className: 'elohim-app.deploy.dev', name: 'publish.seed doorway-alpha.elohim.host elohim-app (browser)', status: 'PASSED', duration: 120 },
        { className: 'elohim-app.deploy.dev', name: 'publish.author elohim-app (browser)', status: 'PASSED', duration: 60 },
        { className: 'elohim-app.deploy.dev', name: 'verify.shell doorway-alpha.elohim.host', status: 'PASSED', duration: 30 },
        { className: 'elohim-app.deploy.dev', name: 'converge.declare elohim-app', status: 'FAILED', duration: 600 },
        { className: 'elohim-app.unit', name: 'publish.seed is not a deploy phase here', status: 'PASSED', duration: 999 },
      ],
    },
  ],
};

test('phaseCases reads only the deploy classname and the concern-scoped phase names', () => {
  const p = phaseCases(report);
  assert.equal(p.length, 5);
  assert.deepEqual(p.map(c => c.phase), ['readiness', 'publish', 'publish', 'verify', 'converge']);
  assert.deepEqual(phaseCases(null), []);
});

test('a phased build splits readiness / publish / verify and counts the verified bundle', () => {
  const r = stageRun({ number: 1800, result: 'UNSTABLE', durationMs: 40 * 60_000 }, [], phaseCases(report));
  assert.equal(r.split, 'phases');
  assert.equal(r.phases.readiness, 4 / 60);
  assert.equal(r.phases.publish, 3);
  assert.equal(r.phases.verify, 0.5);
  assert.equal(r.publishVerifyMin, 3.5);
  assert.equal(r.delivered, 1, 'verify.shell passed → one delivered bundle, converge is measured not gating');
});

test('a stage marked FAILED only because an earlier stage failed has no wall clock', () => {
  const r = stageRun(
    { number: 1717, result: 'FAILURE', durationMs: 4.9 * 60_000 },
    normalizeStages({
      stages: [
        { name: 'Unit Test', status: 'FAILED', durationMillis: 90_000 },
        { name: 'Publish and Verify App Delivery', status: 'FAILED', durationMillis: 160 },
      ],
    }),
    []
  );
  assert.equal(r.publishVerifyMin, null);
  assert.deepEqual(r.stages.map(s => s.name), ['Unit Test']);
});

test('a readiness refusal delivers nothing', () => {
  const refused = { suites: [{ cases: [{ className: 'elohim-app.deploy.dev', name: 'readiness doorway-alpha', status: 'FAILED', duration: 3 }] }] };
  const r = stageRun({ number: 1801, result: 'UNSTABLE', durationMs: 12 * 60_000 }, [], phaseCases(refused));
  assert.equal(r.delivered, 0);
  assert.equal(r.publishVerifyMin, null);
});

test('summarizeStages prices the #1719–#1725 incident: hours spent, none delivered', () => {
  const s = summarizeStages(Array.from({ length: 7 }, () => run1725), { window: 10 });
  assert.equal(s.considered, 7);
  assert.equal(s.delivered, 0);
  assert.equal(s.pipelineHours, 15.11);
  assert.equal(s.costPerDeliveredHours, null, 'no delivered bundle: cost is unbounded, never 0');
  assert.equal(s.publishVerify.p90Min, 122.5);
  assert.equal(s.stages['Publish and Verify App Delivery'].p90Min, 122.5);
  assert.equal(s.stages['Build Image'], undefined, 'a stage that never executed has no wall clock');
  assert.equal(stagesVerdict(s), 1);
});

test('a delivering series is within the declared bounds', () => {
  const s = summarizeStages([deliveredRun(4), deliveredRun(3), deliveredRun(2), deliveredRun(1)], { window: 10 });
  assert.equal(s.delivered, 4);
  assert.equal(s.costPerDeliveredHours, 0.5);
  assert.equal(s.publishVerify.p50Min, 8);
  assert.equal(stagesVerdict(s), 0);
  assert.equal(stagesVerdict(s, { maxPublishVerifyP90Min: 5 }), 1);
});

test('too few builds is never read as health', () => {
  assert.equal(stagesVerdict(summarizeStages([deliveredRun(1)], { window: 10 })), 2);
});

test('the --stages defaults are the bounds declared in measures.yaml, not a second copy', () => {
  const text = readFileSync(new URL('../../.claude/epr-meta/measures.yaml', import.meta.url), 'utf8');
  const hard = id => {
    const block = text.split(/\n(?=  - id: )/).find(b => b.startsWith(`  - id: ${id}\n`));
    assert.ok(block, `${id} is declared`);
    return Number(block.match(/\n    hard: ([0-9.]+)/)[1]);
  };
  assert.equal(STAGE_BOUNDS.maxPublishVerifyP90Min, hard('stage-wallclock-ceiling'));
  assert.equal(STAGE_BOUNDS.maxCostHours, hard('delivery-cost-ceiling'));
});

test('a stage absent from a build is no sample: p50/p90 over present samples only, n per stage', () => {
  // A phased build carries a stage the unsplit builds never ran; a refused phased build has
  // no verify leg. Absence must shrink n, never enter the percentile as NaN.
  const phased = (n, readinessWaitMin, cases) =>
    stageRun(
      { number: n, result: 'UNSTABLE', durationMs: 40 * 60_000 },
      normalizeStages({
        stages: [
          { name: 'Build App', status: 'SUCCESS', durationMillis: 60_000 },
          { name: 'Await Doorway Readiness', status: 'SUCCESS', durationMillis: readinessWaitMin * 60_000 },
        ],
      }),
      phaseCases({ suites: [{ cases }] })
    );
  const leg = (name, status, duration) => ({ className: 'elohim-app.deploy.dev', name, status, duration });
  const ok = phased(1900, 4, [leg('readiness a', 'PASSED', 60), leg('publish.seed a', 'PASSED', 120), leg('verify.shell a', 'PASSED', 30)]);
  const refused = phased(1899, 12, [leg('readiness a', 'FAILED', 300)]);
  const unsplit = stageRun(
    { number: 1898, result: 'SUCCESS', durationMs: 30 * 60_000 },
    normalizeStages({
      stages: [
        { name: 'Build App', status: 'SUCCESS', durationMillis: 120_000 },
        { name: 'Publish and Verify App Delivery', status: 'SUCCESS', durationMillis: 8 * 60_000 },
      ],
    }),
    []
  );
  const s = summarizeStages([ok, refused, unsplit, deliveredRun(1897)], { window: 10 });

  const finiteOrNull = v => v === null || Number.isFinite(v);
  for (const [name, st] of [...Object.entries(s.stages), ...Object.entries(s.phases), ['publishVerify', s.publishVerify]]) {
    assert.ok(Number.isInteger(st.n), `${name}: n is a count`);
    assert.ok(finiteOrNull(st.p50Min) && finiteOrNull(st.p90Min), `${name}: p50/p90 finite or null, never NaN`);
    if (st.n === 0) assert.deepEqual([st.p50Min, st.p90Min], [null, null], `${name}: n=0 reads as absent, not a number`);
  }
  assert.ok(!/NaN/.test(JSON.stringify(s)), 'no NaN reaches the JSON (it would serialize as null)');

  assert.deepEqual(s.stages['Await Doorway Readiness'], { n: 2, p50Min: 4, p90Min: 12 }, 'only the two phased builds ran it');
  assert.deepEqual(s.stages['Publish and Verify App Delivery'], { n: 2, p50Min: 8, p90Min: 8 });
  assert.deepEqual(s.stages['Build App'], { n: 3, p50Min: 1, p90Min: 2 });
  assert.deepEqual(s.phases.readiness, { n: 2, p50Min: 1, p90Min: 5 });
  assert.deepEqual(s.phases.verify, { n: 1, p50Min: 0.5, p90Min: 0.5 }, 'the refused build ran no verify leg');
  assert.equal(s.phases.converge.n, 0);

  const allUnsplit = summarizeStages([unsplit, deliveredRun(1)], { window: 10 });
  assert.deepEqual(allUnsplit.phases.readiness, { n: 0, p50Min: null, p90Min: null });
});
