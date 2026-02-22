/**
 * Mode-aware step definitions — switch between HTTP and Playwright transports.
 *
 * E2E_DEVICE_MODE=http      (default) uses BrowserDevice (undici HTTP client)
 * E2E_DEVICE_MODE=playwright         uses PlaywrightDevice (real Chromium)
 *
 * Same Cucumber scenario, different transport underneath.
 */

import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';

import { Given } from '@cucumber/cucumber';

import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { PlaywrightDevice } from '../src/framework/devices/playwright-device.js';
import { Human } from '../src/framework/human.js';
import { E2EWorld } from '../src/framework/world.js';

/**
 * Resolve the app URL from a doorway URL.
 * doorway-alpha.elohim.host -> alpha.elohim.host
 * doorway-staging.elohim.host -> staging.elohim.host
 * doorway.elohim.host -> elohim.host
 * localhost:8888 -> localhost:4200
 */
function doorwayToAppUrl(doorwayUrl: string): string {
  if (doorwayUrl.includes('localhost:8888')) return 'http://localhost:4200';
  return doorwayUrl
    .replace('doorway-alpha.', 'alpha.')
    .replace('doorway-staging.', 'staging.')
    .replace('doorway.', '');
}

/**
 * Register a human with a device appropriate for the current mode.
 * In HTTP mode: BrowserDevice (API-only, fast).
 * In Playwright mode: PlaywrightDevice (real browser, full UI).
 */
Given(
  'human {string} is on doorway {string} with device',
  async function (this: E2EWorld, humanName: string, doorwayId: string) {
    const doorway = this.getDoorway(doorwayId);
    const runId = randomUUID().slice(0, 8);
    const creds = {
      identifier: `e2e-${humanName.toLowerCase()}-${runId}@test.elohim.host`,
      password: `E2ePass!${runId}`,
      displayName: `${humanName} (E2E ${runId})`,
    };
    const human = new Human(humanName, creds);

    if (this.deviceMode === 'playwright') {
      const browser = await this.getBrowser();
      const appUrl = doorwayToAppUrl(doorway.url);
      const device = new PlaywrightDevice(`${humanName}-pw`, appUrl, doorway.url, browser);
      await device.init();
      human.addDevice(device);

      const auth = await device.register({
        identifier: creds.identifier,
        password: creds.password,
        displayName: creds.displayName,
      });

      human.agentPubKey = auth.agentPubKey;
      human.humanId = auth.humanId;
      human.setToken(doorwayId, auth.token);

      this.onCleanup(async () => device.close());
    } else {
      const device = new BrowserDevice(`${humanName}-browser`, doorway.url);
      human.addDevice(device);

      const auth = await device.register({
        identifier: creds.identifier,
        password: creds.password,
        displayName: creds.displayName,
      });

      human.agentPubKey = auth.agentPubKey;
      human.humanId = auth.humanId;
      human.setToken(doorwayId, auth.token);
    }

    this.addHuman(humanName, human);
  }
);

/**
 * Background step: set up a doorway from env var, supporting both URL formats.
 * Accepts a direct URL or env var name.
 */
Given(
  'doorway {string} at {string}',
  function (this: E2EWorld, doorwayId: string, urlOrEnv: string) {
    const url = process.env[urlOrEnv] ?? urlOrEnv;
    assert.ok(url, `Cannot resolve doorway URL from: ${urlOrEnv}`);
    this.addDoorway(doorwayId, url);
  }
);
