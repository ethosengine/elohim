/**
 * Common step definitions — health checks, doorway readiness, lifecycle hooks.
 */

import { strict as assert } from 'node:assert';
import { mkdirSync, writeFileSync } from 'node:fs';

import {
  Given,
  Before,
  After,
  AfterAll,
  BeforeAll,
  Status,
  setDefaultTimeout,
  setWorldConstructor,
} from '@cucumber/cucumber';

// Remote doorway calls + retry loops can take longer than the 5s default
setDefaultTimeout(30_000);

import { PlaywrightDevice } from '../src/framework/devices/playwright-device.js';
import { retry } from '../src/framework/utils/retry.js';
import { E2EWorld } from '../src/framework/world.js';

setWorldConstructor(E2EWorld);

/**
 * Capture failure artifacts (screenshot, console errors, trace) for a single device.
 */
async function captureFailureArtifacts(
  device: PlaywrightDevice,
  safeName: string,
  humanName: string
): Promise<void> {
  try {
    await device.screenshot(`FAIL-${safeName}-${humanName}`);
  } catch {
    // best-effort — page may have crashed
  }

  const errors = device.getErrors();
  const hasArtifacts = errors.console.length || errors.page.length || errors.network.length;
  if (hasArtifacts) {
    try {
      writeFileSync(
        `reports/console/${safeName}-${humanName}.json`,
        JSON.stringify(errors, null, 2)
      );
    } catch {
      // best-effort
    }
  }

  try {
    await device.saveTrace(`FAIL-${safeName}-${humanName}`);
  } catch {
    // best-effort
  }
}

/**
 * Log runtime context and create report directories at the start of each test run.
 */
BeforeAll(function () {
  mkdirSync('reports/screenshots', { recursive: true });
  mkdirSync('reports/console', { recursive: true });
  mkdirSync('reports/traces', { recursive: true });

  const env = {
    doorwayAlpha: process.env['E2E_DOORWAY_ALPHA'] ?? '(not set)',
    deviceMode: process.env['E2E_DEVICE_MODE'] ?? 'http',
    trace: process.env['E2E_TRACE'] ?? 'false',
    nodeEnv: process.env['NODE_ENV'] ?? '(not set)',
  };
  // eslint-disable-next-line no-console
  console.log(`\n  Environment: ${JSON.stringify(env, null, 2)}\n`);
});

/**
 * Clear Playwright capture state before each scenario so errors don't bleed across scenarios.
 */
Before(function (this: E2EWorld) {
  for (const [, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        device.clearCapture();
      }
    }
  }
});

/**
 * Background step: verify a doorway is healthy.
 * The URL comes from an environment variable.
 */
Given(
  'doorway {string} is healthy at env {string}',
  async function (this: E2EWorld, doorwayId: string, envVar: string) {
    const url = process.env[envVar];
    assert.ok(url, `Environment variable ${envVar} is not set`);

    const entry = this.addDoorway(doorwayId, url);

    await retry(
      async () => {
        const health = await entry.client.health();
        assert.ok(
          health.healthy,
          `Doorway "${doorwayId}" at ${url} is not healthy: status=${health.status}`
        );
      },
      { maxAttempts: 5, initialDelayMs: 2000, timeoutMs: 30_000 }
    );
  }
);

/**
 * Capture artifacts on failure, then run cleanup after each scenario.
 */
After(async function (this: E2EWorld, scenario) {
  if (scenario.result?.status === Status.FAILED) {
    const safeName = scenario.pickle.name.replace(/[^a-zA-Z0-9]/g, '-');

    for (const [name, human] of this.humans) {
      for (const device of human.devices) {
        if (device instanceof PlaywrightDevice) {
          await captureFailureArtifacts(device, safeName, name);
        }
      }
    }
  }

  await this.runCleanup();
});

/**
 * Close the shared Playwright browser after all scenarios complete.
 */
AfterAll(async function () {
  await E2EWorld.closeBrowser();
});
