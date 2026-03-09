/**
 * Discovery Assessment Step Definitions — Playwright-powered discovery flows.
 *
 * These steps require E2E_DEVICE_MODE=playwright. In HTTP mode, browser-only
 * assertions return 'pending' to avoid false passes.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { ASSESSMENT, SOPHIA, LIKERT, DISCOVERY } from '../../src/framework/pages/selectors.js';
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
// Navigation
// ---------------------------------------------------------------------------

When('I navigate to the {string} path', async function (this: E2EWorld, pathName: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.navigate('/lamad');
  await device.page.waitForLoadState('networkidle');
  const pathLink = device.page.getByText(pathName, { exact: false }).first();
  await pathLink.waitFor({ state: 'visible', timeout: 10_000 });
  await pathLink.click();
  await device.page.waitForLoadState('networkidle');
});

When(
  'I advance to the {string} assessment step',
  async function (this: E2EWorld, stepName: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    // Click through sidebar steps until we find the assessment
    const stepLink = device.page.getByText(stepName, { exact: false }).first();
    await stepLink.waitFor({ state: 'visible', timeout: 15_000 });
    await stepLink.click();
    await device.page.waitForLoadState('networkidle');
  }
);

When('I start the assessment', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const startBtn = device.page.locator(`[data-testid="${ASSESSMENT.START}"]`);
  await startBtn.waitFor({ state: 'visible', timeout: 10_000 });
  await startBtn.click();
  await device.page.waitForLoadState('networkidle');
});

/** Click a likert-scale tick mark for the given value. */
async function clickLikertValue(device: PlaywrightDevice, value: number): Promise<void> {
  const tick = device.page
    .locator(
      `[data-testid="${LIKERT.TICK}"][data-value="${value}"], ` +
        `[data-testid="${LIKERT.THUMB}"][data-value="${value}"]`
    )
    .first();

  if (await tick.isVisible().catch(() => false)) {
    await tick.click();
    return;
  }

  // Fallback: click the nth tick mark by position
  const ticks = device.page.locator(
    `[data-testid="${LIKERT.TICK}"], [data-testid="${LIKERT.THUMB}"]`
  );
  const count = await ticks.count();
  if (count > 0) {
    const idx = Math.min(value - 1, count - 1);
    await ticks.nth(idx).click();
  }
}

/** Click a radio option at the given index. */
async function clickRadioChoice(device: PlaywrightDevice, choiceIndex: number): Promise<void> {
  const choices = device.page.locator('[role="radio"]');
  const count = await choices.count();
  if (count > 0) {
    const idx = Math.min(choiceIndex, count - 1);
    await choices.nth(idx).click();
  }
}

When('I answer all likert-scale questions with varied selections', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  let questionIndex = 0;
  const values = [3, 5, 2, 6, 4, 7, 1]; // Varied selections cycling

  // Keep answering until no more questions
  while (true) {
    const likertTrack = device.page.locator(`[data-testid="${LIKERT.TRACK}"]`).first();
    const radioGroup = device.page.locator('[role="radiogroup"]').first();

    const hasLikert = await likertTrack.isVisible().catch(() => false);
    const hasRadio = !hasLikert && (await radioGroup.isVisible().catch(() => false));
    const value = values[questionIndex % values.length];

    if (hasLikert) {
      await clickLikertValue(device, value);
    } else if (hasRadio) {
      await clickRadioChoice(device, value - 1);
    } else {
      break; // No more interactive widgets
    }

    questionIndex++;

    // Click continue/next to advance
    const continueBtn = device.page.locator(`[data-testid="${SOPHIA.CONTINUE}"]`).first();
    if (await continueBtn.isVisible().catch(() => false)) {
      await continueBtn.click();
      await device.page.waitForTimeout(500);
    } else {
      break;
    }
  }
});

When(
  'I select value {string} on the first likert-scale question',
  async function (this: E2EWorld, value: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const tick = device.page
      .locator(
        `[data-testid="${LIKERT.TICK}"][data-value="${value}"], ` +
          `[data-testid="${LIKERT.THUMB}"][data-value="${value}"]`
      )
      .first();
    await tick.waitFor({ state: 'visible', timeout: 10_000 });
    await tick.click();
  }
);

When('I advance through each chapter', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  // Click through all visible path steps
  const nextBtn = device.page.locator('[data-testid="path-nav-next"]');
  let maxSteps = 20;
  while (maxSteps-- > 0) {
    if (await nextBtn.isVisible().catch(() => false)) {
      await nextBtn.click();
      await device.page.waitForLoadState('networkidle');
    } else {
      break;
    }
  }
});

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

Then('only one selection should be active', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const selected = device.page.locator(`[data-testid="${LIKERT.THUMB}"], [aria-checked="true"]`);
  const count = await selected.count();
  assert.equal(count, 1, `Expected 1 selected item, found ${count}`);
});

Then('no other options should appear selected', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const selected = device.page.locator('[aria-checked="true"]');
  const count = await selected.count();
  assert.ok(count <= 1, `Expected at most 1 selected option, found ${count}`);
});

Then('the assessment should complete without console errors', function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const errors = device.getErrors();
  assert.equal(
    errors.page.length,
    0,
    `Expected no page errors, found: ${errors.page.map(e => e.message).join(', ')}`
  );
});

Then('the resonance result should have non-zero subscale scores', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  // Check for discovery profile or subscale scores in DOM
  const subscaleScores = device.page.locator(`[data-testid="${DISCOVERY.SUBSCALE_SCORE}"]`);
  const count = await subscaleScores.count();
  assert.ok(count > 0, 'Expected at least one subscale score displayed');

  // Verify at least one is non-zero
  for (let i = 0; i < count; i++) {
    const nthScore = subscaleScores as unknown as {
      nth(index: number): { textContent(): Promise<string | null> };
    };
    const text = await nthScore.nth(i).textContent();
    if (text && Number.parseFloat(text) > 0) return; // At least one non-zero
  }
  assert.fail('All subscale scores are zero');
});

Then(
  'the {string} attestation should be recorded',
  async function (this: E2EWorld, attestationId: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const stored = await device.page.evaluate((id: string) => {
      const attestations = localStorage.getItem('discovery-attestations');
      if (!attestations) return false;
      try {
        const parsed = JSON.parse(attestations);
        return Array.isArray(parsed) ? parsed.includes(id) : id in parsed;
      } catch {
        return false;
      }
    }, attestationId);

    assert.ok(stored, `Expected attestation "${attestationId}" to be recorded in localStorage`);
  }
);

Then('I should see my values profile breakdown', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const profile = device.page.locator(`[data-testid="${DISCOVERY.PROFILE}"]`);
  await profile.waitFor({ state: 'visible', timeout: 10_000 });
});

// ---------------------------------------------------------------------------
// Attestation milestone
// ---------------------------------------------------------------------------

Given(
  '{word} has not completed any discovery assessments',
  async function (this: E2EWorld, _humanName: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    // Clear any prior discovery attestations so the "first-discovery" badge can fire
    await device.page.evaluate(() => {
      localStorage.removeItem('discovery-attestations');
      localStorage.removeItem('discovery-sessions');
    });
  }
);

Then(
  'I should see the {string} attestation badge on my profile',
  async function (this: E2EWorld, attestationId: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    // Navigate to profile to verify badge display
    await device.navigate('/imagodei/profile');
    await device.page.waitForLoadState('networkidle');

    const badge = device.page.locator(
      `[data-testid="${DISCOVERY.ATTESTATION_BADGE}"][data-attestation="${attestationId}"]`
    );
    await badge.waitFor({ state: 'visible', timeout: 10_000 });
  }
);

// ---------------------------------------------------------------------------
// Error capture
// ---------------------------------------------------------------------------

Then('no JavaScript errors should be captured', function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const errors = device.getErrors();
  assert.equal(
    errors.page.length,
    0,
    `JavaScript errors found: ${errors.page.map(e => e.message).join('; ')}`
  );
});

Then('no failed network requests should be captured', function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';

  const errors = device.getErrors();
  assert.equal(
    errors.network.length,
    0,
    `Failed network requests: ${errors.network.map(e => e.url).join('; ')}`
  );
});
