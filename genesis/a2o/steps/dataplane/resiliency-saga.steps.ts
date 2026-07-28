/**
 * New glue for the resiliency-saga chapters (features/dataplane/resiliency-saga/).
 *
 * Every step below exists ONLY because no existing primitive in
 * steps/dataplane.steps.ts or steps/resilience.steps.ts fit the chapter's proof
 * signal. Where an existing step fit, the chapter feature files reuse it directly
 * (served-head comparisons, /health assertions, the dotted-path "I read" +
 * "response field ... is at least" pair) — see resiliency-saga/README.md's
 * "Reuse over reinvention" section for the full map.
 *
 * Five concerns are covered here (2b was added by the 2026-07-26 story-harvest,
 * which decomposed chapter 5 from one node into an observable pipeline):
 *   1. A label-aware Prometheus step (chapters 2, 7) — pins ONE label combination
 *      on a *Vec metric, and routes elohim_-prefixed metric names to the direct
 *      storage URL (they are emitted by elohim-storage's own /metrics, never the
 *      doorway's port-8080 /metrics) the same way the existing bare-metric step in
 *      dataplane.steps.ts routes p2p_/reconcile_/dedup_ prefixes — extended here to
 *      also cover elohim_, without touching that existing step or its routing.
 *   2. A polling commitment-count step (chapter 5) against GET
 *      /api/v1/commitments?action=...&state=active — resilience.steps.ts's
 *      "I list active {string} commitments" reads this same surface once, with no
 *      retry budget; this chapter needs the "within N seconds" polling idiom.
 *  2b. Chapter-5 STATION steps — an active-item-pin check (/api/v1/pins), a
 *      per-EPR pull-rollup caught-up poll (/api/v1/pins/{id}/pull), and a
 *      rea-FACING commitment check (/api/v1/commitments/facing/rea, the
 *      mishpat-sourced view — deliberately a different home from concern 2's
 *      rea-PROJECTION read). None of these surfaces had a step; the pin steps in
 *      steps/delivery/acquisition-pins.steps.ts are own-node (E2E_STORAGE_URL)
 *      POST-then-read flows, not doorway-proxied read-only assertions.
 *   3. A raw-body query pair (chapter 4) — the existing generic "When I query"
 *      step's captured body is private to dataplane.steps.ts with no
 *      body-substring assertion, and delivery.steps.ts's "the response is HTML
 *      containing ..." reads a DIFFERENT capture store populated only by its own
 *      "When I request/fetch" steps.
 *   4. A cross-doorway truth-compare step + a runLook rendered-card verdict
 *      (chapter 10) — "two doorways, one truth" needs a same-value-AND-non-zero
 *      assertion across two peers in one step; no existing step does both. The
 *      compared field is `stewardingCollectives` (ResilienceSnapshotView) — the
 *      count of stewarding collectives of any kind; there is no
 *      `householdsStewarding` on this route's wire shape.
 */

import { strict as assert } from 'node:assert';
import { existsSync } from 'node:fs';

import { When, Then } from '@cucumber/cucumber';

import { runLook, type LookResult } from '../../scripts/look.js';
import {
  getRaw,
  resolveStorageUrl,
  parseLabeledPrometheusMetric,
  pollForGauge,
  GAUGE_SWEEP_POLL_TIMEOUT_MS,
} from '../../src/framework/dataplane/surfaces.js';
import { PROTOCOL_OMNI } from '../../src/framework/pages/selectors.js';
import { retry } from '../../src/framework/utils/retry.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// 1. Label-aware Prometheus metric assertion (chapters 2, 7)
// ---------------------------------------------------------------------------

/**
 * Resolve the /metrics base URL for a given metric name on a peer, mirroring
 * (and extending) the routing logic in dataplane.steps.ts's bare "metric ... on
 * peer ..." step: an explicit E2E_METRICS_<PEER> env var always wins; otherwise
 * metric names owned by elohim-storage itself (elohim_/p2p_/reconcile_/dedup_
 * prefixes — every one of these is registered in elohim-storage/src/metrics.rs,
 * never in doorway's own metrics module) route to the direct storage URL; every
 * other name falls back to the doorway's own port-8080 /metrics.
 *
 * Returns '' when a storage-owned metric is requested but no direct storage URL
 * is configured for this peer (E2E_STORAGE_ALPHA / E2E_STORAGE_<PEER> unset) —
 * callers must treat that as pending, not a failure: the surface simply isn't
 * reachable in this environment, not proven absent.
 */
function resolveMetricsBaseUrl(world: E2EWorld, peerName: string, metricName: string): string {
  const envKey = `E2E_METRICS_${peerName.toUpperCase().replace(/[^A-Z0-9]/g, '_')}`;
  const override = process.env[envKey];
  if (override) return override;

  const isStorageOwned = /^(elohim_|p2p_|reconcile_|dedup_)/.test(metricName);
  if (isStorageOwned) {
    return resolveStorageUrl(peerName) ?? '';
  }

  const doorway = world.getDoorway(peerName);
  try {
    const parsed = new URL(doorway.url);
    parsed.port = '8080';
    return parsed.origin;
  } catch {
    return doorway.url;
  }
}

/**
 * Assert a labelled Prometheus metric value on the peer's /metrics endpoint,
 * pinning ONE label key/value pair (e.g. action="created", class="stocked").
 *
 * Same pending-not-fail semantics as the bare "metric ... on peer ..." step in
 * dataplane.steps.ts: an unreachable /metrics endpoint, a missing storage URL, or
 * the series/label combination never having been observed in the scrape all
 * return 'pending' rather than a hard failure — those are honest "cannot observe
 * yet" states, never a measured zero.
 *
 * Example:
 *   Then labeled metric "elohim_identity_fill_total" with label "action" "created" on peer "alpha-A" >= 1
 */
Then(
  'labeled metric {string} with label {string} {string} on peer {string} {word} {float}',
  // Default cucumber step timeout (30s) is shorter than the bounded gauge
  // poll below (up to GAUGE_SWEEP_POLL_TIMEOUT_MS = 6 minutes) — without an
  // explicit override cucumber would kill the step long before the poll's
  // own deadline governor fires (same trap the 60s commitment/pull polling
  // steps below document; +15s margin, matching their convention).
  { timeout: GAUGE_SWEEP_POLL_TIMEOUT_MS + 15_000 },
  async function (
    this: E2EWorld,
    metricName: string,
    labelKey: string,
    labelValue: string,
    peerName: string,
    cmp: string,
    expected: number
  ) {
    const base = resolveMetricsBaseUrl(this, peerName, metricName);
    if (!base) {
      // eslint-disable-next-line no-console
      console.log(
        `  PENDING: labeled metric "${metricName}" on ${peerName} — ` +
          `no direct storage URL configured (set E2E_STORAGE_${peerName.toUpperCase()})`
      );
      return 'pending';
    }

    // elohim_-prefixed series (identity-fill, custody-class) are populated by
    // elohim-storage's periodic multi-minute sweep, not on every scrape —
    // poll rather than declaring pending on the first miss. A non-200
    // response or network error is treated the same as "series absent yet"
    // (keep polling — the endpoint may still be coming up post-restart);
    // GAUGE_SWEEP_POLL_TIMEOUT_MS bounds the total wait either way.
    const actual = await pollForGauge(async () => {
      const { status, text: body } = await getRaw(`${base}/metrics`);
      if (status !== 200) {
        throw new Error(`GET ${base}/metrics returned ${status}`);
      }
      return parseLabeledPrometheusMetric(body, metricName, labelKey, labelValue);
    });

    if (actual === undefined) {
      // eslint-disable-next-line no-console
      console.log(
        `  PENDING: labeled metric "${metricName}{${labelKey}="${labelValue}"}" on ${peerName} — ` +
          `series/label combination not present after polling up to ` +
          `${GAUGE_SWEEP_POLL_TIMEOUT_MS / 1000}s (/metrics unreachable, or the sweep genuinely hasn't run yet)`
      );
      return 'pending';
    }

    const label = `labeled metric "${metricName}{${labelKey}="${labelValue}"}" on ${peerName}: ${actual} ${cmp} ${expected}`;
    switch (cmp) {
      case '>=':
        assert.ok(actual >= expected, `${label} — FAILED`);
        break;
      case '<=':
        assert.ok(actual <= expected, `${label} — FAILED`);
        break;
      case '>':
        assert.ok(actual > expected, `${label} — FAILED`);
        break;
      case '<':
        assert.ok(actual < expected, `${label} — FAILED`);
        break;
      case '==':
      case '=':
        assert.strictEqual(actual, expected, `${label} — FAILED`);
        break;
      case '!=':
        assert.notStrictEqual(actual, expected, `${label} — FAILED`);
        break;
      default:
        throw new Error(`Unknown comparison operator: "${cmp}". Use >=, <=, >, <, ==, !=`);
    }
  }
);

// ---------------------------------------------------------------------------
// 2. Polling commitment-count assertion (chapter 5)
// ---------------------------------------------------------------------------

/**
 * Poll GET /api/v1/commitments?action=<action>&state=active on the named
 * doorway until at least one row is present, or the timeout elapses. Real HTTP
 * surface (elohim-storage/src/http.rs -> db::rea_commitments::list_commitments,
 * filtered by action.eq(action) AND state.eq("active")), proxied by the doorway's
 * generic "/api/" service-path prefix. A 200 response with zero rows is a
 * genuine, honest FAILURE (not pending) — the surface works, there is simply
 * nothing notarized yet.
 *
 * Example:
 *   Then within 60 seconds doorway "alpha-A" has at least one active "replicates-commons" commitment
 */
Then(
  'within {int} seconds doorway {string} has at least one active {string} commitment',
  // Cucumber's default step timeout (30s) is shorter than this step's own
  // 60s poll deadline (timeoutMs below) — without an explicit per-step
  // timeout, cucumber kills the step at 30s before the poll's own governor
  // ever fires (edge #1233: "function timed out, ensure the promise
  // resolves within 30000 milliseconds"). 75s gives the 60s poll a margin
  // over cucumber's own timer rather than racing it exactly.
  { timeout: 75_000 },
  async function (this: E2EWorld, seconds: number, peerName: string, action: string) {
    const doorway = this.getDoorway(peerName);
    await retry(
      async () => {
        const { status, text } = await getRaw(
          `${doorway.url}/api/v1/commitments?action=${encodeURIComponent(action)}&state=active`
        );
        assert.strictEqual(
          status,
          200,
          `GET /api/v1/commitments?action=${action}&state=active on ${peerName}: HTTP ${status} (body: ${text.slice(0, 120)})`
        );
        let rows: unknown;
        try {
          rows = JSON.parse(text);
        } catch {
          throw new Error(
            `GET /api/v1/commitments on ${peerName}: response is not valid JSON: ${text.slice(0, 120)}`
          );
        }
        assert.ok(
          Array.isArray(rows),
          `GET /api/v1/commitments on ${peerName}: expected a bare JSON array, got: ${text.slice(0, 120)}`
        );
        assert.ok(
          rows.length >= 1,
          `No active "${action}" commitments on ${peerName} yet — the co-steward agreement has not been notarized and projected.`
        );
      },
      {
        timeoutMs: seconds * 1000,
        initialDelayMs: 1000,
        backoffFactor: 1.2,
        maxDelayMs: 5000,
        // retry()'s own default (maxAttempts: 10) would exhaust in ~21s at this
        // backoff schedule — well short of the "within {seconds} seconds" budget
        // this step promises. Set generously high so timeoutMs (the deadline
        // check at the top of retry()'s loop) is the only real governor, matching
        // the stated intent.
        maxAttempts: 1000,
      }
    );
  }
);

// ---------------------------------------------------------------------------
// 2b. Chapter-5 station steps (added 2026-07-26 — story-harvest of the cure
//     sprint that proved chapter 5 is a PIPELINE, not a single node).
//
//     The chain: explicit pin (consent) -> provide tick -> mishpat notarization
//     -> bounds-validated ProvideAnnounce -> graduation -> mishpat->rea mirror
//     -> the rea projection the chapter's finish-line step reads. Eight stacked
//     defects sat along it. These stations read the EARNED links at their own
//     sources so a red names which link broke, instead of the whole chain
//     collapsing onto one opaque "0 rows" assertion.
//
//     Constraint (carried from the step below, and the reason both polling
//     station steps declare it): cucumber's DEFAULT step timeout is 30s, which
//     is SHORTER than a 60s poll — without an explicit { timeout: 75_000 },
//     cucumber kills the step at 30s before the poll's own deadline governor
//     fires. retry()'s own maxAttempts default (10) likewise exhausts a 60s
//     budget in ~21s at this backoff schedule, so maxAttempts is set high and
//     timeoutMs is left as the only real governor.
//
//     Constraint (route cache): the commitments routes are cached 30s, so a
//     freshly-notarized row can lag a read by up to one cache window. Station
//     reads are steady-state measurements, never just-authored ones.
// ---------------------------------------------------------------------------

/**
 * Assert the named doorway lists at least one ACTIVE, item-kind pin whose
 * head_ref references the given content id — the explicit-consent leg of the
 * co-steward chain (GET /api/v1/pins, doorway-proxied since edge #1238).
 *
 * NAMESPACE RULE (the defect this guards): pin head_refs are `epr:`-prefixed
 * ("epr:elohim-host-landing") while content ids elsewhere — notably the
 * /api/v1/pins/{id}/pull route below — are BARE. The provide tick compared
 * prefixed head_refs against bare content ids, so its desired set was EMPTY for
 * every pin that ever existed. This step accepts EITHER spelling deliberately:
 * the assertion is "a pin references this content", not "a pin spells it my way".
 *
 * Single read, not a poll: a pin either exists or it does not — there is no
 * settling window to wait out.
 *
 * Example:
 *   Then doorway "alpha-A" has an active item pin whose head references "elohim-host-landing"
 */
Then(
  'doorway {string} has an active item pin whose head references {string}',
  async function (this: E2EWorld, peerName: string, contentId: string) {
    const doorway = this.getDoorway(peerName);
    const url = `${doorway.url}/api/v1/pins`;
    const { status, text } = await getRaw(url);
    assert.strictEqual(status, 200, `GET /api/v1/pins on ${peerName}: HTTP ${status}`);

    let payload: unknown;
    try {
      payload = JSON.parse(text);
    } catch {
      throw new Error(`GET /api/v1/pins on ${peerName}: response is not valid JSON`);
    }
    const pins = (payload as { pins?: unknown }).pins;
    assert.ok(
      Array.isArray(pins),
      `GET /api/v1/pins on ${peerName}: expected a "pins" array, got: ${text.slice(0, 160)}`
    );

    // Accept the prefixed and bare spellings of the same reference.
    const accepted = new Set([contentId, `epr:${contentId}`]);
    const match = (pins as Record<string, unknown>[]).find(
      p => accepted.has(String(p['headRef'])) && p['status'] === 'active' && p['kind'] === 'item'
    );
    assert.ok(
      match,
      `No active item pin references "${contentId}" on ${peerName} — ` +
        `no household has declared consent to steward it (${(pins as unknown[]).length} pins listed).`
    );
  }
);

/**
 * Assert the per-EPR pull rollup reports caughtUp — the bytes-have-landed leg.
 * GET /api/v1/pins/{contentId}/pull (BARE id, per the namespace rule above).
 *
 * Byte-arrival, never inventory-arrival: `caughtUp` is the pull queue's own
 * verdict that the pinned bytes are on disk, not that a peer claims to hold
 * them (see project_inventory_exchange_not_byte_replication).
 *
 * Example:
 *   Then within 60 seconds doorway "alpha-A" reports the pull for "elohim-host-landing" is caught up
 */
Then(
  'within {int} seconds doorway {string} reports the pull for {string} is caught up',
  // See the 30s-vs-60s constraint note above: cucumber's default step timeout
  // would kill this at 30s, before the 60s poll's own governor fires.
  { timeout: 75_000 },
  async function (this: E2EWorld, seconds: number, peerName: string, contentId: string) {
    const doorway = this.getDoorway(peerName);
    await retry(
      async () => {
        const { status, text } = await getRaw(
          `${doorway.url}/api/v1/pins/${encodeURIComponent(contentId)}/pull`
        );
        assert.strictEqual(
          status,
          200,
          `GET /api/v1/pins/${contentId}/pull on ${peerName}: HTTP ${status} (body: ${text.slice(0, 120)})`
        );
        let body: Record<string, unknown>;
        try {
          body = JSON.parse(text) as Record<string, unknown>;
        } catch {
          throw new Error(
            `GET /api/v1/pins/${contentId}/pull on ${peerName}: response is not valid JSON`
          );
        }
        assert.strictEqual(
          body['caughtUp'],
          true,
          `pull rollup for "${contentId}" on ${peerName} is not caught up: ${JSON.stringify(body)}`
        );
      },
      {
        timeoutMs: seconds * 1000,
        initialDelayMs: 1000,
        backoffFactor: 1.2,
        maxDelayMs: 5000,
        maxAttempts: 1000,
      }
    );
  }
);

/**
 * Assert the rea-FACING commitment ledger carries at least one active row for the
 * given action — GET /api/v1/commitments/facing/rea.
 *
 * Deliberately a DIFFERENT surface from the chapter's finish-line step: that one
 * reads /api/v1/commitments (the `rea_commitments` PROJECTION); this one reads
 * the mishpat-sourced facing view. Same word, two homes, one mirror between them.
 * A green here with a red finish line localises the break to the mishpat->rea
 * mirror rather than to the notarize/announce/graduate path — which is exactly
 * the state alpha was in on 2026-07-26 (commitment uhCEkd2ZZTOht... active in
 * mishpat, never mirrored into rea_commitments).
 *
 * A 200 with zero matching rows is an honest FAILURE, not pending: the surface
 * works, nothing has been notarized.
 *
 * Example:
 *   Then within 60 seconds doorway "alpha-A" has at least one active "replicates-commons" commitment in the rea-facing ledger
 */
Then(
  'within {int} seconds doorway {string} has at least one active {string} commitment in the rea-facing ledger',
  { timeout: 75_000 },
  async function (this: E2EWorld, seconds: number, peerName: string, action: string) {
    const doorway = this.getDoorway(peerName);
    await retry(
      async () => {
        const { status, text } = await getRaw(`${doorway.url}/api/v1/commitments/facing/rea`);
        assert.strictEqual(
          status,
          200,
          `GET /api/v1/commitments/facing/rea on ${peerName}: HTTP ${status} (body: ${text.slice(0, 120)})`
        );
        let rows: unknown;
        try {
          rows = JSON.parse(text);
        } catch {
          throw new Error(
            `GET /api/v1/commitments/facing/rea on ${peerName}: response is not valid JSON: ${text.slice(0, 120)}`
          );
        }
        assert.ok(
          Array.isArray(rows),
          `GET /api/v1/commitments/facing/rea on ${peerName}: expected a bare JSON array, got: ${text.slice(0, 120)}`
        );
        const active = (rows as Record<string, unknown>[]).filter(
          r => r['action'] === action && r['state'] === 'active'
        );
        assert.ok(
          active.length >= 1,
          `No active "${action}" commitment in the rea-facing ledger on ${peerName} — ` +
            `the notarize -> announce -> graduate path has not completed ` +
            `(${(rows as unknown[]).length} rows, states: ${JSON.stringify(
              (rows as Record<string, unknown>[]).map(
                r => `${String(r['action'])}:${String(r['state'])}`
              )
            )}).`
        );
      },
      {
        timeoutMs: seconds * 1000,
        initialDelayMs: 1000,
        backoffFactor: 1.2,
        maxDelayMs: 5000,
        maxAttempts: 1000,
      }
    );
  }
);

// ---------------------------------------------------------------------------
// 3. Raw-body query pair (chapter 4)
// ---------------------------------------------------------------------------

interface RawCapture {
  status: number;
  text: string;
  url: string;
}
const rawCapture = new WeakMap<E2EWorld, RawCapture>();

/**
 * Hit a surface path on the named peer and store the raw HTTP status + body
 * text (not JSON-parsed) — for asserting on rendered HTML rather than a JSON
 * field. Self-contained: does not share state with dataplane.steps.ts's private
 * "When I query" capture or delivery.steps.ts's responseStore.
 *
 * Example:
 *   When I query "/" on peer "elohim.host" expecting raw text
 */
When(
  'I query {string} on peer {string} expecting raw text',
  async function (this: E2EWorld, path: string, peerName: string) {
    const doorway = this.getDoorway(peerName);
    const url = `${doorway.url}${path}`;
    const { status, text } = await getRaw(url);
    rawCapture.set(this, { status, text, url });
  }
);

Then('the raw response status is {int}', function (this: E2EWorld, expected: number) {
  const c = rawCapture.get(this);
  assert.ok(c, 'No raw response captured — run "When I query ... expecting raw text" first');
  assert.strictEqual(c.status, expected, `${c.url}: status ${c.status}, expected ${expected}`);
});

Then('the raw response body contains {string}', function (this: E2EWorld, needle: string) {
  const c = rawCapture.get(this);
  assert.ok(c, 'No raw response captured — run "When I query ... expecting raw text" first');
  assert.ok(
    c.text.includes(needle),
    `${c.url}: body does not contain "${needle}"; first 200 chars: ${c.text.slice(0, 200)}`
  );
});

// ---------------------------------------------------------------------------
// 4. Cross-doorway truth compare + rendered-card verdict (chapter 10)
// ---------------------------------------------------------------------------

/**
 * "Two doorways, one truth": read stewardingCollectives
 * (GET /api/v1/resilience/{contentId}/household, ResilienceSnapshotView — the
 * type the production handler `handle_get_household_resilience` actually
 * serializes via `household_resilience::snapshot_with_staleness_secs`) from TWO
 * peers and assert both (a) agree, and (b) are non-zero. Agreement alone would
 * false-pass a card that reads 0 identically everywhere; non-zero alone would
 * false-pass two doorways that silently diverge. Both together are the
 * felt-safety invariant.
 *
 * Example:
 *   Then peers "alpha-A" and "elohim.host" report the same non-zero stewardingCollectives for "elohim-host-landing"
 */
Then(
  'peers {string} and {string} report the same non-zero stewardingCollectives for {string}',
  async function (this: E2EWorld, peerA: string, peerB: string, contentId: string) {
    const readStewardingCollectives = async (peerName: string): Promise<number> => {
      const doorway = this.getDoorway(peerName);
      const { status, text } = await getRaw(
        `${doorway.url}/api/v1/resilience/${encodeURIComponent(contentId)}/household`
      );
      assert.strictEqual(
        status,
        200,
        `GET /api/v1/resilience/${contentId}/household on ${peerName}: HTTP ${status} (body: ${text.slice(0, 120)})`
      );
      let body: Record<string, unknown>;
      try {
        body = JSON.parse(text) as Record<string, unknown>;
      } catch {
        throw new Error(
          `GET /api/v1/resilience/${contentId}/household on ${peerName}: response is not valid JSON`
        );
      }
      const value = body['stewardingCollectives'];
      assert.ok(
        typeof value === 'number',
        `stewardingCollectives on ${peerName} is not a number: ${JSON.stringify(value)}`
      );
      return value;
    };

    const a = await readStewardingCollectives(peerA);
    const b = await readStewardingCollectives(peerB);
    assert.ok(
      a > 0,
      `stewardingCollectives on ${peerA} is ${a} — expected > 0 (the resilience card must not read zero)`
    );
    assert.strictEqual(
      a,
      b,
      `stewardingCollectives diverges: ${peerA}=${a} vs ${peerB}=${b} — the two doorways don't tell the same truth`
    );
  }
);

/**
 * One root, not two: read dhtAnchorHash (GET /db/content/{id}) from TWO peers
 * and assert both (a) are present (non-null, non-empty — each conductor holds
 * a notarized entry) and (b) are EQUAL (one converged root). Presence alone
 * false-passes the two-root split discovered 2026-07-26 (alpha-A uhCkkh_Gb… vs
 * elohim.host uhCkkl4C9… — each doorway independently rooted the same content
 * id, which is exactly why the canonical-head declare channel is refused with
 * "no content found": the OTHER peer's entry isn't resolvable locally). This is
 * the interstitial node between upload and heads-converge — a red here explains
 * a red on chapter 10's cross-doorway compare.
 *
 * Example:
 *   Then peers "alpha-A" and "elohim.host" hold the same DHT anchor for content "elohim-host-landing"
 */
Then(
  'peers {string} and {string} hold the same DHT anchor for content {string}',
  async function (this: E2EWorld, peerA: string, peerB: string, contentId: string) {
    const readAnchor = async (peerName: string): Promise<string> => {
      const doorway = this.getDoorway(peerName);
      const { status, text } = await getRaw(
        `${doorway.url}/db/content/${encodeURIComponent(contentId)}`
      );
      assert.strictEqual(
        status,
        200,
        `GET /db/content/${contentId} on ${peerName}: HTTP ${status} (body: ${text.slice(0, 120)})`
      );
      let body: Record<string, unknown>;
      try {
        body = JSON.parse(text) as Record<string, unknown>;
      } catch {
        throw new Error(`GET /db/content/${contentId} on ${peerName}: response is not valid JSON`);
      }
      const anchor = body['dhtAnchorHash'];
      assert.ok(
        typeof anchor === 'string' && anchor.length > 0,
        `dhtAnchorHash on ${peerName} is missing/null for ${contentId} — the conductor holds no notarized entry`
      );
      return anchor;
    };

    const a = await readAnchor(peerA);
    const b = await readAnchor(peerB);
    assert.strictEqual(
      a,
      b,
      `DHT anchor diverges for ${contentId}: ${peerA}=${a} vs ${peerB}=${b} — two independently-rooted entries, not one converged root`
    );
  }
);

/**
 * ADOPT, don't re-author: read the notary HEAD (GET /db/content/{id}/head,
 * ContentHeadView) from the adopting peer and the declaring peer, and assert the
 * adopter (a) reports `declared: true` and (b) names the SAME `headActionHash`.
 *
 * Both halves are load-bearing and neither alone is sufficient:
 *
 * - `headActionHash` equality ALONE false-passes, because that field FALLS BACK
 *   to `dhtAnchorHash` when no declaration exists (see
 *   `content_head_view_from_content`). Two peers that merely converged their
 *   anchors would pass without any declaration having been adopted at all.
 * - `declared: true` ALONE false-passes the defect this station exists to catch:
 *   before adopt-before-author, the adopting peer's restart sweeps authored a
 *   fresh local root and CROWNED IT, so `declared` was true while pointing at
 *   the peer's own competing root — self-election reading as convergence.
 *
 * Deliberately WEAKER than the anchor-equality finish line that follows it: the
 * adopter need not hold the declarer's root to adopt its declaration (that is
 * the whole point of declare-carries-Record). So this station can — and should —
 * flip green BEFORE anchor equality does.
 *
 * Example:
 *   Then peer "elohim.host" resolves the declared head for content "elohim-host-landing" equal to peer "alpha-A"
 */
Then(
  'peer {string} resolves the declared head for content {string} equal to peer {string}',
  async function (this: E2EWorld, adopter: string, contentId: string, declarer: string) {
    const readHead = async (
      peerName: string
    ): Promise<{ headActionHash: string; declared: boolean }> => {
      const doorway = this.getDoorway(peerName);
      const { status, text } = await getRaw(
        `${doorway.url}/db/content/${encodeURIComponent(contentId)}/head`
      );
      assert.strictEqual(
        status,
        200,
        `GET /db/content/${contentId}/head on ${peerName}: HTTP ${status} (body: ${text.slice(0, 120)})`
      );
      let body: Record<string, unknown>;
      try {
        body = JSON.parse(text) as Record<string, unknown>;
      } catch {
        throw new Error(
          `GET /db/content/${contentId}/head on ${peerName}: response is not valid JSON`
        );
      }
      const headActionHash = body['headActionHash'];
      assert.ok(
        typeof headActionHash === 'string' && headActionHash.length > 0,
        `headActionHash on ${peerName} is missing/empty for ${contentId} — the notary has no HEAD answer`
      );
      return { headActionHash, declared: body['declared'] === true };
    };

    const declared = await readHead(declarer);
    assert.ok(
      declared.declared,
      `${declarer} reports declared=false for ${contentId} — there is no canonical declaration for ${adopter} to adopt yet, ` +
        `so this station cannot distinguish adoption from anchor convergence`
    );

    const adopted = await readHead(adopter);
    assert.ok(
      adopted.declared,
      `${adopter} reports declared=false for ${contentId} — it holds only an anchor fallback, ` +
        `never having adopted ${declarer}'s canonical head`
    );
    assert.strictEqual(
      adopted.headActionHash,
      declared.headActionHash,
      `declared head diverges for ${contentId}: ${adopter}=${adopted.headActionHash} vs ${declarer}=${declared.headActionHash} — ` +
        `${adopter} crowned its own root instead of adopting ${declarer}'s declaration`
    );
  }
);

const lookResults = new WeakMap<E2EWorld, LookResult>();
const lookErrors = new WeakMap<E2EWorld, string>();

/**
 * Render the resilience card's host surface (/resource/{contentId}, where
 * <app-protocol-omni> mounts the resilience snapshot) via the shared `runLook`
 * primitive (scripts/look.ts — same capture PlaywrightDevice used by the cucumber
 * suite in browser mode). @wip: launches a real headless browser, which the
 * plain-cucumber-js Dataplane Validation stage does not provision — kept as
 * documented future work per the plan, never executed there.
 *
 * Example:
 *   When I render the resilience card for "elohim-host-landing" on peer "elohim.host"
 */
When(
  'I render the resilience card for {string} on peer {string}',
  async function (this: E2EWorld, contentId: string, peerName: string) {
    const doorway = this.getDoorway(peerName);
    const url = `${doorway.url}/resource/${encodeURIComponent(contentId)}`;
    try {
      const result = await runLook({
        url,
        out: `resiliency-saga-ch10-${peerName}`,
        waitTestid: PROTOCOL_OMNI.CHIP,
      });
      lookResults.set(this, result);
    } catch (err) {
      lookErrors.set(this, err instanceof Error ? err.message : String(err));
    }
  }
);

Then('the rendered card capture has no HTTP errors and a screenshot', function (this: E2EWorld) {
  const result = lookResults.get(this);
  assert.ok(result, `runLook failed to complete: ${lookErrors.get(this) ?? 'unknown error'}`);
  assert.strictEqual(
    result.httpErrors.length,
    0,
    `rendered card had HTTP errors: ${JSON.stringify(result.httpErrors)}`
  );
  assert.ok(existsSync(result.shotPath), `no screenshot written at ${result.shotPath}`);
});
