import { strict as assert } from 'node:assert';

import { Given, Then } from '@cucumber/cucumber';

import {
  classifyStorageTransportStatus,
  getRaw,
  parseLabeledPrometheusMetric,
  pollForGauge,
  pollForGaugeCapturingError,
  probeMetrics,
  probeP2PStatus,
  probeSyncDocHeads,
  resolveStorageUrl,
  type ParsedMetrics,
  type StorageTransportMode,
} from '../../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../../src/framework/world.js';

const HOUSEHOLD_PEERS = ['matthew', 'jessica', 'james'] as const;
const ACTIVITY_TIMEOUT_MS = 90_000;
const STALENESS_TIMEOUT_MS = 120_000;
const STALENESS_METRIC = 'elohim_sync_projected_apply_staleness_seconds';

interface StalenessSample {
  count: number | null;
  sum: number | null;
}

interface SyncSnapshot {
  mode: StorageTransportMode;
  metrics: Map<string, ParsedMetrics>;
  projectedApplyStalenessBefore: Map<string, StalenessSample>;
  receivers: readonly string[];
  selectedPlane?: string;
}

const snapshots = new WeakMap<E2EWorld, SyncSnapshot>();
const selections = new WeakMap<
  E2EWorld,
  { mode: StorageTransportMode; receivers: readonly string[]; selectedPlane?: string }
>();

function storageUrl(peer: string): string {
  const url = resolveStorageUrl(peer);
  assert.ok(url, `E2E_STORAGE_${peer.toUpperCase()} is not set`);
  return url;
}

async function observeHouseholdMode(): Promise<StorageTransportMode> {
  const observations = await Promise.all(
    HOUSEHOLD_PEERS.map(async peer => {
      const status = await probeP2PStatus(storageUrl(peer));
      return { peer, mode: classifyStorageTransportStatus(status.body) };
    })
  );
  const modes = new Set(observations.map(observation => observation.mode));
  assert.equal(
    modes.size,
    1,
    `household peers disagree on transport: ${observations
      .map(observation => `${observation.peer}=${observation.mode}`)
      .join(', ')}`
  );
  const [mode] = modes;
  assert.notEqual(mode, 'unknown', 'the household transport could not be classified');
  return mode;
}

function selectedPlanes(mode: StorageTransportMode): string[] {
  if (mode === 'dual') return ['libp2p', 'iroh'];
  if (mode === 'libp2p' || mode === 'iroh') return [mode];
  return [];
}

function counterAdvanced(
  mode: StorageTransportMode,
  before: ParsedMetrics,
  after: ParsedMetrics
): boolean {
  const advanced = (name: string) => (after.get(name) ?? 0) > (before.get(name) ?? 0);
  const libp2pAdvanced = advanced('elohim_sync_rounds_total');
  const irohAdvanced = advanced('elohim_iroh_sync_rounds_total');
  if (mode === 'libp2p') return libp2pAdvanced;
  if (mode === 'iroh') return irohAdvanced;
  if (mode === 'dual') return libp2pAdvanced && irohAdvanced;
  return false;
}

function sameHeads(left: string[], right: string[]): boolean {
  const leftSet = new Set(left);
  const rightSet = new Set(right);
  return leftSet.size === rightSet.size && [...leftSet].every(head => rightSet.has(head));
}

async function projectedApplyStaleness(peer: string, plane: string): Promise<StalenessSample> {
  const response = await getRaw(`${storageUrl(peer)}/metrics`);
  assert.equal(response.status, 200, `GET ${storageUrl(peer)}/metrics returned ${response.status}`);
  return {
    count: parseLabeledPrometheusMetric(response.text, `${STALENESS_METRIC}_count`, 'plane', plane),
    sum: parseLabeledPrometheusMetric(response.text, `${STALENESS_METRIC}_sum`, 'plane', plane),
  };
}

Given(
  'the household runs in {string} mode with selected planes {string}',
  async function (this: E2EWorld, modeText: string, planesText: string) {
    const expected = process.env['E2E_EXPECTED_TRANSPORT'];
    assert.ok(expected, 'E2E_EXPECTED_TRANSPORT is not set by the mesh lane');
    if (modeText !== expected) return 'skipped';
    const mode = await observeHouseholdMode();
    assert.equal(
      mode,
      modeText,
      `live household transport is ${mode}, scenario selected ${modeText}`
    );
    assert.deepEqual(
      planesText.split('+'),
      selectedPlanes(mode),
      `selected plane declaration does not match ${mode} mode`
    );
    selections.set(this, {
      mode,
      receivers: HOUSEHOLD_PEERS.filter(peer => peer !== 'matthew'),
      selectedPlane: mode === 'libp2p' || mode === 'iroh' ? mode : undefined,
    });
  }
);

Given(
  'Matthew and James run in {string} mode while receiver {string} runs in {string} mode',
  async function (this: E2EWorld, authorMode: string, receiver: string, receiverMode: string) {
    assert.ok(
      HOUSEHOLD_PEERS.includes(receiver as (typeof HOUSEHOLD_PEERS)[number]),
      `unknown household receiver ${receiver}`
    );
    assert.ok(
      authorMode === 'libp2p' || authorMode === 'iroh' || authorMode === 'dual',
      `unsupported author transport ${authorMode}`
    );
    assert.ok(
      receiverMode === 'libp2p' || receiverMode === 'iroh' || receiverMode === 'dual',
      `unsupported receiver transport ${receiverMode}`
    );
    const observations = await Promise.all(
      HOUSEHOLD_PEERS.map(async peer => {
        const status = await probeP2PStatus(storageUrl(peer));
        return { peer, mode: classifyStorageTransportStatus(status.body) };
      })
    );
    for (const observation of observations) {
      const expectedMode: string = observation.peer === receiver ? receiverMode : authorMode;
      assert.equal(
        observation.mode,
        expectedMode,
        `${observation.peer} transport is ${observation.mode}, expected ${expectedMode}`
      );
    }
    selections.set(this, {
      mode: authorMode as StorageTransportMode,
      receivers: [receiver],
      selectedPlane: receiverMode,
    });
  }
);

Given(
  "the household's current transport sync counters are recorded",
  async function (this: E2EWorld) {
    const selection = selections.get(this);
    const mode = selection?.mode ?? (await observeHouseholdMode());
    const metrics = new Map<string, ParsedMetrics>();
    const plane = selection?.selectedPlane ? [selection.selectedPlane] : selectedPlanes(mode);
    const receivers = selection?.receivers ?? HOUSEHOLD_PEERS.filter(peer => peer !== 'matthew');
    const projectedApplyStalenessBefore = new Map<string, StalenessSample>();
    await Promise.all(
      HOUSEHOLD_PEERS.map(async peer => {
        const peerMetrics = await probeMetrics(storageUrl(peer));
        metrics.set(peer, peerMetrics);
        // The dual scenario records sync counters only. Its racing planes
        // cannot be truthfully attributed to one origin-to-apply sample.
        if (plane.length === 1 && receivers.includes(peer)) {
          projectedApplyStalenessBefore.set(peer, await projectedApplyStaleness(peer, plane[0]));
        }
      })
    );
    snapshots.set(this, {
      mode,
      metrics,
      projectedApplyStalenessBefore,
      receivers,
      selectedPlane: plane.length === 1 ? plane[0] : undefined,
    });
  }
);

Then(
  "every household peer serves the new document at Matthew's exact Automerge heads within {int} seconds",
  // Matthew's local-projection budget (15s) plus the requested fresh
  // cross-peer convergence budget (see below) plus assertion/HTTP margin.
  { timeout: 180_000 },
  async function (this: E2EWorld, convergenceSeconds: number) {
    const contentId = this.contentIds.get('lastContentId');
    assert.ok(contentId, 'the fresh Matthew-authored content id was not captured');
    const docId = `node:${contentId}`;

    // Matthew's own local projection is a SEPARATE concern from cross-peer
    // convergence — sharing one 30s deadline across both meant a slow local
    // projection silently ate into the peers' convergence budget. Give it
    // its own 15s budget so a convergence failure always gets its full,
    // fresh 30s window (the intended fresh-write contract — the 60s backstop
    // for the eager-propagation cure landing in storage in parallel).
    let matthewHeads: string[] = [];
    const matthewProjection = await pollForGaugeCapturingError(
      async () => {
        const response = await probeSyncDocHeads(storageUrl('matthew'), 'elohim', docId);
        matthewHeads = response.body.heads;
        return matthewHeads.length > 0 ? true : undefined;
      },
      { intervalMs: 1_000, timeoutMs: 15_000 }
    );
    assert.equal(
      matthewProjection.value,
      true,
      `${docId} did not project on Matthew within 15 seconds` +
        (matthewProjection.lastError ? ` (last error: ${matthewProjection.lastError})` : '')
    );

    const peersConverged = await Promise.all(
      HOUSEHOLD_PEERS.map(async peer => {
        let peerHeads: string[] = [];
        const result = await pollForGaugeCapturingError(
          async () => {
            const response = await probeSyncDocHeads(storageUrl(peer), 'elohim', docId);
            peerHeads = response.body.heads;
            return sameHeads(peerHeads, matthewHeads) ? true : undefined;
          },
          { intervalMs: 1_000, timeoutMs: convergenceSeconds * 1_000 }
        );
        return {
          peer,
          converged: result.value === true,
          peerHeads,
          lastError: result.lastError,
        };
      })
    );
    const divergent = peersConverged.filter(result => !result.converged);
    assert.deepEqual(
      divergent,
      [],
      `new document did not converge to Matthew's heads [${matthewHeads.join(', ')}]: ` +
        divergent
          .map(
            result =>
              `${result.peer}=[${result.peerHeads.join(', ')}]` +
              (result.lastError ? ` (last error: ${result.lastError})` : '')
          )
          .join(', ')
    );
  }
);

async function assertProjectedApplyStaleness(world: E2EWorld): Promise<void> {
  const snapshot = snapshots.get(world);
  assert.ok(snapshot, 'sync metrics were not recorded before Matthew authored the new document');
  const planes = snapshot.selectedPlane ? [snapshot.selectedPlane] : selectedPlanes(snapshot.mode);
  assert.equal(
    planes.length,
    1,
    `single-plane staleness evidence cannot be attributed in ${snapshot.mode} mode`
  );
  const [plane] = planes;
  const receivers = snapshot.receivers;
  const outcomes = await Promise.all(
    receivers.map(async peer => {
      const before = snapshot.projectedApplyStalenessBefore.get(peer) ?? {
        count: null,
        sum: null,
      };
      let after: StalenessSample = { count: null, sum: null };
      const result = await pollForGauge(
        async () => {
          after = await projectedApplyStaleness(peer, plane);
          // Only a new successful histogram sample is evidence. The invalid
          // timestamp counter is deliberately never treated as a substitute.
          let advanced = false;
          if (after.count !== null) {
            if (before.count === null) {
              advanced = after.count > 0;
            } else {
              advanced = after.count > before.count;
            }
          }
          return advanced ? true : undefined;
        },
        { intervalMs: 2_000, timeoutMs: STALENESS_TIMEOUT_MS }
      );
      return {
        peer,
        observed: result === true,
        countDelta: (after.count ?? 0) - (before.count ?? 0),
        sumDeltaSeconds: (after.sum ?? 0) - (before.sum ?? 0),
      };
    })
  );
  world.attach(
    JSON.stringify({ measure: 'origin-to-projected-apply-staleness', plane, outcomes }, null, 2),
    'application/json'
  );
  const missing = outcomes.filter(outcome => !outcome.observed).map(outcome => outcome.peer);
  assert.deepEqual(
    missing,
    [],
    `receiving peers did not record new ${plane} origin-to-successful-projected-apply ` +
      `staleness instrumentation samples (invalid/missing timestamp counters are not valid ` +
      `evidence, and plane-wide samples are not uniquely correlated to this document): ${missing.join(', ')}`
  );
}

Then(
  'every receiving household peer records new valid origin-to-projected-apply samples for the selected plane',
  { timeout: STALENESS_TIMEOUT_MS + 15_000 },
  async function (this: E2EWorld) {
    await assertProjectedApplyStaleness(this);
  }
);

Then(
  'receiver {string} records new valid origin-to-projected-apply samples for the {string} plane',
  { timeout: STALENESS_TIMEOUT_MS + 15_000 },
  async function (this: E2EWorld, receiver: string, plane: string) {
    const selection = selections.get(this);
    assert.ok(selection, 'the mixed transport selection was not recorded');
    assert.deepEqual(selection.receivers, [receiver], 'the named receiver must match the fixture');
    assert.equal(selection.selectedPlane, plane, 'the named plane must match the receiver mode');
    await assertProjectedApplyStaleness(this);
  }
);

Then(
  'every household peer completed new sync work on every selected plane',
  { timeout: ACTIVITY_TIMEOUT_MS + 15_000 },
  async function (this: E2EWorld) {
    const snapshot = snapshots.get(this);
    assert.ok(snapshot, 'sync counters were not recorded before Matthew authored the new document');
    const outcomes = await Promise.all(
      HOUSEHOLD_PEERS.map(async peer => {
        const before = snapshot.metrics.get(peer)!;
        const moved = await pollForGauge(
          async () => {
            const after = await probeMetrics(storageUrl(peer));
            return counterAdvanced(snapshot.mode, before, after) ? true : undefined;
          },
          { intervalMs: 2_000, timeoutMs: ACTIVITY_TIMEOUT_MS }
        );
        return { peer, moved: moved === true };
      })
    );
    const stalled = outcomes.filter(outcome => !outcome.moved).map(outcome => outcome.peer);
    assert.deepEqual(stalled, [], `fresh sync work did not complete on: ${stalled.join(', ')}`);
  }
);
