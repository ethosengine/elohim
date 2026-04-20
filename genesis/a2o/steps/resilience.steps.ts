/**
 * Resilience step definitions — Plan 1 observable auto-distribute acceptance criteria.
 *
 * Covers:
 *   - Household-scoped placement assertions via /api/v1/resilience/{id}/household
 *   - Placement gap detection via /api/v1/placement-gaps
 *   - Content ingest preconditions
 *   - Browser-layer assertions are @wip and exercised in Task 19 / Plan 5 chaos runs
 *
 * Storage URL resolves from E2E_STORAGE_URL (defaults to http://localhost:8090).
 * The doorway background step is handled by mode-aware.steps.ts.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { retry } from '../src/framework/utils/retry.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Resolve the storage base URL from env. */
function storageUrl(): string {
  return process.env['E2E_STORAGE_URL'] ?? 'http://localhost:8090';
}

/** GET a path from elohim-storage, return parsed JSON or throw. */
async function storageGet(path: string): Promise<Record<string, unknown>> {
  const { statusCode, body } = await request(`${storageUrl()}${path}`);
  const text = await body.text();
  assert.equal(statusCode, 200, `GET ${path} failed: ${statusCode} ${text}`);
  return JSON.parse(text) as Record<string, unknown>;
}

/** POST JSON body to elohim-storage, return parsed JSON or throw. */
async function storagePost(
  path: string,
  payload: Record<string, unknown>
): Promise<Record<string, unknown>> {
  const { statusCode, body } = await request(`${storageUrl()}${path}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(payload),
  });
  const text = await body.text();
  assert.ok(statusCode < 300, `POST ${path} failed: ${statusCode} ${text}`);
  return JSON.parse(text) as Record<string, unknown>;
}

// Store last polled JSON body between steps (per-scenario via world).
const lastResponseKey = Symbol('resilience:lastResponse');

function storeResponse(world: E2EWorld, data: Record<string, unknown>): void {
  (world as unknown as Record<symbol, unknown>)[lastResponseKey] = data;
}

function loadResponse(world: E2EWorld): Record<string, unknown> {
  const data = (world as unknown as Record<symbol, unknown>)[lastResponseKey];
  assert.ok(data, 'No resilience response stored — run a polling step first');
  return data as Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

/**
 * Assert that elohim-storage is reachable, resolving the URL from an env var
 * or treating the argument as a literal URL.
 *
 * Example: And elohim-storage is reachable at "E2E_STORAGE_URL"
 */
Given(
  'elohim-storage is reachable at {string}',
  async function (this: E2EWorld, urlOrEnv: string) {
    const url = process.env[urlOrEnv] ?? urlOrEnv;
    const { statusCode } = await request(`${url}/api/v1/health`);
    assert.ok(
      statusCode === 200,
      `elohim-storage not reachable at ${url} (status ${statusCode})`
    );
  }
);

// ---------------------------------------------------------------------------
// Given — cluster preconditions
// ---------------------------------------------------------------------------

/**
 * Verify that the running cluster has at least `count` distinct households
 * each with an active provide commitment for the given reach.
 *
 * This is a precondition check, not a setup action — the cluster must already
 * be seeded. Uses /api/v1/peers/delivery to count distinct household_ids
 * among peers whose commitments include the requested reach.
 *
 * Example:
 *   Given the cluster has peers in at least 2 distinct households each with an active "commons" provide commitment
 */
Given(
  'the cluster has peers in at least {int} distinct households each with an active {string} provide commitment',
  async function (this: E2EWorld, minCount: number, reach: string) {
    const peers = (await storageGet('/api/v1/peers/delivery')) as unknown as Array<
      Record<string, unknown>
    >;

    const committed = peers.filter(p => {
      const commitments = (p['commitments'] as string[] | undefined) ?? [];
      return commitments.includes(reach);
    });

    const households = new Set(committed.map(p => p['householdId'] as string).filter(Boolean));

    assert.ok(
      households.size >= minCount,
      `Expected ≥${minCount} households with an active "${reach}" provide commitment; ` +
        `found ${households.size}. Seed or configure the cluster before running this scenario.`
    );
  }
);

/**
 * Verify that the cluster has peers in exactly 2 households but only `committed`
 * of them has an active provide commitment for `reach`.
 *
 * Example:
 *   Given the cluster has peers in 2 households but only 1 has an active "commons" provide commitment
 */
Given(
  'the cluster has peers in {int} households but only {int} has an active {string} provide commitment',
  async function (this: E2EWorld, _totalHouseholds: number, committedCount: number, reach: string) {
    const peers = (await storageGet('/api/v1/peers/delivery')) as unknown as Array<
      Record<string, unknown>
    >;

    const committed = peers.filter(p => {
      const commitments = (p['commitments'] as string[] | undefined) ?? [];
      return commitments.includes(reach);
    });

    const households = new Set(committed.map(p => p['householdId'] as string).filter(Boolean));

    assert.equal(
      households.size,
      committedCount,
      `Expected exactly ${committedCount} household(s) with "${reach}" commitments; ` +
        `found ${households.size}.`
    );
  }
);

/**
 * Verify that a given content item has been distributed to at least N households
 * (used by @wip UI scenarios as a precondition).
 *
 * Example:
 *   Given "content-alpha" has been distributed to at least 2 households
 */
Given(
  '{string} has been distributed to at least {int} households',
  async function (this: E2EWorld, contentId: string, minHouseholds: number) {
    const snap = await storageGet(`/api/v1/resilience/${contentId}/household`);
    const actual = (snap['stewardingCollectives'] as number) ?? 0;
    assert.ok(
      actual >= minHouseholds,
      `"${contentId}" is stewarded by ${actual} households; expected ≥${minHouseholds}.`
    );
  }
);

/**
 * Verify that at least one placement gap row exists (precondition for signals card scenario).
 *
 * Example:
 *   Given at least one placement gap exists in "/api/v1/placement-gaps"
 */
Given(
  'at least one placement gap exists in {string}',
  async function (this: E2EWorld, path: string) {
    const data = await storageGet(path);
    const items = (data['items'] as unknown[]) ?? [];
    assert.ok(items.length > 0, `No placement gaps found at ${path}. Seed the cluster first.`);
  }
);

/**
 * Verify that a content item appears in the admin content list via the doorway.
 *
 * Example:
 *   Given "content-alpha" is in the admin content list on doorway "alpha"
 */
Given(
  '{string} is in the admin content list on doorway {string}',
  async function (this: E2EWorld, contentId: string, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const { statusCode, body } = await request(`${doorway.url}/db/content/${contentId}`);
    const text = await body.text();
    assert.equal(statusCode, 200, `Content "${contentId}" not found on doorway "${doorwayId}": ${text}`);
  }
);

// ---------------------------------------------------------------------------
// When — actions
// ---------------------------------------------------------------------------

/**
 * POST a minimal content item to elohim-storage for placement testing.
 *
 * Example:
 *   When I ingest a "commons"-reach content item "content-alpha"
 */
When(
  'I ingest a {string}-reach content item {string}',
  async function (this: E2EWorld, reach: string, contentId: string) {
    const payload = {
      id: contentId,
      contentType: 'concept',
      contentFormat: 'markdown',
      reach,
      title: contentId,
      content: `# ${contentId}\n\nA test content item for resilience placement validation.`,
    };
    await storagePost('/db/content', payload);
    this.contentIds.set('lastContentId', contentId);
    this.contentIds.set('lastContentReach', reach);
  }
);

/**
 * Open the content-viewer page for a given content ID (@wip — requires Playwright).
 *
 * Example:
 *   When I open the content-viewer for "content-alpha"
 */
When(
  'I open the content-viewer for {string}',
  async function (this: E2EWorld, _contentId: string) {
    return 'pending';
  }
);

/**
 * Navigate to a given app route (@wip — requires Playwright).
 *
 * Example:
 *   When I open "/shefa/dashboard"
 */
When('I open {string}', async function (this: E2EWorld, _route: string) {
  return 'pending';
});

/**
 * Open the admin content list in the doorway app (@wip — requires Playwright).
 *
 * Example:
 *   When I open the admin content list
 */
When('I open the admin content list', async function (this: E2EWorld) {
  return 'pending';
});

// ---------------------------------------------------------------------------
// Then — assertions
// ---------------------------------------------------------------------------

/**
 * Poll a storage endpoint until a numeric field in the JSON body meets a minimum.
 * Stores the full response for follow-up Then steps.
 *
 * Example:
 *   Then within 30 seconds "/api/v1/resilience/content-alpha/household" reports "stewardingCollectives" >= 2
 */
Then(
  'within {int} seconds {string} reports {string} >= {int}',
  async function (this: E2EWorld, seconds: number, path: string, field: string, minValue: number) {
    const data = await retry(
      async () => {
        const resp = await storageGet(path);
        const actual = resp[field] as number | undefined;
        assert.ok(
          typeof actual === 'number' && actual >= minValue,
          `${field}=${actual} < ${minValue}`
        );
        return resp;
      },
      { timeoutMs: seconds * 1000, initialDelayMs: 1000, backoffFactor: 1.2, maxDelayMs: 5000 }
    );
    storeResponse(this, data);
  }
);

/**
 * Assert that a named field in the last stored response is an empty array.
 *
 * Example:
 *   And the response field "placementGaps" is empty
 */
Then(
  'the response field {string} is empty',
  async function (this: E2EWorld, fieldName: string) {
    const data = loadResponse(this);
    const value = data[fieldName];
    assert.ok(
      Array.isArray(value) && value.length === 0,
      `Expected "${fieldName}" to be empty; got: ${JSON.stringify(value)}`
    );
  }
);

/**
 * Assert that a named string field in the last stored response matches one of the given values.
 *
 * Example:
 *   And the response field "protectionStatus" is "protected" or "partial"
 */
Then(
  'the response field {string} is {string} or {string}',
  async function (this: E2EWorld, fieldName: string, valueA: string, valueB: string) {
    const data = loadResponse(this);
    const actual = data[fieldName] as string | undefined;
    assert.ok(
      actual === valueA || actual === valueB,
      `Expected "${fieldName}" to be "${valueA}" or "${valueB}"; got: "${actual}"`
    );
  }
);

/**
 * Poll a storage path until at least one item row appears.
 * Stores the response for follow-up Then steps.
 *
 * Example:
 *   Then within 30 seconds "/api/v1/placement-gaps?contentId=content-beta" returns at least one row
 */
Then(
  'within {int} seconds {string} returns at least one row',
  async function (this: E2EWorld, seconds: number, path: string) {
    const data = await retry(
      async () => {
        const resp = await storageGet(path);
        const items = (resp['items'] as unknown[]) ?? [];
        assert.ok(items.length > 0, `No rows at ${path} yet`);
        return resp;
      },
      { timeoutMs: seconds * 1000, initialDelayMs: 1000, backoffFactor: 1.2, maxDelayMs: 5000 }
    );
    storeResponse(this, data);
  }
);

/**
 * Assert that the first item row in the last stored response has a `gapKind`
 * matching one of the two given values.
 *
 * Example:
 *   And the row has "gapKind" matching "contracts-short" or "under-committed"
 */
Then(
  'the row has {string} matching {string} or {string}',
  async function (this: E2EWorld, fieldName: string, valueA: string, valueB: string) {
    const data = loadResponse(this);
    const items = (data['items'] as Array<Record<string, unknown>>) ?? [];
    assert.ok(items.length > 0, 'No rows in stored response');
    const firstRow = items[0];
    const actual = firstRow[fieldName] as string | undefined;
    assert.ok(
      actual === valueA || actual === valueB,
      `Expected first row "${fieldName}" to be "${valueA}" or "${valueB}"; got: "${actual}"`
    );
  }
);

// ---------------------------------------------------------------------------
// @wip — Browser-layer assertions (exercised during Task 19 / Plan 5)
// ---------------------------------------------------------------------------

/**
 * Assert that the resilience icon in the content-viewer has the expected CSS class.
 * @wip — requires Playwright browser session.
 */
Then(
  'the resilience icon has class {string} or {string}',
  async function (this: E2EWorld, _classA: string, _classB: string) {
    return 'pending';
  }
);

/**
 * Assert that the resilience tooltip mentions the household count.
 * @wip — requires Playwright browser session.
 */
Then('the tooltip mentions the household count', async function (this: E2EWorld) {
  return 'pending';
});

/**
 * Assert that the shefa signals card shows at least one gap.
 * @wip — requires Playwright browser session.
 */
Then('the signals card shows a non-zero gap count', async function (this: E2EWorld) {
  return 'pending';
});

/**
 * Assert that a gap signal on the signals card links to a recruitment surface.
 * @wip — requires Playwright browser session.
 */
Then(
  'clicking a gap signal scrolls to or links to a shefa recruitment surface',
  async function (this: E2EWorld) {
    return 'pending';
  }
);

/**
 * Assert that every row in the admin content list renders a resilience icon.
 * @wip — requires Playwright browser session.
 */
Then('each row renders an elohim-resilience-snapshot icon', async function (this: E2EWorld) {
  return 'pending';
});

/**
 * Assert that hovering a content row shows the household summary tooltip.
 * @wip — requires Playwright browser session.
 */
Then('hovering a row shows the household summary', async function (this: E2EWorld) {
  return 'pending';
});
