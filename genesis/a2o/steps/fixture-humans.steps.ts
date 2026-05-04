/**
 * Fixture human step definitions — login as pre-seeded humans from humans.json.
 *
 * Naming convention:
 *   "is logged in"  = fixture human (pre-seeded by genesis/seeder/src/seed-humans.ts)
 *   "is on doorway"  = ephemeral human (registered on-the-fly, see mode-aware.steps.ts)
 *
 * Credentials are derived deterministically from humans.json by
 * genesis/a2o/src/framework/fixtures/humans.ts (same algorithm as the seeder).
 */

import { Given } from '@cucumber/cucumber';

import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { PlaywrightDevice } from '../src/framework/devices/playwright-device.js';
import { getFixture, isHumanDeployed } from '../src/framework/fixtures/humans.js';
import { Human } from '../src/framework/human.js';
import { DoorwayDashboardPage } from '../src/framework/pages/index.js';
import { doorwayToAppUrl } from '../src/framework/utils/url.js';
import { E2EWorld } from '../src/framework/world.js';

/**
 * Login a pre-seeded fixture human via HTTP (BrowserDevice).
 *
 * Example:
 *   Given human "Matthew" is logged in on doorway "alpha"
 */
Given(
  'human {string} is logged in on doorway {string}',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    if (!isHumanDeployed(humanName)) {
      // Conductor suspended in deployments.json (e.g. shem outage). The
      // human's account may not be seeded; skip the scenario rather than
      // fail. Toggle "suspended": false in deployments.json to re-enable.
      return 'pending';
    }
    const fixture = getFixture(humanName);
    const doorway = this.getDoorway(doorwayId);

    const human = new Human(humanName, fixture.credentials);
    const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
    human.addDevice(device);

    const auth = await device.login({
      identifier: fixture.credentials.identifier,
      password: fixture.credentials.password,
    });

    human.agentPubKey = auth.agentPubKey;
    human.humanId = auth.humanId;
    human.setToken(doorwayId, auth.token);

    this.addHuman(humanName, human);
  }
);

/**
 * Login a pre-seeded fixture human with a mode-aware device (HTTP or Playwright).
 *
 * Example:
 *   Given human "Matthew" is logged in on doorway "alpha" with device
 */
Given(
  'human {string} is logged in on doorway {string} with device',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    if (!isHumanDeployed(humanName)) {
      return 'pending';
    }
    const fixture = getFixture(humanName);
    const doorway = this.getDoorway(doorwayId);

    const human = new Human(humanName, fixture.credentials);

    if (this.deviceMode === 'playwright') {
      const browser = await this.getBrowser();
      const appUrl = doorwayToAppUrl(doorway.url);
      const device = new PlaywrightDevice(`${humanName}-pw`, appUrl, doorway.url, browser);
      await device.init();
      human.addDevice(device);

      const auth = await device.login({
        identifier: fixture.credentials.identifier,
        password: fixture.credentials.password,
      });

      human.agentPubKey = auth.agentPubKey;
      human.humanId = auth.humanId;
      human.setToken(doorwayId, auth.token);

      this.onCleanup(async () => device.close());
    } else {
      const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
      human.addDevice(device);

      const auth = await device.login({
        identifier: fixture.credentials.identifier,
        password: fixture.credentials.password,
      });

      human.agentPubKey = auth.agentPubKey;
      human.humanId = auth.humanId;
      human.setToken(doorwayId, auth.token);
    }

    this.addHuman(humanName, human);
  }
);

/**
 * Login a pre-seeded fixture human on doorway-app with Playwright.
 * Injects auth via API and navigates to /threshold/dashboard instead of the elohim-app.
 *
 * Example:
 *   Given human "Matthew" is logged in on doorway-app "alpha" with device
 */
Given(
  'human {string} is logged in on doorway-app {string} with device',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    if (!isHumanDeployed(humanName)) {
      return 'pending';
    }
    const fixture = getFixture(humanName);
    const doorway = this.getDoorway(doorwayId);

    const human = new Human(humanName, fixture.credentials);

    if (this.deviceMode === 'playwright') {
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      const browser = await this.getBrowser();
      // doorway-app is served from the doorway URL itself (not the elohim-app URL)
      // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
      const device = new PlaywrightDevice(`${humanName}-pw`, doorway.url, doorway.url, browser);

      // Login via API first to get a valid JWT
      const auth = await device.client.login({
        identifier: fixture.credentials.identifier,
        password: fixture.credentials.password,
      });
      device.client.setToken(auth.token);

      // Init with extra HTTP headers — injects Authorization on every request.
      // This works even if the Angular auth interceptor isn't deployed yet.
      await device.init({
        extraHTTPHeaders: { Authorization: `Bearer ${auth.token}` },
      });
      human.addDevice(device);

      // Also set localStorage so Angular reads the token when the interceptor IS deployed
      await device.preloadLocalStorage('doorway_auth_token', auth.token);

      human.agentPubKey = auth.agentPubKey;
      human.humanId = auth.humanId;
      human.setToken(doorwayId, auth.token);

      // Navigate — auth is injected at the network level
      await device.page.goto(`${doorway.url}/threshold/dashboard`, { waitUntil: 'networkidle' });
      const dashboard = new DoorwayDashboardPage(device.page);
      await dashboard.waitForReady();

      this.onCleanup(async () => device.close());
    } else {
      const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
      human.addDevice(device);

      const auth = await device.login({
        identifier: fixture.credentials.identifier,
        password: fixture.credentials.password,
      });

      human.agentPubKey = auth.agentPubKey;
      human.humanId = auth.humanId;
      human.setToken(doorwayId, auth.token);
    }

    this.addHuman(humanName, human);
  }
);
