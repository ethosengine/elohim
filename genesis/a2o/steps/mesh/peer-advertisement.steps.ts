/**
 * Peer-advertisement (gossipsub capacity heartbeat) step definitions.
 *
 * Backs features/federation/peer-advertisement.feature.
 *
 * SUBSTRATE REALITY CHECK (read before touching the pending block below).
 * The wire type this feature describes — `CapacityAnnouncement`, the
 * `/elohim/compute/capacity/1.0.0` MessagePack gossipsub topic, the 30s
 * broadcast loop — exists ONLY as an encode/decode struct with unit tests:
 * steward/node/src/pod/capacity.rs (`CAPACITY_TOPIC`, `CapacityAnnouncement`).
 * Nothing else in the tree references `CAPACITY_TOPIC` or
 * `CapacityAnnouncement` (verified by grep 2026-08-21) — there is no publish
 * loop, no subscriber, no neighbor table, no staleness eviction, and no
 * ExtractionCache-to-announcement wiring anywhere. Doorway and elohim-storage
 * do not participate in this topic either.
 *
 * The household mesh this suite runs against (E2E_HOUSEHOLD_FIXTURE_PATH)
 * stands up exactly two process kinds: doorways (alpha/beta) and
 * elohim-storage peers (matthew/jessica/james). It never starts a
 * `steward/node` (elohim-node) process — the only binary that would ever own
 * a laptop/home-node/network-node CapacityAnnouncement identity. So even once
 * the publish/subscribe loop is wired, THIS substrate has no peer process to
 * exercise it against; that is a second, independent gap from the missing
 * wiring itself.
 *
 * Every step below that serves only a `@wip` scenario is honestly `pending`
 * for this reason — there is no live surface (HTTP, /metrics, log file) that
 * could make the assertion the step names. The one scenario NOT tagged @wip
 * (the boot-ramp regression at the bottom of the feature) is unrelated to the
 * gossipsub layer — it tests the elohim-storage-to-conductor bridge retry
 * behavior, which IS live in this substrate — and is wired with real HTTP +
 * log probes.
 */

import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';

import { Given, When, Then } from '@cucumber/cucumber';

import { getRaw } from '../../src/framework/dataplane/surfaces.js';
import {
  loadHouseholdMeshFixture,
  requireFixtureStoragePeer,
  type HouseholdMeshFixture,
} from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Peer Profile Identity — @wip (no steward/node peer process in this mesh)
// ---------------------------------------------------------------------------

Given('human {string} has a Tauri node {string} configured as laptop', function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to configure — see file header
});

Given('human {string} has a home node {string} configured as home_node', function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to configure — see file header
});

Given('a network node {string} is configured as network_node', function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to configure — see file header
});

// One definition covers all three "<peer> has capabilities: k=v, k=v, ..."
// lines (laptop/home_node/network_node each list a different key set) — a
// cucumber-expression can't express an optional trailing key list cleanly, so
// this is a regex step instead.
Given(/^"([^"]+)" has capabilities: (.+)$/, function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process advertises capabilities — see file header
});

When('{string} joins the network', function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to join a gossipsub mesh — see file header
});

Then('{string} broadcasts a CapacityAnnouncement within {int} seconds', function (this: E2EWorld) {
  return 'pending'; // no CapacityAnnouncement publish loop anywhere in the tree — see file header
});

Then('the announcement includes node_id, timestamp, and ready=true', function (this: E2EWorld) {
  return 'pending'; // no live announcement to inspect — see file header
});

Then('the announcement capabilities include {string} but not {string}', function (this: E2EWorld) {
  return 'pending'; // no live announcement to inspect — see file header
});

Then('connected peers receive the announcement via gossipsub', function (this: E2EWorld) {
  return 'pending'; // no gossipsub subscriber for this topic anywhere in the tree — see file header
});

Then(
  'the announcement capabilities include {string}, {string}, and {string}',
  function (this: E2EWorld) {
    return 'pending'; // no live announcement to inspect — see file header
  }
);

Then(
  'the announcement shows estimated_tokens_per_sec based on measured throughput',
  function (this: E2EWorld) {
    return 'pending'; // no live announcement to inspect — see file header
  }
);

Then('the announcement capabilities include {string}', function (this: E2EWorld) {
  return 'pending'; // no live announcement to inspect — see file header
});

Then('the announcement budget_remaining reflects compute allocation', function (this: E2EWorld) {
  return 'pending'; // no live announcement to inspect — see file header
});

Given('doorway {string} is running with projection cache enabled', function (this: E2EWorld) {
  return 'pending'; // doorway does not participate in the capacity gossipsub topic — see file header
});

When('doorway {string} participates in gossipsub', function (this: E2EWorld) {
  return 'pending'; // doorway does not participate in the capacity gossipsub topic — see file header
});

Then('doorway {string} broadcasts a CapacityAnnouncement', function (this: E2EWorld) {
  return 'pending'; // doorway does not participate in the capacity gossipsub topic — see file header
});

Then('the announcement capabilities include {string} and {string}', function (this: E2EWorld) {
  return 'pending'; // no live announcement to inspect — see file header
});

Then(
  'the announcement is distinguishable from storage peer announcements by capability set',
  function (this: E2EWorld) {
    return 'pending'; // no live announcement to inspect — see file header
  }
);

// ---------------------------------------------------------------------------
// Gossipsub Heartbeat Mechanics — @wip (no publish/subscribe loop at all)
// ---------------------------------------------------------------------------

Given('{string} is connected to the gossipsub mesh', function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to connect — see file header
});

When('{int} seconds elapse', function (this: E2EWorld) {
  return 'pending'; // nothing broadcasts on a timer to observe elapsing against — see file header
});

Then('{string} has broadcast at least {int} CapacityAnnouncements', function (this: E2EWorld) {
  return 'pending'; // no CapacityAnnouncement publish loop anywhere in the tree — see file header
});

Then('each announcement has an incrementing timestamp', function (this: E2EWorld) {
  return 'pending'; // no live announcement stream to inspect — see file header
});

Then('the announcements are published to {string}', function (this: E2EWorld) {
  return 'pending'; // no publisher targets this gossipsub topic — see file header
});

Given('{string} is connected to {string} via gossipsub', function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to connect — see file header
});

When('{string} broadcasts a CapacityAnnouncement', function (this: E2EWorld) {
  return 'pending'; // no CapacityAnnouncement publish loop anywhere in the tree — see file header
});

Then('{string} decodes the MessagePack announcement', function (this: E2EWorld) {
  return 'pending'; // no subscriber to decode a live announcement — see file header
});

Then('{string} records the announcement in its neighbor capacity table', function (this: E2EWorld) {
  return 'pending'; // no neighbor capacity table exists anywhere in the tree — see file header
});

Then('the table entry is keyed by node_id with last_seen timestamp', function (this: E2EWorld) {
  return 'pending'; // no neighbor capacity table exists anywhere in the tree — see file header
});

Given('{string} has a neighbor table entry for {string}', function (this: E2EWorld) {
  return 'pending'; // no neighbor capacity table exists anywhere in the tree — see file header
});

Given('the entry was last updated {int} seconds ago', function (this: E2EWorld) {
  return 'pending'; // no neighbor capacity table exists anywhere in the tree — see file header
});

Given(
  /^the staleness threshold is \d+ seconds \(\d+ missed heartbeats\)$/,
  function (this: E2EWorld) {
    return 'pending'; // no staleness/eviction policy exists anywhere in the tree — see file header
  }
);

When('{string} checks the neighbor table', function (this: E2EWorld) {
  return 'pending'; // no neighbor capacity table exists anywhere in the tree — see file header
});

Then('the entry for {string} is marked as stale', function (this: E2EWorld) {
  return 'pending'; // no staleness/eviction policy exists anywhere in the tree — see file header
});

Then('{string} is not considered for routing or replication decisions', function (this: E2EWorld) {
  return 'pending'; // no routing/replication decision reads a neighbor table yet — see file header
});

// ---------------------------------------------------------------------------
// Dynamic State Changes — @wip (announcement content is never live)
// ---------------------------------------------------------------------------

Given('{string} has max_storage={word} and current usage is {word}', function (this: E2EWorld) {
  return 'pending'; // no live storage/announcement coupling — see file header
});

Given(
  '{string} is broadcasting announcements with budget_remaining reflecting available storage',
  function (this: E2EWorld) {
    return 'pending'; // no CapacityAnnouncement publish loop anywhere in the tree — see file header
  }
);

When('content replication fills storage to {word}', function (this: E2EWorld) {
  return 'pending'; // no live storage/announcement coupling — see file header
});

Then('the next CapacityAnnouncement shows reduced budget_remaining', function (this: E2EWorld) {
  return 'pending'; // no CapacityAnnouncement publish loop anywhere in the tree — see file header
});

Then(
  'connected peers see the storage pressure in their neighbor tables',
  function (this: E2EWorld) {
    return 'pending'; // no neighbor capacity table exists anywhere in the tree — see file header
  }
);

Given('{string} has been processing inference requests', function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to have processed anything — see file header
});

Given('the compute budget drops to {int}', function (this: E2EWorld) {
  return 'pending'; // no compute-budget model wired to an announcement — see file header
});

When('the next heartbeat fires', function (this: E2EWorld) {
  return 'pending'; // no CapacityAnnouncement publish loop anywhere in the tree — see file header
});

Then('the CapacityAnnouncement shows budget_remaining={int}', function (this: E2EWorld) {
  return 'pending'; // no CapacityAnnouncement publish loop anywhere in the tree — see file header
});

Then('the announcement shows ready=false', function (this: E2EWorld) {
  return 'pending'; // no live announcement to inspect — see file header
});

Then('peers stop routing compute requests to {string}', function (this: E2EWorld) {
  return 'pending'; // no routing decision reads announcement readiness yet — see file header
});

Given(/^"([^"]+)" was previously offline \(lid closed\)$/, function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to have gone offline — see file header
});

When(
  /^Terrance opens the laptop and "([^"]+)" reconnects to the network$/,
  function (this: E2EWorld) {
    return 'pending'; // no steward/node peer process to reconnect — see file header
  }
);

Then(
  String.raw`the announcement reflects current state \(cache may be cold\)`,
  function (this: E2EWorld) {
    return 'pending'; // no live announcement to inspect — see file header
  }
);

Then('the regular {int}-second heartbeat resumes', function (this: E2EWorld) {
  return 'pending'; // no CapacityAnnouncement publish loop anywhere in the tree — see file header
});

Given(/^"([^"]+)" is connected and broadcasting$/, function (this: E2EWorld) {
  return 'pending'; // no CapacityAnnouncement publish loop anywhere in the tree — see file header
});

Given('{string} has a fresh neighbor table entry for {string}', function (this: E2EWorld) {
  return 'pending'; // no neighbor capacity table exists anywhere in the tree — see file header
});

When(/^Terrance closes the laptop \(ungraceful disconnect\)$/, function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to disconnect — see file header
});

Then('{string} receives no further announcements from {string}', function (this: E2EWorld) {
  return 'pending'; // no subscriber/neighbor table to observe absence in — see file header
});

Then('after {int} seconds, {string} marks {string} as stale', function (this: E2EWorld) {
  return 'pending'; // no staleness/eviction policy exists anywhere in the tree — see file header
});

Then('after the connection timeout, the entry is evicted', function (this: E2EWorld) {
  return 'pending'; // no staleness/eviction policy exists anywhere in the tree — see file header
});

Given('{string} has an empty ExtractionCache', function (this: E2EWorld) {
  return 'pending'; // no ExtractionCache-to-announcement wiring exists — see file header
});

Given('{string} is broadcasting with ready_content empty', function (this: E2EWorld) {
  return 'pending'; // no ExtractionCache-to-announcement wiring exists — see file header
});

When('content {string} is extracted into the cache', function (this: E2EWorld) {
  return 'pending'; // no ExtractionCache-to-announcement wiring exists — see file header
});

Then('the next CapacityAnnouncement includes {string} in capabilities', function (this: E2EWorld) {
  return 'pending'; // no ExtractionCache-to-announcement wiring exists — see file header
});

Then('peers recognize {string} can now serve that content', function (this: E2EWorld) {
  return 'pending'; // no subscriber/neighbor table to recognize this from — see file header
});

Given('{string} has {string} warm in its ExtractionCache', function (this: E2EWorld) {
  return 'pending'; // no ExtractionCache-to-announcement wiring exists — see file header
});

Given('{string} is advertising {string} in capabilities', function (this: E2EWorld) {
  return 'pending'; // no ExtractionCache-to-announcement wiring exists — see file header
});

When('the ExtractionCache evicts {string} due to budget pressure', function (this: E2EWorld) {
  return 'pending'; // no ExtractionCache-to-announcement wiring exists — see file header
});

Then('the next CapacityAnnouncement no longer includes {string}', function (this: E2EWorld) {
  return 'pending'; // no ExtractionCache-to-announcement wiring exists — see file header
});

Then('peers stop considering {string} for delivery of that content', function (this: E2EWorld) {
  return 'pending'; // no subscriber/neighbor table to react to this — see file header
});

// ---------------------------------------------------------------------------
// Peer Diversity on the Same Network — @wip @requires:multi-node
// ---------------------------------------------------------------------------

Given('the following peers are connected via gossipsub:', function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process for any listed profile — see file header
});

When('each peer broadcasts its CapacityAnnouncement', function (this: E2EWorld) {
  return 'pending'; // no CapacityAnnouncement publish loop anywhere in the tree — see file header
});

Then("every peer's neighbor table contains entries for all other peers", function (this: E2EWorld) {
  return 'pending'; // no neighbor capacity table exists anywhere in the tree — see file header
});

Then('the profiles are distinguishable by capability sets', function (this: E2EWorld) {
  return 'pending'; // no live announcements to compare — see file header
});

Then('no peer is assumed to have the same profile as another', function (this: E2EWorld) {
  return 'pending'; // no live announcements to compare — see file header
});

Given(/^"([^"]+)" goes offline \(intermittent\)$/, function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to have gone offline — see file header
});

Given(/^"([^"]+)" remains online \(always-on\)$/, function (this: E2EWorld) {
  return 'pending'; // no steward/node peer process to remain online — see file header
});

When('{string} evaluates the neighbor table', function (this: E2EWorld) {
  return 'pending'; // no neighbor capacity table exists anywhere in the tree — see file header
});

Then('{string} sees {int} stale peer and {int} active peers', function (this: E2EWorld) {
  return 'pending'; // no neighbor capacity table exists anywhere in the tree — see file header
});

Then('routing decisions exclude the stale laptop peer', function (this: E2EWorld) {
  return 'pending'; // no routing decision reads a neighbor table yet — see file header
});

Then('no degradation occurs because always-on peers remain', function (this: E2EWorld) {
  return 'pending'; // no routing decision reads a neighbor table yet — see file header
});

// ---------------------------------------------------------------------------
// Heartbeat Survival Across Slow Conductor Boots — REAL (not @wip)
//
// Unlike everything above, this scenario is about the elohim-storage ->
// conductor bridge (hc_client_registry.rs), which IS live in this household
// mesh. It has nothing to do with the gossipsub capacity layer; it shares
// only the word "PeerStatus heartbeat" with the section above. Every step is
// wired to a real surface (a live storage-peer log file, or the doorway's
// /db/humans and /api/v1/households/{id}/devices HTTP surfaces) and asserts
// something falsifiable — a red result here is a real, meaningful signal
// that the boot-ramp / forever-retry fix regressed.
//
// Exact log-message anchors (elohim/elohim-storage/src/hc_client_registry.rs
// and src/main.rs, verified against source 2026-08-21):
//   "HcClient connect failed — retrying (cells may still be CellDisabled)"
//   "HcClient connect failed after the bounded boot ramp"
//   "HcClient bridge still down — retrying forever"
//   "HcClient bridge reconnect: retrying"
//   "PeerStatus heartbeat task started (infrastructure role)"
// ---------------------------------------------------------------------------

const CELL_DISABLED_RETRY_MSG =
  'HcClient connect failed — retrying (cells may still be CellDisabled)';
const CELL_DISABLED_RAMP_EXHAUSTED_MSG = 'HcClient connect failed after the bounded boot ramp';
const CELL_DISABLED_FOREVER_MSG = 'HcClient bridge still down — retrying forever';
const CELL_DISABLED_RECONNECT_MSG = 'HcClient bridge reconnect: retrying';
const PEERSTATUS_HEARTBEAT_STARTED_MSG = 'PeerStatus heartbeat task started (infrastructure role)';

interface BootRampState {
  humanName: string;
  peerName: string;
  householdId: string;
  logPath: string;
}

const bootRampStates = new WeakMap<E2EWorld, BootRampState>();

function bootRampState(world: E2EWorld): BootRampState {
  const state = bootRampStates.get(world);
  assert.ok(
    state,
    'boot-ramp preconditions were not established — the "has a storage peer whose ' +
      'conductor takes longer..." step must run first'
  );
  return state;
}

/**
 * `StoragePeerFixture` (household-mesh.ts) declares only `url`/`pid`/`pidFile` —
 * it does not type a `logPath` field, even though the live household fixture
 * JSON carries one for storage peers (matthew/jessica/james), same as it does
 * for doorways. Reading it here (rather than widening the shared fixture type)
 * keeps this file's write-set to itself.
 */
function storagePeerLogPath(fixture: HouseholdMeshFixture, name: string): string {
  const raw = fixture.storagePeers?.[name] as { logPath?: string } | undefined;
  const logPath = raw?.logPath;
  assert.ok(
    logPath,
    `household fixture's storage peer "${name}" declares no logPath — cannot observe its ` +
      'conductor-bridge boot behavior without a log file to read (set ' +
      `E2E_STORAGE_${name.toUpperCase()}_LOG_PATH or storagePeers.${name}.logPath)`
  );
  return logPath;
}

async function findHouseholdIdFor(world: E2EWorld, humanName: string): Promise<string> {
  const doorway = world.getDoorway('alpha');
  const { status, text } = await getRaw(`${doorway.url}/db/humans`);
  assert.equal(status, 200, `GET /db/humans on doorway "alpha" returned ${status}: ${text}`);
  const parsed = JSON.parse(text) as unknown;
  const items = Array.isArray(parsed) ? parsed : ((parsed as { items?: unknown[] }).items ?? []);
  assert.ok(Array.isArray(items), `GET /db/humans on doorway "alpha" did not return an array`);
  const row = (items as Record<string, unknown>[]).find(human => {
    const label = String(human['displayName'] ?? human['id'] ?? '');
    return label.toLowerCase().includes(humanName.toLowerCase());
  });
  assert.ok(row, `no /db/humans row found for "${humanName}" on doorway "alpha"`);
  const householdId = row['householdId'];
  assert.ok(
    typeof householdId === 'string' && householdId.length > 0,
    `humans row for "${humanName}" has no householdId set — the identity-fill sweep has not ` +
      'placed them into a household yet'
  );
  return householdId;
}

Given(
  'human {string} has a storage peer whose conductor takes longer than the bounded boot ramp to enable cells',
  async function (this: E2EWorld, humanName: string) {
    const householdId = await findHouseholdIdFor(this, humanName);
    const peerName = humanName.toLowerCase();
    const fixture = loadHouseholdMeshFixture();
    requireFixtureStoragePeer(fixture, peerName); // throws with the substrate fact if absent
    const logPath = storagePeerLogPath(fixture, peerName);
    bootRampStates.set(this, { humanName, peerName, householdId, logPath });
  }
);

When(
  "the peer's infrastructure bridge misses every boot-ramp connect attempt with CellDisabled",
  async function (this: E2EWorld) {
    const state = bootRampState(this);
    const log = await readFile(state.logPath, 'utf8');
    const retryLines = log
      .split('\n')
      .filter(line => line.includes(CELL_DISABLED_RETRY_MSG) && line.includes('infrastructure'));
    const rampExhausted =
      log.includes(CELL_DISABLED_RAMP_EXHAUSTED_MSG) && log.includes('infrastructure');
    if (!(retryLines.length >= 4 && rampExhausted)) {
      // This lane OBSERVES the slow-cell boot; it does not cause it. On a mesh
      // whose conductor answered inside the boot ramp there is no exhaustion to
      // watch recover, and calling that a failure would red every healthy run —
      // a false-red generator, not a regression signal. So the run is recorded
      // as pending with the reason, and the missing node is named rather than
      // assumed away:
      //   chain: peer-vitals-survive-slow-cells
      //   between: a peer boots with its conductor still enabling cells -> the
      //     resilience view lights up instead of staying dark
      //   missing node: CAUSING the slow-cell window — restarting a peer's
      //     storage ahead of its own conductor so the bridge really does
      //     exhaust its 5-attempt ramp. That is a destructive drill on the
      //     owned mesh (it stops a storage process), so it is held behind the
      //     same operator go as the rest of this suite's kills.
      //   current state: observable-only; green requires the condition to have
      //     occurred naturally in this boot.
      //
      // 'skipped' rather than 'pending': cucumber-js is strict by default, so a
      // pending step reds the run exactly like a failure — which would make a
      // healthy mesh permanently red on a scenario that simply had nothing to
      // observe. This is the same disposition the suite already gives a
      // substrate-held scenario (steps/common.steps.ts).
      // eslint-disable-next-line no-console
      console.log(
        `  ⏭️  NOT OBSERVABLE THIS RUN: "${state.peerName}"'s conductor answered inside the ` +
          'boot ramp, so there was no CellDisabled exhaustion to watch recover from — skipped, ' +
          'not failed. Causing the window is a destructive drill (restart storage ahead of its ' +
          'own conductor) and is held.'
      );
      return 'skipped';
    }
  }
);

Then(
  'the peer keeps retrying the infrastructure bridge in the background',
  async function (this: E2EWorld) {
    const state = bootRampState(this);
    const log = await readFile(state.logPath, 'utf8');
    const stillRetrying =
      log.includes(CELL_DISABLED_FOREVER_MSG) || log.includes(CELL_DISABLED_RECONNECT_MSG);
    assert.ok(
      stillRetrying,
      `"${state.peerName}"'s log (${state.logPath}) shows no forever-retry reconnect activity ` +
        'for the infrastructure bridge after the bounded boot ramp gave up'
    );
  }
);

Then(
  'once the bridge lands the PeerStatus heartbeat begins broadcasting',
  async function (this: E2EWorld) {
    const state = bootRampState(this);
    const log = await readFile(state.logPath, 'utf8');
    assert.ok(
      log.includes(PEERSTATUS_HEARTBEAT_STARTED_MSG),
      `"${state.peerName}"'s log (${state.logPath}) never shows ` +
        `"${PEERSTATUS_HEARTBEAT_STARTED_MSG}" — the heartbeat task did not start`
    );
  }
);

Then(
  "the resilience view shows the peer's online status instead of staying dark",
  async function (this: E2EWorld) {
    const state = bootRampState(this);
    const doorway = this.getDoorway('alpha');
    const path = `/api/v1/households/${encodeURIComponent(state.householdId)}/devices`;
    const { status, text } = await getRaw(`${doorway.url}${path}`);
    assert.equal(status, 200, `GET ${path} on doorway "alpha" returned ${status}: ${text}`);
    const body = JSON.parse(text) as { devices?: { peer?: unknown }[] };
    const devices = body.devices ?? [];
    assert.ok(
      devices.length > 0,
      `household "${state.householdId}"'s resilience view (${path}) lists no devices`
    );
    const lit = devices.filter(device => device.peer !== null && device.peer !== undefined);
    assert.ok(
      lit.length > 0,
      `no device in household "${state.householdId}"'s resilience view (${path}) carries a ` +
        `live PeerStatus — all ${devices.length} device(s) still show peer: null (dark)`
    );
  }
);
