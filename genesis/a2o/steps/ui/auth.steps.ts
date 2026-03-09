/**
 * Browser Auth Step Definitions — drive the real login UI via Playwright.
 *
 * The hosted login flow uses an OAuth-style redirect:
 *   1. App navigates to /identity/login
 *   2. App redirects to doorway's /threshold/login (OAuth authorize page)
 *   3. User fills identifier + password on doorway's login form
 *   4. Doorway redirects back to app /auth/callback?code=...
 *   5. App processes callback and navigates to /lamad (or returnUrl)
 *
 * Selectors are managed by page objects in src/framework/pages/.
 * Angular apps expose `data-testid` attributes; page objects consume them.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import {
  PlaywrightDevice,
  type CapturedConsoleLog,
} from '../../src/framework/devices/playwright-device.js';
import { getFixture } from '../../src/framework/fixtures/humans.js';
import { Human } from '../../src/framework/human.js';
import { ThresholdLoginPage, AppShellPage } from '../../src/framework/pages/index.js';
import {
  isSpaRoutingNoise,
  isExpectedNetworkFailure,
} from '../../src/framework/utils/console-filters.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const LOGIN_PATH = '/identity/login';
const THRESHOLD_PATH = '/threshold/login';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function requirePlaywrightDevice(world: E2EWorld, humanName: string): PlaywrightDevice {
  const human = world.getHuman(humanName);
  const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
  assert.ok(device, `${humanName} has no Playwright device. Is E2E_DEVICE_MODE=playwright?`);
  return device;
}

/** Find PlaywrightDevices across all humans and assert on their console errors. */
function assertNoConsoleErrors(world: E2EWorld): void {
  for (const [name, human] of world.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        const consoleErrors: CapturedConsoleLog[] = device.consoleLogs.filter(
          l => l.level === 'error' && !isSpaRoutingNoise(l)
        );
        assert.equal(
          consoleErrors.length,
          0,
          `${name} had ${consoleErrors.length} console error(s):\n` +
            consoleErrors.map(e => `  [${e.level}] ${e.text} (${e.url})`).join('\n')
        );
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Setup: give a human a Playwright browser (no auth injection)
// ---------------------------------------------------------------------------

/**
 * Create a human with a Playwright browser but do NOT inject auth.
 * This is for scenarios that drive the login UI directly.
 *
 * Example: Given human "Matthew" has a browser on doorway "alpha"
 */
Given(
  'human {string} has a browser on doorway {string}',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    const fixture = getFixture(humanName);
    const doorway = this.getDoorway(doorwayId);
    const appUrl = doorwayToAppUrl(doorway.url);

    const human = new Human(humanName, fixture.credentials);
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
    const browser = await this.getBrowser();
    // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
    const device = new PlaywrightDevice(`${humanName}-pw`, appUrl, doorway.url, browser);
    await device.init();
    human.addDevice(device);

    this.addHuman(humanName, human);
    this.onCleanup(async () => device.close());
  }
);

// ---------------------------------------------------------------------------
// Actions: drive the login UI
// ---------------------------------------------------------------------------

/**
 * Full browser login flow via OAuth redirect.
 *
 * The app at alpha.elohim.host redirects to doorway's /threshold/login,
 * where the user fills credentials, then redirects back via /auth/callback.
 *
 * Example: When Matthew logs in through the browser on doorway "alpha"
 */
When(
  '{word} logs in through the browser on doorway {string}',
  async function (this: E2EWorld, humanName: string, _doorwayId: string) {
    const device = requirePlaywrightDevice(this, humanName);
    const human = this.getHuman(humanName);

    // Navigate to app login — this triggers OAuth redirect to doorway
    await device.navigate(LOGIN_PATH);

    // Fill credentials and submit via page object
    const loginPage = new ThresholdLoginPage(device.page);
    await loginPage.login(human.credentials.identifier, human.credentials.password);

    // Wait for redirect back to app (past /threshold and /auth/callback)
    await device.page.waitForURL(
      (url: URL) =>
        !url.pathname.includes(THRESHOLD_PATH) && !url.pathname.includes('/auth/callback'),
      { timeout: 30_000 }
    );
    await device.page.waitForLoadState('networkidle');
  }
);

/**
 * Attempt login with wrong credentials (for error scenario).
 *
 * Example: When Matthew enters wrong credentials in the browser
 */
When(
  '{word} enters wrong credentials in the browser',
  async function (this: E2EWorld, humanName: string) {
    const device = requirePlaywrightDevice(this, humanName);
    const human = this.getHuman(humanName);

    // Navigate to app login — triggers OAuth redirect to doorway
    await device.navigate(LOGIN_PATH);

    // Fill with correct identifier but wrong password via page object
    const loginPage = new ThresholdLoginPage(device.page);
    await loginPage.login(human.credentials.identifier, 'WrongPassword123!');

    // Wait for error to appear (should stay on threshold login page)
    await device.page.waitForTimeout(3000);
  }
);

/**
 * Navigate to a path in the browser.
 *
 * Example: When Matthew navigates to "/lamad" in the browser
 */
When(
  '{word} navigates to {string} in the browser',
  async function (this: E2EWorld, humanName: string, path: string) {
    const device = requirePlaywrightDevice(this, humanName);
    await device.navigate(path);
  }
);

/**
 * Logout through the browser UI.
 *
 * Clicks the profile bubble in the header to open the dropdown menu,
 * then clicks "Log out".
 *
 * Example: When Matthew logs out through the browser UI
 */
When('{word} logs out through the browser UI', async function (this: E2EWorld, humanName: string) {
  const device = requirePlaywrightDevice(this, humanName);
  const shell = new AppShellPage(device.page);
  await shell.logout();
});

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

/**
 * Verify the authenticated shell is displayed (not on login/threshold page).
 *
 * Example: Then Matthew should see the authenticated shell
 */
Then(
  '{word} should see the authenticated shell',
  async function (this: E2EWorld, humanName: string) {
    const device = requirePlaywrightDevice(this, humanName);
    const url = device.page.url();
    assert.ok(
      !url.includes(LOGIN_PATH) && !url.includes(THRESHOLD_PATH),
      `Expected to be past login but still at: ${url}`
    );

    // Verify app shell rendered
    const shell = new AppShellPage(device.page);
    await shell.waitForReady();
  }
);

/**
 * Verify the login page shows an error message.
 *
 * Example: Then the login page should show an error message
 */
Then('the login page should show an error message', async function (this: E2EWorld) {
  for (const [, human] of this.humans) {
    const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
    if (!device) continue;

    // Wait for threshold login error banner via page object
    const loginPage = new ThresholdLoginPage(device.page);
    await loginPage.waitForError();
    return;
  }
  assert.fail('No Playwright device found');
});

/**
 * Verify no console errors were captured (after login variant).
 *
 * Example: Then there should be no console errors after login
 */
Then('there should be no console errors after login', function (this: E2EWorld) {
  assertNoConsoleErrors(this);
});

/**
 * Verify no console errors (generic — works for any scenario).
 *
 * Example: Then there should be no console errors
 */
Then('there should be no console errors', function (this: E2EWorld) {
  assertNoConsoleErrors(this);
});

/**
 * Verify no uncaught JS errors (window.onerror / unhandled rejections).
 *
 * Example: Then there should be no uncaught JS errors
 */
Then('there should be no uncaught JS errors', function (this: E2EWorld) {
  for (const [name, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        assert.equal(
          device.pageErrors.length,
          0,
          `${name} had ${device.pageErrors.length} uncaught JS error(s):\n` +
            device.pageErrors.map(e => `  ${e.message} (${e.url})`).join('\n')
        );
      }
    }
  }
});

/**
 * Verify no unexpected failed network requests.
 * Filters out ERR_ABORTED (SPA navigation) and external resource failures.
 *
 * Example: Then there should be no failed network requests
 */
Then('there should be no failed network requests', function (this: E2EWorld) {
  for (const [name, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        const unexpected = device.failedRequests.filter(r => !isExpectedNetworkFailure(r));
        assert.equal(
          unexpected.length,
          0,
          `${name} had ${unexpected.length} unexpected failed request(s):\n` +
            unexpected.map(r => `  ${r.method} ${r.url} — ${r.failure ?? 'unknown'}`).join('\n')
        );
      }
    }
  }
});

/**
 * Verify redirect to login page after logout.
 *
 * Example: Then Matthew should be redirected to the login page
 */
Then(
  '{word} should be redirected to the login page',
  async function (this: E2EWorld, humanName: string) {
    const device = requirePlaywrightDevice(this, humanName);

    // After logout, the app navigates away from authenticated routes.
    // It may land at the root (/), a login page (/identity/login), or
    // the doorway's threshold page — any of these are valid post-logout states.
    await device.page.waitForURL(
      (url: URL) => !url.pathname.includes('/lamad') && !url.pathname.includes('/content'),
      { timeout: 15_000 }
    );

    const url = device.page.url();
    assert.ok(
      !url.includes('/lamad') && !url.includes('/content'),
      `Expected to leave authenticated routes after logout but at: ${url}`
    );
  }
);
