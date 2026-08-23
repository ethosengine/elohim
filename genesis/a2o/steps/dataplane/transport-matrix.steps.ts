import { strict as assert } from 'node:assert';

import { Given, Then } from '@cucumber/cucumber';

import {
  classifyStorageTransportStatus,
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

interface SyncSnapshot {
  mode: StorageTransportMode;
  metrics: Map<string, ParsedMetrics>;
}

const snapshots = new WeakMap<E2EWorld, SyncSnapshot>();

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
  }
);

Given(
  "the household's current transport sync counters are recorded",
  async function (this: E2EWorld) {
    const mode = await observeHouseholdMode();
    const metrics = new Map<string, ParsedMetrics>();
    await Promise.all(
      HOUSEHOLD_PEERS.map(async peer => metrics.set(peer, await probeMetrics(storageUrl(peer))))
    );
    snapshots.set(this, { mode, metrics });
  }
);

Then(
  "every household peer serves the new document at Matthew's exact Automerge heads within 30 seconds",
  // Matthew's local-projection budget (15s) plus a FRESH 30s cross-peer
  // convergence budget (see below) plus assertion/HTTP margin.
  { timeout: 60_000 },
  async function (this: E2EWorld) {
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
          { intervalMs: 1_000, timeoutMs: 30_000 }
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
