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
 *   - MemPalace wing=memory: project_m5_reframe_auth_portal_convergence (graduated 2026-06-02)
 *   - MemPalace wing=memory: project_peer_native_account_canonical_surface (graduated 2026-06-02)
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { DoorwayClient } from '../../src/framework/api/doorway-client.js';
import { BrowserDevice } from '../../src/framework/devices/browser-device.js';
import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { getFixture } from '../../src/framework/fixtures/humans.js';
import { Human } from '../../src/framework/human.js';
import { ThresholdLoginPage } from '../../src/framework/pages/index.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// contentIds keys — captured login response + registered portal host
// ---------------------------------------------------------------------------

const LOGIN_STATUS_KEY = 'portalHandoffLoginStatus';
const LOGIN_BODY_KEY = 'portalHandoffLoginBody';
const REGISTERED_HOST_KEY = 'portalHostUrl'; // shared key with account-m5 redirect steps

/**
 * Captured URL of the client-driven navigation toward the portal host.
 *
 * The portal host (`https://matthew.steward.example/…`) is a fixture origin with
 * no real server, so the attempted navigation can never resolve. In Playwright
 * mode the submit step installs a `page.route` interceptor that ABORTS the
 * portal-host navigation and records the attempted URL here, so the redirect
 * assertions can observe the client's intent without hanging on an unresolvable
 * load. See the submit step and the "browser is redirected" Then step.
 */
const REDIRECT_ATTEMPT_KEY = 'portalHandoffRedirectAttemptUrl';

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

/**
 * Resolve a bearer token from the first human's first token-bearing device.
 * The dev-mode portal-health override route is hard-gated by dev_mode (no admin
 * auth required), but we pass a token when one exists for forward-compatibility.
 */
function firstAuthToken(world: E2EWorld): string | undefined {
  for (const [, human] of world.humans) {
    for (const device of human.devices) {
      const anyDevice = device as { token?: string; client?: { token?: string } };
      if (anyDevice.token) return anyDevice.token;
      if (anyDevice.client?.token) return anyDevice.client.token;
    }
  }
  return undefined;
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

/**
 * Raw PUT that captures status + body without throwing on non-2xx — mirrors
 * rawPost above. Used by the steward grant so a FIXTURE_ONLY/dev_mode rejection
 * surfaces a clear diagnostic rather than an opaque undici throw.
 */
async function rawPut(
  baseUrl: string,
  path: string,
  payload: unknown,
  token?: string
): Promise<{ statusCode: number; body: string }> {
  const headers: Record<string, string> = { 'content-type': 'application/json' };
  if (token) headers['authorization'] = `Bearer ${token}`;
  const { statusCode, body } = await request(`${baseUrl}${path}`, {
    method: 'PUT',
    headers,
    body: JSON.stringify(payload),
  });
  const text = await body.text();
  return { statusCode, body: text };
}

/**
 * Force-grant graduated-steward status to a fixture human via the dev/fixture
 * admin surface (PUT /admin/users/{id}/steward).
 *
 * Admin auth: the steward-login fixtures are staged by Matthew, who is the a2o
 * admin fixture (humans.ts: ADMIN_HUMAN_ID = 'human-matthew-manager',
 * isAdmin: true). His login token is therefore an admin bearer — the same
 * pattern user-management.steps.ts uses for adminListUsers / adminSetUserStatus.
 *
 * The grant endpoint keys on the Mongo `_id` (handle_grant_steward does
 * ObjectId::parse_str), NOT the Holochain humanId — so we resolve the id from
 * the admin users list by identifier, exactly as the suspend/view-details steps
 * do. The grant is REQUIRED: a non-2xx throws, naming the most likely cause
 * (the FIXTURE_ONLY dev-mode gate) so the failure is self-explanatory.
 */
async function grantStewardFixture(
  doorway: string,
  adminToken: string,
  identifier: string
): Promise<void> {
  // Resolve the Mongo _id from the admin users list (keyed by identifier).
  const adminClient = new DoorwayClient(doorway);
  adminClient.setToken(adminToken);
  const list = await adminClient.adminListUsers({ search: identifier, limit: 50 });
  const entry = list.users.find(u => u.identifier === identifier);
  if (!entry) {
    throw new Error(
      `Cannot grant stewardship: user "${identifier}" not found in the admin users list. ` +
        'Is the fixture human seeded on this doorway?'
    );
  }

  const { statusCode, body } = await rawPut(
    doorway,
    `/admin/users/${encodeURIComponent(entry.id)}/steward`,
    {},
    adminToken
  );

  if (statusCode < 200 || statusCode >= 300) {
    throw new Error(
      `Steward grant for "${identifier}" (id=${entry.id}) failed: PUT /admin/users/${entry.id}/steward ` +
        `→ ${statusCode}: ${body}. ` +
        'A 403 with code "FIXTURE_ONLY" means the doorway is NOT in dev mode — this ' +
        'fixture-only grant surface is hard-gated off unless dev_mode is set.'
    );
  }
}

// ---------------------------------------------------------------------------
// Given — fixture setup
// ---------------------------------------------------------------------------

/**
 * Matthew is a graduated steward whose peer-native portal host is registered.
 *
 * Extends the fixture-humans Matthew toward the graduated-steward-with-portal
 * variant: log in via HTTP (BrowserDevice) for the bearer token, register the
 * portal host through the canonical /api/v1/account/portal-hosts surface, then
 * GRANT graduated-steward status via the dev/fixture admin surface so the
 * /auth/login response actually carries isSteward:true + portalHostUrl.
 *
 * Admin auth: Matthew IS the a2o admin fixture (humans.ts ADMIN_HUMAN_ID,
 * isAdmin:true), and he is also the grant target here — so his own login token
 * is the admin bearer for the grant. The grant is REQUIRED (not best-effort): a
 * failure throws, naming the FIXTURE_ONLY/dev_mode gate as the likely cause.
 *
 * Playwright mode: a BrowserDevice (HTTP, for the bearer token + grant) is
 * always attached; when deviceMode === 'playwright' a PlaywrightDevice is ALSO
 * attached so firstPlaywrightDevice() resolves and the submit/redirect steps can
 * drive and observe the real client-driven navigation.
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
    // a non-2xx here does not fail setup — the host URL is already recorded
    // above, and the grant below is what makes the response carry portalHostUrl).
    await rawPost(
      doorway,
      '/api/v1/account/portal-hosts',
      { hostUrl: portalHostUrl, label: `${humanName}'s steward portal` },
      device.token
    );

    // Grant graduated-steward status (REQUIRED). Matthew's own token is the
    // admin bearer; the grant keys on the Mongo _id resolved from the users
    // list by identifier (see grantStewardFixture). Throws on failure.
    const adminToken = device.token;
    if (!adminToken) {
      throw new Error(`${humanName} has no bearer token after login — cannot grant stewardship.`);
    }
    await grantStewardFixture(doorway, adminToken, fixture.credentials.identifier);

    // In Playwright mode, ALSO attach a real browser device so
    // firstPlaywrightDevice() finds one and the submit/redirect steps can drive
    // the client-driven navigation. Mirrors fixture-humans.steps.ts.
    if (this.deviceMode === 'playwright') {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      const browser = await this.getBrowser();
      const appUrl = doorwayToAppUrl(doorway);
      // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
      const pwDevice = new PlaywrightDevice(`${humanName}-pw`, appUrl, doorway, browser);
      await pwDevice.init();
      human.addDevice(pwDevice);
      await pwDevice.login({
        identifier: fixture.credentials.identifier,
        password: fixture.credentials.password,
      });
      this.onCleanup(async () => pwDevice.close());
    }
  }
);

/**
 * Portal host is unreachable — doorway falls through to local auth.
 *
 * Note the wording "the portal host" (account-m5 uses "my portal host"); both
 * record the same unreachable precondition on world state.
 *
 * This runs in scenario 3 AFTER the Background's "responds to /healthz with 200"
 * step (which set the override to healthy=true), so it must EXPLICITLY override
 * the override back to healthy=false — the probe then skips the host and the
 * /auth/login response omits portalHostUrl. We also keep the legacy world flag.
 * The PUT is best-effort here: even if the override surface is unreachable the
 * non-resolving `.example` host's live HEAD still fails, so the fall-through
 * scenario stays correct either way; but a hard-failure FIXTURE_ONLY response is
 * still surfaced loudly to avoid a silent mis-set.
 *
 * Example:
 *   Given the portal host does not respond to /healthz
 */
Given(String.raw`the portal host does not respond to \/healthz`, async function (this: E2EWorld) {
  this.contentIds.set('portalHostUnreachable', 'true');

  const doorway = requireDoorwayUrl(this);
  const hostUrl = this.contentIds.get(REGISTERED_HOST_KEY);
  if (!hostUrl) return; // No host recorded ⇒ nothing to override; world flag stands.

  const { statusCode, body } = await rawPut(
    doorway,
    '/admin/dev/portal-health',
    { hostUrl, healthy: false },
    firstAuthToken(this)
  );
  if (statusCode === 403) {
    throw new Error(
      `Failed to set portal-health override (unhealthy) for "${hostUrl}": ` +
        `PUT /admin/dev/portal-health → 403: ${body}. A 403 with code ` +
        '"FIXTURE_ONLY" means the doorway is NOT in dev mode — this override ' +
        'surface is hard-gated off unless dev_mode is set.'
    );
  }
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

    // The portal host is a fixture origin with no real server, so the
    // client-driven navigation toward it can never resolve. Intercept it: abort
    // the request and CAPTURE its URL so the redirect assertions can observe the
    // client's intent without hanging on an unresolvable load. Registered BEFORE
    // the login navigation so the very first portal-host request is caught.
    const portalHost = this.contentIds.get(REGISTERED_HOST_KEY);
    if (portalHost) {
      const portalOrigin = new URL(portalHost).origin;
      await device.page.route(`${portalOrigin}/**`, async route => {
        // Record the full attempted URL (path + query) for the Then steps.
        this.contentIds.set(REDIRECT_ATTEMPT_KEY, route.request().url());
        await route.abort();
      });
    }

    await device.navigate('/threshold/login');
    const loginPage = new ThresholdLoginPage(device.page);
    await loginPage.login(fixture.credentials.identifier, fixture.credentials.password);
    // The client-driven navigation toward the portal host is intercepted+aborted
    // above; allow the doorway-side state to settle. networkidle may not fire if
    // the abort leaves the page mid-navigation, so cap the wait and swallow a
    // timeout — the captured-URL assertions are the source of truth, not load.
    await device.page.waitForLoadState('networkidle', { timeout: 5_000 }).catch(() => undefined);
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
 * Wait for the intercepted portal-host navigation URL to be captured.
 *
 * The submit step installs a `page.route` interceptor that aborts the
 * client-driven navigation toward the (server-less) portal host and records the
 * attempted URL in REDIRECT_ATTEMPT_KEY. Because the request is aborted, the
 * page URL never changes — so the redirect assertions read this captured URL,
 * not `page.url()`. Polls briefly to absorb the gap between form submit and the
 * SPA issuing the navigation.
 */
async function waitForRedirectAttemptUrl(world: E2EWorld): Promise<string> {
  const deadline = Date.now() + 15_000;
  for (;;) {
    const captured = world.contentIds.get(REDIRECT_ATTEMPT_KEY);
    if (captured) return captured;
    if (Date.now() >= deadline) {
      throw new Error(
        'No client-driven navigation toward the portal host was observed within 15s. ' +
          'Expected the SPA to read portalHostUrl from /auth/login and navigate; the ' +
          'page.route interceptor never fired (REDIRECT_ATTEMPT_KEY unset).'
      );
    }
    await new Promise(resolve => setTimeout(resolve, 100));
  }
}

/**
 * The browser is redirected to the portal host.
 *
 * CLIENT-driven: the SPA reads portalHostUrl from the JSON and navigates —
 * asserted against the INTERCEPTED navigation URL (captured + aborted by the
 * submit step), NOT an HTTP 302 and NOT `page.url()` (the abort means the page
 * never actually loads the server-less portal host). HTTP mode has no browser to
 * observe, so this returns 'pending'.
 *
 * Example:
 *   Then the browser is redirected to "https://matthew.steward.example/account"
 */
Then(
  'the browser is redirected to {string}',
  async function (this: E2EWorld, portalHostUrl: string) {
    const device = firstPlaywrightDevice(this);
    if (!device) return 'pending';

    const attempted = await waitForRedirectAttemptUrl(this);
    assert.ok(
      attempted.startsWith(portalHostUrl),
      `Expected client-driven redirect to "${portalHostUrl}" but the SPA navigated to: ${attempted}`
    );
    return undefined;
  }
);

/**
 * The redirect URL carries a session_token query parameter.
 *
 * Reads the intercepted navigation URL (see waitForRedirectAttemptUrl), not
 * `page.url()` — the portal-host navigation is aborted, so the token only ever
 * appears on the attempted request URL.
 *
 * Example:
 *   And the redirect URL carries a session_token query parameter
 */
Then('the redirect URL carries a session_token query parameter', async function (this: E2EWorld) {
  const device = firstPlaywrightDevice(this);
  if (!device) return 'pending';

  const url = new URL(await waitForRedirectAttemptUrl(this));
  assert.ok(
    url.searchParams.has('session_token'),
    `Expected a session_token query param but redirect URL is: ${url.href}`
  );
  return undefined;
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
  async function (this: E2EWorld) {
    const device = firstPlaywrightDevice(this);
    if (!device) return 'pending';

    const redirectUrl = new URL(await waitForRedirectAttemptUrl(this));
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
 * The genuine fall-through invariant is **no portal hand-off**: when no
 * portalHostUrl is handed back, the SPA does NOT navigate to the portal host —
 * the doorway completes auth as the IdP. This is the clean inverse of
 * "the browser is redirected to {portal}": the page.route interceptor armed in
 * the When step (on the portal origin) must NEVER fire, so REDIRECT_ATTEMPT_KEY
 * stays unset.
 *
 * ORIGIN-AGNOSTIC by design. The doorway threshold/login IdP surface is served
 * on BOTH the doorway origin (doorway-alpha.elohim.host) AND the app origin
 * (alpha.elohim.host) — the doorway forwards /threshold/* to doorway-app
 * (routes/threshold.rs) and the app ingress mirrors it (both return the Doorway
 * Operator Dashboard). The PlaywrightDevice base is appUrl, so the browser sits
 * on the app origin throughout; a brittle `origin === doorwayOrigin` check tests
 * the device base, not the product, and can never pass for this app-initiated
 * flow. "Doorway completes auth as IdP without delegating to a portal" is
 * faithfully captured as "no portal-host navigation was attempted." HTTP mode
 * has no browser ⇒ 'pending'.
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

    // Let the page settle so a (wrongly) attempted portal nav has registered via
    // the interceptor before we assert its ABSENCE. The When step already armed
    // the interceptor and waited networkidle; this is a short guard for the
    // abort-mid-navigation case where networkidle may not have fired.
    await device.page.waitForLoadState('networkidle', { timeout: 5_000 }).catch(() => undefined);

    const attempted = this.contentIds.get(REDIRECT_ATTEMPT_KEY);
    assert.ok(
      !attempted,
      `Expected NO portal hand-off — doorway completes auth as relying-party-and-identity-provider — ` +
        `but the SPA attempted a client-driven navigation to the portal host: ${attempted}`
    );

    // And the browser must not have landed on the portal host origin itself.
    const portalHost = this.contentIds.get(REGISTERED_HOST_KEY);
    if (portalHost) {
      const current = new URL(device.page.url());
      const portalOrigin = new URL(portalHost).origin;
      assert.notStrictEqual(
        current.origin,
        portalOrigin,
        `Browser landed on the portal host origin "${portalOrigin}"; expected doorway-local ` +
          `completion (no hand-off). Current: ${current.href}`
      );
    }
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
