/**
 * Household-testable federation failover measures.
 *
 * The client scenarios run deterministically against the real ElohimClient
 * with a controlled transport. Pool and peer-loss scenarios deliberately
 * require live household fixture endpoints. Missing fixture authority is a
 * named assertion failure, never a pending/skip, because these scenarios are
 * un-@wip invariants.
 */

import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';

import { Given, Then, When } from '@cucumber/cucumber';

import { retry } from '../src/framework/utils/retry.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Doorway multi-address client failover
// ---------------------------------------------------------------------------

interface FailoverClient {
  fetch<T>(path: string, options?: RequestInit): Promise<T | null>;
}

type FailoverClientConstructor = new (config: {
  mode: {
    type: 'browser';
    doorway: { url: string; identity: string; fallbacks?: string[] };
  };
}) => FailoverClient;

function isHttpUrl(value: string): boolean {
  return value.startsWith('http://') || value.startsWith('https://');
}

function withoutTrailingSlashes(value: string): string {
  let end = value.length;
  while (end > 0 && value[end - 1] === '/') end -= 1;
  return value.slice(0, end);
}

interface ClientAttempt {
  url: string;
  method: string;
}

interface ClientFailoverState {
  primaryUrl: string;
  fallbackUrl?: string;
  primaryOnline: boolean;
  attempts: ClientAttempt[];
  client: FailoverClient;
  sessionId: string;
  contentView: string;
  lastError?: unknown;
  lastResult?: Record<string, unknown> | null;
  writeAttemptStart?: number;
}

const clientFailoverStates = new WeakMap<E2EWorld, ClientFailoverState>();

function syntheticUrl(value: string, fallback: string): string {
  const configured = process.env[value] ?? value;
  return isHttpUrl(configured) ? withoutTrailingSlashes(configured) : fallback;
}

async function loadFailoverClient(): Promise<FailoverClientConstructor> {
  // Keep the A2O typecheck boundary narrow while still exercising the source
  // implementation at runtime. A static cross-project import would make the
  // Node16 A2O compiler re-typecheck the library's bundler-resolution graph.
  const moduleUrl = new URL(
    '../../../app/elohim-library/projects/elohim-service/src/client/elohim-client.ts',
    import.meta.url
  ).href;
  const loaded = (await import(moduleUrl)) as { ElohimClient: FailoverClientConstructor };
  return loaded.ElohimClient;
}

async function configureFailoverClient(
  world: E2EWorld,
  primaryUrl: string,
  fallbackUrl?: string
): Promise<ClientFailoverState> {
  const ElohimClient = await loadFailoverClient();
  const state: ClientFailoverState = {
    primaryUrl,
    fallbackUrl,
    primaryOnline: true,
    attempts: [],
    client: new ElohimClient({
      mode: {
        type: 'browser',
        doorway: {
          url: primaryUrl,
          identity: 'doorway:alpha',
          fallbacks: fallbackUrl ? [fallbackUrl] : undefined,
        },
      },
    }),
    sessionId: 'household-session-1',
    contentView: 'manifesto',
  };

  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async (input: string | URL | Request, init?: RequestInit) => {
    await Promise.resolve();
    const url =
      typeof input === 'string' || input instanceof URL ? input.toString() : input.url.toString();
    const method = (init?.method ?? 'GET').toUpperCase();
    state.attempts.push({ url, method });

    if (url.startsWith(state.primaryUrl) && !state.primaryOnline) {
      throw new TypeError(`network unavailable at ${state.primaryUrl}`);
    }

    return new Response(
      JSON.stringify({
        contentId: new URL(url).pathname.split('/').at(-1),
        sessionId: state.sessionId,
      }),
      { status: 200, headers: { 'content-type': 'application/json' } }
    );
  }) as typeof fetch;
  world.onCleanup(async () => {
    globalThis.fetch = originalFetch;
    await Promise.resolve();
  });

  clientFailoverStates.set(world, state);
  return state;
}

function clientState(world: E2EWorld): ClientFailoverState {
  const state = clientFailoverStates.get(world);
  assert.ok(state, 'doorway failover client was not configured by the scenario Background');
  return state;
}

Given(
  'doorway {string} is configured with a fallback doorway address {string}',
  async function (this: E2EWorld, doorwayId: string, fallbackSetting: string) {
    const doorway = this.getDoorway(doorwayId);
    const primaryUrl = syntheticUrl(doorway.url, 'https://primary-doorway.test');
    const fallbackUrl = syntheticUrl(fallbackSetting, 'https://fallback-doorway.test');
    await configureFailoverClient(this, primaryUrl, fallbackUrl);
  }
);

Given(
  'the person is viewing a lamad content page served by the primary doorway address',
  async function (this: E2EWorld) {
    const state = clientState(this);
    state.lastResult = await state.client.fetch<Record<string, unknown>>('/db/content/manifesto');
    assert.equal(state.attempts.at(-1)?.url.startsWith(state.primaryUrl), true);
    state.contentView = 'manifesto';
  }
);

Given('the primary doorway address stops answering', function (this: E2EWorld) {
  clientState(this).primaryOnline = false;
});

When('the person navigates to another piece of content', async function (this: E2EWorld) {
  const state = clientState(this);
  state.lastError = undefined;
  try {
    state.lastResult = await state.client.fetch<Record<string, unknown>>(
      '/db/content/constitution'
    );
    state.contentView = 'constitution';
  } catch (error) {
    state.lastError = error;
  }
});

Then('the content loads through the fallback doorway address', function (this: E2EWorld) {
  const state = clientState(this);
  assert.equal(state.lastError, undefined);
  assert.ok(state.lastResult, 'the fallback read returned no content');
  assert.equal(state.attempts.at(-1)?.url.startsWith(state.fallbackUrl ?? 'missing:'), true);
});

Then("the person's session and content view are not lost", function (this: E2EWorld) {
  const state = clientState(this);
  assert.equal(state.sessionId, 'household-session-1');
  assert.equal(state.contentView, 'constitution');
  assert.equal(state.lastResult?.['sessionId'], state.sessionId);
});

Then(
  'subsequent requests from the client stay sticky to the fallback address',
  async function (this: E2EWorld) {
    const state = clientState(this);
    const attemptStart = state.attempts.length;
    await state.client.fetch('/db/content/theology');
    const newAttempts = state.attempts.slice(attemptStart);
    assert.equal(newAttempts.length, 1, 'a sticky read should need exactly one address');
    assert.equal(newAttempts[0]?.url.startsWith(state.fallbackUrl ?? 'missing:'), true);
  }
);

When(
  'the person submits a write while the client is still pointed at the primary address',
  async function (this: E2EWorld) {
    const state = clientState(this);
    state.writeAttemptStart = state.attempts.length;
    state.lastError = undefined;
    try {
      await state.client.fetch('/db/content', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ id: 'household-write' }),
      });
    } catch (error) {
      state.lastError = error;
    }
  }
);

Then(
  'the write fails with an honest error and is not silently retried against another address',
  function (this: E2EWorld) {
    const state = clientState(this);
    assert.ok(state.lastError instanceof TypeError, 'the failed write did not surface its error');
    const attempts = state.attempts.slice(state.writeAttemptStart ?? 0);
    assert.equal(attempts.length, 1, 'a non-idempotent write was retried');
    assert.equal(attempts[0]?.method, 'POST');
    assert.equal(attempts[0]?.url.startsWith(state.primaryUrl), true);
  }
);

When(
  "the person's next read succeeds through the fallback doorway address",
  async function (this: E2EWorld) {
    const state = clientState(this);
    const attemptStart = state.attempts.length;
    state.lastResult = await state.client.fetch('/db/content/confession');
    const attempts = state.attempts.slice(attemptStart);
    assert.equal(attempts.length, 1);
    assert.equal(attempts[0]?.url.startsWith(state.fallbackUrl ?? 'missing:'), true);
  }
);

Then(
  'subsequent writes are attempted against the fallback doorway address',
  async function (this: E2EWorld) {
    const state = clientState(this);
    const attemptStart = state.attempts.length;
    await state.client.fetch('/db/content', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ id: 'write-after-proven-read' }),
    });
    const attempts = state.attempts.slice(attemptStart);
    assert.equal(attempts.length, 1);
    assert.equal(attempts[0]?.method, 'POST');
    assert.equal(attempts[0]?.url.startsWith(state.fallbackUrl ?? 'missing:'), true);
  }
);

Given(
  'doorway {string} has no fallback doorway address configured',
  async function (this: E2EWorld, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    await configureFailoverClient(this, syntheticUrl(doorway.url, 'https://primary-doorway.test'));
  }
);

Then(
  'the request fails the same way it did before multi-address failover existed',
  function (this: E2EWorld) {
    assert.ok(clientState(this).lastError instanceof TypeError);
  }
);

Then('no fallback address is attempted', function (this: E2EWorld) {
  const state = clientState(this);
  assert.equal(state.attempts.length, 1);
  assert.equal(state.attempts[0]?.url.startsWith(state.primaryUrl), true);
});

// ---------------------------------------------------------------------------
// Doorway EPR pool degradation
// ---------------------------------------------------------------------------

interface ProjectionRow {
  urlPath?: string;
  eprId?: string;
}

interface CoherenceManifest {
  doorwayId: string;
  generation: number;
  heads: { urlPath: string; eprId: string }[];
}

interface PoolFailoverState {
  doorwayUrl: string;
  doorwayId: string;
  primaryUrl: string;
  poolUrls: string[];
  generationBefore: number;
  expected: 'peer-projections' | 'empty';
  expectedHeads: { urlPath: string; eprId: string }[];
  servingPeerUrl?: string;
  observed?: CoherenceManifest;
  logPath: string;
  logOffset: number;
  newLogs?: string;
}

const poolFailoverStates = new WeakMap<E2EWorld, PoolFailoverState>();

function requiredHttpUrl(name: string, fallbackName?: string): string {
  const value = process.env[name] ?? (fallbackName ? process.env[fallbackName] : undefined);
  const fallbackHint = fallbackName ? ` (or ${fallbackName})` : '';
  assert.ok(
    value && isHttpUrl(value),
    `live federation fixture missing: set ${name}${fallbackHint}`
  );
  return withoutTrailingSlashes(value);
}

function requiredCsvUrls(name: string): string[] {
  const configured = process.env[name]
    ?.split(',')
    .map(value => value.trim())
    .filter(Boolean);
  const conventional = ['MATTHEW', 'JESSICA', 'JAMES']
    .map(peer => process.env[`E2E_STORAGE_${peer}`])
    .filter((value): value is string => Boolean(value));
  const values = configured?.length ? configured : conventional;
  assert.ok(
    values.length > 0 && values.every(isHttpUrl),
    `live federation fixture missing: set ${name} to comma-separated storage URLs`
  );
  return [...new Set(values.map(withoutTrailingSlashes))];
}

function registeredDoorwayUrl(world: E2EWorld, id: string): string {
  const url = world.getDoorway(id).url;
  assert.ok(isHttpUrl(url), `set the registered ${id} doorway environment URL`);
  return withoutTrailingSlashes(url);
}

async function jsonGet<T>(baseUrl: string, path: string): Promise<T> {
  const response = await fetch(`${baseUrl}${path}`, { signal: AbortSignal.timeout(10_000) });
  const body = await response.text();
  assert.ok(response.ok, `GET ${baseUrl}${path} failed: ${response.status} ${body}`);
  try {
    return JSON.parse(body) as T;
  } catch {
    assert.fail(`GET ${baseUrl}${path} did not return JSON: ${body.slice(0, 240)}`);
  }
}

async function coherence(doorwayUrl: string): Promise<CoherenceManifest> {
  return jsonGet<CoherenceManifest>(doorwayUrl, '/api/v1/federation/coherence');
}

async function projections(storageUrl: string, doorwayId: string): Promise<ProjectionRow[]> {
  const query = new URL(`${storageUrl}/db/rea_commitments`);
  query.searchParams.set('action', 'project-epr');
  query.searchParams.set('doorwayId', doorwayId);
  const response = await fetch(query, { signal: AbortSignal.timeout(10_000) });
  const body = await response.text();
  assert.ok(response.ok, `projection query at ${storageUrl} failed: ${response.status} ${body}`);
  const parsed = JSON.parse(body) as unknown;
  assert.ok(Array.isArray(parsed), `projection query at ${storageUrl} did not return an array`);
  return parsed as ProjectionRow[];
}

function projectionHeads(rows: ProjectionRow[]): { urlPath: string; eprId: string }[] {
  return rows.map(row => {
    const { urlPath, eprId } = row;
    assert.equal(typeof urlPath, 'string', 'projection row has no urlPath');
    assert.equal(typeof eprId, 'string', 'projection row has no eprId');
    return { urlPath: urlPath as string, eprId: eprId as string };
  });
}

async function captureLogStart(): Promise<{ path: string; offset: number }> {
  const path = process.env['E2E_DOORWAY_LOG_PATH'];
  assert.ok(path, 'live federation fixture missing: set E2E_DOORWAY_LOG_PATH');
  let text: string;
  try {
    text = await readFile(path, 'utf8');
  } catch (error) {
    assert.fail(`cannot read E2E_DOORWAY_LOG_PATH (${path}): ${String(error)}`);
  }
  return { path, offset: text.length };
}

async function newLogText(path: string, offset: number): Promise<string> {
  const text = await readFile(path, 'utf8');
  return text.length >= offset ? text.slice(offset) : text;
}

async function initialPoolState(world: E2EWorld): Promise<PoolFailoverState> {
  const doorwayUrl = registeredDoorwayUrl(world, 'alpha');
  const initial = await coherence(doorwayUrl);
  const primaryUrl = requiredHttpUrl('E2E_DOORWAY_PRIMARY_STORAGE_URL', 'E2E_STORAGE_URL');
  const poolUrls = requiredCsvUrls('E2E_DOORWAY_POOL_STORAGE_URLS').filter(
    url => url !== primaryUrl
  );
  assert.ok(poolUrls.length > 0, 'storage pool must contain a non-primary peer URL');
  const log = await captureLogStart();
  const state: PoolFailoverState = {
    doorwayUrl,
    doorwayId: initial.doorwayId,
    primaryUrl,
    poolUrls,
    generationBefore: initial.generation,
    expected: 'empty',
    expectedHeads: [],
    logPath: log.path,
    logOffset: log.offset,
  };
  poolFailoverStates.set(world, state);
  return state;
}

function poolState(world: E2EWorld): PoolFailoverState {
  const state = poolFailoverStates.get(world);
  assert.ok(state, 'EPR pool preconditions were not measured');
  return state;
}

Given(
  "the doorway's primary storage returns zero {string} rows for its doorwayId",
  async function (this: E2EWorld, action: string) {
    assert.equal(action, 'project-epr');
    const state = await initialPoolState(this);
    const rows = await projections(state.primaryUrl, state.doorwayId);
    assert.equal(rows.length, 0, `primary ${state.primaryUrl} returned ${rows.length} rows`);
  }
);

Given(
  "a configured pool peer holds the doorway's {string} rows",
  async function (this: E2EWorld, action: string) {
    assert.equal(action, 'project-epr');
    const state = poolState(this);
    for (const peerUrl of state.poolUrls) {
      const rows = await projections(peerUrl, state.doorwayId);
      if (rows.length > 0) {
        state.expected = 'peer-projections';
        state.expectedHeads = projectionHeads(rows);
        state.servingPeerUrl = peerUrl;
        return;
      }
    }
    assert.fail(`no pool peer held project-epr rows for ${state.doorwayId}`);
  }
);

Given(
  "the doorway's primary storage and all pool peers return zero {string} rows",
  async function (this: E2EWorld, action: string) {
    assert.equal(action, 'project-epr');
    const state = await initialPoolState(this);
    for (const storageUrl of [state.primaryUrl, ...state.poolUrls]) {
      const rows = await projections(storageUrl, state.doorwayId);
      assert.equal(rows.length, 0, `${storageUrl} returned ${rows.length} project-epr rows`);
    }
    state.expected = 'empty';
  }
);

When('the EPR router refresh runs', { timeout: 65_000 }, async function (this: E2EWorld) {
  const state = poolState(this);
  const timeoutMs = Number(process.env['E2E_EPR_REFRESH_WINDOW_MS'] ?? 55_000);
  state.observed = await retry(
    async () => {
      const current = await coherence(state.doorwayUrl);
      assert.ok(
        current.generation > state.generationBefore,
        `router generation has not advanced from ${state.generationBefore}`
      );
      if (state.expected === 'empty') {
        assert.equal(current.heads.length, 0, 'router has not converged to genuine empty state');
      } else {
        for (const expected of state.expectedHeads) {
          assert.ok(
            current.heads.some(
              head => head.urlPath === expected.urlPath && head.eprId === expected.eprId
            ),
            `router is missing pool projection ${expected.urlPath} -> ${expected.eprId}`
          );
        }
      }
      return current;
    },
    {
      maxAttempts: 60,
      initialDelayMs: 500,
      backoffFactor: 1.2,
      maxDelayMs: 2_000,
      timeoutMs,
    }
  );
  state.newLogs = await newLogText(state.logPath, state.logOffset);
});

Then("the router table contains the pool peer's projections", function (this: E2EWorld) {
  const state = poolState(this);
  assert.ok(state.observed);
  for (const expected of state.expectedHeads) {
    assert.ok(
      state.observed.heads.some(
        head => head.urlPath === expected.urlPath && head.eprId === expected.eprId
      )
    );
  }
});

Then('a WARN log names the degraded primary and the serving pool peer', function (this: E2EWorld) {
  const state = poolState(this);
  const logs = state.newLogs ?? '';
  assert.match(logs, /EPR router DEGRADED/);
  assert.ok(logs.includes(state.primaryUrl), `WARN did not name primary ${state.primaryUrl}`);
  assert.ok(
    logs.includes(state.servingPeerUrl ?? 'missing-serving-peer'),
    `WARN did not name serving peer ${state.servingPeerUrl}`
  );
});

Then('the router table is empty', function (this: E2EWorld) {
  assert.deepEqual(poolState(this).observed?.heads, []);
});

Then('the empty state is logged at INFO with the consulted peer list', function (this: E2EWorld) {
  const state = poolState(this);
  const logs = state.newLogs ?? '';
  assert.match(logs, /every storage pool member returned 0 projections/);
  for (const url of [state.primaryUrl, ...state.poolUrls]) {
    assert.ok(logs.includes(url), `empty-state INFO did not name consulted peer ${url}`);
  }
});

Given(
  "the apex doorway's primary storage is missing its projection rows",
  async function (this: E2EWorld) {
    const apexUrl = registeredDoorwayUrl(this, 'apex');
    const manifest = await coherence(apexUrl);
    const primary = requiredHttpUrl('E2E_APEX_PRIMARY_STORAGE_URL');
    const rows = await projections(primary, manifest.doorwayId);
    assert.equal(rows.length, 0, `apex primary ${primary} returned ${rows.length} rows`);
  }
);

// ---------------------------------------------------------------------------
// Household peer-loss failover
// ---------------------------------------------------------------------------

type HouseholdPeerName = 'matthew' | 'jessica' | 'james';

interface HouseholdPeer {
  name: HouseholdPeerName;
  url: string;
  peerId?: string;
}

interface P2PStatus {
  peerId: string;
  connectedPeers: number;
}

interface DeliveryPeer {
  peerId: string;
}

interface InventoryParityWire {
  gossiped_but_missing?: string[];
  local_but_not_gossiped?: string[];
  filesystem_count?: number;
  gossiped_count?: number;
  gossipedButMissing?: string[];
  localButNotGossiped?: string[];
  filesystemCount?: number;
  gossipedCount?: number;
}

interface InventoryParity {
  gossipedButMissing: string[];
  localButNotGossiped: string[];
  filesystemCount: number;
  gossipedCount: number;
}

interface PeerLossState {
  peers: HouseholdPeer[];
  paused: Set<HouseholdPeerName>;
  manifestoHash?: string;
  manifestoBytes?: Uint8Array;
  doorwayFetchBytes?: Uint8Array;
  floor: number;
}

const peerLossStates = new WeakMap<E2EWorld, PeerLossState>();

function householdState(world: E2EWorld): PeerLossState {
  const existing = peerLossStates.get(world);
  if (existing) return existing;

  const names: HouseholdPeerName[] = ['matthew', 'jessica', 'james'];
  const peers = names.map(name => ({
    name,
    url: requiredHttpUrl(`E2E_STORAGE_${name.toUpperCase()}`),
  }));
  const floor = Number(process.env['E2E_CONNECTED_PEERS_FLOOR'] ?? peers.length - 1);
  assert.ok(Number.isInteger(floor) && floor >= 0, 'E2E_CONNECTED_PEERS_FLOOR must be an integer');

  const state: PeerLossState = { peers, paused: new Set(), floor };
  peerLossStates.set(world, state);
  world.onCleanup(async () => {
    for (const name of state.paused) {
      resumePeerProcess(name);
    }
    state.paused.clear();
    await Promise.resolve();
  });
  return state;
}

function peerByName(state: PeerLossState, name: HouseholdPeerName): HouseholdPeer {
  const peer = state.peers.find(candidate => candidate.name === name);
  assert.ok(peer);
  return peer;
}

function peerPid(name: HouseholdPeerName): number {
  const envName = `E2E_STORAGE_${name.toUpperCase()}_PID`;
  const pid = Number(process.env[envName]);
  assert.ok(
    Number.isSafeInteger(pid) && pid > 1 && pid !== process.pid,
    `live peer-loss fixture missing: set ${envName} to that local storage process PID`
  );
  return pid;
}

function pausePeerProcess(name: HouseholdPeerName): void {
  process.kill(peerPid(name), 'SIGSTOP');
}

function resumePeerProcess(name: HouseholdPeerName): void {
  process.kill(peerPid(name), 'SIGCONT');
}

function pausePeer(world: E2EWorld, name: HouseholdPeerName): void {
  const state = householdState(world);
  if (state.paused.has(name)) return;
  pausePeerProcess(name);
  state.paused.add(name);
}

function resumePeer(world: E2EWorld, name: HouseholdPeerName): void {
  const state = householdState(world);
  if (!state.paused.has(name)) return;
  resumePeerProcess(name);
  state.paused.delete(name);
}

async function responseBytes(baseUrl: string, path: string): Promise<Uint8Array> {
  const response = await fetch(`${baseUrl}${path}`, { signal: AbortSignal.timeout(20_000) });
  const bytes = new Uint8Array(await response.arrayBuffer());
  assert.ok(response.ok, `GET ${baseUrl}${path} failed: ${response.status}`);
  return bytes;
}

function normalizeInventoryParity(wire: InventoryParityWire): InventoryParity {
  const normalized = {
    gossipedButMissing: wire.gossiped_but_missing ?? wire.gossipedButMissing ?? [],
    localButNotGossiped: wire.local_but_not_gossiped ?? wire.localButNotGossiped ?? [],
    filesystemCount: wire.filesystem_count ?? wire.filesystemCount ?? -1,
    gossipedCount: wire.gossiped_count ?? wire.gossipedCount ?? -1,
  };
  assert.ok(normalized.filesystemCount >= 0, 'inventory parity omitted filesystem count');
  assert.ok(normalized.gossipedCount >= 0, 'inventory parity omitted gossiped count');
  return normalized;
}

async function loadManifesto(world: E2EWorld, state: PeerLossState): Promise<void> {
  const doorwayUrl = registeredDoorwayUrl(world, 'alpha');
  const content = await jsonGet<Record<string, unknown>>(doorwayUrl, '/db/content/manifesto');
  const hash = content['blobHash'];
  assert.equal(typeof hash, 'string', 'manifesto projection has no blobHash');
  assert.match(hash as string, /^sha256-[a-f0-9]{64}$/);
  state.manifestoHash = hash as string;
}

async function measurePeerIds(state: PeerLossState): Promise<void> {
  for (const peer of state.peers) {
    const status = await jsonGet<P2PStatus>(peer.url, '/p2p/status');
    assert.equal(typeof status.peerId, 'string');
    peer.peerId = status.peerId;
  }
}

async function assertConnectedFloor(state: PeerLossState, peer: HouseholdPeer): Promise<void> {
  const status = await jsonGet<P2PStatus>(peer.url, '/p2p/status');
  assert.ok(
    status.connectedPeers >= state.floor,
    `${peer.name} has ${status.connectedPeers} connected peers; floor is ${state.floor}`
  );
}

async function establishReplicatedManifesto(world: E2EWorld): Promise<PeerLossState> {
  const state = householdState(world);
  await loadManifesto(world, state);
  const path = `/blob/${state.manifestoHash}`;
  let expected: Uint8Array | undefined;
  for (const peer of state.peers) {
    const bytes = await responseBytes(peer.url, path);
    expected ??= bytes;
    assert.deepEqual(bytes, expected, `${peer.name} holds different manifesto bytes`);
  }
  state.manifestoBytes = expected;
  return state;
}

Given(
  'the manifesto blob is replicated under custody across the household mesh',
  async function (this: E2EWorld) {
    await establishReplicatedManifesto(this);
  }
);

Given('every household peer meets its connected-peers floor', async function (this: E2EWorld) {
  const state = householdState(this);
  await measurePeerIds(state);
  for (const peer of state.peers) {
    await assertConnectedFloor(state, peer);
  }
});

When("Jessica's storage peer goes down", function (this: E2EWorld) {
  pausePeer(this, 'jessica');
});

Then(
  'fetching the manifesto blob through the doorway still returns the content',
  async function (this: E2EWorld) {
    const state = householdState(this);
    assert.ok(state.manifestoHash && state.manifestoBytes);
    state.doorwayFetchBytes = await responseBytes(
      registeredDoorwayUrl(this, 'alpha'),
      `/blob/${state.manifestoHash}`
    );
    assert.deepEqual(state.doorwayFetchBytes, state.manifestoBytes);
  }
);

Then('the serving peer is a surviving household peer', function (this: E2EWorld) {
  const state = householdState(this);
  assert.ok(state.paused.has('jessica'), 'Jessica was not actually removed from service');
  assert.ok(state.doorwayFetchBytes?.length, 'no doorway response was served');
  assert.ok(
    state.peers.some(peer => peer.name !== 'jessica' && !state.paused.has(peer.name)),
    'no surviving household peer was available'
  );
});

Given(
  "Jessica's storage peer was down while the mesh kept serving",
  async function (this: E2EWorld) {
    const state = await establishReplicatedManifesto(this);
    for (const peer of state.peers) await assertConnectedFloor(state, peer);
    pausePeer(this, 'jessica');
    const bytes = await responseBytes(
      registeredDoorwayUrl(this, 'alpha'),
      `/blob/${state.manifestoHash}`
    );
    assert.deepEqual(bytes, state.manifestoBytes);
  }
);

When("Jessica's storage peer comes back", function (this: E2EWorld) {
  resumePeer(this, 'jessica');
});

Then(
  "within the re-sync window Jessica's peer meets its connected-peers floor again",
  { timeout: 65_000 },
  async function (this: E2EWorld) {
    const state = householdState(this);
    await retry(async () => assertConnectedFloor(state, peerByName(state, 'jessica')), {
      maxAttempts: 40,
      initialDelayMs: 500,
      backoffFactor: 1.25,
      maxDelayMs: 2_500,
      timeoutMs: Number(process.env['E2E_PEER_RESYNC_WINDOW_MS'] ?? 55_000),
    });
  }
);

Then(
  "Jessica's peer inventory parity matches the mesh again",
  { timeout: 65_000 },
  async function (this: E2EWorld) {
    const state = householdState(this);
    await retry(
      async () => {
        const reports = await Promise.all(
          state.peers.map(async peer => {
            const wire = await jsonGet<InventoryParityWire>(
              peer.url,
              '/api/v1/diagnostics/inventory-parity'
            );
            return normalizeInventoryParity(wire);
          })
        );
        const jessica = reports[state.peers.findIndex(peer => peer.name === 'jessica')];
        assert.deepEqual(jessica?.gossipedButMissing, []);
        assert.deepEqual(jessica?.localButNotGossiped, []);
        assert.equal(jessica?.filesystemCount, jessica?.gossipedCount);
        assert.equal(
          new Set(reports.map(report => report.filesystemCount)).size,
          1,
          'household filesystem inventories still differ'
        );
      },
      {
        maxAttempts: 40,
        initialDelayMs: 500,
        backoffFactor: 1.25,
        maxDelayMs: 2_500,
        timeoutMs: Number(process.env['E2E_PEER_RESYNC_WINDOW_MS'] ?? 55_000),
      }
    );
  }
);

Given('the household peers matthew, jessica, and james are up', async function (this: E2EWorld) {
  const state = householdState(this);
  for (const peer of state.peers) {
    const response = await fetch(`${peer.url}/health`, { signal: AbortSignal.timeout(5_000) });
    assert.ok(response.ok, `${peer.name} health returned ${response.status}`);
  }
  await measurePeerIds(state);
});

When("each peer's live peer set is inspected", async function (this: E2EWorld) {
  const state = householdState(this);
  for (const peer of state.peers) {
    const live = await jsonGet<DeliveryPeer[]>(peer.url, '/api/v1/peers/delivery');
    this.contentIds.set(`live-peers:${peer.name}`, live.map(item => item.peerId).join(','));
  }
});

Then(
  'every household peer lists every other household peer as connected',
  function (this: E2EWorld) {
    const state = householdState(this);
    for (const peer of state.peers) {
      const live = new Set((this.contentIds.get(`live-peers:${peer.name}`) ?? '').split(','));
      for (const other of state.peers.filter(candidate => candidate.name !== peer.name)) {
        assert.ok(other.peerId, `${other.name} peer id was not measured`);
        assert.ok(live.has(other.peerId), `${peer.name} does not list ${other.name} as connected`);
      }
    }
  }
);

Given("Jessica's storage peer holds content it stewards locally", async function (this: E2EWorld) {
  const state = householdState(this);
  const jessica = peerByName(state, 'jessica');
  const content = await jsonGet<Record<string, unknown>>(jessica.url, '/db/content/manifesto');
  const hash = content['blobHash'];
  assert.equal(typeof hash, 'string');
  state.manifestoHash = hash as string;
  state.manifestoBytes = await responseBytes(jessica.url, `/blob/${hash}`);
});

When('every other household peer is unreachable', function (this: E2EWorld) {
  pausePeer(this, 'matthew');
  pausePeer(this, 'james');
});

Then("Jessica's peer still serves its locally stewarded content", async function (this: E2EWorld) {
  const state = householdState(this);
  assert.ok(state.manifestoHash && state.manifestoBytes);
  const bytes = await responseBytes(
    peerByName(state, 'jessica').url,
    `/blob/${state.manifestoHash}`
  );
  assert.deepEqual(bytes, state.manifestoBytes);
});

Then(
  'the degraded mesh state is visible in her peer status, not hidden',
  { timeout: 35_000 },
  async function (this: E2EWorld) {
    const state = householdState(this);
    const status = await retry(
      async () => {
        const current = await jsonGet<P2PStatus>(peerByName(state, 'jessica').url, '/p2p/status');
        assert.ok(
          current.connectedPeers < state.floor,
          `Jessica still reports ${current.connectedPeers} peers at floor ${state.floor}`
        );
        return current;
      },
      {
        maxAttempts: 30,
        initialDelayMs: 500,
        backoffFactor: 1.2,
        maxDelayMs: 2_000,
        timeoutMs: Number(process.env['E2E_PEER_DEGRADE_WINDOW_MS'] ?? 25_000),
      }
    );
    assert.ok(status.connectedPeers < state.floor);
  }
);
