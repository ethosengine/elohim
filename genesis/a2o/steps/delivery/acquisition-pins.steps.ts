/**
 * Step definitions for features/delivery/acquisition-pins.feature
 *
 * These steps exercise the /api/v1/pins own-node API on elohim-storage
 * directly via E2E_STORAGE_URL (default: http://localhost:8090).
 *
 * The pin routes are own-node — they are NOT proxied through the doorway —
 * so these steps bypass the E2EWorld doorway map and speak directly to
 * elohim-storage. This mirrors the pattern used in resilience.steps.ts.
 *
 * ## What runs vs what is @wip
 *
 * Scenarios 1 and 2 (airplane-mode API) run whenever a local elohim-storage
 * is reachable at E2E_STORAGE_URL. The @requires:doorway tag signals a
 * running instance is required — the substrate-scope gate holds these
 * scenarios automatically when the stack is unavailable.
 *
 * Scenario 3 (@requires:household-nodes @wip) is the two-node scenario. Its
 * steps are implemented here as @wip stubs returning 'pending'. The load-
 * bearing regression for two-node byte-arrival semantics is the Rust test
 * elohim/elohim-storage/tests/acquisition_pull_e2e.rs — not a cucumber run.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import type { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers — storage direct (own-node)
// ---------------------------------------------------------------------------

/** Resolve the storage base URL from env (matches resilience.steps.ts pattern). */
function storageUrl(): string {
  return process.env['E2E_STORAGE_URL'] ?? 'http://localhost:8090';
}

interface RawResponse {
  status: number;
  body: string;
}

/** POST JSON to elohim-storage, return raw status + body text. */
async function storagePostRaw(
  path: string,
  payload: Record<string, unknown>
): Promise<RawResponse> {
  const { statusCode, body } = await request(`${storageUrl()}${path}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(payload),
  });
  const text = await body.text();
  return { status: statusCode, body: text };
}

/** GET from elohim-storage, return parsed JSON. */
async function storageGetJson(path: string): Promise<{ status: number; body: unknown }> {
  const { statusCode, body } = await request(`${storageUrl()}${path}`);
  const text = await body.text();
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(text);
  } catch {
    parsed = text;
  }
  return { status: statusCode, body: parsed };
}

// World-scoped capture for pin responses (uses a module-level WeakMap to
// avoid mutating the E2EWorld type — same pattern as responseStore in
// delivery.steps.ts).
const pinResponseStore = new WeakMap<E2EWorld, RawResponse>();

// ---------------------------------------------------------------------------
// Scenario 1 — item pin creation and durability
// ---------------------------------------------------------------------------

When(
  'I POST a pin for {string} to \\/api\\/v1\\/pins',
  async function (this: E2EWorld, headRef: string) {
    const resp = await storagePostRaw('/api/v1/pins', { headRef });
    pinResponseStore.set(this, resp);
  }
);

Then('the pin response status is {int}', function (this: E2EWorld, expectedStatus: number) {
  const resp = pinResponseStore.get(this);
  assert.ok(resp, 'No pin response captured — run a "When I POST a pin" step first');
  assert.equal(
    resp.status,
    expectedStatus,
    `Expected status ${expectedStatus} but got ${resp.status}; body: ${resp.body.slice(0, 200)}`
  );
});

Then(
  'GET \\/api\\/v1\\/pins lists one active pin for {string}',
  async function (this: E2EWorld, headRef: string) {
    const { status, body } = await storageGetJson('/api/v1/pins');
    assert.equal(status, 200, `GET /api/v1/pins returned ${status}`);

    const payload = body as Record<string, unknown>;
    const pins = payload['pins'] as Array<Record<string, unknown>>;
    assert.ok(
      Array.isArray(pins),
      `"pins" must be an array; got ${JSON.stringify(payload).slice(0, 200)}`
    );

    const active = pins.filter(p => p['status'] === 'active' && p['headRef'] === headRef);
    assert.equal(
      active.length,
      1,
      `Expected exactly 1 active pin for "${headRef}"; got ${active.length}. Pins: ${JSON.stringify(pins).slice(0, 300)}`
    );
  }
);

// ---------------------------------------------------------------------------
// Scenario 2 — cluster pin 501
// ---------------------------------------------------------------------------

When(
  'I POST a pin with kind {string} for {string} to \\/api\\/v1\\/pins',
  async function (this: E2EWorld, kind: string, headRef: string) {
    const resp = await storagePostRaw('/api/v1/pins', { headRef, kind });
    pinResponseStore.set(this, resp);
  }
);

Then('the pin response body mentions {string}', function (this: E2EWorld, fragment: string) {
  const resp = pinResponseStore.get(this);
  assert.ok(resp, 'No pin response captured — run a "When I POST a pin" step first');
  assert.ok(
    resp.body.includes(fragment),
    `Expected response body to mention "${fragment}"; got: ${resp.body.slice(0, 300)}`
  );
});

// ---------------------------------------------------------------------------
// Scenario 3 — two-node byte-arrival (@wip @requires:household-nodes)
//
// The load-bearing regression for this scenario is the Rust integration test
// elohim/elohim-storage/tests/acquisition_pull_e2e.rs.
// These cucumber steps are scaffolded as pending; a real two-node cucumber
// fixture requires the full household stack which is out of scope for slice 1.
// ---------------------------------------------------------------------------

Given('two connected storage peers where only peer A holds {string}', function (_headRef: string) {
  // Pending: requires a live two-node household stack (E2E_PEER_B_STORAGE_URL).
  // The Rust test acquisition_pull_e2e.rs is the binding regression.
  return 'pending';
});

When('peer B pins {string}', function (_headRef: string) {
  return 'pending';
});

When('the pull queue drains', function () {
  return 'pending';
});

Then(
  "peer B's pull status shows fetched {int} of total {int}",
  function (_fetched: number, _total: number) {
    return 'pending';
  }
);

Then("the content row exists in peer B's local projection", function () {
  return 'pending';
});
