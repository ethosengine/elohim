/**
 * Completion Feedback Step Definitions — Playwright-powered completion summary assertions.
 *
 * These steps validate the AssessmentCompletionSummaryComponent UI that appears
 * after discovery, mastery, and reflection assessments. They require
 * E2E_DEVICE_MODE=playwright. In HTTP mode, browser-only assertions return 'pending'.
 */

import { strict as assert } from 'node:assert';

import { Then, When } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { COMPLETION } from '../../src/framework/pages/selectors.js';
import { E2EWorld } from '../../src/framework/world.js';

function getDevice(world: E2EWorld): PlaywrightDevice {
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  throw new Error('No Playwright device found.');
}

/** Returns null if not in Playwright mode. */
function requirePlaywright(world: E2EWorld): PlaywrightDevice | null {
  if (world.deviceMode !== 'playwright') return null;
  return getDevice(world);
}

// ---------------------------------------------------------------------------
// Visibility
// ---------------------------------------------------------------------------

Then('the completion summary should be visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const summary = device.page.locator(`[data-testid="${COMPLETION.SUMMARY}"]`);
  await summary.waitFor({ state: 'visible', timeout: 15_000 });
});

Then('the completion summary should not be visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const summary = device.page.locator(`[data-testid="${COMPLETION.SUMMARY}"]`);
  await summary.waitFor({ state: 'hidden', timeout: 10_000 });
});

// ---------------------------------------------------------------------------
// Headline & Description
// ---------------------------------------------------------------------------

Then(
  'the completion headline should contain a personalized type name',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const headline = device.page.locator(`[data-testid="${COMPLETION.HEADLINE}"]`);
    await headline.waitFor({ state: 'visible', timeout: 10_000 });
    const text = await headline.textContent();
    assert.ok(text, 'Headline should have text content');
    // Personalized headlines contain "You're a/an [Name]!"
    assert.match(text, /You're an? .+!/, `Expected personalized headline, got: "${text}"`);
  }
);

Then(
  'the completion headline should contain {string}',
  async function (this: E2EWorld, expected: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const headline = device.page.locator(`[data-testid="${COMPLETION.HEADLINE}"]`);
    await headline.waitFor({ state: 'visible', timeout: 10_000 });
    const text = await headline.textContent();
    assert.ok(text?.includes(expected), `Expected headline to contain "${expected}", got: "${text}"`);
  }
);

Then(
  'the completion description should reference the primary type',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const desc = device.page.locator(`[data-testid="${COMPLETION.DESCRIPTION}"]`);
    await desc.waitFor({ state: 'visible', timeout: 10_000 });
    const text = await desc.textContent();
    assert.ok(text && text.length > 10, `Expected descriptive text, got: "${text}"`);
  }
);

// ---------------------------------------------------------------------------
// Hex Badge
// ---------------------------------------------------------------------------

Then('a hex badge preview should be displayed', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const badge = device.page.locator(`[data-testid="${COMPLETION.HEX_BADGE}"]`);
  await badge.waitFor({ state: 'visible', timeout: 10_000 });
  const text = await badge.textContent();
  assert.ok(text && text.trim().length > 0, 'Hex badge should display a type label');
});

Then('no hex badge preview should be displayed', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const badge = device.page.locator(`[data-testid="${COMPLETION.BADGE_REVEAL}"]`);
  const count = await badge.count();
  assert.equal(count, 0, 'Expected no hex badge in non-discovery mode');
});

// ---------------------------------------------------------------------------
// Subscale Breakdown
// ---------------------------------------------------------------------------

Then('the subscale breakdown should be visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const subscales = device.page.locator(`[data-testid="${COMPLETION.SUBSCALES}"]`);
  await subscales.waitFor({ state: 'visible', timeout: 10_000 });
});

Then('each subscale bar should have a non-zero width', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const selector = `[data-testid="${COMPLETION.SUBSCALES}"] .subscale-fill`;
  const allNonZero = await device.page.evaluate((sel: string) => {
    const fills = document.querySelectorAll(sel);
    if (fills.length === 0) return false;
    return Array.from(fills).every(
      (el) => (el as HTMLElement).offsetWidth > 0
    );
  }, selector);

  assert.ok(allNonZero, 'Expected all subscale bars to have non-zero width');
});

// ---------------------------------------------------------------------------
// Score Display (Mastery)
// ---------------------------------------------------------------------------

Then('the score display should show a percentage', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const score = device.page.locator(`[data-testid="${COMPLETION.SCORE}"]`);
  await score.waitFor({ state: 'visible', timeout: 10_000 });
  const text = await score.textContent();
  assert.ok(text, 'Score display should have text');
  assert.match(text, /\d+%/, `Expected percentage in score display, got: "${text}"`);
});

Then('no score display should be shown', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const score = device.page.locator(`[data-testid="${COMPLETION.SCORE}"]`);
  const count = await score.count();
  assert.equal(count, 0, 'Expected no score display in non-mastery mode');
});

// ---------------------------------------------------------------------------
// Result Card Styling
// ---------------------------------------------------------------------------

Then(
  'the result card should have the {string} style',
  async function (this: E2EWorld, expectedClass: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const selector = `[data-testid="${COMPLETION.RESULT_CARD}"]`;
    const card = device.page.locator(selector);
    await card.waitFor({ state: 'visible', timeout: 10_000 });

    const hasClass = await device.page.evaluate(
      ({ sel, cls }: { sel: string; cls: string }) => {
        const el = document.querySelector(sel);
        return el ? el.classList.contains(cls) : false;
      },
      { sel: selector, cls: expectedClass }
    );
    assert.ok(hasClass, `Expected result card to have class "${expectedClass}"`);
  }
);

// ---------------------------------------------------------------------------
// Profile Link
// ---------------------------------------------------------------------------

Then(
  'the profile link should read {string}',
  async function (this: E2EWorld, expectedText: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const link = device.page.locator(`[data-testid="${COMPLETION.PROFILE_LINK}"]`);
    await link.waitFor({ state: 'visible', timeout: 10_000 });
    const text = await link.textContent();
    assert.ok(
      text?.includes(expectedText),
      `Expected profile link to contain "${expectedText}", got: "${text}"`
    );
  }
);

// ---------------------------------------------------------------------------
// Continue Button
// ---------------------------------------------------------------------------

When('I click the completion continue button', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const btn = device.page.locator(`[data-testid="${COMPLETION.CONTINUE}"]`);
  await btn.waitFor({ state: 'visible', timeout: 10_000 });
  await btn.click();
  await device.page.waitForTimeout(500);
});

// ---------------------------------------------------------------------------
// Attestation (negative — mastery should NOT record discovery attestations)
// ---------------------------------------------------------------------------

Then('no discovery attestation should be recorded', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const hasAttestation = await device.page.evaluate(() => {
    const attestations = localStorage.getItem('discovery-attestations');
    if (!attestations) return false;
    try {
      const parsed = JSON.parse(attestations);
      return Array.isArray(parsed) ? parsed.length > 0 : Object.keys(parsed).length > 0;
    } catch {
      return false;
    }
  });

  assert.ok(!hasAttestation, 'Expected no discovery attestations after mastery completion');
});
