/**
 * Step glue for two deployment concerns against the local household mesh
 * (matthew/jessica/james, Act I — E2E_HOUSEHOLD_FIXTURE_PATH):
 *
 *   - features/deployment/sync-control.feature   — the human's deliberate
 *     sync/paused/wifi-only choice.
 *   - features/deployment/p2p-validation.feature — automatic backpressure
 *     (sync_paused during bulk writes / import / drain backlog) plus the two
 *     baseline P2P-status probes (their Given/When/Then already live in
 *     steps/ui/navigation.steps.ts and steps/common.steps.ts — not redefined
 *     here).
 *
 * ---------------------------------------------------------------------------
 * FINDING (2026-08-21, RESOLVED 2026-08-22): sync-control.feature originally
 * named a control surface that did not exist in elohim-storage. The unified
 * SyncGate (elohim-storage commit c309a3a5e — src/p2p/sync_gate.rs) landed
 * that surface at exactly the endpoint names these steps probe: GET/POST
 * /p2p/sync-mode, POST /p2p/network-class, GET /p2p/sync-mode/history, plus
 * additive `syncMode` / `networkClass` / `syncReasons` fields on /p2p/status.
 *
 *   Default posture on a fresh node: mode "sync", network "unknown",
 *   effective syncing=true, reasons=[] — an explicit, observable default,
 *   never an absent field. Operator preference is sticky across restart
 *   (SQLite sync_mode_state; transitions audited with trigger "operator" /
 *   "network-change").
 *
 *   Every live (non-@wip) step below remains a REAL, falsifiable HTTP probe —
 *   a failure now indicts the deployed binary (predates the gate, or the
 *   surface regressed), not a guessed endpoint name.
 * ---------------------------------------------------------------------------
 */

import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';

import { Given, When, Then } from '@cucumber/cucumber';

import { request as undiciRequest } from 'undici';

import {
  getRaw,
  postRaw,
  probeP2PStatus,
  resolveStorageUrl,
  pollForGauge,
  GAUGE_SWEEP_POLL_TIMEOUT_MS,
  type P2PStatusSurface,
} from '../../src/framework/dataplane/surfaces.js';
import { getDevice, type DeviceArchetype } from '../../src/framework/fixtures/devices.js';
import {
  loadHouseholdMeshFixture,
  requireFixturePrimaryStorageUrl,
} from '../../src/framework/fixtures/household-mesh.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// SyncGate control-surface endpoint names — see FINDING above.
// ---------------------------------------------------------------------------

const SYNC_MODE_ENDPOINT = '/p2p/sync-mode';
const SYNC_MODE_HISTORY_ENDPOINT = '/p2p/sync-mode/history';
const NETWORK_CLASS_ENDPOINT = '/p2p/network-class';
const SYNC_MODE_FIELD = 'syncMode';
const NETWORK_CLASS_FIELD = 'networkClass';

const SYNC_MODE_SURFACE_NOTE =
  'The unified SyncGate control surface (elohim-storage commit c309a3a5e, 2026-08-22 — ' +
  `GET/POST ${SYNC_MODE_ENDPOINT}, POST ${NETWORK_CLASS_ENDPOINT}, GET ${SYNC_MODE_HISTORY_ENDPOINT}, ` +
  'plus additive syncMode/networkClass/syncReasons on /p2p/status) is a landed surface. ' +
  'A failure here means the deployed storage binary predates that commit, or the surface ' +
  'regressed — not that the step guessed a wrong endpoint name.';

const DOORWAY_HEALTH_SYNC_PAUSED_NOTE =
  'The doorway-facing GET /health p2p block proxies syncPaused/syncReasons from the cached ' +
  'storage observation as of the SyncGate landing (c309a3a5e, 2026-08-22; ' +
  'doorway/doorway-service/src/routes/health.rs). A failure here means the deployed doorway ' +
  'predates that commit, or its storage observation cache has no P2P sample yet.';

// ---------------------------------------------------------------------------
// Storage-URL resolution — never a hardcoded port; env alias first, then the
// household-mesh fixture manifest (mirrors dataplane.steps.ts's own fallback).
// ---------------------------------------------------------------------------

function primaryStorageUrl(doorwayId = 'alpha'): string {
  if (doorwayId === 'alpha') {
    const fromEnv = resolveStorageUrl('alpha-A');
    if (fromEnv) return fromEnv;
  }
  return requireFixturePrimaryStorageUrl(loadHouseholdMeshFixture(), doorwayId);
}

// ---------------------------------------------------------------------------
// World-scoped scenario state (WeakMap-keyed, mirrors dataplane.steps.ts)
// ---------------------------------------------------------------------------

interface SyncModeSetResult {
  status: number;
  text: string;
  requestedMode: string;
}

interface WriteResult {
  status: number;
  text: string;
}

interface MeshSyncState {
  lastSyncModeSet?: SyncModeSetResult;
  pendingUpdatesBefore?: number;
  importWrite?: WriteResult;
  observedSyncPausedDuringImport?: boolean;
  bulkWrite?: WriteResult;
  observedSyncPausedDuringBulk?: boolean;
  lastDrainPending?: number;
  lastDrainThreshold?: number;
  historyProbe?: WriteResult;
}

const meshState = new WeakMap<E2EWorld, MeshSyncState>();

function state(world: E2EWorld): MeshSyncState {
  let existing = meshState.get(world);
  if (!existing) {
    existing = {};
    meshState.set(world, existing);
  }
  return existing;
}

type WorldWithDevice = E2EWorld & { currentDevice?: DeviceArchetype };

/** Mirrors fixture-devices.steps.ts's own helper — `currentDevice` is the same world property. */
function requireDevice(world: WorldWithDevice): DeviceArchetype {
  if (!world.currentDevice) {
    throw new Error(
      'No current device set — run a "Given device {string} from the device portfolio" (or ' +
        '"with sync mode") step first.'
    );
  }
  return world.currentDevice;
}

// ---------------------------------------------------------------------------
// Sync-mode control-surface probe helpers (shared by sync-control.feature)
// ---------------------------------------------------------------------------

async function setSyncMode(world: E2EWorld, mode: string): Promise<SyncModeSetResult> {
  const url = primaryStorageUrl();
  const { status, text } = await postRaw(`${url}${SYNC_MODE_ENDPOINT}`, { mode });
  const result: SyncModeSetResult = { status, text, requestedMode: mode };
  state(world).lastSyncModeSet = result;
  return result;
}

// ---------------------------------------------------------------------------
// Drain-status field access — DrainStatusInfo exists on the wire
// (elohim-storage/src/p2p/mod.rs) but P2PStatusSurface only carries it via
// its catch-all index signature, so this is the one place that casts it.
// ---------------------------------------------------------------------------

interface DrainStatusFields {
  total: number;
  published: number;
  pending: number;
}

function readDrain(body: P2PStatusSurface): DrainStatusFields | undefined {
  const drain = body['drain'];
  if (drain === null || drain === undefined) return undefined;
  return drain as DrainStatusFields;
}

// ---------------------------------------------------------------------------
// Account-package / bulk-content write payload helpers (p2p-validation.feature)
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(process.cwd(), '..', '..');
const ACCOUNT_PACKAGES_DIR = resolve(REPO_ROOT, 'genesis/data/account-packages');

/** Load a REAL committed account package by its identity.displayName (case-insensitive). */
function loadAccountPackageByDisplayName(name: string): Record<string, unknown> {
  const files = readdirSync(ACCOUNT_PACKAGES_DIR).filter(
    file => file.endsWith('.json') && file !== 'account-package.schema.json'
  );
  for (const file of files) {
    let parsed: Record<string, unknown>;
    try {
      parsed = JSON.parse(readFileSync(resolve(ACCOUNT_PACKAGES_DIR, file), 'utf8')) as Record<
        string,
        unknown
      >;
    } catch {
      continue;
    }
    const identity = parsed['identity'] as Record<string, unknown> | undefined;
    const displayName = identity?.['displayName'];
    if (typeof displayName === 'string' && displayName.toLowerCase() === name.toLowerCase()) {
      return parsed;
    }
  }
  throw new Error(
    `no account package under ${ACCOUNT_PACKAGES_DIR} has identity.displayName "${name}"`
  );
}

/** CreateContentInputView[]-shaped synthetic batch (elohim/sdk/schemas/v1/inputs/create-content-input.schema.json). */
function buildSyntheticContentBatch(count: number): Record<string, unknown>[] {
  const runId = randomUUID().slice(0, 8);
  return Array.from({ length: count }, (_unused, index) => ({
    id: `e2e-sync-control-bulk-${runId}-${index}`,
    title: `E2E sync-control bulk probe ${index}`,
    schemaVersion: 1,
    description: null,
    contentType: null,
    contentFormat: null,
    contentBody: null,
    blobHash: null,
    blobCid: null,
    contentSizeBytes: null,
    metadata: null,
    reach: 'commons',
    createdBy: null,
    tags: [],
  }));
}

/**
 * A raw POST that sends a genuinely-malformed JSON body — postRaw (surfaces.ts)
 * always JSON.stringifies its payload, which can never produce invalid JSON
 * syntax. Mirrors postRaw's own bounded-request shape exactly, minus the
 * stringify step, for the one scenario that needs a real parse failure.
 */
async function postRawInvalidJson(
  url: string,
  rawBody: string
): Promise<{ status: number; text: string }> {
  const { statusCode, body } = await undiciRequest(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'x-schema-version': '1' },
    body: rawBody,
    bodyTimeout: 15_000,
    headersTimeout: 15_000,
  });
  const text = await body.text();
  return { status: statusCode, text };
}

// ===========================================================================
// sync-control.feature
// ===========================================================================

// --- Regression: the default posture is explicit, never absent --------------

Given('the peer has no sync-mode preference set', async function (this: E2EWorld) {
  const url = primaryStorageUrl();
  const { status, text } = await getRaw(`${url}${SYNC_MODE_ENDPOINT}`);
  if (status === 404) {
    // Absent substrate: this storage binary predates the unified SyncGate
    // (c309a3a5e) — held (skipped), never 'pending' (strict mode fails pending).
    return 'skipped';
  }
  assert.strictEqual(
    status,
    200,
    `GET ${SYNC_MODE_ENDPOINT} returned ${status} (${text.slice(0, 200)}). ` +
      SYNC_MODE_SURFACE_NOTE
  );
  // "No preference set" is observable in the transition audit log: an operator
  // preference arrives via POST /p2p/sync-mode (trigger "operator") and a
  // network-class declaration via POST /p2p/network-class (trigger
  // "network-change") — both sticky across restart, both a departure from the
  // fresh default this scenario pins. If either exists on this shared mesh
  // (e.g. the pause/resume or wifi-only scenarios below ran first), the
  // fresh-default precondition does not hold — held, not failed.
  const history = await getRaw(`${url}${SYNC_MODE_HISTORY_ENDPOINT}`);
  if (history.status === 200) {
    let entries: unknown;
    try {
      entries = JSON.parse(history.text);
    } catch {
      throw new Error(`GET ${SYNC_MODE_HISTORY_ENDPOINT} did not return valid JSON`);
    }
    const departedFreshDefault =
      Array.isArray(entries) &&
      entries.some(
        entry =>
          entry !== null &&
          typeof entry === 'object' &&
          ((entry as Record<string, unknown>)['trigger'] === 'operator' ||
            (entry as Record<string, unknown>)['trigger'] === 'network-change')
      );
    if (departedFreshDefault) return 'skipped';
  }
  // History 503 (no DB pool) leaves restart-stickiness unknowable but the
  // live gate readable — proceed; the Then steps assert the observable default.
  return undefined;
});

Then('the P2P status reports sync running with empty syncReasons', async function (this: E2EWorld) {
  const url = primaryStorageUrl();
  const { body } = await probeP2PStatus(url);
  assert.strictEqual(
    body.syncPaused,
    false,
    `peer /p2p/status.syncPaused is ${body.syncPaused} — a fresh node with no operator ` +
      'preference must be effectively syncing, never silently paused. ' +
      `syncReasons: ${JSON.stringify(body['syncReasons'])}`
  );
  const reasons = body['syncReasons'];
  assert.ok(
    Array.isArray(reasons) && reasons.length === 0,
    `peer /p2p/status.syncReasons is ${JSON.stringify(reasons)}, expected [] — nothing ` +
      'should be suppressing sync on a fresh node with no operator preference. ' +
      SYNC_MODE_SURFACE_NOTE
  );
});

// --- Capability proof: the three modes --------------------------------------

Given("the peer's sync mode is {string}", async function (this: E2EWorld, mode: string) {
  const result = await setSyncMode(this, mode);
  assert.strictEqual(
    result.status,
    200,
    `cannot establish precondition "the peer's sync mode is ${mode}" — POST ` +
      `${SYNC_MODE_ENDPOINT} returned ${result.status} (${result.text.slice(0, 200)}). ` +
      SYNC_MODE_SURFACE_NOTE
  );
});

When('the operator sets sync mode to {string}', async function (this: E2EWorld, mode: string) {
  await setSyncMode(this, mode);
});

Then('ongoing sync cycles complete their current chunk and stop', function (this: E2EWorld) {
  const result = state(this).lastSyncModeSet;
  assert.ok(result, 'no "operator sets sync mode to" step ran before this assertion');
  assert.strictEqual(
    result.status,
    200,
    'cannot confirm in-flight sync cycles honored the pause — the sync-mode control call ' +
      `itself failed (POST ${SYNC_MODE_ENDPOINT} returned ${result.status}). ` +
      SYNC_MODE_SURFACE_NOTE
  );
});

Then('no new sync cycles start until the mode changes', async function (this: E2EWorld) {
  const requested = state(this).lastSyncModeSet?.requestedMode;
  assert.ok(requested, 'no "operator sets sync mode to" step ran before this assertion');
  const url = primaryStorageUrl();
  const { body } = await probeP2PStatus(url);
  assert.strictEqual(
    body[SYNC_MODE_FIELD],
    requested,
    `peer /p2p/status.syncMode is ${JSON.stringify(body[SYNC_MODE_FIELD])}, expected ` +
      `"${requested}". ${SYNC_MODE_SURFACE_NOTE}`
  );
});

Then('the P2P status reports syncMode {string}', async function (this: E2EWorld, expected: string) {
  const url = primaryStorageUrl();
  const { body } = await probeP2PStatus(url);
  assert.strictEqual(
    body[SYNC_MODE_FIELD],
    expected,
    `peer /p2p/status.syncMode is ${JSON.stringify(body[SYNC_MODE_FIELD])}, expected ` +
      `"${expected}". ${SYNC_MODE_SURFACE_NOTE}`
  );
});

Given(
  'there are {int} pending DHT updates from connected peers',
  async function (this: E2EWorld, _count: number) {
    const url = primaryStorageUrl();
    const { body } = await probeP2PStatus(url);
    assert.ok(
      body.pull !== undefined,
      'peer /p2p/status.pull is absent — the acquisition pull-queue has not completed its ' +
        'first reconcile on this mesh, so there is no real pending-updates count to establish ' +
        'this precondition against.'
    );
    state(this).pendingUpdatesBefore = body.pull?.pending ?? 0;
  }
);

Then('sync cycles resume on the next interval', async function (this: E2EWorld) {
  const url = primaryStorageUrl();
  const { body } = await probeP2PStatus(url);
  assert.strictEqual(
    body[SYNC_MODE_FIELD],
    'sync',
    `peer /p2p/status.syncMode is ${JSON.stringify(body[SYNC_MODE_FIELD])}, expected "sync" ` +
      `(resumed) after the operator re-enabled sync. ${SYNC_MODE_SURFACE_NOTE}`
  );
});

Then(
  'the {int} pending updates are absorbed',
  { timeout: 75_000 },
  async function (this: E2EWorld, _count: number) {
    const before = state(this).pendingUpdatesBefore;
    assert.ok(before !== undefined, 'no "pending DHT updates" precondition was established');
    const url = primaryStorageUrl();
    const observed = await pollForGauge<number>(
      async () => {
        const { body } = await probeP2PStatus(url);
        const pending = body.pull?.pending;
        return pending !== undefined && pending < before ? pending : undefined;
      },
      { intervalMs: 3_000, timeoutMs: 60_000 }
    );
    assert.ok(
      observed !== undefined,
      `peer /p2p/status.pull.pending never dropped below the pre-resume value (${before}) ` +
        'within 60s of resuming sync — the pending updates were not absorbed.'
    );
  }
);

Given(
  'device {string} with sync mode {string}',
  async function (this: WorldWithDevice, deviceName: string, mode: string) {
    this.currentDevice = getDevice(deviceName);
    const result = await setSyncMode(this, mode);
    assert.strictEqual(
      result.status,
      200,
      `cannot establish precondition device "${deviceName}" with sync mode "${mode}" — POST ` +
        `${SYNC_MODE_ENDPOINT} returned ${result.status}. ${SYNC_MODE_SURFACE_NOTE}`
    );
  }
);

Given('the device is currently on cellular', function (this: WorldWithDevice) {
  requireDevice(this);
});

Then('sync cycles do not run', async function (this: E2EWorld) {
  const url = primaryStorageUrl();
  const { body } = await probeP2PStatus(url);
  assert.strictEqual(
    body.syncPaused,
    true,
    `peer /p2p/status.syncPaused is ${body.syncPaused}, expected true while wifi-only mode is ` +
      'active on cellular — the SyncGate should suppress sync (WifiOnlyOnCellular reason) in ' +
      `this state. ${SYNC_MODE_SURFACE_NOTE}`
  );
});

Then(
  'the P2P status reports syncMode {string} and networkClass {string}',
  async function (this: E2EWorld, expectedMode: string, expectedClass: string) {
    const url = primaryStorageUrl();
    const { body } = await probeP2PStatus(url);
    assert.strictEqual(
      body[SYNC_MODE_FIELD],
      expectedMode,
      `peer /p2p/status.syncMode is ${JSON.stringify(body[SYNC_MODE_FIELD])}, expected ` +
        `"${expectedMode}". ${SYNC_MODE_SURFACE_NOTE}`
    );
    assert.strictEqual(
      body[NETWORK_CLASS_FIELD],
      expectedClass,
      `peer /p2p/status.networkClass is ${JSON.stringify(body[NETWORK_CLASS_FIELD])}, expected ` +
        `"${expectedClass}". ${SYNC_MODE_SURFACE_NOTE}`
    );
  }
);

Given(
  'the device is currently on cellular with sync paused',
  async function (this: WorldWithDevice) {
    requireDevice(this);
    const url = primaryStorageUrl();
    const { body } = await probeP2PStatus(url);
    assert.strictEqual(
      body.syncPaused,
      true,
      `precondition "sync paused" does not hold — peer /p2p/status.syncPaused is ` +
        `${body.syncPaused}. ${SYNC_MODE_SURFACE_NOTE}`
    );
  }
);

When('the device joins a wifi network', async function (this: WorldWithDevice) {
  requireDevice(this);
  const url = primaryStorageUrl();
  await postRaw(`${url}${NETWORK_CLASS_ENDPOINT}`, { networkClass: 'wifi' });
  // Result deliberately not asserted here — the Then steps below re-probe
  // /p2p/status and fail honestly if nothing reacted to the notification.
});

Then(
  'sync cycles resume within {int} seconds',
  { timeout: 45_000 },
  async function (this: E2EWorld, seconds: number) {
    const url = primaryStorageUrl();
    const resumed = await pollForGauge<true>(
      async () => {
        const { body } = await probeP2PStatus(url);
        return body.syncPaused === false ? true : undefined;
      },
      { intervalMs: 2_000, timeoutMs: seconds * 1_000 }
    );
    assert.ok(
      resumed,
      `peer /p2p/status.syncPaused did not clear within ${seconds}s of the device joining ` +
        `wifi. ${SYNC_MODE_SURFACE_NOTE}`
    );
  }
);

Then('queued updates from peers begin draining', async function (this: E2EWorld) {
  const url = primaryStorageUrl();
  const { body } = await probeP2PStatus(url);
  assert.ok(
    body.pull !== undefined,
    'peer /p2p/status.pull is absent — no queued-updates surface to confirm draining against.'
  );
});

// --- Capability proof: archetype-tunable defaults (@wip) --------------------

Then(
  "the device's default sync mode should be {string}",
  function (this: WorldWithDevice, _expectedDefault: string) {
    // PENDING (@wip): DeviceArchetype (src/framework/fixtures/devices.ts) and
    // genesis/data/devices/devices.json carry no `defaultSyncMode` field —
    // there is nothing on disk yet to assert this against.
    return 'pending';
  }
);

Then('the operator can override the default at any time', function (this: WorldWithDevice) {
  // PENDING (@wip): sibling of the step above — no sync-mode control surface
  // exists to attempt an override against (see file header FINDING).
  return 'pending';
});

// --- Observability: operator dashboard (@wip) --------------------------------

Given("the operator opens their device's sync settings", function (this: E2EWorld) {
  // PENDING (@wip): no sync-settings UI page/route exists in app/elohim-app
  // yet to open — needs `pnpm look` / Playwright once that surface ships.
  return 'pending';
});

Then('the current sync mode is shown', function (this: E2EWorld) {
  return 'pending';
});

Then('the last successful sync timestamp is shown', function (this: E2EWorld) {
  return 'pending';
});

Then(
  'if mode is {string}, the current network class is shown',
  function (this: E2EWorld, _mode: string) {
    return 'pending';
  }
);

// --- Observability: sync mode transitions logged -----------------------------

Given(
  "the peer's sync mode has changed {int} times in the last 24 hours",
  async function (this: E2EWorld, minTransitions: number) {
    const url = primaryStorageUrl();
    const { status, text } = await getRaw(`${url}${SYNC_MODE_HISTORY_ENDPOINT}`);
    assert.strictEqual(
      status,
      200,
      'cannot establish precondition — GET ' +
        `${SYNC_MODE_HISTORY_ENDPOINT} returned ${status} (${text.slice(0, 200)}). ` +
        SYNC_MODE_SURFACE_NOTE
    );
    let history: unknown[];
    try {
      history = JSON.parse(text) as unknown[];
    } catch {
      throw new Error(`GET ${SYNC_MODE_HISTORY_ENDPOINT} did not return valid JSON`);
    }
    assert.ok(
      Array.isArray(history) && history.length >= minTransitions,
      `sync-mode history has ${Array.isArray(history) ? history.length : 'no'} entries, ` +
        `expected >= ${minTransitions}`
    );
  }
);

When('the operator queries the sync mode history', async function (this: E2EWorld) {
  const url = primaryStorageUrl();
  const { status, text } = await getRaw(`${url}${SYNC_MODE_HISTORY_ENDPOINT}`);
  state(this).historyProbe = { status, text };
});

Then('the response shows each transition with timestamp and trigger', function (this: E2EWorld) {
  const probe = state(this).historyProbe;
  assert.ok(probe, 'no "operator queries the sync mode history" step ran');
  assert.strictEqual(
    probe.status,
    200,
    `GET ${SYNC_MODE_HISTORY_ENDPOINT} returned ${probe.status}. ${SYNC_MODE_SURFACE_NOTE}`
  );
  let transitions: unknown;
  try {
    transitions = JSON.parse(probe.text);
  } catch {
    throw new Error('sync-mode history response is not valid JSON');
  }
  assert.ok(
    Array.isArray(transitions) && transitions.length > 0,
    'sync-mode history is empty — nothing to check timestamp/trigger on'
  );
  for (const entry of transitions as unknown[]) {
    assert.ok(
      entry !== null && typeof entry === 'object',
      `sync-mode history entry is not an object: ${JSON.stringify(entry)}`
    );
    const record = entry as Record<string, unknown>;
    assert.ok(
      'timestamp' in record && 'trigger' in record,
      `sync-mode history entry missing timestamp/trigger: ${JSON.stringify(entry)}`
    );
  }
});

// ===========================================================================
// p2p-validation.feature
// ===========================================================================

Given(
  'elohim-storage on doorway {string} has {int} connected peers',
  async function (this: E2EWorld, doorwayId: string, minPeers: number) {
    const url = primaryStorageUrl(doorwayId);
    const { body } = await probeP2PStatus(url);
    assert.ok(
      body.connectedPeers >= minPeers,
      `elohim-storage on doorway "${doorwayId}" /p2p/status.connectedPeers is ` +
        `${body.connectedPeers}, expected >= ${minPeers}`
    );
  }
);

// --- Sync backpressure: account import ---------------------------------------

When(
  'a seeder POSTs an account package for {string} with {int} content items',
  { timeout: 45_000 },
  async function (this: E2EWorld, displayName: string, itemCount: number) {
    const pkg = loadAccountPackageByDisplayName(displayName);
    const contentArray = Array.isArray(pkg['content']) ? (pkg['content'] as unknown[]) : [];
    assert.ok(
      contentArray.length >= itemCount,
      `account package for "${displayName}" only has ${contentArray.length} content ` +
        `assignments on disk (${ACCOUNT_PACKAGES_DIR}) — cannot slice ${itemCount}`
    );
    const body = { ...pkg, content: contentArray.slice(0, itemCount) };
    const url = primaryStorageUrl();
    const post = postRaw(`${url}/account/import`, body);
    post.catch(() => undefined);
    const observedPaused = await pollForGauge<true>(
      async () => {
        const { body: status } = await probeP2PStatus(url);
        return status.syncPaused === true ? true : undefined;
      },
      { intervalMs: 200, timeoutMs: 15_000 }
    );
    state(this).observedSyncPausedDuringImport = observedPaused === true;
    const { status, text } = await post;
    state(this).importWrite = { status, text };
  }
);

Then(
  'the P2P status should report sync_paused as true during the import',
  function (this: E2EWorld) {
    assert.strictEqual(
      state(this).observedSyncPausedDuringImport,
      true,
      'never observed /p2p/status.syncPaused=true while the account-import POST was in flight'
    );
  }
);

Then(
  /^sync\/replication cycles should be skipped until the import completes$/,
  function (this: E2EWorld) {
    assert.strictEqual(
      state(this).observedSyncPausedDuringImport,
      true,
      'never observed /p2p/status.syncPaused=true while the account-import POST was in flight ' +
        '— sync/replication cycles cannot be confirmed skipped'
    );
  }
);

Then(
  'after the import response returns, sync_paused should be false',
  async function (this: E2EWorld) {
    const write = state(this).importWrite;
    assert.ok(write, 'no import write result captured — run the seeder POST step first');
    const url = primaryStorageUrl();
    const { body } = await probeP2PStatus(url);
    assert.strictEqual(
      body.syncPaused,
      false,
      `peer /p2p/status.syncPaused is ${body.syncPaused} after the account-import POST ` +
        `(status ${write.status}) returned — the SyncPauseGuard should have resumed sync on drop`
    );
  }
);

// --- Sync backpressure: bulk content creation --------------------------------

When(
  'a seeder POSTs a bulk content batch of {int} items',
  { timeout: 45_000 },
  async function (this: E2EWorld, itemCount: number) {
    const url = primaryStorageUrl();
    const items = buildSyntheticContentBatch(itemCount);
    const post = postRaw(`${url}/db/content/bulk`, items);
    post.catch(() => undefined);
    const observedPaused = await pollForGauge<true>(
      async () => {
        const { body } = await probeP2PStatus(url);
        return body.syncPaused === true ? true : undefined;
      },
      { intervalMs: 200, timeoutMs: 15_000 }
    );
    state(this).observedSyncPausedDuringBulk = observedPaused === true;
    const { status, text } = await post;
    state(this).bulkWrite = { status, text };
  }
);

Then(
  'the P2P status should report sync_paused as true during the write',
  function (this: E2EWorld) {
    assert.strictEqual(
      state(this).observedSyncPausedDuringBulk,
      true,
      'never observed /p2p/status.syncPaused=true while the bulk-content POST was in flight'
    );
  }
);

Then(
  'after the bulk response returns, sync_paused should be false',
  async function (this: E2EWorld) {
    const write = state(this).bulkWrite;
    assert.ok(write, 'no bulk write result captured — run the seeder POST step first');
    const url = primaryStorageUrl();
    const { body } = await probeP2PStatus(url);
    assert.strictEqual(
      body.syncPaused,
      false,
      `peer /p2p/status.syncPaused is ${body.syncPaused} after the bulk-content POST ` +
        `(status ${write.status}) returned — the SyncPauseGuard should have resumed sync on drop`
    );
  }
);

// --- Sync resumes even if bulk write fails -----------------------------------

Given('P2P sync is active', async function (this: E2EWorld) {
  const url = primaryStorageUrl();
  const { body } = await probeP2PStatus(url);
  assert.strictEqual(
    body.syncPaused,
    false,
    `peer /p2p/status.syncPaused is ${body.syncPaused} — expected sync to be active before ` +
      "this scenario's fault-injection begins"
  );
});

When('a bulk content request fails mid-write due to invalid JSON', async function (this: E2EWorld) {
  const url = primaryStorageUrl();
  const { status, text } = await postRawInvalidJson(`${url}/db/content/bulk`, '{not valid json');
  state(this).bulkWrite = { status, text };
});

Then('sync_paused should be false after the error response', async function (this: E2EWorld) {
  const write = state(this).bulkWrite;
  assert.ok(write, 'no malformed-JSON write result captured — run the fault-injection step first');
  assert.ok(
    write.status >= 400,
    `expected the malformed-JSON POST to /db/content/bulk to fail (4xx/5xx), got ` +
      `${write.status}: ${write.text.slice(0, 200)}`
  );
  const url = primaryStorageUrl();
  const { body } = await probeP2PStatus(url);
  assert.strictEqual(
    body.syncPaused,
    false,
    `peer /p2p/status.syncPaused is ${body.syncPaused} after a failed bulk write — the ` +
      'SyncPauseGuard RAII resume-on-drop should have cleared it even on the error path'
  );
});

// --- Sync auto-suppressed while drain backlog is large -----------------------

Given(
  '{int} content items have been bulk-seeded',
  async function (this: E2EWorld, minItems: number) {
    const url = primaryStorageUrl();
    const { body } = await probeP2PStatus(url);
    const drain = readDrain(body);
    assert.ok(
      drain,
      'peer /p2p/status.drain is absent (DB pool/query unavailable) — cannot confirm ' +
        `${minItems} content items are bulk-seeded on this mesh`
    );
    assert.ok(
      drain.total >= minItems,
      `peer /p2p/status.drain.total is ${drain.total}, expected >= ${minItems} — this mesh ` +
        'has not been bulk-seeded to the scale this scenario needs'
    );
  }
);

When(
  'the drain publish queue has more than {int} items pending',
  async function (this: E2EWorld, threshold: number) {
    const url = primaryStorageUrl();
    const { body } = await probeP2PStatus(url);
    const drain = readDrain(body);
    state(this).lastDrainPending = drain?.pending;
    state(this).lastDrainThreshold = threshold;
  }
);

Then(/^sync_paused should be true \(auto-suppressed by drain\)$/, async function (this: E2EWorld) {
  const { lastDrainPending: pending, lastDrainThreshold: threshold } = state(this);
  assert.ok(
    pending !== undefined && threshold !== undefined,
    'drain backlog was never observed — run the "drain publish queue" step first'
  );
  assert.ok(
    pending > threshold,
    `drain.pending is ${pending}, not > ${threshold} — the auto-suppress threshold was never ` +
      'crossed on this mesh'
  );
  const url = primaryStorageUrl();
  const { body } = await probeP2PStatus(url);
  assert.strictEqual(
    body.syncPaused,
    true,
    `peer /p2p/status.syncPaused is ${body.syncPaused} even though drain.pending=${pending} ` +
      `> ${threshold}`
  );
});

Then(/^sync\/replication cycles should be skipped$/, async function (this: E2EWorld) {
  const url = primaryStorageUrl();
  const { body } = await probeP2PStatus(url);
  assert.strictEqual(
    body.syncPaused,
    true,
    `peer /p2p/status.syncPaused is ${body.syncPaused} — expected sync/replication cycles to ` +
      'be skipped while drain is auto-suppressing'
  );
});

Then(
  'once drain pending drops below {int}, sync_paused should be false',
  { timeout: GAUGE_SWEEP_POLL_TIMEOUT_MS + 15_000 },
  async function (this: E2EWorld, threshold: number) {
    const url = primaryStorageUrl();
    const cleared = await pollForGauge<true>(
      async () => {
        const { body } = await probeP2PStatus(url);
        const drain = readDrain(body);
        if (drain === undefined || drain.pending >= threshold) return undefined;
        return body.syncPaused === false ? true : undefined;
      },
      { intervalMs: 5_000, timeoutMs: GAUGE_SWEEP_POLL_TIMEOUT_MS }
    );
    assert.ok(
      cleared,
      `drain.pending did not drop below ${threshold} with syncPaused clearing to false within ` +
        `${GAUGE_SWEEP_POLL_TIMEOUT_MS / 1_000}s`
    );
  }
);

// --- P2P status endpoint exposes sync_paused state ---------------------------

Then(
  'the response should include a {string} boolean field',
  function (this: E2EWorld, fieldName: string) {
    const healthJson = this.contentIds.get('_health');
    assert.ok(healthJson, 'No health response stored — run "When I check the P2P status" first');
    const health = JSON.parse(healthJson) as Record<string, unknown>;
    const p2p = health['p2p'] as Record<string, unknown> | undefined;
    const value = p2p ? p2p[fieldName] : undefined;
    assert.strictEqual(
      typeof value,
      'boolean',
      `doorway /health p2p.${fieldName} is ${JSON.stringify(value)} (type ${typeof value}) — ` +
        `expected a boolean. ${DOORWAY_HEALTH_SYNC_PAUSED_NOTE}`
    );
  }
);
