import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { aggregate } from '../lib/aggregate.js';

import type { DeclaredConcernSet } from '../lib/declared-concerns.js';
import type { ScenarioResult } from '../lib/load-cucumber.js';
import type { RunEnv } from '../lib/run-env.js';

const TAG_DATAPLANE = '@dataplane';

/**
 * The denominator this lane is DECLARED to measure — read from habits.yaml by
 * the caller, never computed by the runner (Law I). Held small and explicit
 * here so the notMeasured arithmetic is readable.
 */
const BLOB_CUSTODY = 'blob-custody';
const CONTENT_SYNC = 'content-sync';
const NOTARY_AUTHORITY = 'notary-authority';
const PEER_MESH = 'peer-mesh';
const PEER_MESH_SCENARIO = 'Peer mesh discovers new node';

const DECLARED: DeclaredConcernSet = {
  source: 'habits.yaml:checks + @concern registry',
  concerns: [BLOB_CUSTODY, CONTENT_SYNC, NOTARY_AUTHORITY, PEER_MESH],
};

/** The two literals every fixture below derives, so no string is duplicated. */
function surfaceFor(concern: string): string {
  return `features/dataplane/${concern}.feature`;
}
function tagFor(concern: string): string {
  return `@concern:${concern}`;
}

const HOUSEHOLD_ENV: RunEnv = {
  lane: 'household',
  peers: 3,
  processControl: true,
  networkStage: 'bootstrap',
  sut: 'sha256:0123456789abcdef',
  sutParts: { storage: 'tree:aaaa' },
  unknown: [],
};

function makeByConcernScenarios(): ScenarioResult[] {
  return [
    {
      name: 'Content sync delivers to peer',
      feature: surfaceFor(CONTENT_SYNC),
      status: 'passed',
      tags: ['@e2e', TAG_DATAPLANE, tagFor(CONTENT_SYNC)],
    },
    {
      name: 'Content sync fails under partition',
      feature: surfaceFor(CONTENT_SYNC),
      status: 'failed',
      failureMessage: 'AssertionError: expected sync within 5s',
      tags: ['@e2e', TAG_DATAPLANE, tagFor(CONTENT_SYNC)],
    },
    {
      name: PEER_MESH_SCENARIO,
      feature: surfaceFor(PEER_MESH),
      status: 'passed',
      tags: ['@e2e', TAG_DATAPLANE, tagFor(PEER_MESH)],
    },
  ];
}

void describe('aggregate byConcern', () => {
  void it('buckets scenarios into byConcern keyed by @concern: tag value', () => {
    const r = aggregate({
      scenarios: makeByConcernScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
      env: HOUSEHOLD_ENV,
      declaredConcerns: DECLARED,
    });

    assert.ok(r.summary.byConcern, 'byConcern must be present in summary');

    const contentSync = r.summary.byConcern[CONTENT_SYNC];
    assert.ok(contentSync, 'content-sync concern must be present');
    assert.equal(contentSync.passed, 1);
    assert.equal(contentSync.failed, 1);
    assert.equal(contentSync.pending, 0);

    const peerMesh = r.summary.byConcern[PEER_MESH];
    assert.ok(peerMesh, 'peer-mesh concern must be present');
    assert.equal(peerMesh.passed, 1);
    assert.equal(peerMesh.failed, 0);
    assert.equal(peerMesh.pending, 0);
  });

  void it('scenarios array in each concern has name, status, surface', () => {
    const r = aggregate({
      scenarios: makeByConcernScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
    });

    const contentSync = r.summary.byConcern[CONTENT_SYNC];
    assert.equal(contentSync.scenarios.length, 2);

    const peerMesh = r.summary.byConcern[PEER_MESH];
    assert.equal(peerMesh.scenarios.length, 1);
    assert.equal(peerMesh.scenarios[0].name, PEER_MESH_SCENARIO);
    assert.equal(peerMesh.scenarios[0].status, 'passed');
    assert.equal(peerMesh.scenarios[0].surface, surfaceFor(PEER_MESH));
  });

  void it('scenarios without @concern: tag are excluded from byConcern', () => {
    const scenarios: ScenarioResult[] = [
      ...makeByConcernScenarios(),
      {
        name: 'Untagged scenario',
        feature: 'features/auth/fixture-humans.feature',
        status: 'passed',
        tags: ['@e2e'],
      },
    ];
    const r = aggregate({
      scenarios,
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
    });

    assert.deepEqual(
      Object.keys(r.summary.byConcern).sort((a, b) => a.localeCompare(b)),
      ['content-sync', 'peer-mesh']
    );
  });

  void it('byConcern is an empty object when no scenarios carry a @concern: tag', () => {
    const r = aggregate({
      scenarios: [
        {
          name: 'Some scenario',
          feature: 'features/auth/fixture-humans.feature',
          status: 'passed',
          tags: ['@e2e'],
        },
      ],
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
    });

    assert.deepEqual(r.summary.byConcern, {});
  });

  void it('pending/undefined scenarios count toward pending in their concern bucket', () => {
    const r = aggregate({
      scenarios: [
        {
          name: 'Step not yet implemented',
          feature: surfaceFor(BLOB_CUSTODY),
          status: 'pending',
          tags: ['@e2e', TAG_DATAPLANE, tagFor(BLOB_CUSTODY)],
        },
        {
          name: 'Undefined step',
          feature: surfaceFor(BLOB_CUSTODY),
          status: 'undefined',
          tags: ['@e2e', TAG_DATAPLANE, tagFor(BLOB_CUSTODY)],
        },
      ],
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
    });

    const blobCustody = r.summary.byConcern[BLOB_CUSTODY];
    assert.ok(blobCustody, 'blob-custody concern must be present');
    assert.equal(blobCustody.passed, 0);
    assert.equal(blobCustody.failed, 0);
    assert.equal(blobCustody.pending, 2);
  });
});

void describe('aggregate declared (Law I — the denominator is anchored outside the run)', () => {
  void it('derives notMeasured as declared minus exercised', () => {
    const r = aggregate({
      scenarios: makeByConcernScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
      declaredConcerns: DECLARED,
    });

    assert.ok(r.declared, 'declared must be present when a denominator was supplied');
    assert.equal(r.declared.source, DECLARED.source);
    assert.deepEqual(r.declared.concerns, DECLARED.concerns);
    assert.deepEqual(r.declared.exercised, [CONTENT_SYNC, PEER_MESH]);
    assert.deepEqual(r.declared.notMeasured, [BLOB_CUSTODY, NOTARY_AUTHORITY]);
  });

  void it('a TAG-SCOPED run still reports every unexercised declared concern', () => {
    // The 76-of-80 case. A run scoped to one tag exercises one concern; if the
    // runner were allowed to declare its own denominator it would report 100%
    // permitted and nothing missing. The declared set does not move, so
    // narrowing the scope makes notMeasured GROW.
    const fullRun = aggregate({
      scenarios: makeByConcernScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'full',
      profile: 'mesh',
      declaredConcerns: DECLARED,
    });
    const scopedRun = aggregate({
      scenarios: makeByConcernScenarios().filter(s => s.tags.includes(tagFor(PEER_MESH))),
      consoleArtifacts: [],
      gaps: [],
      runId: 'scoped',
      profile: 'mesh',
      declaredConcerns: DECLARED,
    });

    assert.deepEqual(Object.keys(scopedRun.summary.byConcern), [PEER_MESH]);
    assert.deepEqual(scopedRun.declared?.exercised, [PEER_MESH]);
    assert.deepEqual(scopedRun.declared?.notMeasured, [
      BLOB_CUSTODY,
      CONTENT_SYNC,
      NOTARY_AUTHORITY,
    ]);
    assert.ok(
      (scopedRun.declared?.notMeasured.length ?? 0) > (fullRun.declared?.notMeasured.length ?? 0),
      'a re-scope must not be able to make missing coverage vanish'
    );
  });

  void it('counts a concern with NO verdict as NOT MEASURED, however many scenarios it ran', () => {
    // guidestar §S3's status table: `Skipped` → NOT MEASURED, restated in §6.
    // Presence in byConcern is not measurement. Ran-and-skipped and never-ran
    // are both absences — the report tells them apart with
    // notMeasuredRanAndSkipped, not by promoting one of them to exercised.
    const r = aggregate({
      scenarios: [
        {
          name: 'Notary authority pending',
          feature: surfaceFor(NOTARY_AUTHORITY),
          status: 'pending',
          tags: ['@e2e', TAG_DATAPLANE, tagFor(NOTARY_AUTHORITY)],
        },
      ],
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
      declaredConcerns: DECLARED,
    });

    assert.deepEqual(r.declared?.exercised, []);
    assert.equal(r.summary.byConcern[NOTARY_AUTHORITY].passed, 0);
    assert.equal(r.summary.byConcern[NOTARY_AUTHORITY].pending, 1);
    assert.ok(r.declared?.notMeasured.includes(NOTARY_AUTHORITY));
    // It RAN — that is a fixture/apparatus first move, not a scope one.
    assert.deepEqual(r.declared?.notMeasuredRanAndSkipped, [NOTARY_AUTHORITY]);
  });

  void it('an ALL-SKIPPED concern is NOT MEASURED — the edge #1376 shape', () => {
    // The motivating build: 80 scenarios, 3 passed, 1 failed, 76 skipped, and
    // three habits reading green off it. A concern whose every scenario
    // skipped must never read as exercised.
    const skipped = (name: string, concern: string): ScenarioResult => ({
      name,
      feature: surfaceFor(concern),
      status: 'skipped',
      tags: ['@e2e', TAG_DATAPLANE, tagFor(concern)],
    });
    const r = aggregate({
      scenarios: [
        skipped('Notary re-certifies reach', NOTARY_AUTHORITY),
        skipped('Notary declares a head', NOTARY_AUTHORITY),
        skipped('Blob lands on a second household', BLOB_CUSTODY),
        {
          name: PEER_MESH_SCENARIO,
          feature: surfaceFor(PEER_MESH),
          status: 'passed',
          tags: ['@e2e', TAG_DATAPLANE, tagFor(PEER_MESH)],
        },
      ],
      consoleArtifacts: [],
      gaps: [],
      runId: 'edge-1376-shape',
      profile: 'dataplane',
      declaredConcerns: DECLARED,
    });

    // Only the concern that produced a real verdict is exercised.
    assert.deepEqual(r.declared?.exercised, [PEER_MESH]);
    assert.deepEqual(r.declared?.notMeasured, [BLOB_CUSTODY, CONTENT_SYNC, NOTARY_AUTHORITY]);
    // …and the report says WHICH of those ran and skipped.
    assert.deepEqual(r.declared?.notMeasuredRanAndSkipped, [BLOB_CUSTODY, NOTARY_AUTHORITY]);

    // skipped is carried as itself, and `pending` stays the no-verdict union
    // so fulfill.rs's `pending == 0` gate and saga-status.py's `pending > 0`
    // keep their exact meaning.
    const notary = r.summary.byConcern[NOTARY_AUTHORITY];
    assert.equal(notary.skipped, 2);
    assert.equal(notary.pending, 2);
    assert.equal(notary.passed, 0);
    assert.deepEqual(
      notary.scenarios.map(s => s.status),
      ['skipped', 'skipped']
    );
  });

  void it('a concern with passes AND skips is exercised, but never reads all-green', () => {
    // The narrowing trap: if `pending` stopped counting skips, 3-passed-
    // 1-skipped would satisfy fulfill.rs's `failed==0 && pending==0 &&
    // passed>0` and discharge a commitment the run did not keep.
    const r = aggregate({
      scenarios: [
        {
          name: 'Sync delivers',
          feature: surfaceFor(CONTENT_SYNC),
          status: 'passed',
          tags: [TAG_DATAPLANE, tagFor(CONTENT_SYNC)],
        },
        {
          name: 'Sync under partition',
          feature: surfaceFor(CONTENT_SYNC),
          status: 'skipped',
          tags: [TAG_DATAPLANE, tagFor(CONTENT_SYNC)],
        },
      ],
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
      declaredConcerns: DECLARED,
    });

    assert.deepEqual(r.declared?.exercised, [CONTENT_SYNC]);
    const cs = r.summary.byConcern[CONTENT_SYNC];
    assert.equal(cs.passed, 1);
    assert.equal(cs.skipped, 1);
    assert.equal(cs.pending, 1, 'pending must remain the no-verdict UNION, or the gate un-gates');
  });

  void it('an UNREADABLE denominator is loud in a field, not only in prose', () => {
    // concerns: [] renders `0 declared · 0 not measured` — byte-identical to
    // perfect coverage for every count-based reader.
    const r = aggregate({
      scenarios: makeByConcernScenarios(),
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
      declaredConcerns: {
        source: 'habits.yaml:checks + @concern registry (UNREADABLE /nope: ENOENT)',
        unreadable: 'UNREADABLE /nope: ENOENT',
        concerns: [],
      },
    });

    assert.equal(r.declared?.unreadable, 'UNREADABLE /nope: ENOENT');
    assert.deepEqual(r.declared?.concerns, []);
    assert.deepEqual(r.declared?.notMeasured, []);
  });

  void it('reports an undeclared concern in byConcern without inventing a denominator', () => {
    const r = aggregate({
      scenarios: [
        {
          name: 'Brand new concern',
          feature: surfaceFor('new'),
          status: 'passed',
          tags: ['@e2e', '@concern:not-in-habits'],
        },
      ],
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
      declaredConcerns: DECLARED,
    });

    assert.ok(r.summary.byConcern['not-in-habits']);
    assert.deepEqual(r.declared?.exercised, []);
    assert.deepEqual(r.declared?.notMeasured, DECLARED.concerns);
  });
});
