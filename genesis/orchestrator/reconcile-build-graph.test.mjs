// Tests for reconcile-build-graph.mjs.
//
// The reconciler compares what the orchestrator PREDICTED would run
// (predicted-build-graph.json from Determine Build Plan) against what
// ACTUALLY happened (actual-build-graph.json from Execute Builds).
//
// Drift between the two is the highest-value signal for pipeline quality —
// every disconnect is a candidate for investigation toward predictive,
// compiler-validated builds.

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

import { reconcile } from './reconcile-build-graph.mjs';

// ── Fixtures ────────────────────────────────────────────────────────

function predictedFixture(overrides = {}) {
  return {
    schemaVersion: '1',
    predictedAt: '2026-05-09T03:27:30Z',
    commitSha: 'abc123',
    branch: 'dev',
    mode: 'auto',
    changedFileCount: 12,
    skipCi: false,
    forceBuildPipelines: [],
    deployOnly: false,
    pipelines: ['elohim-holochain', 'elohim-edge', 'elohim', 'elohim-genesis'],
    levels: [['elohim-holochain'], ['elohim-edge', 'elohim']],
    genesisAfterLevels: true,
    analysis: {
      'elohim-holochain': {
        shouldRun: true,
        matchedPatterns: ['elohim/holochain/dna/'],
        matchedFileCount: 3,
      },
      'elohim-edge': {
        shouldRun: true,
        matchedPatterns: ['dependency:elohim-holochain'],
        matchedFileCount: 0,
      },
      elohim: {
        shouldRun: true,
        matchedPatterns: ['app/elohim-app/'],
        matchedFileCount: 5,
      },
      'elohim-genesis': {
        shouldRun: true,
        matchedPatterns: ['triggered-by:elohim-edge'],
        matchedFileCount: 0,
      },
    },
    ...overrides,
  };
}

function actualFixture(overrides = {}) {
  return {
    schemaVersion: '1',
    executedAt: '2026-05-09T06:21:30Z',
    commitSha: 'abc123',
    branch: 'dev',
    totalDurationMs: 10444666,
    results: {
      'elohim-holochain': { result: 'SUCCESS', durationMs: 4080000, level: 0 },
      'elohim-edge': { result: 'UNSTABLE', durationMs: 1980000, level: 1 },
      elohim: { result: 'SUCCESS', durationMs: 1140000, level: 1 },
      'elohim-genesis': { result: 'ABORTED', durationMs: 3960000, level: 'genesis-trailer' },
    },
    abortedBeforeStart: [],
    ...overrides,
  };
}

// ── Verdict and disconnect classification ──────────────────────────

describe('reconcile — clean match', () => {
  it('emits verdict=match when every predicted pipeline ran with SUCCESS', () => {
    const predicted = predictedFixture();
    const actual = actualFixture({
      results: {
        'elohim-holochain': { result: 'SUCCESS', durationMs: 4080000, level: 0 },
        'elohim-edge': { result: 'SUCCESS', durationMs: 1980000, level: 1 },
        elohim: { result: 'SUCCESS', durationMs: 1140000, level: 1 },
        'elohim-genesis': { result: 'SUCCESS', durationMs: 600000, level: 'genesis-trailer' },
      },
    });

    const r = reconcile({ predicted, actual });

    assert.equal(r.verdict, 'match');
    assert.deepEqual(r.disconnects.predictedNotExecuted, []);
    assert.deepEqual(r.disconnects.executedNotPredicted, []);
    assert.deepEqual(r.disconnects.abortedAfterStart, []);
    assert.deepEqual(r.disconnects.unstableResults, []);
  });
});

describe('reconcile — predictedNotExecuted', () => {
  it('flags pipelines that were predicted but never started (cascade abort)', () => {
    const predicted = predictedFixture();
    const actual = actualFixture({
      results: {
        'elohim-holochain': { result: 'FAILURE', durationMs: 800000, level: 0 },
      },
      abortedBeforeStart: ['elohim-edge', 'elohim', 'elohim-genesis'],
    });

    const r = reconcile({ predicted, actual });

    assert.equal(r.verdict, 'drift');
    assert.deepEqual(
      r.disconnects.predictedNotExecuted.sort(),
      ['elohim', 'elohim-edge', 'elohim-genesis'],
    );
  });
});

describe('reconcile — executedNotPredicted', () => {
  it('flags pipelines that ran but were not in the prediction', () => {
    const predicted = predictedFixture({
      pipelines: ['elohim'],
      levels: [['elohim']],
      analysis: {
        elohim: { shouldRun: true, matchedPatterns: ['app/elohim-app/'], matchedFileCount: 5 },
      },
    });
    const actual = actualFixture({
      results: {
        elohim: { result: 'SUCCESS', durationMs: 1140000, level: 0 },
        'elohim-edge': { result: 'SUCCESS', durationMs: 1980000, level: 0 },
        'elohim-genesis': { result: 'SUCCESS', durationMs: 600000, level: 'genesis-trailer' },
      },
    });

    const r = reconcile({ predicted, actual });

    assert.equal(r.verdict, 'drift');
    assert.deepEqual(
      r.disconnects.executedNotPredicted.sort(),
      ['elohim-edge', 'elohim-genesis'],
    );
  });
});

describe('reconcile — abortedAfterStart', () => {
  it('flags pipelines that started but did not finish successfully (ABORTED, FAILURE)', () => {
    const predicted = predictedFixture();
    const actual = actualFixture();

    const r = reconcile({ predicted, actual });

    assert.equal(r.verdict, 'drift');
    assert.deepEqual(r.disconnects.abortedAfterStart, ['elohim-genesis']);
    assert.deepEqual(r.disconnects.unstableResults, ['elohim-edge']);
  });
});

describe('reconcile — schema validation', () => {
  it('throws when predicted is missing pipelines field', () => {
    assert.throws(
      () => reconcile({ predicted: { schemaVersion: '1' }, actual: actualFixture() }),
      /predicted\.pipelines/,
    );
  });

  it('throws when actual is missing results field', () => {
    assert.throws(
      () => reconcile({ predicted: predictedFixture(), actual: { schemaVersion: '1' } }),
      /actual\.results/,
    );
  });
});

describe('reconcile — commit SHA mismatch', () => {
  it('flags commit-sha drift as a critical disconnect', () => {
    const predicted = predictedFixture({ commitSha: 'abc123' });
    const actual = actualFixture({ commitSha: 'def456' });

    const r = reconcile({ predicted, actual });

    assert.equal(r.verdict, 'drift');
    assert.match(r.summary, /commit sha drift/i);
    assert.equal(r.disconnects.commitShaDrift, true);
  });
});

describe('reconcile — skip-ci edge case', () => {
  it('handles predicted=skipped, actual=empty as match', () => {
    const predicted = predictedFixture({
      skipCi: true,
      pipelines: [],
      levels: [],
      genesisAfterLevels: false,
      analysis: {},
    });
    const actual = actualFixture({ results: {}, abortedBeforeStart: [] });

    const r = reconcile({ predicted, actual });

    assert.equal(r.verdict, 'match');
    assert.deepEqual(r.disconnects.predictedNotExecuted, []);
    assert.deepEqual(r.disconnects.executedNotPredicted, []);
  });

  it('flags drift if skipped predicted but pipelines actually ran', () => {
    const predicted = predictedFixture({
      skipCi: true,
      pipelines: [],
      levels: [],
      analysis: {},
    });
    const actual = actualFixture({
      results: {
        'elohim-edge': { result: 'SUCCESS', durationMs: 1980000, level: 0 },
      },
    });

    const r = reconcile({ predicted, actual });

    assert.equal(r.verdict, 'drift');
    assert.deepEqual(r.disconnects.executedNotPredicted, ['elohim-edge']);
  });
});

describe('reconcile — investigation pointers', () => {
  it('produces a pointer for each aborted/failed pipeline with its url', () => {
    const predicted = predictedFixture();
    const actual = actualFixture({
      results: {
        'elohim-holochain': { result: 'SUCCESS', durationMs: 4080000, level: 0 },
        'elohim-edge': { result: 'UNSTABLE', durationMs: 1980000, level: 1 },
        elohim: { result: 'SUCCESS', durationMs: 1140000, level: 1 },
        'elohim-genesis': {
          result: 'ABORTED',
          durationMs: 3960000,
          level: 'genesis-trailer',
          url: 'https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1002/',
        },
      },
    });

    const r = reconcile({ predicted, actual });

    assert.ok(r.investigationPointers.length >= 1);
    assert.ok(
      r.investigationPointers.some(p => p.includes('elohim-genesis') && p.includes('1002')),
      `expected genesis pointer with build URL; got ${JSON.stringify(r.investigationPointers)}`,
    );
  });

  it('omits pointers when actual is a clean SUCCESS sweep', () => {
    const predicted = predictedFixture();
    const actual = actualFixture({
      results: {
        'elohim-holochain': { result: 'SUCCESS', durationMs: 4080000, level: 0 },
        'elohim-edge': { result: 'SUCCESS', durationMs: 1980000, level: 1 },
        elohim: { result: 'SUCCESS', durationMs: 1140000, level: 1 },
        'elohim-genesis': { result: 'SUCCESS', durationMs: 600000, level: 'genesis-trailer' },
      },
    });

    const r = reconcile({ predicted, actual });

    assert.equal(r.investigationPointers.length, 0);
  });
});

describe('reconcile — summary text', () => {
  it('summarises disconnect categories with counts', () => {
    const predicted = predictedFixture();
    const actual = actualFixture();

    const r = reconcile({ predicted, actual });

    // From the default fixtures: 1 ABORTED (genesis), 1 UNSTABLE (edge), 0 dispatch drift.
    assert.match(r.summary, /1 aborted/i);
    assert.match(r.summary, /1 unstable/i);
  });
});

describe('reconcile — stageAnnotations pointers', () => {
  it('surfaces conductor-readiness unready peers as pointers', () => {
    const predicted = predictedFixture();
    const actual = actualFixture({
      results: {
        'elohim-holochain': { result: 'SUCCESS', durationMs: 4080000, level: 0 },
        'elohim-edge': { result: 'SUCCESS', durationMs: 1980000, level: 1 },
        elohim: { result: 'SUCCESS', durationMs: 1140000, level: 1 },
        'elohim-genesis': {
          result: 'UNSTABLE',
          durationMs: 600000,
          level: 'genesis-trailer',
          stageAnnotations: {
            conductorReadiness: {
              schemaVersion: '1',
              stage: 'Seed Agent Peer Bindings',
              readyCount: 2,
              unreadyCount: 1,
              partial: true,
              allReady: false,
              allUnready: false,
              ready: [],
              unready: [
                { url: 'ws://elohim-timothy-alpha:4445', host: 'elohim-timothy-alpha', adminPort: 4444, reason: 'tcp-timeout-or-refused' },
              ],
            },
          },
        },
      },
    });

    const r = reconcile({ predicted, actual });

    assert.ok(
      r.investigationPointers.some(p => p.includes('conductor-readiness') && p.includes('elohim-timothy-alpha')),
      `expected conductor-readiness pointer naming timothy; got ${JSON.stringify(r.investigationPointers)}`,
    );
  });

  it('surfaces seed-results partial humans as pointers', () => {
    const predicted = predictedFixture();
    const actual = actualFixture({
      results: {
        'elohim-holochain': { result: 'SUCCESS', durationMs: 4080000, level: 0 },
        'elohim-edge': { result: 'SUCCESS', durationMs: 1980000, level: 1 },
        elohim: { result: 'SUCCESS', durationMs: 1140000, level: 1 },
        'elohim-genesis': {
          result: 'UNSTABLE',
          durationMs: 600000,
          level: 'genesis-trailer',
          stageAnnotations: {
            seedResultsConductorIdentities: {
              schemaVersion: '1',
              counts: { created: 2, exists: 0, failed: 1, succeeded: 2, total: 3 },
              partial: true,
              allSucceeded: false,
              allFailed: false,
              results: [
                { humanId: 'human-matthew-manager', result: 'created' },
                { humanId: 'human-jessica-spouse', result: 'created' },
                { humanId: 'human-timothy-tutor', result: 'failed', error: 'admin WS timeout' },
              ],
            },
          },
        },
      },
    });

    const r = reconcile({ predicted, actual });

    assert.ok(
      r.investigationPointers.some(p => p.includes('seed-conductor-identities') && p.includes('partial') && p.includes('human-timothy-tutor')),
      `expected seed-conductor-identities partial pointer naming timothy; got ${JSON.stringify(r.investigationPointers)}`,
    );
  });

  it('omits stageAnnotations pointers when annotations are clean (allReady + allSucceeded)', () => {
    const predicted = predictedFixture();
    const actual = actualFixture({
      results: {
        'elohim-holochain': { result: 'SUCCESS', durationMs: 4080000, level: 0 },
        'elohim-edge': { result: 'SUCCESS', durationMs: 1980000, level: 1 },
        elohim: { result: 'SUCCESS', durationMs: 1140000, level: 1 },
        'elohim-genesis': {
          result: 'SUCCESS',
          durationMs: 600000,
          level: 'genesis-trailer',
          stageAnnotations: {
            conductorReadiness: { partial: false, allReady: true, allUnready: false, ready: [], unready: [] },
            seedResultsConductorIdentities: {
              counts: { created: 3, exists: 0, failed: 0, succeeded: 3, total: 3 },
              partial: false,
              allSucceeded: true,
              allFailed: false,
              results: [],
            },
          },
        },
      },
    });

    const r = reconcile({ predicted, actual });

    assert.ok(
      !r.investigationPointers.some(p => p.includes('conductor-readiness') || p.includes('seed-conductor-identities')),
      `expected no stageAnnotations pointers on clean genesis; got ${JSON.stringify(r.investigationPointers)}`,
    );
  });

  it('handles missing stageAnnotations gracefully (older builds)', () => {
    const predicted = predictedFixture();
    const actual = actualFixture(); // genesis has no stageAnnotations field

    // Should not throw; reconciler returns valid output.
    const r = reconcile({ predicted, actual });
    assert.ok(Array.isArray(r.investigationPointers));
  });
});
