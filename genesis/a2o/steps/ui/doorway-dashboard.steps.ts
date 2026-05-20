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
const NO_PW_DEVICE = 'No Playwright device found';

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
        | 'topology'
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
  assert.fail(NO_PW_DEVICE);
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
// Orchestrator-gated steps (capability fixture not yet pluggable)
// ---------------------------------------------------------------------------

When('no orchestrator is configured', async function (this: E2EWorld) {
  // Precondition assertion — this step does NOT mutate doorway state; it
  // verifies the current deployment has the orchestrator capability disabled.
  // Driving capability state from the test runner requires a fixture this
  // task does not provide (see B4 status report). Until that lands, the
  // owning scenario stays @wip.
  for (const [name, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        const dashboard = new DoorwayDashboardPage(device.page);
        const caps = await dashboard.readCapabilities();
        assert.equal(
          caps.orchestrator,
          false,
          `Expected orchestrator to be disabled on ${name}'s doorway, but it is enabled. ` +
            'This scenario requires a fixture that disables the orchestrator capability.'
        );
        return;
      }
    }
  }
  assert.fail(NO_PW_DEVICE);
});

Then(
  'the nodes tab should show a {string} message',
  async function (this: E2EWorld, expectedMessage: string) {
    for (const [name, human] of this.humans) {
      for (const device of human.devices) {
        if (device instanceof PlaywrightDevice) {
          const dashboard = new DoorwayDashboardPage(device.page);
          assert.ok(
            await dashboard.hasNodesNoOrchestrator(),
            `${name}'s nodes tab is missing the no-orchestrator empty-state`
          );
          const text = await device.page
            .locator('[data-testid="nodes-no-orchestrator"]')
            .textContent();
          const haystack = (text ?? '').toLowerCase();
          assert.ok(
            haystack.includes(expectedMessage.toLowerCase()),
            `Expected nodes tab message to contain "${expectedMessage}" but got "${text ?? ''}"`
          );
          return;
        }
      }
    }
    assert.fail(NO_PW_DEVICE);
  }
);

// ---------------------------------------------------------------------------
// Capability surface steps
// ---------------------------------------------------------------------------

Then(
  'the capabilities badge should indicate which features are enabled',
  async function (this: E2EWorld) {
    for (const [name, human] of this.humans) {
      for (const device of human.devices) {
        if (device instanceof PlaywrightDevice) {
          const dashboard = new DoorwayDashboardPage(device.page);
          assert.ok(
            await dashboard.hasCapabilitiesBadge(),
            `${name}'s dashboard has no capabilities badge`
          );
          const caps = await dashboard.readCapabilities();
          const keys = Object.keys(caps);
          assert.ok(keys.length > 0, `${name}'s capabilities badge surfaced no capability chips`);
          // Every chip must have a deterministic enabled/disabled state — no `null`s
          for (const key of keys) {
            assert.equal(
              typeof caps[key],
              'boolean',
              `Capability ${key} has indeterminate state on ${name}'s dashboard`
            );
          }
          return;
        }
      }
    }
    assert.fail(NO_PW_DEVICE);
  }
);

Then(
  'disabled features should show a placeholder instead of errors',
  async function (this: E2EWorld) {
    for (const [name, human] of this.humans) {
      for (const device of human.devices) {
        if (device instanceof PlaywrightDevice) {
          const dashboard = new DoorwayDashboardPage(device.page);
          const caps = await dashboard.readCapabilities();
          const disabled = Object.entries(caps)
            .filter(([, enabled]) => !enabled)
            .map(([key]) => key);

          // For any capability that is disabled and has a placeholder testid,
          // confirm the placeholder is rendered. Capabilities with no
          // placeholder (e.g. federation) are degraded by hiding their cards
          // without surfacing a stat-card; that's the same "no errors" contract.
          for (const key of disabled) {
            const placeholderTestid = `capability-placeholder-${key}`;
            const placeholderLocator = device.page.locator(`[data-testid="${placeholderTestid}"]`);
            const present = (await placeholderLocator.count()) > 0;
            if (present) {
              assert.ok(
                await placeholderLocator.first().isVisible(),
                `${name}'s dashboard hid capability-placeholder-${key} despite ${key} being disabled`
              );
            }
          }

          // Independent check: no console errors leaked while rendering disabled features
          const consoleErrors = device.consoleLogs.filter(
            l => l.level === 'error' && !isSpaRoutingNoise(l)
          );
          assert.equal(
            consoleErrors.length,
            0,
            `${name}'s dashboard logged errors instead of showing placeholders:\n` +
              consoleErrors.map(e => `  ${e.text}`).join('\n')
          );
          return;
        }
      }
    }
    assert.fail(NO_PW_DEVICE);
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
