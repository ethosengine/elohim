/**
 * Step glue for the operator-runtime-surface concern
 * (features/dataplane/operator-commitment-gated-verbs.feature) — operator
 * verbs must be REA-commitment-gated protocol acts, never kubectl-shaped
 * cluster surgery. See that feature's header comment for the full RED-FIRST
 * rationale and the "what the code already taught us" section — the
 * commitment-gated authorization primitive already exists
 * (`operation_authorization::authorize_operation`,
 * `elohim/elohim-storage/src/services/operation_authorization.rs`) and is
 * fail-closed today; what is MISSING is the operator verb route that consults
 * it. These steps probe the target contract honestly and must run RED until
 * that wiring lands — see the honesty notes below.
 *
 * Four scenarios live here:
 *   1. A commitment holder ("matthew") drives a reconcile on peer
 *      "alpha-A" and gets a 200 attestation naming the commitment it
 *      acted under.
 *   2. A caller with no grant ("james") is refused (401/403).
 *   3. A revoked commitment stops working immediately (401/403).
 *   4. A peer answers its own runtime telemetry directly (no observability
 *      hop) — this one already works today (existing `/p2p/status` surface).
 *
 * Background peers ("alpha-A", "elohim.host") are registered by the
 * `Given peer {string} at {string}` step in dataplane.steps.ts — NOT
 * redefined here (cucumber errors on ambiguous step matches).
 *
 * ## Target contracts (not yet built for scenarios 1-3)
 *
 *   POST {doorwayBase}/api/v1/operator/reconcile
 *     Body: {} (performer identity comes from the caller's JWT, doorway-side).
 *     Expected once built: 200 + `{ verb: "reconcile", commitmentCid, ... }`
 *     attestation; 403 when the caller holds no active/valid grant.
 *     TODAY: this route does not exist (404) — scenarios 1-3 are honestly RED.
 *
 * ## Grant fixture lever
 *
 *   POST {storageBase}/admin/seed/delegates-compute
 *     Rust source: elohim/elohim-storage/src/api/seed_delegates_compute.rs.
 *     Flag-gated by ALLOW_SEED_DELEGATES_COMPUTE (403 when off — the dev/prod
 *     honesty boundary; NEVER set in production). A storage NODE-LOCAL admin
 *     route, NOT proxied by the doorway — reached via `resolveStorageUrl()`
 *     (E2E_STORAGE_ALPHA/E2E_STORAGE_URL for alpha-A, E2E_STORAGE_<PEER>
 *     otherwise), mirroring the `/admin/seed/shard-manifest` precedent in
 *     resilience.steps.ts. When the storage URL isn't configured for this
 *     peer, OR the lever answers 403, the Given step returns 'pending' —
 *     never a false pass/fail.
 *
 *   `bounds_validator::validate` (elohim-storage/src/services/bounds_validator.rs)
 *   requires `bounds.epr_scope` (array, `"*"` wildcard accepted) and
 *   `bounds.reach_ceiling` with no permissive default — the seeded grant sets
 *   both maximally open (`["*"]` / `"commons"`) so it authorizes whatever the
 *   not-yet-built route ends up requesting. `rate_per_hour` and
 *   `rotation_ttl_days` default to unlimited when omitted (checked in
 *   bounds_validator.rs checks 6-7), so they're left out.
 *
 *   `commitment.scope` must equal the reconcile route's `capability` string
 *   (bounds_validator check 4: `event.action == commitment.scope`) — pinned
 *   to `operator-reconcile` per OPERATOR_RECONCILE_CAPABILITY in
 *   elohim/elohim-storage/src/api/operator_verbs.rs (the route's contract).
 *
 * ## Honesty rules
 *
 *   - "the request is refused" asserts an EXPLICIT 401/403. A 404 (route not
 *     built) or 5xx (infra failure) FAILS the step — that is the intended RED
 *     today, not a thing to paper over.
 *   - "performs no reconcile" only asserts non-acceptance (status !== 200).
 *   - "performs the reconcile" / "attests the commitment cid" only assert
 *     once a real 200 attestation exists.
 *
 * ## Auth note (the "API-mode auth gap" was falsified 2026-08-18)
 *
 * There is no existing "log this fixture human in and hand me a bearer token
 * for arbitrary API-mode use" helper outside the Playwright/BrowserDevice
 * login flows (fixture-humans.steps.ts). `ensurePersonaLoggedIn()` below logs
 * in directly against the peer's DoorwayClient (registered by the Background
 * `Given peer ...` step) and reads the bearer off `client.session.token` for
 * a raw authenticated `undici` POST — the same "call the private post via the
 * client's token" shape used in `operator-onboarding.steps.ts`'s
 * `adminDelete()`, just without depending on a BrowserDevice.
 *
 * That raw login WORKS — the historical "Invalid credentials" failures were
 * not an API-mode gap but a doorway-realm mismatch: the scenarios originally
 * addressed peer "elohim.host" (doorway alpha-B), a separate identity realm
 * (own user DB, own JWT secret) the seeder never populates, so login 401'd
 * before any verb was reached. The scenarios now address peer "alpha-A", the
 * realm the fixture personas are seeded into (seed-humans.ts targets
 * doorway-alpha only). An A-issued JWT is NOT valid at doorway-B (different
 * JWT secrets), so login and the gated verb must target the same doorway.
 */

import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { resolveStorageUrl, probeP2PStatus } from '../../src/framework/dataplane/surfaces.js';
import { getFixture, isHumanDeployed } from '../../src/framework/fixtures/humans.js';
import { Human } from '../../src/framework/human.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** See the "Grant fixture lever" doc comment above for why this value is inferred, not verified. */
const OPERATOR_RECONCILE_CAPABILITY = 'operator-reconcile';

/** Arbitrary dev-seed grantor identity — bounds_validator never checks `provider` identity. */
const OPERATOR_RECONCILE_GRANTOR = 'agent:operator-commitment-gated-verbs-dev-seed';

// ---------------------------------------------------------------------------
// World-scoped state (WeakMap keyed by E2EWorld, mirrors dataplane.steps.ts)
// ---------------------------------------------------------------------------

interface OperatorVerbState {
  /** The delegates-compute commitment cid seeded by a "holds/held" Given step. */
  commitmentCid?: string;
  /** The direct storage base URL used to seed/revoke that commitment. */
  storageBaseUrl?: string;
  /** Last response from POST {doorway}/api/v1/operator/reconcile. */
  lastReconcile?: { status: number; body: Record<string, unknown> | undefined; text: string };
  /** Last response from GET {storage}/p2p/status (direct runtime-telemetry probe). */
  lastTelemetry?: { storageBaseUrl: string; body: Record<string, unknown> };
}

const stateFor = new WeakMap<E2EWorld, OperatorVerbState>();

function state(world: E2EWorld): OperatorVerbState {
  let s = stateFor.get(world);
  if (!s) {
    s = {};
    stateFor.set(world, s);
  }
  return s;
}

// ---------------------------------------------------------------------------
// Persona helpers
// ---------------------------------------------------------------------------

/** humans.json displayNames are capitalized ("Matthew", "James"); the feature uses lowercase. */
function capitalizePersona(raw: string): string {
  return raw.charAt(0).toUpperCase() + raw.slice(1);
}

/**
 * Log a fixture persona in against the given peer's already-registered
 * DoorwayClient (added by the Background `Given peer {string} at {string}`
 * step) and return their agent CID + bearer token. Idempotent — safe to call
 * from more than one step in a scenario. Returns 'pending' when the persona
 * isn't deployed on this test bench (see isHumanDeployed doc).
 */
async function ensurePersonaLoggedIn(
  world: E2EWorld,
  personaRaw: string,
  peerName: string
): Promise<{ displayName: string; agentPubKey: string; token: string } | 'pending'> {
  const displayName = capitalizePersona(personaRaw);
  if (!isHumanDeployed(displayName)) {
    return 'pending';
  }
  const fixture = getFixture(displayName);
  const doorway = world.getDoorway(peerName);

  const auth = await doorway.client.login({
    identifier: fixture.credentials.identifier,
    password: fixture.credentials.password,
  });
  doorway.client.setToken(auth.token);

  let human = world.humans.get(displayName);
  if (!human) {
    human = new Human(displayName, fixture.credentials);
    world.addHuman(displayName, human);
  }
  human.agentPubKey = auth.agentPubKey;
  human.humanId = auth.humanId;
  human.setToken(peerName, auth.token);

  return { displayName, agentPubKey: auth.agentPubKey, token: auth.token };
}

/**
 * Seed an active delegates-compute grant for `personaRaw` over `peerName` via
 * the honesty-gated dev-seed lever. Stores the minted cid + storage URL on
 * world state for follow-up steps (revoke, attestation-cid comparison).
 * Returns 'pending' when the persona isn't deployed, the peer's storage URL
 * isn't configured for this harness, or the lever is off (403) — never a
 * false pass/fail. Returns undefined on success.
 */
async function grantDelegatesCompute(
  world: E2EWorld,
  personaRaw: string,
  peerName: string
): Promise<string | undefined> {
  const login = await ensurePersonaLoggedIn(world, personaRaw, peerName);
  if (login === 'pending') return 'pending';

  const storageBaseUrl = resolveStorageUrl(peerName);
  if (!storageBaseUrl) return 'pending';

  const cid = `commitment:operator-reconcile-${personaRaw}-${randomUUID()}`;
  const nowIso = new Date().toISOString();
  const validUntil = new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString();

  const { statusCode, body } = await request(`${storageBaseUrl}/admin/seed/delegates-compute`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      cid,
      scope: OPERATOR_RECONCILE_CAPABILITY,
      provider: OPERATOR_RECONCILE_GRANTOR,
      recipient: login.agentPubKey,
      // epr_scope + reach_ceiling are the two bounds fields bounds_validator
      // requires with no permissive default (checks 4b/5) — wide open so the
      // grant authorizes whatever the not-yet-built route ends up sending.
      bounds: { epr_scope: ['*'], reach_ceiling: 'commons', _provenance: 'dev-seed' },
      validFrom: nowIso,
      validUntil,
    }),
  });
  const text = await body.text();

  if (statusCode === 403) {
    // Honesty gate: ALLOW_SEED_DELEGATES_COMPUTE is off on this storage
    // process. Skip rather than false-fail (shard-manifest lever precedent).
    return 'pending';
  }
  assert.ok(statusCode < 300, `seed delegates-compute failed: ${statusCode} ${text}`);

  state(world).commitmentCid = cid;
  state(world).storageBaseUrl = storageBaseUrl;
  return undefined;
}

// ---------------------------------------------------------------------------
// Given — grant fixtures
// ---------------------------------------------------------------------------

Given(
  '{string} holds an active {string} commitment over peer {string}',
  async function (
    this: E2EWorld,
    personaRaw: string,
    commitmentAction: string,
    peerName: string
  ): Promise<string | undefined> {
    // perform_seed (seed_delegates_compute.rs) always writes action=
    // "delegates-compute" — the endpoint doesn't accept an action field at
    // all. This equality is a documentary sanity check on the story, not
    // something wired to the seed call.
    assert.equal(
      commitmentAction,
      'delegates-compute',
      `only "delegates-compute" commitments are seedable via this lever, got "${commitmentAction}"`
    );
    return grantDelegatesCompute(this, personaRaw, peerName);
  }
);

Given(
  '{string} held a {string} commitment over peer {string}',
  async function (
    this: E2EWorld,
    personaRaw: string,
    commitmentAction: string,
    peerName: string
  ): Promise<string | undefined> {
    assert.equal(
      commitmentAction,
      'delegates-compute',
      `only "delegates-compute" commitments are seedable via this lever, got "${commitmentAction}"`
    );
    return grantDelegatesCompute(this, personaRaw, peerName);
  }
);

Given(
  '{string} holds no {string} commitment over peer {string}',
  async function (
    this: E2EWorld,
    personaRaw: string,
    _commitmentAction: string,
    peerName: string
  ): Promise<string | undefined> {
    // No grant seeded — just authenticate so the refusal is attributable to
    // the gate, not to a missing/invalid bearer token.
    const login = await ensurePersonaLoggedIn(this, personaRaw, peerName);
    return login === 'pending' ? 'pending' : undefined;
  }
);

Given('that commitment has been revoked', async function (this: E2EWorld): Promise<void> {
  const s = state(this);
  assert.ok(
    s.commitmentCid && s.storageBaseUrl,
    'no commitment was seeded by a prior "held a ... commitment" step'
  );

  const { statusCode, body } = await request(`${s.storageBaseUrl}/admin/seed/delegates-compute`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ cid: s.commitmentCid, revoke: true }),
  });
  const text = await body.text();
  assert.ok(statusCode < 300, `revoke delegates-compute failed: ${statusCode} ${text}`);
});

// ---------------------------------------------------------------------------
// When — the commitment-gated verb itself
// ---------------------------------------------------------------------------

When(
  '{string} requests a reconcile on peer {string} through the doorway',
  async function (
    this: E2EWorld,
    personaRaw: string,
    peerName: string
  ): Promise<string | undefined> {
    const login = await ensurePersonaLoggedIn(this, personaRaw, peerName);
    if (login === 'pending') return 'pending';

    const doorway = this.getDoorway(peerName);
    const { statusCode, body } = await request(`${doorway.url}/api/v1/operator/reconcile`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        authorization: `Bearer ${login.token}`,
      },
      body: JSON.stringify({}),
    });
    const text = await body.text();
    let json: Record<string, unknown> | undefined;
    try {
      json = JSON.parse(text) as Record<string, unknown>;
    } catch {
      json = undefined;
    }
    state(this).lastReconcile = { status: statusCode, body: json, text };
    return undefined;
  }
);

// ---------------------------------------------------------------------------
// Then — acceptance path
// ---------------------------------------------------------------------------

Then('the request is accepted', function (this: E2EWorld): void {
  const s = state(this);
  assert.ok(s.lastReconcile, 'no reconcile response recorded — run the reconcile When step first');
  assert.equal(
    s.lastReconcile.status,
    200,
    `expected 200 (accepted), got ${s.lastReconcile.status}: ${s.lastReconcile.text.slice(0, 300)}`
  );
});

Then('peer {string} performs the reconcile', function (this: E2EWorld, _peerName: string): void {
  const s = state(this);
  assert.ok(s.lastReconcile, 'no reconcile response recorded — run the reconcile When step first');
  assert.equal(
    s.lastReconcile.status,
    200,
    `expected a 200 attestation, got ${s.lastReconcile.status}: ${s.lastReconcile.text.slice(0, 300)}`
  );
  const body = s.lastReconcile.body;
  assert.ok(body, `200 response body was not valid JSON: ${s.lastReconcile.text.slice(0, 300)}`);
  assert.equal(
    body['verb'],
    'reconcile',
    `expected verb:"reconcile" in the attestation body, got ${JSON.stringify(body)}`
  );
  // The design doesn't pin an exact outcome-field name yet (task spec: "an
  // outcome/accepted field") — check the plausible candidates rather than
  // over-fitting a guess.
  const acceptedIndicators = [
    body['accepted'] === true,
    body['outcome'] === 'accepted',
    body['performed'] === true,
    body['reconciled'] === true,
  ];
  assert.ok(
    acceptedIndicators.some(Boolean),
    `no outcome/accepted-shaped field found in attestation body: ${JSON.stringify(body)}`
  );
});

Then(
  'peer {string} attests the commitment cid it acted under',
  function (this: E2EWorld, _peerName: string): void {
    const s = state(this);
    const body = s.lastReconcile?.body;
    assert.ok(body, 'no 200 reconcile attestation body recorded');
    const commitmentCid = body['commitmentCid'];
    assert.equal(
      typeof commitmentCid,
      'string',
      `expected a commitmentCid string field, got ${JSON.stringify(body)}`
    );
    assert.ok((commitmentCid as string).length > 0, 'commitmentCid was an empty string');
    if (s.commitmentCid) {
      assert.equal(
        commitmentCid,
        s.commitmentCid,
        `attested commitmentCid (${String(commitmentCid)}) does not match the grant seeded for ` +
          `this scenario (${s.commitmentCid}) — the peer acted under the wrong commitment`
      );
    }
  }
);

// ---------------------------------------------------------------------------
// Then — refusal path
// ---------------------------------------------------------------------------

Then('the request is refused', function (this: E2EWorld): void {
  const s = state(this);
  assert.ok(s.lastReconcile, 'no reconcile response recorded — run the reconcile When step first');
  assert.ok(
    s.lastReconcile.status === 401 || s.lastReconcile.status === 403,
    `expected an explicit 401/403 refusal, got ${s.lastReconcile.status}: ` +
      `${s.lastReconcile.text.slice(0, 300)} (a 404 means the route isn't built yet, a 5xx is an ` +
      'infra failure — neither counts as "refused")'
  );
});

Then('peer {string} performs no reconcile', function (this: E2EWorld, _peerName: string): void {
  const s = state(this);
  assert.ok(s.lastReconcile, 'no reconcile response recorded — run the reconcile When step first');
  assert.notEqual(
    s.lastReconcile.status,
    200,
    `expected no acceptance, but the peer answered 200: ${s.lastReconcile.text.slice(0, 300)}`
  );
});

// ---------------------------------------------------------------------------
// Scenario 4 — a peer serves its own runtime telemetry directly
// ---------------------------------------------------------------------------

When(
  'peer {string} is asked for its runtime status directly',
  async function (this: E2EWorld, peerName: string): Promise<string | undefined> {
    const storageBaseUrl = resolveStorageUrl(peerName);
    if (!storageBaseUrl) return 'pending';

    const { body } = await probeP2PStatus(storageBaseUrl);
    state(this).lastTelemetry = {
      storageBaseUrl,
      body: body as unknown as Record<string, unknown>,
    };
    return undefined;
  }
);

Then(
  'peer {string} reports its own sync, reconcile and peer counts',
  function (this: E2EWorld, _peerName: string): void {
    const s = state(this);
    assert.ok(
      s.lastTelemetry,
      'no telemetry response recorded — run the runtime-status When step first'
    );
    const body = s.lastTelemetry.body;
    assert.equal(
      typeof body['connectedPeers'],
      'number',
      `expected numeric connectedPeers (peer count), got ${JSON.stringify(body['connectedPeers'])}`
    );
    assert.equal(
      typeof body['syncDocuments'],
      'number',
      `expected numeric syncDocuments (sync count), got ${JSON.stringify(body['syncDocuments'])}`
    );
    assert.equal(
      typeof body['reconcilePassesTotal'],
      'number',
      `expected numeric reconcilePassesTotal (reconcile count), got ` +
        `${JSON.stringify(body['reconcilePassesTotal'])}`
    );
  }
);

Then(
  "the answer does not traverse the operator's observability cluster",
  function (this: E2EWorld): void {
    const s = state(this);
    assert.ok(
      s.lastTelemetry,
      'no telemetry response recorded — run the runtime-status When step first'
    );
    // The probe hit the peer's OWN storage host directly (storageBaseUrl,
    // resolved above) — never Grafana/Loki/Prometheus. There is no
    // observability-cluster hop to traverse by construction; this assertion
    // makes that fact visible in the scenario rather than leaving it implicit.
    assert.ok(
      !/grafana|loki|prometheus/i.test(s.lastTelemetry.storageBaseUrl),
      `unexpectedly probed an observability host: ${s.lastTelemetry.storageBaseUrl}`
    );
  }
);
