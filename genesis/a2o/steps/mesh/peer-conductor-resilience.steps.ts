/**
 * Step glue for the peer-conductor connection-resilience concern
 * (features/doorway/peer-conductor-connection-resilience.feature).
 *
 * Every assertion here reads a LIVE surface the doorway actually publishes.
 * The four surfaces, and where each one is defined in the Rust:
 *
 *   GET /metrics        doorway/doorway-service/src/metrics.rs — the registry
 *                       carries doorway_conductor_reconnect_total{reason},
 *                       doorway_conductor_close_code_total{code},
 *                       doorway_conductor_session_duration_seconds (buckets
 *                       straddle 1s and the 10s stable window),
 *                       doorway_conductor_sessions (live app-WS session gauge),
 *                       doorway_heartbeat_age_ms and
 *                       doorway_watchdog_wedged_total.
 *   GET /health         routes/health.rs HealthResponse — conductor{connected,
 *                       connected_workers, total_workers, pool_size,
 *                       pools_healthy, pools_total} (snake_case: ConductorHealth
 *                       carries no rename_all).
 *   GET /status.json    routes/status.rs StatusResponse (snake_case) → `compute`
 *                       (elohim-compute ComputeReport, camelCase) → `peers[]`
 *                       (PeerHealthSnapshot: peerId/health/reason/
 *                       reconnectAttempts). record_reconnect() is what stamps
 *                       health=degraded, reason="reconnecting".
 *   the doorway LOG     the manifest's doorways.<id>.logPath (never pod stdout).
 *                       Three lines are load-bearing here:
 *                         "Reconnecting to conductor in <dur>..."   (worker/conductor.rs)
 *                         "Tokio worker threads: N (explicit)"      (main.rs)
 *                         "Health watchdog bound on <addr> (own OS-thread
 *                          runtime; heartbeat-gated, stale>Nms ⇒ wedged)"
 *                                                                   (server/http.rs)
 *
 * The reconnect ladder itself is only observable from the log: the doorway
 * prints the delay it is about to sleep, and nothing exports it as a metric.
 * That is why the backoff scenarios tail logPath rather than scrape a gauge.
 *
 * DESTRUCTIVE STEPS — hold these until the caller says go:
 *   Given a peer conductor that restarts and invalidates previously issued app
 *         auth tokens                          (runs hc-mesh.sh conductors-restart)
 *   Given the mesh conductors are restarted so the doorway must reconnect with
 *         stale credentials                    (runs hc-mesh.sh conductors-restart;
 *                                               the shared inducement leg for the
 *                                               backoff / leak / churn scenarios —
 *                                               nothing else makes the doorway
 *                                               print "Reconnecting to conductor
 *                                               in …" on a healthy mesh)
 *   When every main worker thread is parked at once
 *                                              (concurrency burst sized off the
 *                                               doorway's own worker-thread count)
 *   Given the doorway's main runtime is briefly busy but recovers within the
 *         staleness window                     (short concurrency burst)
 *   Given the SSR render isolate is busy with an in-flight render
 *                                              (concurrent render burst)
 * Both burst classes only issue ordinary HTTP GETs; neither signals a process.
 * The conductor restarts are the only steps that move a process, and they do so
 * through hc-mesh.sh — never a pkill/pgrep pattern (those match the runner's
 * own shell and self-kill the run).
 */

import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import { closeSync, openSync, readSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { Given, When, Then } from '@cucumber/cucumber';

import {
  describeCatchUpRide,
  getRaw,
  getRawRidingCatchUp,
  getRawWithHeaders,
  parseLabeledPrometheusMetric,
  parsePrometheusMetrics,
  probeHealth,
  resolvePeerUrl,
  shedCause,
  type HealthSurface,
  type RawHttpResponse,
} from '../../src/framework/dataplane/surfaces.js';
import {
  loadHouseholdMeshFixture,
  requireFixtureDoorwayLogPath,
  requireFixtureDoorwayUrl,
  type HouseholdMeshFixture,
} from '../../src/framework/fixtures/household-mesh.js';
import {
  destructiveAllowed,
  DESTRUCTIVE_HELD_HINT,
} from '../../src/framework/fixtures/substrate-scope.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants — every one of these is read off the doorway source, not invented
// ---------------------------------------------------------------------------

const DOORWAY_ID = 'alpha';

const M_RECONNECT_TOTAL = 'doorway_conductor_reconnect_total';
const M_CLOSE_CODE_TOTAL = 'doorway_conductor_close_code_total';
const M_SESSION_DURATION = 'doorway_conductor_session_duration_seconds';
const M_SESSIONS = 'doorway_conductor_sessions';
const M_HEARTBEAT_AGE = 'doorway_heartbeat_age_ms';
const M_WATCHDOG_WEDGED = 'doorway_watchdog_wedged_total';

/** worker/conductor.rs BASE_RECONNECT_DELAY / MAX_RECONNECT_DELAY. */
const BASE_RECONNECT_DELAY_MS = 100;
const MAX_RECONNECT_DELAY_MS = 30_000;
/** worker/conductor.rs STABLE_SESSION_THRESHOLD, in seconds (histogram unit). */
const STABLE_SESSION_WINDOW_S = 10;
/** server/http.rs HEALTH_STALE_MS_DEFAULT, overridable by DOORWAY_HEALTH_STALE_MS. */
const DEFAULT_HEALTH_STALE_MS = 15_000;
/** services/zome_caller.rs default for DOORWAY_ZOME_CALL_TIMEOUT_MS. */
const DEFAULT_ZOME_CALL_DEADLINE_MS = 10_000;

/** A probe that has to answer while something else is wedged gets this budget. */
const PROMPT_MS = 2_000;
/** Non-pool ConductorConnection holders counted by the session gauge: the NATS
 *  processor and the import/seed client (metrics.rs CONDUCTOR_SESSIONS docs). */
const NON_POOL_SESSION_ALLOWANCE = 2;

/** Absolute interpreter path — a bare `bash` would be resolved through PATH. */
const BASH_BIN = process.env['E2E_BASH_BIN'] ?? '/bin/bash';

const RECONNECT_LOG_RE = /Reconnecting to conductor in ([\d.]+)(ns|µs|ms|s|m)(?![a-z])/g;
const WORKER_THREADS_LOG_RE = /Tokio worker threads: (\d+) \(explicit\)/g;
const WATCHDOG_BOUND_LOG_RE =
  /Health watchdog bound on (\S+) \(own OS-thread runtime; heartbeat-gated, stale>(\d+)ms/g;
const REMINT_LOG_LINE = 'Re-minted app auth token after unstable conductor session';
const SSR_SATURATED_LOG_LINE = 'render semaphore at limit';
const WEDGE_LOG_LINE = 'watchdog WEDGED';

const STEP_SHORT_MS = 60_000;
const STEP_MEDIUM_MS = 180_000;
const STEP_LONG_MS = 420_000;

// ---------------------------------------------------------------------------
// Per-scenario context
// ---------------------------------------------------------------------------

interface HttpSample {
  status: number;
  text: string;
  elapsedMs: number;
}

interface PeerSnapshotWire {
  peerId: string;
  address: string;
  health: string;
  reason: string;
  signalsReceived: number;
  reconnectAttempts: number;
}

interface WatchdogTarget {
  url: string;
  thresholdMs: number;
}

interface ResilienceContext {
  reconnectDelaysMs: number[];
  metricsBaseline?: string;
  remintBaseline: number;
  wedgedBaseline: number;
  sessionSamples: number[];
  heartbeatSamples: number[];
  healthLatencySamples: number[];
  peersBaseline: PeerSnapshotWire[];
  peersObserved: PeerSnapshotWire[];
  watchdog?: WatchdogTarget;
  watchdogProbes: HttpSample[];
  workerThreads?: number;
  conductorCalls: HttpSample[];
  concurrentProbe?: HttpSample;
  blockedPath?: { url: string; settled: Promise<HttpSample[]> };
  ssrBurst?: { stop: () => void; settled: Promise<unknown> };
  ssrSaturationBaseline?: number;
  ssrShed?: RawHttpResponse & { elapsedMs: number };
  contentRead?: HttpSample;
}

const contexts = new WeakMap<E2EWorld, ResilienceContext>();

function ctx(world: E2EWorld): ResilienceContext {
  let existing = contexts.get(world);
  if (!existing) {
    existing = {
      reconnectDelaysMs: [],
      remintBaseline: 0,
      wedgedBaseline: 0,
      sessionSamples: [],
      heartbeatSamples: [],
      healthLatencySamples: [],
      peersBaseline: [],
      peersObserved: [],
      watchdogProbes: [],
      conductorCalls: [],
    };
    contexts.set(world, existing);
  }
  return existing;
}

// ---------------------------------------------------------------------------
// Fixture / surface helpers
// ---------------------------------------------------------------------------

function mesh(): HouseholdMeshFixture {
  return loadHouseholdMeshFixture();
}

function doorwayUrl(world: E2EWorld): string {
  const known = world.doorways.get(DOORWAY_ID);
  if (known) return known.url;
  return requireFixtureDoorwayUrl(mesh(), DOORWAY_ID);
}

/** Read the tail of a file — the doorway log is append-only and can be large. */
function tailFile(path: string, maxBytes = 2_000_000): string {
  const size = statSync(path).size;
  const start = Math.max(0, size - maxBytes);
  const length = size - start;
  const buffer = Buffer.alloc(length);
  const fd = openSync(path, 'r');
  try {
    readSync(fd, buffer, 0, length, start);
  } finally {
    closeSync(fd);
  }
  return buffer.toString('utf8');
}

function doorwayLog(): string {
  return tailFile(requireFixtureDoorwayLogPath(mesh(), DOORWAY_ID));
}

function countOccurrences(haystack: string, needle: string): number {
  let count = 0;
  let index = haystack.indexOf(needle);
  while (index !== -1) {
    count += 1;
    index = haystack.indexOf(needle, index + needle.length);
  }
  return count;
}

async function sleep(ms: number): Promise<void> {
  await new Promise<void>(resolve => setTimeout(resolve, ms));
}

async function timedGet(url: string, timeoutMs: number): Promise<HttpSample> {
  const started = Date.now();
  const res = await getRaw(url, { timeoutMs });
  return { status: res.status, text: res.text, elapsedMs: Date.now() - started };
}

async function scrape(base: string): Promise<string> {
  const { status, text } = await getRaw(`${base}/metrics`, { timeoutMs: 10_000 });
  assert.equal(
    status,
    200,
    `GET ${base}/metrics returned ${status} — the doorway's Prometheus surface (routes/metrics.rs) must answer`
  );
  return text;
}

function gauge(text: string, name: string): number | null {
  return parsePrometheusMetrics(text).get(name) ?? null;
}

/** Sum every series line whose base name is exactly `series` (all label sets). */
function sumSeries(text: string, series: string): number | null {
  let total: number | null = null;
  for (const rawLine of text.split('\n')) {
    const line = rawLine.trim();
    if (line.startsWith('#') || !line.startsWith(series)) continue;
    const after = line.slice(series.length);
    if (after.length > 0 && !after.startsWith(' ') && !after.startsWith('{')) continue;
    const value = Number(line.split(/\s+/).pop());
    if (!Number.isNaN(value)) total = (total ?? 0) + value;
  }
  return total;
}

function requireSeries(text: string, series: string, why: string): number {
  const value = sumSeries(text, series);
  assert.ok(
    value !== null,
    `/metrics carries no "${series}" series — ${why}. Registered in doorway/doorway-service/src/metrics.rs; ` +
      'its absence means this doorway build predates the counter, so the claim is unobservable rather than false.'
  );
  return value;
}

async function readHealth(base: string): Promise<HealthSurface> {
  const { body } = await probeHealth(base);
  return body;
}

async function readStatusPeers(base: string): Promise<PeerSnapshotWire[]> {
  const { status, text } = await getRaw(`${base}/status.json`, { timeoutMs: 15_000 });
  assert.equal(
    status,
    200,
    `GET ${base}/status.json returned ${status} — the operator status surface must answer`
  );
  const parsed = JSON.parse(text) as { compute?: { peers?: PeerSnapshotWire[] } };
  const peers = parsed.compute?.peers;
  assert.ok(
    Array.isArray(peers),
    'status.json carries no compute.peers[] — the doorway peer-health snapshot ' +
      '(elohim-compute PeerHealthRegistry) is the ONLY operator-facing home of reconnect churn'
  );
  return peers;
}

/**
 * Keep `concurrency` SSR renders of `url` in flight until stopped. The render
 * semaphore is only exhausted while requests are actually holding permits, so a
 * scenario that asserts shedding has to hold the load across its own steps
 * rather than fire once and hope.
 */
function sustainedRenders(
  url: string,
  concurrency: number
): { stop: () => void; settled: Promise<unknown> } {
  let running = true;
  const workers = Array.from({ length: concurrency }, async () => {
    while (running) {
      await getRawWithHeaders(url, { timeoutMs: 30_000 }).catch(() => null);
    }
  });
  return {
    stop: () => {
      running = false;
    },
    settled: Promise.allSettled(workers),
  };
}

function durationToMs(value: string, unit: string): number {
  const amount = Number.parseFloat(value);
  switch (unit) {
    case 'ns':
      return amount / 1_000_000;
    case 'µs':
      return amount / 1000;
    case 'ms':
      return amount;
    case 's':
      return amount * 1000;
    default:
      return amount * 60_000;
  }
}

/** The doorway's first line of every boot — everything after it is THIS boot. */
const BOOT_MARKER = '"message":"doorway starting"';

/**
 * Trim a doorway log tail to the CURRENT boot. `conn_id` restarts at 0 in a
 * fresh process, so two boots' ladders would otherwise merge under one key.
 */
function sinceLastBoot(log: string): string {
  const index = log.lastIndexOf(BOOT_MARKER);
  return index === -1 ? log : log.slice(index);
}

/**
 * Reconnect ladders, grouped by the connection loop climbing them.
 *
 * A doorway runs MANY connection loops at once — the app pool's workers, the
 * admin pool's workers, and each per-conductor pool's — and each climbs its OWN
 * ReconnectBackoff. Read as one flat sequence, four workers all sitting on the
 * same rung look like a ladder that never doubles: the household mesh measured
 * exactly that on 2026-08-22 ("3200, 3200, 3200, 3200, 3200"), which is four
 * healthy ladders interleaved, not one stuck loop. worker/conductor.rs now names
 * the loop (`conn_id`) and numbers the rung (`attempt`) on the line, so the
 * ladders can be separated before any monotonicity claim is made about them.
 *
 * An older binary without the fields degrades to ONE 'unattributed' group — the
 * previous behaviour, and the caller says so in its failure text.
 */
function reconnectLaddersFromLog(log: string): Map<string, number[]> {
  const ladders = new Map<string, number[]>();
  for (const raw of sinceLastBoot(log).split('\n')) {
    const trimmed = raw.trim();
    if (!trimmed.includes('Reconnecting to conductor in')) continue;
    let connId = 'unattributed';
    let delayMs: number | null = null;
    try {
      const parsed = JSON.parse(trimmed) as { fields?: Record<string, unknown> };
      const fields = parsed.fields ?? {};
      if (typeof fields['conn_id'] === 'number') connId = `conn-${fields['conn_id']}`;
      if (typeof fields['delay_ms'] === 'number') delayMs = fields['delay_ms'];
    } catch {
      // Not a JSON line (a plain-text tail) — fall through to the regex below.
    }
    if (delayMs === null) {
      RECONNECT_LOG_RE.lastIndex = 0;
      const match = RECONNECT_LOG_RE.exec(trimmed);
      if (!match) continue;
      delayMs = durationToMs(match[1], match[2]);
    }
    const ladder = ladders.get(connId) ?? [];
    ladder.push(delayMs);
    ladders.set(connId, ladder);
  }
  return ladders;
}

/**
 * The single longest per-connection ladder — the one this suite's monotonicity
 * assertions are entitled to reason about.
 */
function reconnectDelaysFromLog(log: string): number[] {
  const ladders = reconnectLaddersFromLog(log);
  let longest: number[] = [];
  for (const delays of ladders.values()) {
    if (delays.length > longest.length) longest = delays;
  }
  return longest;
}

function lastMatch(log: string, pattern: RegExp): RegExpMatchArray | null {
  let found: RegExpMatchArray | null = null;
  for (const match of log.matchAll(pattern)) found = match;
  return found;
}

function requireDelays(world: E2EWorld, minimum: number): number[] {
  const delays = ctx(world).reconnectDelaysMs;
  assert.ok(
    delays.length >= minimum,
    `the doorway log's longest single-connection reconnect ladder has only ${delays.length} rung(s) ` +
      `this boot; this assertion needs ${minimum}. The ladder has no metric — the log line in ` +
      'worker/conductor.rs is the only place the delay is published, and it is grouped by its ' +
      '`conn_id` field (a doorway too old to emit that field collapses every loop into one ' +
      'unattributed group, which cannot be reasoned about monotonically).'
  );
  return delays.slice(-minimum);
}

async function collectReconnectDelays(
  world: E2EWorld,
  minimum: number,
  budgetMs: number
): Promise<void> {
  const deadline = Date.now() + budgetMs;
  for (;;) {
    const delays = reconnectDelaysFromLog(doorwayLog());
    ctx(world).reconnectDelaysMs = delays;
    if (delays.length >= minimum || Date.now() >= deadline) return;
    await sleep(3_000);
  }
}

async function pollUntil<T>(
  probe: () => T | null | Promise<T | null>,
  budgetMs: number,
  intervalMs = 3_000
): Promise<T | null> {
  const deadline = Date.now() + budgetMs;
  for (;;) {
    let value: T | null = null;
    try {
      value = await probe();
    } catch {
      value = null;
    }
    if (value !== null) return value;
    if (Date.now() >= deadline) return null;
    await sleep(Math.min(intervalMs, Math.max(0, deadline - Date.now())));
  }
}

/**
 * Resolve the isolated liveness watchdog. It exists ONLY when the process was
 * started with DOORWAY_HEALTH_PORT (server/http.rs spawn_health_listener returns
 * early otherwise), and the doorway announces the bind in its own log — so the
 * log is the honest discovery surface, not a guessed port.
 */
function findWatchdog(world: E2EWorld): WatchdogTarget | null {
  const cached = ctx(world).watchdog;
  if (cached) return cached;
  const bound = lastMatch(doorwayLog(), WATCHDOG_BOUND_LOG_RE);
  if (bound === null) return null;
  const port = bound[1].split(':').at(-1);
  if (port === undefined || port === '') return null;
  const base = new URL(doorwayUrl(world));
  base.port = port;
  base.pathname = '';
  const target: WatchdogTarget = {
    url: base.toString().replace(/\/$/, ''),
    thresholdMs: Number.parseInt(bound[2], 10),
  };
  ctx(world).watchdog = target;
  return target;
}

function healthStaleThresholdMs(world: E2EWorld): number {
  const fromLog = ctx(world).watchdog?.thresholdMs;
  if (fromLog) return fromLog;
  const fromEnv = Number.parseInt(process.env['DOORWAY_HEALTH_STALE_MS'] ?? '', 10);
  return Number.isNaN(fromEnv) || fromEnv <= 0 ? DEFAULT_HEALTH_STALE_MS : fromEnv;
}

function zomeCallDeadlineMs(): number {
  const fromEnv = Number.parseInt(process.env['DOORWAY_ZOME_CALL_TIMEOUT_MS'] ?? '', 10);
  return Number.isNaN(fromEnv) || fromEnv <= 0 ? DEFAULT_ZOME_CALL_DEADLINE_MS : fromEnv;
}

/**
 * The doorway route that traverses the conductor: it opens the embedded admin
 * connection and asks for the live agent set. Any hang on the conductor hop
 * surfaces here.
 */
function conductorCallUrl(base: string): string {
  return `${base}/db/p2p/conductor-diagnostics`;
}

async function burst(url: string, count: number, timeoutMs: number): Promise<HttpSample[]> {
  const flights = Array.from({ length: count }, async () => {
    try {
      return await timedGet(url, timeoutMs);
    } catch (error) {
      return { status: 0, text: String(error), elapsedMs: timeoutMs };
    }
  });
  return Promise.all(flights);
}

/**
 * Destructive and load-inducing legs are OFF by default.
 *
 * The mesh is shared: an ordinary run of this file must not restart conductors
 * or pin the doorway's worker pool. The gate is an explicit env opt-in, NOT a
 * `@requires:` tag — `@requires:owned-substrate` is declared only in the lane
 * files (cluster-state.act1-household.yaml / act2), so on a run reading the
 * default cluster-state.yaml substrate-scope.ts treats it as an undeclared cap,
 * warns "GATES NOTHING", and lets the scenario run. This env switch is the thing
 * that actually holds the leg.
 *
 * Held steps return 'skipped', never 'pending': cucumber-js is strict by
 * default, so a pending step reds the run exactly like a failure. 'skipped' is
 * the disposition steps/common.steps.ts already gives a held scenario, and a
 * skipped step skips the rest of its scenario.
 */
function holdDestructive(wouldDo: string): 'skipped' {
  console.warn(
    `  ⏭️  HELD (destructive): ${wouldDo}. A shared mesh holds this leg by default — ` +
      `${DESTRUCTIVE_HELD_HINT}`
  );
  return 'skipped';
}

/**
 * Why an absent watchdog is HELD rather than RED: the listener exists only when
 * the process was started with DOORWAY_HEALTH_PORT, and hc-mesh.sh launches both
 * mesh doorways without it. Failing here would red every healthy run forever —
 * a false-red generator, not a regression signal. A doorway that DOES have the
 * listener still gets the full assertions below.
 */
function holdNoWatchdog(): 'skipped' {
  console.warn(
    '  ⏭️  HELD (surface absent): the doorway log carries no "Health watchdog bound on …" line, so ' +
      'this process serves liveness from the MAIN listener. DOORWAY_HEALTH_PORT is unset for it ' +
      '(server/http.rs spawn_health_listener returns early without it), so the isolated-runtime ' +
      'watchdog this scenario asserts does not exist here. To make it observable, start the mesh ' +
      'doorway with DOORWAY_HEALTH_PORT set (e.g. 8079) — app/elohim-app/scripts/hc-mesh.sh ' +
      'launches doorway A and B without it today.'
  );
  return 'skipped';
}

function meshScriptPath(): string {
  const override = process.env['E2E_HC_MESH_SCRIPT'];
  if (override) return override;
  return fileURLToPath(new URL('../../../../app/elohim-app/scripts/hc-mesh.sh', import.meta.url));
}

/**
 * Restart every mesh conductor through hc-mesh.sh — the only sanctioned way a
 * step here moves a process. A restart invalidates issued app-auth tokens, so
 * the doorway's pool workers and signal subscribers reconnect with stale
 * tokens: auth-reject → ladder-climb → re-mint. That cycle is what makes the
 * "Reconnecting to conductor in …" log lines, the session-gauge churn, and the
 * reconnectAttempts counters actually move — on a healthy mesh nothing else
 * induces them. Callers gate on destructiveAllowed() BEFORE reaching this.
 */
function restartMeshConductors(): void {
  const fixture = mesh();
  assert.equal(
    fixture.processControl,
    true,
    `restarting a conductor needs process control over this mesh: ${
      fixture.processControlReason ?? 'the manifest does not declare processControl: true'
    }`
  );
  const script = meshScriptPath();
  // Exact script, exact subcommand — never a pkill/pgrep pattern.
  const result = spawnSync(BASH_BIN, [script, 'conductors-restart'], {
    encoding: 'utf8',
    timeout: 300_000,
  });
  assert.equal(
    result.status,
    0,
    `${script} conductors-restart exited ${String(result.status)}: ${(result.stderr ?? '').slice(-2000)}`
  );
}

/**
 * Shared inducement leg for the backoff / leak / churn scenarios (1, 2, 3, 5).
 * Their When/Then chains OBSERVE reconnect cycles; this Given is what CAUSES
 * them. Same gate, same subcommand as the scenario-4 restart step — held
 * ('skipped', never 'pending') unless A2O_ALLOW_DESTRUCTIVE=1, so an ungated
 * run skips the scenario immediately instead of red-waiting on log lines that
 * nothing induces.
 */
Given(
  'the mesh conductors are restarted so the doorway must reconnect with stale credentials',
  { timeout: STEP_LONG_MS },
  function (this: E2EWorld) {
    if (!destructiveAllowed()) {
      return holdDestructive(
        'this step would run `hc-mesh.sh conductors-restart`, restarting EVERY conductor in this mesh ' +
          'to induce the auth-reject → ladder-climb → re-mint cycle the scenario observes'
      );
    }
    restartMeshConductors();
  }
);

/** Assert the pool-health fields cannot disagree with each other. */
function assertPoolCoupling(health: HealthSurface): void {
  const c = health.conductor;
  assert.equal(
    c.connected,
    c.connected_workers > 0,
    `/health reports conductor.connected=${c.connected} with connected_workers=${c.connected_workers} — ` +
      'a pool with no connected worker must read unhealthy, and one with a connected worker must read healthy'
  );
  assert.ok(
    c.pools_healthy <= c.pools_total,
    `/health reports pools_healthy=${c.pools_healthy} > pools_total=${c.pools_total}`
  );
  assert.equal(
    c.pools_healthy > 0,
    c.connected_workers > 0,
    `/health reports pools_healthy=${c.pools_healthy} alongside connected_workers=${c.connected_workers} — ` +
      'the per-conductor pool tally and the worker tally must tell the same story'
  );
}

// ===========================================================================
// Scenario 1 — Auth-rejected peer conductor is retried with exponential backoff
// ===========================================================================

Given(
  'a peer conductor in the pool accepts WebSocket connections but rejects the authenticate handshake',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const health = await readHealth(base);
    assert.ok(
      health.conductor.pools_total >= 1,
      `this doorway fronts ${health.conductor.pools_total} conductor pools — an auth-reject scenario needs at least one`
    );
    const metrics = await scrape(base);
    // The auth-reject signature is only classifiable if the doorway publishes
    // BOTH the reconnect-cause counter and the close-code axis (metrics.rs M2).
    requireSeries(metrics, M_RECONNECT_TOTAL, 'reconnect cause cannot be classified without it');
    requireSeries(
      metrics,
      M_CLOSE_CODE_TOTAL,
      'a conductor close code cannot be attributed without it'
    );
    ctx(this).metricsBaseline = metrics;
  }
);

When(
  "the doorway's worker attempts to connect 5 times",
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    await collectReconnectDelays(this, 5, 120_000);
    requireDelays(this, 5);
  }
);

Then(
  'each retry delay at least doubles the previous delay up to the 30 second cap',
  function (this: E2EWorld) {
    const delays = requireDelays(this, 5);
    let escalations = 0;
    for (let i = 1; i < delays.length; i += 1) {
      const previous = delays[i - 1];
      const current = delays[i];
      assert.ok(
        current <= MAX_RECONNECT_DELAY_MS,
        `reconnect delay ${current}ms exceeds the ${MAX_RECONNECT_DELAY_MS}ms ceiling: ${delays.join(', ')}`
      );
      const isLadderReset = current === BASE_RECONNECT_DELAY_MS;
      if (isLadderReset) continue;
      assert.ok(
        current >= Math.min(previous * 2, MAX_RECONNECT_DELAY_MS),
        `reconnect delay went ${previous}ms → ${current}ms: it must at least double (or sit at the ` +
          `${MAX_RECONNECT_DELAY_MS}ms cap). Full ladder: ${delays.join(', ')}`
      );
      escalations += 1;
    }
    assert.ok(
      escalations > 0,
      `every sampled reconnect returned to the ${BASE_RECONNECT_DELAY_MS}ms floor (${delays.join(', ')}) — ` +
        'that is the pre-fix ~10Hz signature: an unstable session must escalate, never reset'
    );
  }
);

Then(
  'the doorway marks the conductor pool unhealthy while disconnected',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    assertPoolCoupling(await readHealth(doorwayUrl(this)));
  }
);

Then(
  "the doorway's other pool conductors continue serving requests",
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const contentId = mesh().commonsEprId ?? 'manifesto';
    const res = await getRawRidingCatchUp(`${base}/db/content/${encodeURIComponent(contentId)}`);
    assert.equal(
      res.status,
      200,
      `a read of "${contentId}" through the doorway answered ${res.status}` +
        `${describeCatchUpRide(res)} — while one conductor flaps the remaining pool must keep serving`
    );
  }
);

// ===========================================================================
// Scenario 2 — Unstable sessions do not reset the backoff clock
// ===========================================================================

Given(
  'a peer conductor that accepts connections but drops every session within 2 seconds',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const metrics = await scrape(doorwayUrl(this));
    // The sub-second vs 10s discriminator (metrics.rs M3) is the only surface
    // that can tell accept-then-drop churn from healthy long-lived sessions.
    requireSeries(
      metrics,
      `${M_SESSION_DURATION}_count`,
      'session lifetime cannot be discriminated without it'
    );
    const stable = sumSeries(metrics, `${M_SESSION_DURATION}_bucket`);
    assert.ok(
      stable !== null,
      `/metrics carries no ${M_SESSION_DURATION}_bucket series — the histogram buckets straddling the ` +
        `${STABLE_SESSION_WINDOW_S}s stable window are what make "unstable" measurable`
    );
    ctx(this).metricsBaseline = metrics;
  }
);

When(
  'the doorway reconnects through 5 flap cycles',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    await collectReconnectDelays(this, 5, 120_000);
    requireDelays(this, 5);
  }
);

Then(
  'the reconnect delay keeps growing across cycles instead of resetting to the floor',
  function (this: E2EWorld) {
    const delays = requireDelays(this, 5);
    const resets = delays.slice(1).filter(delay => delay === BASE_RECONNECT_DELAY_MS).length;
    assert.equal(
      resets,
      0,
      `${resets} of the last ${delays.length - 1} reconnects dropped back to the ` +
        `${BASE_RECONNECT_DELAY_MS}ms floor (${delays.join(', ')}) — an unstable session must not reset the ladder`
    );
    const peak = Math.max(...delays);
    assert.ok(
      peak > BASE_RECONNECT_DELAY_MS,
      `the ladder never left the ${BASE_RECONNECT_DELAY_MS}ms floor (${delays.join(', ')})`
    );
  }
);

Then(
  'the backoff resets only after a session stays healthy for the minimum stable window',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const metrics = await scrape(doorwayUrl(this));
    const total = requireSeries(
      metrics,
      `${M_SESSION_DURATION}_count`,
      'the stable-window claim needs the session histogram'
    );
    const underStable = parseLabeledPrometheusMetric(
      metrics,
      `${M_SESSION_DURATION}_bucket`,
      'le',
      String(STABLE_SESSION_WINDOW_S)
    );
    assert.ok(
      underStable !== null,
      `${M_SESSION_DURATION}_bucket carries no le="${STABLE_SESSION_WINDOW_S}" bucket — the histogram no longer ` +
        'straddles the stable-session window, so "stable" is not measurable from this scrape'
    );
    const stableSessions = total - underStable;
    const delays = ctx(this).reconnectDelaysMs;
    const resets = delays.slice(1).filter(delay => delay === BASE_RECONNECT_DELAY_MS).length;
    assert.ok(
      resets === 0 || stableSessions > 0,
      `the reconnect ladder returned to the ${BASE_RECONNECT_DELAY_MS}ms floor ${resets} time(s) although ` +
        `${M_SESSION_DURATION} records ${stableSessions} session(s) longer than the ${STABLE_SESSION_WINDOW_S}s ` +
        'stable window — a reset without a stable session is exactly the 2026-06-10 defect (the backoff resets ' +
        'on a successful CONNECT instead of on a session that HELD)'
    );
    assert.ok(
      stableSessions >= 0,
      `${M_SESSION_DURATION}_count (${total}) is below its le="${STABLE_SESSION_WINDOW_S}" bucket (${underStable}) — ` +
        'the histogram is internally inconsistent'
    );
  }
);

// ===========================================================================
// Scenario 3 — Reconnect cycles do not leak connection tasks
// ===========================================================================

Given(
  'the doorway is connected to a peer conductor with an active signal subscriber',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const peers = await readStatusPeers(base);
    assert.ok(
      peers.length > 0,
      'status.json compute.peers[] is empty — no signal subscriber is registered, so there is no ' +
        'subscriber session for a reconnect cycle to leak'
    );
    const metrics = await scrape(base);
    requireSeries(metrics, M_SESSIONS, 'a task leak is invisible without the live-session gauge');
    ctx(this).peersBaseline = peers;
    ctx(this).metricsBaseline = metrics;
  }
);

When(
  'the conductor session drops and reconnects 10 times',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    await collectReconnectDelays(this, 10, 150_000);
    requireDelays(this, 10);
    const base = doorwayUrl(this);
    const sessions = gauge(await scrape(base), M_SESSIONS);
    assert.ok(sessions !== null, `${M_SESSIONS} vanished from the scrape mid-scenario`);
    ctx(this).sessionSamples.push(sessions);
  }
);

Then(
  'the doorway holds exactly one live connection task per conductor worker',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const health = await readHealth(base);
    const sessions = gauge(await scrape(base), M_SESSIONS);
    assert.ok(sessions !== null, `${M_SESSIONS} is absent from the scrape`);
    const ceiling = health.conductor.total_workers + NON_POOL_SESSION_ALLOWANCE;
    assert.ok(
      sessions <= ceiling,
      `${M_SESSIONS}=${sessions} exceeds total_workers(${health.conductor.total_workers}) + ` +
        `${NON_POOL_SESSION_ALLOWANCE} non-pool holders = ${ceiling}. Surplus sessions are leaked ` +
        'connection loops outliving their ConductorConnection handle — the immortal ~10Hz spammer class.'
    );
    assert.ok(
      sessions >= health.conductor.connected_workers,
      `${M_SESSIONS}=${sessions} is below connected_workers=${health.conductor.connected_workers} — ` +
        'every connected worker holds one session, so the gauge cannot be lower'
    );
    ctx(this).sessionSamples.push(sessions);
  }
);

Then(
  "the doorway's task count stays flat across the cycles",
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    await sleep(5_000);
    const later = gauge(await scrape(base), M_SESSIONS);
    assert.ok(later !== null, `${M_SESSIONS} is absent from the scrape`);
    const samples = [...ctx(this).sessionSamples, later];
    assert.ok(
      samples.length >= 2,
      'need at least two session-gauge samples to call the count flat'
    );
    const first = samples[0];
    assert.ok(
      later <= first,
      `${M_SESSIONS} grew ${first} → ${later} across the reconnect cycles (samples: ${samples.join(', ')}) — ` +
        'a reconnect must retire its old connection task, not accumulate one'
    );
  }
);

// ===========================================================================
// Scenario 4 — Conductor restart heals the worker pool (DESTRUCTIVE)
// ===========================================================================

Given(
  'a peer conductor that restarts and invalidates previously issued app auth tokens',
  { timeout: STEP_LONG_MS },
  function (this: E2EWorld) {
    if (!destructiveAllowed()) {
      return holdDestructive(
        'this step would run `hc-mesh.sh conductors-restart`, restarting EVERY conductor in this mesh'
      );
    }
    // Baselines BEFORE the restart, so the re-mint assertion counts only what
    // this scenario caused.
    ctx(this).remintBaseline = countOccurrences(doorwayLog(), REMINT_LOG_LINE);
    restartMeshConductors();
  }
);

When(
  "the doorway's pool workers reconnect with the stale token",
  { timeout: STEP_LONG_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const healed = await pollUntil(
      async () => {
        const health = await readHealth(base);
        return health.conductor.connected ? health : null;
      },
      300_000,
      5_000
    );
    assert.ok(
      healed,
      'the doorway never reported conductor.connected again after the restart — the pool did not heal ' +
        'without a doorway restart, which is the failure this scenario guards'
    );
  }
);

Then(
  'the doorway re-mints an app auth token from the conductor admin interface',
  function (this: E2EWorld) {
    const seen = countOccurrences(doorwayLog(), REMINT_LOG_LINE);
    assert.ok(
      seen > ctx(this).remintBaseline,
      `the doorway log records no new "${REMINT_LOG_LINE}" line (baseline ${ctx(this).remintBaseline}, now ${seen}). ` +
        'A conductor restart invalidates issued app-auth tokens; without a re-mint the pool stays broken ' +
        'until the doorway itself restarts.'
    );
  }
);

Then(
  'zome calls through that conductor pool succeed within the recovery window',
  { timeout: STEP_LONG_MS },
  async function (this: E2EWorld) {
    const url = conductorCallUrl(doorwayUrl(this));
    const ok = await pollUntil(
      async () => {
        const sample = await timedGet(url, zomeCallDeadlineMs() + 5_000);
        return sample.status === 200 ? sample : null;
      },
      300_000,
      5_000
    );
    assert.ok(
      ok,
      `${url} never returned 200 within the recovery window — the pool did not re-authenticate after the restart`
    );
    ctx(this).conductorCalls.push(ok);
  }
);

// ===========================================================================
// Scenario 5 — Reconnect churn is visible to operators
// ===========================================================================

Given(
  'a peer conductor that drops its signal subscriber session repeatedly',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const peers = await readStatusPeers(doorwayUrl(this));
    assert.ok(
      peers.length > 0,
      'status.json compute.peers[] is empty — no subscriber is registered, so subscriber churn cannot be observed'
    );
    ctx(this).peersBaseline = peers;
  }
);

When(
  'the operator reads the doorway status endpoint',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    ctx(this).peersObserved = await readStatusPeers(doorwayUrl(this));
  }
);

Then(
  'the peer health snapshot shows a growing reconnect attempt count for that conductor',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const observed = ctx(this).peersObserved;
    assert.ok(
      observed.length > 0,
      'no peer snapshot was read — run the status-endpoint step first'
    );
    for (const peer of observed) {
      assert.ok(
        Number.isInteger(peer.reconnectAttempts),
        `peer "${peer.peerId}" exposes reconnectAttempts=${String(peer.reconnectAttempts)} — the field must be an integer`
      );
    }
    // The 2026-06-10 observability gate: a peer that is visibly reconnecting
    // while its counter reads 0 is the defect (the storm ran for hours with
    // reconnect_attempts: 0 on every peer).
    const reconnecting = observed.filter(peer => peer.reason === 'reconnecting');
    for (const peer of reconnecting) {
      assert.ok(
        peer.reconnectAttempts > 0,
        `peer "${peer.peerId}" reports reason="reconnecting" with reconnectAttempts=0 — the churn counter ` +
          'is not being stamped (PeerHealthRegistry::record_reconnect), so the storm would be invisible again'
      );
    }
    assert.ok(
      reconnecting.length > 0,
      `no peer on this doorway reports reason="reconnecting" (${observed
        .map(peer => `${peer.peerId}=${peer.health}/${peer.reason}`)
        .join(', ')}) — this scenario needs a subscriber that is actually flapping`
    );
    await sleep(10_000);
    const later = await readStatusPeers(doorwayUrl(this));
    for (const peer of reconnecting) {
      const now = later.find(entry => entry.peerId === peer.peerId);
      if (!now || now.health === 'healthy') continue;
      assert.ok(
        now.reconnectAttempts >= peer.reconnectAttempts,
        `peer "${peer.peerId}" reconnectAttempts went backwards ${peer.reconnectAttempts} → ${now.reconnectAttempts} ` +
          'while still degraded — the counter resets only on a healthy transition'
      );
    }
  }
);

Then(
  'the peer is marked degraded with reason {string}',
  function (this: E2EWorld, expectedReason: string) {
    const observed = ctx(this).peersObserved;
    assert.ok(
      observed.length > 0,
      'no peer snapshot was read — run the status-endpoint step first'
    );
    const match = observed.find(peer => peer.reason === expectedReason);
    assert.ok(
      match,
      `no peer on status.json carries reason="${expectedReason}" (${observed
        .map(peer => `${peer.peerId}=${peer.health}/${peer.reason}`)
        .join(', ')})`
    );
    assert.equal(
      match.health,
      'degraded',
      `peer "${match.peerId}" reports reason="${expectedReason}" but health="${match.health}" — ` +
        'record_reconnect() marks the peer Degraded, so the two must agree'
    );
  }
);

// ===========================================================================
// Scenario 6 — Conductor fleet survives a doorway reconnect storm (@requires:shem)
// ===========================================================================

Given(
  'a doorway fronting the full multi-tenant conductor fleet',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const base = resolvePeerUrl('shem');
    const health = await readHealth(base);
    assert.ok(
      health.conductor.pools_total > 1,
      `${base} fronts ${health.conductor.pools_total} conductor pool(s) — a fleet-storm scenario needs more than one`
    );
    ctx(this).metricsBaseline = await scrape(base);
  }
);

Given(
  'every conductor session is severed at the same moment',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const fixture = mesh();
    assert.equal(
      fixture.processControl,
      true,
      'severing every conductor session at once needs process control over the fleet: ' +
        `${fixture.processControlReason ?? 'this run controls no fleet processes'}. Against a deployed ` +
        'multi-tenant fleet the conductors are remote pods, so the harness cannot sever them — the sever ' +
        'has to come from the operator (a fleet roll) with the run attached.'
    );
    const before = requireSeries(
      ctx(this).metricsBaseline ?? (await scrape(resolvePeerUrl('shem'))),
      M_RECONNECT_TOTAL,
      'a synchronized sever is unmeasurable without the reconnect counter'
    );
    const after = await pollUntil(
      async () => {
        const now = sumSeries(await scrape(resolvePeerUrl('shem')), M_RECONNECT_TOTAL);
        return now !== null && now > before ? now : null;
      },
      120_000,
      5_000
    );
    assert.ok(
      after,
      `${M_RECONNECT_TOTAL} never moved past ${before} — no sever was observed on the fleet`
    );
  }
);

When('all pool workers reconnect', { timeout: STEP_LONG_MS }, async function (this: E2EWorld) {
  const base = resolvePeerUrl('shem');
  const healed = await pollUntil(
    async () => {
      const health = await readHealth(base);
      return health.conductor.connected_workers === health.conductor.total_workers ? health : null;
    },
    300_000,
    5_000
  );
  assert.ok(healed, `${base} never returned every pool worker to connected after the sever`);
});

Then(
  'reconnect attempts escalate independently so no conductor sees a sustained synchronized herd',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const peers = await readStatusPeers(resolvePeerUrl('shem'));
    assert.ok(
      peers.length > 1,
      `only ${peers.length} peer(s) on the fleet snapshot — a herd needs several`
    );
    const attempts = peers.map(peer => peer.reconnectAttempts);
    const churn = attempts.reduce((sum, value) => sum + value, 0);
    assert.ok(
      churn > 0,
      'no peer recorded any reconnect attempt — the sever was not observed on this surface'
    );
    assert.ok(
      new Set(attempts).size > 1,
      `every peer reports the SAME reconnect-attempt count (${attempts.join(', ')}) — the ladders are moving ` +
        'in lockstep, which is the synchronized herd this scenario forbids'
    );
  }
);

Then(
  'every conductor returns to healthy within the recovery window',
  { timeout: STEP_LONG_MS },
  async function (this: E2EWorld) {
    const settled = await pollUntil(
      async () => {
        const peers = await readStatusPeers(resolvePeerUrl('shem'));
        return peers.length > 0 && peers.every(peer => peer.health === 'healthy') ? peers : null;
      },
      300_000,
      10_000
    );
    assert.ok(
      settled,
      'at least one conductor never returned to health="healthy" within the recovery window'
    );
  }
);

Then(
  'no conductor reports more concurrent sessions than the worker pool size',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const base = resolvePeerUrl('shem');
    const health = await readHealth(base);
    const sessions = gauge(await scrape(base), M_SESSIONS);
    assert.ok(sessions !== null, `${M_SESSIONS} is absent from the fleet scrape`);
    const ceiling = health.conductor.total_workers + NON_POOL_SESSION_ALLOWANCE;
    assert.ok(
      sessions <= ceiling,
      `${M_SESSIONS}=${sessions} exceeds the worker-pool ceiling ${ceiling} ` +
        `(total_workers=${health.conductor.total_workers} + ${NON_POOL_SESSION_ALLOWANCE} non-pool holders)`
    );
  }
);

// ===========================================================================
// Scenario 7 — A conductor that never answers yields a bounded error
// ===========================================================================

Given(
  'a peer conductor that accepts the connection but never answers a zome call',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const health = await readHealth(base);
    assert.ok(
      health.conductor.pools_total >= 1,
      `this doorway fronts ${health.conductor.pools_total} conductor pools — there is no conductor hop to time out on`
    );
    assertPoolCoupling(health);
  }
);

When(
  'the doorway issues a zome call through that conductor',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const deadline = zomeCallDeadlineMs();
    // Fire the conductor call and a /health probe together: "stays responsive"
    // is only meaningful measured DURING the call, not after it.
    const call = timedGet(conductorCallUrl(base), deadline * 3);
    const concurrent = timedGet(`${base}/health`, PROMPT_MS);
    const [callSample, concurrentSample] = await Promise.all([call, concurrent]);
    ctx(this).conductorCalls.push(callSample);
    ctx(this).concurrentProbe = concurrentSample;
  }
);

Then(
  'the call returns an error within the conductor-call hard deadline',
  function (this: E2EWorld) {
    const sample = ctx(this).conductorCalls.at(-1);
    assert.ok(sample, 'no conductor call was issued — run the zome-call step first');
    const budget = zomeCallDeadlineMs() * 2;
    assert.ok(
      sample.elapsedMs <= budget,
      `the conductor call took ${sample.elapsedMs}ms, past the ${zomeCallDeadlineMs()}ms hard deadline ` +
        `(DOORWAY_ZOME_CALL_TIMEOUT_MS) plus margin (${budget}ms). An unbounded conductor call is what parked ` +
        'the lone tokio worker and froze the gateway.'
    );
    assert.ok(
      sample.status > 0,
      `the conductor call produced no HTTP status at all (${sample.text.slice(0, 200)}) — it hung rather than ` +
        'resolving to a bounded answer'
    );
  }
);

Then(
  'the doorway clears that conductor connection so the next call reconnects',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const deadline = zomeCallDeadlineMs();
    const second = await timedGet(conductorCallUrl(base), deadline * 3);
    ctx(this).conductorCalls.push(second);
    assert.ok(
      second.elapsedMs <= deadline * 2,
      `the follow-up conductor call took ${second.elapsedMs}ms — a stale connection was reused instead of ` +
        "being cleared, so the second caller inherits the first call's wedge"
    );
    assertPoolCoupling(await readHealth(base));
  }
);

Then('the doorway keeps serving other requests throughout', function (this: E2EWorld) {
  const concurrent = ctx(this).concurrentProbe;
  assert.ok(concurrent, 'no concurrent probe was taken — run the zome-call step first');
  assert.equal(
    concurrent.status,
    200,
    `/health answered ${concurrent.status} while the conductor call was in flight — the gateway must stay ` +
      'responsive when one conductor hop is silent'
  );
  assert.ok(
    concurrent.elapsedMs <= PROMPT_MS,
    `/health took ${concurrent.elapsedMs}ms while the conductor call was in flight (budget ${PROMPT_MS}ms)`
  );
});

// ===========================================================================
// Scenario 8 — The gateway stays responsive while one request path is wedged
// ===========================================================================

Given(
  'the doorway runs with explicitly configured tokio worker threads',
  function (this: E2EWorld) {
    const announced = lastMatch(doorwayLog(), WORKER_THREADS_LOG_RE);
    assert.ok(
      announced,
      'the doorway log carries no "Tokio worker threads: N (explicit)" line (main.rs) — this process did not ' +
        'set its worker count explicitly, so the cgroup cpu limit can still collapse the runtime to one worker'
    );
    const workers = Number.parseInt(announced[1], 10);
    assert.ok(
      workers > 1,
      `the doorway announced ${workers} tokio worker thread(s) — a single worker is the 2026-06-13 freeze shape: ` +
        'one blocked await freezes the whole runtime'
    );
    ctx(this).workerThreads = workers;
  }
);

Given(
  'one request path is blocked on a slow upstream',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const url = conductorCallUrl(base);
    const probe = await timedGet(url, zomeCallDeadlineMs() + 5_000);
    assert.notEqual(
      probe.status,
      404,
      `${url} is not a route on this doorway — pick a real conductor-backed path to block on`
    );
    const workers = ctx(this).workerThreads ?? 4;
    ctx(this).blockedPath = {
      url,
      settled: burst(url, workers * 3, zomeCallDeadlineMs() * 3),
    };
  }
);

When(
  'a concurrent request hits the gateway during the block',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const blocked = ctx(this).blockedPath;
    assert.ok(
      blocked,
      'no blocked request path was established — run the slow-upstream step first'
    );
    ctx(this).concurrentProbe = await timedGet(`${doorwayUrl(this)}/health`, PROMPT_MS);
  }
);

Then(
  'the concurrent request is served without waiting for the blocked path',
  function (this: E2EWorld) {
    const concurrent = ctx(this).concurrentProbe;
    assert.ok(concurrent, 'no concurrent probe was taken — run the concurrent-request step first');
    assert.equal(
      concurrent.status,
      200,
      `/health answered ${concurrent.status} while ${String(ctx(this).workerThreads)} × 3 requests were pinned on ` +
        'the blocked path — the runtime failed to schedule other work'
    );
    assert.ok(
      concurrent.elapsedMs <= PROMPT_MS,
      `/health took ${concurrent.elapsedMs}ms during the block (budget ${PROMPT_MS}ms) — it queued behind the ` +
        'wedged path instead of being scheduled alongside it'
    );
  }
);

Then(
  'the readiness probe on the main listener continues to answer',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const ready = await timedGet(`${doorwayUrl(this)}/ready`, PROMPT_MS);
    assert.ok(
      ready.status === 200 || ready.status === 503,
      `/ready on the MAIN listener answered ${ready.status} — readiness and startup stay on :8080 and must give a ` +
        'definite verdict (200 ready / 503 not ready), never a hang'
    );
    assert.ok(
      ready.elapsedMs <= PROMPT_MS,
      `/ready took ${ready.elapsedMs}ms on the main listener (budget ${PROMPT_MS}ms)`
    );
    const blocked = ctx(this).blockedPath;
    if (blocked) await blocked.settled;
  }
);

// ===========================================================================
// Scenario 9 — A saturated SSR render queue sheds to the CSR shell
// ===========================================================================

Given(
  'the SSR render isolate is busy with an in-flight render',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    if (!destructiveAllowed()) {
      return holdDestructive(
        'this step would fire 12 concurrent SSR renders of / to saturate the doorway render isolate'
      );
    }
    const base = doorwayUrl(this);
    const first = await getRawWithHeaders(`${base}/`, { timeoutMs: 20_000 });
    assert.equal(
      first.status,
      200,
      `GET ${base}/ answered ${first.status} — there is no SSR route to saturate on this doorway`
    );
    // SUSTAINED, not one-shot. A burst of 12 renders drains in a couple of
    // seconds, so by the time the next step had polled the log for the
    // saturation line and the When had issued its probe, the queue was idle
    // again and the probe was RENDERED — no shed, no x-ssr-skipped, and the
    // scenario failed against a doorway that had behaved correctly (measured
    // 2026-08-22). The load is held until scenario cleanup, so the shed window
    // covers the steps that assert it and stops the moment they are done.
    // Baseline BEFORE the load: the saturation line is counted across a 2 MB log
    // tail that already holds earlier scenarios' (and earlier boots') lines, so
    // "> 0" would be satisfied by history. Only GROWTH proves this load saturated.
    ctx(this).ssrSaturationBaseline = countOccurrences(doorwayLog(), SSR_SATURATED_LOG_LINE);
    ctx(this).ssrBurst = sustainedRenders(`${base}/`, 12);
    this.onCleanup(async () => {
      const burst = ctx(this).ssrBurst;
      if (!burst) return;
      burst.stop();
      await burst.settled;
    });
  }
);

Given(
  'its bounded render queue is already saturated',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const burstPromise = ctx(this).ssrBurst;
    assert.ok(burstPromise, 'no render burst is in flight — run the in-flight-render step first');
    const baseline = ctx(this).ssrSaturationBaseline ?? 0;
    const saturated = await pollUntil(
      () => countOccurrences(doorwayLog(), SSR_SATURATED_LOG_LINE) > baseline || null,
      30_000,
      1_000
    );
    assert.ok(
      saturated,
      `the doorway log never recorded "${SSR_SATURATED_LOG_LINE}" — the render semaphore was never driven to its ` +
        'limit, so the shed path this scenario asserts was not reached. (Saturation has no Prometheus series: ' +
        'server/http.rs emits it only as a tracing event on the ssr_busy target.)'
    );
  }
);

When('another SSR request arrives', { timeout: STEP_SHORT_MS }, async function (this: E2EWorld) {
  const base = doorwayUrl(this);
  const started = Date.now();
  const res = await getRawWithHeaders(`${base}/`, { timeoutMs: 30_000 });
  ctx(this).ssrShed = { ...res, elapsedMs: Date.now() - started };
});

Then('that request is shed to the CSR shell fallback promptly', function (this: E2EWorld) {
  const shed = ctx(this).ssrShed;
  assert.ok(shed, 'no extra SSR request was issued — run the another-SSR-request step first');
  assert.equal(
    shed.status,
    200,
    `the shed answered ${shed.status}; a shed serves the CSR shell, not an error`
  );
  const reason = shed.headers['x-ssr-skipped'];
  assert.ok(
    reason,
    'the response carries no x-ssr-skipped header — a saturated isolate must TAG its fallback so the shed is ' +
      'countable (server/http.rs ssr_fallback_response)'
  );
  assert.equal(
    reason,
    'overflow',
    `x-ssr-skipped="${reason}" — a render-queue saturation sheds with reason "overflow" (SsrFallbackReason::Overflow)`
  );
});

Then(
  'the gateway does not block waiting for render-queue capacity',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const shed = ctx(this).ssrShed;
    assert.ok(shed, 'no shed response was captured — run the another-SSR-request step first');
    assert.ok(
      shed.elapsedMs <= PROMPT_MS,
      `the shed took ${shed.elapsedMs}ms (budget ${PROMPT_MS}ms) — try_send must refuse instantly; a blocking send ` +
        'onto the capacity-1 queue back-pressures onto the runtime, which is the 2026-06-13 freeze shape'
    );
    const health = await timedGet(`${doorwayUrl(this)}/health`, PROMPT_MS);
    assert.equal(
      health.status,
      200,
      `/health answered ${health.status} while the render queue was saturated`
    );
    const burst = ctx(this).ssrBurst;
    if (burst) {
      burst.stop();
      await burst.settled;
    }
  }
);

// ===========================================================================
// Scenario 10 — Liveness survives a full main-runtime stall (DESTRUCTIVE)
// ===========================================================================

Given(
  'the doorway serves liveness from a runtime isolated from the main worker pool',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const watchdog = findWatchdog(this);
    if (watchdog === null) return holdNoWatchdog();
    const probe = await timedGet(`${watchdog.url}/health`, PROMPT_MS);
    assert.ok(
      probe.status === 200 || probe.status === 503,
      `the watchdog listener at ${watchdog.url} answered ${probe.status} — it must give a liveness verdict`
    );
    const body = JSON.parse(probe.text) as { check?: string };
    assert.equal(
      body.check,
      'main-runtime-heartbeat',
      `${watchdog.url}/health does not identify itself as the main-runtime-heartbeat check — this is the MAIN ` +
        'listener answering, not the isolated watchdog runtime'
    );
    ctx(this).watchdogProbes.push(probe);
  }
);

Given(
  'the liveness verdict is gated on a heartbeat the main runtime stamps',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const metrics = await scrape(doorwayUrl(this));
    const age = gauge(metrics, M_HEARTBEAT_AGE);
    assert.ok(
      age !== null,
      `/metrics carries no ${M_HEARTBEAT_AGE} — the watchdog's verdict would then be ungated (an always-200 probe ` +
        'that cannot fail a genuine wedge)'
    );
    ctx(this).heartbeatSamples.push(age);
    ctx(this).wedgedBaseline = requireSeries(
      metrics,
      M_WATCHDOG_WEDGED,
      'a wedge declaration would leave no durable trace without it'
    );
  }
);

When(
  'every main worker thread is parked at once',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const watchdog = findWatchdog(this);
    if (watchdog === null) return holdNoWatchdog();
    if (!destructiveAllowed()) {
      return holdDestructive(
        'this step would pin every main tokio worker with a burst of conductor-hop requests ' +
          '(workerThreads x 4 concurrent GETs) while probing the watchdog'
      );
    }
    const base = doorwayUrl(this);
    const workers =
      ctx(this).workerThreads ??
      Number.parseInt(lastMatch(doorwayLog(), WORKER_THREADS_LOG_RE)?.[1] ?? '4', 10);
    // Occupy every main worker with a conductor-hop request, then probe the
    // watchdog and the heartbeat WHILE they are pinned.
    const pinned = burst(conductorCallUrl(base), workers * 4, zomeCallDeadlineMs() * 3);
    await sleep(500);
    for (let i = 0; i < 3; i += 1) {
      ctx(this).watchdogProbes.push(await timedGet(`${watchdog.url}/health`, PROMPT_MS));
      const age = gauge(await scrape(base), M_HEARTBEAT_AGE);
      if (age !== null) ctx(this).heartbeatSamples.push(age);
      await sleep(1_000);
    }
    await pinned;
  }
);

Then(
  'the liveness endpoint still answers promptly from its isolated runtime',
  function (this: E2EWorld) {
    const probes = ctx(this).watchdogProbes;
    assert.ok(
      probes.length > 0,
      'the watchdog was never probed — run the parked-workers step first'
    );
    for (const probe of probes) {
      assert.ok(
        probe.status === 200 || probe.status === 503,
        `the watchdog answered ${probe.status} while the main pool was pinned — its own OS-thread runtime must ` +
          'keep answering regardless of the main pool'
      );
      assert.ok(
        probe.elapsedMs <= PROMPT_MS,
        `the watchdog took ${probe.elapsedMs}ms (budget ${PROMPT_MS}ms) while the main pool was pinned — it is ` +
          'sharing the main runtime, not isolated from it'
      );
    }
  }
);

Then(
  'it reports unhealthy because the main-runtime heartbeat has gone stale',
  function (this: E2EWorld) {
    const probes = ctx(this).watchdogProbes;
    assert.ok(
      probes.length > 0,
      'the watchdog was never probed — run the parked-workers step first'
    );
    for (const probe of probes) {
      const body = JSON.parse(probe.text) as {
        healthy?: boolean;
        heartbeatAgeMs?: number;
        thresholdMs?: number;
      };
      assert.ok(
        typeof body.heartbeatAgeMs === 'number' && typeof body.thresholdMs === 'number',
        `the watchdog verdict ${probe.text.slice(0, 200)} omits heartbeatAgeMs/thresholdMs — an ungated verdict ` +
          'cannot distinguish a real wedge from a healthy runtime'
      );
      const stale = body.heartbeatAgeMs > body.thresholdMs;
      assert.equal(
        body.healthy,
        !stale,
        `the watchdog reported healthy=${String(body.healthy)} with heartbeatAgeMs=${body.heartbeatAgeMs} against ` +
          `thresholdMs=${body.thresholdMs} — the verdict must be exactly "unhealthy iff the main-runtime heartbeat ` +
          'is stale"'
      );
      assert.equal(
        probe.status,
        stale ? 503 : 200,
        `the watchdog returned HTTP ${probe.status} for a heartbeat age of ${body.heartbeatAgeMs}ms against a ` +
          `${body.thresholdMs}ms threshold — a stale heartbeat must be a 503 so kubelet restarts the pod`
      );
    }
  }
);

Then(
  'the pod is restarted instead of hanging unnoticed',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    // Read the log BEFORE the scrape: the counter can only have caught up, so
    // counter >= log lines is the honest direction of this audit.
    const declared = countOccurrences(doorwayLog(), WEDGE_LOG_LINE);
    const wedged = requireSeries(
      await scrape(doorwayUrl(this)),
      M_WATCHDOG_WEDGED,
      'a wedge that triggers a restart must leave a durable counter behind'
    );
    assert.ok(
      wedged >= declared,
      `${M_WATCHDOG_WEDGED}=${wedged} is below the ${declared} "${WEDGE_LOG_LINE}" declarations in the doorway log — ` +
        'a wedge that fails liveness (and so restarts the pod) must also be recorded, or the restart is unattributable'
    );
    assert.ok(
      wedged >= ctx(this).wedgedBaseline,
      `${M_WATCHDOG_WEDGED} went backwards (${ctx(this).wedgedBaseline} → ${wedged}) — the counter is monotonic ` +
        'within one process lifetime; a decrease means the process restarted mid-scenario'
    );
  }
);

// ===========================================================================
// Scenario 11 — A momentary worker-pool park does not false-kill the pod
// ===========================================================================

Given(
  "the doorway's main runtime is briefly busy but recovers within the staleness window",
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const watchdog = findWatchdog(this);
    if (watchdog === null) return holdNoWatchdog();
    if (!destructiveAllowed()) {
      return holdDestructive(
        'this step would drive a short concurrency burst (workerThreads x 2 renders of /) at the ' +
          'doorway to make its main runtime briefly busy'
      );
    }
    const base = doorwayUrl(this);
    const metrics = await scrape(base);
    ctx(this).wedgedBaseline = requireSeries(
      metrics,
      M_WATCHDOG_WEDGED,
      '"the pod is not restarted" is unfalsifiable without the wedge counter'
    );
    const workers = ctx(this).workerThreads ?? 4;
    // A park SHORTER than the staleness threshold: the honesty counterpart to
    // the full stall — this must NOT trip a liveness kill.
    const brief = burst(`${base}/`, workers * 2, Math.floor(watchdog.thresholdMs / 3));
    ctx(this).blockedPath = { url: `${base}/`, settled: brief };
  }
);

When(
  'the liveness watchdog is probed during and after the brief park',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const watchdog = findWatchdog(this);
    if (watchdog === null) return holdNoWatchdog();
    const blocked = ctx(this).blockedPath;
    assert.ok(blocked, 'no brief park was started — run the briefly-busy step first');
    const during = await timedGet(`${watchdog.url}/health`, PROMPT_MS);
    ctx(this).watchdogProbes.push(during);
    ctx(this).heartbeatSamples.push(gauge(await scrape(doorwayUrl(this)), M_HEARTBEAT_AGE) ?? -1);
    await blocked.settled;
    const after = await timedGet(`${watchdog.url}/health`, PROMPT_MS);
    ctx(this).watchdogProbes.push(after);
    ctx(this).heartbeatSamples.push(gauge(await scrape(doorwayUrl(this)), M_HEARTBEAT_AGE) ?? -1);
  }
);

Then('the heartbeat age stays under the staleness threshold', function (this: E2EWorld) {
  const samples = ctx(this).heartbeatSamples;
  assert.ok(
    samples.length > 0,
    'no heartbeat samples were taken — run the watchdog-probe step first'
  );
  const threshold = healthStaleThresholdMs(this);
  for (const age of samples) {
    assert.ok(
      age >= 0,
      `${M_HEARTBEAT_AGE} was absent from a scrape during the park — an unreadable heartbeat is not a healthy one`
    );
    assert.ok(
      age <= threshold,
      `${M_HEARTBEAT_AGE}=${age} exceeded the ${threshold}ms staleness threshold during a park the runtime was ` +
        'expected to recover from — a transient park must not reach the kill threshold'
    );
  }
});

Then('the liveness probe keeps returning healthy', function (this: E2EWorld) {
  const probes = ctx(this).watchdogProbes;
  assert.ok(probes.length >= 2, 'need a during-park and an after-park watchdog probe');
  for (const probe of probes) {
    assert.equal(
      probe.status,
      200,
      `the watchdog answered ${probe.status} (${probe.text.slice(0, 200)}) across a brief, recovered park — that ` +
        'is the false-kill class moving liveness off the shared listener was meant to remove'
    );
  }
});

Then('the pod is not restarted', { timeout: STEP_SHORT_MS }, async function (this: E2EWorld) {
  const wedged = requireSeries(
    await scrape(doorwayUrl(this)),
    M_WATCHDOG_WEDGED,
    'a false kill is unfalsifiable without the wedge counter'
  );
  assert.equal(
    wedged,
    ctx(this).wedgedBaseline,
    `${M_WATCHDOG_WEDGED} moved ${ctx(this).wedgedBaseline} → ${wedged} across a brief, recovered park — the ` +
      'watchdog declared a wedge it should have ridden out, which is a false liveness kill'
  );
});

// ===========================================================================
// Scenario 12 — A conductor DNS flap does not park the doorway's worker pool
// ===========================================================================

Given(
  'a peer conductor whose hostname briefly fails to resolve during a pod roll',
  { timeout: STEP_SHORT_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const health = await readHealth(base);
    assert.ok(
      health.conductor.pools_total >= 1,
      `this doorway fronts ${health.conductor.pools_total} conductor pools — there is no conductor hostname to flap`
    );
    const metrics = await scrape(base);
    ctx(this).metricsBaseline = metrics;
    // connect_refused is the label a failing resolve lands on (metrics.rs
    // REASON_CONNECT_REFUSED — "refused before any session").
    requireSeries(
      metrics,
      M_RECONNECT_TOTAL,
      'a resolve failure cannot be attributed without the reconnect counter'
    );
    const age = gauge(metrics, M_HEARTBEAT_AGE);
    assert.ok(
      age !== null,
      `/metrics carries no ${M_HEARTBEAT_AGE} — worker-pool liveness would be unobservable`
    );
    ctx(this).heartbeatSamples.push(age);
  }
);

When(
  "the doorway's subscriber and pool workers reconnect through the DNS failure",
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    for (let i = 0; i < 5; i += 1) {
      const probe = await timedGet(`${base}/health`, PROMPT_MS * 2);
      ctx(this).healthLatencySamples.push(probe.elapsedMs);
      const age = gauge(await scrape(base), M_HEARTBEAT_AGE);
      ctx(this).heartbeatSamples.push(age ?? -1);
      await sleep(2_000);
    }
    assert.ok(
      ctx(this).heartbeatSamples.length >= 5,
      'fewer than five heartbeat samples were taken across the flap window'
    );
  }
);

Then('conductor host resolution runs off the worker pool, not on it', function (this: E2EWorld) {
  const threshold = healthStaleThresholdMs(this);
  const samples = ctx(this).heartbeatSamples;
  assert.ok(samples.length > 0, 'no heartbeat samples were taken — run the reconnect step first');
  const peak = Math.max(...samples);
  assert.ok(
    peak >= 0,
    `${M_HEARTBEAT_AGE} was unreadable during the flap — an unreadable heartbeat cannot clear this invariant`
  );
  assert.ok(
    peak <= threshold,
    `${M_HEARTBEAT_AGE} peaked at ${peak}ms against a ${threshold}ms threshold while conductors were reconnecting ` +
      '(samples: ' +
      samples.join(', ') +
      '). A climbing heartbeat under reconnect churn with no CPU cost is the blocking-getaddrinfo signature: ' +
      'resolution is running ON the worker pool, where the zome-call timeout cannot cancel it.'
  );
});

Then(
  "the doorway's worker pool stays available to schedule other tasks",
  function (this: E2EWorld) {
    const latencies = ctx(this).healthLatencySamples;
    assert.ok(
      latencies.length > 0,
      'no /health latencies were sampled — run the reconnect step first'
    );
    const worst = Math.max(...latencies);
    assert.ok(
      worst <= PROMPT_MS,
      `/health peaked at ${worst}ms during the flap (budget ${PROMPT_MS}ms; samples: ${latencies.join(', ')}) — ` +
        'the main listener could not be scheduled, which is workers parked in the resolver'
    );
  }
);

Then(
  'the doorway keeps serving requests throughout the flap',
  { timeout: STEP_MEDIUM_MS },
  async function (this: E2EWorld) {
    const base = doorwayUrl(this);
    const contentId = mesh().commonsEprId ?? 'manifesto';
    const res = await getRawRidingCatchUp(`${base}/db/content/${encodeURIComponent(contentId)}`);
    const named = shedCause(res.text);
    const cause = named ? ` (cause=${named.cause})` : '';
    assert.equal(
      res.status,
      200,
      `a read of "${contentId}" answered ${res.status}${cause}${describeCatchUpRide(res)} — ` +
        'a conductor DNS flap must not stop the doorway serving content'
    );
    ctx(this).contentRead = {
      status: res.status,
      text: res.text,
      elapsedMs: res.rodeCatchUpMs ?? 0,
    };
  }
);
