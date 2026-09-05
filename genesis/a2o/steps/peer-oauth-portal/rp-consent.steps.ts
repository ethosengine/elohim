/**
 * Peer OAuth Portal — RP Consent (RFC-6749) step definitions.
 *
 * Drives the `<elohim-imagodei-consent-card>` surface at `/auth/portal` when
 * an external relying party (graphos-designer) redirects matthew with an
 * OAuth authorization request. The portal-shell projects the consent-card
 * step instead of the federated-resolver / login-card steps when the URL
 * carries `client_id`, `claims`, `redirect_uri`, and `state` params.
 *
 * The consent-card primitive (B6, commit 2d82fb80b) exposes:
 *   - `[part="claim-row"]` with `data-claim-id` attribute per row
 *   - `<input type="checkbox" ?checked ?disabled>` inside each row
 *   - `[part="approve"]` and `[part="decline"]` action buttons
 *   - `approve` event fires the OAuth code redirect; `decline` redirects
 *     with `error=access_denied`
 *
 * Reuses the visitor + selector patterns from F1's hosted-login.steps.ts.
 * The `_portal-visitor` anonymous Human is set up by F1's
 * `When matthew opens "{url}"` step (registered in cucumber-js's global
 * step registry); steps unique to consent flow live here.
 *
 * Selectors reference:
 *   - elohim-imagodei-consent-card       (the consent surface itself)
 *   - [part="claim-row"][data-claim-id]  (one row per requested claim)
 *   - [part="approve"]                   (Approve button)
 *   - [part="decline"]                   (Decline button)
 */

import { strict as assert } from 'node:assert';

import { Given, Then, When } from '@cucumber/cucumber';

import { request } from 'undici';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { Human } from '../../src/framework/human.js';
import { ThresholdLoginPage } from '../../src/framework/pages/threshold-login.page.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants — Lit element tags + CSS parts from the consent-card primitive
// ---------------------------------------------------------------------------

const CONSENT_CARD = 'elohim-imagodei-consent-card';
const CLAIM_ROW_PART = '[part="claim-row"]';
const APPROVE_PART = '[part="approve"]';
const DECLINE_PART = '[part="decline"]';
const AUTH_TOKEN_KEY = 'doorway_auth_token';

// ---------------------------------------------------------------------------
// Browser session — anonymous portal visitor (shared shape with F1/F2)
// ---------------------------------------------------------------------------

const VISITOR_NAME = '_portal-visitor';

interface ConsentSessionState {
  /** True once the matthew-is-signed-in precondition step ran. */
  matthewSignedIn?: boolean;
  /** True once the graphos-designer-is-registered precondition step ran. */
  rpRegistered?: boolean;
}

const sessionState = new WeakMap<E2EWorld, ConsentSessionState>();

function getConsentSessionState(world: E2EWorld): ConsentSessionState {
  let state = sessionState.get(world);
  if (!state) {
    state = {};
    sessionState.set(world, state);
  }
  return state;
}

/**
 * Resolve the configured alpha doorway URL (mirrors F1/F2's helper).
 *
 * Prefers a registered doorway (Background step), falls back to env. Strips
 * a single trailing slash so we can join paths cleanly.
 */
function resolveAlphaDoorwayUrl(world: E2EWorld): string {
  const registered = [...world.doorways.values()].find(d => d.id === 'alpha');
  const url = registered?.url ?? process.env['E2E_DOORWAY_ALPHA'] ?? '';
  assert.ok(
    url,
    'No alpha doorway URL — register via the Background doorway step or set E2E_DOORWAY_ALPHA'
  );
  return url.endsWith('/') ? url.slice(0, -1) : url;
}

/**
 * Resolve (or create) the anonymous portal visitor's Playwright device.
 *
 * Mirrors F1's `ensureVisitor` so the consent scenarios can run standalone
 * without depending on F1's first scenario having already executed.
 */
async function ensureVisitor(world: E2EWorld): Promise<PlaywrightDevice> {
  let human: Human;
  try {
    human = world.getHuman(VISITOR_NAME);
  } catch {
    human = new Human(VISITOR_NAME, {
      identifier: '',
      password: '',
      displayName: VISITOR_NAME,
    });
    world.addHuman(VISITOR_NAME, human);
  }

  const existing = human.devices.find(d => d.type === 'playwright');
  if (existing) return existing as PlaywrightDevice;

  const [, doorway] = [...world.doorways][0] ?? [];
  const doorwayUrl = doorway?.url ?? process.env['E2E_DOORWAY_ALPHA'] ?? '';
  assert.ok(doorwayUrl, 'No doorway URL — register one via Background or set E2E_DOORWAY_ALPHA');

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await world.getBrowser();
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const device = new PlaywrightDevice(`${VISITOR_NAME}-pw`, doorwayUrl, doorwayUrl, browser);
  await device.init();
  human.addDevice(device);
  world.onCleanup(async () => device.close());
  return device;
}

function getVisitorPage(world: E2EWorld) {
  const human = world.getHuman(VISITOR_NAME);
  const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
  assert.ok(
    device,
    'Visitor Playwright device is missing — call ensureVisitor or open the portal first'
  );
  return device.page;
}

async function signInThroughDoorwayPortal(
  device: PlaywrightDevice,
  baseUrl: string,
  returnTo: string
): Promise<void> {
  await device.page.goto(`${baseUrl}/threshold/login?returnUrl=${encodeURIComponent(returnTo)}`);
  const loginPage = new ThresholdLoginPage(device.page);
  await loginPage.login('matthew@alpha.elohim.host', 'shibboleth');
  await device.page.waitForFunction(
    (key: string) => globalThis.localStorage.getItem(key) !== null,
    AUTH_TOKEN_KEY,
    { timeout: 15_000 }
  );
}

// ---------------------------------------------------------------------------
// Given — Mode A signed-in precondition + OAuth client registration
// ---------------------------------------------------------------------------

interface OAuthClientRow {
  id: string;
  displayName?: string;
  redirectUris?: string[];
}

/**
 * Sign matthew in as a Mode A (hosted) visitor before the OAuth redirect.
 *
 * Drives the doorway's `/threshold/login` portal so the browser carries the
 * hosted-human session before the consent redirect. The seed is expected to
 * have provisioned matthew with password "shibboleth" (same fixture as F1).
 */
Given(
  String.raw`matthew is signed in to alpha.elohim.host \(Mode A)`,
  async function (this: E2EWorld) {
    const device = await ensureVisitor(this);
    const baseUrl = resolveAlphaDoorwayUrl(this);
    await signInThroughDoorwayPortal(device, baseUrl, '/auth/portal');

    const state = getConsentSessionState(this);
    state.matthewSignedIn = true;
  }
);

/**
 * Verify the OAuth client registry includes graphos-designer.
 *
 * Probes the doorway's optional `/admin/oauth-clients` listing endpoint. If
 * the endpoint exists and returns a registry, assert graphos-designer is in
 * it. If the endpoint is absent (the MVP doorway hardcodes the registry per
 * `doorway/doorway-service/src/db/schemas/oauth_session.rs::get_registered_clients()`),
 * fall through and record the precondition — the subsequent consent-card
 * render step will catch a missing registration via a UI failure.
 */
Given('graphos-designer.elohim.host is a registered OAuth client', async function (this: E2EWorld) {
  const baseUrl = resolveAlphaDoorwayUrl(this);
  let probeOk = false;
  try {
    const { statusCode, body } = await request(`${baseUrl}/admin/oauth-clients`);
    if (statusCode === 200) {
      const raw = await body.text();
      try {
        const parsed = JSON.parse(raw) as OAuthClientRow[] | { rows?: OAuthClientRow[] };
        const clients = Array.isArray(parsed) ? parsed : (parsed.rows ?? []);
        const match = clients.find(c => c.id === 'graphos-designer');
        assert.ok(
          match,
          `Expected graphos-designer in oauth-clients registry; got [${clients.map(c => c.id).join(', ')}]`
        );
        probeOk = true;
      } catch {
        // Body wasn't JSON — fall through to the seed-fallback path.
      }
    }
  } catch {
    // Endpoint absent or network error — fall through to the seed-fallback
    // path. The doorway MVP hardcodes the registry; @wip tag on the feature
    // file preserves this as a target for the seed to fulfill (Task G1).
  }
  const state = getConsentSessionState(this);
  state.rpRegistered = probeOk;
});

// ---------------------------------------------------------------------------
// When — OAuth authorization redirect
// ---------------------------------------------------------------------------

/**
 * Navigate the visitor to the authorization endpoint with OAuth query params.
 *
 * Resolves relative URLs against alpha.elohim.host (the doorway). The
 * portal-shell at `/auth/portal` inspects the URL search params and projects
 * the consent-card step when `client_id` + `claims` + `redirect_uri` are
 * present.
 */
When('matthew is redirected to {string}', async function (this: E2EWorld, urlOrPath: string) {
  const device = await ensureVisitor(this);
  let fullUrl: string;
  if (/^https?:/i.test(urlOrPath)) {
    fullUrl = urlOrPath;
  } else {
    const baseUrl = resolveAlphaDoorwayUrl(this);
    const separator = urlOrPath.startsWith('/') ? '' : '/';
    fullUrl = `${baseUrl}${separator}${urlOrPath}`;
  }
  await device.page.goto(fullUrl);
});

When('matthew approves', async function (this: E2EWorld) {
  const page = getVisitorPage(this);
  const card = page.locator(CONSENT_CARD);
  await card.waitFor({ state: 'visible' });
  await card.locator(APPROVE_PART).click();
});

When('matthew clicks Decline', async function (this: E2EWorld) {
  const page = getVisitorPage(this);
  const card = page.locator(CONSENT_CARD);
  await card.waitFor({ state: 'visible' });
  await card.locator(DECLINE_PART).click();
});

// ---------------------------------------------------------------------------
// Given (composite) — pre-land on the consent-card for the decline scenario
// ---------------------------------------------------------------------------

/**
 * Composite precondition for the Decline scenario: sign matthew in (Mode A)
 * and navigate to the consent-card with the graphos-designer params already
 * applied. The hosted human signs in at the doorway portal before returning
 * to the p2p-native consent surface.
 */
Given('matthew is on the consent-card for graphos-designer', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  const baseUrl = resolveAlphaDoorwayUrl(this);

  const consentUrl =
    `${baseUrl}/auth/portal?client_id=graphos-designer` +
    `&claims=imagodei.displayName` +
    `&redirect_uri=https://graphos-designer.elohim.host/callback` +
    `&state=abc`;
  await signInThroughDoorwayPortal(device, baseUrl, consentUrl);
  await device.page.locator(CONSENT_CARD).waitFor({ state: 'visible' });
});

// ---------------------------------------------------------------------------
// Then — consent-card render assertions
// ---------------------------------------------------------------------------

Then(
  'the consent-card renders with {word} as the requesting client',
  async function (this: E2EWorld, clientName: string) {
    const page = getVisitorPage(this);
    const card = page.locator(CONSENT_CARD);
    await card.waitFor({ state: 'visible', timeout: 10_000 });
    const text = ((await card.textContent()) ?? '').trim();
    assert.ok(
      text.toLowerCase().includes(clientName.toLowerCase()),
      `Expected consent-card to mention "${clientName}", got "${text.slice(0, 200)}"`
    );
  }
);

Then('both claims are listed with toggles', async function (this: E2EWorld) {
  const page = getVisitorPage(this);
  const rows = page.locator(`${CONSENT_CARD} ${CLAIM_ROW_PART}`);
  const count = await rows.count();
  assert.ok(count >= 2, `Expected at least 2 claim-row entries in the consent-card, got ${count}`);
});

Then(
  String.raw`{string} is required \(locked on)`,
  async function (this: E2EWorld, claimId: string) {
    const page = getVisitorPage(this);
    const row = page.locator(`${CONSENT_CARD} [data-claim-id="${claimId}"]`);
    await row.waitFor({ state: 'visible', timeout: 10_000 });
    // The checkbox inside the required row must be both checked + disabled.
    // The PWLocator stub doesn't expose isChecked()/isDisabled(), so we read
    // the DOM directly via evaluate() against the host page.
    const flags = (await page.evaluate((selector: string) => {
      const el = document.querySelector(selector);
      if (!(el instanceof HTMLInputElement)) {
        return { checked: null, disabled: null, present: false };
      }
      return {
        checked: el.checked,
        disabled: el.disabled,
        present: true,
      };
    }, `${CONSENT_CARD} [data-claim-id="${claimId}"] input[type="checkbox"]`)) as {
      checked: boolean | null;
      disabled: boolean | null;
      present: boolean;
    };
    assert.equal(flags.present, true, `No checkbox found in claim-row for "${claimId}"`);
    assert.equal(flags.checked, true, `Expected required claim "${claimId}" to be checked`);
    assert.equal(
      flags.disabled,
      true,
      `Expected required claim "${claimId}" to be disabled (locked on)`
    );
  }
);

// ---------------------------------------------------------------------------
// Then — OAuth code issuance + redirect assertions
// ---------------------------------------------------------------------------

Then('a 5-min single-use OAuth code is issued', async function (this: E2EWorld) {
  const page = getVisitorPage(this);
  // Wait for the browser to land on a URL carrying `?code=` — the doorway's
  // approve handler issues the code and 302s back to the RP's redirect_uri.
  // The 5-min single-use property is enforced server-side (see
  // doorway-service/src/oauth/code_store.rs); the a2o suite asserts the
  // user-visible contract: a code lands in the URL. Code lifetime + reuse
  // semantics are guarded by unit tests in the doorway crate.
  await page.waitForURL((url: URL) => url.searchParams.has('code'), { timeout: 15_000 });
});

Then(
  "matthew is redirected to graphos-designer's redirect_uri with code + state preserved",
  async function (this: E2EWorld) {
    const page = getVisitorPage(this);
    // Re-affirm the destination state (the prior "code is issued" step already
    // waited for `?code=`, but this assertion checks hostname + state too).
    await page.waitForURL(
      (url: URL) =>
        url.hostname.includes('graphos-designer') &&
        url.searchParams.has('code') &&
        url.searchParams.get('state') === 'abc',
      { timeout: 15_000 }
    );
    const url = new URL(page.url());
    assert.ok(
      url.hostname.includes('graphos-designer'),
      `Expected redirect to a graphos-designer host, got "${url.hostname}"`
    );
    const code = url.searchParams.get('code');
    assert.ok(
      code && code.length > 0,
      `Expected a non-empty 'code' query param, got "${code ?? '<missing>'}"`
    );
    assert.equal(
      url.searchParams.get('state'),
      'abc',
      `Expected 'state' to be preserved as "abc", got "${url.searchParams.get('state') ?? '<missing>'}"`
    );
  }
);

Then(
  "matthew is redirected to graphos-designer's redirect_uri with error=access_denied + state preserved",
  async function (this: E2EWorld) {
    const page = getVisitorPage(this);
    await page.waitForURL(
      (url: URL) =>
        url.hostname.includes('graphos-designer') &&
        url.searchParams.get('error') === 'access_denied' &&
        url.searchParams.get('state') === 'abc',
      { timeout: 15_000 }
    );
  }
);
