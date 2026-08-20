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

/** Own-node pins API path (runtime arg; the cucumber-expression patterns escape the slashes). */
const PINS_PATH = '/api/v1/pins';

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
  String.raw`I POST a pin for {string} to \/api\/v1\/pins`,
  async function (this: E2EWorld, headRef: string) {
    const resp = await storagePostRaw(PINS_PATH, { headRef });
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
  String.raw`GET \/api\/v1\/pins lists one active pin for {string}`,
  async function (this: E2EWorld, headRef: string) {
    const { status, body } = await storageGetJson(PINS_PATH);
    assert.equal(status, 200, `GET /api/v1/pins returned ${status}`);

    const payload = body as Record<string, unknown>;
    const pins = payload['pins'] as Record<string, unknown>[];
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
  String.raw`I POST a pin with kind {string} for {string} to \/api\/v1\/pins`,
  async function (this: E2EWorld, kind: string, headRef: string) {
    const resp = await storagePostRaw(PINS_PATH, { headRef, kind });
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
// Provide pins (slice 2b — rung 4) — runnable own-node steps
//
// The provide pin (provide:true) is the same own-node /api/v1/pins API; the
// provide flag tells the reconciler to author a replicates-commons commitment.
// GET /api/v1/pins/{eprId}/pull returns the per-EPR rollup the
// PinProgressComponent renders. Both legs run on a single household node.
// ---------------------------------------------------------------------------

When(
  String.raw`I POST a provide pin for {string} to \/api\/v1\/pins`,
  async function (this: E2EWorld, headRef: string) {
    const id = headRef.replace(/^epr:/, '');
    const resp = await storagePostRaw(PINS_PATH, {
      headRef: id,
      kind: 'item',
      provide: true,
    });
    pinResponseStore.set(this, resp);
  }
);

When(
  'I POST a provide pin with a forced browser context for {string}',
  async function (this: E2EWorld, headRef: string) {
    const id = headRef.replace(/^epr:/, '');
    // The provide flag is honored only on a peer-capable node; a browser-context
    // provide must be refused 400. The own-node API enforces this server-side.
    const resp = await storagePostRaw(PINS_PATH, {
      headRef: id,
      kind: 'item',
      provide: true,
      context: 'browser',
    });
    pinResponseStore.set(this, resp);
  }
);

Then(
  String.raw`GET \/api\/v1\/pins\/{word}\/pull reports a pull rollup`,
  async function (this: E2EWorld, eprId: string) {
    const { status, body } = await storageGetJson(`/api/v1/pins/${eprId}/pull`);
    // A 404 here means "this node holds no pin for that EPR" — which would
    // contradict the 201 the previous step just got. It must NOT mean "the pin
    // exists but acquisition has not measured it yet"; that answer is a 200
    // carrying the tri-state nulls (total/caughtUp null = keep waiting).
    assert.equal(
      status,
      200,
      `GET pull rollup returned ${status}` +
        (status === 404
          ? ` — the node denied holding a pin it accepted one step ago; body: ${JSON.stringify(body).slice(0, 200)}`
          : '')
    );
    const rollup = body as Record<string, unknown>;
    // The rollup is grouped by head_ref; the shape must carry the whole wire
    // contract (epr-pull-status.schema.json): the id it answers for, the
    // counters PinProgressComponent renders, and the tri-state total/caughtUp.
    for (const field of ['eprId', 'total', 'fetched', 'pending', 'failed', 'caughtUp']) {
      assert.ok(
        field in rollup,
        `pull rollup missing "${field}"; got ${JSON.stringify(rollup).slice(0, 200)}`
      );
    }
    // Never a false-complete: an unmeasured rollup reports caughtUp null, and a
    // measured one reports a boolean — but never `true` with a null total.
    if (rollup['total'] === null) {
      assert.equal(
        rollup['caughtUp'],
        null,
        `an unmeasured rollup must not claim caught-up; got ${JSON.stringify(rollup).slice(0, 200)}`
      );
    }
  }
);

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

// Provide-serve cross-node leg (@wip @requires:household-nodes). The binding
// regression is the Rust e2e replicates_commons test, not this cucumber stub.
When('peer A pins {string} as a peer', function (_headRef: string) {
  // Pending: requires a live two-node household stack. The provide-serve
  // regression lives in the Rust integration test (replicates_commons e2e).
  return 'pending';
});

Then('peer B fetched the bytes from peer A', function () {
  return 'pending';
});

// ---------------------------------------------------------------------------
// Acquisition constraint regressions (@wip @regression @requires:household-nodes)
//
// Harvested 2026-07-26 from the resiliency-saga overnight cure sprint. Both
// defects lived inside the eight-deep chapter-5 deadlock chain and neither
// announced itself — acquisition just went quiet.
//
// The BINDING regressions are Rust unit tests, not these stubs:
//   * retry rotation — elohim/elohim-storage/src/p2p/mod.rs,
//     mod acquisition_rotation_tests
//     (successive_retries_of_a_stable_position_walk_distinct_peers). Parameters:
//     MAX_RETRIES=3, 6-peer fabric, 60s retry-on-next-cycle reconcile.
//   * in-flight budget release — elohim/elohim-storage/src/p2p/reconcile_rails.rs
//     (dispatch_budget_caps_inflight) + the ShardResponse::Error arm in
//     p2p/mod.rs. Parameters: MAX_ACQUISITION_INFLIGHT=25; 25 leaked slots wedge
//     acquisition permanently and silently.
//
// These cucumber steps stay pending until a household-scale acquisition fixture
// (a 6-peer fabric with injectable failure responses) exists — the same
// scaffolded-pending posture as the two-node scenarios above.
// ---------------------------------------------------------------------------

Given('an acquisition fabric of {int} connected peers', function (_peerCount: number) {
  // Pending: needs a multi-peer household stack with an injectable peer set.
  return 'pending';
});

Given('{string} sits at a stable position in the acquisition batch', function (_headRef: string) {
  // Pending: the stable batch position comes from list_active_pins' DB order —
  // reproducing it needs control over the pin table, not just the HTTP surface.
  return 'pending';
});

When('its acquisition retries until the retry budget is exhausted', function () {
  // Pending: MAX_RETRIES=3 attempts across 3 x 60s reconcile cycles.
  return 'pending';
});

Then('the item was probed on {int} distinct peers', function (_distinctPeers: number) {
  return 'pending';
});

When(
  '{int} acquisition dispatches are answered with an error-class shard response',
  function (_count: number) {
    // Pending: needs a peer that can be told to answer ShardResponse::Error.
    return 'pending';
  }
);

Then('the available dispatch budget returns to {int}', function (_available: number) {
  return 'pending';
});

Then('a subsequent acquisition drain dispatches at least one request', function () {
  return 'pending';
});
