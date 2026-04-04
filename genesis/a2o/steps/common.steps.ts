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

import {
  PlaywrightDevice,
  type CapturedConsoleLog,
  type CapturedPageError,
} from '../src/framework/devices/playwright-device.js';
import {
  isTestnetActive,
  stopTestnet,
  getComputeSummary,
  writeComputeReport,
} from '../src/framework/testnet-manager.js';
import { isSpaRoutingNoise } from '../src/framework/utils/console-filters.js';
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

/** Collect errors from a single Playwright device and save artifact. */
function collectDeviceErrors(
  device: PlaywrightDevice,
  safeName: string,
  humanName: string
): string[] {
  const consoleErrors: CapturedConsoleLog[] = device.consoleLogs.filter(
    l => l.level === 'error' && !isSpaRoutingNoise(l)
  );
  const pageErrors: CapturedPageError[] = device.pageErrors;

  if (!consoleErrors.length && !pageErrors.length) return [];

  try {
    writeFileSync(
      `reports/console/${safeName}-${humanName}-errors.json`,
      JSON.stringify({ consoleErrors, pageErrors }, null, 2)
    );
  } catch {
    // best-effort
  }

  return [
    ...consoleErrors.map(e => `[${humanName}] console.error: ${e.text} (${e.url})`),
    ...pageErrors.map(e => `[${humanName}] uncaught: ${e.message} (${e.url})`),
  ];
}

/** Collect browser errors from all Playwright devices in the world. */
function collectBrowserErrors(world: E2EWorld, scenarioName: string): string[] {
  const safeName = scenarioName.replace(/[^a-zA-Z0-9]/g, '-');
  const errorReport: string[] = [];

  for (const [name, human] of world.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        errorReport.push(...collectDeviceErrors(device, safeName, name));
      }
    }
  }

  return errorReport;
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
 * Also begins an observation session on the first registered doorway (best-effort).
 */
Before(async function (this: E2EWorld, scenario) {
  for (const [, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        device.clearCapture();
      }
    }
  }

  // Start observation session if a doorway is registered
  for (const [, doorway] of this.doorways) {
    try {
      const featureSlug = scenario.pickle.uri
        .replace(/^.*features\//, '')
        .replace(/\.feature$/, '')
        .replace(/\//g, '-');
      const scenarioSlug = scenario.pickle.name
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-|-$/g, '');
      const scenarioId = `${featureSlug}--${scenarioSlug}`;

      await doorway.client.beginObservation({
        scenario: scenario.pickle.name,
        scenarioId,
        tags: scenario.pickle.tags.map(t => t.name),
        feature: scenario.pickle.uri,
      });
    } catch {
      // Observation is best-effort — don't block scenarios if storage is down
    }
    break; // Only observe on the first doorway
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
 * Collect observation report from the first doorway (best-effort).
 * Works for both HTTP and Playwright modes.
 */
async function collectObservationReport(
  world: E2EWorld,
  scenario: { pickle: { name: string }; result?: { status: string } }
): Promise<void> {
  const safeName = scenario.pickle.name.replace(/[^a-zA-Z0-9]/g, '-');
  for (const [, doorway] of world.doorways) {
    if (!doorway.client.observationId) break;
    try {
      const report = await doorway.client.getObservationReport();
      const errorCount = report.summary.bySeverity['error'] ?? 0;
      if (errorCount > 0 || scenario.result?.status === Status.FAILED) {
        mkdirSync('reports/observations', { recursive: true });
        writeFileSync(`reports/observations/${safeName}.json`, JSON.stringify(report, null, 2));
      }
    } catch {
      // Best-effort — don't mask the real test result
    }
    break;
  }
}

/**
 * Capture artifacts on failure, assert console cleanliness on pass, then run cleanup.
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

  // For passing scenarios, assert that no real console errors were logged.
  // This makes console cleanliness an automatic test contract for all browser scenarios.
  if (scenario.result?.status === Status.PASSED) {
    const errorReport = collectBrowserErrors(this, scenario.pickle.name);
    if (errorReport.length) {
      throw new Error(
        `Scenario passed but had ${errorReport.length} browser error(s):\n` +
          errorReport.map(e => `  ${e}`).join('\n')
      );
    }
  }

  await collectObservationReport(this, scenario);
  await this.runCleanup();
});

/**
 * Close the shared Playwright browser and tear down testnet after all scenarios complete.
 */
AfterAll(async function () {
  await E2EWorld.closeBrowser();

  // Testnet lifecycle cleanup — settle, report, stop
  if (isTestnetActive()) {
    const summary = getComputeSummary();
    writeComputeReport(summary);
    await stopTestnet();
  }
});
