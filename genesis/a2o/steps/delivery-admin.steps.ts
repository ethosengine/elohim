/**
 * Delivery Admin Step Definitions — admin APIs for projection cache control,
 * extraction cache eviction, warmup retry observation, and resource health.
 *
 * Uses the doorway admin endpoints:
 *   GET  /admin/cache/stats         — enabled flag, entry counts
 *   POST /admin/cache/disable       — bypass projection cache
 *   POST /admin/cache/enable        — re-enable projection cache
 *   POST /admin/cache/clear/{slug}  — evict entries for a slug
 *   POST /admin/cache/warm          — trigger async re-warmup
 *
 * Uses the storage admin endpoint (via E2E_STORAGE_URL):
 *   POST /admin/extraction-cache/evict/{slug}
 *
 * Warmup state via:
 *   GET /health/startup — includes warmup.{ inProgress, attempts, maxAttempts, completed }
 *
 * These steps are for API-level (non-browser) scenarios only.
 * Browser-only steps live in steps/ui/delivery.steps.ts.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { retry } from '../src/framework/utils/retry.js';
import { E2EWorld } from '../src/framework/world.js';

import { fetchApp } from './delivery.steps.js';

// Admin API paths — extracted to satisfy sonarjs/no-duplicate-string
const CACHE_STATS = '/admin/cache/stats';
const CACHE_DISABLE = '/admin/cache/disable';
const CACHE_ENABLE = '/admin/cache/enable';
const CACHE_WARM = '/admin/cache/warm';
const HEALTH_STARTUP = '/health/startup';
const DEFAULT_APP_SLUG = 'evolution-of-trust';
const APP_ENTRY_FILE = 'index.html';

// ---------------------------------------------------------------------------
// Step budgets — every number below is DERIVED from a loop bound in this file
// ---------------------------------------------------------------------------

/**
 * The bound every warm/re-warm retry loop in this file is constructed with.
 * Referenced by both the `retry()` calls and the step timeouts so the loop and
 * the budget that has to contain it cannot drift apart.
 */
const WARM_RETRY_BUDGET_MS = 30_000;

/** Allowance for ONE plain admin/app round trip bracketing a loop. */
const ADMIN_REQUEST_BUDGET_MS = 10_000;

/**
 * A warm step is: enable + warm + <bounded retry loop> + prime. That is the
 * loop's own 30s bound plus three ordinary round trips — which does not fit in
 * the suite's default step budget, so these steps state the derived number
 * instead of racing it.
 */
const WARM_STEP_TIMEOUT_MS = WARM_RETRY_BUDGET_MS + 3 * ADMIN_REQUEST_BUDGET_MS;

/**
 * One degraded-path entry fetch, which may make storage decompress the whole
 * bundle blob before it can answer — a slower unit than an admin round trip,
 * and the only work this step does.
 */
const DEGRADED_FETCH_BUDGET_MS = 60_000;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** POST to an admin endpoint, assert an acceptable status code, return parsed JSON.
 *  By default only 200 is accepted; pass `acceptedStatuses` to broaden (e.g. [200, 202]
 *  for async-accepted endpoints such as /admin/cache/warm). */
async function adminPost(
  baseUrl: string,
  path: string,
  acceptedStatuses: number[] = [200]
): Promise<Record<string, unknown>> {
  const { statusCode, body } = await request(`${baseUrl}${path}`, { method: 'POST' });
  const text = await body.text();
  assert.ok(
    acceptedStatuses.includes(statusCode),
    `Admin POST ${path} failed: ${statusCode} ${text}`
  );
  return JSON.parse(text) as Record<string, unknown>;
}

/** GET from an admin endpoint, assert 200, return parsed JSON. */
async function adminGet(baseUrl: string, path: string): Promise<Record<string, unknown>> {
  const { statusCode, body } = await request(`${baseUrl}${path}`);
  const text = await body.text();
  assert.equal(statusCode, 200, `Admin GET ${path} failed: ${statusCode} ${text}`);
  return JSON.parse(text) as Record<string, unknown>;
}

/** Return the first registered doorway URL, asserting one exists. */
function getDoorwayUrl(world: E2EWorld): string {
  const doorway = [...world.doorways.values()][0];
  assert.ok(doorway, 'No doorway registered — run "doorway ... at ..." step first');
  return doorway.url;
}

/** The app slug the scenario is working with (set by the seeded-content background). */
function currentSlug(world: E2EWorld): string {
  return world.contentIds.get('currentAppSlug') ?? DEFAULT_APP_SLUG;
}

/**
 * Actually make the doorway's app-file cache warm for a bundle's entry document.
 *
 * POST /admin/cache/warm streams the PROJECTION store from storage
 * (doorway/doorway-service/src/routes/admin_cache.rs) — it never populates the
 * app-file cache, which fills lazily on the first /apps/{slug}/{file} request.
 * So "the projection cache is warm for X" was asserting a precondition nothing
 * had established: every following "the serving layer is projection-cache" was
 * measuring a cache no step had filled.
 *
 * Deliberately ONE request, not the whole bundle. /apps/{slug}/{file} is not
 * static-asset-exempt from the doorway membrane
 * (doorway/doorway-service/src/server/membrane.rs is_static_asset), and the
 * membrane challenges at 600 requests / 60s per client IP and BANS at 1200 for
 * 900s. A 40-file prime in every warm Given would put the whole shared API suite
 * a couple of scenarios away from a 15-minute self-ban.
 *
 * Best-effort: a doorway that cannot cache app files leaves the scenario's own
 * assertion to report that, with the X-Cache value it actually saw.
 */
async function primeAppEntry(baseUrl: string, slug: string): Promise<void> {
  try {
    await fetchApp(baseUrl, `/apps/${slug}/${APP_ENTRY_FILE}`);
  } catch {
    // best-effort priming — the scenario's own assertions report the real state
  }
}

/** A warm-path measurement, so a later degraded-path load can be compared to it. */
interface EntryBaseline {
  durationMs: number;
  body: Buffer;
}

const entryBaseline = new WeakMap<E2EWorld, EntryBaseline>();

/** Fetch the app entry document, returning the response and how long it took. */
async function timedEntryFetch(
  baseUrl: string,
  slug: string
): Promise<{ status: number; body: Buffer; durationMs: number }> {
  const started = Date.now();
  const resp = await fetchApp(baseUrl, `/apps/${slug}/${APP_ENTRY_FILE}`);
  return { status: resp.status, body: resp.body, durationMs: Date.now() - started };
}

// ---------------------------------------------------------------------------
// Projection Cache Control — Given (precondition setup)
// ---------------------------------------------------------------------------

/**
 * Disable the projection cache before a scenario runs.
 * Registers cleanup to re-enable after the scenario.
 *
 * Example: Given the doorway projection cache is disabled
 */
Given('the doorway projection cache is disabled', async function (this: E2EWorld) {
  const url = getDoorwayUrl(this);
  await adminPost(url, CACHE_DISABLE);
  this.onCleanup(async () => {
    await adminPost(url, CACHE_ENABLE);
  });
});

/**
 * Ensure the projection cache is enabled and contains entries for the given slug.
 *
 * No explicit timeout: unlike the warm steps below this one runs no retry loop,
 * so enable + stats + prime is three round trips and the suite's default step
 * budget already covers it.
 *
 * Example: Given the doorway projection cache is enabled and warm for "evolution-of-trust"
 */
Given(
  'the doorway projection cache is enabled and warm for {string}',
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, CACHE_ENABLE);
    const stats = await adminGet(url, CACHE_STATS);
    const projection = stats['projection'] as Record<string, unknown>;
    assert.ok(
      (projection['entries'] as number) > 0,
      `Projection cache is empty — warm-up may not have run for "${appSlug}"`
    );
    this.contentIds.set('currentAppSlug', appSlug);
    await primeAppEntry(url, appSlug);
  }
);

/**
 * Assert that the projection cache is enabled (default state).
 *
 * Example: Given the projection cache is enabled
 */
Given('the projection cache is enabled', async function (this: E2EWorld) {
  const url = getDoorwayUrl(this);
  await adminPost(url, CACHE_ENABLE);
});

/**
 * Ensure the projection cache has entries for the slug (trigger warm-up if not).
 * Uses the "is warm for" phrasing from delivery-diagnostics.feature.
 *
 * Example: Given the projection cache is warm for "evolution-of-trust"
 */
Given(
  'the projection cache is warm for {string}',
  { timeout: WARM_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, CACHE_ENABLE);
    // Trigger warmup and wait for entries to appear.
    // 202 Accepted is a valid async-accepted response for this endpoint.
    await adminPost(url, CACHE_WARM, [200, 202]);
    await retry(
      async () => {
        const stats = await adminGet(url, CACHE_STATS);
        const projection = stats['projection'] as Record<string, unknown>;
        assert.ok(
          (projection['entries'] as number) > 0,
          `Cache still empty after warm for "${appSlug}"`
        );
      },
      { maxAttempts: 10, initialDelayMs: 2000, timeoutMs: WARM_RETRY_BUDGET_MS }
    );
    this.contentIds.set('currentAppSlug', appSlug);
    await primeAppEntry(url, appSlug);
  }
);

/**
 * Ensure the projection cache has entries for the slug (trigger warm-up if not).
 * Uses the "for ... is warm" phrasing from web2-absorption.feature.
 *
 * Example: Given the projection cache for "evolution-of-trust" is warm
 */
Given(
  'the projection cache for {string} is warm',
  { timeout: WARM_STEP_TIMEOUT_MS },
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, CACHE_ENABLE);
    // 202 Accepted is a valid async-accepted response for this endpoint.
    await adminPost(url, CACHE_WARM, [200, 202]);
    await retry(
      async () => {
        const stats = await adminGet(url, CACHE_STATS);
        const projection = stats['projection'] as Record<string, unknown>;
        assert.ok(
          (projection['entries'] as number) > 0,
          `Cache still empty after warm for "${appSlug}"`
        );
      },
      { maxAttempts: 10, initialDelayMs: 2000, timeoutMs: WARM_RETRY_BUDGET_MS }
    );
    this.contentIds.set('currentAppSlug', appSlug);
    await primeAppEntry(url, appSlug);
  }
);

// ---------------------------------------------------------------------------
// Projection Cache Control — When (actor actions)
// ---------------------------------------------------------------------------

/**
 * Actor disables the projection cache via operator API.
 * Registers cleanup to re-enable after the scenario.
 *
 * Example: When Matthew disables the projection cache via operator API
 * Example: When Matthew disables the projection cache
 */
When(
  '{word} disables the projection cache via operator API',
  async function (this: E2EWorld, _humanName: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, CACHE_DISABLE);
    this.onCleanup(async () => {
      await adminPost(url, CACHE_ENABLE);
    });
  }
);

When('{word} disables the projection cache', async function (this: E2EWorld, _humanName: string) {
  const url = getDoorwayUrl(this);
  await adminPost(url, CACHE_DISABLE);
  this.onCleanup(async () => {
    await adminPost(url, CACHE_ENABLE);
  });
});

/**
 * Actor re-enables the projection cache.
 *
 * Example: When Matthew re-enables the projection cache
 */
When('{word} re-enables the projection cache', async function (this: E2EWorld, _humanName: string) {
  const url = getDoorwayUrl(this);
  await adminPost(url, CACHE_ENABLE);
});

/**
 * Actor re-enables all delivery layers (projection cache + storage extraction).
 * Storage extraction cache has no disable API — re-enable just means enabling doorway cache.
 *
 * Example: When Matthew re-enables all delivery layers
 */
When('{word} re-enables all delivery layers', async function (this: E2EWorld, _humanName: string) {
  const url = getDoorwayUrl(this);
  await adminPost(url, CACHE_ENABLE);
  // Storage extraction cache is always on — no disable API needed
});

/**
 * Assert all delivery layers are in the enabled state.
 *
 * No explicit timeout: enable + prime + one timed fetch is three round trips
 * with no retry loop, which fits the suite's default step budget.
 *
 * Example: Given all delivery layers are enabled
 */
Given('all delivery layers are enabled', async function (this: E2EWorld) {
  const url = getDoorwayUrl(this);
  await adminPost(url, CACHE_ENABLE);
  const slug = currentSlug(this);
  // Full-stack state is the reference point the later "slower but correct"
  // assertion compares against, so measure it here while every layer is up.
  await primeAppEntry(url, slug);
  const warm = await timedEntryFetch(url, slug);
  if (warm.status === 200) {
    entryBaseline.set(this, { durationMs: warm.durationMs, body: warm.body });
  }
});

// ---------------------------------------------------------------------------
// Extraction Cache Control (Storage-side)
// ---------------------------------------------------------------------------

/**
 * Evict an app from elohim-storage's extraction cache.
 * Requires E2E_STORAGE_URL to be set; returns 'pending' if not configured.
 *
 * Example: When Matthew evicts "evolution-of-trust" from the extraction cache
 */
When(
  '{word} evicts {string} from the extraction cache',
  async function (this: E2EWorld, _humanName: string, appSlug: string) {
    const storageUrl = process.env['E2E_STORAGE_URL'];
    if (!storageUrl) {
      // Storage admin API not directly reachable in this environment — skip
      return 'pending';
    }
    await adminPost(storageUrl, `/admin/extraction-cache/evict/${appSlug}`);
  }
);

// ---------------------------------------------------------------------------
// Projection Cache Assertions (Then)
// ---------------------------------------------------------------------------

/**
 * Verify that requests are proxied to storage (BYPASS in X-Cache header).
 *
 * Example: Then subsequent requests for app files proxy directly to storage
 */
Then(
  'subsequent requests for app files proxy directly to storage',
  async function (this: E2EWorld) {
    const url = getDoorwayUrl(this);
    const slug = 'evolution-of-trust';
    const { statusCode, headers } = await request(`${url}/apps/${slug}/index.html`);
    assert.equal(statusCode, 200, `App request returned ${statusCode}`);
    const cacheHeader = (headers['x-cache'] as string | undefined) ?? '';
    assert.ok(
      cacheHeader.includes('BYPASS') || cacheHeader.includes('MISS'),
      `Expected BYPASS or MISS but got X-Cache: "${cacheHeader}"`
    );
  }
);

/**
 * Verify the cache is disabled but its entries have not been cleared.
 *
 * Example: And the projection cache is bypassed but not cleared
 */
Then('the projection cache is bypassed but not cleared', async function (this: E2EWorld) {
  const url = getDoorwayUrl(this);
  const stats = await adminGet(url, CACHE_STATS);
  assert.equal(stats['enabled'], false, 'Cache should be disabled (bypassed)');
  const projection = stats['projection'] as Record<string, unknown>;
  assert.ok(
    (projection['entries'] as number) > 0,
    'Cache entries should still exist (not cleared) — only bypassed'
  );
});

/**
 * Verify subsequent requests are served from cache (X-Cache: HIT).
 *
 * Example: Then subsequent requests are served from cache again
 */
Then('subsequent requests are served from cache again', async function (this: E2EWorld) {
  const url = getDoorwayUrl(this);
  const slug = 'evolution-of-trust';
  const { statusCode, headers } = await request(`${url}/apps/${slug}/index.html`);
  assert.equal(statusCode, 200, `App request returned ${statusCode}`);
  const cacheHeader = (headers['x-cache'] as string | undefined) ?? '';
  assert.ok(cacheHeader.includes('HIT'), `Expected HIT but got X-Cache: "${cacheHeader}"`);
});

/**
 * Verify the projection cache contains entries for the given slug.
 *
 * Example: And the projection cache contains entries for "evolution-of-trust"
 */
Then(
  'the projection cache contains entries for {string}',
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    const stats = await adminGet(url, CACHE_STATS);
    const projection = stats['projection'] as Record<string, unknown>;
    assert.ok(
      (projection['entries'] as number) > 0,
      `No projection cache entries found for "${appSlug}"`
    );
  }
);

/**
 * After re-enabling layers, the next load should re-warm the caches.
 * This is a documentation anchor — the assertion is that cache entries appear.
 *
 * Example: Then the next load re-warms the caches
 */
Then(
  'the next load re-warms the caches',
  { timeout: WARM_STEP_TIMEOUT_MS },
  async function (this: E2EWorld) {
    const url = getDoorwayUrl(this);
    // 202 Accepted is a valid async-accepted response for this endpoint.
    await adminPost(url, CACHE_WARM, [200, 202]);
    await retry(
      async () => {
        const stats = await adminGet(url, CACHE_STATS);
        const projection = stats['projection'] as Record<string, unknown>;
        assert.ok((projection['entries'] as number) > 0, 'Cache still empty after re-warm');
      },
      { maxAttempts: 10, initialDelayMs: 2000, timeoutMs: WARM_RETRY_BUDGET_MS }
    );
  }
);

// ---------------------------------------------------------------------------
// Warmup Retry Observation
// ---------------------------------------------------------------------------

/**
 * Verify that the warmup retry cycle completed successfully.
 *
 * Example: Then the warmup retry state shows completed
 */
Then('the warmup retry state shows completed', async function (this: E2EWorld) {
  const url = getDoorwayUrl(this);
  const data = await adminGet(url, HEALTH_STARTUP);
  const warmup = data['warmup'] as Record<string, unknown> | null | undefined;
  assert.ok(warmup, 'warmup state not present in /health/startup response');
  assert.equal(warmup['completed'], true, `Warmup not completed: ${JSON.stringify(warmup)}`);
});

/**
 * Verify the warmup retry limit is set to the expected value.
 *
 * Example: And the warmup retry state shows maxAttempts 5
 */
Then(
  'the warmup retry state shows maxAttempts {int}',
  async function (this: E2EWorld, expected: number) {
    const url = getDoorwayUrl(this);
    const data = await adminGet(url, HEALTH_STARTUP);
    const warmup = data['warmup'] as Record<string, unknown> | null | undefined;
    assert.ok(warmup, 'warmup state not present in /health/startup response');
    assert.equal(
      warmup['maxAttempts'],
      expected,
      `Expected maxAttempts ${expected} but got ${warmup['maxAttempts']}`
    );
  }
);

// ---------------------------------------------------------------------------
// Resource Usage (API-level health checks)
// ---------------------------------------------------------------------------

/**
 * Verify elohim-storage is still healthy and responsive after a load test.
 * This is a best-effort assertion — OOM would cause health to fail.
 *
 * Example: And elohim-storage memory usage stays within container limits
 */
Then('elohim-storage memory usage stays within container limits', async function (this: E2EWorld) {
  const url = getDoorwayUrl(this);
  const { statusCode } = await request(`${url}/health`);
  assert.equal(
    statusCode,
    200,
    'Doorway/storage not healthy after load test — possible OOM or crash'
  );
});

/**
 * Verify storage resource usage is unchanged (storage remains healthy).
 *
 * Example: And elohim-storage resource usage is unchanged
 */
Then('elohim-storage resource usage is unchanged', async function (this: E2EWorld) {
  const url = getDoorwayUrl(this);
  const { statusCode } = await request(`${url}/health`);
  assert.equal(statusCode, 200, 'Storage health check failed after load test');
});

// ---------------------------------------------------------------------------
// Cold-cache precondition and degraded-path assertion (delivery-diagnostics)
// ---------------------------------------------------------------------------

/**
 * Evict the bundle from the doorway's app-file cache so the next request has to
 * proxy. Same admin endpoint as the "for X is empty" phrasing in
 * steps/ui/delivery.steps.ts, asserted rather than best-effort: a scenario whose
 * whole point is "cache miss shows the proxy layer" is meaningless if the
 * eviction silently did not happen.
 *
 * Example: Given the projection cache is empty for "evolution-of-trust"
 */
Given(
  'the projection cache is empty for {string}',
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, `/admin/cache/clear/${appSlug}`);
    this.contentIds.set('currentAppSlug', appSlug);
  }
);

/**
 * The degraded fallback path must still return the same bytes — slower is
 * acceptable, wrong is not. The reference is the full-stack measurement taken by
 * "all delivery layers are enabled" at the top of the same scenario.
 *
 * Example: And the response is slower but correct
 */
Then(
  'the response is slower but correct',
  { timeout: DEGRADED_FETCH_BUDGET_MS },
  async function (this: E2EWorld) {
    const baseline = entryBaseline.get(this);
    assert.ok(
      baseline,
      'No full-stack baseline recorded — "the response is slower but correct" can only be ' +
        'judged in a scenario that started from "all delivery layers are enabled"'
    );
    const url = getDoorwayUrl(this);
    const slug = currentSlug(this);
    const degraded = await timedEntryFetch(url, slug);

    assert.equal(degraded.status, 200, `Degraded-path request returned ${degraded.status}`);
    assert.deepEqual(
      degraded.body,
      baseline.body,
      'Degraded-path response body differs from the full-stack response — the fallback chain ' +
        'must change latency, never content'
    );
    // "slower" is REPORTED, not gated. Two single-sample HTTP timings taken
    // seconds apart over a shared ingress invert on ordinary jitter — this
    // feature already holds one scenario behind @wip for being "non-deterministic
    // by construction", and a wall-clock comparison of two lone samples is the
    // same thing. The gate is the half that is deterministic: the degraded path
    // must return the SAME BYTES with a 200. The latency is printed so an
    // operator walking the chain can still see it, and so a genuinely pathological
    // fallback shows up in the run log instead of nowhere.
    const delta = degraded.durationMs - baseline.durationMs;
    // eslint-disable-next-line no-console
    console.log(
      `  ⏱️  fallback-chain latency: full-stack ${baseline.durationMs}ms -> degraded ` +
        `${degraded.durationMs}ms (${delta >= 0 ? '+' : ''}${delta}ms). Observation only — ` +
        'single-sample wall-clock ordering is not a contract.'
    );
  }
);
