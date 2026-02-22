/**
 * Common step definitions — health checks, doorway readiness, lifecycle hooks.
 */

import { strict as assert } from 'node:assert';

import { Given, After, AfterAll, setWorldConstructor } from '@cucumber/cucumber';

import { retry } from '../src/framework/utils/retry.js';
import { E2EWorld } from '../src/framework/world.js';

setWorldConstructor(E2EWorld);

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
 * Cleanup after each scenario: remove test content, best-effort.
 */
After(async function (this: E2EWorld) {
  await this.runCleanup();
});

/**
 * Close the shared Playwright browser after all scenarios complete.
 */
AfterAll(async function () {
  await E2EWorld.closeBrowser();
});
