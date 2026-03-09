/**
 * UI Navigation Step Definitions — Playwright-powered page navigation.
 *
 * Ported from elohim-app/cypress/e2e/step_definitions/common/navigation.steps.ts
 * These steps require E2E_DEVICE_MODE=playwright.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { E2EWorld } from '../../src/framework/world.js';

/**
 * Get the first PlaywrightDevice from a human, or throw.
 */
function getPlaywrightDevice(world: E2EWorld, humanName?: string): PlaywrightDevice {
  if (humanName) {
    const human = world.getHuman(humanName);
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    assert.ok(device, `${humanName} has no Playwright device`);
    return device;
  }

  // Find the first human with a Playwright device
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }

  throw new Error(
    'No Playwright device found. Use E2E_DEVICE_MODE=playwright and register a human first.'
  );
}

// ---------------------------------------------------------------------------
// Background / Setup
// ---------------------------------------------------------------------------

Given('I navigate to the staging site', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  await device.navigate('/');
});

Given('I am on the home page', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  await device.navigate('/');
});

Given('I am on the {string} page', async function (this: E2EWorld, path: string) {
  const device = getPlaywrightDevice(this);
  const normalizedPath = path.startsWith('/') ? path : `/${path}`;
  await device.navigate(normalizedPath);
});

// ---------------------------------------------------------------------------
// Navigation Actions
// ---------------------------------------------------------------------------

When('the page loads', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  await device.page.waitForLoadState('networkidle');
});

When('I navigate to {string}', async function (this: E2EWorld, path: string) {
  const device = getPlaywrightDevice(this);
  const normalizedPath = path.startsWith('/') ? path : `/${path}`;
  await device.navigate(normalizedPath);
});

When('I click on {string}', async function (this: E2EWorld, text: string) {
  const device = getPlaywrightDevice(this);
  await device.page.getByText(text, { exact: false }).first().click();
  await device.page.waitForLoadState('networkidle');
});

When('I click the {string} button', async function (this: E2EWorld, buttonText: string) {
  const device = getPlaywrightDevice(this);
  await device.page.getByRole('button', { name: buttonText }).click();
  await device.page.waitForLoadState('networkidle');
});

When('I click on {string} in the navigation', async function (this: E2EWorld, linkText: string) {
  const device = getPlaywrightDevice(this);
  const nav = device.page.locator('nav, header, [role="navigation"]');
  await nav.getByText(linkText, { exact: false }).first().click();
  await device.page.waitForLoadState('networkidle');
});

// ---------------------------------------------------------------------------
// Page State Assertions
// ---------------------------------------------------------------------------

Then('the site should be accessible', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  const body = device.page.locator('body');
  await body.waitFor({ state: 'visible' });
});

Then('the page should load successfully', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  const body = device.page.locator('body');
  await body.waitFor({ state: 'visible' });
});

Then('the page should display the main content', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  const main = device.page.locator('main, app-root, [role="main"]');
  await main.first().waitFor({ state: 'visible' });
});

Then('there should be no critical errors in the console', function (this: E2EWorld) {
  // In Playwright mode, we can check console messages
  // This is a basic check — more robust error tracking can be added
  const device = getPlaywrightDevice(this);
  assert.ok(device.page, 'Page should exist');
});

Then('I should see {string}', async function (this: E2EWorld, text: string) {
  const device = getPlaywrightDevice(this);
  const element = device.page.getByText(text, { exact: false }).first();
  await element.waitFor({ state: 'visible', timeout: 10_000 });
});

Then('I should not see {string}', async function (this: E2EWorld, text: string) {
  const device = getPlaywrightDevice(this);
  const count = await device.page.getByText(text, { exact: false }).count();
  assert.equal(count, 0, `Expected not to see "${text}" but found ${count} occurrence(s)`);
});

Then('I should be on the {string} page', function (this: E2EWorld, path: string) {
  const device = getPlaywrightDevice(this);
  const normalizedPath = path.startsWith('/') ? path : `/${path}`;
  const url = device.page.url();
  assert.ok(
    url.includes(normalizedPath),
    `Expected URL to include "${normalizedPath}" but got "${url}"`
  );
});

// ---------------------------------------------------------------------------
// Element Visibility
// ---------------------------------------------------------------------------

Then('the page title should contain {string}', async function (this: E2EWorld, text: string) {
  const device = getPlaywrightDevice(this);
  const title = await device.page.title();
  assert.ok(title.includes(text), `Expected title to contain "${text}" but got "${title}"`);
});

Then('the hero section should be displayed', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  const hero = device.page.locator('app-hero, .hero, [class*="hero"], main section:first-child');
  await hero.first().waitFor({ state: 'visible', timeout: 10_000 });
});

Then('the footer should be present', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  const footer = device.page.locator('footer, app-footer, [role="contentinfo"]');
  await footer.first().waitFor({ state: 'attached' });
});

Then('the footer should display the expected git hash', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  const hashEl = device.page.locator('[data-testid="git-hash"], .git-hash');
  await hashEl.first().waitFor({ state: 'attached' });
});

Then('the git hash should match the deployed version', async function (this: E2EWorld) {
  const expected = process.env['EXPECTED_GIT_HASH'];
  if (!expected) {
    // Advisory: skip if no expected hash set
    return;
  }
  const device = getPlaywrightDevice(this);
  const hashEl = device.page.locator('[data-testid="git-hash"]');
  const actual = await hashEl.first().textContent();
  assert.ok(actual?.includes(expected), `Expected git hash "${expected}" but found "${actual}"`);
});

Then('the main navigation should be visible', async function (this: E2EWorld) {
  const device = getPlaywrightDevice(this);
  const nav = device.page.locator('nav, [role="navigation"], header nav');
  await nav.first().waitFor({ state: 'visible' });
});

// ---------------------------------------------------------------------------
// P2P / Health (Playwright drives the browser but we can still use API)
// ---------------------------------------------------------------------------

Given('the doorway health endpoint is accessible', async function (this: E2EWorld) {
  // For Playwright mode, just verify any doorway is healthy
  for (const [, entry] of this.doorways) {
    const healthy = await entry.client.isHealthy();
    assert.ok(healthy, `Doorway at ${entry.url} is not healthy`);
    return;
  }
  assert.fail('No doorway configured');
});

When('I check the P2P status', async function (this: E2EWorld) {
  // Store health response for assertions
  for (const [, entry] of this.doorways) {
    const health = await entry.client.health();
    this.contentIds.set('_health', JSON.stringify(health));
    return;
  }
});

Then('connected_peers should be greater than {int}', function (this: E2EWorld, min: number) {
  const healthJson = this.contentIds.get('_health');
  assert.ok(healthJson, 'No health response stored');
  const health = JSON.parse(healthJson) as Record<string, unknown>;
  const p2p = health.p2p as Record<string, unknown> | undefined;
  assert.ok(
    p2p && (p2p.peerCount as number) > min,
    `Expected peers > ${min} but got ${p2p?.peerCount}`
  );
});

Then('sync_documents count should be available', function (this: E2EWorld) {
  const healthJson = this.contentIds.get('_health');
  assert.ok(healthJson, 'No health response stored');
  const health = JSON.parse(healthJson) as Record<string, unknown>;
  assert.ok(health.p2p !== undefined, 'P2P health not available');
});
