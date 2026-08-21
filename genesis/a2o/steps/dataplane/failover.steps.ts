/**
 * Step glue for the doorway-failover concern
 * (features/dataplane/doorway-failover.feature) — the "two doorways, one
 * name" invariant. See that feature's header comment for the full
 * serving/shedding/dead vocabulary; classifyDoorwayState() in
 * src/framework/dataplane/surfaces.ts is the single implementation of it.
 *
 * Six steps live here — three classification/head steps, then three
 * freshness steps (added 2026-08-21) that read the wire trust signal:
 *   1. `Then doorway {string} classifies as serving or shedding, not dead`
 *      — the honest-classification bar. Fails ONLY on 'dead'; shedding is a
 *      pass (it is the specified degraded contract, not an outage).
 *   2. `Then at least one of doorways {string} and {string} is serving`
 *      — the pair-floor bar. Passes iff either peer classifies 'serving'.
 *   3. `Then every serving doorway among {string} and {string} resolves the
 *      same declared head for content {string}` — the "whoever serves,
 *      serves the same truth" bar. Classifies both peers, then reads
 *      probeDeclaredHead() (surfaces.ts — shared with the ch10 comparator in
 *      resiliency-saga.steps.ts) on every peer classified 'serving'. Zero
 *      serving peers is a FAIL here (the pair-floor scenario above is the
 *      one that owns "at least one serves" — this step must never
 *      vacuously pass just because nobody was serving).
 *   4. `Then the knowledge read {string} on doorway {string} is answered and
 *      declares its freshness` — a Knowledge-class read must be ANSWERED
 *      (200; a 503 on this class is the failure the freshness cure exists to
 *      remove) and must name its colour on the wire. Amber additionally has
 *      to carry its receipts.
 *   5. `Then the authority read {string} on doorway {string} is green or
 *      honestly shed` — the floor row. Never a 200 carrying amber, at any
 *      status; a shed must name `green` as the colour it required. The class
 *      header is asserted at EVERY status, so a doorway that mis-routes an
 *      authority read into the knowledge class fails here rather than
 *      silently disarming the amber ban.
 *   6. `Then doorway {string} advertises a freshness stage, with authority
 *      green-only and knowledge amber-ok` — the advertised policy (C7): what
 *      the doorway publishes must equal what it would serve.
 *
 * Wire contract for 4-6 (doorway freshness spec, 2026-08-21):
 *   x-elohim-freshness: green|amber            (every proxied GET)
 *   x-elohim-freshness-class: knowledge|value|authority
 *   x-elohim-stocked-at: <RFC3339>             (amber only)
 *   x-elohim-served-head: <bafkrei…>           (amber only — CIDv1 raw of the
 *                                               served bytes)
 *   x-elohim-freshness-warn: 1                 (ServeAmberWarn only)
 *   x-elohim-freshness-required: green         (on the 503 shed)
 *   status.json / /health/serving → `freshness` { stage, policy[] }
 *
 * Peer resolution goes straight through resolvePeerUrl() (not
 * world.getDoorway()) — these steps run standalone against the "alpha-A" /
 * "elohim.host" aliases the feature's Background registers by the same
 * names, so no per-scenario Given step is required for classification alone.
 */

import { strict as assert } from 'node:assert';

import { Then } from '@cucumber/cucumber';

import {
  classifyDoorwayState,
  getRawWithHeaders,
  probeDeclaredHead,
  resolvePeerUrl,
  CLASSIFY_TIMEOUT_MS,
  CATCHUP_RIDE_STEP_TIMEOUT_MS,
  type DoorwayState,
  type RawHttpResponse,
} from '../../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../../src/framework/world.js';

/**
 * classifyDoorwayState issues up to 3 sequential bounded requests (/, then
 * /status.json, then /health on the 503 tie-break path) — worst case
 * ~3 * CLASSIFY_TIMEOUT_MS for ONE peer. Steps that classify two peers in
 * parallel still bound to the same per-peer worst case (Promise.all), plus
 * headroom for the declared-head fetch(es) on whichever peers are serving.
 */
const SINGLE_CLASSIFY_TIMEOUT_MS = 3 * CLASSIFY_TIMEOUT_MS + 15_000;
const PAIR_CLASSIFY_TIMEOUT_MS = 3 * CLASSIFY_TIMEOUT_MS + 15_000;
/**
 * Re-derived 2026-08-20: `probeDeclaredHead` now RIDES the bounded catching-up
 * shed (surfaces.ts) instead of failing on the first 503, so the head fetch is
 * no longer one bounded `getRaw` — it can take up to CATCHUP_RIDE_TIMEOUT_MS.
 * The old `+ 30_000` headroom was sized against the pre-ride behaviour and
 * would kill this step on cucumber's generic timeout mid-ride, discarding the
 * `describeCatchUpRide` diagnosis the ride exists to produce. Classification
 * and the head fetch are sequential phases here, so the two caps add.
 */
const PAIR_CLASSIFY_AND_HEAD_TIMEOUT_MS = 3 * CLASSIFY_TIMEOUT_MS + CATCHUP_RIDE_STEP_TIMEOUT_MS;

// ---------------------------------------------------------------------------
// 1. Honest classification — shed is not death
// ---------------------------------------------------------------------------

Then(
  'doorway {string} classifies as serving or shedding, not dead',
  { timeout: SINGLE_CLASSIFY_TIMEOUT_MS },
  async function (this: E2EWorld, peerName: string) {
    const peerUrl = resolvePeerUrl(peerName);
    const state = await classifyDoorwayState(peerUrl);
    assert.notStrictEqual(
      state,
      'dead',
      `doorway "${peerName}" (${peerUrl}) classified as dead — connect error/timeout on BOTH / and /health; ` +
        'a healthy or shedding doorway must answer at least one of them'
    );
  }
);

// ---------------------------------------------------------------------------
// 2. The pair floor — at least one doorway is serving
// ---------------------------------------------------------------------------

Then(
  'at least one of doorways {string} and {string} is serving',
  { timeout: PAIR_CLASSIFY_TIMEOUT_MS },
  async function (this: E2EWorld, peerA: string, peerB: string) {
    const urlA = resolvePeerUrl(peerA);
    const urlB = resolvePeerUrl(peerB);
    const [stateA, stateB] = await Promise.all([
      classifyDoorwayState(urlA),
      classifyDoorwayState(urlB),
    ]);
    assert.ok(
      stateA === 'serving' || stateB === 'serving',
      `neither doorway is serving — "${peerA}"=${stateA}, "${peerB}"=${stateB}. ` +
        'A correlated shed/outage across the whole pair means no one can reach the commons at all.'
    );
  }
);

// ---------------------------------------------------------------------------
// 3. Whoever serves, serves the same declared truth
// ---------------------------------------------------------------------------

Then(
  'every serving doorway among {string} and {string} resolves the same declared head for content {string}',
  { timeout: PAIR_CLASSIFY_AND_HEAD_TIMEOUT_MS },
  async function (this: E2EWorld, peerA: string, peerB: string, contentId: string) {
    const peers = [
      { name: peerA, url: resolvePeerUrl(peerA) },
      { name: peerB, url: resolvePeerUrl(peerB) },
    ];

    const states: DoorwayState[] = await Promise.all(
      peers.map(async peer => classifyDoorwayState(peer.url))
    );
    const servingPeers = peers.filter((_, i) => states[i] === 'serving');

    assert.ok(
      servingPeers.length > 0,
      `no doorway among "${peerA}" and "${peerB}" is serving (${peerA}=${states[0]}, ${peerB}=${states[1]}) — ` +
        'the pair-floor scenario owns "at least one serves"; this step must not vacuously pass on zero serving peers'
    );

    const heads = await Promise.all(
      servingPeers.map(async peer => {
        const head = await probeDeclaredHead(peer.url, contentId);
        assert.ok(
          head.declared,
          `"${peer.name}" is classified serving but reports declared=false for "${contentId}" — ` +
            'no canonical declaration for a serving doorway to have resolved'
        );
        assert.ok(
          typeof head.headActionHash === 'string' && head.headActionHash.length > 0,
          `"${peer.name}" is classified serving but returned a null/empty declared head for "${contentId}"`
        );
        return { name: peer.name, headActionHash: head.headActionHash };
      })
    );

    if (heads.length === 2) {
      assert.strictEqual(
        heads[0].headActionHash,
        heads[1].headActionHash,
        `declared head diverges between serving doorways for "${contentId}": ` +
          `"${heads[0].name}"=${heads[0].headActionHash} vs "${heads[1].name}"=${heads[1].headActionHash} — ` +
          'failover changed the answer'
      );
    }
    // heads.length === 1: the sole serving peer's own declared head is
    // sufficient — nothing to compare it against.
  }
);

// ---------------------------------------------------------------------------
// 4-6. Freshness graded by declared stakes (2026-08-21)
// ---------------------------------------------------------------------------

/**
 * One bounded GET per probe, and deliberately NOT `getRawRidingCatchUp`:
 * riding a shed until it clears is exactly what these steps must not do. The
 * Knowledge scenario calls a 503 on the landing read a FAILURE, and the
 * Authority scenario asserts what the shed itself says — a helper that waits
 * out the shed would erase both measurements.
 */
const FRESHNESS_PROBE_TIMEOUT_MS = 15_000;
/** One probe for steps 4-5; up to two sequential surfaces for step 6, plus headroom. */
const FRESHNESS_STEP_TIMEOUT_MS = 3 * FRESHNESS_PROBE_TIMEOUT_MS;

const FRESHNESS = 'x-elohim-freshness';
const FRESHNESS_CLASS = 'x-elohim-freshness-class';
const STOCKED_AT = 'x-elohim-stocked-at';
const SERVED_HEAD = 'x-elohim-served-head';
const FRESHNESS_WARN = 'x-elohim-freshness-warn';
const FRESHNESS_REQUIRED = 'x-elohim-freshness-required';

/** CIDv1, base32, raw codec + sha2-256 — the shape `x-elohim-served-head` must carry. */
const CIDV1_RAW_SHA256 = /^bafkrei[a-z2-7]+$/;

/** The four declared network stages the advertised policy may name. */
const KNOWN_STAGES = ['simulacra', 'bootstrap', 'coordinated', 'enforced'];

/** Surfaces on which a doorway may publish its freshness policy (either satisfies). */
const ADVERTISING_SURFACES = ['/status.json', '/health/serving'];

/** Compact rendering of the freshness signal actually on the wire, for failure messages. */
function describeFreshnessSignal(res: RawHttpResponse): string {
  const named = [
    FRESHNESS,
    FRESHNESS_CLASS,
    STOCKED_AT,
    SERVED_HEAD,
    FRESHNESS_WARN,
    FRESHNESS_REQUIRED,
  ]
    .filter(name => res.headers[name] !== undefined)
    .map(name => `${name}: ${res.headers[name]}`);
  return named.length > 0
    ? `HTTP ${res.status}; ${named.join('; ')}`
    : `HTTP ${res.status}; NO x-elohim-freshness* header on the response at all`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * The receipts an amber answer owes its reader: when the bytes were stocked,
 * and which content-addressed head they are. Without both, "amber" is an
 * unfalsifiable claim — the reader cannot tell a 20-minute-old page from a
 * fabricated one. `no-store` keeps a downstream cache from re-serving those
 * bytes as if they were fresh, and the warn flag must be absent because the
 * Knowledge class is AmberOk in every stage — never AmberWarn.
 */
function assertAmberReceipts(res: RawHttpResponse, where: string): void {
  const stockedAt = res.headers[STOCKED_AT];
  assert.ok(
    typeof stockedAt === 'string' && Number.isFinite(Date.parse(stockedAt)),
    `${where}: served amber without a parseable ${STOCKED_AT} (${describeFreshnessSignal(res)}) — ` +
      'an undated last-good answer cannot be told apart from a fabricated one'
  );
  const servedHead = res.headers[SERVED_HEAD];
  assert.ok(
    typeof servedHead === 'string' && CIDV1_RAW_SHA256.test(servedHead),
    `${where}: ${SERVED_HEAD} is ${JSON.stringify(servedHead)}, expected a CIDv1 raw/sha2-256 ` +
      `(bafkrei…) of the bytes served (${describeFreshnessSignal(res)})`
  );
  assert.strictEqual(
    (res.headers['cache-control'] ?? '').includes('no-store'),
    true,
    `${where}: amber served without Cache-Control: no-store (got ` +
      `${JSON.stringify(res.headers['cache-control'])}) — a downstream cache would re-serve ` +
      'these bytes as if they were fresh'
  );
  assert.strictEqual(
    res.headers[FRESHNESS_WARN],
    undefined,
    `${where}: knowledge-class amber carried ${FRESHNESS_WARN} — the knowledge class is ` +
      'amber-ok in every stage, so it never warns'
  );
}

// 4. Knowledge class — the read is answered, and the answer names its colour.

Then(
  'the knowledge read {string} on doorway {string} is answered and declares its freshness',
  { timeout: FRESHNESS_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, path: string, peerName: string) {
    const peerUrl = resolvePeerUrl(peerName);
    const where = `knowledge read ${path} on "${peerName}" (${peerUrl})`;
    const res = await getRawWithHeaders(`${peerUrl}${path}`, {
      timeoutMs: FRESHNESS_PROBE_TIMEOUT_MS,
    });

    assert.strictEqual(
      res.status,
      200,
      `${where}: answered ${res.status}, expected 200 (${describeFreshnessSignal(res)}). ` +
        'A knowledge-class read must be served — green from upstream, or amber from the ' +
        "doorway's own stocked bytes. Shedding it teaches a person the commons is down when " +
        'it is only behind.'
    );

    const freshness = res.headers[FRESHNESS];
    assert.ok(
      freshness === 'green' || freshness === 'amber',
      `${where}: ${FRESHNESS} is ${JSON.stringify(freshness)}, expected "green" or "amber" ` +
        `(${describeFreshnessSignal(res)}) — an answer that does not declare its colour is ` +
        'indistinguishable from one that is quietly stale'
    );
    assert.strictEqual(
      res.headers[FRESHNESS_CLASS],
      'knowledge',
      `${where}: ${FRESHNESS_CLASS} is ${JSON.stringify(res.headers[FRESHNESS_CLASS])}, ` +
        `expected "knowledge" (${describeFreshnessSignal(res)})`
    );

    if (freshness === 'amber') assertAmberReceipts(res, where);
  }
);

// 5. Authority class — the floor row: green, or an honest shed that names green.

Then(
  'the authority read {string} on doorway {string} is green or honestly shed',
  { timeout: FRESHNESS_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, path: string, peerName: string) {
    const peerUrl = resolvePeerUrl(peerName);
    const where = `authority read ${path} on "${peerName}" (${peerUrl})`;
    const res = await getRawWithHeaders(`${peerUrl}${path}`, {
      timeoutMs: FRESHNESS_PROBE_TIMEOUT_MS,
    });
    const freshness = res.headers[FRESHNESS];

    // Asserted at EVERY status: if the doorway routes this read into the
    // knowledge class, the amber ban below is disarmed without anyone noticing.
    assert.strictEqual(
      res.headers[FRESHNESS_CLASS],
      'authority',
      `${where}: ${FRESHNESS_CLASS} is ${JSON.stringify(res.headers[FRESHNESS_CLASS])}, ` +
        `expected "authority" (${describeFreshnessSignal(res)}) — a head-record read is what a ` +
        'peer stakes its declared truth on'
    );
    assert.notStrictEqual(
      freshness,
      'amber',
      `${where}: served amber (${describeFreshnessSignal(res)}) — an authority answer the ` +
        'doorway has not re-witnessed is worse than no answer, because the caller cannot tell'
    );

    if (res.status === 200) {
      assert.strictEqual(
        freshness,
        'green',
        `${where}: answered 200 with ${FRESHNESS}=${JSON.stringify(freshness)}, expected ` +
          `"green" (${describeFreshnessSignal(res)})`
      );
      assert.ok(
        res.headers[STOCKED_AT] === undefined && res.headers[SERVED_HEAD] === undefined,
        `${where}: answered green but carried amber receipts (${describeFreshnessSignal(res)}) — ` +
          'a live upstream answer was not stocked bytes'
      );
    } else if (res.status === 503) {
      assert.strictEqual(
        res.headers[FRESHNESS_REQUIRED],
        'green',
        `${where}: shed ${res.status} without ${FRESHNESS_REQUIRED}=green ` +
          `(${describeFreshnessSignal(res)}) — a caller told only "unavailable" cannot tell ` +
          'whether to retry for a fresher answer or to give up'
      );
    }

    // Any other status (404 = this peer holds no head record for the id, 502 =
    // its conductor bridge could not serve one) is the upstream's own honest
    // answer, passed through. The class + never-amber assertions above are the
    // ones that still bite there; the status itself is not this step's business.
  }
);

// 6. The advertised policy (C7) — what the doorway publishes it would serve.

interface AdvertisedFreshness {
  surface: string;
  block: Record<string, unknown>;
}

/**
 * Read the `freshness` block a doorway publishes, trying each advertising
 * surface in turn and returning the first that carries one. Returns the notes
 * gathered along the way so a failure can say WHY each surface did not answer
 * (unreachable / non-200 / not JSON / 200 without the block) instead of only
 * that none did.
 */
async function readAdvertisedFreshness(
  peerUrl: string
): Promise<{ found: AdvertisedFreshness | null; notes: string[] }> {
  const notes: string[] = [];
  for (const surface of ADVERTISING_SURFACES) {
    let res: RawHttpResponse;
    try {
      // Sequential on purpose: the second surface is only consulted when the
      // first did not advertise, so the peer is never probed twice for nothing.
      res = await getRawWithHeaders(`${peerUrl}${surface}`, {
        timeoutMs: FRESHNESS_PROBE_TIMEOUT_MS,
      });
    } catch (err) {
      notes.push(`${surface}: unreachable (${(err as Error).message})`);
      continue;
    }
    if (res.status !== 200) {
      notes.push(`${surface}: HTTP ${res.status}`);
      continue;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(res.text);
    } catch {
      notes.push(`${surface}: 200 but the body is not JSON`);
      continue;
    }
    const block = isRecord(parsed) ? parsed['freshness'] : undefined;
    if (!isRecord(block)) {
      notes.push(`${surface}: 200 but carries no "freshness" object`);
      continue;
    }
    return { found: { surface, block }, notes };
  }
  return { found: null, notes };
}

/** The requirement the advertised policy table states for one read class. */
function advertisedRequirementFor(policy: unknown, readClass: string, where: string): unknown {
  assert.ok(
    Array.isArray(policy) && policy.length > 0,
    `${where}: freshness.policy is ${JSON.stringify(policy)}, expected a non-empty table of ` +
      '{class, requirement} rows'
  );
  const row = (policy as unknown[]).find(entry => isRecord(entry) && entry['class'] === readClass);
  assert.ok(
    isRecord(row),
    `${where}: the advertised policy names no "${readClass}" row (rows: ` +
      `${JSON.stringify(policy)}) — a class the doorway serves but does not advertise is a ` +
      'contract nobody can read ahead of time'
  );
  return row['requirement'];
}

Then(
  'doorway {string} advertises a freshness stage, with authority green-only and knowledge amber-ok',
  { timeout: FRESHNESS_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, peerName: string) {
    const peerUrl = resolvePeerUrl(peerName);
    const { found, notes } = await readAdvertisedFreshness(peerUrl);
    assert.ok(
      found,
      `doorway "${peerName}" (${peerUrl}) advertises no freshness block on ` +
        `${ADVERTISING_SURFACES.join(' or ')} — ${notes.join(' | ')}`
    );

    const where = `doorway "${peerName}" ${found.surface} freshness`;
    const stage = found.block['stage'];
    assert.ok(
      typeof stage === 'string' && KNOWN_STAGES.includes(stage),
      `${where}: stage is ${JSON.stringify(stage)}, expected one of ${KNOWN_STAGES.join(', ')}`
    );

    const policy = found.block['policy'];
    assert.strictEqual(
      advertisedRequirementFor(policy, 'authority', where),
      'green-only',
      `${where}: the authority row does not advertise "green-only" (policy: ` +
        `${JSON.stringify(policy)}) — the floor row is never priced by stage`
    );
    assert.strictEqual(
      advertisedRequirementFor(policy, 'knowledge', where),
      'amber-ok',
      `${where}: the knowledge row does not advertise "amber-ok" (policy: ` +
        `${JSON.stringify(policy)}) — knowledge reads are the ones that may ride behind`
    );
  }
);
