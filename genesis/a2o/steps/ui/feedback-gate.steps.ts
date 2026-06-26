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
  // §12.1 Slice 2 — content is served at the universal /epr/{id} address.
  await device.page.goto(`${appUrl}/epr/${DEFAULT_CONTENT_ID}`, {
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
    await panel.waitFor({ state: 'visible', timeout: 5_000 });

    // The gate feedback UI is now the Lit <elohim-gate-feedback-trigger> element,
    // which renders a fixed `.modal-overlay` (z-index 1000) in shadow DOM — NOT a
    // native <dialog>/top-layer (the old Angular component). "Stacks above the ToC
    // sidebar" therefore means the panel is genuinely on top and hit-testable: a
    // Playwright trial click resolves actionability (visible + stable + receives
    // pointer events + NOT obscured) and pierces shadow DOM. If a sidebar overlaid
    // the panel, the trial click would throw.
    await panel.click({ trial: true, timeout: 5_000 });
  }
);

Then('the artifact textarea should be selectable and focusable', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const textarea = device.page.locator(`[data-testid="${FEEDBACK_GATE.ARTIFACT_TEXTAREA}"]`);
  await textarea.waitFor({ state: 'visible', timeout: 5_000 });
  await textarea.click();
  // The textarea lives in the Lit element's shadow root, so document.activeElement
  // is the host; check focus within the textarea's own root node (Playwright's
  // element-scoped evaluate resolves the shadow element directly).
  const isFocused = await textarea.evaluate(
    (el: HTMLElement) => el === (el.getRootNode() as Document | ShadowRoot).activeElement
  );
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

  // The Lit modal overlay (`.modal-overlay`) dismisses on a click whose target
  // is the overlay itself; the centered panel calls stopPropagation, so a click
  // ON the panel does NOT close it. Click the overlay near a corner — outside the
  // centered panel — so the overlay (not the panel) receives the click. Playwright
  // pierces shadow DOM and resolves the position against the overlay's box.
  await device.page
    .locator(`[data-testid="${FEEDBACK_GATE.MODAL_BACKDROP}"]`)
    .click({ position: { x: 8, y: 8 } });
});

When('{word} presses the Escape key', async function (this: E2EWorld, humanName: string) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';

  // The Lit modal closes on Escape via the overlay's keydown handler (the panel
  // lets Escape bubble up to it). Focus an element inside the modal so the
  // keypress originates within the overlay subtree, then press Escape for real.
  const textarea = device.page.locator(`[data-testid="${FEEDBACK_GATE.ARTIFACT_TEXTAREA}"]`);
  if (await textarea.isVisible().catch(() => false)) {
    await textarea.click();
  }
  await device.page.keyboard.press('Escape');
});
