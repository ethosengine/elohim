/**
 * Generic browser-element step definitions + anonymous landing-page visitor.
 *
 * Provides the shared `[data-testid="..."]` vocabulary used by the protocol
 * chrome features (and any browser feature that asserts on test ids):
 *   - When I open the landing page in a browser
 *   - Then the element [data-testid="X"] is visible
 *   - Then the element [data-testid="X"] is not visible
 *   - When I click the element [data-testid="X"]
 *   - Then the element [data-testid="X"] text contains "Y"
 *
 * The landing page is an ANONYMOUS visit served BY the doorway (the swap-test
 * surface): a synthetic `_landing-visitor` human carries a Playwright device
 * pointed at the doorway URL, mirroring the established `_portal-visitor`
 * pattern in steps/peer-oauth-portal/*. Requires E2E_DEVICE_MODE=playwright.
 *
 * The data-testid steps run against the "active" browser device — the landing
 * visitor when present (anonymous protocol scenarios), otherwise the first
 * logged-in human's Playwright device (e.g. auth/agency scenarios) — so the
 * same vocabulary serves both anonymous and authenticated browser flows.
 */

import { strict as assert } from 'node:assert';

import { Then, When } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { Human } from '../../src/framework/human.js';
import { E2EWorld } from '../../src/framework/world.js';

const LANDING_VISITOR = '_landing-visitor';
const VISIBLE_TIMEOUT = 10_000;

/**
 * Resolve (or lazily create) an anonymous browser device pointed at the alpha
 * doorway. Mirrors `ensureVisitor` in peer-oauth-portal/rp-consent.steps.ts so
 * landing scenarios run standalone without a logged-in human.
 */
async function ensureLandingVisitor(world: E2EWorld): Promise<PlaywrightDevice> {
  let human: Human;
  try {
    human = world.getHuman(LANDING_VISITOR);
  } catch {
    human = new Human(LANDING_VISITOR, {
      identifier: '',
      password: '',
      displayName: LANDING_VISITOR,
    });
    world.addHuman(LANDING_VISITOR, human);
  }

  const existing = human.devices.find(d => d.type === 'playwright');
  if (existing) return existing as PlaywrightDevice;

  const [, doorway] = [...world.doorways][0] ?? [];
  const doorwayUrl = doorway?.url ?? process.env['E2E_DOORWAY_ALPHA'] ?? '';
  assert.ok(doorwayUrl, 'No doorway URL — register one via Background or set E2E_DOORWAY_ALPHA');

  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await world.getBrowser();
  // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
  const device = new PlaywrightDevice(`${LANDING_VISITOR}-pw`, doorwayUrl, doorwayUrl, browser);
  await device.init();
  human.addDevice(device);
  world.onCleanup(async () => device.close());
  return device;
}

/**
 * The browser device the testid steps act on: prefer the anonymous landing
 * visitor; fall back to the first logged-in human's Playwright device.
 */
function activeDevice(world: E2EWorld): PlaywrightDevice {
  try {
    const visitor = world.getHuman(LANDING_VISITOR).devices.find(d => d.type === 'playwright');
    if (visitor) return visitor as PlaywrightDevice;
  } catch {
    // no landing visitor — fall through to a logged-in human
  }
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright');
    if (device) return device as PlaywrightDevice;
  }
  throw new Error(
    'No Playwright device — open the landing page or log a human in first (E2E_DEVICE_MODE=playwright).'
  );
}

When('I open the landing page in a browser', async function (this: E2EWorld) {
  const device = await ensureLandingVisitor(this);
  await device.navigate('/');
});

Then(
  /^the element \[data-testid="([^"]+)"\] is visible$/,
  async function (this: E2EWorld, testid: string) {
    const el = activeDevice(this).page.locator(`[data-testid="${testid}"]`).first();
    await el.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });
  }
);

Then(
  /^the element \[data-testid="([^"]+)"\] is not visible$/,
  async function (this: E2EWorld, testid: string) {
    const locator = activeDevice(this).page.locator(`[data-testid="${testid}"]`);
    const count = await locator.count();
    if (count === 0) return; // absent from the DOM ⇒ not visible
    const visible = await locator.first().isVisible();
    assert.ok(!visible, `Expected [data-testid="${testid}"] not to be visible, but it was`);
  }
);

When(
  /^I click the element \[data-testid="([^"]+)"\]$/,
  async function (this: E2EWorld, testid: string) {
    const el = activeDevice(this).page.locator(`[data-testid="${testid}"]`).first();
    await el.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });
    await el.click();
  }
);

Then(
  /^the element \[data-testid="([^"]+)"\] text contains "([^"]+)"$/,
  async function (this: E2EWorld, testid: string, expected: string) {
    const el = activeDevice(this).page.locator(`[data-testid="${testid}"]`).first();
    await el.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });
    const text = (await el.textContent()) ?? '';
    assert.ok(
      text.includes(expected),
      `Expected [data-testid="${testid}"] text to contain "${expected}" but was "${text.trim()}"`
    );
  }
);

// --- protocol-signal badge convenience steps (landing-page-dogfood) ---------

When('I click the protocol-signal badge', async function (this: E2EWorld) {
  const el = activeDevice(this).page.locator('[data-testid="protocol-signal-badge-pill"]').first();
  await el.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });
  await el.click();
});

Then('the panel text contains {string}', async function (this: E2EWorld, expected: string) {
  const el = activeDevice(this).page.locator('[data-testid="protocol-signal-panel"]').first();
  await el.waitFor({ state: 'visible', timeout: VISIBLE_TIMEOUT });
  const text = (await el.textContent()) ?? '';
  assert.ok(
    text.includes(expected),
    `Expected protocol-signal-panel text to contain "${expected}" but was "${text.trim()}"`
  );
});
