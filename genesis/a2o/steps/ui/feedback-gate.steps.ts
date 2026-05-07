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
  await device.page.locator('.markdown-content').waitFor({ state: 'visible', timeout: 15_000 });
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

When('{word} navigates to a markdown content page', async function (
  this: E2EWorld,
  humanName: string,
) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';
  await ensureOnContentPage(device);
});

Given('{word} is viewing a markdown content page', async function (
  this: E2EWorld,
  humanName: string,
) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';
  await ensureOnContentPage(device);
});

// ---------------------------------------------------------------------------
// Trigger visibility
// ---------------------------------------------------------------------------

Then('the feedback trigger should be visible in the content actions', async function (
  this: E2EWorld,
) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const trigger = device.page.locator(`[data-testid="${VIEWER.FEEDBACK_TRIGGER}"]`);
  await trigger.waitFor({ state: 'visible', timeout: 10_000 });
});

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

Given('{word} has opened the feedback trigger menu', async function (
  this: E2EWorld,
  humanName: string,
) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';

  const triggerBtn = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_BTN}"]`);
  await triggerBtn.click();
  const menu = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_MENU}"]`);
  await menu.waitFor({ state: 'visible', timeout: 5_000 });
});

Then('the feedback menu should list {string}', async function (this: E2EWorld, label: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const menu = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_MENU}"]`);
  const item = menu.getByText(label, { exact: true });
  await item.waitFor({ state: 'visible', timeout: 5_000 });
});

When('{word} selects {string} from the menu', async function (
  this: E2EWorld,
  humanName: string,
  label: string,
) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';

  const menu = device.page.locator(`[data-testid="${FEEDBACK_GATE.TRIGGER_MENU}"]`);
  await menu.getByText(label, { exact: true }).click();

  const panel = device.page.locator(`[data-testid="${FEEDBACK_GATE.MODAL_PANEL}"]`);
  await panel.waitFor({ state: 'visible', timeout: 5_000 });
});

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

    const backdropZ = await device.page
      .locator(`[data-testid="${FEEDBACK_GATE.MODAL_BACKDROP}"]`)
      .evaluate(el => parseInt(window.getComputedStyle(el).zIndex || '0', 10));

    // The TOC sidebar's known stacking ceiling is 1100 (markdown-renderer.component.css).
    assert.ok(
      backdropZ > 1100,
      `feedback backdrop z-index ${backdropZ} must exceed TOC sidebar (1100)`,
    );

    const panel = device.page.locator(`[data-testid="${FEEDBACK_GATE.MODAL_PANEL}"]`);
    await panel.waitFor({ state: 'visible' });

    // Hit-testing: the click target at the panel center should be the panel
    // (or one of its descendants), not a chrome element above it.
    const panelOnTop = await panel.evaluate(el => {
      const rect = el.getBoundingClientRect();
      const x = rect.left + rect.width / 2;
      const y = rect.top + rect.height / 2;
      const hit = document.elementFromPoint(x, y);
      return hit ? el.contains(hit) || el === hit : false;
    });
    assert.ok(panelOnTop, 'feedback dialogue panel center must be hit-testable (nothing above it)');
  },
);

Then('the artifact textarea should be selectable and focusable', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const textarea = device.page.locator(`[data-testid="${FEEDBACK_GATE.ARTIFACT_TEXTAREA}"]`);
  await textarea.waitFor({ state: 'visible', timeout: 5_000 });
  await textarea.click();
  const isFocused = await textarea.evaluate(el => el === document.activeElement);
  assert.ok(isFocused, 'artifact textarea did not receive focus on click');
});

Then('the artifact textarea placeholder should be {string}', async function (
  this: E2EWorld,
  expected: string,
) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const textarea = device.page.locator(`[data-testid="${FEEDBACK_GATE.ARTIFACT_TEXTAREA}"]`);
  await textarea.waitFor({ state: 'visible', timeout: 5_000 });
  const placeholder = await textarea.getAttribute('placeholder');
  assert.equal(placeholder, expected, `expected placeholder "${expected}", got "${placeholder}"`);
});

Then('the feedback dialogue panel title should read {string}', async function (
  this: E2EWorld,
  expected: string,
) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const title = device.page.locator(`[data-testid="${FEEDBACK_GATE.MODAL_TITLE}"]`);
  await title.waitFor({ state: 'visible', timeout: 5_000 });
  const text = (await title.textContent())?.trim();
  assert.equal(text, expected, `expected title "${expected}", got "${text}"`);
});

// ---------------------------------------------------------------------------
// Dismissal
// ---------------------------------------------------------------------------

Given('{word} has the feedback dialogue panel open with {string} selected', async function (
  this: E2EWorld,
  humanName: string,
  label: string,
) {
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
});

When('{word} clicks the dialogue close button', async function (
  this: E2EWorld,
  humanName: string,
) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';

  await device.page.locator(`[data-testid="${FEEDBACK_GATE.MODAL_CLOSE}"]`).click();
});

When('{word} clicks the dialogue backdrop', async function (this: E2EWorld, humanName: string) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';

  // Click the backdrop edge (not the panel) so stopPropagation on the panel
  // doesn't swallow it. position: { x: 5, y: 5 } targets a corner.
  await device.page
    .locator(`[data-testid="${FEEDBACK_GATE.MODAL_BACKDROP}"]`)
    .click({ position: { x: 5, y: 5 } });
});

When('{word} presses the Escape key', async function (this: E2EWorld, humanName: string) {
  const device = requirePlaywright(this, humanName);
  if (!device) return 'pending';

  await device.page.keyboard.press('Escape');
});
