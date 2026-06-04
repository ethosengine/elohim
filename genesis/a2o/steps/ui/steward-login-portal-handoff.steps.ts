/**
 * Steward login portal-handoff step definitions.
 *
 * Backs genesis/a2o/features/auth/steward-login-portal-handoff.feature:
 * doorway, acting as OAuth relying-party, recognizes a graduated steward at
 * login and hands the authentication off to the steward's peer-native portal
 * host. Doorway never owns a steward's login decision; when the credential
 * check succeeds and the AuthResponse carries a reachable portalHostUrl, the
 * SPA reads that URL from the JSON and navigates to the peer-native portal,
 * which completes the OAuth code dance back at the original client_id.
 *
 * Architectural note — the redirect is CLIENT-driven (the Angular SPA reads
 * `portalHostUrl` from the /auth/login JSON and navigates). There is
 * deliberately NO doorway 302. The browser assertions below therefore wait on
 * the resulting navigation URL, NOT on an HTTP 302 status. This satisfies the
 * scenario wording "the browser is redirected" while honouring the design.
 *
 * Two threading modes (mirrors account-m5.steps.ts):
 *   - HTTP mode (default): the response-field assertions run against a raw POST
 *     to /auth/login; the browser-navigation assertions return 'pending'.
 *   - Playwright mode (E2E_DEVICE_MODE=playwright): the submit step ALSO drives
 *     the threshold-login form so the client-driven navigation can be observed.
 *
 * Reality threading: the AuthResponse `isSteward` / `portalHostUrl` fields and
 * the SPA's read-and-navigate behaviour land with the GAP-1 doorway change.
 * Until then these steps bind (dry-run / typecheck clean) but scenario
 * EXECUTION awaits GAP-1; the response-field assertions will fail-loud against
 * the current AuthResponse shape, which is the intended red→green signal.
 *
 * Framework: Playwright + @cucumber/cucumber (NOT Cypress).
 * Tags: @e2e @auth @browser-only @requires:doorway @recovery-m5
 *       @auth-portal-convergence
 *
 * Scenario 4 ("Hosted visitor receives no portalHostUrl") is tagged
 * @requires:shem and is HELD at runtime by the substrate-scope Before gate in
 * steps/common.steps.ts (it needs the remote multi-tenant canvas to seed a
 * hosted visitor). Its steps are nonetheless DEFINED here: cucumber's
 * --dry-run binding check resolves steps independently of the runtime gate, so
 * leaving them undefined would surface as UNDEFINED in binding validation. The
 * @requires:shem tag holds the scenario; these definitions keep the binding
 * clean.
 *
 * See:
 *   - genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md
 *   - .claude/memory/project_m5_reframe_auth_portal_convergence.md
 *   - .claude/memory/project_peer_native_account_canonical_surface.md
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { BrowserDevice } from '../../src/framework/devices/browser-device.js';
import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { getFixture } from '../../src/framework/fixtures/humans.js';
import { Human } from '../../src/framework/human.js';
import { ThresholdLoginPage } from '../../src/framework/pages/index.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// contentIds keys — captured login response + registered portal host
// ---------------------------------------------------------------------------

const LOGIN_STATUS_KEY = 'portalHandoffLoginStatus';
const LOGIN_BODY_KEY = 'portalHandoffLoginBody';
const REGISTERED_HOST_KEY = 'portalHostUrl'; // shared key with account-m5 redirect steps

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Resolve the first registered doorway URL. Throws if the Background missed. */
function requireDoorwayUrl(world: E2EWorld): string {
  for (const [, doorway] of world.doorways) {
    return doorway.url;
  }
  throw new Error('No doorway registered. Did the Background step run?');
}

/** First Playwright device in the world, or null when not in playwright mode. */
function firstPlaywrightDevice(world: E2EWorld): PlaywrightDevice | null {
  if (world.deviceMode !== 'playwright') return null;
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  return null;
}

/** Parse the captured /auth/login JSON body, or fail with the raw text. */
function loginResponseJson(world: E2EWorld): Record<string, unknown> {
  const raw = world.contentIds.get(LOGIN_BODY_KEY);
  assert.ok(
    raw !== undefined,
    'No /auth/login response captured — did the "submits credentials" step run?'
  );
  try {
    return JSON.parse(raw) as Record<string, unknown>;
  } catch {
    throw new Error(`/auth/login response was not JSON: ${raw}`);
  }
}

/**
 * Raw POST that captures status + body without throwing on non-2xx (the typed
 * DoorwayClient throws, which would mask the wire-shape assertions here).
 * Mirrors rawPost in account-m5.steps.ts.
 */
async function rawPost(
  baseUrl: string,
  path: string,
  payload: unknown,
  token?: string
): Promise<{ statusCode: number; body: string }> {
  const headers: Record<string, string> = { 'content-type': 'application/json' };
  if (token) headers['authorization'] = `Bearer ${token}`;
  const { statusCode, body } = await request(`${baseUrl}${path}`, {
    method: 'POST',
    headers,
    body: JSON.stringify(payload),
  });
  const text = await body.text();
  return { statusCode, body: text };
}

// ---------------------------------------------------------------------------
// Given — fixture setup
// ---------------------------------------------------------------------------

/**
 * Matthew is a graduated steward whose peer-native portal host is registered.
 *
 * Extends the fixture-humans Matthew toward the graduated-steward-with-portal
 * variant: log in via HTTP (BrowserDevice) for the bearer token, then register
 * the portal host through the same /api/v1/account/portal-hosts surface the
 * account-m5 portal-host-discovery scenarios use. The registered host URL is
 * stashed for the redirect/match assertions. Registration is best-effort —
 * until the GAP-1 surface accepts it, the host URL is still recorded from the
 * scenario argument so the wire-shape assertions remain meaningful.
 *
 * Example:
 *   Given human "Matthew" is a graduated steward with portal host
 *     "https://matthew.steward.example/account"
 */
Given(
  'human {string} is a graduated steward with portal host {string}',
  async function (this: E2EWorld, humanName: string, portalHostUrl: string) {
    // Record the registered host for the later match / redirect assertions.
    this.contentIds.set(REGISTERED_HOST_KEY, portalHostUrl);

    const fixture = getFixture(humanName);
    const doorway = requireDoorwayUrl(this);

    // Establish an authenticated session (HTTP — works in both device modes).
    const human = new Human(humanName, fixture.credentials);
    const device = new BrowserDevice(`${humanName}-browser`, doorway);
    human.addDevice(device);
    const auth = await device.login({
      identifier: fixture.credentials.identifier,
      password: fixture.credentials.password,
    });
    human.agentPubKey = auth.agentPubKey;
    human.humanId = auth.humanId;
    this.addHuman(humanName, human);

    // Register the portal host via the canonical account surface (best-effort:
    // the graduated-steward grant + this surface land with GAP-1; a non-2xx
    // here does not fail setup — the host URL is already recorded above).
    await rawPost(
      doorway,
      '/api/v1/account/portal-hosts',
      { hostUrl: portalHostUrl, label: `${humanName}'s steward portal` },
      device.token
    );
  }
);

/**
 * Portal host is unreachable — doorway falls through to local auth.
 *
 * Note the wording "the portal host" (account-m5 uses "my portal host"); both
 * record the same unreachable precondition on world state.
 *
 * Example:
 *   Given the portal host does not respond to /healthz
 */
Given(String.raw`the portal host does not respond to \/healthz`, function (this: E2EWorld) {
  this.contentIds.set('portalHostUnreachable', 'true');
});

/**
 * A hosted visitor with no portal host registered (scenario 4, @requires:shem).
 *
 * Held at runtime by the substrate gate; defined so binding validation passes.
 * Records the visitor's fixture credentials for the submit step.
 *
 * Example:
 *   Given human "Susan" is a hosted visitor with no portal host registered
 */
Given(
  'human {string} is a hosted visitor with no portal host registered',
  async function (this: E2EWorld, humanName: string) {
    const fixture = getFixture(humanName);
    const doorway = requireDoorwayUrl(this);

    const human = new Human(humanName, fixture.credentials);
    const device = new BrowserDevice(`${humanName}-browser`, doorway);
    human.addDevice(device);
    const auth = await device.login({
      identifier: fixture.credentials.identifier,
      password: fixture.credentials.password,
    });
    human.agentPubKey = auth.agentPubKey;
    human.humanId = auth.humanId;
    this.addHuman(humanName, human);
  }
);

// ---------------------------------------------------------------------------
// When — submit credentials at the threshold-login page
// ---------------------------------------------------------------------------

/**
 * Submit credentials at the doorway threshold-login page.
 *
 * Captures the /auth/login JSON (status + body) for the wire-shape assertions
 * via a raw POST. In Playwright mode it ALSO drives the real threshold-login
 * form so the client-driven navigation (the SPA reading portalHostUrl and
 * navigating to the portal host) can be observed by the redirect assertions.
 *
 * Uses {word} so a single definition serves both "Matthew submits…" and
 * "Susan submits…".
 *
 * Example:
 *   When Matthew submits credentials at the threshold-login page
 */
When(
  '{word} submits credentials at the threshold-login page',
  async function (this: E2EWorld, humanName: string) {
    const doorway = requireDoorwayUrl(this);
    const fixture = getFixture(humanName);

    // 1) Capture the /auth/login wire shape (raw — never throws on non-2xx).
    const { statusCode, body } = await rawPost(doorway, '/auth/login', {
      identifier: fixture.credentials.identifier,
      password: fixture.credentials.password,
    });
    this.contentIds.set(LOGIN_STATUS_KEY, String(statusCode));
    this.contentIds.set(LOGIN_BODY_KEY, body);

    // 2) In Playwright mode, drive the form so the client-driven redirect is
    //    observable. The SPA reads portalHostUrl from the JSON and navigates;
    //    there is no doorway 302 to wait on.
    const device = firstPlaywrightDevice(this);
    if (!device) return; // HTTP mode — browser-navigation assertions skip.

    await device.navigate('/threshold/login');
    const loginPage = new ThresholdLoginPage(device.page);
    await loginPage.login(fixture.credentials.identifier, fixture.credentials.password);
    // Allow the post-auth navigation (client-driven) to settle.
    await device.page.waitForLoadState('networkidle');
  }
);

// ---------------------------------------------------------------------------
// Then — /auth/login response-field assertions (wire shape)
// ---------------------------------------------------------------------------

/**
 * The /auth/login response includes "isSteward": true (or false).
 *
 * The feature phrases this as `"isSteward": true` / `"isSteward": false`;
 * cucumber binds the quoted field as {string} and the boolean literal in the
 * step text. Two definitions (true / false) keep the phrasing readable.
 *
 * Example:
 *   Then the /auth/login response includes "isSteward": true
 */
Then(
  String.raw`the \/auth\/login response includes {string}: true`,
  function (this: E2EWorld, field: string) {
    const json = loginResponseJson(this);
    assert.strictEqual(
      json[field],
      true,
      `Expected /auth/login "${field}" to be true, got ${JSON.stringify(json[field])}`
    );
  }
);

Then(
  String.raw`the \/auth\/login response includes {string}: false`,
  function (this: E2EWorld, field: string) {
    const json = loginResponseJson(this);
    assert.strictEqual(
      json[field],
      false,
      `Expected /auth/login "${field}" to be false, got ${JSON.stringify(json[field])}`
    );
  }
);

/**
 * The /auth/login response includes a "portalHostUrl" matching his registered host.
 *
 * Example:
 *   Then the /auth/login response includes a "portalHostUrl" matching his registered host
 */
Then(
  String.raw`the \/auth\/login response includes a {string} matching his registered host`,
  function (this: E2EWorld, field: string) {
    const json = loginResponseJson(this);
    const expected = this.contentIds.get(REGISTERED_HOST_KEY);
    assert.ok(expected, 'No registered portal host recorded — did the fixture step run?');
    assert.strictEqual(
      json[field],
      expected,
      `Expected /auth/login "${field}" to equal registered host "${expected}", got ` +
        `${JSON.stringify(json[field])}`
    );
  }
);

/**
 * The /auth/login response does NOT include "portalHostUrl".
 *
 * Shared by the unreachable-portal fall-through (scenario 3) and the hosted
 * visitor (scenario 4). A field is "not included" if absent or null.
 *
 * Example:
 *   Then the /auth/login response does NOT include "portalHostUrl"
 */
Then(
  String.raw`the \/auth\/login response does NOT include {string}`,
  function (this: E2EWorld, field: string) {
    const json = loginResponseJson(this);
    const value = json[field];
    assert.ok(
      value === undefined || value === null,
      `Expected /auth/login to NOT include "${field}", but got ${JSON.stringify(value)}`
    );
  }
);

// ---------------------------------------------------------------------------
// Then — client-driven browser redirect (scenario 2)
// ---------------------------------------------------------------------------

/**
 * The browser is redirected to the portal host.
 *
 * CLIENT-driven: the SPA reads portalHostUrl from the JSON and navigates —
 * asserted against the resulting navigation URL, NOT an HTTP 302. HTTP mode
 * has no browser to observe, so this returns 'pending'.
 *
 * Example:
 *   Then the browser is redirected to "https://matthew.steward.example/account"
 */
Then(
  'the browser is redirected to {string}',
  async function (this: E2EWorld, portalHostUrl: string) {
    const device = firstPlaywrightDevice(this);
    if (!device) return 'pending';

    await device.page.waitForURL((url: URL) => url.href.startsWith(portalHostUrl), {
      timeout: 15_000,
    });
    const current = device.page.url();
    assert.ok(
      current.startsWith(portalHostUrl),
      `Expected client-driven redirect to "${portalHostUrl}" but at: ${current}`
    );
  }
);

/**
 * The redirect URL carries a session_token query parameter.
 *
 * Example:
 *   And the redirect URL carries a session_token query parameter
 */
Then('the redirect URL carries a session_token query parameter', function (this: E2EWorld) {
  const device = firstPlaywrightDevice(this);
  if (!device) return 'pending';

  const url = new URL(device.page.url());
  assert.ok(
    url.searchParams.has('session_token'),
    `Expected a session_token query param but URL is: ${url.href}`
  );
});

/**
 * The redirect URL preserves the OAuth client_id, redirect_uri, response_type,
 * and state when present.
 *
 * "when present" — each OAuth param is asserted only if the original
 * threshold-login navigation carried it (the doorway is the relying party, so
 * it round-trips whatever the originating client sent). With no inbound OAuth
 * params this is vacuously satisfied.
 *
 * Example:
 *   And the redirect URL preserves the OAuth client_id, redirect_uri,
 *     response_type, and state when present
 */
Then(
  'the redirect URL preserves the OAuth client_id, redirect_uri, response_type, and state when present',
  function (this: E2EWorld) {
    const device = firstPlaywrightDevice(this);
    if (!device) return 'pending';

    const redirectUrl = new URL(device.page.url());
    const inboundRaw = this.contentIds.get('oauthInboundParams');
    // No inbound OAuth params recorded for this run ⇒ "when present" is vacuous.
    if (!inboundRaw) return undefined;

    const inbound = new URLSearchParams(inboundRaw);
    for (const param of ['client_id', 'redirect_uri', 'response_type', 'state']) {
      const original = inbound.get(param);
      if (original === null) continue; // not present on the inbound request
      assert.strictEqual(
        redirectUrl.searchParams.get(param),
        original,
        `Expected redirect to preserve OAuth "${param}"="${original}", got ` +
          `"${redirectUrl.searchParams.get(param) ?? '(absent)'}"`
      );
    }
    return undefined;
  }
);

// ---------------------------------------------------------------------------
// Then — fall-through: doorway completes OAuth as relying-party (scenarios 3, 4)
// ---------------------------------------------------------------------------

/**
 * The browser completes the OAuth dance at the doorway as
 * relying-party-and-identity-provider (portal unreachable — scenario 3).
 *
 * When no portalHostUrl is handed back, the SPA does NOT navigate away to a
 * portal host; the OAuth dance finishes at the doorway origin. Asserted by the
 * browser remaining on the doorway origin (not the portal host). HTTP mode has
 * no browser ⇒ 'pending'.
 *
 * Example:
 *   And the browser completes the OAuth dance at the doorway as
 *     relying-party-and-identity-provider
 */
Then(
  'the browser completes the OAuth dance at the doorway as relying-party-and-identity-provider',
  async function (this: E2EWorld) {
    const device = firstPlaywrightDevice(this);
    if (!device) return 'pending';

    await device.page.waitForLoadState('networkidle');
    const doorway = requireDoorwayUrl(this);
    const current = new URL(device.page.url());
    const expectedOrigin = new URL(doorway).origin;
    assert.strictEqual(
      current.origin,
      expectedOrigin,
      `Expected the OAuth dance to finish at the doorway origin "${expectedOrigin}" ` +
        `(no portal hand-off) but the browser is at: ${current.href}`
    );
    return undefined;
  }
);

/**
 * The browser completes the OAuth dance at the doorway normally (hosted
 * visitor — scenario 4, @requires:shem). Same fall-through shape: the browser
 * stays at the doorway origin. Defined for binding cleanliness; held at runtime
 * by the substrate gate.
 *
 * Example:
 *   And the browser completes the OAuth dance at the doorway normally
 */
Then(
  'the browser completes the OAuth dance at the doorway normally',
  async function (this: E2EWorld) {
    const device = firstPlaywrightDevice(this);
    if (!device) return 'pending';

    await device.page.waitForLoadState('networkidle');
    const doorway = requireDoorwayUrl(this);
    const current = new URL(device.page.url());
    const expectedOrigin = new URL(doorway).origin;
    assert.strictEqual(
      current.origin,
      expectedOrigin,
      `Expected a hosted visitor's OAuth dance to finish at the doorway origin ` +
        `"${expectedOrigin}" but the browser is at: ${current.href}`
    );
    return undefined;
  }
);
