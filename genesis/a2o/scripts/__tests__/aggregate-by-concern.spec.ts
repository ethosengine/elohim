import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { aggregate } from '../lib/aggregate.js';

import type { ScenarioResult } from '../lib/load-cucumber.js';

const TAG_DATAPLANE = '@dataplane';

function makeByConcernScenarios(): ScenarioResult[] {
  return [
    {
      name: 'Content sync delivers to peer',
      feature: 'features/dataplane/content-sync.feature',
      status: 'passed',
      tags: ['@e2e', TAG_DATAPLANE, '@concern:content-sync'],
    },
    {
      name: 'Content sync fails under partition',
      feature: 'features/dataplane/content-sync.feature',
      status: 'failed',
      failureMessage: 'AssertionError: expected sync within 5s',
      tags: ['@e2e', TAG_DATAPLANE, '@concern:content-sync'],
    },
    {
      name: 'Peer mesh discovers new node',
      feature: 'features/dataplane/peer-mesh.feature',
      status: 'passed',
      tags: ['@e2e', TAG_DATAPLANE, '@concern:peer-mesh'],
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
    });

    assert.ok(r.summary.byConcern, 'byConcern must be present in summary');

    const contentSync = r.summary.byConcern['content-sync'];
    assert.ok(contentSync, 'content-sync concern must be present');
    assert.equal(contentSync.passed, 1);
    assert.equal(contentSync.failed, 1);
    assert.equal(contentSync.pending, 0);

    const peerMesh = r.summary.byConcern['peer-mesh'];
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

    const contentSync = r.summary.byConcern['content-sync'];
    assert.equal(contentSync.scenarios.length, 2);

    const peerMesh = r.summary.byConcern['peer-mesh'];
    assert.equal(peerMesh.scenarios.length, 1);
    assert.equal(peerMesh.scenarios[0].name, 'Peer mesh discovers new node');
    assert.equal(peerMesh.scenarios[0].status, 'passed');
    assert.equal(peerMesh.scenarios[0].surface, 'features/dataplane/peer-mesh.feature');
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
          feature: 'features/dataplane/blob-custody.feature',
          status: 'pending',
          tags: ['@e2e', TAG_DATAPLANE, '@concern:blob-custody'],
        },
        {
          name: 'Undefined step',
          feature: 'features/dataplane/blob-custody.feature',
          status: 'undefined',
          tags: ['@e2e', TAG_DATAPLANE, '@concern:blob-custody'],
        },
      ],
      consoleArtifacts: [],
      gaps: [],
      runId: 'r-test',
      profile: 'alpha',
    });

    const blobCustody = r.summary.byConcern['blob-custody'];
    assert.ok(blobCustody, 'blob-custody concern must be present');
    assert.equal(blobCustody.passed, 0);
    assert.equal(blobCustody.failed, 0);
    assert.equal(blobCustody.pending, 2);
  });
});
