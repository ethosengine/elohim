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
import { viewportArchetype } from '../src/framework/fixtures/viewport-archetypes.js';
import { PROTOCOL_OMNI } from '../src/framework/pages/selectors.js';
import { retry } from '../src/framework/utils/retry.js';
import { doorwayToAppUrl } from '../src/framework/utils/url.js';
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
export async function storageGet(path: string): Promise<Record<string, unknown>> {
  const { statusCode, body } = await request(`${storageUrl()}${path}`);
  const text = await body.text();
  assert.equal(statusCode, 200, `GET ${path} failed: ${statusCode} ${text}`);
  return JSON.parse(text) as Record<string, unknown>;
}

/**
 * GET /db/humans and unwrap the `{items: HumanView[], count}` envelope
 * (handle_list_humans_inner, elohim-storage/src/http.rs) into the bare array.
 * This lever was previously always 403-skipped before any real ALLOW_SEED_SHARD_MANIFEST
 * deploy, so the earlier `(await storageGet(...)) as unknown as Record<string,unknown>[]`
 * cast (treating the envelope itself as the array) never actually executed in CI —
 * surfaced as "TypeError: humans is not iterable" the first time the lever ran for real
 * (edge #1144, 2026-07-03).
 */
async function listHumans(): Promise<Record<string, unknown>[]> {
  const data = await storageGet('/db/humans');
  return (data['items'] as Record<string, unknown>[] | undefined) ?? [];
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

/**
 * Idempotent content create. Re-running a scenario must not 500 on the
 * `UNIQUE constraint failed: content.h_app_id, content.id` PK (genesis #1182
 * Cluster E): storage correctly enforces the primary key, so the TEST FIXTURE
 * must be idempotent, not the storage. GET `/db/content/{id}` first; if it
 * already exists (200) treat the create as a no-op and return the existing
 * record; otherwise POST it.
 *
 * A restricted-reach item (e.g. "intimate") the harness re-checks with no
 * auth header comes back 403 Forbidden, not 404 — `handle_db_content_by_id`
 * (elohim-storage/src/http.rs) only reaches the reach-gate FORBIDDEN branches
 * when the row WAS found (`Ok(Some(view))`); a genuinely-missing id 404s.
 * So 403 here means "exists, just not readable by us" — also a no-op, else
 * the fallthrough POST 500s on the same UNIQUE constraint (edge #1144).
 */
async function storagePostContent(
  payload: Record<string, unknown>
): Promise<Record<string, unknown>> {
  const id = payload['id'] as string;
  const { statusCode, body } = await request(`${storageUrl()}/db/content/${id}`);
  const text = await body.text(); // drain the body even on a miss (undici requirement)
  if (statusCode === 200) {
    return JSON.parse(text) as Record<string, unknown>;
  }
  if (statusCode === 403) {
    return { id, _existsButForbidden: true };
  }
  return storagePost('/db/content', payload);
}

/** PUT JSON body to elohim-storage, return {statusCode, json}. Does NOT throw on
 *  non-2xx (the seed-lever scenarios assert on the status themselves). */
async function storagePut(
  path: string,
  payload: Record<string, unknown>
): Promise<{ statusCode: number; json: Record<string, unknown> }> {
  const { statusCode, body } = await request(`${storageUrl()}${path}`, {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(payload),
  });
  const text = await body.text();
  let json: Record<string, unknown> = {};
  try {
    json = JSON.parse(text) as Record<string, unknown>;
  } catch {
    json = { _raw: text };
  }
  return { statusCode, json };
}

/** Read a possibly dotted field path from a JSON object: "feltStatus.reassurance". */
function readPath(obj: Record<string, unknown>, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, key) => {
    if (acc && typeof acc === 'object') {
      return (acc as Record<string, unknown>)[key];
    }
    return undefined;
  }, obj);
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
  // An unset env var falls through to the literal var NAME, which undici then
  // rejects as "TypeError: Invalid URL" — fail with the real reason instead.
  if (!/^https?:\/\//.test(url)) {
    assert.fail(
      `env var ${urlOrEnv} is not set (and "${url}" is not a URL) — ` +
        `set ${urlOrEnv} (CI does; for local runs point it at the storage endpoint)`
    );
  }
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
 * This is a precondition check, not a setup action — /api/v1/peers/delivery
 * reflects LIVE libp2p/iroh peer discovery (handle_delivery_peers, elohim-storage
 * src/http.rs), not a DB row a seed lever can write. Confirmed honest-`[]`-when-
 * degraded (2026-07-03 investigation): unlike the shard-manifest lever, there is
 * no admin endpoint that can fabricate discovered peers, so this precondition
 * honors real substrate state and skips (pending) rather than false-failing
 * against a mesh that hasn't (yet) discovered committed peers.
 *
 * Example:
 *   Given the cluster has peers in at least 2 distinct households each with an active "commons" provide commitment
 */
Given(
  'the cluster has peers in at least {int} distinct households each with an active {string} provide commitment',
  async function (this: E2EWorld, minCount: number, reach: string): Promise<string | undefined> {
    const peers = (await storageGet('/api/v1/peers/delivery')) as unknown as Record<
      string,
      unknown
    >[];

    const committed = peers.filter(p => {
      const commitments = (p['commitments'] as string[] | undefined) ?? [];
      return commitments.includes(reach);
    });

    const households = new Set(committed.map(p => p['householdId'] as string).filter(Boolean));

    if (households.size < minCount) {
      // No lever can fabricate live peer discovery — a precondition gap, not a
      // code failure. Skip (pending) rather than false-fail against the
      // substrate-owned condition.
      return 'pending';
    }
    return undefined;
  }
);

/**
 * Verify that the cluster has peers in exactly 2 households but only `committed`
 * of them has an active provide commitment for `reach`. Same honesty-gated
 * skip discipline as the sibling step above — /api/v1/peers/delivery cannot be
 * seeded.
 *
 * Example:
 *   Given the cluster has peers in 2 households but only 1 has an active "commons" provide commitment
 */
Given(
  'the cluster has peers in {int} households but only {int} has an active {string} provide commitment',
  async function (
    this: E2EWorld,
    _totalHouseholds: number,
    committedCount: number,
    reach: string
  ): Promise<string | undefined> {
    const peers = (await storageGet('/api/v1/peers/delivery')) as unknown as Record<
      string,
      unknown
    >[];

    const committed = peers.filter(p => {
      const commitments = (p['commitments'] as string[] | undefined) ?? [];
      return commitments.includes(reach);
    });

    const households = new Set(committed.map(p => p['householdId'] as string).filter(Boolean));

    if (households.size !== committedCount) {
      return 'pending';
    }
    return undefined;
  }
);

/**
 * Ensure a given content item has been distributed to at least N households
 * (a `Given`-precondition helper, so ensure-semantics is correct — not a bare
 * read-assert). Shared by every "stewarded by N households" flavor of Given
 * step below, so the honesty-gated seeding logic lives in exactly one place.
 *
 * Root cause (2026-07-03, blob-durability/genesis 73x fingerprint): content-alpha is
 * normally ingested inline-markdown (no blobHash) which never enters distribute_shards,
 * so `stewardingCollectives` was permanently 0 regardless of cluster health — a
 * test-authoring gap, not a substrate or commitment-activation bug. Fix mirrors the
 * already-green `grandma-photos-survive-node-loss.feature` pattern: idempotent
 * (honors real distribution first), then drives the deterministic
 * `/admin/seed/shard-manifest` lever exactly like "the operator seeds a shard
 * manifest for ... stewarded by N distinct households" below. Honesty-gated: a
 * disabled lever (403) or too few identity-healed households skips (pending)
 * rather than false-passing or false-failing.
 */
async function ensureStewardedByHouseholds(
  world: E2EWorld,
  contentId: string,
  minHouseholds: number
): Promise<string | undefined> {
  world.contentIds.set('lastContentId', contentId);

  // 1. Idempotent: honor real distribution (live mesh / prior seed) if present.
  let snap = await storageGet(`/api/v1/resilience/${contentId}/household`);
  if (((snap['stewardingCollectives'] as number) ?? 0) >= minHouseholds) {
    return undefined;
  }

  // 2. Deterministic lever, honesty-gated (ALLOW_SEED_SHARD_MANIFEST). Off → 403 →
  // clean HELD skip, not a false-fail against a mesh that was never asked to distribute.
  const probe = await storagePut('/admin/seed/shard-manifest', {});
  if (probe.statusCode === 403) {
    return 'pending';
  }

  // 3. One steward agent_pub_key per distinct household (identity-healed by
  // seed-provide-rows.ts / POST /api/v1/identity/heal).
  const humans = await listHumans();
  const byHousehold = new Map<string, string>();
  for (const h of humans) {
    const hh = h['householdId'] as string | undefined;
    const key = h['agentPubKey'] as string | undefined;
    if (hh && key && !byHousehold.has(hh)) {
      byHousehold.set(hh, key);
    }
  }
  const stewards = [...byHousehold.values()].slice(0, minHouseholds);
  if (stewards.length < minHouseholds) {
    // Not enough seeded/identity-healed households — a precondition gap, not
    // a code failure. Skip (pending), same as the shard-manifest lever step.
    return 'pending';
  }

  // 4. Seed the manifest + agent-keyed shard_locations deterministically.
  const { statusCode, json } = await storagePut('/admin/seed/shard-manifest', {
    contentId,
    reach: world.contentIds.get('lastContentReach') ?? 'commons',
    blobHash: `sha256-seed-${contentId}`,
    stewards,
    totalSizeBytes: 4096,
  });
  assert.ok(
    statusCode < 300,
    `seed shard-manifest failed for "${contentId}": ${statusCode} ${JSON.stringify(json)}`
  );

  // 5. Verify.
  snap = await storageGet(`/api/v1/resilience/${contentId}/household`);
  const actual = (snap['stewardingCollectives'] as number) ?? 0;
  assert.ok(
    actual >= minHouseholds,
    `"${contentId}" is stewarded by ${actual} households; expected ≥${minHouseholds}.`
  );
  return undefined;
}

/**
 * Example:
 *   Given "content-alpha" has been distributed to at least 2 households
 */
Given(
  '{string} has been distributed to at least {int} households',
  async function (
    this: E2EWorld,
    contentId: string,
    minHouseholds: number
  ): Promise<string | undefined> {
    return ensureStewardedByHouseholds(this, contentId, minHouseholds);
  }
);

/**
 * Short form of the ensure-step above — no per-household commitment clause.
 *
 * Example:
 *   Given a content item "summer-1974" stewarded by 3 distinct households
 */
Given(
  'a content item {string} stewarded by {int} distinct households',
  async function (
    this: E2EWorld,
    contentId: string,
    minHouseholds: number
  ): Promise<string | undefined> {
    return ensureStewardedByHouseholds(this, contentId, minHouseholds);
  }
);

/**
 * Short-protection variant — the "needs-help" felt state requires <=1 household
 * holding the content, so this targets exactly the floor of the ensure-helper.
 *
 * Example:
 *   Given a content item "wedding-album" stewarded by only 1 household
 */
Given(
  'a content item {string} stewarded by only {int} household',
  async function (
    this: E2EWorld,
    contentId: string,
    minHouseholds: number
  ): Promise<string | undefined> {
    return ensureStewardedByHouseholds(this, contentId, minHouseholds);
  }
);

/**
 * Full form: ensures distribution to N households AND that the reach is set so
 * commitment-backing (household_resilience.rs's content::reach-scoped
 * `commitmentBackedCollectives` join) is checkable. No HTTP lever fabricates
 * `rea_commitments` rows the way `/admin/seed/shard-manifest` fabricates
 * shard_locations — so this step honors real seeded commitments if present and
 * skips (pending) rather than claim commitment-backing that isn't real.
 *
 * Example:
 *   Given a content item "summer-1974" stewarded by 3 distinct households each with an active "intimate" provide commitment
 */
Given(
  'a content item {string} stewarded by {int} distinct households each with an active {string} provide commitment',
  async function (
    this: E2EWorld,
    contentId: string,
    minHouseholds: number,
    reach: string
  ): Promise<string | undefined> {
    this.contentIds.set('lastContentReach', reach);
    const distributed = await ensureStewardedByHouseholds(this, contentId, minHouseholds);
    if (distributed === 'pending') {
      return 'pending';
    }
    const snap = await storageGet(`/api/v1/resilience/${contentId}/household`);
    const backed = (snap['commitmentBackedCollectives'] as number) ?? 0;
    if (backed < 1) {
      return 'pending';
    }
    return undefined;
  }
);

/**
 * Verify every stewarding household on the last-known content item carries a
 * human-readable collective label ("names, not nines"). Reads
 * details.stewardingCollectives[].label off the live snapshot — the same
 * collectives.name join household_resilience.rs::snapshot() performs.
 *
 * Example:
 *   Given each stewarding household has a human-readable collective name
 */
Given(
  'each stewarding household has a human-readable collective name',
  async function (this: E2EWorld) {
    const contentId = this.contentIds.get('lastContentId');
    assert.ok(contentId, 'No content id on the world — run a "stewarded by" step first');
    const snap = await storageGet(`/api/v1/resilience/${contentId}/household`);
    const details = (snap['details'] as Record<string, unknown> | undefined) ?? {};
    const entries =
      (details['stewardingCollectives'] as Record<string, unknown>[] | undefined) ?? [];
    assert.ok(entries.length > 0, `No stewarding collectives found for "${contentId}"`);
    for (const entry of entries) {
      const label = entry['label'];
      assert.ok(
        typeof label === 'string' && label.trim().length > 0,
        `Expected every stewarding household to have a human-readable label; got: ${JSON.stringify(entry)}`
      );
    }
  }
);

/**
 * Narrative bridge — grandma's edge node going offline does not itself change
 * any resilience-projection row: `feltStatus` is computed from steward-
 * household rows (shard_locations) and placement gaps, not any single peer's
 * live/offline signal. The chaos proof that "K-of-N shards survive a household
 * loss" lives in the Rust integration suite
 * (elohim-storage/tests/chaos_dataplane.rs) and P-PROOFS' chaos-peer-churn
 * feature; this HTTP-level a2o suite couples that proof to the felt surface by
 * reading the SAME steward-household state the chaos proof already exercises.
 * No action needed here — the step exists so the scenario reads naturally.
 *
 * Example:
 *   When grandma's edge node goes offline
 */
When("grandma's edge node goes offline", async function (this: E2EWorld) {
  // Intentional no-op — see doc comment above.
});

/**
 * Ensure a placement-gap row exists for the content id (a household stopped
 * holding a shard). No HTTP lever synthesizes a `placement_gaps` row — unlike
 * the shard-manifest seed lever, those rows are written only by the runtime
 * P2P reconcile path (`p2p/mod.rs`) when it genuinely detects a lapsed holder.
 * Honor a real gap if one is already present; otherwise skip (pending) rather
 * than fabricate a chaos event this HTTP-level suite cannot genuinely trigger.
 *
 * Example:
 *   Given a placement-gap event has fired for "summer-1974" because one household stopped holding a shard
 */
Given(
  'a placement-gap event has fired for {string} because one household stopped holding a shard',
  async function (this: E2EWorld, contentId: string): Promise<string | undefined> {
    const data = await storageGet(
      `/api/v1/placement-gaps?contentId=${encodeURIComponent(contentId)}`
    );
    const items = (data['items'] as unknown[]) ?? [];
    if (items.length === 0) {
      return 'pending';
    }
    return undefined;
  }
);

/**
 * Ensure a content item exists but has never entered the distribution plane —
 * the inverse precondition of the "stewarded by" steps. Plain inline-markdown
 * ingest (no blobHash) never enters distribute_shards, so distributionState
 * honestly reads "unmeasured" regardless of reach.
 *
 * Example:
 *   Given a content item "private-letters" that has never entered the distribution plane
 */
Given(
  'a content item {string} that has never entered the distribution plane',
  async function (this: E2EWorld, contentId: string) {
    await ingestInlineContent(this, 'intimate', contentId);
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
 * POST a minimal inline-markdown content item to elohim-storage. Shared by "I
 * ingest a {string}-reach content item {string}" and any Given step that needs
 * plain (never-blob-backed) content as a precondition — inline-body content
 * carries no blobHash, so it never enters distribute_shards.
 */
async function ingestInlineContent(
  world: E2EWorld,
  reach: string,
  contentId: string
): Promise<void> {
  const payload = {
    id: contentId,
    contentType: 'concept',
    contentFormat: 'markdown',
    reach,
    title: contentId,
    content: `# ${contentId}\n\nA test content item for resilience placement validation.`,
  };
  await storagePostContent(payload);
  world.contentIds.set('lastContentId', contentId);
  world.contentIds.set('lastContentReach', reach);
}

/**
 * POST a minimal content item to elohim-storage for placement testing.
 *
 * Example:
 *   When I ingest a "commons"-reach content item "content-alpha"
 */
When(
  'I ingest a {string}-reach content item {string}',
  async function (this: E2EWorld, reach: string, contentId: string) {
    await ingestInlineContent(this, reach, contentId);
  }
);

/**
 * Shared @wip stub behind both content-viewer step spellings (see below) — one
 * behavior, two cucumber expressions.
 */
async function openContentViewerStub(_contentId: string): Promise<string> {
  return 'pending';
}

/**
 * Open the content-viewer page for a given content ID (@wip — requires Playwright).
 *
 * Example:
 *   When I open the content-viewer for "content-alpha"
 */
When('I open the content-viewer for {string}', async function (this: E2EWorld, contentId: string) {
  return openContentViewerStub(contentId);
});

/**
 * Space-spelling alias of "I open the content-viewer for {string}" — 3
 * scenarios (observable-distribution.feature, resilience-dimensions.feature)
 * write "content viewer" (no hyphen). Delegates to the same stub so there is
 * exactly one behavior behind both cucumber expressions.
 *
 * Example:
 *   When I open the content viewer for "dim-pair"
 */
When('I open the content viewer for {string}', async function (this: E2EWorld, contentId: string) {
  return openContentViewerStub(contentId);
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

/**
 * Open the EPR resource page (/resource/{id}) in elohim-app — one of the two
 * protocolContent routes where app.component mounts <app-protocol-omni>
 * (the trust surface). Waits for the omni chip so follow-up steps can expand it.
 *
 * Example:
 *   When I open the EPR resource page for "content-alpha"
 */
When(
  'I open the EPR resource page for {string}',
  async function (this: E2EWorld, contentId: string) {
    const device = findPwDevice(this);
    if (!device) return 'pending';
    const appUrl = doorwayToAppUrl(device.client.url);
    await device.page.goto(`${appUrl}/resource/${encodeURIComponent(contentId)}`, {
      waitUntil: 'networkidle',
    });
    await device.page
      .locator(`[data-testid="${PROTOCOL_OMNI.CHIP}"]`)
      .waitFor({ state: 'visible', timeout: 15_000 });
    return undefined;
  }
);

/**
 * Expand the protocol omni toolbar by clicking the collapsed chip. The
 * expansion is also what triggers the lazy resilience-snapshot fetch —
 * the collapsed chip never spends a round-trip.
 *
 * Example:
 *   And I expand the protocol omni toolbar
 */
When('I expand the protocol omni toolbar', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) return 'pending';
  await device.page.locator(`[data-testid="${PROTOCOL_OMNI.CHIP}"]`).click();
  await device.page
    .locator(`[data-testid="${PROTOCOL_OMNI.TOOLBAR}"]`)
    .waitFor({ state: 'visible', timeout: 5_000 });
  return undefined;
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
 * Assert that the protocol-omni resilience segment renders the LIVE snapshot
 * icon — i.e. the <elohim-resilience-snapshot> [data-testid="resilience-icon"]
 * nested inside [data-testid="protocol-omni-resilience"], not the neutral
 * no-data glyph. The snapshot fetch is lazy (fires on toolbar expansion), so
 * this waits for the icon to appear rather than asserting instantaneously.
 */
Then('the omni resilience segment renders a live resilience icon', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const icon = device.page
    .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE}"] [data-testid="resilience-icon"]`)
    .first();
  await icon.waitFor({ state: 'visible', timeout: 15_000 });
});

/**
 * Omni-scoped variant of the class assertion: on /resource/{id} TWO
 * resilience-icon instances can coexist (protocol-omni topbar + the
 * content-viewer header). The unscoped step's `.first()` could match the
 * wrong one, so omni scenarios assert through the PROTOCOL_OMNI.RESILIENCE
 * wrapper explicitly.
 */
Then(
  'the omni resilience icon has class {string} or {string}',
  async function (this: E2EWorld, classA: string, classB: string) {
    const device = findPwDevice(this);
    if (!device) {
      assert.fail(NO_PW_DEVICE);
    }
    const icon = device.page
      .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE}"] [data-testid="resilience-icon"]`)
      .first();
    await icon.waitFor({ state: 'visible', timeout: 15_000 });
    const raw = (await icon.getAttribute('class')) ?? '';
    const classes = raw.split(/\s+/).filter(Boolean);
    assert.ok(
      classes.includes(classA) || classes.includes(classB),
      `Expected omni resilience-icon to have class "${classA}" or "${classB}"; ` +
        `got [${classes.join(', ')}]`
    );
  }
);

/**
 * Assert that the omni resilience tooltip names the stewarding-collective
 * count. The icon-density tooltip renders "{N} collectives · …" (see
 * resilience-snapshot.component.html) — the headline is collectives, not
 * peers, because the household/collective is the resilience unit. The
 * tooltip is CSS-hidden until hover, so this hovers the icon first and
 * waits for visibility — matching what a human actually does.
 */
Then(
  'the omni resilience tooltip mentions stewarding collectives',
  async function (this: E2EWorld) {
    const device = findPwDevice(this);
    if (!device) {
      assert.fail(NO_PW_DEVICE);
    }
    const icon = device.page
      .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE}"] [data-testid="resilience-icon"]`)
      .first();
    await icon.waitFor({ state: 'visible', timeout: 15_000 });
    await icon.hover();
    const tooltip = device.page
      .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE}"] [data-testid="resilience-tooltip"]`)
      .first();
    await tooltip.waitFor({ state: 'visible', timeout: 5_000 });
    const text = ((await tooltip.textContent()) ?? '').trim();
    // Bounded \d{1,9} to avoid super-linear backtracking on adversarial input.
    assert.match(
      text,
      /\d{1,9} collectives/i,
      `Expected omni resilience tooltip to name the collective count; got: "${text}"`
    );
  }
);

// ---------------------------------------------------------------------------
// Omni resilience tooltip direction + hypercard progressive disclosure
// (omnibar-consolidation spec §11). Two defects healed here: (1) the icon
// tooltip flipped UP (bottom:125%) and clipped above y=0 because protocol-omni
// is pinned to the top viewport edge — it now folds DOWN, inline-start aligned
// (the @regression anchor below); (2) the icon gains a click affordance that
// folds down <elohim-hypercard-panel> (testid PROTOCOL_OMNI.RESILIENCE_HYPERCARD)
// carrying the context-density body + an action row. The panel's action buttons
// live in its shadow root (data-action-id="<id>", part="action"); slotted
// full-density content (.resilience-full-card) lives in the default slot (light
// DOM). Playwright's CSS engine pierces shadow DOM, so a descendant query off
// the testid reaches both.
// ---------------------------------------------------------------------------

/**
 * Resolve the omni resilience icon locator — the interactive button rendered by
 * <elohim-resilience-snapshot> inside the PROTOCOL_OMNI.RESILIENCE wrapper.
 * Two resilience-icon instances can coexist on /resource/{id} (omni topbar +
 * content-viewer header), so omni steps always scope through the wrapper.
 */
function omniResilienceIcon(device: PlaywrightDevice) {
  return device.page
    .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE}"] [data-testid="resilience-icon"]`)
    .first();
}

/**
 * Map a human-facing hypercard action label to its stable action id. The
 * built-in "View full resilience" action is { id: 'view-full' } (spec §11.3);
 * the table grows as Epic E actions get wired (§11.6). Centralised so the
 * "offers a {string} action" and "choose the {string} action" steps agree.
 */
const HYPERCARD_ACTION_IDS: Record<string, string> = {
  'View full resilience': 'view-full',
};

function hypercardActionId(label: string): string {
  const id = HYPERCARD_ACTION_IDS[label];
  assert.ok(
    id,
    `Unknown hypercard action label "${label}" — add it to HYPERCARD_ACTION_IDS ` +
      `(resilience.steps.ts) mapping label → data-action-id.`
  );
  return id;
}

// Captured by "I click the omni resilience icon" so the "browser URL is
// unchanged" assertion can prove the in-place card flip never navigated.
// Lives on the world (per-scenario) rather than a module-level let so parallel
// scenarios don't clobber one another.
const resilienceUrlKey = Symbol('resilience:urlBefore');

/**
 * Hover the omni resilience icon — opens the icon-density tooltip (the L1
 * zero-click glance). Mirrors the hover-then-waitFor idiom the
 * "tooltip mentions stewarding collectives" step uses.
 */
When('I hover the omni resilience icon', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const icon = omniResilienceIcon(device);
  await icon.waitFor({ state: 'visible', timeout: 15_000 });
  await icon.hover();
  // Wait for the tooltip to render so the direction/viewport assertions that
  // follow measure a laid-out box, not display:none geometry.
  await device.page
    .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE}"] [data-testid="resilience-tooltip"]`)
    .first()
    .waitFor({ state: 'visible', timeout: 5_000 });
});

/**
 * REGRESSION ANCHOR (omnibar spec §11.3): the tooltip was hard-coded
 * `bottom: 125%` (always flips up); protocol-omni is fixed to the top viewport
 * edge, so on desktop the tooltip rendered above y=0 — invisible. Assert the
 * tooltip's bounding box is FULLY inside the viewport: x>=0, y>=0, and the
 * right/bottom edges within page.viewportSize(). This fails if the tooltip
 * flips up off-screen again.
 */
Then('the omni resilience tooltip is fully inside the viewport', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const tooltip = device.page
    .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE}"] [data-testid="resilience-tooltip"]`)
    .first();
  await tooltip.waitFor({ state: 'visible', timeout: 5_000 });
  const box = await tooltip.boundingBox();
  assert.ok(box, 'Expected the resilience tooltip to have a bounding box (is it visible?)');
  const viewport = device.page.viewportSize();
  assert.ok(viewport, 'Expected a viewport size (browser device should set one)');
  assert.ok(
    box.x >= 0 && box.y >= 0,
    `Tooltip clips off the top/left edge: box=(${box.x}, ${box.y}). ` +
      `Regression: it must fold DOWN into the viewport, never UP out of it.`
  );
  assert.ok(
    box.x + box.width <= viewport.width && box.y + box.height <= viewport.height,
    `Tooltip clips off the right/bottom edge: box right=${box.x + box.width}, ` +
      `bottom=${box.y + box.height}; viewport=${viewport.width}x${viewport.height}.`
  );
});

/**
 * Assert the tooltip renders BELOW the icon (folds down, not up). Tooltip box
 * top must be at or below the icon box bottom (1px slack for sub-pixel layout).
 * Paired with the viewport-containment anchor above — together they pin the
 * down-fold convention (matching distribution-badge's top:100%/left:0).
 */
Then(
  'the omni resilience tooltip renders below the resilience icon',
  async function (this: E2EWorld) {
    const device = findPwDevice(this);
    if (!device) {
      assert.fail(NO_PW_DEVICE);
    }
    const iconBox = await omniResilienceIcon(device).boundingBox();
    const tooltip = device.page
      .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE}"] [data-testid="resilience-tooltip"]`)
      .first();
    const tooltipBox = await tooltip.boundingBox();
    assert.ok(iconBox, 'Expected the resilience icon to have a bounding box');
    assert.ok(tooltipBox, 'Expected the resilience tooltip to have a bounding box');
    assert.ok(
      tooltipBox.y >= iconBox.y + iconBox.height - 1,
      `Tooltip must render below the icon: tooltip top=${tooltipBox.y}, ` +
        `icon bottom=${iconBox.y + iconBox.height}. It should fold DOWN, not UP.`
    );
  }
);

/**
 * Resize the browser to a named viewport archetype (phone-small | phone |
 * tablet | desktop) — the viewport dimension of the component matrix, sibling
 * of the light/dark theme cells. Must run BEFORE navigation so media queries
 * and vw-relative clamps resolve against the archetype from first paint.
 */
Given(
  'the browser viewport is the {string} archetype',
  async function (this: E2EWorld, name: string) {
    const device = findPwDevice(this);
    if (!device) {
      assert.fail(NO_PW_DEVICE);
    }
    const { width, height } = viewportArchetype(name);
    await device.page.setViewportSize({ width, height });
  }
);

/**
 * REGRESSION ANCHOR (omnibar spec §11.2 end-alignment amendment): the
 * hypercard panel pinned inset-inline-start:0 to the icon wrap with a 240px
 * min width; anchored in end-side omni chrome on a phone archetype it
 * projected ~173px off the right screen edge. Assert the open panel's box is
 * FULLY inside the viewport — the inline-axis twin of the tooltip's
 * fold-down containment anchor above.
 */
Then(
  'the resilience hypercard panel is fully inside the viewport',
  async function (this: E2EWorld) {
    const device = findPwDevice(this);
    if (!device) {
      assert.fail(NO_PW_DEVICE);
    }
    const panel = device.page
      .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE_HYPERCARD}"]`)
      .first();
    await panel.waitFor({ state: 'visible', timeout: 5_000 });
    const box = await panel.boundingBox();
    assert.ok(box, 'Expected the resilience hypercard to have a bounding box (is it open?)');
    const viewport = device.page.viewportSize();
    assert.ok(viewport, 'Expected a viewport size (browser device should set one)');
    assert.ok(
      box.x >= 0 && box.y >= 0,
      `Hypercard clips off the top/left edge: box=(${box.x}, ${box.y}).`
    );
    assert.ok(
      box.x + box.width <= viewport.width && box.y + box.height <= viewport.height,
      `Hypercard clips off the right/bottom edge: box right=${box.x + box.width}, ` +
        `bottom=${box.y + box.height}; viewport=${viewport.width}x${viewport.height}. ` +
        `Regression: end-side chrome must end-align the panel (align="end") and ` +
        `the panel must clamp its inline size to the viewport.`
    );
  }
);

/**
 * WCAG 2.5.8 minimum target size: the resilience icon trigger's hit area must
 * be at least 24x24 CSS px (it was a 7x14px glyph with zero padding). The
 * bounding box includes padding, so the padded button is what's measured.
 */
Then('the omni resilience icon meets the minimum tap target size', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const box = await omniResilienceIcon(device).boundingBox();
  assert.ok(box, 'Expected the resilience icon to have a bounding box');
  assert.ok(
    box.width >= 24 && box.height >= 24,
    `Resilience icon tap target is ${box.width}x${box.height}px — below the ` +
      `WCAG 2.5.8 minimum of 24x24. Pad the trigger button's hit area.`
  );
});

/**
 * Click the omni resilience icon — folds down the hypercard panel (the L2
 * context-density disclosure). Also captures the current URL onto the world so
 * the later "browser URL is unchanged" assertion can prove the in-place card
 * flip never navigated.
 */
When('I click the omni resilience icon', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const icon = omniResilienceIcon(device);
  await icon.waitFor({ state: 'visible', timeout: 15_000 });
  // Capture before the click so the URL-unchanged check has a baseline even if
  // the panel were (wrongly) to navigate.
  (this as unknown as Record<symbol, unknown>)[resilienceUrlKey] = device.page.url();
  await icon.click();
});

/**
 * Assert the hypercard panel is visible AND folds DOWN (its box top is at or
 * below the icon box bottom, 1px slack). Same down-fold convention as the
 * tooltip — the click affordance must not flip up out of the top-pinned omni.
 */
Then('the resilience hypercard panel is visible below the icon', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const panel = device.page
    .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE_HYPERCARD}"]`)
    .first();
  await panel.waitFor({ state: 'visible', timeout: 5_000 });
  const panelBox = await panel.boundingBox();
  const iconBox = await omniResilienceIcon(device).boundingBox();
  assert.ok(panelBox, 'Expected the resilience hypercard to have a bounding box');
  assert.ok(iconBox, 'Expected the resilience icon to have a bounding box');
  assert.ok(
    panelBox.y >= iconBox.y + iconBox.height - 1,
    `Hypercard must fold down below the icon: panel top=${panelBox.y}, ` +
      `icon bottom=${iconBox.y + iconBox.height}.`
  );
});

/**
 * Assert the panel's context body names the stewarding-collective count — a
 * number followed by "collectives" (the household/collective is the resilience
 * unit). Bounded \d{1,9} per the file's backtracking discipline; the count and
 * label may be separated by markup, so they're matched independently.
 */
Then(
  'the resilience hypercard names the stewarding collective count',
  async function (this: E2EWorld) {
    const device = findPwDevice(this);
    if (!device) {
      assert.fail(NO_PW_DEVICE);
    }
    const panel = device.page
      .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE_HYPERCARD}"]`)
      .first();
    await panel.waitFor({ state: 'visible', timeout: 5_000 });
    const text = ((await panel.textContent()) ?? '').trim();
    // Bounded \d{1,9} avoids super-linear backtracking on adversarial input.
    assert.match(text, /\d{1,9}/, `Expected the hypercard to render a count; got: "${text}"`);
    assert.match(
      text,
      /collectives/i,
      `Expected the hypercard to name "collectives"; got: "${text}"`
    );
  }
);

/**
 * Assert the panel offers a named action — the action button is rendered inside
 * the panel's shadow root carrying data-action-id="<id>". Map the human label
 * to its id ("View full resilience" → "view-full"); Playwright's CSS engine
 * pierces shadow DOM, so the descendant query off the testid reaches it.
 */
Then(
  'the resilience hypercard offers a {string} action',
  async function (this: E2EWorld, label: string) {
    const device = findPwDevice(this);
    if (!device) {
      assert.fail(NO_PW_DEVICE);
    }
    const id = hypercardActionId(label);
    const action = device.page
      .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE_HYPERCARD}"] [data-action-id="${id}"]`)
      .first();
    await action.waitFor({ state: 'visible', timeout: 5_000 });
  }
);

/**
 * Choose (click) a named hypercard action — drives the in-place card flip
 * (view-full) or, once wired, an Epic E destination (§11.6).
 */
When('I choose the {string} hypercard action', async function (this: E2EWorld, label: string) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const id = hypercardActionId(label);
  const action = device.page
    .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE_HYPERCARD}"] [data-action-id="${id}"]`)
    .first();
  await action.waitFor({ state: 'visible', timeout: 5_000 });
  await action.click();
});

/**
 * Assert the in-place card flip happened — the panel now renders the full-
 * density resilience card. The slotted full card lives in the default slot
 * (light DOM): assert either the .resilience-full-card container or its
 * "Resilience snapshot" heading is present under the panel testid.
 */
Then('the resilience hypercard shows the full resilience card', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const fullCard = device.page
    .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE_HYPERCARD}"] .resilience-full-card`)
    .first();
  const fullCardCount = await fullCard.count();
  if (fullCardCount > 0) {
    await fullCard.waitFor({ state: 'visible', timeout: 5_000 });
    return;
  }
  // Fallback: the full card surfaces its "Resilience snapshot" heading even if
  // the .resilience-full-card class name shifts.
  const panel = device.page
    .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE_HYPERCARD}"]`)
    .first();
  const text = ((await panel.textContent()) ?? '').trim();
  assert.match(
    text,
    /Resilience snapshot/i,
    `Expected the full resilience card (.resilience-full-card or "Resilience snapshot" ` +
      `heading) after the in-place flip; got: "${text}"`
  );
});

/**
 * Assert the browser URL is unchanged since "I click the omni resilience icon"
 * captured it — proves HyperCard semantics (the card flips in place; no
 * navigation). Fails with a clear message if the capture step never ran.
 */
Then('the browser URL is unchanged', function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const before = (this as unknown as Record<symbol, unknown>)[resilienceUrlKey] as
    | string
    | undefined;
  assert.ok(
    before !== undefined,
    'No baseline URL captured — the "I click the omni resilience icon" step must run ' +
      'before "the browser URL is unchanged".'
  );
  assert.equal(
    device.page.url(),
    before,
    'Expected the URL to be unchanged (HyperCard in-place flip, no navigation).'
  );
});

/**
 * Press Escape from inside the resilience hypercard — closes the non-modal
 * dialog (the element listens for Escape from anywhere inside, per spec §11.2).
 */
When('I press Escape in the resilience hypercard', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  await device.page.keyboard.press('Escape');
});

/**
 * Assert the hypercard panel is no longer visible (closed).
 */
Then('the resilience hypercard panel is not visible', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  await device.page
    .locator(`[data-testid="${PROTOCOL_OMNI.RESILIENCE_HYPERCARD}"]`)
    .first()
    .waitFor({ state: 'hidden', timeout: 5_000 });
});

/**
 * Assert focus returned to the omni resilience icon — the panel restores focus
 * to the previously focused element on close (spec §11.2). Compare the icon's
 * element identity against document.activeElement, evaluated in-page (mirrors
 * the feedback-gate focus assertion). The icon lives in the snapshot's shadow
 * root, so resolve it via the same selector chain the locator uses.
 */
Then('the omni resilience icon has focus', async function (this: E2EWorld) {
  const device = findPwDevice(this);
  if (!device) {
    assert.fail(NO_PW_DEVICE);
  }
  const icon = omniResilienceIcon(device);
  await icon.waitFor({ state: 'visible', timeout: 5_000 });
  const focused = await icon.evaluate(el => el === document.activeElement);
  assert.ok(focused, 'Expected focus to return to the omni resilience icon after Escape.');
});

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

// ---------------------------------------------------------------------------
// Household reciprocity — the M1 named-pair flag (terrance-drift RCA 2026-06-04)
// ---------------------------------------------------------------------------

/**
 * Commitment rows store provider/receiver as deterministic PEER ids; the
 * humanIds ride in metadata (seed-commitments.ts buildCustodyCommitmentBody).
 * The named-pair assertions match on metadata so the flag fires on persona
 * drift regardless of archetype/peer-id choices.
 */
interface CommitmentRow {
  action?: string;
  state?: string;
  metadata?: { providerHumanId?: string; receiverHumanId?: string } | null;
}

export const commitmentListKey = Symbol('resilience:commitmentList');

When('I list active {string} commitments', async function (this: E2EWorld, action: string) {
  // Query through the DOORWAY (manifest-routed /api/v1/commitments), not
  // E2E_STORAGE_URL — the genesis API stage doesn't set the storage env var
  // (browser stage only), and the doorway path is what users actually hit.
  let doorwayUrl = '';
  for (const [, d] of this.doorways) {
    doorwayUrl = d.url;
    break;
  }
  assert.ok(doorwayUrl, 'No doorway registered — did the Background step run?');
  const { statusCode, body } = await request(
    `${doorwayUrl}/api/v1/commitments?action=${encodeURIComponent(action)}&state=active`
  );
  const text = await body.text();
  assert.equal(statusCode, 200, `GET /api/v1/commitments failed: ${statusCode} ${text}`);
  const data = JSON.parse(text) as Record<string, unknown>;
  // Service may return a bare array or an {items: []} envelope — accept both.
  const rows = Array.isArray(data)
    ? data
    : ((data['items'] ?? data['commitments'] ?? []) as unknown[]);
  (this as unknown as Record<symbol, unknown>)[commitmentListKey] = rows;
});

Then(
  'an active {string} commitment exists from {string} to {string}',
  function (this: E2EWorld, action: string, providerHumanId: string, receiverHumanId: string) {
    const rows = ((this as unknown as Record<symbol, unknown>)[commitmentListKey] ??
      []) as CommitmentRow[];
    assert.ok(
      rows.length > 0,
      'No commitments listed — did the listing step run (and is anything seeded)?'
    );
    const match = rows.find(
      r =>
        (r.action ?? action) === action &&
        r.metadata?.providerHumanId === providerHumanId &&
        r.metadata?.receiverHumanId === receiverHumanId
    );
    assert.ok(
      match,
      `No active ${action} commitment from ${providerHumanId} to ${receiverHumanId}. ` +
        `This is the M1 named-pair flag: if a persona rename moved this pair, update the ` +
        `seeder + this scenario TOGETHER (see household-reciprocity.feature header). ` +
        `Pairs present: ${
          rows
            .map(r => `${r.metadata?.providerHumanId ?? '?'}→${r.metadata?.receiverHumanId ?? '?'}`)
            .join(', ') || '(none)'
        }`
    );
  }
);

// ---------------------------------------------------------------------------
// Epic B — blob-backed content lights stewarding (the resilience card's
// distributionState + stewardingCollectives columns)
//
// Two complementary paths prove the card LIGHTS for genuinely shard-distributed
// content:
//   1. The operator seed lever (PUT /admin/seed/shard-manifest) — deterministic,
//      off by default (ALLOW_SEED_SHARD_MANIFEST=1), for demo/test/thin-mesh.
//   2. The real P2P path (POST /db/content with a blob → distribute_shards) on
//      the live 2-node household mesh.
// ---------------------------------------------------------------------------

/** Generate a small deterministic blob payload for a content id. */
function blobPayloadFor(contentId: string): string {
  return `BLOB:${contentId}:` + 'x'.repeat(64);
}

/**
 * Create a blob-backed content item. A photo album IS the blob case — it carries
 * a blob_hash, so it enters the distribution plane (unlike inline-body content,
 * which is DHT-replicated and honestly reads distributionState: unmeasured).
 *
 * Example:
 *   Given a blob-backed content item "grandma-album-1974" exists with reach "intimate"
 */
Given(
  'a blob-backed content item {string} exists with reach {string}',
  async function (this: E2EWorld, contentId: string, reach: string) {
    const payload = {
      id: contentId,
      contentType: 'album',
      contentFormat: 'external',
      reach,
      title: contentId,
      // A blob-backed item carries blobHash; the body is the blob the substrate
      // would shard. We send both so the create path records a manifest.
      blobHash: `sha256-seed-${contentId}`,
      content: blobPayloadFor(contentId),
    };
    await storagePostContent(payload);
    this.contentIds.set('lastContentId', contentId);
    this.contentIds.set('lastContentReach', reach);
  }
);

/**
 * Precondition gate for the operator seed lever. The lever is OFF by default
 * (the honesty boundary); the storage process must be started with
 * ALLOW_SEED_SHARD_MANIFEST=1. When the env says it's disabled here, the
 * scenario skips (pending) rather than false-failing — the lever's whole point
 * is that it cannot be on by accident.
 *
 * Example:
 *   And the operator seed-shard-manifest lever is enabled
 */
Given('the operator seed-shard-manifest lever is enabled', async function (this: E2EWorld): Promise<
  string | undefined
> {
  // Probe the endpoint with an intentionally-empty body: a 400 means the lever
  // is ON (it got past the gate to validation); a 403 means it's OFF.
  const { statusCode } = await storagePut('/admin/seed/shard-manifest', {});
  if (statusCode === 403) {
    // Lever disabled — skip rather than fail. Set ALLOW_SEED_SHARD_MANIFEST=1
    // on the storage process to exercise this scenario.
    return 'pending';
  }
  return undefined;
});

/**
 * Drive the deterministic lever: discover N real households (from /db/humans),
 * then PUT a shard manifest + agent-keyed shard_locations placing the album's
 * single shard on one steward per household. The endpoint is agent-keyed
 * (peerId = agent_pub_key), so the rows join humans → household_id exactly like
 * the runtime distribute_shards path.
 *
 * Example:
 *   When the operator seeds a shard manifest for "grandma-album-1974" stewarded by 3 distinct households
 */
When(
  'the operator seeds a shard manifest for {string} stewarded by {int} distinct households',
  async function (
    this: E2EWorld,
    contentId: string,
    households: number
  ): Promise<string | undefined> {
    const humans = await listHumans();
    // One steward agent key per distinct household.
    const byHousehold = new Map<string, string>();
    for (const h of humans) {
      const hh = h['householdId'] as string | undefined;
      const key = h['agentPubKey'] as string | undefined;
      if (hh && key && !byHousehold.has(hh)) {
        byHousehold.set(hh, key);
      }
    }
    const stewards = [...byHousehold.values()].slice(0, households);
    if (stewards.length < households) {
      // Not enough seeded households with agent keys — skip (pending). This is a
      // @requires:seeded-content precondition, not a code failure.
      return 'pending';
    }

    const { statusCode, json } = await storagePut('/admin/seed/shard-manifest', {
      contentId,
      reach: this.contentIds.get('lastContentReach') ?? 'commons',
      blobHash: `sha256-seed-${contentId}`,
      stewards,
      totalSizeBytes: 4096,
    });
    assert.ok(
      statusCode < 300,
      `seed shard-manifest failed: ${statusCode} ${JSON.stringify(json)}`
    );
    assert.equal(json['seeded'], true, `expected a seeded report; got ${JSON.stringify(json)}`);
    return undefined;
  }
);

/**
 * POST a blob-backed content item to elohim-storage and trigger the real
 * distribute_shards path. Distinct from "I ingest a {string}-reach content item"
 * (inline body) — this carries a blobHash so the substrate shards + distributes.
 *
 * Example:
 *   When I ingest a blob-backed "commons"-reach content item "mesh-album-1974"
 */
When(
  'I ingest a blob-backed {string}-reach content item {string}',
  async function (this: E2EWorld, reach: string, contentId: string) {
    const payload = {
      id: contentId,
      contentType: 'album',
      contentFormat: 'external',
      reach,
      title: contentId,
      blobHash: `sha256-seed-${contentId}`,
      content: blobPayloadFor(contentId),
    };
    await storagePostContent(payload);
    this.contentIds.set('lastContentId', contentId);
    this.contentIds.set('lastContentReach', reach);
  }
);

/**
 * GET a storage path and store the response for follow-up Then steps.
 *
 * Example:
 *   And I read "/api/v1/resilience/grandma-album-1974/household"
 */
When('I read {string}', async function (this: E2EWorld, path: string) {
  const data = await storageGet(path);
  storeResponse(this, data);
});

/**
 * Assert a (possibly dotted) field in the last stored response equals a value.
 *
 * Example:
 *   Then the response field "distributionState" is "measured"
 *   And the response field "feltStatus.reassurance" is "protected"
 */
Then(
  'the response field {string} is {string}',
  function (this: E2EWorld, fieldName: string, expected: string) {
    const data = loadResponse(this);
    const actual = readPath(data, fieldName);
    assert.equal(
      String(actual),
      expected,
      `Expected "${fieldName}" to be "${expected}"; got: "${String(actual)}"`
    );
  }
);

/**
 * Assert a numeric (possibly dotted) field in the last stored response is at
 * least a minimum.
 *
 * Example:
 *   And the response field "stewardingCollectives" is at least 3
 */
Then(
  'the response field {string} is at least {int}',
  function (this: E2EWorld, fieldName: string, min: number) {
    const data = loadResponse(this);
    const actual = readPath(data, fieldName);
    assert.ok(
      typeof actual === 'number' && actual >= min,
      `Expected "${fieldName}" >= ${min}; got: ${String(actual)}`
    );
  }
);

/**
 * Numeric-literal sibling of "the response field {string} is {string}" — same
 * readPath + equality pattern, comparing against a bare `{int}` rather than a
 * quoted string. Guards with `typeof actual === 'number'` for consistency with
 * "...is at least {int}" above.
 *
 * Example:
 *   And the response field "stewardingCollectives" is 0
 */
Then(
  'the response field {string} is {int}',
  function (this: E2EWorld, fieldName: string, expected: number) {
    const data = loadResponse(this);
    const actual = readPath(data, fieldName);
    assert.ok(
      typeof actual === 'number' && actual === expected,
      `Expected "${fieldName}" to be ${expected}; got: ${String(actual)}`
    );
  }
);

/**
 * Assert a (possibly dotted) string field in the last stored response is NOT a
 * given value — the negated sibling of "the response field {string} is {string}".
 *
 * Example:
 *   And the response field "feltStatus.reassurance" is not "at-risk"
 */
Then(
  'the response field {string} is not {string}',
  function (this: E2EWorld, fieldName: string, unexpected: string) {
    const data = loadResponse(this);
    const actual = readPath(data, fieldName);
    assert.notEqual(
      String(actual),
      unexpected,
      `Expected "${fieldName}" to not be "${unexpected}"; got: "${String(actual)}"`
    );
  }
);

/**
 * Assert a (possibly dotted) string field in the last stored response contains
 * a substring.
 *
 * Example:
 *   And the response field "feltStatus.headline" contains "still"
 */
Then(
  'the response field {string} contains {string}',
  function (this: E2EWorld, fieldName: string, expected: string) {
    const data = loadResponse(this);
    const actual = readPath(data, fieldName);
    assert.ok(
      typeof actual === 'string' && actual.includes(expected),
      `Expected "${fieldName}" to contain "${expected}"; got: ${JSON.stringify(actual)}`
    );
  }
);

/**
 * Assert a (possibly dotted) string field in the last stored response matches
 * a regular expression, given as a string pattern.
 *
 * Example:
 *   And the response field "feltStatus.headline" matches "^Held by 3 households: "
 */
Then(
  'the response field {string} matches {string}',
  function (this: E2EWorld, fieldName: string, pattern: string) {
    const data = loadResponse(this);
    const actual = readPath(data, fieldName);
    const re = new RegExp(pattern);
    assert.ok(
      typeof actual === 'string' && re.test(actual),
      `Expected "${fieldName}" to match /${pattern}/; got: ${JSON.stringify(actual)}`
    );
  }
);

/**
 * Assert every entry in an array field (possibly dotted path) has a non-empty
 * string subfield — e.g. every held-by collective carries a display label.
 *
 * Example:
 *   And every "feltStatus.heldBy" entry has a non-empty "label"
 */
Then(
  'every {string} entry has a non-empty {string}',
  function (this: E2EWorld, arrayPath: string, subField: string) {
    const data = loadResponse(this);
    const items = readPath(data, arrayPath);
    assert.ok(
      Array.isArray(items),
      `Expected "${arrayPath}" to be an array; got: ${JSON.stringify(items)}`
    );
    assert.ok((items as unknown[]).length > 0, `Expected "${arrayPath}" to be non-empty`);
    for (const item of items as Record<string, unknown>[]) {
      const value = item[subField];
      assert.ok(
        typeof value === 'string' && value.trim().length > 0,
        `Expected every "${arrayPath}" entry to have a non-empty "${subField}"; got: ${JSON.stringify(item)}`
      );
    }
  }
);

/**
 * Assert a (possibly dotted) string field does not read a faceless SLA
 * percentage ("99.9%" style) — the felt surface names households, not nines.
 * Bounded \d{1,3} to avoid super-linear backtracking on adversarial input.
 *
 * Example:
 *   And the response does not contain a faceless SLA percentage in "feltStatus.headline"
 */
Then(
  'the response does not contain a faceless SLA percentage in {string}',
  function (this: E2EWorld, fieldName: string) {
    const data = loadResponse(this);
    const actual = readPath(data, fieldName);
    assert.ok(
      typeof actual === 'string',
      `Expected "${fieldName}" to be a string; got: ${JSON.stringify(actual)}`
    );
    assert.ok(
      !/\d{1,3}(\.\d{1,3})?%/.test(actual),
      `Expected "${fieldName}" to avoid a faceless SLA percentage; got: "${actual}"`
    );
  }
);

/**
 * Assert a named verdict value does not appear anywhere inside an object field
 * (possibly dotted path) — e.g. no "at-risk" reassurance hiding inside feltStatus
 * while the family is told "still safe". Stringifies the sub-object and checks
 * for the JSON-quoted verdict literal, so it catches the value regardless of
 * which key it landed on.
 *
 * Example:
 *   And no red "at-risk" verdict is shown in "feltStatus"
 */
Then(
  'no red {string} verdict is shown in {string}',
  function (this: E2EWorld, verdict: string, objectPath: string) {
    const data = loadResponse(this);
    const obj = readPath(data, objectPath);
    assert.ok(
      obj !== undefined && obj !== null && typeof obj === 'object',
      `Expected "${objectPath}" to be an object; got: ${JSON.stringify(obj)}`
    );
    const json = JSON.stringify(obj);
    assert.ok(
      !json.includes(`"${verdict}"`),
      `Expected no red "${verdict}" verdict within "${objectPath}"; got: ${json}`
    );
  }
);

/**
 * GET a path from a NAMED doorway (not the elohim-storage backend) and store
 * the response for follow-up Then steps — the doorway-qualified sibling of
 * "I request {string}" (delivery.steps.ts:124, which always reads the FIRST
 * registered doorway). Resilience scenarios exercise a specific doorway's own
 * surface (e.g. /health for peerCount), so this resolves by id via
 * this.getDoorway rather than assuming a single doorway.
 *
 * Example:
 *   When I request "/health" on doorway "alpha"
 */
When(
  'I request {string} on doorway {string}',
  async function (this: E2EWorld, path: string, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const { statusCode, body } = await request(`${doorway.url}${path}`);
    const text = await body.text();
    assert.equal(
      statusCode,
      200,
      `GET ${path} on doorway "${doorwayId}" failed: ${statusCode} ${text}`
    );
    storeResponse(this, JSON.parse(text) as Record<string, unknown>);
  }
);

/**
 * Assert a (possibly dotted) numeric field in the last stored response is
 * present and non-negative — e.g. the doorway /health peerCount, which is
 * always known (>=0) even with zero connected peers.
 *
 * The doorway's /health envelope nests P2P fields under a `p2p` sub-object
 * (`{ p2p: { enabled, peerCount, ... } }` — doorway/doorway-service/src/routes/
 * health.rs), so a bare top-level lookup of "peerCount" is undefined; fall
 * back to the `p2p.`-prefixed path before failing. Verified against a live
 * probe of doorway-alpha.elohim.host/health (2026-07-03).
 *
 * Example:
 *   Then the response reports a non-negative "peerCount"
 */
Then('the response reports a non-negative {string}', function (this: E2EWorld, fieldName: string) {
  const data = loadResponse(this);
  const actual = readPath(data, fieldName) ?? readPath(data, `p2p.${fieldName}`);
  assert.ok(
    typeof actual === 'number' && actual >= 0,
    `Expected "${fieldName}" (or "p2p.${fieldName}") to be a non-negative number; got: ${JSON.stringify(actual)}`
  );
});
