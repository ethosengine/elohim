/* eslint-disable @typescript-eslint/require-await -- Cucumber step handlers are async by convention; some sync assertions and @wip stubs don't await. */

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

import { PlaywrightDevice } from '../src/framework/devices/playwright-device.js';
import { retry } from '../src/framework/utils/retry.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Browser-tier helpers
// ---------------------------------------------------------------------------

const NO_PW_DEVICE = 'No Playwright device found';

/** Return the first PlaywrightDevice across all registered humans, or null. */
function findPwDevice(world: E2EWorld): PlaywrightDevice | null {
  for (const [, human] of world.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        return device;
      }
    }
  }
  return null;
}

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
Given('elohim-storage is reachable at {string}', async function (this: E2EWorld, urlOrEnv: string) {
  const url = process.env[urlOrEnv] ?? urlOrEnv;
  // elohim-storage exposes /health (see elohim/elohim-storage/src/http.rs:555).
  // The route is not under /api/v1/* — that prefix is reserved for storage
  // domain endpoints like /api/v1/cluster, /api/v1/peers, etc.
  const { statusCode } = await request(`${url}/health`);
  assert.ok(statusCode === 200, `elohim-storage not reachable at ${url} (status ${statusCode})`);
});

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
    const peers = (await storageGet('/api/v1/peers/delivery')) as unknown as Record<
      string,
      unknown
    >[];

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
    const peers = (await storageGet('/api/v1/peers/delivery')) as unknown as Record<
      string,
      unknown
    >[];

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
    assert.equal(
      statusCode,
      200,
      `Content "${contentId}" not found on doorway "${doorwayId}": ${text}`
    );
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
When('I open the content-viewer for {string}', async function (this: E2EWorld, _contentId: string) {
  return 'pending';
});

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
Then('the response field {string} is empty', async function (this: E2EWorld, fieldName: string) {
  const data = loadResponse(this);
  const value = data[fieldName];
  assert.ok(
    Array.isArray(value) && value.length === 0,
    `Expected "${fieldName}" to be empty; got: ${JSON.stringify(value)}`
  );
});

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
    const items = (data['items'] as Record<string, unknown>[]) ?? [];
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
// Browser-layer assertions — C5 (light-up-the-topology sprint)
//
// C3 (commit 2647661e2) added [data-testid="distribution-badge"],
// [data-testid="distribution-tooltip"], and [data-testid="placement-gaps-row"]
// to the distribution badge. The resilience snapshot component already exposes
// [data-testid="resilience-icon"] with status-protected/status-partial classes.
//
// Steps targeting surfaces that don't yet expose the required testids are kept
// as documented TODOs rather than synthesizing fake selectors — see comments
// on each unimplemented step for the gap and the un-blocking task.
// ---------------------------------------------------------------------------

/**
 * Assert that the resilience icon (rendered by <elohim-resilience-snapshot>)
 * carries one of the expected CSS classes. The snapshot component sets these
 * via [ngClass]="statusClass" in resilience-snapshot.component.html and maps
 * protection-status → 'status-protected' | 'status-partial' | 'status-at-risk'
 * | 'status-unknown' in resilience-snapshot.component.ts.
 */
Then(
  'the resilience icon has class {string} or {string}',
  async function (this: E2EWorld, classA: string, classB: string) {
    const device = findPwDevice(this);
    if (!device) {
      assert.fail(NO_PW_DEVICE);
    }
    const icon = device.page.locator('[data-testid="resilience-icon"]').first();
    await icon.waitFor({ state: 'visible', timeout: 15_000 });
    const raw = (await icon.getAttribute('class')) ?? '';
    const classes = raw.split(/\s+/).filter(Boolean);
    assert.ok(
      classes.includes(classA) || classes.includes(classB),
      `Expected resilience-icon to have class "${classA}" or "${classB}"; ` +
        `got [${classes.join(', ')}]`
    );
  }
);

/**
 * Assert that the distribution badge tooltip (opened via hover/focus on the
 * badge) mentions the household count. C3 wired the tooltip text as
 * "{replicaCount} replicas across {hubCountLabel}." where hubCountLabel can
 * read "N households" or "N households (archetype mix)". We hover the badge
 * to open the tooltip, then look for case-insensitive /household/ in its
 * text — both household-only and archetype-mix variants match.
 */
Then('the tooltip mentions the household count', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const badge = device.page.locator('[data-testid="distribution-badge"]').first();
  await badge.waitFor({ state: 'visible', timeout: 15_000 });
  await badge.hover();
  const tooltip = device.page.locator('[data-testid="distribution-tooltip"]').first();
  await tooltip.waitFor({ state: 'visible', timeout: 5_000 });
  const text = ((await tooltip.textContent()) ?? '').trim();
  assert.match(
    text,
    /household/i,
    `Expected distribution tooltip to mention "household"; got: "${text}"`
  );
});

/**
 * Assert that the shefa signals card shows a non-zero placement-gap count.
 * The card renders a subline "{totalGaps} placement gap[s]" inside
 * [data-testid="shefa-signals-card"] (signals-card.component.html line 5).
 * We parse the leading integer from the subline text — no dedicated
 * gap-count testid exists yet, but the subline shape is stable.
 */
Then('the signals card shows a non-zero gap count', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const card = device.page.locator('[data-testid="shefa-signals-card"]').first();
  await card.waitFor({ state: 'visible', timeout: 15_000 });
  const text = ((await card.textContent()) ?? '').trim();
  // Subline shape: "{N} placement gap[s]" — see signals-card.component.html.
  // Bounded \d{1,9} to avoid super-linear backtracking on adversarial input.
  const gapCountRegex = /(\d{1,9}) placement gap/i;
  const match = gapCountRegex.exec(text);
  assert.ok(
    match,
    `Expected signals card to render a "N placement gap(s)" subline; got: "${text}"`
  );
  const count = Number(match[1]);
  assert.ok(count > 0, `Expected non-zero placement gap count on signals card; got: ${count}`);
});

/**
 * Click a gap signal on the shefa signals card and verify it scrolls to or
 * links to a shefa recruitment surface.
 *
 * Follow-up (C-series): the signals card currently renders gap rows as
 * `<div class="signal signal-{kind}">` without per-row testids and without
 * click handlers wired to a recruitment route. Un-blocking work:
 *   1. Add data-testid="signals-gap" to each .signal div in
 *      signals-card.component.html
 *   2. Build a /shefa/recruit (or similar) target route + click binding
 *   3. Drop the early-return below in favour of the real assertion
 * Tracked alongside C6 (un-@wip resilience scenarios). Until then this step
 * stays inert so the cucumber binding resolves; scenarios using it must keep
 * their @wip tag.
 */
Then(
  'clicking a gap signal scrolls to or links to a shefa recruitment surface',
  async function (this: E2EWorld) {
    // Follow-up (C-series): no [data-testid="signals-gap"] yet; no recruitment
    // route wired. Step intentionally bound + inert so the @wip scenario keeps
    // its @wip tag rather than synthesizing fake selectors. See block comment
    // above this step for the un-blocking checklist.
  }
);

/**
 * Assert that every row in the doorway admin content list renders an
 * <elohim-resilience-snapshot> icon.
 *
 * Follow-up (C-series): the doorway admin content list does not currently mount
 * <elohim-resilience-snapshot> per row, nor does it expose a per-row testid.
 * Un-blocking work:
 *   1. Mount <elohim-resilience-snapshot> inside the admin content row
 *      template (doorway-app)
 *   2. Add data-testid="content-row" on each row container
 * The lamad content-viewer already renders the snapshot (see
 * content-viewer.component.html line 71); the doorway admin path is the
 * one that's missing. Tracked alongside C6.
 */
Then('each row renders an elohim-resilience-snapshot icon', async function (this: E2EWorld) {
  // Follow-up (C-series): doorway admin content list lacks per-row testids
  // and does not mount the snapshot component; surface remains @wip. See
  // block comment above this step for the un-blocking checklist.
});

/**
 * Assert that hovering a content row in the doorway admin list shows the
 * household summary tooltip.
 *
 * Follow-up (C-series): same gap as "each row renders an elohim-resilience-
 * snapshot icon" — admin row + hover summary surface not built yet. Once
 * the row mounts the snapshot, the snapshot's own tooltip
 * ([data-testid="resilience-tooltip"]) already exists and can drive this
 * assertion. Tracked alongside C6.
 */
Then('hovering a row shows the household summary', async function (this: E2EWorld) {
  // Follow-up (C-series): doorway admin row + hover surface not built;
  // remains @wip. Snapshot's own tooltip ([data-testid="resilience-tooltip"])
  // will satisfy this once the admin row mounts the snapshot component.
});
