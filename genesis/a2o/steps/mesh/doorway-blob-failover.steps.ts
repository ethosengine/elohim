/**
 * Step glue for ONE scenario in features/dataplane/doorway-failover.feature:
 * "A blob rides through its primary's bad hour".
 *
 * WHY THIS IS ITS OWN FILE. The other failover steps (steps/dataplane/
 * failover.steps.ts) are read-only classifiers that run against a fleet this
 * suite does not own. This scenario is a DRILL: it stops a real process. Keeping
 * the destructive glue separate keeps that file's read-set honest — nothing here
 * is imported by, or importable from, a classification step.
 *
 * The drill in one line: doorway "alpha-A"'s primary storage peer is SIGSTOPped
 * until the doorway's circuit for it opens; a blob read then has to arrive
 * anyway, because `/blob/<hash>` is content-addressed and the sibling peers in
 * the same steward pool hold the identical bytes.
 *
 * Process-control contract (same discipline as steps/mesh/household-chaos.steps.ts):
 *   - the pid is resolved by EXACT argv match on the peer's listen port
 *     (`requireSinglePidByArgv(storagePeerArgvMatch(port))`), never `pkill -f`,
 *     which also matches the runner's own shell and self-kills the run;
 *   - SIGSTOP, not SIGKILL: a paused peer still completes the TCP handshake, so
 *     the doorway sees TIMEOUTS — the "bad hour" shape the breaker exists for —
 *     rather than an instant connection refusal. It is also restorable with one
 *     signal, no re-exec and no re-key;
 *   - every pause registers its SIGCONT in the `After` hook below, so a
 *     mid-scenario failure never leaves a peer stopped for the rest of the run;
 *   - every destructive step asks `destructiveAllowed()` FIRST and answers
 *     `skipped` (never `pending` — cucumber-js is strict and a pending step reds
 *     the run exactly like a failure) when this lane does not own the substrate.
 *     A skipped step skips the rest of the scenario, which is how the edge
 *     Dataplane Validation lane runs this feature file without ever signalling a
 *     pod it does not own.
 *
 * WHY THE PROOF READS A METRIC AND NOT JUST A STATUS. A 200 alone proves
 * nothing: doorway keeps a blob PANTRY, and a cached copy answers before the
 * upstream breaker is ever consulted (routes/storage_proxy.rs
 * `forward_blob_to_storage` — the pantry hit short-circuits above
 * `breakers.begin`). So the proving read sends `Range: bytes=0-0`, which
 * deliberately BYPASSES the pantry and forces a real upstream hop, and the
 * assertion is that `doorway_blob_target_failover_total` moved — the doorway
 * saying, in its own numbers, "I chose a different single target." The plain
 * full-body read that follows is the person-facing half: the bytes actually
 * arrive.
 */

import { strict as assert } from 'node:assert';

import { After, Given, Then, When } from '@cucumber/cucumber';

import { getRaw, probeContent, resolvePeerUrl } from '../../src/framework/dataplane/surfaces.js';
import {
  loadHouseholdMeshFixture,
  requireFixturePoolStorageUrls,
  requireFixturePrimaryStorageUrl,
} from '../../src/framework/fixtures/household-mesh.js';
import {
  requireSinglePidByArgv,
  storagePeerArgvMatch,
} from '../../src/framework/fixtures/process-control.js';
import {
  destructiveAllowed,
  DESTRUCTIVE_HELD_HINT,
} from '../../src/framework/fixtures/substrate-scope.js';
import { retry } from '../../src/framework/utils/retry.js';
import { E2EWorld } from '../../src/framework/world.js';

/** Doorway names as the failover feature says them → the fixture's doorway ids. */
const FIXTURE_DOORWAY_ID: Record<string, string> = {
  'alpha-A': 'alpha',
  'elohim.host': 'beta',
};

/** doorway/doorway-service/src/routes/upstream_health.rs — the live policy. */
const CIRCUIT_FAIL_THRESHOLD = 3;
/** doorway/doorway-service/src/routes/storage_proxy.rs STORAGE_PROXY_REQUEST_TIMEOUT_SECS. */
const UPSTREAM_REQUEST_TIMEOUT_MS = 12_000;
/** One trip round (concurrent) plus the circuit poll plus headroom. */
const TRIP_STEP_TIMEOUT_MS = 180_000;
const FAILOVER_METRIC = 'doorway_blob_target_failover_total';

interface FailoverDrill {
  doorwayUrl: string;
  primaryStorageUrl: string;
  siblingStorageUrls: string[];
  blobHash?: string;
  /** Set once SIGSTOP has actually landed, so the After hook only resumes what it stopped. */
  pausedPid?: number;
}

const drills = new WeakMap<E2EWorld, FailoverDrill>();

function held(description: string): 'skipped' {
  process.stdout.write(`[a2o] destructive step HELD: ${description}. ${DESTRUCTIVE_HELD_HINT}\n`);
  return 'skipped';
}

function withoutTrailingSlashes(url: string): string {
  let end = url.length;
  while (end > 0 && url[end - 1] === '/') end -= 1;
  return url.slice(0, end);
}

function drill(world: E2EWorld, doorwayName: string): FailoverDrill {
  const existing = drills.get(world);
  if (existing) return existing;

  const fixtureId = FIXTURE_DOORWAY_ID[doorwayName];
  assert.ok(
    fixtureId,
    `doorway "${doorwayName}" has no household-fixture id — this drill knows ` +
      `${Object.keys(FIXTURE_DOORWAY_ID).join(', ')}`
  );
  const fixture = loadHouseholdMeshFixture();
  const primaryStorageUrl = requireFixturePrimaryStorageUrl(fixture, fixtureId);
  const pool = requireFixturePoolStorageUrls(fixture, fixtureId);
  const state: FailoverDrill = {
    doorwayUrl: withoutTrailingSlashes(resolvePeerUrl(doorwayName)),
    primaryStorageUrl,
    siblingStorageUrls: pool.filter(url => url !== primaryStorageUrl),
  };
  drills.set(world, state);
  return state;
}

function requireDrill(world: E2EWorld): FailoverDrill {
  const state = drills.get(world);
  assert.ok(state, 'no failover drill in flight — the Given step must run first');
  return state;
}

/** The pid of the elohim-storage listening on `storageUrl`'s port, by exact argv. */
function storagePidFor(storageUrl: string): number {
  const port = new URL(storageUrl).port;
  assert.ok(port, `storage peer "${storageUrl}" has no port to resolve a pid by`);
  return requireSinglePidByArgv(storagePeerArgvMatch(port));
}

/** Read one scalar Prometheus counter off a doorway's /metrics; 0 when absent. */
async function metricValue(doorwayUrl: string, name: string): Promise<number> {
  const { status, text } = await getRaw(`${doorwayUrl}/metrics`, { timeoutMs: 20_000 });
  assert.strictEqual(status, 200, `GET ${doorwayUrl}/metrics → ${status}`);
  for (const line of text.split('\n')) {
    if (line.startsWith('#')) continue;
    const match = new RegExp(String.raw`^${name}(?:\{[^}]*\})?[ \t]+([^ \t]+)$`).exec(line.trim());
    if (match) return Number(match[1]);
  }
  return 0;
}

/** The doorway's own view of one upstream's circuit, from /health/startup. */
async function circuitFor(doorwayUrl: string, upstream: string): Promise<string | undefined> {
  const { status, text } = await getRaw(`${doorwayUrl}/health/startup`, { timeoutMs: 20_000 });
  if (status !== 200 && status !== 503) return undefined;
  let body: Record<string, unknown>;
  try {
    body = JSON.parse(text) as Record<string, unknown>;
  } catch {
    return undefined;
  }
  const upstreams = body['upstreams'] as { upstream?: string; circuit?: string }[] | undefined;
  if (!Array.isArray(upstreams)) return undefined;
  return upstreams.find(entry => sameUpstream(entry.upstream, upstream))?.circuit;
}

/**
 * One upstream, two spellings. The fixture names a peer `http://localhost:8090`;
 * the doorway names the same peer by the argv it was launched with
 * (`http://127.0.0.1:8090`) — measured 2026-08-23 on the household mesh, where
 * the first run read circuit="unknown" for a primary whose breaker HAD opened.
 * Loopback is one host; a port decides identity there. Anything else must
 * match host-for-host.
 */
function sameUpstream(a: string | undefined, b: string | undefined): boolean {
  if (!a || !b) return false;
  let ua: URL;
  let ub: URL;
  try {
    ua = new URL(a);
    ub = new URL(b);
  } catch {
    return withoutTrailingSlashes(a) === withoutTrailingSlashes(b);
  }
  const loopback = (h: string) => h === 'localhost' || h === '127.0.0.1' || h === '[::1]';
  const hostA = loopback(ua.hostname) ? '127.0.0.1' : ua.hostname;
  const hostB = loopback(ub.hostname) ? '127.0.0.1' : ub.hostname;
  return hostA === hostB && ua.port === ub.port;
}

// ---------------------------------------------------------------------------
// Given — a blob the pool holds, addressable through the doorway
// ---------------------------------------------------------------------------

Given(
  'content {string} has a blob that a holder behind doorway {string} other than its primary answers for',
  { timeout: 120_000 },
  async function (this: E2EWorld, contentId: string, doorwayName: string) {
    if (!destructiveAllowed()) {
      return held(`resolve a blob to ride past doorway "${doorwayName}"'s primary`);
    }
    const state = drill(this, doorwayName);
    assert.ok(
      state.siblingStorageUrls.length > 0,
      `doorway "${doorwayName}" declares only one storage peer (${state.primaryStorageUrl}) — a ` +
        'pool of one has nobody to ride to, so this concern cannot be measured on this substrate'
    );

    const { body } = await probeContent(state.doorwayUrl, contentId);
    const hash = body.blobHash;
    assert.ok(
      typeof hash === 'string' && hash.length > 0,
      `content "${contentId}" carries no blobHash on doorway "${doorwayName}" — there are no ` +
        'bytes to ride'
    );
    state.blobHash = hash;

    // Prove a sibling can actually SERVE these bytes before we stop the primary.
    // This is an HTTP read on purpose (get_blob_or_heal): the precondition this
    // scenario needs is "a sibling answers with the bytes", and a read that
    // heals-then-serves establishes exactly that. (Contrast the chaos drills,
    // which probe the filesystem — they CREATE a missing-bytes condition and an
    // HTTP probe would repair the very thing under test.)
    for (const sibling of state.siblingStorageUrls) {
      const { status } = await getRaw(`${sibling}/blob/${encodeURIComponent(hash)}`, {
        timeoutMs: 60_000,
      });
      assert.ok(
        status === 200,
        `storage peer "${sibling}" cannot serve blob ${hash} (GET → ${status}) — it is not a ` +
          `holder, so it cannot stand in for doorway "${doorwayName}"'s primary`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// When — the primary's bad hour
// ---------------------------------------------------------------------------

When(
  'doorway {string} loses its primary storage peer to a stall',
  { timeout: TRIP_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, doorwayName: string) {
    if (!destructiveAllowed()) {
      return held(`stall doorway "${doorwayName}"'s primary storage peer`);
    }
    const state = requireDrill(this);
    assert.ok(state.blobHash, 'no blob resolved — the Given step must run first');

    const pid = storagePidFor(state.primaryStorageUrl);
    process.kill(pid, 'SIGSTOP');
    state.pausedPid = pid;

    // A breaker opens on evidence, not on our say-so: THRESHOLD failures inside
    // the failure window. A stalled peer answers nothing, so each of these burns
    // the doorway's 12s upstream timeout — fired together so the failures really
    // are a burst rather than a streak spread past the window.
    const url = `${state.doorwayUrl}/blob/${encodeURIComponent(state.blobHash)}`;
    await Promise.all(
      Array.from({ length: CIRCUIT_FAIL_THRESHOLD }, async () => {
        try {
          return await getRaw(url, {
            timeoutMs: UPSTREAM_REQUEST_TIMEOUT_MS + 8_000,
            headers: { range: 'bytes=0-0' },
          });
        } catch {
          // A stalled peer is EXPECTED to make this throw or time out — the
          // failure is the evidence the breaker needs, not a fault of the drill.
          return { status: 0, text: '' };
        }
      })
    );

    await retry(
      async () => {
        const circuit = await circuitFor(state.doorwayUrl, state.primaryStorageUrl);
        assert.strictEqual(
          circuit,
          'open',
          `doorway "${doorwayName}" still reads circuit="${circuit ?? 'unknown'}" for its ` +
            `stalled primary ${state.primaryStorageUrl} — the bad hour has not started yet`
        );
      },
      { maxAttempts: 30, initialDelayMs: 2_000, maxDelayMs: 2_000 }
    );
  }
);

// ---------------------------------------------------------------------------
// Then — the read arrives anyway, from a holder that is serving
// ---------------------------------------------------------------------------

Then(
  'the blob still arrives through doorway {string}, from a holder that is not the stalled primary',
  { timeout: 120_000 },
  async function (this: E2EWorld, doorwayName: string) {
    if (!destructiveAllowed()) {
      return held(`read a blob through doorway "${doorwayName}" during its primary's stall`);
    }
    const state = requireDrill(this);
    assert.ok(state.blobHash, 'no blob resolved — the Given step must run first');
    const url = `${state.doorwayUrl}/blob/${encodeURIComponent(state.blobHash)}`;

    // Baseline taken HERE, after the trip: the requests that opened the circuit
    // were selected while it was still closed and must not be counted as
    // failovers of their own.
    const before = await metricValue(state.doorwayUrl, FAILOVER_METRIC);

    // Range read: pantry-bypassing, so this is provably an upstream hop and not
    // a cached copy answering from inside the doorway.
    const ranged = await getRaw(url, {
      timeoutMs: 60_000,
      headers: { range: 'bytes=0-0' },
    });
    assert.ok(
      ranged.status === 200 || ranged.status === 206,
      `doorway "${doorwayName}" answered ${ranged.status} for a blob its pool still holds — the ` +
        "primary's stall became the reader's outage"
    );

    const after = await metricValue(state.doorwayUrl, FAILOVER_METRIC);
    assert.ok(
      after > before,
      `${FAILOVER_METRIC} did not move (${before} → ${after}) — doorway "${doorwayName}" answered ` +
        'without ever choosing a different holder, so nothing here proves failover'
    );

    // And the person-facing half: the whole object, not a probe byte.
    const whole = await getRaw(url, { timeoutMs: 60_000 });
    assert.strictEqual(
      whole.status,
      200,
      `doorway "${doorwayName}" answered ${whole.status} for the full blob read`
    );
    assert.ok(whole.text.length > 0, `doorway "${doorwayName}" served an empty blob body`);
  }
);

Then(
  "I restore doorway {string}'s primary storage peer, and it answers again",
  { timeout: 120_000 },
  async function (this: E2EWorld, doorwayName: string) {
    if (!destructiveAllowed()) {
      return held(`resume doorway "${doorwayName}"'s primary storage peer`);
    }
    const state = requireDrill(this);
    resume(state);
    await retry(
      async () => {
        const { status } = await getRaw(`${state.primaryStorageUrl}/health`, { timeoutMs: 15_000 });
        assert.strictEqual(
          status,
          200,
          `primary storage peer ${state.primaryStorageUrl} did not come back after SIGCONT ` +
            `(GET /health → ${status})`
        );
      },
      { maxAttempts: 30, initialDelayMs: 2_000, maxDelayMs: 2_000 }
    );
  }
);

function resume(state: FailoverDrill): void {
  if (state.pausedPid === undefined) return;
  try {
    process.kill(state.pausedPid, 'SIGCONT');
  } catch {
    // Already gone (a supervisor restarted it) — nothing left to resume.
  }
  state.pausedPid = undefined;
}

// A stopped peer must never outlive the scenario that stopped it: without this,
// a mid-scenario failure leaves the whole remaining run reading a frozen peer.
After({ tags: '@concern:doorway-failover', timeout: 120_000 }, function (this: E2EWorld) {
  const state = drills.get(this);
  if (!state) return;
  resume(state);
});
