/**
 * Peer OAuth Portal — Hosted Login (Mode A) step definitions.
 *
 * Drives the `<elohim-imagodei-portal-shell>` chrome at `/auth/portal` via
 * Playwright. The host doorway projects the portal EPR (`imagodei-portal`)
 * at this URL path; the shell hosts a federated-resolver step that advances
 * to a login-card step once the user submits an identifier.
 *
 * The Lit primitives expose their semantic surface via custom-element tag
 * names — these steps locate the elements by tag (no data-testid coverage)
 * and interact with the slotted/shadow inputs through Playwright's selector
 * piercing. See `src/framework/pages/selectors.ts::LOGIN` for the canonical
 * tag map.
 *
 * Selectors reference:
 *   - elohim-imagodei-portal-shell          (host shell, [step] reflects state)
 *   - elohim-imagodei-federated-resolver    (step=resolve)
 *   - elohim-imagodei-login-card            (step=login)
 *   - elohim-imagodei-trust-indicator       (chrome at top of shell)
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { request } from 'undici';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { Human } from '../../src/framework/human.js';
import { LOGIN } from '../../src/framework/pages/selectors.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants — Lit element tags from the portal-shell composition
// ---------------------------------------------------------------------------

const PORTAL_SHELL = LOGIN.PORTAL_SHELL;
const FEDERATED_RESOLVER = LOGIN.FEDERATED_RESOLVER;
const LOGIN_CARD = LOGIN.LOGIN_CARD;
const TRUST_INDICATOR = 'elohim-imagodei-trust-indicator';
const FLYWHEEL_HINT_PART = '[part="flywheel-hint"]';
const LOGIN_ERROR_PART = '[part="error"]';

// ---------------------------------------------------------------------------
// Browser session — anonymous portal visitor (no fixture human attached)
// ---------------------------------------------------------------------------

const VISITOR_NAME = '_portal-visitor';

interface PortalSessionState {
  identifier?: string;
  password?: string;
  expectedHumanId?: string;
}

const sessionState = new WeakMap<E2EWorld, PortalSessionState>();

function getSessionState(world: E2EWorld): PortalSessionState {
  let state = sessionState.get(world);
  if (!state) {
    state = {};
    sessionState.set(world, state);
  }
  return state;
}

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

  // Resolve a doorway URL — prefer one registered by the Background step,
  // fall back to E2E_DOORWAY_ALPHA so this scenario can run standalone.
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
  assert.ok(device, 'Visitor Playwright device is missing — call ensureVisitor first');
  return device.page;
}

/**
 * Resolve the configured alpha doorway URL.
 *
 * Prefers a registered doorway (Background step), falls back to env. Strips
 * trailing slashes so we can join paths cleanly.
 */
function resolveAlphaDoorwayUrl(world: E2EWorld): string {
  const registered = [...world.doorways.values()].find(d => d.id === 'alpha');
  const url = registered?.url ?? process.env['E2E_DOORWAY_ALPHA'] ?? '';
  assert.ok(
    url,
    'No alpha doorway URL — register via the Background doorway step or set E2E_DOORWAY_ALPHA'
  );
  // Strip a single trailing slash (no quantifier — keeps the regex linear).
  return url.endsWith('/') ? url.slice(0, -1) : url;
}

// ---------------------------------------------------------------------------
// Given — doorway projection precondition
// ---------------------------------------------------------------------------

interface ProjectionRow {
  urlPath?: string;
  eprId?: string;
}

/**
 * Verify a projection for the imagodei-portal EPR exists at the given path.
 *
 * The doorway exposes its rea_commitments projections at
 * `/db/rea_commitments?action=project-epr&doorwayId=<id>`. We list and
 * search for a row matching the requested urlPath + portal EPR id.
 */
Given(
  'the alpha.elohim.host doorway has a projection for the peer-oauth-portal at {string}',
  async function (this: E2EWorld, path: string) {
    const baseUrl = resolveAlphaDoorwayUrl(this);
    const { statusCode, body } = await request(
      `${baseUrl}/db/rea_commitments?action=project-epr&doorwayId=alpha-elohim-host`
    );
    assert.equal(
      statusCode,
      200,
      `Expected 200 listing rea_commitments projections, got ${statusCode}`
    );
    const raw = await body.text();
    let projections: ProjectionRow[];
    try {
      const parsed = JSON.parse(raw) as ProjectionRow[] | { rows?: ProjectionRow[] };
      projections = Array.isArray(parsed) ? parsed : (parsed.rows ?? []);
    } catch (err) {
      assert.fail(`rea_commitments projection list was not JSON: ${(err as Error).message}`);
    }
    const portalProj = projections.find(p => p.urlPath === path && p.eprId === 'imagodei-portal');
    assert.ok(
      portalProj,
      `No imagodei-portal projection found at "${path}". Projections returned: ` +
        JSON.stringify(projections.slice(0, 5))
    );
  }
);

/**
 * Record fixture credentials for matthew. The scenario assumes the seed-data
 * pipeline has provisioned matthew@alpha.elohim.host on the doorway; this
 * step captures the password for the later submit step.
 */
Given(
  'matthew is a pre-registered imagodei on alpha.elohim.host with password {string}',
  function (this: E2EWorld, password: string) {
    const state = getSessionState(this);
    state.identifier = 'matthew@alpha.elohim.host';
    state.password = password;
  }
);

/**
 * Composite state-setup step for the second scenario: navigate to the
 * portal, fill the federated-resolver, and wait for the login-card to
 * appear. Mirrors the first scenario's prefix so scenarios can be ordered
 * independently.
 */
Given('matthew is on the login-card step at alpha.elohim.host', async function (this: E2EWorld) {
  const device = await ensureVisitor(this);
  const baseUrl = resolveAlphaDoorwayUrl(this);
  await device.page.goto(`${baseUrl}/auth/portal`);

  const resolver = device.page.locator(FEDERATED_RESOLVER);
  await resolver.waitFor({ state: 'visible' });
  await resolver.locator('input').fill('matthew@alpha.elohim.host');
  await resolver.locator('button[type="submit"]').click();

  await device.page.locator(LOGIN_CARD).waitFor({ state: 'visible' });
});

// ---------------------------------------------------------------------------
// When — visitor interactions
// ---------------------------------------------------------------------------

When('matthew opens {string}', async function (this: E2EWorld, url: string) {
  const device = await ensureVisitor(this);
  await device.page.goto(url);
  await device.page.locator(PORTAL_SHELL).waitFor({ state: 'visible' });
});

When(
  'types {string} into the federated-resolver',
  async function (this: E2EWorld, identifier: string) {
    const page = getVisitorPage(this);
    const resolver = page.locator(FEDERATED_RESOLVER);
    await resolver.waitFor({ state: 'visible' });
    await resolver.locator('input').fill(identifier);
    await resolver.locator('button[type="submit"]').click();
  }
);

When('matthew submits the password {string}', async function (this: E2EWorld, password: string) {
  const page = getVisitorPage(this);
  const card = page.locator(LOGIN_CARD);
  await card.waitFor({ state: 'visible' });
  await card.locator('input[type="password"]').fill(password);
  await card.locator('button[type="submit"]').click();
});

When('matthew submits an incorrect password', async function (this: E2EWorld) {
  const page = getVisitorPage(this);
  const card = page.locator(LOGIN_CARD);
  await card.waitFor({ state: 'visible' });
  await card.locator('input[type="password"]').fill('not-the-right-password-99999');
  await card.locator('button[type="submit"]').click();
});

// ---------------------------------------------------------------------------
// Then — assertions
// ---------------------------------------------------------------------------

Then('the portal advances to the login-card step', async function (this: E2EWorld) {
  const page = getVisitorPage(this);
  await page.locator(LOGIN_CARD).waitFor({ state: 'visible', timeout: 10_000 });
});

Then(
  'the trust-indicator reads {string} with the flywheel hint visible',
  async function (this: E2EWorld, label: string) {
    const page = getVisitorPage(this);
    const indicator = page.locator(TRUST_INDICATOR);
    await indicator.waitFor({ state: 'visible' });
    const text = ((await indicator.textContent()) ?? '').trim();
    assert.ok(
      text.includes(label),
      `Expected trust-indicator to contain "${label}", got "${text}"`
    );
    const hintCount = await indicator.locator(FLYWHEEL_HINT_PART).count();
    assert.equal(hintCount, 1, `Expected 1 flywheel-hint, found ${hintCount}`);
  }
);

interface CookieRow {
  name: string;
  value: string;
}

Then('the doorway sets the elohim_session cookie', async function (this: E2EWorld) {
  const page = getVisitorPage(this);
  // Read cookies via document.cookie — the Playwright type stub here does
  // not expose page.context(), so we go through evaluate() instead. This
  // catches non-HttpOnly cookies; the doorway sets elohim_session as a
  // first-party cookie readable from the page origin.
  const cookieString = (await page.evaluate(() => document.cookie)) as string;
  const cookies: CookieRow[] = cookieString
    .split(/;\s*/)
    .filter(s => s.length > 0)
    .map(pair => {
      const eq = pair.indexOf('=');
      return eq === -1
        ? { name: pair, value: '' }
        : { name: pair.slice(0, eq), value: pair.slice(eq + 1) };
    });
  const session = cookies.find(c => c.name === 'elohim_session');
  assert.ok(
    session,
    `Expected elohim_session cookie, found: [${cookies.map(c => c.name).join(', ')}]`
  );
});

Then("the trust-indicator updates to show matthew's humanId", async function (this: E2EWorld) {
  const page = getVisitorPage(this);
  const indicator = page.locator(TRUST_INDICATOR);
  await indicator.waitFor({ state: 'visible' });
  // After auth the indicator should reflect the signed-in identity — the
  // identifier or humanId should appear somewhere in its rendered text.
  await page.waitForFunction(
    (selector: string) => {
      const el = document.querySelector(selector);
      return el ? /matthew/i.test(el.textContent ?? '') : false;
    },
    TRUST_INDICATOR,
    { timeout: 10_000 }
  );
  const text = ((await indicator.textContent()) ?? '').trim();
  assert.match(text, /matthew/i, `Expected trust-indicator to mention matthew, got "${text}"`);
});

Then('the browser navigates to {string}', async function (this: E2EWorld, path: string) {
  const page = getVisitorPage(this);
  await page.waitForURL((url: URL) => url.pathname === path || url.pathname.startsWith(path), {
    timeout: 15_000,
  });
});

Then('the login-card shows the inline credentials error', async function (this: E2EWorld) {
  const page = getVisitorPage(this);
  const card = page.locator(LOGIN_CARD);
  const error = card.locator(LOGIN_ERROR_PART);
  await error.waitFor({ state: 'visible', timeout: 10_000 });
  const text = ((await error.textContent()) ?? '').trim();
  assert.ok(text.length > 0, 'Expected non-empty error text in login-card');
});

Then(
  'the trust-indicator remains visible at the top of the shell',
  async function (this: E2EWorld) {
    const page = getVisitorPage(this);
    const indicator = page.locator(TRUST_INDICATOR);
    const visible = await indicator.isVisible();
    assert.equal(visible, true, 'Expected trust-indicator to remain visible after error');
  }
);

Then(
  String.raw`an {string} link points to \/identity\/recovery`,
  async function (this: E2EWorld, linkText: string) {
    const page = getVisitorPage(this);
    const link = page.locator('a[href="/identity/recovery"]');
    await link.first().waitFor({ state: 'visible', timeout: 10_000 });
    const text = ((await link.first().textContent()) ?? '').trim().toLowerCase();
    const expected = linkText.toLowerCase().replace(/"/g, '');
    assert.ok(
      text.includes(expected),
      `Expected recovery link text to contain "${expected}", got "${text}"`
    );
  }
);
