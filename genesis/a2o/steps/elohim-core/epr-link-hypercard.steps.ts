/**
 * EPR-link HyperCard step definitions — Task B23 of the pillar EPR
 * decomposition plan.
 *
 * Feature: features/elohim-core/epr-link-hypercard.feature
 *
 * EPR-links flip in place (HyperCard semantics): clicking does NOT
 * trigger a browser navigation; the chip resolves inline; right-click
 * opens a keyboard-navigable context menu; offline EPRs fall back to a
 * declared previewEprRef.
 *
 * All three B23 scenarios are BROWSER-VERIFIABLE — they assert against
 * DOM state inside a mounted pillar bundle. The a2o framework drives the
 * browser via Playwright (E2E_DEVICE_MODE=playwright). These steps pierce
 * the nested shadow roots (elohim-epr-link → elohim-context-menu) and
 * assert by tag + ARIA role + label (the blank-slate primitives expose no
 * data-testids by design — see selectors.ts EPR_LINK / CONTEXT_MENU).
 *
 * The implementation follows steps/ui/epr-content.steps.ts: a Playwright
 * device is required; without one (non-browser mode, or no device wired
 * yet) the steps return 'pending' so the feature parses and runs cleanly.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { CONTEXT_MENU, EPR_LINK } from '../../src/framework/pages/selectors.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** URL captured before a resolve/right-click, for the "no navigation" assertion. */
let urlBefore: string | undefined;

/**
 * Return the first registered human's Playwright device, or null if none
 * (non-Playwright mode, or no browser human wired into this scenario yet).
 * Steps that get null return 'pending' — the scenario is skipped, not failed.
 */
function pwDevice(world: E2EWorld): PlaywrightDevice | null {
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  return null;
}

/**
 * The light-DOM locator for the host element. Playwright pierces the host's
 * own shadow root for descendant `>>>` queries, but the host tag itself is
 * in light DOM.
 */
function hostLocator(device: PlaywrightDevice) {
  return device.page.locator(EPR_LINK.HOST).first();
}

// ---------------------------------------------------------------------------
// Given — fixture / starting-state steps
// ---------------------------------------------------------------------------

Given(
  'the user is viewing {string} in a mounted lamad bundle',
  async function (this: E2EWorld, path: string) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    const appUrl = doorwayToAppUrl(device.client.url);
    await device.page.goto(`${appUrl}${path}`, { waitUntil: 'networkidle' });
    await hostLocator(device).waitFor({ state: 'attached', timeout: 15_000 });
    return undefined;
  }
);

Given(
  'the concept view contains <elohim-epr-link epr={string} display={string}>',
  async function (this: E2EWorld, eprUri: string, _display: string) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    const count = await device.page.locator(`${EPR_LINK.HOST}[epr="${eprUri}"]`).count();
    assert.ok(count > 0, `Expected an <elohim-epr-link epr="${eprUri}"> on the page`);
    return undefined;
  }
);

Given('an EPR-link points to an EPR that is currently unreachable', function (this: E2EWorld) {
  // Fixture setup (a sentinel EPR the storage layer denies) is wired during
  // the browser shake-out; the L4/unreachable path is unit-proven in
  // elohim-epr-link.spec.ts. Deferred until the unreachable fixture lands.
  return 'pending';
});

Given('the EPR has a previewEprRef declared', function (this: E2EWorld) {
  // Paired with the unreachable fixture above — the EPR Head previewEprRef
  // assertion is wired during the browser shake-out.
  return 'pending';
});

Given(
  'the user is viewing a page containing an <elohim-epr-link>',
  async function (this: E2EWorld) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    await hostLocator(device).waitFor({ state: 'attached', timeout: 15_000 });
    return undefined;
  }
);

// ---------------------------------------------------------------------------
// When — interaction steps
// ---------------------------------------------------------------------------

When('the chip resolves', async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  // Resolved chip renders the anchor button (L2/L3) inside the host shadow root.
  await device.page
    .locator(`${EPR_LINK.HOST} >>> ${EPR_LINK.ANCHOR}`)
    .first()
    .waitFor({ state: 'visible', timeout: 15_000 });
  // Record the URL so a later "no navigation" assertion can compare.
  urlBefore = device.page.url();
  return undefined;
});

When('the link resolves', async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  await hostLocator(device).waitFor({ state: 'attached', timeout: 15_000 });
  urlBefore = device.page.url();
  return undefined;
});

When('the user right-clicks the link', async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  urlBefore = device.page.url();
  await device.page
    .locator(`${EPR_LINK.HOST} >>> ${EPR_LINK.ANCHOR}`)
    .first()
    .click({ button: 'right' });
  await device.page
    .locator(`${EPR_LINK.HOST} >>> ${EPR_LINK.CONTEXT_MENU}[open]`)
    .first()
    .waitFor({ state: 'visible', timeout: 5_000 });
  return undefined;
});

// ---------------------------------------------------------------------------
// Then — outcome assertions
// ---------------------------------------------------------------------------

Then("the chip renders with the landing EPR's title and metadata", async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  const text =
    (await device.page.locator(`${EPR_LINK.HOST} >>> ${EPR_LINK.ANCHOR}`).first().textContent()) ??
    '';
  assert.ok(text.trim().length > 0, 'Expected the resolved chip to render a title');
  return undefined;
});

Then('no browser navigation occurs', function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  assert.equal(device.page.url(), urlBefore, 'Expected no browser navigation (HyperCard flip)');
  return undefined;
});

Then('the lamad Angular app remains mounted', async function (this: E2EWorld) {
  const device = pwDevice(this);
  if (!device) return 'pending';
  const mounted = await device.page.locator('app-root').count();
  assert.ok(mounted > 0, 'Expected the lamad app-root to remain mounted');
  return undefined;
});

Then("the chip renders the preview EPR's content", function (this: E2EWorld) {
  // Asserts the L4 preview render — deferred with the unreachable fixture.
  return 'pending';
});

Then(/^the chip displays the offline\/unreachable marker$/, function (this: E2EWorld) {
  // Asserts the [data-state="offline"] marker — deferred with the unreachable fixture.
  return 'pending';
});

Then(
  'a context menu opens including Open, About this EPR, and Copy EPR link',
  async function (this: E2EWorld) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    // Pierce host shadow → context-menu shadow → menuitems.
    const items = device.page.locator(
      `${EPR_LINK.HOST} >>> ${EPR_LINK.CONTEXT_MENU}[open] >>> ${CONTEXT_MENU.ITEM_ROLE}`
    );
    await items.first().waitFor({ state: 'visible', timeout: 5_000 });
    const labels = (await items.allTextContents()).map(t => t.trim());
    // Presence assertion (NOT exhaustive — the full Epic E set may be present).
    for (const expected of CONTEXT_MENU.MVP_LABELS) {
      assert.ok(
        labels.includes(expected),
        `Expected context menu to include "${expected}". Got: ${labels.join(', ')}`
      );
    }
    return undefined;
  }
);

Then(
  /^the menu can be navigated by keyboard \(arrows, Enter, Escape\)$/,
  async function (this: E2EWorld) {
    const device = pwDevice(this);
    if (!device) return 'pending';
    const menu = device.page.locator(`${EPR_LINK.HOST} >>> ${EPR_LINK.CONTEXT_MENU}[open]`).first();
    await menu.waitFor({ state: 'visible', timeout: 5_000 });
    // ArrowDown/ArrowUp move focus across menuitems; Escape closes the menu.
    // The stub PWPage exposes no `keyboard`, so dispatch keydowns through the
    // menu's [role=menu] (which owns the keydown handler) via evaluate. The
    // events must cross the host → context-menu shadow boundary.
    await device.page.evaluate(() => {
      const host = document.querySelector('elohim-epr-link');
      const menuEl = host?.shadowRoot
        ?.querySelector('elohim-context-menu')
        ?.shadowRoot?.querySelector('[role="menu"]');
      if (!menuEl) return;
      for (const key of ['ArrowDown', 'ArrowUp', 'Escape']) {
        menuEl.dispatchEvent(
          new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true })
        );
      }
    });
    await menu.waitFor({ state: 'hidden', timeout: 5_000 });
    return undefined;
  }
);
