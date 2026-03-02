/**
 * Doorway Dashboard Browser Step Definitions — drive the admin dashboard via Playwright.
 *
 * These steps operate on doorway-app's `/threshold/dashboard` page, using
 * page objects that target `data-testid` selectors. URL routing is owned
 * by step definitions; page objects are pure DOM interaction.
 */

import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import {
  DoorwayDashboardPage,
  DoorwayToolbarPage,
  UsersTabPage,
} from '../../src/framework/pages/index.js';
import { isSpaRoutingNoise } from '../../src/framework/utils/console-filters.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const DASHBOARD_PATH = '/threshold/dashboard';

function requirePwDevice(world: E2EWorld, humanName: string): PlaywrightDevice {
  const human = world.getHuman(humanName);
  const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
  assert.ok(device, `${humanName} has no Playwright device. Is E2E_DEVICE_MODE=playwright?`);
  return device;
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

When('{word} opens the doorway dashboard', async function (this: E2EWorld, humanName: string) {
  const device = requirePwDevice(this, humanName);
  // Navigate to the doorway-app dashboard (not the elohim-app)
  const doorwayUrl = device.client.url;
  await device.page.goto(`${doorwayUrl}${DASHBOARD_PATH}`, { waitUntil: 'networkidle' });
  const dashboard = new DoorwayDashboardPage(device.page);
  await dashboard.waitForReady();
});

When(
  '{word} opens the {word} tab',
  async function (this: E2EWorld, humanName: string, tabName: string) {
    const device = requirePwDevice(this, humanName);
    const dashboard = new DoorwayDashboardPage(device.page);
    await dashboard.switchTab(
      tabName as
        | 'overview'
        | 'nodes'
        | 'users'
        | 'resources'
        | 'federation'
        | 'pipeline'
        | 'graduation'
    );
  }
);

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

Then('the dashboard should render without JavaScript errors', function (this: E2EWorld) {
  for (const [name, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        const consoleErrors = device.consoleLogs.filter(
          l => l.level === 'error' && !isSpaRoutingNoise(l)
        );
        const pageErrors = device.pageErrors;
        assert.equal(
          consoleErrors.length + pageErrors.length,
          0,
          `${name} had browser errors:\n` +
            consoleErrors.map(e => `  [console] ${e.text}`).join('\n') +
            pageErrors.map(e => `  [uncaught] ${e.message}`).join('\n')
        );
      }
    }
  }
});

Then('the overview tab should be active', async function (this: E2EWorld) {
  for (const [, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        const dashboard = new DoorwayDashboardPage(device.page);
        const tab = await dashboard.activeTab();
        assert.strictEqual(tab, 'overview', `Expected overview tab but got: ${tab}`);
        return;
      }
    }
  }
  assert.fail('No Playwright device found');
});

Then('no unexpected console errors should be logged', function (this: E2EWorld) {
  for (const [name, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        const consoleErrors = device.consoleLogs.filter(
          l => l.level === 'error' && !isSpaRoutingNoise(l)
        );
        assert.equal(
          consoleErrors.length,
          0,
          `${name} had ${consoleErrors.length} unexpected console error(s):\n` +
            consoleErrors.map(e => `  ${e.text} (${e.url})`).join('\n')
        );
      }
    }
  }
});

// ---------------------------------------------------------------------------
// Users tab steps
// ---------------------------------------------------------------------------

When(
  '{word} searches for user {string}',
  async function (this: E2EWorld, humanName: string, query: string) {
    const device = requirePwDevice(this, humanName);
    const usersTab = new UsersTabPage(device.page);
    await usersTab.searchUser(query);
  }
);

When(
  '{word} opens user detail for {word}',
  async function (this: E2EWorld, humanName: string, identifier: string) {
    const device = requirePwDevice(this, humanName);
    const usersTab = new UsersTabPage(device.page);
    await usersTab.openUserDetail(identifier);
  }
);

// ---------------------------------------------------------------------------
// Toolbar steps
// ---------------------------------------------------------------------------

When('{word} logs out via the toolbar', async function (this: E2EWorld, humanName: string) {
  const device = requirePwDevice(this, humanName);
  const toolbar = new DoorwayToolbarPage(device.page);
  await toolbar.waitForReady();
  await toolbar.logout();
});
