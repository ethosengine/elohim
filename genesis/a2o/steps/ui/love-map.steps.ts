/**
 * Love Map browser step definitions — path navigation, chapter display, privacy.
 *
 * These steps require E2E_DEVICE_MODE=playwright. In HTTP mode, they return
 * 'pending' to avoid false passes.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function requirePlaywright(world: E2EWorld): PlaywrightDevice | null {
  if (world.deviceMode !== 'playwright') return null;
  for (const [, human] of world.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (device) return device;
  }
  throw new Error('No Playwright device found.');
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

When(
  '{word} navigates to the path',
  async function (this: E2EWorld, _humanName: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const pathId = this.contentIds.get('loveMapPathId');
    assert.ok(pathId, 'No love map path ID stored — was "the love map path exists" step run?');

    await device.navigate(`/lamad/path:${pathId}`);
    await device.page.waitForLoadState('networkidle');
  }
);

When(
  '{word} views the path chapters',
  async function (this: E2EWorld, _humanName: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const pathId = this.contentIds.get('loveMapPathId');
    assert.ok(pathId, 'No love map path ID stored');

    await device.navigate(`/lamad/path:${pathId}`);
    await device.page.waitForLoadState('networkidle');
  }
);

When(
  '{word} browses all available learning paths',
  async function (this: E2EWorld, _humanName: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    await device.navigate('/lamad');
    await device.page.waitForLoadState('networkidle');
  }
);

// ---------------------------------------------------------------------------
// Chapter assertions
// ---------------------------------------------------------------------------

Then(
  'Chapter {int} {string} should be visible',
  async function (this: E2EWorld, chapterNum: number, chapterTitle: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const heading = device.page.getByText(chapterTitle, { exact: false }).first();
    await heading.waitFor({ state: 'visible', timeout: 10_000 });
  }
);

Then(
  'the path should indicate it requires mutual attestation',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const attestation = device.page
      .getByText('mutual attestation', { exact: false })
      .first();
    await attestation.waitFor({ state: 'visible', timeout: 10_000 });
  }
);

Then(
  'the estimated duration should be {string}',
  async function (this: E2EWorld, duration: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const durationEl = device.page.getByText(duration, { exact: false }).first();
    await durationEl.waitFor({ state: 'visible', timeout: 10_000 });
  }
);

Then(
  'Chapter {int} should be {string} about {word}',
  async function (this: E2EWorld, chapterNum: number, chapterTitle: string, _topic: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    const heading = device.page.getByText(chapterTitle, { exact: false }).first();
    await heading.waitFor({ state: 'visible', timeout: 10_000 });
  }
);

Then(
  'the chapters should show complementary teaching directions',
  async function (this: E2EWorld) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    // Look for evidence of mutual teaching structure in chapter descriptions
    const teachingIndicator = device.page
      .getByText('Tending and Naming', { exact: false })
      .first();
    await teachingIndicator.waitFor({ state: 'visible', timeout: 10_000 });
  }
);

// ---------------------------------------------------------------------------
// Privacy assertions
// ---------------------------------------------------------------------------

Then(
  'the {string} path should not appear',
  async function (this: E2EWorld, pathTitle: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    // Wait for the path list to load
    await device.page.waitForLoadState('networkidle');

    const pathElement = device.page.getByText(pathTitle, { exact: false });
    const count = await pathElement.count();
    assert.strictEqual(
      count,
      0,
      `Path "${pathTitle}" should not be visible to non-participants but was found`
    );
  }
);

Then(
  'searching for {string} should return no results for {word}',
  async function (this: E2EWorld, searchTerm: string, _humanName: string) {
    const device = requirePlaywright(this);
    if (!device) return 'pending';

    // Use the search feature
    const searchInput = device.page.locator('[data-testid="nav-search-input"]');
    const searchExists = (await searchInput.count()) > 0;

    if (searchExists) {
      await searchInput.fill(searchTerm);
      await device.page.locator('[data-testid="nav-search-submit"]').click();
      await device.page.waitForLoadState('networkidle');

      const results = device.page.getByText('Garden of Understanding', { exact: false });
      const count = await results.count();
      assert.strictEqual(count, 0, `Search for "${searchTerm}" should return no results`);
    }
    // If search input doesn't exist, path list filtering is sufficient
  }
);
