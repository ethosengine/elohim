/**
 * Step glue for features/doorway/self-healing-flow-control.feature — the
 * doorway self-healing control plane (shift/self-healing-control-plane):
 *
 *   Plan A — warm-up upstream self-protection (single-pass, frozen tick=0;
 *            `doorway::projection::warm_stream`).
 *   Plan B — bilateral inbound/outbound flow control: doorway's inbound
 *            admission semaphore (`server/http.rs`) and the storage-proxy
 *            per-upstream circuit breaker (`routes/upstream_health.rs`), plus
 *            elohim-storage's own per-request inflight shed (`http.rs`).
 *   Plan C — the unified `/admin/self-healing` read model
 *            (`routes/self_healing.rs`).
 *
 * Every probe here reads a LIVE surface — an HTTP endpoint, a Prometheus
 * counter, or the running doorway's own JSON-lines log file (tailed via the
 * household fixture's `logPath`) — never a synthesized/mocked result.
 *
 * TWO NAMED FIDELITY GAPS this file could not close against the doorway
 * source (see the individual step comments for the code citations):
 *
 *   1. Plan A's per-peer skip ("that upstream's circuit is Open and it is
 *      skipped before the serial loop runs" / "records the skip as a
 *      self-heal event") has NO discrete self-heal event in
 *      warm_stream.rs — only the ALL-skipped case ("anti-self-partition")
 *      and the whole-pass "budget-exhausted" case emit a named
 *      `WarmupSelfHealEvent`. A single gated-out peer only shows up as a
 *      peer_count < total_peers delta on the "Starting cache stream
 *      warm-up (health-gated)" log line — there is no per-peer skip event
 *      to assert on. The Then step below asserts the SCENARIO's literal
 *      claim (so it fails honestly against real code) and documents the gap.
 *   2. Scenario C's Then step ("the set is non-empty and contains every
 *      configured upstream") does not match `gate_upstreams`' actual
 *      all-skipped behaviour: it returns `vec![least_bad]` — a ONE-element
 *      set, not every configured upstream (warm_stream.rs
 *      `gate_never_self_partitions_to_empty` unit test pins exactly this).
 *      The step asserts the scenario's literal claim; it will fail against
 *      the real "least-bad singleton" behaviour even when the all-open
 *      precondition is met. Also reported to the caller.
 *   3. Scenario G's exact regression ("200 headers then stalls before the
 *      body completes") needs a controllable mock storage upstream that can
 *      hold the body open after status/headers are already on the wire.
 *      The household-mesh fixture only offers REAL storage peers (whole
 *      process pause via SIGSTOP), which reproduces a connect/read FAILURE,
 *      not a body-phase stall. The step exercises the same
 *      record-once-per-terminal-path breaker discipline via that closest
 *      real substitute and says so.
 *
 * Warm-up (Plan A) runs exactly ONCE per doorway process at startup
 * (spawn_stream_task, warm_stream.rs) — there is no sanctioned mechanism in
 * this harness to force a fresh pass (doorway restart is not one of the
 * process-control primitives this suite is authorized to drive; only
 * per-peer SIGSTOP/SIGCONT and the documented `hc-mesh.sh conductors-restart`
 * are). Scenarios A/B/C therefore OBSERVE the already-completed pass from
 * the doorway's own JSON-lines log rather than manufacturing the
 * precondition — an honest, falsifiable read of what actually happened at
 * boot, not a fabricated success.
 */

import { strict as assert } from 'node:assert';
import { readFile } from 'node:fs/promises';
import { request as httpRequest } from 'node:http';

import { Given, Then, When } from '@cucumber/cucumber';

import {
  getRaw,
  getRawWithHeaders,
  probeHealth,
  type RawHttpResponse,
} from '../../src/framework/dataplane/surfaces.js';
import {
  loadHouseholdMeshFixture,
  requireFixtureDoorwayLogPath,
  requireFixturePeerPid,
  requireFixturePoolStorageUrls,
  requireFixturePrimaryStorageUrl,
  requireFixtureStoragePeer,
  type HouseholdMeshFixture,
} from '../../src/framework/fixtures/household-mesh.js';
import { doorwayPidForPort, reexecProcess } from '../../src/framework/fixtures/process-control.js';
import {
  destructiveAllowed,
  DESTRUCTIVE_HELD_HINT,
} from '../../src/framework/fixtures/substrate-scope.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Shared constants — mirror the doorway/storage source (feature's "Operational
// parameters" table). Kept local (not imported) because these are Rust
// `const`s with no TS projection; each is cited to its source line.
// ---------------------------------------------------------------------------

/** warm_stream.rs WARMUP_TOTAL_BUDGET_SECS */
const WARMUP_TOTAL_BUDGET_SECS = 75;
/** warm_stream.rs WARMUP_CIRCUIT_FAIL_THRESHOLD */
const WARMUP_CIRCUIT_FAIL_THRESHOLD = 3;
/** upstream_health.rs UPSTREAM_CIRCUIT_FAIL_THRESHOLD */
const UPSTREAM_CIRCUIT_FAIL_THRESHOLD = 3;
/** upstream_health.rs UPSTREAM_CIRCUIT_COOLDOWN_SECS */
const UPSTREAM_CIRCUIT_COOLDOWN_SECS = 30;
/** server/http.rs DOORWAY_ADMISSION_RETRY_AFTER_SECS */
const DOORWAY_ADMISSION_RETRY_AFTER_SECS = 2;
/** server/http.rs DEFAULT_MAX_INFLIGHT_READ (read-admission pool ceiling) */
const DOORWAY_MAX_INFLIGHT_READ = 512;
/** elohim-storage http.rs STORAGE_SHED_RETRY_AFTER_SECS */
const STORAGE_SHED_RETRY_AFTER_SECS = 2;
/** elohim-storage http.rs MAX_CONCURRENT_READS */
const STORAGE_MAX_CONCURRENT_READS = 256;

/** A cheap, non-mutating read route every peer serves, used to build inbound bursts. */
const CHEAP_READ_PATH = '/db/content?limit=1';

/**
 * A read this scenario suite deliberately targets that storage will answer
 * 404 (Neutral, per `ProxyOutcome::classify`) rather than 200 — used to drive
 * a legitimate client-error trial outcome (scenario H) without mutating
 * anything.
 */
const UNKNOWN_CONTENT_PATH = '/db/content/e2e-self-healing-flow-control-unknown-id';

function withoutTrailingSlash(url: string): string {
  let end = url.length;
  while (end > 0 && url[end - 1] === '/') end -= 1;
  return url.slice(0, end);
}

// ---------------------------------------------------------------------------
// Doorway / storage resolution (household-mesh fixture, never a hardcoded port)
// ---------------------------------------------------------------------------

function fixture(): HouseholdMeshFixture {
  return loadHouseholdMeshFixture();
}

function doorwayUrl(world: E2EWorld, id = 'alpha'): string {
  return withoutTrailingSlash(world.getDoorway(id).url);
}

function primaryStorageUrl(id = 'alpha'): string {
  return requireFixturePrimaryStorageUrl(fixture(), id);
}

function poolStorageUrls(id = 'alpha'): string[] {
  return requireFixturePoolStorageUrls(fixture(), id);
}

// ---------------------------------------------------------------------------
// JSON-lines log tailing — doorway logs one JSON object per line (verified
// live against /tmp/elohim-local-mesh/logs/doorway.log): {"timestamp",
// "level","fields":{"message",...},"target"}. Offset-based tail mirrors
// federation-failover.steps.ts's captureLogStart/newLogText pattern (a
// sibling file, not reused directly — no such helper is exported there).
// ---------------------------------------------------------------------------

interface DoorwayLogLine {
  timestamp: string;
  level: string;
  fields: Record<string, unknown> & { message?: string };
  target: string;
}

function parseLogLines(text: string): DoorwayLogLine[] {
  const lines: DoorwayLogLine[] = [];
  for (const raw of text.split('\n')) {
    const trimmed = raw.trim();
    if (!trimmed) continue;
    try {
      const parsed = JSON.parse(trimmed) as unknown;
      if (parsed && typeof parsed === 'object' && 'fields' in (parsed as Record<string, unknown>)) {
        lines.push(parsed as DoorwayLogLine);
      }
    } catch {
      // Non-JSON line (startup ASCII banner etc.) — not a structured event.
    }
  }
  return lines;
}

async function readDoorwayLogLines(doorwayId = 'alpha'): Promise<DoorwayLogLine[]> {
  const path = requireFixtureDoorwayLogPath(fixture(), doorwayId);
  let text: string;
  try {
    text = await readFile(path, 'utf8');
  } catch (error) {
    assert.fail(`cannot read doorway "${doorwayId}" log (${path}): ${String(error)}`);
  }
  return parseLogLines(text);
}

/** The doorway's first line of every boot (main.rs, before anything else). */
const BOOT_MARKER = 'doorway starting';

/**
 * The lines of the CURRENT boot only.
 *
 * The mesh doorway appends to ONE log file across every restart, so the whole
 * file is a stack of boots going back days. A pass-scoped assertion read against
 * the unscoped file reads the FIRST boot's evidence, not this one's — which is
 * how "this pass's log" dumps used to start at the top of a 34k-line file.
 * Everything a warm-up/restart drill asserts is scoped here, at the last
 * `doorway starting` marker.
 */
function linesSinceLastBoot(lines: DoorwayLogLine[]): DoorwayLogLine[] {
  let start = 0;
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    if (lines[i].fields.message === BOOT_MARKER) {
      start = i;
      break;
    }
  }
  return lines.slice(start);
}

/**
 * Host aliases the mesh mixes freely: the fixture says `localhost`, the doorway
 * is launched with `127.0.0.1`, and both name the same peer. Compared as
 * `host:port` with the alias folded, never as raw strings — a raw compare made
 * `/admin/self-healing`'s `upstreams[].endpoint` ("http://127.0.0.1:8090")
 * invisible to a step looking for "localhost:8090".
 */
function authorityOf(url: string): string {
  const withoutScheme = withoutTrailingSlash(url).replace(/^[a-z]+:\/\//, '');
  const authority = withoutScheme.split('/')[0];
  return authority.replace(/^localhost:/, '127.0.0.1:');
}

/** Do two URLs name the same peer (scheme- and alias-insensitive, path ignored)? */
function sameUpstream(a: string, b: string): boolean {
  return authorityOf(a) === authorityOf(b);
}

// ---------------------------------------------------------------------------
// Process control — SIGSTOP/SIGCONT by the fixture's EXACT pid only, never a
// pattern kill. Mirrors federation-failover.steps.ts's pausePeer/resumePeer
// idiom (not imported — that file does not export it).
// ---------------------------------------------------------------------------

type HouseholdPeerName = 'matthew' | 'jessica' | 'james';

function peerPid(name: HouseholdPeerName): number {
  return requireFixturePeerPid(fixture(), name);
}

function pausePeerProcess(name: HouseholdPeerName): void {
  process.kill(peerPid(name), 'SIGSTOP');
}

function resumePeerProcess(name: HouseholdPeerName): void {
  process.kill(peerPid(name), 'SIGCONT');
}

/** Storage peer name that backs a doorway's primary storage URL (household convention). */
function primaryPeerName(doorwayId = 'alpha'): HouseholdPeerName {
  const primary = primaryStorageUrl(doorwayId);
  for (const name of ['matthew', 'jessica', 'james'] as const) {
    if (requireFixtureStoragePeer(fixture(), name).url === primary) return name;
  }
  assert.fail(
    `no household storage peer's fixture URL matches doorway "${doorwayId}"'s primary ` +
      `storage URL (${primary}) — cannot resolve which peer to pause`
  );
}

async function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// ---------------------------------------------------------------------------
// Destructive-leg gate — ONE switch for every step that mutates live mesh
// state (pauses a real process, fires a saturating burst, or restarts a real
// doorway). Default OFF: the mesh is shared and this suite does not get to
// decide when it is safe to take a peer down. Set A2O_ALLOW_DESTRUCTIVE=1 to
// let the gated Given steps construct their precondition for real.
//
// Gating only the FIRST Given of a scenario is sufficient: cucumber-js marks
// every later step in the same scenario SKIPPED once one step's result is
// 'skipped' (test_case_runner.js `isSkippingSteps()` — a SKIPPED result is
// "not PASSED", so it cascades), so a two-Given scenario's second Given never
// even runs when the first one is held.
// ---------------------------------------------------------------------------

const DESTRUCTIVE = destructiveAllowed();

/** Log the reason and mark the step (and every later step this scenario) skipped, not failed. */
function skipHeldDestructive(reason: string): 'skipped' {
  // eslint-disable-next-line no-console
  console.log(
    `  ⏭️  DESTRUCTIVE STEP HELD: ${reason} ${DESTRUCTIVE_HELD_HINT} ` +
      '(same disposition steps/common.steps.ts already gives a substrate-held scenario).'
  );
  return 'skipped';
}

// ---------------------------------------------------------------------------
// Doorway restart — a re-exec of the SAME process (argv/env/cwd read back out
// of /proc before the kill, replayed after). The technique lives ONCE in
// src/framework/fixtures/process-control.ts (doorwayPidForPort, reexecProcess),
// shared with steps/delivery/substrate-rea-replication.steps.ts's
// "doorway {string}'s pod is restarted" step. The pid is found by an EXACT
// argv match on the listen port, refusing on zero OR more than one match.
// ---------------------------------------------------------------------------

async function waitForDoorwayHealthy(url: string, budgetMs: number): Promise<void> {
  const deadline = Date.now() + budgetMs;
  let last = 'never answered';
  while (Date.now() < deadline) {
    try {
      const health = await probeHealth(url);
      if (health.status === 200 && health.body.healthy === true) return;
      last = `status ${health.status}`;
    } catch (error) {
      last = String(error);
    }
    await sleep(1_000);
  }
  assert.fail(`doorway at ${url} did not come back healthy within ${budgetMs}ms (last: ${last})`);
}

const DOORWAY_RESTART_GRACE_MS = 20_000;
const DOORWAY_RESTART_HEALTH_BUDGET_MS = 90_000;

/**
 * Restart doorway "alpha" for real, as the same process re-exec'd. This is
 * what makes Plan A's warm-up scenarios CONSTRUCTIBLE: warm-up is a
 * single-pass, startup-only task (spawn_stream_task, warm_stream.rs) — the
 * only way to make it run again against a chosen storage-peer state is to
 * reboot the doorway that runs it.
 *
 * DESTRUCTIVE: takes doorway "alpha" down for the length of one boot. Callers
 * must already be inside the `DESTRUCTIVE` gate.
 */
async function restartDoorwayAlpha(world: E2EWorld): Promise<void> {
  const url = doorwayUrl(world, 'alpha');
  const port = new URL(url).port || '80';
  const pid = doorwayPidForPort(port);

  await reexecProcess(pid, {
    graceMs: DOORWAY_RESTART_GRACE_MS,
    logPath: fixture().doorways?.['alpha']?.logPath,
  });

  await waitForDoorwayHealthy(url, DOORWAY_RESTART_HEALTH_BUDGET_MS);
  world.onCleanup(async () => {
    // Best-effort: if a later step in this scenario left the doorway down
    // (e.g. an assertion threw mid-restart), give it one more chance to come
    // back so the mesh isn't stranded for the next scenario. Never throws —
    // this is cleanup, not an assertion.
    try {
      await waitForDoorwayHealthy(url, 15_000);
    } catch {
      // eslint-disable-next-line no-console
      console.log(`  ⚠️  doorway "alpha" was not healthy at cleanup time after a restart drill`);
    }
  });
}

function stopPeersForConstruction(world: E2EWorld, names: readonly HouseholdPeerName[]): void {
  for (const name of names) {
    pausePeerProcess(name);
    world.onCleanup(async () => {
      resumePeerProcess(name);
      await Promise.resolve();
    });
  }
}

/**
 * Pause the given peers, restart doorway "alpha" so its warm-up pass runs
 * fresh against that peer state, then poll the doorway's own log (bounded)
 * until the pass has visibly finished — either every gated peer reached a
 * terminal outcome, or the whole-pass budget cut it off. Returns the
 * evidence for the caller's Given step to assert its own precondition
 * against; a caller whose precondition still cannot be established from that
 * evidence reports so honestly (see the per-scenario timing-incoherence
 * comments below) rather than loosening the assertion.
 */
const WARMUP_PASS_TERMINAL_MESSAGES = new Set([
  'Cache stream warm-up completed successfully',
  'Cache stream warm-up failed after max retries',
  'Upstream circuit opened; bailing retry ladder early',
]);

/**
 * The lines warm_stream.rs emits when a peer FAILS its stream — as distinct from
 * the terminal set above, which also contains the SUCCESS line. An evidence
 * assertion built on the terminal set counted a healthy peer's completion as a
 * failure; these three are the honest failure vocabulary:
 *   - "Cache stream warm-up failed, retrying"          (attempt < MAX_WARMUP_RETRIES)
 *   - "Cache stream warm-up failed after max retries"  (ladder exhausted)
 *   - "Upstream circuit opened; bailing retry ladder early" (breaker bailed the ladder)
 */
const WARMUP_FAILURE_MESSAGES = new Set([
  'Cache stream warm-up failed, retrying',
  'Cache stream warm-up failed after max retries',
  'Upstream circuit opened; bailing retry ladder early',
]);

/**
 * This pass's recorded warm-up FAILURES for one peer.
 *
 * Every warm_stream.rs failure line keys the peer on `storage_url` — the peer's
 * BASE url ("http://127.0.0.1:8091"), not the `/api/v1/cache/stream` url that
 * only the "Connected to cache stream" line carries in `url`. Matching a
 * suffixed stream url against `storage_url` therefore never matched anything, so
 * the evidence read as "no recorded failure" no matter what the peer did. Both
 * field spellings are accepted here and compared by peer authority, so the
 * fixture's `localhost` and the doorway's `127.0.0.1` name the same peer.
 */
function warmupFailureLinesFor(state: WarmupPassState, peerUrl: string): DoorwayLogLine[] {
  return state.lines.filter(l => {
    if (!WARMUP_FAILURE_MESSAGES.has(l.fields.message ?? '')) return false;
    const keyed = l.fields['storage_url'] ?? l.fields['url'];
    return typeof keyed === 'string' && sameUpstream(keyed, peerUrl);
  });
}

async function constructWarmupBoot(
  world: E2EWorld,
  stoppedPeers: readonly HouseholdPeerName[]
): Promise<WarmupPassState> {
  stopPeersForConstruction(world, stoppedPeers);
  await restartDoorwayAlpha(world);

  const deadline = Date.now() + 180_000;
  for (;;) {
    const state = await loadWarmupPassState(world);
    const budgetHit = state.lines.some(l => l.fields['event'] === 'budget-exhausted');
    const antiSelfPartition = state.lines.some(l => l.fields['event'] === 'anti-self-partition');
    const terminalCount = state.lines.filter(l =>
      WARMUP_PASS_TERMINAL_MESSAGES.has(l.fields.message ?? '')
    ).length;
    const passLooksDone =
      state.startedLine !== undefined &&
      (budgetHit ||
        antiSelfPartition ||
        terminalCount >= (state.peerCount ?? Number.POSITIVE_INFINITY));
    if (passLooksDone || Date.now() >= deadline) return state;
    await sleep(3_000);
  }
}

// ---------------------------------------------------------------------------
// /admin/self-healing surface (Plan C)
// ---------------------------------------------------------------------------

interface SelfHealingUpstream {
  endpoint: string;
  circuit: string;
  errorStreak: number;
  skipped: boolean;
}

interface SelfHealingView {
  autoPreset: unknown;
  admission: { maxInflight: number; available: number; shedTotal: number } | null;
  upstreams: SelfHealingUpstream[];
  projector: {
    lagSeconds: number | null;
    caughtUp: boolean | null;
    divergentAnchor: number | null;
  };
  peers: unknown[];
  render: { total: number; degenerateRate: number };
  warmup: { inProgress: boolean; attempts: number; completed: boolean; lastError: string | null };
  conductor: { connected: boolean; connectedWorkers: number; totalWorkers: number };
}

async function fetchSelfHealing(url: string): Promise<SelfHealingView> {
  const res = await getRaw(`${url}/admin/self-healing`, { timeoutMs: 10_000 });
  assert.equal(
    res.status,
    200,
    `GET ${url}/admin/self-healing returned ${res.status}: ${res.text}`
  );
  return JSON.parse(res.text) as SelfHealingView;
}

function findUpstream(
  view: SelfHealingView,
  endpointFragment: string
): SelfHealingUpstream | undefined {
  return view.upstreams.find(u => sameUpstream(u.endpoint, endpointFragment));
}

/** The host:port `/admin/self-healing`'s `upstreams[].endpoint` keys on. */
function endpointFragmentOf(url: string): string {
  return authorityOf(url);
}

// ===========================================================================
// Plan A — warm-up upstream self-protection (single startup pass)
// ===========================================================================

interface WarmupPassState {
  lines: DoorwayLogLine[];
  startedLine?: DoorwayLogLine;
  peerCount?: number;
  totalPeers?: number;
  connectedUrls: Set<string>;
  gatedUrls: string[];
}

const warmupStates = new WeakMap<E2EWorld, WarmupPassState>();

async function loadWarmupPassState(world: E2EWorld): Promise<WarmupPassState> {
  // THIS boot's lines only — the restart drill's whole point is that the pass
  // ran against the peer state we just constructed.
  const lines = linesSinceLastBoot(await readDoorwayLogLines('alpha'));
  const startedLine = lines.find(
    l => l.fields.message === 'Starting cache stream warm-up (health-gated)'
  );
  const peerCount = startedLine ? Number(startedLine.fields['peer_count']) : undefined;
  const totalPeers = startedLine ? Number(startedLine.fields['total_peers']) : undefined;
  const connectedUrls = new Set(
    lines
      .filter(l => l.fields.message === 'Connected to cache stream')
      .map(l => String(l.fields['url'] ?? ''))
      .filter(Boolean)
  );
  const configured = poolStorageUrls('alpha').map(u => `${u}/api/v1/cache/stream`);
  const gatedUrls = configured.filter(
    streamUrl => ![...connectedUrls].some(connected => sameUpstream(connected, streamUrl))
  );
  const state: WarmupPassState = {
    lines,
    startedLine,
    peerCount,
    totalPeers,
    connectedUrls,
    gatedUrls,
  };
  warmupStates.set(world, state);
  return state;
}

function warmupState(world: E2EWorld): WarmupPassState {
  const state = warmupStates.get(world);
  assert.ok(state, 'warm-up pass evidence was not read yet');
  return state;
}

/** Peer this file's Plan A scenarios break — chosen distinct from primaryPeerName('alpha') so
 * Plan A (warm-up breaker) and Plan B (storage-proxy breaker, F/G/H/I) construction never overlap. */
const WARMUP_BROKEN_PEER: HouseholdPeerName = 'jessica';

Given(
  'a storage upstream that has failed its warm-up stream 3 times this pass',
  { timeout: 240_000 },
  async function (this: E2EWorld) {
    if (!DESTRUCTIVE) {
      return skipHeldDestructive(
        `needs "${WARMUP_BROKEN_PEER}" paused BEFORE doorway boots, then a real doorway restart, so ` +
          'its warm-up pass encounters the failure live — constructible via SIGSTOP ' +
          `(requireFixturePeerPid("${WARMUP_BROKEN_PEER}")) + the /proc re-exec restart technique.`
      );
    }
    const state = await constructWarmupBoot(this, [WARMUP_BROKEN_PEER]);
    const peerUrl = requireFixtureStoragePeer(fixture(), WARMUP_BROKEN_PEER).url;
    const failureCount = warmupFailureLinesFor(state, peerUrl).length;
    assert.ok(
      failureCount >= 1,
      `restarted doorway with "${WARMUP_BROKEN_PEER}" paused, but this pass's log shows no recorded ` +
        `failure for ${peerUrl} (this boot's messages: ` +
        `${JSON.stringify(state.lines.map(l => l.fields.message))})`
    );
    if (failureCount < WARMUP_CIRCUIT_FAIL_THRESHOLD) {
      // TIMING FINDING, real and reproducible: WARMUP_CIRCUIT_FAIL_THRESHOLD(3) x
      // WARMUP_STREAM_TIMEOUT_SECS(45) = 135s of attempt time alone (before any
      // inter-attempt backoff) already exceeds WARMUP_TOTAL_BUDGET_SECS(75) — so
      // a peer that fails by TIMING OUT (what SIGSTOP produces: the kernel still
      // ACKs the SYN and queues it, so the client hangs for the full stream
      // timeout rather than getting an immediate connection-refused) usually
      // cannot complete its own 3-strike sequence before the whole-pass budget
      // guard cuts the pass off first. A peer that fails FAST (connection
      // refused) could complete the sequence within budget; this fixture only
      // offers the slow-timeout failure mode via SIGSTOP.
      // eslint-disable-next-line no-console
      console.log(
        `  ℹ️  TIMING FINDING: only ${failureCount}/${WARMUP_CIRCUIT_FAIL_THRESHOLD} failures ` +
          `recorded for "${WARMUP_BROKEN_PEER}" before this pass ended — see the code comment on ` +
          'this step for why a SIGSTOP-induced (timeout) failure rarely completes its own 3-strike ' +
          'sequence inside the 75s whole-pass budget.'
      );
    }
  }
);

When('the doorway starts the next warm-up pass', async function (this: E2EWorld) {
  // Warm-up is a single startup-only pass (spawn_stream_task, warm_stream.rs)
  // — there is no "next" pass to trigger from a running process. This step
  // re-reads the log so the Then steps assert against the freshest evidence,
  // honestly naming that no fresh pass was driven.
  await loadWarmupPassState(this);
});

Then(
  "that upstream's circuit is Open and it is skipped before the serial loop runs",
  function (this: E2EWorld) {
    const state = warmupState(this);
    // Precise criterion, deliberately NOT "absent from Connected to cache
    // stream": that looser proxy is satisfied by an ENTIRELY DIFFERENT
    // mechanism — the per-peer inner retry loop bailing out MID-pass after 3
    // failures (warm_stream.rs "Upstream circuit opened; bailing retry ladder
    // early") — which the SIGSTOP construction above actually exercises and
    // which would make this assertion a FALSE GREEN for the wrong reason. The
    // scenario's literal claim is the PRE-loop gate: gate_upstreams()
    // excludes the peer from the "Starting cache stream warm-up
    // (health-gated)" set BEFORE any peer is attempted at all — observable
    // only as peer_count < total_peers on that one line, captured before any
    // attempt/retry/bail line exists.
    assert.ok(
      state.peerCount !== undefined &&
        state.totalPeers !== undefined &&
        state.peerCount < state.totalPeers,
      `peer_count (${state.peerCount}) == total_peers (${state.totalPeers}) on the "Starting cache ` +
        'stream warm-up (health-gated)" line — gate_upstreams() did not pre-filter anything, even ' +
        'though the constructed peer went on to fail mid-pass. STRUCTURAL FINDING, with the ' +
        'arithmetic: gate_upstreams() reads warmup_state.health at the TOP of a pass, and a peer is ' +
        'only excluded once it has WARMUP_CIRCUIT_FAIL_THRESHOLD(3) recorded failures. Under ' +
        'SIGSTOP the kernel still ACKs the SYN, so each attempt fails by TIMING OUT at ' +
        'WARMUP_STREAM_TIMEOUT_SECS(45); attempt 2 starts at 45+WARMUP_RETRY_BASE_SECS(10)=55s and ' +
        'is cut mid-flight by WARMUP_TOTAL_BUDGET_SECS(75) — so exactly ONE failure is recorded per ' +
        'pass. The outer re-warm loop does re-enter (produced==0) after ' +
        'WARMUP_REWARM_POLL_SECS(30), so the breaker reaches 3 only on the ~4th pass, i.e. ~315s of ' +
        'the peer staying paused. The gate is therefore not unreachable in principle — it is ' +
        'unreachable inside one boot-drill window, and a fast-FAILING upstream (connection refused) ' +
        'would reach it in one pass. The mid-pass "bailing retry ladder early" line is a real but ' +
        "DIFFERENT mechanism, so it is not accepted as evidence for this scenario's pre-loop claim."
    );
  }
);

Then(
  'the doorway records the skip as a self-heal event for the elevate arm',
  function (this: E2EWorld) {
    const state = warmupState(this);
    const selfHealEvent = state.lines.find(l => typeof l.fields['event'] === 'string');
    assert.ok(
      selfHealEvent,
      'no structured self-heal `event` field found in the doorway log for this pass. GAP: ' +
        'warm_stream.rs only emits a named WarmupSelfHealEvent for "anti-self-partition" (every ' +
        'configured upstream skipped) and "budget-exhausted" (whole-pass timeout) — a SINGLE ' +
        "gated-out peer (this scenario's precondition) is not logged as its own self-heal event " +
        "today. This assertion is written to the scenario's literal claim and fails honestly " +
        'against the current implementation rather than being loosened to pass.'
    );
  }
);

/** Peers this scenario stalls — 2 of 3, "several" without needing every peer down. */
const WARMUP_SLOW_PEERS: readonly HouseholdPeerName[] = ['jessica', 'james'];

Given(
  'several storage upstreams are slow to answer their warm-up stream',
  { timeout: 240_000 },
  async function (this: E2EWorld) {
    if (!DESTRUCTIVE) {
      return skipHeldDestructive(
        `needs ${WARMUP_SLOW_PEERS.join(' and ')} paused before a real doorway restart so the ` +
          'serial warm-up pass genuinely stalls — constructible via SIGSTOP on both + the /proc ' +
          're-exec restart technique.'
      );
    }
    const state = await constructWarmupBoot(this, WARMUP_SLOW_PEERS);
    const stalledPeerUrls = WARMUP_SLOW_PEERS.map(
      name => requireFixtureStoragePeer(fixture(), name).url
    );
    const sawFailure = stalledPeerUrls.some(
      peerUrl => warmupFailureLinesFor(state, peerUrl).length > 0
    );
    assert.ok(
      sawFailure,
      `restarted doorway with ${WARMUP_SLOW_PEERS.join(', ')} paused, but the pass's log shows no ` +
        'failure evidence for either stalled peer'
    );
  }
);

When('the cumulative warm-up time reaches the total warm-up budget', function (this: E2EWorld) {
  const state = warmupState(this);
  const budgetLine = state.lines.find(
    l =>
      l.fields['event'] === 'budget-exhausted' ||
      (typeof l.fields.message === 'string' && l.fields.message.includes('total budget exhausted'))
  );
  assert.ok(
    budgetLine,
    `no "warm-up total budget exhausted" event in the doorway log — this boot's warm-up pass ` +
      `finished well under the ${WARMUP_TOTAL_BUDGET_SECS}s budget (the household mesh's storage ` +
      'peers answered quickly), so the budget-exhaustion precondition was never reached'
  );
});

Then('the doorway abandons the remaining streams for this pass', function (this: E2EWorld) {
  const state = warmupState(this);
  const budgetLine = state.lines.find(l => l.fields['event'] === 'budget-exhausted');
  assert.ok(budgetLine, 'no budget-exhausted self-heal event recorded for this pass');
  assert.match(
    String(budgetLine.fields.detail ?? ''),
    new RegExp(`exceeded ${WARMUP_TOTAL_BUDGET_SECS}s total budget`),
    `budget-exhausted event detail did not name the ${WARMUP_TOTAL_BUDGET_SECS}s constant`
  );
});

Then('the liveness probe answers throughout the warm-up pass', async function (this: E2EWorld) {
  // The one part of this scenario that IS live-observable right now,
  // independent of warm-up history: /health must answer 200 at this instant.
  const url = doorwayUrl(this, 'alpha');
  const health = await probeHealth(url);
  assert.equal(health.status, 200, `GET ${url}/health returned ${health.status}`);
  assert.equal(health.body.healthy, true, `GET ${url}/health body.healthy was not true`);
});

const WARMUP_ALL_PEERS: readonly HouseholdPeerName[] = ['matthew', 'jessica', 'james'];

Given(
  "every storage upstream's warm-up circuit is Open",
  { timeout: 240_000 },
  async function (this: E2EWorld) {
    if (!DESTRUCTIVE) {
      return skipHeldDestructive(
        'needs EVERY configured storage peer paused before a real doorway restart — constructible ' +
          'via SIGSTOP on all three household peers + the /proc re-exec restart technique.'
      );
    }
    const state = await constructWarmupBoot(this, WARMUP_ALL_PEERS);
    // Achievable postcondition: every configured peer failed to answer its
    // stream (no "Connected to cache stream" line). This is NOT the same
    // claim as "every peer's warm-up circuit-breaker recorded 3 failures and
    // transitioned to Open" — see the timing-incoherence finding on the Then
    // step below, which is why that stronger claim is checked there instead
    // of gating the precondition on it.
    assert.equal(
      state.connectedUrls.size,
      0,
      `restarted doorway with every household peer paused, but ${state.connectedUrls.size} of ` +
        `${poolStorageUrls('alpha').length} still logged "Connected to cache stream" — the ` +
        'construction did not actually make every upstream unreachable'
    );
  }
);

When('the doorway computes the warm-up upstream set', async function (this: E2EWorld) {
  await loadWarmupPassState(this);
});

Then('the set is non-empty and contains every configured upstream', function (this: E2EWorld) {
  const state = warmupState(this);
  const totalPeers = state.totalPeers ?? poolStorageUrls('alpha').length;
  const antiSelfPartitionFired = state.lines.some(l => l.fields['event'] === 'anti-self-partition');

  assert.ok(
    state.peerCount !== undefined && state.peerCount > 0,
    'the gated warm-up set was empty — gate_upstreams() must never self-partition the node ' +
      '(warm_stream.rs `gate_never_self_partitions_to_empty`)'
  );

  if (antiSelfPartitionFired) {
    // gate_upstreams() genuinely took the all-skipped branch this pass: per
    // its own code (and its own unit test gate_never_self_partitions_to_empty)
    // that branch returns vec![least_bad] — a ONE-element set — never "every
    // configured upstream". The scenario's claim is directly contradicted.
    assert.equal(
      state.peerCount,
      totalPeers,
      `MISMATCH between this scenario's claim and gate_upstreams()'s real all-skipped behaviour: ` +
        `it returned a ${state.peerCount}-element set (expected 1, the least-bad singleton — pinned ` +
        `by gate_never_self_partitions_to_empty), never "every configured upstream" (${totalPeers}).`
    );
  } else {
    // STRUCTURAL FINDING (confirmed by this constructed run, not merely
    // reasoned from source): gate_upstreams() is called EXACTLY ONCE per
    // doorway process, as the FIRST-EVER read of warmup_state.health — so its
    // breaker map is ALWAYS empty at that call, on every single-pass boot,
    // regardless of which storage peers were paused beforehand. It therefore
    // ALWAYS takes the "admitted" branch (returns the full set unfiltered)
    // and can NEVER reach the all-skipped/least-bad path this scenario means
    // to exercise — peer_count == total_peers here reflects that always-taken
    // "nothing has failed yet" branch, not a correctly-handled "every upstream
    // open" case. Asserting the scenario's literal claim as if it had been
    // meaningfully tested would be theatre; fail honestly instead.
    assert.fail(
      `gate_upstreams() never reached its all-skipped branch (no anti-self-partition event) even ` +
        `though every configured peer was paused before this restart. peer_count=` +
        `${state.peerCount} of total_peers=${totalPeers} reflects the always-taken branch. ` +
        "STRUCTURAL FINDING, sharper than the sibling scenario's: the all-skipped branch needs " +
        "EVERY peer's circuit open, but the warm-up pass is SERIAL and bounded by " +
        'WARMUP_TOTAL_BUDGET_SECS(75) while a SIGSTOPped peer costs WARMUP_STREAM_TIMEOUT_SECS(45) ' +
        'per attempt — so a pass never gets PAST THE FIRST PEER. Peers 2..N are never attempted, ' +
        'never record an outcome, and can never open. No amount of waiting reaches the all-open ' +
        'state through this fixture; it needs upstreams that fail FAST (connection refused, not ' +
        'SIGSTOP) so a single pass can charge every peer.'
    );
  }
});

Then('the doorway records an "anti-self-partition" self-heal event', function (this: E2EWorld) {
  const state = warmupState(this);
  const event = state.lines.find(l => l.fields['event'] === 'anti-self-partition');
  assert.ok(
    event,
    'no anti-self-partition self-heal event recorded in the doorway log even though every ' +
      'configured peer was paused before this restart — see the structural finding on the ' +
      'preceding Then step: gate_upstreams() cannot reach this branch on a fresh single-pass boot, ' +
      'because it is the FIRST read of an always-empty breaker map.'
  );
  assert.match(
    String(event?.fields.detail ?? ''),
    /circuit-open; proceeding least-bad/,
    'anti-self-partition event detail did not carry the expected "least-bad" wording'
  );
});

// ===========================================================================
// Plan B — bilateral inbound/outbound flow control
// ===========================================================================

// ── Inbound admission (doorway's own semaphore) ─────────────────────────────

interface InboundSaturationRig {
  doorwayUrl: string;
  stop: () => void;
  settled: Promise<unknown>;
  saturated: boolean;
  probeStatus: number;
  probeBody: string;
}

const inboundRigs = new WeakMap<E2EWorld, InboundSaturationRig>();

/**
 * A read heavy enough that serving it HOLDS an admission permit for a
 * measurable window.
 *
 * The suite used to saturate with `?limit=1`. Measured on the mesh 2026-08-22:
 * 576 concurrent `limit=1` reads all answered 200 and the probe was admitted —
 * the doorway answers a one-row read in ~2ms, so the client can never keep 512
 * of them simultaneously in flight and the ceiling is never reached. Saturation
 * is a function of permit HOLD TIME, not request count; the same 576 against
 * `limit=2000` sheds ~90 with 503. This is the honest construction of "the
 * permits are exhausted" — real load the node genuinely cannot absorb at once,
 * not a bigger count of work it finds trivial.
 */
const SATURATING_READ_PATH = '/db/content?limit=100';

/**
 * How far past the ceiling the load runs.
 *
 * Overshooting by a handful (ceiling + 64) exhausts the pool but only ~11% of
 * arrivals are shed at steady state, so a SINGLE probe is admitted ~9 times out
 * of 10 and the scenario reads "not saturated" against a node that is. Measured
 * on the mesh 2026-08-22 at 3x the ceiling: 66% shed, the whole burst drains in
 * ~2.7s, and /admin/self-healing answers in 2ms the moment the load stops — so
 * this is both the RELIABLE construction and the gentle one. (3x of a
 * `limit=100` read is far lighter on the peer than 1.1x of a `limit=2000` one,
 * which took ~10s to drain and left the following scenario timing out on the
 * diagnostic endpoint.)
 */
const SATURATION_OVERSHOOT = 3;

interface SustainedLoad {
  stop: () => void;
  settled: Promise<unknown>;
}

/**
 * Hold `concurrency` requests in flight until stopped — as opposed to firing
 * one burst and hoping the later steps land inside its shadow. A one-shot burst
 * drains in seconds, so the scenario's own When ("a non-exempt request arrives")
 * measured an UNSATURATED doorway and the Then asserted a shed that had already
 * stopped happening. The load is stopped in scenario cleanup, so the window is
 * exactly the scenario, never longer.
 */
function sustainedLoad(url: string, concurrency: number): SustainedLoad {
  let running = true;
  const workers = Array.from({ length: concurrency }, async () => {
    while (running) {
      await getRaw(url, { timeoutMs: 10_000 }).catch(() => null);
    }
  });
  return {
    stop: () => {
      running = false;
    },
    settled: Promise.allSettled(workers),
  };
}

/**
 * Probe until the node sheds (503) or the budget runs out; returns the last
 * answer.
 *
 * A probe that THROWS is itself a verdict, not an accident: a node that queues
 * instead of shedding leaves the caller hanging until the client timeout, which
 * is exactly the "blocking accept loop" failure the shed exists to prevent. It
 * is captured as status 0 carrying the transport error so the caller's assertion
 * can name what actually happened rather than dying on an unhandled rejection.
 */
async function probeUntilShed(url: string, budgetMs: number): Promise<RawHttpResponse> {
  const deadline = Date.now() + budgetMs;
  const once = async (): Promise<RawHttpResponse> => {
    try {
      return await getRawWithHeaders(url, { timeoutMs: 10_000 });
    } catch (error) {
      return {
        status: 0,
        text: `probe never answered (queued, not shed): ${String(error)}`,
        headers: {},
      };
    }
  };
  let last = await once();
  while (last.status !== 503 && Date.now() < deadline) {
    await sleep(100);
    last = await once();
  }
  return last;
}

/**
 * Drive the doorway's own read-admission semaphore (DEFAULT_MAX_INFLIGHT_READ,
 * 512) to exhaustion and KEEP it there for the scenario.
 * DESTRUCTIVE/HEAVY: real concurrent load against the live mesh — hold for
 * explicit operator go before ever executing.
 */
async function saturateInboundAdmission(
  world: E2EWorld,
  url: string
): Promise<InboundSaturationRig> {
  const concurrency = Number(
    process.env['E2E_INBOUND_SATURATION_BURST'] ?? DOORWAY_MAX_INFLIGHT_READ * SATURATION_OVERSHOOT
  );
  const load = sustainedLoad(`${url}${SATURATING_READ_PATH}`, concurrency);
  const rig: InboundSaturationRig = {
    doorwayUrl: url,
    stop: load.stop,
    settled: load.settled,
    saturated: false,
    probeStatus: 0,
    probeBody: '',
  };
  inboundRigs.set(world, rig);
  world.onCleanup(async () => {
    load.stop();
    await load.settled;
  });
  // Let the load actually land in flight, then probe until it sheds.
  await sleep(250);
  const probe = await probeUntilShed(`${url}${SATURATING_READ_PATH}`, 10_000);
  rig.saturated = probe.status === 503;
  rig.probeStatus = probe.status;
  rig.probeBody = probe.text;
  return rig;
}

function inboundRig(world: E2EWorld): InboundSaturationRig {
  const rig = inboundRigs.get(world);
  assert.ok(rig, 'inbound admission was not saturated yet');
  return rig;
}

Given("the doorway's inbound permits are exhausted", async function (this: E2EWorld) {
  if (!DESTRUCTIVE) {
    return skipHeldDestructive(
      `holds ${DOORWAY_MAX_INFLIGHT_READ * SATURATION_OVERSHOOT} concurrent reads against the live ` +
        'doorway to saturate its read-admission pool.'
    );
  }
  const url = doorwayUrl(this, 'alpha');
  const rig = await saturateInboundAdmission(this, url);
  assert.equal(
    rig.probeStatus,
    503,
    `expected 503 after saturating the doorway's read-admission pool (ceiling ` +
      `${DOORWAY_MAX_INFLIGHT_READ}) with a concurrent burst, got ${rig.probeStatus}: ${rig.probeBody}`
  );
});

interface NonExemptRequestState {
  response: RawHttpResponse;
  elapsedMs: number;
}

const nonExemptRequests = new WeakMap<E2EWorld, NonExemptRequestState>();

When('a non-exempt request arrives', { timeout: 30_000 }, async function (this: E2EWorld) {
  const rig = inboundRig(this);
  const start = Date.now();
  const response = await getRawWithHeaders(`${rig.doorwayUrl}${CHEAP_READ_PATH}`, {
    timeoutMs: 10_000,
  });
  nonExemptRequests.set(this, { response, elapsedMs: Date.now() - start });
});

function nonExemptRequest(world: E2EWorld): NonExemptRequestState {
  const state = nonExemptRequests.get(world);
  assert.ok(state, 'no non-exempt request was issued yet');
  return state;
}

Then('the request is shed with HTTP 503 and a Retry-After header', function (this: E2EWorld) {
  const { response } = nonExemptRequest(this);
  assert.equal(response.status, 503, `expected 503, got ${response.status}: ${response.text}`);
  assert.equal(
    response.headers['retry-after'],
    String(DOORWAY_ADMISSION_RETRY_AFTER_SECS),
    `expected Retry-After: ${DOORWAY_ADMISSION_RETRY_AFTER_SECS}, got ` +
      `${response.headers['retry-after'] ?? '<absent>'}`
  );
});

Then('the body advertises status {string}', function (this: E2EWorld, expectedStatus: string) {
  const { response } = nonExemptRequest(this);
  const body = JSON.parse(response.text) as Record<string, unknown>;
  assert.equal(
    body['status'],
    expectedStatus,
    `body.status was "${body['status']}": ${response.text}`
  );
});

Then(
  'the doorway never blocks the caller waiting for an inbound permit',
  function (this: E2EWorld) {
    const { elapsedMs } = nonExemptRequest(this);
    // try_acquire_owned never awaits a permit — a queued design could take
    // arbitrarily long; a shed answers near-instantly. 5s is generous margin
    // over local-loopback latency while still catching a regression to
    // acquire().await (which would hang until a burst request completed).
    assert.ok(
      elapsedMs < 5_000,
      `non-exempt request took ${elapsedMs}ms to answer while permits were exhausted — a shed ` +
        'must return immediately (try_acquire_owned), never block waiting for a permit to free up'
    );
  }
);

Given(
  'the doorway is shedding inbound requests because its permits are exhausted',
  async function (this: E2EWorld) {
    if (!DESTRUCTIVE) {
      return skipHeldDestructive(
        `holds ${DOORWAY_MAX_INFLIGHT_READ * SATURATION_OVERSHOOT} concurrent reads against the live ` +
          'doorway to saturate its read-admission pool.'
      );
    }
    const url = doorwayUrl(this, 'alpha');
    const rig = await saturateInboundAdmission(this, url);
    assert.equal(
      rig.probeStatus,
      503,
      `expected 503 after saturating inbound admission, got ${rig.probeStatus}: ${rig.probeBody}`
    );
  }
);

/**
 * A request carrying real `Connection: Upgrade` headers.
 *
 * undici REFUSES to send a `connection` header at all (InvalidArgumentError:
 * "invalid connection header"), so the shared `getRawWithHeaders` cannot express
 * this probe — and the doorway's exemption is keyed on exactly those headers
 * (`admission_exempt(path, is_upgrade)`). node:http will send them, so the
 * upgrade probe drops to the lower-level client rather than pretending a request
 * without the header tests the same thing.
 */
async function rawUpgradeRequest(url: string, timeoutMs: number): Promise<RawHttpResponse> {
  const target = new URL(url);
  return new Promise<RawHttpResponse>(resolve => {
    const req = httpRequest(
      {
        hostname: target.hostname,
        port: target.port,
        path: `${target.pathname}${target.search}`,
        method: 'GET',
        agent: false,
        headers: {
          Connection: 'Upgrade',
          Upgrade: 'websocket',
          'Sec-WebSocket-Version': '13',
          'Sec-WebSocket-Key': 'ZTJlLXNlbGYtaGVhbGluZy1wcm9iZQ==',
        },
      },
      response => {
        const chunks: Buffer[] = [];
        response.on('data', (chunk: Buffer) => chunks.push(chunk));
        response.on('end', () => {
          const headers: Record<string, string | undefined> = {};
          for (const [key, value] of Object.entries(response.headers)) {
            headers[key.toLowerCase()] = Array.isArray(value) ? value.join(', ') : value;
          }
          resolve({
            status: response.statusCode ?? 0,
            text: Buffer.concat(chunks).toString('utf8'),
            headers,
          });
        });
      }
    );
    req.setTimeout(timeoutMs, () =>
      req.destroy(new Error(`upgrade probe timed out after ${timeoutMs}ms`))
    );
    req.on('upgrade', (response, socket) => {
      socket.destroy();
      resolve({ status: response.statusCode ?? 101, text: '', headers: {} });
    });
    req.on('error', error => {
      resolve({ status: 0, text: `upgrade probe failed: ${String(error)}`, headers: {} });
    });
    req.end();
  });
}

interface ExemptProbesState {
  liveness: RawHttpResponse;
  webSocketUpgrade: RawHttpResponse;
}

const exemptProbes = new WeakMap<E2EWorld, ExemptProbesState>();

When(
  'the liveness probe and a WebSocket upgrade arrive',
  { timeout: 30_000 },
  async function (this: E2EWorld) {
    const rig = inboundRig(this);
    const [liveness, webSocketUpgrade] = await Promise.all([
      getRawWithHeaders(`${rig.doorwayUrl}/health`, { timeoutMs: 10_000 }),
      // admission_exempt(path, is_upgrade) returns true for ANY path once the
      // request carries upgrade headers (server/http.rs) — targeting the same
      // normally-gated read path isolates the upgrade exemption from the
      // liveness-path exemption tested above.
      rawUpgradeRequest(`${rig.doorwayUrl}${CHEAP_READ_PATH}`, 10_000),
    ]);
    exemptProbes.set(this, { liveness, webSocketUpgrade });
  }
);

function exemptProbeState(world: E2EWorld): ExemptProbesState {
  const state = exemptProbes.get(world);
  assert.ok(state, 'exempt probes were not issued yet');
  return state;
}

/**
 * The non-exempt request this scenario issued, or — for a scenario whose
 * non-exempt request IS the saturation rig's probe — that probe.
 */
function nonExemptRequestOrRigProbe(world: E2EWorld): RawHttpResponse {
  const explicit = nonExemptRequests.get(world);
  if (explicit) return explicit.response;
  const rig = inboundRig(world);
  return { status: rig.probeStatus, text: rig.probeBody, headers: {} };
}

function isAdmissionShed(response: RawHttpResponse): boolean {
  if (response.status !== 503) return false;
  try {
    return (JSON.parse(response.text) as Record<string, unknown>)['status'] === 'catching-up';
  } catch {
    return false;
  }
}

Then('both are served without consuming an inbound permit', function (this: E2EWorld) {
  const { liveness, webSocketUpgrade } = exemptProbeState(this);
  assert.equal(liveness.status, 200, `GET /health returned ${liveness.status} while saturated`);
  assert.ok(
    !isAdmissionShed(webSocketUpgrade),
    `the upgrade-headers request was admission-shed (503 catching-up) while saturated — ` +
      `admission_exempt(path, is_upgrade=true) must exempt ANY path, got ${webSocketUpgrade.status}: ` +
      webSocketUpgrade.text
  );
});

Then('only non-exempt requests receive the 503 catching-up shed', function (this: E2EWorld) {
  // This scenario has no "a non-exempt request arrives" When of its own — the
  // saturation rig's OWN probe is the non-exempt request, issued against the
  // same saturated node. (Reaching for the sibling scenario's WeakMap entry read
  // as "no non-exempt request was issued yet": each scenario gets a fresh
  // world.)
  const nonExempt = nonExemptRequestOrRigProbe(this);
  const { liveness, webSocketUpgrade } = exemptProbeState(this);
  assert.ok(
    isAdmissionShed(nonExempt),
    `the non-exempt probe was NOT shed while saturated (status ${nonExempt.status}) — the ` +
      'saturation window may have already drained; re-run closer to the burst'
  );
  assert.ok(!isAdmissionShed(liveness), 'liveness was shed — it must be exempt');
  assert.ok(
    !isAdmissionShed(webSocketUpgrade),
    'the WebSocket upgrade was shed — it must be exempt'
  );
});

// ── Outbound circuit breaker (storage-proxy UpstreamBreakers) ───────────────

interface BreakerTripState {
  peerName: HouseholdPeerName;
  storageUrl: string;
  endpointFragment: string;
  /** Failing proxied requests this drill has issued — the ceiling on honest outcome records. */
  issuedFailures: number;
}

const breakerTrips = new WeakMap<E2EWorld, BreakerTripState>();

/** Every household storage peer — the whole pool this doorway can proxy to. */
const ALL_HOUSEHOLD_PEERS: readonly HouseholdPeerName[] = ['matthew', 'jessica', 'james'];

/**
 * Pause EVERY household storage peer and drive UPSTREAM_CIRCUIT_FAIL_THRESHOLD
 * proxied requests through doorway so the storage-proxy breaker opens for real
 * (connect/read-timeout failures, STORAGE_PROXY_CONNECT_TIMEOUT_SECS=3 /
 * STORAGE_PROXY_REQUEST_TIMEOUT_SECS=12 per attempt).
 *
 * WHY THE WHOLE POOL, and why the keyed upstream is DISCOVERED rather than
 * assumed. This step used to pause the doorway's PRIMARY peer and assert the
 * breaker opened for that peer's endpoint. Measured on the household mesh
 * 2026-08-22, neither half held: with the primary (8090) paused, every probe
 * answered in ~10-140ms and no outcome was recorded against it at all, because
 * the doorway's target for this route is a DIFFERENT single peer (8091) —
 * routes/storage_proxy.rs is single-target dispatch, so pausing a peer the route
 * does not target is invisible to it. With the whole pool paused the same probes
 * take the full 12s, the breaker opens after 3, and a 4th is shed in ~12ms.
 *
 * The endpoint the breaker actually keyed is then READ BACK from
 * /admin/self-healing rather than assumed from the fixture's primary — which
 * peer a given route targets is the doorway's routing decision, not this step's.
 *
 * DESTRUCTIVE: pauses every live household storage peer for the length of the
 * trip — hold for explicit operator go.
 */
async function tripStorageBreaker(world: E2EWorld, doorway: string): Promise<BreakerTripState> {
  for (const name of ALL_HOUSEHOLD_PEERS) {
    pausePeerProcess(name);
    world.onCleanup(async () => {
      resumePeerProcess(name);
      await Promise.resolve();
    });
  }

  // CONCURRENTLY, not one after another. The breaker opens on failThreshold
  // failures WITHIN failWindowSeconds (3 within 20s — /admin/self-healing
  // publishes both under upstreamPolicy). A stalled peer costs
  // STORAGE_PROXY_REQUEST_TIMEOUT_SECS(12) per attempt, so three SERIAL probes
  // land their failures ~12s apart and the third sees only two inside the
  // window — the trip was timing-marginal and depended on unrelated background
  // traffic to top it up. Fired together they all fail at ~t+12s, inside one
  // window, which is what "returned errors past the upstream failure threshold"
  // actually means.
  await Promise.all(
    Array.from({ length: UPSTREAM_CIRCUIT_FAIL_THRESHOLD }, async () =>
      getRaw(`${doorway}${UNKNOWN_CONTENT_PATH}`, { timeoutMs: 20_000 }).catch(() => undefined)
    )
  );

  // Which upstream did the doorway actually charge for these failures?
  const opened = await fetchSelfHealing(doorway)
    .then(view => view.upstreams.find(u => u.circuit === 'open'))
    .catch(() => undefined);
  const state: BreakerTripState = {
    peerName: primaryPeerName('alpha'),
    storageUrl: opened?.endpoint ?? primaryStorageUrl('alpha'),
    endpointFragment: endpointFragmentOf(opened?.endpoint ?? primaryStorageUrl('alpha')),
    issuedFailures: UPSTREAM_CIRCUIT_FAIL_THRESHOLD,
  };
  breakerTrips.set(world, state);
  return state;
}

function breakerTripState(world: E2EWorld): BreakerTripState {
  const state = breakerTrips.get(world);
  assert.ok(state, 'the storage-proxy breaker was not tripped yet');
  return state;
}

Given(
  'the storage upstream has returned errors past the upstream failure threshold',
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    if (!DESTRUCTIVE) {
      return skipHeldDestructive(
        'pauses EVERY household storage peer (SIGSTOP by exact pid) so the route the doorway ' +
          "actually targets genuinely fails and the doorway's storage-proxy breaker trips for real."
      );
    }
    const url = doorwayUrl(this, 'alpha');
    const state = await tripStorageBreaker(this, url);
    const view = await fetchSelfHealing(url);
    const upstream = findUpstream(view, state.endpointFragment);
    assert.ok(
      upstream,
      `/admin/self-healing carries no upstreams[] entry for ${state.endpointFragment} — ` +
        `known endpoints: ${view.upstreams.map(u => u.endpoint).join(', ') || '<none>'}`
    );
    assert.equal(
      upstream.circuit,
      'open',
      `breaker for ${state.endpointFragment} is "${upstream.circuit}", expected "open" after ` +
        `${UPSTREAM_CIRCUIT_FAIL_THRESHOLD} failed proxied requests`
    );
  }
);

interface ProxiedRequestState {
  response: RawHttpResponse;
  elapsedMs: number;
}

const proxiedRequests = new WeakMap<E2EWorld, ProxiedRequestState>();

When(
  'a request that would proxy to storage arrives',
  { timeout: 30_000 },
  async function (this: E2EWorld) {
    const url = doorwayUrl(this, 'alpha');
    const start = Date.now();
    const response = await getRawWithHeaders(`${url}${UNKNOWN_CONTENT_PATH}`, {
      timeoutMs: 15_000,
    });
    proxiedRequests.set(this, { response, elapsedMs: Date.now() - start });
  }
);

function proxiedRequest(world: E2EWorld): ProxiedRequestState {
  const state = proxiedRequests.get(world);
  assert.ok(state, 'no proxied request was issued yet');
  return state;
}

Then(
  'the doorway sheds it with 503 catching-up without calling storage',
  function (this: E2EWorld) {
    const { response, elapsedMs } = proxiedRequest(this);
    assert.equal(response.status, 503, `expected 503, got ${response.status}: ${response.text}`);
    const body = JSON.parse(response.text) as Record<string, unknown>;
    assert.equal(body['status'], 'catching-up', `body.status was "${body['status']}"`);
    assert.equal(
      body['cause'],
      'upstream',
      `body.cause was "${body['cause']}", expected "upstream"`
    );
    // A breaker-open shed never calls storage: it must answer far faster than
    // the 3s connect timeout a real (even doomed) attempt would burn.
    assert.ok(
      elapsedMs < 2_000,
      `shed took ${elapsedMs}ms — a circuit-open shed must never attempt the upstream connect ` +
        '(STORAGE_PROXY_CONNECT_TIMEOUT_SECS=3)'
    );
  }
);

Then('the shed advertises a Retry-After header', function (this: E2EWorld) {
  const { response } = proxiedRequest(this);
  assert.ok(
    response.headers['retry-after'] !== undefined,
    `no Retry-After header on the shed response (headers: ${JSON.stringify(response.headers)})`
  );
});

// ── The body-phase double-count regression (closest real substitute) ───────

Given(
  'a storage upstream that returns 200 headers then stalls before the body completes',
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    // GAP (see file header #3): this household-mesh fixture offers only real
    // storage peers via SIGSTOP, which reproduces a connect/read FAILURE, not
    // a 200-headers-then-stall body failure — no controllable mock upstream
    // is wired into this harness. This step exercises the same
    // record-once-per-terminal-path breaker discipline via that closest real
    // substitute (repeated connect failures) and names the gap rather than
    // silently downgrading the scenario's claim.
    if (!DESTRUCTIVE) {
      return skipHeldDestructive(
        'pauses EVERY household storage peer to trip the storage-proxy breaker for real ' +
          '(see the GAP comment above: the exact body-phase-stall trigger needs a mock upstream this ' +
          'fixture does not have — the constructed condition is the closest real substitute).'
      );
    }
    await tripStorageBreaker(this, doorwayUrl(this, 'alpha'));
  }
);

When(
  'that failure mode repeats past the upstream failure threshold',
  async function (this: E2EWorld) {
    // tripStorageBreaker already drove UPSTREAM_CIRCUIT_FAIL_THRESHOLD attempts
    // in the Given step (the pool is still paused) — re-affirm via one more
    // real proxied attempt so this step itself performs a live probe. It is
    // COUNTED, because the Then's anti-double-count claim is about outcomes per
    // request, not about a fixed number.
    const url = doorwayUrl(this, 'alpha');
    const before = await getRaw(`${url}${UNKNOWN_CONTENT_PATH}`, { timeoutMs: 15_000 }).catch(
      () => undefined
    );
    // A shed (503 catching-up, answered in milliseconds) means the breaker was
    // ALREADY open and this probe never reached the upstream — so it records no
    // outcome and must not be counted.
    if (before !== undefined && before.status !== 503) {
      breakerTripState(this).issuedFailures += 1;
    }
  }
);

Then('the breaker opens for that upstream', async function (this: E2EWorld) {
  const state = breakerTripState(this);
  const url = doorwayUrl(this, 'alpha');
  const view = await fetchSelfHealing(url);
  const upstream = findUpstream(view, state.endpointFragment);
  assert.ok(upstream, `no upstreams[] entry for ${state.endpointFragment}`);
  assert.equal(
    upstream.circuit,
    'open',
    `breaker circuit is "${upstream.circuit}", expected "open"`
  );
  // "records exactly one outcome per terminal path": the streak must be at
  // least the threshold (or the circuit could not be open) and at most the
  // number of failing requests this drill actually issued. Pinning it to the
  // THRESHOLD was wrong in both directions — the scenario's own When issues a
  // further failing probe, so 4 records for 4 failed requests is the correct
  // one-outcome-per-path behaviour, not a double count.
  assert.ok(
    upstream.errorStreak >= UPSTREAM_CIRCUIT_FAIL_THRESHOLD,
    `error_streak is ${upstream.errorStreak}, below the ${UPSTREAM_CIRCUIT_FAIL_THRESHOLD} that ` +
      'must have been recorded for the circuit to be open'
  );
  assert.ok(
    upstream.errorStreak <= state.issuedFailures,
    `error_streak is ${upstream.errorStreak} for ${state.issuedFailures} failing request(s) — a ` +
      'higher count means an outcome was recorded more than once for the same terminal path'
  );
});

Then(
  'subsequent requests are shed catching-up rather than retried into the stall',
  { timeout: 20_000 },
  async function (this: E2EWorld) {
    const url = doorwayUrl(this, 'alpha');
    const start = Date.now();
    const response = await getRaw(`${url}${UNKNOWN_CONTENT_PATH}`, { timeoutMs: 10_000 });
    const elapsedMs = Date.now() - start;
    assert.equal(response.status, 503, `expected 503, got ${response.status}: ${response.text}`);
    const body = JSON.parse(response.text) as Record<string, unknown>;
    assert.equal(body['status'], 'catching-up');
    assert.ok(
      elapsedMs < 2_000,
      `shed took ${elapsedMs}ms — a request "retried into the stall" would burn the full request ` +
        'timeout instead of shedding immediately'
    );
  }
);

// ── Half-open trial + client-error-does-not-consume-without-outcome ────────

Given(
  "the storage upstream's breaker is half-open and allowing one trial call",
  { timeout: 120_000 },
  async function (this: E2EWorld) {
    if (!DESTRUCTIVE) {
      return skipHeldDestructive(
        'trips the storage-proxy breaker for real (pause the primary peer), resumes it, then waits ' +
          `the real ${UPSTREAM_CIRCUIT_COOLDOWN_SECS}s wall-clock cooldown for a half-open trial.`
      );
    }
    const url = doorwayUrl(this, 'alpha');
    await tripStorageBreaker(this, url);
    // Resume EVERY peer so the wall-clock cooldown genuinely admits a healthy
    // trial rather than another failure — the storage-proxy breaker is a
    // REAL-clock breaker (unlike Plan A's frozen-tick warm-up breaker), so this
    // wait is real time, not simulated. (The trip pauses the whole pool, so
    // resuming only the primary would leave the peer this route actually
    // targets still stalled, and the "healthy trial" would be another failure.)
    for (const name of ALL_HOUSEHOLD_PEERS) resumePeerProcess(name);
    await sleep((UPSTREAM_CIRCUIT_COOLDOWN_SECS + 2) * 1_000);
  }
);

interface TrialOutcomeState {
  response: RawHttpResponse;
}

const trialOutcomes = new WeakMap<E2EWorld, TrialOutcomeState>();

When(
  'the trial call returns a client-error status from a healthy upstream',
  async function (this: E2EWorld) {
    const url = doorwayUrl(this, 'alpha');
    // Storage answers 404 for an unknown content id — ProxyOutcome::classify
    // maps 4xx to Neutral, which the trial records as a SUCCESS
    // (storage_proxy.rs: `trial.record(ProxyOutcome::classify(status) != Failure)`).
    const response = await getRawWithHeaders(`${url}${UNKNOWN_CONTENT_PATH}`, {
      timeoutMs: 15_000,
    });
    trialOutcomes.set(this, { response });
  }
);

function trialOutcome(world: E2EWorld): TrialOutcomeState {
  const state = trialOutcomes.get(world);
  assert.ok(state, 'no trial call was issued yet');
  return state;
}

Then(
  'the trial is recorded as a success, not consumed without an outcome',
  async function (this: E2EWorld) {
    const { response } = trialOutcome(this);
    assert.ok(
      response.status === 404 || response.status < 500,
      `expected the trial to reach the now-healthy storage peer and get a real client-class ` +
        `answer, got ${response.status}: ${response.text}`
    );
    const state = breakerTripState(this);
    const url = doorwayUrl(this, 'alpha');
    const view = await fetchSelfHealing(url);
    const upstream = findUpstream(view, state.endpointFragment);
    assert.ok(upstream, `no upstreams[] entry for ${state.endpointFragment}`);
    assert.notEqual(
      upstream.circuit,
      'half-open',
      'breaker is still half-open with no outcome recorded — a consumed trial with no recorded ' +
        'outcome latches the circuit half-open forever (upstream_health.rs ' +
        'halfopen_without_record_deadlocks_forever)'
    );
  }
);

Then(
  'the breaker is eligible to close on the next healthy response',
  async function (this: E2EWorld) {
    const state = breakerTripState(this);
    const url = doorwayUrl(this, 'alpha');
    // A real content read against the now-resumed peer should succeed cleanly.
    const response = await getRaw(`${url}/health`, { timeoutMs: 10_000 });
    assert.equal(response.status, 200, `GET /health returned ${response.status} post-recovery`);
    const view = await fetchSelfHealing(url);
    const upstream = findUpstream(view, state.endpointFragment);
    assert.ok(upstream, `no upstreams[] entry for ${state.endpointFragment}`);
    assert.notEqual(
      upstream.circuit,
      'open',
      'breaker is still open after a recorded-success trial — it should be closed or ' +
        'half-open-and-eligible, never latched open'
    );
  }
);

// ── Storage breaker cooldown recovery (@wip scenario — implemented for real) ─

Given(
  "the storage upstream's breaker is Open",
  { timeout: 90_000 },
  async function (this: E2EWorld) {
    if (!DESTRUCTIVE) {
      return skipHeldDestructive(
        'trips the storage-proxy breaker for real (pause the primary peer).'
      );
    }
    await tripStorageBreaker(this, doorwayUrl(this, 'alpha'));
  }
);

Given('the upstream has recovered and now answers normally', function (this: E2EWorld) {
  const state = breakerTripState(this);
  resumePeerProcess(state.peerName);
});

When(
  "the breaker's cooldown window elapses and a request arrives",
  { timeout: (UPSTREAM_CIRCUIT_COOLDOWN_SECS + 20) * 1_000 },
  async function (this: E2EWorld) {
    await sleep((UPSTREAM_CIRCUIT_COOLDOWN_SECS + 2) * 1_000);
    const url = doorwayUrl(this, 'alpha');
    const response = await getRawWithHeaders(`${url}${UNKNOWN_CONTENT_PATH}`, {
      timeoutMs: 15_000,
    });
    trialOutcomes.set(this, { response });
  }
);

Then('the doorway admits a single half-open trial to the upstream', function (this: E2EWorld) {
  const { response } = trialOutcome(this);
  assert.ok(
    response.status < 500,
    `expected the admitted trial to reach the (now-healthy) upstream rather than being shed, ` +
      `got ${response.status}: ${response.text}`
  );
});

Then(
  'the successful trial closes the breaker and normal proxying resumes',
  async function (this: E2EWorld) {
    const state = breakerTripState(this);
    const url = doorwayUrl(this, 'alpha');
    const view = await fetchSelfHealing(url);
    const upstream = findUpstream(view, state.endpointFragment);
    assert.ok(upstream, `no upstreams[] entry for ${state.endpointFragment}`);
    assert.equal(
      upstream.circuit,
      'closed',
      `breaker circuit is "${upstream.circuit}", expected "closed" after a successful half-open trial`
    );
    const followUp = await getRaw(`${url}${UNKNOWN_CONTENT_PATH}`, { timeoutMs: 15_000 });
    assert.ok(
      followUp.status < 500,
      `normal proxying did not resume — follow-up request returned ${followUp.status}: ${followUp.text}`
    );
  }
);

// ── elohim-storage's own per-request inflight shed ──────────────────────────

interface StorageSaturationRig {
  storageUrl: string;
  stop: () => void;
  settled: Promise<unknown>;
  probeStatus: number;
  probeHeaders: Record<string, string | undefined>;
  probeBody: string;
}

const storageRigs = new WeakMap<E2EWorld, StorageSaturationRig>();

/**
 * Hold real concurrent, non-mutating reads against ONE storage peer (bypassing
 * doorway) past MAX_CONCURRENT_READS (256) so its per-request inflight
 * semaphore is genuinely exhausted for the length of the scenario — same
 * hold-time construction as the doorway rig above, for the same reason.
 * DESTRUCTIVE/HEAVY: real concurrent load against a live household storage
 * peer — hold for explicit operator go.
 */
async function saturateStorageInflight(
  world: E2EWorld,
  storageUrl: string
): Promise<StorageSaturationRig> {
  const concurrency = Number(
    process.env['E2E_STORAGE_SATURATION_BURST'] ??
      STORAGE_MAX_CONCURRENT_READS * SATURATION_OVERSHOOT
  );
  const load = sustainedLoad(`${storageUrl}${SATURATING_READ_PATH}`, concurrency);
  const rig: StorageSaturationRig = {
    storageUrl,
    stop: load.stop,
    settled: load.settled,
    probeStatus: 0,
    probeHeaders: {},
    probeBody: '',
  };
  storageRigs.set(world, rig);
  world.onCleanup(async () => {
    load.stop();
    await load.settled;
  });
  await sleep(250);
  const probe = await probeUntilShed(`${storageUrl}${SATURATING_READ_PATH}`, 10_000);
  rig.probeStatus = probe.status;
  rig.probeHeaders = probe.headers;
  rig.probeBody = probe.text;
  return rig;
}

function storageRig(world: E2EWorld): StorageSaturationRig {
  const rig = storageRigs.get(world);
  assert.ok(rig, "elohim-storage's inflight permits were not saturated yet");
  return rig;
}

Given("elohim-storage's inflight permits are exhausted", async function (this: E2EWorld) {
  if (!DESTRUCTIVE) {
    return skipHeldDestructive(
      `holds ${STORAGE_MAX_CONCURRENT_READS * SATURATION_OVERSHOOT} concurrent reads directly ` +
        'against a live storage peer to saturate its read pool.'
    );
  }
  const rig = await saturateStorageInflight(this, primaryStorageUrl('alpha'));
  assert.equal(
    rig.probeStatus,
    503,
    `expected 503 after saturating storage's read pool (ceiling ${STORAGE_MAX_CONCURRENT_READS}), ` +
      `got ${rig.probeStatus}: ${rig.probeBody}. MEASURED 2026-08-22 against this mesh: 2000 ` +
      "concurrent heavy reads at a peer produced ZERO 'storage request admission at ceiling' log " +
      'lines while elohim_http_requests_in_flight read 1 and /metrics itself went unanswered for ' +
      '~6s — storage serves this load essentially serially, so its per-request semaphore is never ' +
      'reached and the shed cannot fire. The caller queues instead, which is the very failure this ' +
      'scenario names. The gap is in elohim-storage (src/http.rs handle_request admission gate), ' +
      'not in this step.'
  );
});

interface StorageRequestState {
  response: RawHttpResponse;
}

const storageRequests = new WeakMap<E2EWorld, StorageRequestState>();

When(
  'a non-exempt request arrives at storage',
  { timeout: 20_000 },
  async function (this: E2EWorld) {
    const rig = storageRig(this);
    const response = await getRawWithHeaders(`${rig.storageUrl}${CHEAP_READ_PATH}`, {
      timeoutMs: 10_000,
    });
    storageRequests.set(this, { response });
  }
);

function storageRequest(world: E2EWorld): StorageRequestState {
  const state = storageRequests.get(world);
  assert.ok(state, 'no request was issued to storage yet');
  return state;
}

Then('storage sheds it with HTTP 503 and a Retry-After header', function (this: E2EWorld) {
  const { response } = storageRequest(this);
  assert.equal(response.status, 503, `expected 503, got ${response.status}: ${response.text}`);
  assert.equal(
    response.headers['retry-after'],
    String(STORAGE_SHED_RETRY_AFTER_SECS),
    `expected Retry-After: ${STORAGE_SHED_RETRY_AFTER_SECS}, got ` +
      `${response.headers['retry-after'] ?? '<absent>'}`
  );
});

Then(
  'storage continues accepting connections so liveness still answers',
  async function (this: E2EWorld) {
    const rig = storageRig(this);
    const health = await getRaw(`${rig.storageUrl}/health`, { timeoutMs: 10_000 });
    assert.equal(
      health.status,
      200,
      `GET ${rig.storageUrl}/health returned ${health.status} while the inflight pool was ` +
        'saturated — the accept loop must keep serving liveness even at zero permits'
    );
  }
);

// ── Doorway honoring storage's own backpressure ─────────────────────────────

Given(
  'elohim-storage responds with 503 and its own Retry-After header',
  async function (this: E2EWorld) {
    if (!DESTRUCTIVE) {
      return skipHeldDestructive(
        `holds ${STORAGE_MAX_CONCURRENT_READS * SATURATION_OVERSHOOT} concurrent reads directly ` +
          'against a live storage peer to saturate its read pool.'
      );
    }
    const rig = await saturateStorageInflight(this, primaryStorageUrl('alpha'));
    assert.equal(
      rig.probeStatus,
      503,
      `expected storage to shed (503), got ${rig.probeStatus}: ${rig.probeBody}. This scenario's ` +
        'precondition is an upstream that sheds with its OWN Retry-After, so it is blocked by the ' +
        'same measured elohim-storage gap as the sibling scenario above: under real concurrent ' +
        'load the peer queues rather than reaching its read-admission ceiling, so it never emits ' +
        'the 503 the doorway is supposed to surface.'
    );
    assert.equal(
      rig.probeHeaders['retry-after'],
      String(STORAGE_SHED_RETRY_AFTER_SECS),
      `expected storage's own Retry-After: ${STORAGE_SHED_RETRY_AFTER_SECS}, got ` +
        `${rig.probeHeaders['retry-after'] ?? '<absent>'}`
    );
  }
);

When(
  'the doorway proxies a request to storage',
  { timeout: 20_000 },
  async function (this: E2EWorld) {
    const url = doorwayUrl(this, 'alpha');
    const response = await getRawWithHeaders(`${url}${CHEAP_READ_PATH}`, { timeoutMs: 15_000 });
    proxiedRequests.set(this, { response, elapsedMs: 0 });
  }
);

Then(
  'the doorway surfaces 503 catching-up to the client preserving the upstream Retry-After',
  function (this: E2EWorld) {
    const { response } = proxiedRequest(this);
    assert.equal(response.status, 503, `expected 503, got ${response.status}: ${response.text}`);
    const body = JSON.parse(response.text) as Record<string, unknown>;
    assert.equal(body['status'], 'catching-up', `body.status was "${body['status']}"`);
    // storage_proxy.rs: `retry_after = upstream_ra.unwrap_or(UPSTREAM_CIRCUIT_COOLDOWN_SECS)` —
    // when storage's own Retry-After parses, doorway preserves that VALUE
    // verbatim rather than substituting its own default.
    assert.equal(
      response.headers['retry-after'],
      String(STORAGE_SHED_RETRY_AFTER_SECS),
      `expected the doorway to preserve storage's Retry-After (${STORAGE_SHED_RETRY_AFTER_SECS}), ` +
        `got ${response.headers['retry-after'] ?? '<absent>'}`
    );
  }
);

Then('the doorway does not return a bare 502', function (this: E2EWorld) {
  const { response } = proxiedRequest(this);
  assert.notEqual(
    response.status,
    502,
    'doorway returned a bare 502 instead of surfacing backpressure'
  );
});

// ===========================================================================
// Plan C — the self-healing read model (observe / elevate)
// ===========================================================================

const selfHealingViews = new WeakMap<E2EWorld, SelfHealingView>();

Given('the doorway is serving', async function (this: E2EWorld) {
  const url = doorwayUrl(this, 'alpha');
  const health = await probeHealth(url);
  assert.equal(health.status, 200, `GET ${url}/health returned ${health.status}`);
  assert.equal(health.body.healthy, true, `GET ${url}/health body.healthy was not true`);
});

When('an operator reads {string}', async function (this: E2EWorld, path: string) {
  assert.equal(
    path,
    '/admin/self-healing',
    `this step file only wires ${path === '' ? '(empty)' : path}`
  );
  const url = doorwayUrl(this, 'alpha');
  const view = await fetchSelfHealing(url);
  selfHealingViews.set(this, view);
});

function selfHealingViewState(world: E2EWorld): SelfHealingView {
  const view = selfHealingViews.get(world);
  assert.ok(view, '/admin/self-healing was not read yet');
  return view;
}

Then(
  'the response includes the projector, peers, render, warmup, and conductor blocks',
  function (this: E2EWorld) {
    const view = selfHealingViewState(this);
    for (const key of ['projector', 'peers', 'render', 'warmup', 'conductor'] as const) {
      assert.ok(
        Object.prototype.hasOwnProperty.call(view, key),
        `/admin/self-healing response is missing the "${key}" block`
      );
    }
  }
);

Then('the projector block reports whether the node is caught up', function (this: E2EWorld) {
  const view = selfHealingViewState(this);
  assert.ok(
    Object.prototype.hasOwnProperty.call(view.projector, 'caughtUp'),
    '/admin/self-healing projector block has no "caughtUp" key'
  );
  // caughtUp is Option<bool> in Rust (null when storage's reconcile task
  // never spawned) — assert the key is present and, when non-null, boolean;
  // never require it to be a specific value.
  assert.ok(
    view.projector.caughtUp === null || typeof view.projector.caughtUp === 'boolean',
    `projector.caughtUp is neither null nor boolean: ${JSON.stringify(view.projector.caughtUp)}`
  );
});

Then(
  /^the render block reports the degenerate \(stalled or timed-out\) rate$/,
  function (this: E2EWorld) {
    const view = selfHealingViewState(this);
    assert.equal(
      typeof view.render.degenerateRate,
      'number',
      `render.degenerateRate is not a number: ${JSON.stringify(view.render.degenerateRate)}`
    );
    assert.ok(
      Number.isFinite(view.render.degenerateRate),
      `render.degenerateRate is not finite: ${view.render.degenerateRate}`
    );
    assert.ok(
      view.render.degenerateRate >= 0 && view.render.degenerateRate <= 1,
      `render.degenerateRate (${view.render.degenerateRate}) is outside the expected [0,1] rate range`
    );
  }
);
