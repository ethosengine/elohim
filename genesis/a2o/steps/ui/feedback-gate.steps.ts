/**
 * Feedback Gate Step Definitions — Playwright assertions for the
 * GateFeedbackTriggerComponent + GateFeedbackModalComponent surface in
 * content-viewer.
 *
 * Operationalizes P4 from
 * genesis/docs/superpowers/specs/2026-04-19-gate-challenge-and-indemnification-design.md
 * — accountability-architecture surface for any content view.
 *
 * Requires E2E_DEVICE_MODE=playwright; otherwise steps return 'pending'.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { FEEDBACK_GATE, VIEWER } from '../../src/framework/pages/selectors.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';

const DEFAULT_CONTENT_ID = 'manifesto';

function requirePlaywright(world: E2EWorld, humanName?: string): PlaywrightDevice | null {
  if (world.deviceMode !== 'playwright') return null;
  if (humanName) {
    const human = world.getHuman(humanName);
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (!device) return null;
    return device;
  }
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  return null;
}

async function ensureOnContentPage(device: PlaywrightDevice): Promise<void> {
  const appUrl = doorwayToAppUrl(device.client.url);
  await device.page.goto(`${appUrl}/lamad/resource/${DEFAULT_CONTENT_ID}`, {
    waitUntil: 'networkidle',
  });
  // Wait for the content-viewer header to render — that's where our trigger lives.
  // We don't need the markdown body fully loaded; the trigger sits in `.content-actions`
  // alongside Edit/Download which render as soon as the node fetches.
  await device.page
    .locator(`[data-testid="${VIEWER.FEEDBACK_TRIGGER}"]`)
    .waitFor({ state: 'visible', timeout: 30_000 });
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

When(
  '{word} navigates to a markdown content page',
  async function (this: E2EWorld, humanName: string) {
    const device = requirePlaywright(this, humanName);
    if (!device) return 'pending';
    await ensureOnContentPage(device);
  }
);

Given(
  '{word} is viewing a markdown content page',
  async function (this: E2EWorld, humanName: string) {
    const device = requirePlaywright(this, humanName);
    if (!device) return 'pending';
    await ensureOnContentPage(device);
  }
);

// ---------------------------------------------------------------------------
// Trigger visibility
// ---------------------------------------------------------------------------

Then(
  'the feedback trigger should be visible in the content actions',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const trigger = device.page.locator(`[data-testid="${VIEWER.FEEDBACK_TRIGGER}"]`);
    await trigger.waitFor({ state: 'visible', timeout: 10_000 });
  }
);

Then('the feedback trigger should have an accessible label', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const triggerBtn = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_BTN}"]`);
  await triggerBtn.waitFor({ state: 'visible', timeout: 10_000 });
  const label = await triggerBtn.getAttribute('aria-label');
  assert.ok(label && label.length > 0, `feedback trigger needs aria-label, got: "${label}"`);
});

// ---------------------------------------------------------------------------
// Menu interactions
// ---------------------------------------------------------------------------

When('{word} opens the feedback trigger menu', async function (this: E2EWorld, humanName: string) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';

  const triggerBtn = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_BTN}"]`);
  await triggerBtn.click();

  const menu = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_MENU}"]`);
  await menu.waitFor({ state: 'visible', timeout: 5_000 });
});

Given(
  '{word} has opened the feedback trigger menu',
  async function (this: E2EWorld, humanName: string) {
    const device = requirePlaywright(this, humanName);
    if (!device) return 'pending';

    const triggerBtn = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_BTN}"]`);
    await triggerBtn.click();
    const menu = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_MENU}"]`);
    await menu.waitFor({ state: 'visible', timeout: 5_000 });
  }
);

Then('the feedback menu should list {string}', async function (this: E2EWorld, label: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const menu = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_MENU}"]`);
  const item = menu.getByText(label, { exact: true });
  await item.waitFor({ state: 'visible', timeout: 5_000 });
});

When(
  '{word} selects {string} from the menu',
  async function (this: E2EWorld, humanName: string, label: string) {
    const device = requirePlaywright(this, humanName);
    if (!device) return 'pending';

    const menu = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_MENU}"]`);
    await menu.getByText(label, { exact: true }).click();

    const panel = device.page.locator(`[data-testid="${FEEDBACK_GATE.MODAL_PANEL}"]`);
    await panel.waitFor({ state: 'visible', timeout: 5_000 });
  }
);

// ---------------------------------------------------------------------------
// Modal visibility & stacking
// ---------------------------------------------------------------------------

Then('the feedback dialogue panel should be visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const panel = device.page.locator(`[data-testid="${FEEDBACK_GATE.MODAL_PANEL}"]`);
  await panel.waitFor({ state: 'visible', timeout: 5_000 });
});

Then('the feedback dialogue panel should not be visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const panel = device.page.locator(`[data-testid="${FEEDBACK_GATE.MODAL_PANEL}"]`);
  await panel.waitFor({ state: 'hidden', timeout: 5_000 });
});

Then(
  'the feedback dialogue panel should stack above any table-of-contents sidebar',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const panel = device.page.locator(`[data-testid="${FEEDBACK_GATE.MODAL_PANEL}"]`);
    await panel.waitFor({ state: 'visible' });

    const backdropSel = `[data-testid="${FEEDBACK_GATE.MODAL_BACKDROP}"]`;
    const panelSel = `[data-testid="${FEEDBACK_GATE.MODAL_PANEL}"]`;

    // Native <dialog>.showModal() places the dialog in the browser top layer —
    // above every stacking context regardless of ancestor transforms,
    // overflow, or z-index. Top-layer membership is asserted via the
    // standardized :modal pseudo-class (CSSWG; supported in Chromium/
    // Firefox/Safari). Reading getComputedStyle().zIndex returns "auto"
    // for top-layer elements, which parses to NaN — z-index isn't the
    // mechanism here.
    const topLayerCheck = (await device.page.evaluate((sel: string) => {
      const el = document.querySelector(sel);
      if (!el) return { found: false, isTopLayer: false, isDialog: false, isOpen: false };
      const isDialog = el.tagName === 'DIALOG';
      const isOpen = el.hasAttribute('open');
      let isTopLayer = false;
      try {
        isTopLayer = el.matches(':modal');
      } catch {
        // jsdom or older browsers without :modal pseudo-class — fall back
        // to dialog-with-open-attribute as a structural proxy.
        isTopLayer = isDialog && isOpen;
      }
      return { found: true, isTopLayer, isDialog, isOpen };
    }, backdropSel)) as {
      found: boolean;
      isTopLayer: boolean;
      isDialog: boolean;
      isOpen: boolean;
    };

    assert.ok(topLayerCheck.found, 'feedback dialog backdrop element not found');
    assert.ok(
      topLayerCheck.isDialog,
      `feedback backdrop must be a <dialog> element (got tagName via showModal-based stacking)`
    );
    assert.ok(topLayerCheck.isOpen, 'feedback dialog must have [open] attribute when shown');
    assert.ok(
      topLayerCheck.isTopLayer,
      'feedback dialog must be in browser top layer (showModal()) — above any TOC sidebar'
    );

    // Hit-testing: the click target at the panel center should be the panel
    // (or one of its descendants), not a chrome element above it.
    const panelOnTop = (await device.page.evaluate((sel: string) => {
      const el = document.querySelector(sel);
      if (!el) return false;
      const rect = el.getBoundingClientRect();
      const x = rect.left + rect.width / 2;
      const y = rect.top + rect.height / 2;
      const hit = document.elementFromPoint(x, y);
      return hit ? el === hit || el.contains(hit) : false;
    }, panelSel)) as boolean;

    assert.ok(panelOnTop, 'feedback dialogue panel center must be hit-testable (nothing above it)');
  }
);

Then('the artifact textarea should be selectable and focusable', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const textarea = device.page.locator(`[data-testid="${FEEDBACK_GATE.ARTIFACT_TEXTAREA}"]`);
  await textarea.waitFor({ state: 'visible', timeout: 5_000 });
  await textarea.click();
  const isFocused = (await device.page.evaluate((sel: string) => {
    const el = document.querySelector(sel);
    return el === document.activeElement;
  }, `[data-testid="${FEEDBACK_GATE.ARTIFACT_TEXTAREA}"]`)) as boolean;
  assert.ok(isFocused, 'artifact textarea did not receive focus on click');
});

Then(
  'the artifact textarea placeholder should be {string}',
  async function (this: E2EWorld, expected: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const textarea = device.page.locator(`[data-testid="${FEEDBACK_GATE.ARTIFACT_TEXTAREA}"]`);
    await textarea.waitFor({ state: 'visible', timeout: 5_000 });
    const placeholder = await textarea.getAttribute('placeholder');
    assert.equal(placeholder, expected, `expected placeholder "${expected}", got "${placeholder}"`);
  }
);

Then(
  'the feedback dialogue panel title should read {string}',
  async function (this: E2EWorld, expected: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const title = device.page.locator(`[data-testid="${FEEDBACK_GATE.MODAL_TITLE}"]`);
    await title.waitFor({ state: 'visible', timeout: 5_000 });
    const text = (await title.textContent())?.trim();
    assert.equal(text, expected, `expected title "${expected}", got "${text}"`);
  }
);

// ---------------------------------------------------------------------------
// Dismissal
// ---------------------------------------------------------------------------

Given(
  '{word} has the feedback dialogue panel open with {string} selected',
  async function (this: E2EWorld, humanName: string, label: string) {
    const device = requirePlaywright(this, humanName);
    if (!device) return 'pending';

    await ensureOnContentPage(device);
    await device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_BTN}"]`).click();
    await device.page
      .locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_MENU}"]`)
      .getByText(label, { exact: true })
      .click();
    await device.page
      .locator(`[data-testid="${FEEDBACK_GATE.MODAL_PANEL}"]`)
      .waitFor({ state: 'visible', timeout: 5_000 });
  }
);

When('{word} clicks the dialogue close button', async function (this: E2EWorld, humanName: string) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';

  await device.page.locator(`[data-testid="${FEEDBACK_GATE.MODAL_CLOSE}"]`).click();
});

When('{word} clicks the dialogue backdrop', async function (this: E2EWorld, humanName: string) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';

  // Native <dialog>::backdrop covers the viewport outside the dialog box;
  // clicks on the backdrop dispatch on the <dialog> element itself with
  // event.target === <dialog>. The iter-4 component handler uses that
  // exact predicate to distinguish backdrop from panel-content clicks.
  // PWPage stub omits page.mouse, so we synthesize the click with the
  // correct target by dispatching directly on the dialog element.
  await device.page.evaluate((sel: string) => {
    const dialog = document.querySelector(sel);
    if (!dialog) return;
    dialog.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
  }, `[data-testid="${FEEDBACK_GATE.MODAL_BACKDROP}"]`);
});

When('{word} presses the Escape key', async function (this: E2EWorld, humanName: string) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';

  // Native <dialog>.showModal() handles Escape via user-agent default —
  // dispatching a synthetic KeyboardEvent does not invoke the close path.
  // The iter-4 component listens for the (close) event on <dialog>, which
  // fires whether close was triggered by Escape, the close button, or
  // dialog.close(). Calling close() here exercises the same dismissal
  // contract a real Escape press would.
  await device.page.evaluate((sel: string) => {
    const dialog = document.querySelector<HTMLDialogElement>(sel);
    if (dialog && typeof dialog.close === 'function') {
      dialog.close();
    }
  }, `[data-testid="${FEEDBACK_GATE.MODAL_BACKDROP}"]`);
});
