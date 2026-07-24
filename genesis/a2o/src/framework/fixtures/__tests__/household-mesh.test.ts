import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  mergeHouseholdMeshEnvironment,
  normalizeHouseholdFootprint,
  requireFixtureDoorwayUrl,
  requireFixturePeerPid,
  requireFixturePoolStorageUrls,
  requireFixtureStoragePeer,
  type HouseholdMeshFixture,
} from '../household-mesh.js';

const MATTHEW_URL = 'https://matthew.example';
const JESSICA_URL = 'https://jessica.example';
const JAMES_URL = 'https://james.example';
const COMMONS_EPR_ID = 'bafy-commons';
const HOUSEHOLD_KIND = 'household';
const DOWELL_HOLDER = 'dowell-home';
const ADAM_HOLDER = 'adam-home';

const manifest: HouseholdMeshFixture = {
  commonsEprId: 'bafy-manifest',
  connectedPeersFloor: 2,
  doorways: {
    alpha: {
      url: 'https://alpha.example',
      primaryStorageUrl: 'https://alpha-storage.example',
      poolStorageUrls: [MATTHEW_URL, JESSICA_URL],
      logPath: '/logs/doorway-alpha.log',
    },
    beta: { url: 'https://beta.example' },
  },
  storagePeers: {
    matthew: { url: MATTHEW_URL, pid: 41_001 },
    jessica: { url: JESSICA_URL, pid: 41_002 },
    james: { url: JAMES_URL, pid: 41_003 },
  },
};

void describe('household mesh fixture', () => {
  void it('keeps one manifest authority for doorways, pools, peers, and PIDs', () => {
    const fixture = mergeHouseholdMeshEnvironment(manifest, {});

    assert.equal(requireFixtureDoorwayUrl(fixture, 'beta'), 'https://beta.example');
    assert.deepEqual(requireFixturePoolStorageUrls(fixture, 'alpha'), [MATTHEW_URL, JESSICA_URL]);
    assert.equal(requireFixtureStoragePeer(fixture, 'james').url, JAMES_URL);
    assert.equal(requireFixturePeerPid(fixture, 'jessica'), 41_002);
  });

  void it('lets ephemeral CI environment values override checked-in fixture values', () => {
    const fixture = mergeHouseholdMeshEnvironment(manifest, {
      E2E_DOORWAY_BETA: 'https://runtime-beta.example/',
      E2E_STORAGE_JESSICA: 'https://runtime-jessica.example/',
      E2E_STORAGE_JESSICA_PID: '51002',
      E2E_DOORWAY_POOL_STORAGE_URLS:
        'https://runtime-matthew.example/, https://runtime-jessica.example/',
      E2E_COMMONS_EPR_ID: 'bafy-runtime',
    });

    assert.equal(requireFixtureDoorwayUrl(fixture, 'beta'), 'https://runtime-beta.example');
    assert.deepEqual(requireFixturePoolStorageUrls(fixture, 'alpha'), [
      'https://runtime-matthew.example',
      'https://runtime-jessica.example',
    ]);
    assert.equal(
      requireFixtureStoragePeer(fixture, 'jessica').url,
      'https://runtime-jessica.example'
    );
    assert.equal(requireFixturePeerPid(fixture, 'jessica'), 51_002);
    assert.equal(fixture.commonsEprId, 'bafy-runtime');
  });

  void it('preserves the conventional per-peer environment fallback for the storage pool', () => {
    const fixture = mergeHouseholdMeshEnvironment(
      {},
      {
        E2E_STORAGE_MATTHEW: MATTHEW_URL,
        E2E_STORAGE_JESSICA: JESSICA_URL,
        E2E_STORAGE_JAMES: JAMES_URL,
      }
    );

    assert.deepEqual(requireFixturePoolStorageUrls(fixture, 'alpha'), [
      MATTHEW_URL,
      JESSICA_URL,
      JAMES_URL,
    ]);
  });

  void it('fails loudly when a requested live fixture leg is absent', () => {
    const fixture = mergeHouseholdMeshEnvironment({}, {});

    assert.throws(() => requireFixtureDoorwayUrl(fixture, 'beta'), /missing doorway "beta"/);
    assert.throws(() => requireFixtureStoragePeer(fixture, 'jessica'), /storage peer "jessica"/);
    assert.throws(() => requireFixturePeerPid(fixture, 'jessica'), /safe PID for "jessica"/);
  });

  void it('normalizes holder ordering while keeping commitment testimony exact', () => {
    const first = normalizeHouseholdFootprint(
      {
        contentId: COMMONS_EPR_ID,
        commitmentBackedCollectives: 2,
        feltStatus: {
          heldBy: [
            { id: DOWELL_HOLDER, kind: HOUSEHOLD_KIND, label: 'the Dowells' },
            { id: ADAM_HOLDER, kind: HOUSEHOLD_KIND, label: 'the Adams' },
          ],
        },
      },
      COMMONS_EPR_ID
    );
    const second = normalizeHouseholdFootprint(
      {
        contentId: COMMONS_EPR_ID,
        commitmentBackedCollectives: 2,
        details: {
          stewardingCollectives: [
            { id: ADAM_HOLDER, kind: HOUSEHOLD_KIND },
            { id: DOWELL_HOLDER, kind: HOUSEHOLD_KIND },
          ],
        },
      },
      COMMONS_EPR_ID
    );

    assert.deepEqual(second, first);
  });

  void it('rejects an unbacked fixture instead of declaring vacuous convergence', () => {
    assert.throws(
      () =>
        normalizeHouseholdFootprint(
          {
            contentId: COMMONS_EPR_ID,
            commitmentBackedCollectives: 0,
            feltStatus: { heldBy: [{ id: DOWELL_HOLDER, kind: HOUSEHOLD_KIND }] },
          },
          COMMONS_EPR_ID
        ),
      /not commitment-backed/
    );
  });
});
