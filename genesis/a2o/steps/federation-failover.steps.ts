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
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { once } from 'node:events';
import { readFile } from 'node:fs/promises';

import { After, Given, Then, When } from '@cucumber/cucumber';

import {
  loadHouseholdMeshFixture,
  requireFixtureDoorwayLogPath,
  requireFixturePeerPid,
  requireFixturePoolStorageUrls,
  requireFixturePrimaryStorageUrl,
  requireFixtureStoragePeer,
  type HouseholdMeshFixture,
} from '../src/framework/fixtures/household-mesh.js';
import {
  resolveOwnedMeshProcess,
  signalOwnedMeshProcess,
  type OwnedProcessHandle,
} from '../src/framework/fixtures/owned-doorway-process.js';
import {
  destructiveAllowed,
  DESTRUCTIVE_HELD_HINT,
} from '../src/framework/fixtures/substrate-scope.js';
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

type FailoverMode = 'owned-substrate' | 'synthetic';
const OWNED_SUBSTRATE: FailoverMode = 'owned-substrate';
const SYNTHETIC: FailoverMode = 'synthetic';

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
  /** Which leg is really running: a real SIGSTOP against a mesh this run owns, or the mocked-fetch synthetic path. */
  mode: FailoverMode;
  /** The real, un-patched `fetch` — used for this file's OWN verification probes so they never pollute `attempts`. */
  rawFetch: typeof fetch;
  /** Resolved lazily on first pause; absent in synthetic mode. */
  doorwayHandle?: OwnedProcessHandle;
  doorwayPaused: boolean;
}

const clientFailoverStates = new WeakMap<E2EWorld, ClientFailoverState>();

function syntheticUrl(value: string, fallback: string): string {
  const configured = process.env[value] ?? value;
  return isHttpUrl(configured) ? withoutTrailingSlashes(configured) : fallback;
}

/**
 * Does THIS run own its substrate well enough to induce a REAL doorway
 * outage instead of a mocked one? Both real doorway processes must be
 * addressable (E2E_DOORWAY_ALPHA / E2E_DOORWAY_B, the mesh lane's own names
 * for doorway "a" and doorway "b" — see justfile's `test mesh` recipe) AND
 * the household fixture must declare `processControl: true` (a local stack
 * this harness can SIGSTOP/SIGCONT, never a deployed pod). Any other shape
 * — the deployed fleet, a partial env, a missing/unreadable fixture — falls
 * through to the synthetic mock, exactly as before this scenario upgrade.
 */
function ownedFailoverSubstrate(): { primaryUrl: string; fallbackUrl: string } | null {
  const primaryEnv = process.env['E2E_DOORWAY_ALPHA'];
  const fallbackEnv = process.env['E2E_DOORWAY_B'];
  if (!primaryEnv || !fallbackEnv || !isHttpUrl(primaryEnv) || !isHttpUrl(fallbackEnv)) {
    return null;
  }
  let fixture: HouseholdMeshFixture;
  try {
    fixture = loadHouseholdMeshFixture();
  } catch {
    return null;
  }
  if (fixture.processControl !== true) return null;
  return {
    primaryUrl: withoutTrailingSlashes(primaryEnv),
    fallbackUrl: withoutTrailingSlashes(fallbackEnv),
  };
}

/**
 * An "honest" failure for these scenarios: a network-level rejection, never
 * a parsed HTTP response. The synthetic mock always throws a bare
 * `TypeError`; a REAL frozen doorway can also surface as a caller/attempt
 * timeout (`AbortError` from an explicit abort, `TimeoutError` from
 * `AbortSignal.timeout()`) depending on which side notices the outage first.
 */
function isHonestNetworkFailure(error: unknown): boolean {
  if (!(error instanceof Error)) return false;
  if (error instanceof TypeError) return true;
  return error.name === 'AbortError' || error.name === 'TimeoutError';
}

// ---------------------------------------------------------------------------
// Real-substrate fault control — SIGSTOPping the owned doorway "a" process,
// the same start-tick-guarded /proc technique as
// dataplane/doorway-sibling-reader.steps.ts:77's `signalOwnedDoorway` (see
// src/framework/fixtures/owned-doorway-process.ts). Coordinated with
// whatever ELSE is pausing/resuming owned mesh processes in this run through
// the same `$MESH_DIR/a2o.lock` flock doorway-sibling-reader.steps.ts and
// apex-transition.steps.ts already use for the identical reason.
// ---------------------------------------------------------------------------

const doorwayLeases = new WeakMap<E2EWorld, ChildProcessWithoutNullStreams>();

async function acquireDoorwayLease(world: E2EWorld): Promise<void> {
  if (doorwayLeases.has(world)) return;
  const child = spawn(
    '/usr/bin/flock',
    [
      '-n',
      // eslint-disable-next-line sonarjs/publicly-writable-directories -- The owned mesh's shared lock coordinates every fault run.
      `${process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh'}/a2o.lock`,
      '/bin/bash',
      '-c',
      'echo locked; read -r _',
    ],
    { stdio: 'pipe' }
  );
  doorwayLeases.set(world, child);
  const granted = await Promise.race([
    once(child.stdout, 'data').then(([chunk]) => String(chunk).includes('locked')),
    once(child, 'exit').then(() => false),
  ]);
  assert.ok(granted, 'household is already in use; no doorway fault is allowed');
}

function releaseDoorwayLease(world: E2EWorld): void {
  doorwayLeases.get(world)?.stdin.end();
  doorwayLeases.delete(world);
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

/**
 * The two real content ids this owned-substrate leg reads through the
 * failover — deliberately real, seeded rows (curl-verified 200 on the
 * household mesh), never the fabricated ids the synthetic mock accepted for
 * any path. Which of the two a given step reads is arbitrary (transport is
 * what these scenarios certify, not content diversity); reusing a small,
 * known-good pair keeps every real read a genuine 200 with a real body.
 */
const OWNED_CONTENT_PRIMARY = 'manifesto';
const OWNED_CONTENT_SECONDARY = 'elohim-host-landing';

async function configureFailoverClient(
  world: E2EWorld,
  primaryUrl: string,
  fallbackUrl: string | undefined,
  mode: FailoverMode
): Promise<ClientFailoverState> {
  const ElohimClient = await loadFailoverClient();
  const originalFetch = globalThis.fetch;
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
    mode,
    rawFetch: originalFetch,
    doorwayPaused: false,
  };

  if (mode === OWNED_SUBSTRATE) {
    // Resolve doorway "a"'s real process handle up front so a later pause
    // step never has to (the resolution itself is read-only and cheap).
    state.doorwayHandle = await resolveOwnedMeshProcess('doorway', 'a', 'doorway A');
    // A passthrough SPY, not a mock: every request the client issues really
    // goes over the wire to the real doorway/fallback, and is merely
    // recorded for the Then steps' attempt-tracking assertions. This file's
    // OWN verification probes (health checks around pause/resume) use
    // `state.rawFetch` directly so they never pollute `attempts`.
    globalThis.fetch = (async (input: string | URL | Request, init?: RequestInit) => {
      const url =
        typeof input === 'string' || input instanceof URL ? input.toString() : input.url.toString();
      const method = (init?.method ?? 'GET').toUpperCase();
      state.attempts.push({ url, method });
      return originalFetch(input, init);
    }) as typeof fetch;
  } else {
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
  }

  const modeLabel = mode === OWNED_SUBSTRATE ? 'owned-substrate (real SIGSTOP)' : SYNTHETIC;
  // eslint-disable-next-line no-console
  console.log(`  mode: ${modeLabel}`);
  world.attach(JSON.stringify({ mode: modeLabel, primaryUrl, fallbackUrl }), 'application/json');

  clientFailoverStates.set(world, state);
  return state;
}

/** A content path this leg's mode can actually resolve with a real 200 body. */
function contentPath(state: ClientFailoverState, syntheticId: string, ownedId: string): string {
  return state.mode === OWNED_SUBSTRATE ? `/db/content/${ownedId}` : `/db/content/${syntheticId}`;
}

function clientState(world: E2EWorld): ClientFailoverState {
  const state = clientFailoverStates.get(world);
  assert.ok(state, 'doorway failover client was not configured by the scenario Background');
  return state;
}

/**
 * Recovery has priority over everything else: this runs for EVERY scenario
 * (no tag filter — the WeakMap lookup is a no-op for every scenario that
 * never configured a failover client) so a real SIGSTOP against the shared
 * household mesh is undone even when an assertion fails mid-scenario. SIGCONT
 * first, THEN verify the primary answers again — same order as
 * doorway-sibling-reader.steps.ts's own `@concern:doorway-failover` After
 * hook and apex-transition.steps.ts's, for the identical reason.
 */
After({ timeout: 20_000 }, async function (this: E2EWorld) {
  const state = clientFailoverStates.get(this);
  if (!state) return;
  try {
    globalThis.fetch = state.rawFetch;
    if (state.doorwayPaused && state.doorwayHandle) {
      await signalOwnedMeshProcess(state.doorwayHandle, 'SIGCONT', 'doorway A');
      state.doorwayPaused = false;
      const health = await state.rawFetch(`${state.primaryUrl}/health`, {
        signal: AbortSignal.timeout(5000),
      });
      assert.equal(health.status, 200, 'fault teardown must restore the primary doorway');
    }
  } finally {
    releaseDoorwayLease(this);
    clientFailoverStates.delete(this);
  }
});

Given(
  'doorway {string} is configured with a fallback doorway address {string}',
  async function (this: E2EWorld, doorwayId: string, fallbackSetting: string) {
    const owned = ownedFailoverSubstrate();
    if (owned) {
      await configureFailoverClient(this, owned.primaryUrl, owned.fallbackUrl, OWNED_SUBSTRATE);
      return;
    }
    const doorway = this.getDoorway(doorwayId);
    const primaryUrl = syntheticUrl(doorway.url, 'https://primary-doorway.test');
    const fallbackUrl = syntheticUrl(fallbackSetting, 'https://fallback-doorway.test');
    await configureFailoverClient(this, primaryUrl, fallbackUrl, 'synthetic');
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

Given(
  'the primary doorway address stops answering',
  { timeout: 15_000 },
  async function (this: E2EWorld) {
    const state = clientState(this);
    if (state.mode === 'synthetic') {
      state.primaryOnline = false;
      return;
    }
    assert.ok(state.doorwayHandle, 'owned doorway process handle was not resolved');
    await acquireDoorwayLease(this);
    await signalOwnedMeshProcess(state.doorwayHandle, 'SIGSTOP', 'doorway A');
    state.doorwayPaused = true;
    await assert.rejects(
      state.rawFetch(`${state.primaryUrl}/health`, { signal: AbortSignal.timeout(1500) }),
      'the paused primary doorway must actually stop answering'
    );
  }
);

When('the person navigates to another piece of content', async function (this: E2EWorld) {
  const state = clientState(this);
  state.lastError = undefined;
  try {
    state.lastResult = await state.client.fetch<Record<string, unknown>>(
      contentPath(state, 'constitution', OWNED_CONTENT_SECONDARY)
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
  if (state.mode === 'synthetic') {
    // The synthetic mock echoes the client's own sessionId back; a real
    // content row carries no such field, so this correlation is
    // synthetic-only. Real mode already proved a real 200 body above.
    assert.equal(state.lastResult?.['sessionId'], state.sessionId);
  }
});

Then(
  'subsequent requests from the client stay sticky to the fallback address',
  async function (this: E2EWorld) {
    const state = clientState(this);
    const attemptStart = state.attempts.length;
    await state.client.fetch(contentPath(state, 'theology', OWNED_CONTENT_PRIMARY));
    const newAttempts = state.attempts.slice(attemptStart);
    assert.equal(newAttempts.length, 1, 'a sticky read should need exactly one address');
    assert.equal(newAttempts[0]?.url.startsWith(state.fallbackUrl ?? 'missing:'), true);
  }
);

/**
 * Owned mode only: how long the write-during-outage attempt is allowed to
 * run before this test gives up waiting for it. A frozen doorway (SIGSTOP)
 * still completes the TCP handshake — the kernel accepts it — so with no
 * bound the request would hang until undici's own multi-minute default
 * timeout, breaching the "seconds, not minutes" freeze-window contract this
 * scenario upgrade is required to hold. The client's own non-retriable-write
 * path deliberately injects no abort of its own (a real in-flight write must
 * run to completion rather than risk a duplicate) — so this file supplies
 * the bound itself, exactly as a real caller's own network stack eventually
 * would.
 */
const OWNED_WRITE_TIMEOUT_MS = 4_000;

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
        ...(state.mode === OWNED_SUBSTRATE
          ? { signal: AbortSignal.timeout(OWNED_WRITE_TIMEOUT_MS) }
          : {}),
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
    assert.ok(
      isHonestNetworkFailure(state.lastError),
      `the failed write did not surface an honest network error: ${String(state.lastError)}`
    );
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
    state.lastResult = await state.client.fetch(
      contentPath(state, 'confession', OWNED_CONTENT_SECONDARY)
    );
    const attempts = state.attempts.slice(attemptStart);
    if (state.mode === 'synthetic') {
      // The synthetic mock's immediate (non-abort) TypeError on the earlier
      // failed write already advanced the client's sticky preference, so
      // this read needs exactly one hop.
      assert.equal(attempts.length, 1);
    } else {
      // The earlier real write was bounded by THIS file's own AbortSignal
      // (OWNED_WRITE_TIMEOUT_MS) rather than the client's natural
      // network-failure path, so it does not itself advance stickiness (a
      // caller-aborted attempt short-circuits before that update — see
      // ElohimClient.fetchAgainstCandidates). This read may therefore need
      // to probe the still-frozen primary once (its OWN internally-injected
      // per-attempt timeout, not a caller abort) before it succeeds through
      // the fallback — 1 or 2 real attempts, never more.
      assert.ok(
        attempts.length === 1 || attempts.length === 2,
        `expected 1-2 real attempts, saw ${attempts.length}`
      );
    }
    assert.equal(attempts.at(-1)?.url.startsWith(state.fallbackUrl ?? 'missing:'), true);
  }
);

Then(
  'subsequent writes are attempted against the fallback doorway address',
  async function (this: E2EWorld) {
    const state = clientState(this);
    const attemptStart = state.attempts.length;
    try {
      await state.client.fetch('/db/content', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ id: 'write-after-proven-read' }),
      });
    } catch {
      // Routing to the fallback is what this step certifies; the real
      // storage's write-schema acceptance of this fixture body is out of
      // scope (the synthetic mock always accepts, so this never fires there).
    }
    const attempts = state.attempts.slice(attemptStart);
    assert.equal(attempts.length, 1);
    assert.equal(attempts[0]?.method, 'POST');
    assert.equal(attempts[0]?.url.startsWith(state.fallbackUrl ?? 'missing:'), true);
  }
);

Given(
  'doorway {string} has no fallback doorway address configured',
  async function (this: E2EWorld, doorwayId: string) {
    const owned = ownedFailoverSubstrate();
    if (owned) {
      await configureFailoverClient(this, owned.primaryUrl, undefined, OWNED_SUBSTRATE);
      return;
    }
    const doorway = this.getDoorway(doorwayId);
    await configureFailoverClient(
      this,
      syntheticUrl(doorway.url, 'https://primary-doorway.test'),
      undefined,
      'synthetic'
    );
  }
);

Then(
  'the request fails the same way it did before multi-address failover existed',
  function (this: E2EWorld) {
    assert.ok(isHonestNetworkFailure(clientState(this).lastError));
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

/** The doorwayId-scoped commitment kind these degrade scenarios shade/observe. */
const PROJECT_EPR_KIND = 'project-epr';

async function projections(storageUrl: string, doorwayId: string): Promise<ProjectionRow[]> {
  const query = new URL(`${storageUrl}/db/rea_commitments`);
  query.searchParams.set('action', PROJECT_EPR_KIND);
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
  const path = requireFixtureDoorwayLogPath(loadHouseholdMeshFixture(), 'alpha');
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
  const fixture = loadHouseholdMeshFixture();
  const primaryUrl = requireFixturePrimaryStorageUrl(fixture, 'alpha');
  const poolUrls = requireFixturePoolStorageUrls(fixture, 'alpha').filter(
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

/**
 * The degraded-primary shape these scenarios describe (a doorway's primary
 * storage answering ZERO project-epr rows while a pool peer holds them) used
 * to be an incident shape this lane could only OBSERVE, never construct —
 * elohim-storage had no verb that shaded a peer's project-epr rows, and
 * deleting them would race the 30 s projection reconcile that heals exactly
 * that gap. `POST {storage}/admin/projections/shade` (landing separately,
 * from the storage side) closes that gap: it hides rows of one `kind` from
 * the read route the doorway's EPR router refresh consults, WITHOUT
 * deleting them — nothing for the reconcile to race, nothing lost on
 * restore. Until that verb is deployed on the build under test, the route
 * answers 404 and the precondition steps below HOLD, naming the missing
 * verb rather than failing on a premise the substrate does not yet offer.
 */
const SHADE_ROUTE_PATH = '/admin/projections/shade';

function storageAdminApiKey(): string {
  return (
    process.env['STORAGE_API_KEY_ADMIN'] ?? process.env['API_KEY_ADMIN'] ?? 'mesh-admin-dev-key'
  );
}

interface ShadeRouteResult {
  ok: boolean;
  shaded: string[];
}

/**
 * Shade (or un-shade) one storage peer's rows of `kind`. Returns
 * `ok: false` ONLY for a 404 (the verb not deployed on this build) — any
 * other non-200 status is a real failure and throws, since a degrade
 * precondition must never silently pass on an unexplained error.
 */
async function callShadeRoute(
  storageUrl: string,
  kind: string,
  shaded: boolean
): Promise<ShadeRouteResult> {
  const response = await fetch(`${storageUrl}${SHADE_ROUTE_PATH}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'X-API-Key': storageAdminApiKey() },
    body: JSON.stringify({ kind, shaded }),
    signal: AbortSignal.timeout(10_000),
  });
  if (response.status === 404) return { ok: false, shaded: [] };
  const body = await response.text();
  assert.equal(
    response.status,
    200,
    `POST ${storageUrl}${SHADE_ROUTE_PATH} (shaded=${shaded}) -> ${response.status}: ${body}`
  );
  const parsed = JSON.parse(body) as { shaded?: string[] };
  return { ok: true, shaded: parsed.shaded ?? [] };
}

function holdMissingShadeVerb(storageUrl: string): 'skipped' {
  console.warn(
    `  ⏭️  HELD (missing verb): POST ${storageUrl}${SHADE_ROUTE_PATH} -> 404. The degraded-primary ` +
      "precondition needs elohim-storage's admin projections-shade verb to hide one peer's " +
      "project-epr rows without deleting them; it is not deployed on this build's storage " +
      'binary yet. Establish it out of band, or land the verb.'
  );
  return 'skipped';
}

/**
 * Storage peers this scenario shaded, so the After hook below can ALWAYS
 * un-shade them — pass or fail, HELD or not (a HELD precondition never adds
 * to this set, since it returns before calling `trackShaded`).
 */
const shadedStoragePeers = new WeakMap<E2EWorld, Set<string>>();

function trackShaded(world: E2EWorld, storageUrl: string): void {
  const set = shadedStoragePeers.get(world) ?? new Set<string>();
  set.add(storageUrl);
  shadedStoragePeers.set(world, set);
}

After({ timeout: 20_000 }, async function (this: E2EWorld) {
  const peers = shadedStoragePeers.get(this);
  if (!peers || peers.size === 0) return;
  shadedStoragePeers.delete(this);
  const failures: string[] = [];
  for (const storageUrl of peers) {
    try {
      await callShadeRoute(storageUrl, PROJECT_EPR_KIND, false);
    } catch (error) {
      failures.push(`${storageUrl}: ${String(error)}`);
    }
  }
  assert.equal(
    failures.length,
    0,
    `pool-degrade teardown failed to un-shade project-epr on: ${failures.join('; ')}`
  );
});

Given(
  "the doorway's primary storage returns zero {string} rows for its doorwayId",
  async function (this: E2EWorld, action: string) {
    assert.equal(action, PROJECT_EPR_KIND);
    const state = await initialPoolState(this);
    const shade = await callShadeRoute(state.primaryUrl, action, true);
    if (!shade.ok) return holdMissingShadeVerb(state.primaryUrl);
    trackShaded(this, state.primaryUrl);
    const rows = await projections(state.primaryUrl, state.doorwayId);
    assert.equal(rows.length, 0, `primary ${state.primaryUrl} still returns rows after shading`);
    return undefined;
  }
);

Given(
  "a configured pool peer holds the doorway's {string} rows",
  async function (this: E2EWorld, action: string) {
    assert.equal(action, PROJECT_EPR_KIND);
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
    assert.equal(action, PROJECT_EPR_KIND);
    const state = await initialPoolState(this);
    const targets = [state.primaryUrl, ...state.poolUrls];
    for (const storageUrl of targets) {
      const shade = await callShadeRoute(storageUrl, action, true);
      if (!shade.ok) return holdMissingShadeVerb(storageUrl);
      trackShaded(this, storageUrl);
    }
    for (const storageUrl of targets) {
      const rows = await projections(storageUrl, state.doorwayId);
      assert.equal(rows.length, 0, `${storageUrl} still returns rows after shading`);
    }
    state.expected = 'empty';
    return undefined;
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
  { timeout: 65_000 },
  async function (this: E2EWorld) {
    const apexUrl = registeredDoorwayUrl(this, 'apex');
    const before = await coherence(apexUrl);
    const primary = requireFixturePrimaryStorageUrl(loadHouseholdMeshFixture(), 'apex');
    const shade = await callShadeRoute(primary, PROJECT_EPR_KIND, true);
    if (!shade.ok) return holdMissingShadeVerb(primary);
    trackShaded(this, primary);
    const rows = await projections(primary, before.doorwayId);
    assert.equal(rows.length, 0, `apex primary ${primary} still returns rows after shading`);
    // No explicit "router refresh runs" step follows in this scenario's own
    // Gherkin (it goes straight to GETting the front door) — wait for the
    // apex router's own refresh cycle to have actually consulted the
    // now-shaded primary before returning, so the GET below measures the
    // degraded-but-served state rather than a stale pre-shade cache.
    await retry(
      async () => {
        const current = await coherence(apexUrl);
        assert.ok(
          current.generation > before.generation,
          `apex router generation has not advanced from ${before.generation} since shading`
        );
      },
      {
        maxAttempts: 60,
        initialDelayMs: 500,
        backoffFactor: 1.2,
        maxDelayMs: 2_000,
        timeoutMs: 55_000,
      }
    );
    return undefined;
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
  /** Each peer's filesystem blob count measured BEFORE the outage. */
  inventoryBaseline?: Map<HouseholdPeerName, number>;
}

const peerLossStates = new WeakMap<E2EWorld, PeerLossState>();

function householdState(world: E2EWorld): PeerLossState {
  const existing = peerLossStates.get(world);
  if (existing) return existing;

  const fixture = loadHouseholdMeshFixture();
  const names: HouseholdPeerName[] = ['matthew', 'jessica', 'james'];
  const peers = names.map(name => ({
    name,
    url: requireFixtureStoragePeer(fixture, name).url,
  }));
  const floor = fixture.connectedPeersFloor ?? peers.length - 1;
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
  return requireFixturePeerPid(loadHouseholdMeshFixture(), name);
}

// ---------------------------------------------------------------------------
// Destructive gate — SIGSTOPping a live mesh peer is an inducement on shared
// state, same class as the conductor-bounce legs in
// steps/mesh/peer-conductor-resilience.steps.ts, and rides the same gate
// (substrate-scope.ts destructiveAllowed: the lane's declared owned-substrate
// cap, or A2O_ALLOW_DESTRUCTIVE=1). A held step returns
// 'skipped' (never 'pending'), which skips the rest of its scenario.
// ---------------------------------------------------------------------------

function holdDestructive(wouldDo: string): 'skipped' {
  console.warn(
    `  ⏭️  HELD (destructive): ${wouldDo}. A shared mesh holds this leg by default — ` +
      `${DESTRUCTIVE_HELD_HINT}`
  );
  return 'skipped';
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

/**
 * The CID-first migration moved blobHash off the legacy `sha256-<64hex>`
 * shape to CIDv1 (base32 lower, `bafkrei…` = raw codec + sha2-256) for
 * freshly-ingested content — manifesto is one such row. See
 * elohim/elohim-storage/src/blob_store.rs (`Cid::new_v1(RAW_CODEC, ...)`).
 */
const CID_V1_RAW_SHA256 = /^bafkrei[a-z2-7]+$/;

async function loadManifesto(world: E2EWorld, state: PeerLossState): Promise<void> {
  const doorwayUrl = registeredDoorwayUrl(world, 'alpha');
  const content = await jsonGet<Record<string, unknown>>(doorwayUrl, '/db/content/manifesto');
  const hash = content['blobHash'];
  assert.equal(typeof hash, 'string', 'manifesto projection has no blobHash');
  assert.match(
    hash as string,
    CID_V1_RAW_SHA256,
    `manifesto blobHash "${hash}" does not look like a CID content address (expected bafkrei…)`
  );
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
  if (!destructiveAllowed()) {
    return holdDestructive("SIGSTOP jessica's live storage process");
  }
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
    if (!destructiveAllowed()) {
      return holdDestructive("SIGSTOP jessica's live storage process");
    }
    const state = await establishReplicatedManifesto(this);
    for (const peer of state.peers) await assertConnectedFloor(state, peer);
    state.inventoryBaseline = new Map(
      await Promise.all(
        state.peers.map(async peer => {
          const wire = await jsonGet<InventoryParityWire>(
            peer.url,
            '/api/v1/diagnostics/inventory-parity'
          );
          return [peer.name, normalizeInventoryParity(wire).filesystemCount] as const;
        })
      )
    );
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
        // "Parity" is each peer's gossip view agreeing with its own
        // filesystem — NOT every peer holding the same number of blobs. A
        // household's peers legitimately hold different sets (the seeding
        // peer carries the whole corpus; the others hold what custody
        // commitments placed on them — 3619 / 77 / 43 on the Act I mesh), so
        // an equal-counts assertion was red before the outage ever started.
        // Jessica's claim is: her view is clean again, she lost nothing she
        // held before going dark, and the survivors' views are clean too.
        const baseline = state.inventoryBaseline;
        assert.ok(baseline, 'inventory baseline was not measured before the outage');
        for (const [index, peer] of state.peers.entries()) {
          const report = reports[index];
          assert.ok(report);
          assert.deepEqual(
            report.gossipedButMissing,
            [],
            `${peer.name} gossips blobs it does not hold: ${report.gossipedButMissing.join(', ')}`
          );
          assert.deepEqual(
            report.localButNotGossiped,
            [],
            `${peer.name} holds blobs it has not gossiped: ${report.localButNotGossiped.join(', ')}`
          );
          assert.equal(
            report.filesystemCount,
            report.gossipedCount,
            `${peer.name} filesystem/gossip counts differ`
          );
          const before = baseline.get(peer.name) ?? 0;
          assert.ok(
            report.filesystemCount >= before,
            `${peer.name} holds ${report.filesystemCount} blobs, fewer than the ${before} it ` +
              'held before the outage'
          );
        }
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
  if (!destructiveAllowed()) {
    return holdDestructive("SIGSTOP matthew's and james's live storage processes");
  }
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

/**
 * How long a silent peer may stay in Jessica's connected set. A SIGSTOP'd
 * peer sends no FIN, so the only thing that removes it is a failed libp2p
 * ping: elohim-storage closes the connection on the first failure, and with
 * the default cadence (interval 15 s, timeout 20 s — `P2P_PING_INTERVAL_SECS`
 * / `P2P_PING_TIMEOUT_SECS`) the worst case is ~35 s after the pause. The
 * window holds that plus slack; `E2E_PEER_DEGRADE_WINDOW_MS` overrides it.
 */
const PEER_DEGRADE_WINDOW_MS = Number(process.env['E2E_PEER_DEGRADE_WINDOW_MS'] ?? 60_000);

Then(
  'the degraded mesh state is visible in her peer status, not hidden',
  { timeout: PEER_DEGRADE_WINDOW_MS + 15_000 },
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
        maxAttempts: 60,
        initialDelayMs: 500,
        backoffFactor: 1.2,
        maxDelayMs: 2_000,
        timeoutMs: PEER_DEGRADE_WINDOW_MS,
      }
    );
    assert.ok(status.connectedPeers < state.floor);
  }
);
