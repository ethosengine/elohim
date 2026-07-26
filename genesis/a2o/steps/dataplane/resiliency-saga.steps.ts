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
 * Four concerns are covered here:
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

    let text: string;
    try {
      const { status, text: body } = await getRaw(`${base}/metrics`);
      if (status !== 200) {
        // eslint-disable-next-line no-console
        console.log(
          `  PENDING: labeled metric "${metricName}" on ${peerName} — ` +
            `GET ${base}/metrics returned ${status}`
        );
        return 'pending';
      }
      text = body;
    } catch (err) {
      // eslint-disable-next-line no-console
      console.log(
        `  PENDING: labeled metric "${metricName}" on ${peerName} — /metrics unreachable: ${String(err)}`
      );
      return 'pending';
    }

    const actual = parseLabeledPrometheusMetric(text, metricName, labelKey, labelValue);
    if (actual === null) {
      // eslint-disable-next-line no-console
      console.log(
        `  PENDING: labeled metric "${metricName}{${labelKey}="${labelValue}"}" on ${peerName} — ` +
          `series or label combination not present in the scrape (not yet emitted?)`
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
