/**
 * Learning Journey Step Definitions — Playwright-powered lamad flows.
 *
 * Ported from elohim-app/cypress/e2e/step_definitions/lamad/learning_journey_steps.ts
 * These steps require E2E_DEVICE_MODE=playwright. In HTTP mode, browser-only
 * assertions return 'pending' to avoid false passes.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
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
// Background / Setup
// ---------------------------------------------------------------------------

Given('I am a new traveler {string}', async function (this: E2EWorld, _name: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.navigate('/lamad');
  await device.page.evaluate(() => localStorage.clear());
  await device.page.reload({ waitUntil: 'networkidle' });
});

Given('the {string} path exists', async function (this: E2EWorld, pathName: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const pathElement = device.page.getByText(pathName, { exact: false }).first();
  await pathElement.waitFor({ state: 'visible', timeout: 10_000 });
});

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

When('I start the {string} path', async function (this: E2EWorld, pathName: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.page.getByText(pathName, { exact: false }).first().click();
  await device.page.waitForLoadState('networkidle');
});

Given(
  'I am on step {int} {string}',
  async function (this: E2EWorld, stepIndex: number, _stepTitle: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    await device.navigate(`/lamad/path:default-elohim-protocol/step:${stepIndex}`);
  }
);

When('I try to access step {int}', async function (this: E2EWorld, stepIndex: number) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.navigate(`/lamad/path:default-elohim-protocol/step:${stepIndex}`);
});

When('I read the content', async function (this: E2EWorld) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  await device.page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
  await device.page.waitForLoadState('networkidle');
});

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

Then(
  'I should be on step {int} {string}',
  async function (this: E2EWorld, stepIndex: number, stepTitle: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    const url = device.page.url();
    assert.ok(
      url.includes(`/step:${stepIndex}`),
      `Expected URL to include /step:${stepIndex}, got: ${url}`
    );
    const heading = device.page.locator('h1').getByText(stepTitle, { exact: false });
    await heading.waitFor({ state: 'visible', timeout: 10_000 });
  }
);

Then(
  'the content for {string} should be visible',
  async function (this: E2EWorld, stepTitle: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    const content = device.page.locator('.content-body, main');
    await content.first().waitFor({ state: 'visible' });
    const text = device.page.getByText(stepTitle, { exact: false });
    await text.first().waitFor({ state: 'visible' });
  }
);

Then(
  'step {int} {string} should be visible as a preview',
  async function (this: E2EWorld, stepIndex: number, stepTitle: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    const stepItem = device.page.locator(`.step-list-item[data-index="${stepIndex}"]`);
    await stepItem.waitFor({ state: 'visible', timeout: 5_000 });
    const text = await stepItem.textContent();
    assert.ok(text?.includes(stepTitle), `Expected step ${stepIndex} to contain "${stepTitle}"`);
  }
);

Then(
  String.raw`step {int} {string} should be hidden \(Fog of War\)`,
  async function (this: E2EWorld, stepIndex: number, _stepTitle: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    const stepItem = device.page.locator(`.step-list-item[data-index="${stepIndex}"]`);
    const count = await stepItem.count();
    assert.equal(
      count,
      0,
      `Expected step ${stepIndex} to be hidden (Fog of War) but it was visible`
    );
  }
);

Then(
  'my affinity for {string} should increase',
  async function (this: E2EWorld, _nodeTitle: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    const affinityCircle = device.page.locator('.affinity-circle');
    const text = await affinityCircle.first().textContent();
    assert.ok(text && !text.includes('0%'), 'Expected affinity to increase from 0%');
  }
);

Then('my progress on {string} should update', async function (this: E2EWorld, _pathName: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const progress = device.page.locator('.path-progress');
  await progress.first().waitFor({ state: 'visible' });
});

// ---------------------------------------------------------------------------
// Attestations
// ---------------------------------------------------------------------------

Given(
  'step {int} {string} requires the {string} attestation',
  function (this: E2EWorld, _stepIndex: number, _stepTitle: string, _attestationName: string) {
    if (this.deviceMode !== 'playwright') return 'pending';
  }
);

Given(
  'I do not have the {string} attestation',
  function (this: E2EWorld, _attestationName: string) {
    if (this.deviceMode !== 'playwright') return 'pending';
  }
);

Then('I should see a {string} message', async function (this: E2EWorld, message: string) {
  const device = requirePlaywright(this);
  if (!device) return 'pending';
  const element = device.page.getByText(message, { exact: false }).first();
  await element.waitFor({ state: 'visible', timeout: 10_000 });
});

Then(
  'I should be guided to earn {string}',
  async function (this: E2EWorld, attestationName: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';
    const guideText = device.page.getByText(`Earn ${attestationName}`, { exact: false });
    await guideText.first().waitFor({ state: 'visible', timeout: 10_000 });
  }
);
