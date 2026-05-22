/**
 * Threshold-Login Domain-Scoping Step Definitions
 *
 * Drives the doorway threshold-login form via Playwright. The form is the
 * doorway's relying-party OAuth surface; for the domain-scoping feature we
 * never actually submit credentials — we inspect input behaviour and the
 * decorative suffix that names the gateway.
 *
 * Selectors live in src/framework/pages/selectors.ts (THRESHOLD.*).
 */

import { strict as assert } from 'node:assert';

import { Then, When } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { Human } from '../../src/framework/human.js';
import { THRESHOLD } from '../../src/framework/pages/selectors.js';
import { E2EWorld } from '../../src/framework/world.js';

// The doorway-app Angular route is `/login`; doorway-service exposes it at
// `/threshold/login` via reverse proxy. We try the proxy form first, then fall
// back to the app-direct route — works under both E2E_DOORWAY_ALPHA pointing
// at the doorway-service (8888) and pointing at the doorway-app directly (8081).
const THRESHOLD_PROXY_PATH = '/threshold/login';
const THRESHOLD_DIRECT_PATH = '/login';

// ---------------------------------------------------------------------------
// Inspector — a tiny anonymous browser session for assertions about the form
// itself (no auth, no fixture human).
// ---------------------------------------------------------------------------

const INSPECTOR_NAME = '_threshold-inspector';

async function ensureInspector(world: E2EWorld): Promise<PlaywrightDevice> {
  let human: Human;
  try {
    human = world.getHuman(INSPECTOR_NAME);
  } catch {
    human = new Human(INSPECTOR_NAME, { identifier: '', password: '', displayName: INSPECTOR_NAME });
    world.addHuman(INSPECTOR_NAME, human);
  }

  const existing = human.devices.find(d => d.type === 'playwright');
  if (existing) return existing as PlaywrightDevice;

  // Pick the first registered doorway as the inspection target.
  const [, doorway] = [...world.doorways][0] ?? [];
  assert.ok(doorway, 'No doorway registered. Did the Background step run?');

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await world.getBrowser();
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const device = new PlaywrightDevice(`${INSPECTOR_NAME}-pw`, doorway.url, doorway.url, browser);
  await device.init();
  human.addDevice(device);
  world.onCleanup(async () => device.close());
  return device;
}

function getInspectorPage(world: E2EWorld) {
  const human = world.getHuman(INSPECTOR_NAME);
  const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
  assert.ok(device, 'Inspector Playwright device is missing');
  return device.page;
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

When('I navigate to the threshold-login page', async function (this: E2EWorld) {
  const device = await ensureInspector(this);
  // Try /threshold/login first (production-shape via doorway-service proxy).
  // If that returns 404 (e.g., in local dev where E2E_DOORWAY_ALPHA points at
  // doorway-app:8081 directly), fall back to /login.
  await device.navigate(THRESHOLD_PROXY_PATH);
  const proxyOk = await device.page
    .locator(`[data-testid="${THRESHOLD.IDENTIFIER}"]`)
    .waitFor({ state: 'visible', timeout: 5_000 })
    .then(() => true)
    .catch(() => false);
  if (!proxyOk) {
    await device.navigate(THRESHOLD_DIRECT_PATH);
    await device.page.locator(`[data-testid="${THRESHOLD.IDENTIFIER}"]`).waitFor({
      state: 'visible',
      timeout: 15_000,
    });
  }
});

// ---------------------------------------------------------------------------
// Visible suffix
// ---------------------------------------------------------------------------

Then('I see the identifier input followed by a domain suffix', async function (this: E2EWorld) {
  const page = getInspectorPage(this);
  const suffix = page.locator(`[data-testid="${THRESHOLD.DOMAIN_SUFFIX}"]`);
  await suffix.waitFor({ state: 'visible' });
  const text = (await suffix.textContent())?.trim() ?? '';
  assert.ok(text.startsWith('@'), `Expected domain suffix to begin with "@", got "${text}"`);
});

Then("the domain suffix matches the doorway's gateway hostname", async function (this: E2EWorld) {
  const page = getInspectorPage(this);
  const suffix =
    (await page.locator(`[data-testid="${THRESHOLD.DOMAIN_SUFFIX}"]`).textContent())?.trim() ?? '';
  assert.ok(suffix.startsWith('@'), `suffix must start with @, got "${suffix}"`);
  const url = new URL(page.url());
  // gatewayDomain() in threshold-login.component.ts strips a 'doorway-' prefix
  // from the hostname (e.g., 'doorway-alpha.elohim.host' → 'alpha.elohim.host')
  // and otherwise echoes the hostname as-is (e.g., 'localhost' → 'localhost').
  const expectedDomain = url.hostname.startsWith('doorway-')
    ? url.hostname.slice('doorway-'.length)
    : url.hostname;
  assert.equal(
    suffix.slice(1),
    expectedDomain,
    `suffix should reflect the gateway hostname — expected "@${expectedDomain}", got "${suffix}"`
  );
});

Then(
  'the hint text names the doorway\'s domain rather than offering "full email"',
  async function (this: E2EWorld) {
    const page = getInspectorPage(this);
    const hint = await page.locator('.input-hint').textContent();
    assert.ok(
      hint && !/full email/i.test(hint),
      `expected hint to not mention "full email", got "${hint}"`
    );
  }
);

// ---------------------------------------------------------------------------
// '@'-strip behaviour
// ---------------------------------------------------------------------------

When('I type {string} into the identifier input', async function (this: E2EWorld, value: string) {
  const page = getInspectorPage(this);
  const input = page.locator(`[data-testid="${THRESHOLD.IDENTIFIER}"]`);
  await input.fill(value);
});

Then('the identifier input value is {string}', async function (this: E2EWorld, expected: string) {
  const page = getInspectorPage(this);
  const input = page.locator(`[data-testid="${THRESHOLD.IDENTIFIER}"]`);
  // The two-way binding strips '@'-segments via onIdentifierChange — wait
  // for the projection to settle.
  await page.waitForFunction(
    ({ selector, expected }: { selector: string; expected: string }) => {
      const el = document.querySelector(selector) as HTMLInputElement | null;
      return el?.value === expected;
    },
    { selector: `[data-testid="${THRESHOLD.IDENTIFIER}"]`, expected },
    { timeout: 2_000 }
  );
  const actual = await input.inputValue();
  assert.equal(actual, expected, `identifier value mismatch: got "${actual}"`);
});

Then('the visible domain suffix is unchanged by what I typed', async function (this: E2EWorld) {
  const page = getInspectorPage(this);
  const suffix =
    (await page.locator(`[data-testid="${THRESHOLD.DOMAIN_SUFFIX}"]`).textContent())?.trim() ?? '';
  assert.ok(suffix.startsWith('@'), `expected suffix to start with '@', got "${suffix}"`);
  const inputValue = await page.locator(`[data-testid="${THRESHOLD.IDENTIFIER}"]`).inputValue();
  assert.ok(
    !inputValue.includes('@'),
    `identifier input still contains '@': "${inputValue}" — strip logic regressed`
  );
});

// ---------------------------------------------------------------------------
// Federated-doorway hint
// ---------------------------------------------------------------------------

Then('the hint contains a {string} link', async function (this: E2EWorld, linkText: string) {
  const page = getInspectorPage(this);
  const link = page.locator(`[data-testid="${THRESHOLD.DIFFERENT_DOORWAY_HINT}"]`);
  await link.waitFor({ state: 'visible' });
  const actual = (await link.textContent())?.trim() ?? '';
  assert.equal(actual, linkText);
});

Then('the link points at the federated doorway picker', async function (this: E2EWorld) {
  const page = getInspectorPage(this);
  const link = page.locator(`[data-testid="${THRESHOLD.DIFFERENT_DOORWAY_HINT}"]`);
  const href = await link.getAttribute('href');
  assert.ok(href, 'expected hint link to have an href');
  assert.ok(
    href.includes('/threshold/doorways'),
    `expected federated-doorway picker URL, got "${href}"`
  );
});
