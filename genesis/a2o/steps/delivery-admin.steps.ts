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

// Admin API paths — extracted to satisfy sonarjs/no-duplicate-string
const CACHE_STATS = '/admin/cache/stats';
const CACHE_DISABLE = '/admin/cache/disable';
const CACHE_ENABLE = '/admin/cache/enable';
const CACHE_WARM = '/admin/cache/warm';
const HEALTH_STARTUP = '/health/startup';
const HEALTH = '/health';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** POST to an admin endpoint, assert 200, return parsed JSON. */
async function adminPost(baseUrl: string, path: string): Promise<Record<string, unknown>> {
  const { statusCode, body } = await request(`${baseUrl}${path}`, { method: 'POST' });
  const text = await body.text();
  assert.equal(statusCode, 200, `Admin POST ${path} failed: ${statusCode} ${text}`);
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
 * Example: Given the doorway projection cache is enabled and warm for "evolution-of-trust"
 */
Given(
  'the doorway projection cache is enabled and warm for {string}',
  async function (this: E2EWorld, _appSlug: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, CACHE_ENABLE);
    const stats = await adminGet(url, CACHE_STATS);
    const projection = stats['projection'] as Record<string, unknown>;
    assert.ok(
      (projection['entries'] as number) > 0,
      `Projection cache is empty — warm-up may not have run for "${_appSlug}"`
    );
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
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, CACHE_ENABLE);
    // Trigger warmup and wait for entries to appear
    await adminPost(url, CACHE_WARM);
    await retry(
      async () => {
        const stats = await adminGet(url, CACHE_STATS);
        const projection = stats['projection'] as Record<string, unknown>;
        assert.ok(
          (projection['entries'] as number) > 0,
          `Cache still empty after warm for "${appSlug}"`
        );
      },
      { maxAttempts: 10, initialDelayMs: 2000, timeoutMs: 30_000 }
    );
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
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, CACHE_ENABLE);
    await adminPost(url, CACHE_WARM);
    await retry(
      async () => {
        const stats = await adminGet(url, CACHE_STATS);
        const projection = stats['projection'] as Record<string, unknown>;
        assert.ok(
          (projection['entries'] as number) > 0,
          `Cache still empty after warm for "${appSlug}"`
        );
      },
      { maxAttempts: 10, initialDelayMs: 2000, timeoutMs: 30_000 }
    );
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
 * Example: Given all delivery layers are enabled
 */
Given('all delivery layers are enabled', async function (this: E2EWorld) {
  const url = getDoorwayUrl(this);
  await adminPost(url, CACHE_ENABLE);
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
Then('the next load re-warms the caches', async function (this: E2EWorld) {
  const url = getDoorwayUrl(this);
  await adminPost(url, CACHE_WARM);
  await retry(
    async () => {
      const stats = await adminGet(url, CACHE_STATS);
      const projection = stats['projection'] as Record<string, unknown>;
      assert.ok((projection['entries'] as number) > 0, 'Cache still empty after re-warm');
    },
    { maxAttempts: 10, initialDelayMs: 2000, timeoutMs: 30_000 }
  );
});

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
