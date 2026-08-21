import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  fixtureDoorwayUrl,
  mergeHouseholdMeshEnvironment,
  resolveDoorwayUrl,
  normalizeHouseholdFootprint,
  requireFixtureDoorwayLogPath,
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
const ALPHA_URL = 'https://alpha.example';
const LITERAL_URL = 'https://literal.example';

const manifest: HouseholdMeshFixture = {
  commonsEprId: 'bafy-manifest',
  connectedPeersFloor: 2,
  doorways: {
    alpha: {
      url: ALPHA_URL,
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

  void it('resolves "beta" from the E2E_DOORWAY_B CI alias when BETA is unset', () => {
    const fixture = mergeHouseholdMeshEnvironment(manifest, {
      E2E_DOORWAY_B: 'https://ci-b.example/',
    });

    assert.equal(requireFixtureDoorwayUrl(fixture, 'beta'), 'https://ci-b.example');
  });

  void it('prefers E2E_DOORWAY_BETA over E2E_DOORWAY_B when both are set', () => {
    const fixture = mergeHouseholdMeshEnvironment(manifest, {
      E2E_DOORWAY_BETA: 'https://runtime-beta.example/',
      E2E_DOORWAY_B: 'https://ci-b.example/',
    });

    assert.equal(requireFixtureDoorwayUrl(fixture, 'beta'), 'https://runtime-beta.example');
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
    assert.throws(
      () => requireFixturePeerPid(fixture, 'jessica'),
      /no safe PID for it.*local process on this host/s
    );
  });

  void it('names the doorways it did resolve when one is missing', () => {
    const fixture = mergeHouseholdMeshEnvironment(manifest, {});

    assert.throws(
      () => requireFixtureDoorwayUrl(fixture, 'gamma'),
      /doorways this run resolved: alpha, beta/
    );
  });

  void it('reports a declared-absent doorway as a topology fact, not a variable name', () => {
    const fixture = mergeHouseholdMeshEnvironment(
      {
        doorways: {
          alpha: { url: ALPHA_URL },
          gamma: { absentReason: 'this fleet deploys two doorways; there is no third' },
        },
      },
      {}
    );

    for (const resolve of [
      () => fixtureDoorwayUrl(fixture, 'gamma'),
      () => requireFixtureDoorwayUrl(fixture, 'gamma'),
    ]) {
      assert.throws(resolve, (error: Error) => {
        assert.match(error.message, /declares doorway "gamma" absent/);
        assert.match(error.message, /this fleet deploys two doorways; there is no third/);
        assert.doesNotMatch(error.message, /E2E_DOORWAY_GAMMA/);
        return true;
      });
    }
  });

  void it("lets the caller's own URL win before the fixture is even loaded", () => {
    // Regression: a declared-absent doorway used to be resolved EAGERLY, ahead
    // of the caller's own candidates, so `doorway "gamma" at "https://…"` threw
    // the absence instead of using the literal it was handed. The fixture
    // loader here throws on sight — reaching it at all is the failure.
    const explode = (): HouseholdMeshFixture => {
      throw new Error('fixture must not be consulted when the caller already has a URL');
    };

    assert.equal(resolveDoorwayUrl('gamma', [undefined, LITERAL_URL], explode), LITERAL_URL);
    assert.equal(
      resolveDoorwayUrl('gamma', ['https://env.example', LITERAL_URL], explode),
      'https://env.example'
    );
    assert.equal(resolveDoorwayUrl('gamma', ['', undefined, LITERAL_URL], explode), LITERAL_URL);
  });

  void it('still explains a declared absence when the caller has nothing', () => {
    const fixture = mergeHouseholdMeshEnvironment(
      {
        doorways: {
          alpha: { url: ALPHA_URL },
          gamma: { absentReason: 'this fleet deploys two doorways; there is no third' },
        },
      },
      {}
    );

    assert.throws(
      () => resolveDoorwayUrl('gamma', [undefined, undefined], () => fixture),
      /declares doorway "gamma" absent/
    );
    // A doorway the fixture says nothing about stays a plain "unknown", so the
    // caller can report its own missing-variable message.
    assert.equal(
      resolveDoorwayUrl('delta', [undefined], () => fixture),
      undefined
    );
  });

  void it('lets a supplied URL overrule an absent declaration', () => {
    const fixture = mergeHouseholdMeshEnvironment(
      { doorways: { gamma: { absentReason: 'no third doorway on this fleet' } } },
      { E2E_DOORWAY_GAMMA: 'https://gamma.example/' }
    );

    assert.equal(requireFixtureDoorwayUrl(fixture, 'gamma'), 'https://gamma.example');
  });

  void it('blames the substrate, not the variable, when the harness has no process control', () => {
    const fixture = mergeHouseholdMeshEnvironment(
      {
        processControl: false,
        processControlReason: 'the alpha fleet runs every peer and doorway as a Kubernetes pod',
        doorways: { alpha: { url: ALPHA_URL } },
        storagePeers: { jessica: { url: JESSICA_URL } },
      },
      {}
    );

    for (const resolve of [
      () => requireFixturePeerPid(fixture, 'jessica'),
      () => requireFixtureDoorwayLogPath(fixture, 'alpha'),
    ]) {
      assert.throws(resolve, (error: Error) => {
        assert.match(
          error.message,
          /the alpha fleet runs every peer and doorway as a Kubernetes pod/
        );
        assert.match(error.message, /local-stack-only and belongs behind @local/);
        return true;
      });
    }
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
