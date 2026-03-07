/**
 * Elohim Presence Step Definitions — Playwright-powered elohim insight flows.
 *
 * These steps validate the elohim presence UI: transparent insight panels that
 * appear after discovery assessments, constitutional reasoning display, and
 * computation cost transparency. They require E2E_DEVICE_MODE=playwright.
 * In HTTP mode, browser-only assertions return 'pending' to skip.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { ELOHIM_PRESENCE, BANNER } from '../../src/framework/pages/selectors.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/** Complete an assessment by interacting with likert/radio widgets until done. */
async function completeAssessment(device: PlaywrightDevice): Promise<void> {
  let maxQuestions = 30;
  while (maxQuestions-- > 0) {
    const likertTrack = device.page.locator('[data-testid="likert-scale-track"]').first();
    const radioGroup = device.page.locator('[role="radiogroup"]').first();

    const hasLikert = await likertTrack.isVisible().catch(() => false);
    const hasRadio = !hasLikert && (await radioGroup.isVisible().catch(() => false));

    if (hasLikert) {
      const ticks = device.page.locator(
        '[data-testid="likert-scale-tick"], [data-testid="likert-scale-thumb"]'
      );
      const count = await ticks.count();
      if (count > 0) {
        await ticks.nth(Math.floor(count / 2)).click();
      }
    } else if (hasRadio) {
      const choices = device.page.locator('[role="radio"]');
      const count = await choices.count();
      if (count > 0) {
        await choices.first().click();
      }
    } else {
      break;
    }

    const continueBtn = device.page
      .locator('[data-testid="sophia-continue"], [data-testid="quiz-continue"]')
      .first();
    if (await continueBtn.isVisible().catch(() => false)) {
      await continueBtn.click();
      await device.page.waitForTimeout(500);
    } else {
      break;
    }
  }

  await device.page.waitForTimeout(1000);
}

// ---------------------------------------------------------------------------
// Navigation & Setup
// ---------------------------------------------------------------------------

Given('the learner navigates to a discovery assessment', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  // Navigate to the learning paths page, then enter a discovery assessment
  await device.navigate('/lamad');
  await device.page.waitForLoadState('networkidle');

  // Look for a discovery assessment link/card
  const discoveryLink = device.page.getByText('discovery', { exact: false }).first();
  await discoveryLink.waitFor({ state: 'visible', timeout: 15_000 });
  await discoveryLink.click();
  await device.page.waitForLoadState('networkidle');
});

Given('the learner navigates to {string}', async function (this: E2EWorld, path: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const normalizedPath = path.startsWith('/') ? path : `/${path}`;
  await device.navigate(normalizedPath);
  await device.page.waitForLoadState('networkidle');
});

// ---------------------------------------------------------------------------
// Assessment Completion
// ---------------------------------------------------------------------------

When('the learner completes the assessment', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  await completeAssessment(device);
});

Given('the learner has completed a discovery assessment', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  // Navigate to discovery and complete it
  await device.navigate('/lamad');
  await device.page.waitForLoadState('networkidle');

  const discoveryLink = device.page.getByText('discovery', { exact: false }).first();
  if (await discoveryLink.isVisible().catch(() => false)) {
    await discoveryLink.click();
    await device.page.waitForLoadState('networkidle');
  }

  await completeAssessment(device);
});

// ---------------------------------------------------------------------------
// Elohim Insight Visibility
// ---------------------------------------------------------------------------

Given('the elohim insight is visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const insight = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.INSIGHT_SECTION}"]`);
  await insight.waitFor({ state: 'visible', timeout: 15_000 });
});

Given('the elohim has responded with an insight', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const insight = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.INSIGHT_SECTION}"]`);
  await insight.waitFor({ state: 'visible', timeout: 15_000 });
});

Then('an elohim insight section appears below the results', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const insight = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.INSIGHT_SECTION}"]`);
  await insight.waitFor({ state: 'visible', timeout: 15_000 });
});

// ---------------------------------------------------------------------------
// Recommendation Message
// ---------------------------------------------------------------------------

Then(
  'the insight contains a learning path recommendation message',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const message = device.page.locator(
      `[data-testid="${ELOHIM_PRESENCE.RECOMMENDATION_MESSAGE}"]`
    );
    await message.waitFor({ state: 'visible', timeout: 10_000 });
    const text = await message.textContent();
    assert.ok(text && text.trim().length > 0, 'Expected recommendation message to have content');
  }
);

// ---------------------------------------------------------------------------
// Constitutional Reasoning
// ---------------------------------------------------------------------------

Then(
  'the insight includes a {string} expandable section',
  async function (this: E2EWorld, _sectionName: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const expandable = device.page.locator(
      `[data-testid="${ELOHIM_PRESENCE.REASONING_EXPANDABLE}"]`
    );
    await expandable.waitFor({ state: 'visible', timeout: 10_000 });
  }
);

When('the learner expands the reasoning details', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const expandable = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.REASONING_EXPANDABLE}"]`);
  await expandable.waitFor({ state: 'visible', timeout: 10_000 });
  await expandable.click();
  await device.page.waitForTimeout(300);
});

Then('the primary constitutional principle is visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const principle = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.REASONING_PRINCIPLE}"]`);
  await principle.waitFor({ state: 'visible', timeout: 10_000 });
  const text = await principle.textContent();
  assert.ok(text && text.trim().length > 0, 'Expected constitutional principle to have content');
});

Then('the interpretation of the principle is visible', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const interpretation = device.page.locator(
    `[data-testid="${ELOHIM_PRESENCE.REASONING_INTERPRETATION}"]`
  );
  await interpretation.waitFor({ state: 'visible', timeout: 10_000 });
  const text = await interpretation.textContent();
  assert.ok(text && text.trim().length > 0, 'Expected interpretation to have content');
});

// ---------------------------------------------------------------------------
// Computation Cost Transparency
// ---------------------------------------------------------------------------

Then(
  'the computation cost is visible showing tokens and processing time',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const tokens = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.COST_TOKENS}"]`);
    const time = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.COST_TIME}"]`);
    await tokens.waitFor({ state: 'visible', timeout: 10_000 });
    await time.waitFor({ state: 'visible', timeout: 10_000 });
  }
);

Then('the insight shows tokens processed', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const tokens = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.COST_TOKENS}"]`);
  await tokens.waitFor({ state: 'visible', timeout: 10_000 });
  const text = await tokens.textContent();
  assert.ok(text && text.trim().length > 0, 'Expected tokens processed to have content');
});

Then('the insight shows processing time in milliseconds', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const time = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.COST_TIME}"]`);
  await time.waitFor({ state: 'visible', timeout: 10_000 });
  const text = await time.textContent();
  assert.ok(text && text.trim().length > 0, 'Expected processing time to have content');
  // Verify it contains a numeric value (milliseconds)
  assert.match(text, /\d/u, `Expected processing time to contain a number, got: "${text}"`);
});

// ---------------------------------------------------------------------------
// Test Connection (Config page)
// ---------------------------------------------------------------------------

When(
  'the learner clicks the {string} button',
  async function (this: E2EWorld, buttonLabel: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    // First try by data-testid for known buttons, then fall back to role
    if (buttonLabel === 'Test Connection') {
      const btn = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.TEST_CONNECTION_BTN}"]`);
      await btn.waitFor({ state: 'visible', timeout: 10_000 });
      await btn.click();
    } else {
      const btn = device.page.getByRole('button', { name: buttonLabel });
      await btn.waitFor({ state: 'visible', timeout: 10_000 });
      await btn.click();
    }
    await device.page.waitForLoadState('networkidle');
  }
);

Then('the test result shows the mock backend is available', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const result = device.page.locator(`[data-testid="${ELOHIM_PRESENCE.TEST_RESULT}"]`);
  await result.waitFor({ state: 'visible', timeout: 15_000 });
  const text = await result.textContent();
  assert.ok(text && text.trim().length > 0, 'Expected test result to have content');
});

Then('a banner notification appears confirming the connection', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const banner = device.page.locator(`[data-testid="${BANNER.NOTIFICATION}"]`);
  await banner.waitFor({ state: 'visible', timeout: 10_000 });
  const text = await banner.textContent();
  assert.ok(text && text.trim().length > 0, 'Expected banner notification to have content');
});
