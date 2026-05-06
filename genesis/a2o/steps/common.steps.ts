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
 * Compute slugs for a cucumber scenario the same way they're used in
 * observation reports. Idempotent and pure.
 */
function computeSlugs(
  scenarioUri: string,
  scenarioName: string
): {
  featureSlug: string;
  scenarioSlug: string;
} {
  const featureSlug = scenarioUri
    .replace(/^.*features\//, '')
    .replace(/\.feature$/, '')
    .replace(/\//g, '-');
  const scenarioSlug = scenarioName
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
  return { featureSlug, scenarioSlug };
}

/**
 * Capture a full-page screenshot for a single device into the per-feature
 * subdir. Called for every Playwright scenario regardless of pass/fail.
 */
async function captureVisualEvidence(
  device: PlaywrightDevice,
  featureSlug: string,
  scenarioSlug: string,
  humanName: string
): Promise<void> {
  const dir = `reports/screenshots/${featureSlug}`;
  mkdirSync(dir, { recursive: true });
  try {
    await device.screenshot(`${featureSlug}/${scenarioSlug}--${humanName}`);
  } catch {
    // best-effort — page may have crashed
  }
}

/**
 * Write a sibling .error.json next to the screenshot for failed scenarios so
 * a reviewer can read the failure context without consulting the cucumber JSON.
 */
function writeErrorSidecar(
  featureSlug: string,
  scenarioSlug: string,
  humanName: string,
  failureMessage: string
): void {
  try {
    const path = `reports/screenshots/${featureSlug}/${scenarioSlug}--${humanName}.error.json`;
    writeFileSync(
      path,
      JSON.stringify(
        {
          status: 'failed',
          failureMessage: failureMessage.split('\n')[0],
        },
        null,
        2
      )
    );
  } catch {
    // best-effort
  }
}

/**
 * Capture failure-only artifacts (console errors JSON, Playwright trace) for a single
 * device. The screenshot is captured separately by captureVisualEvidence in the
 * universal-capture path of the After hook.
 */
async function captureFailureArtifactsExceptScreenshot(
  device: PlaywrightDevice,
  safeName: string,
  humanName: string
): Promise<void> {
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

  const { featureSlug, scenarioSlug } = computeSlugs(scenario.pickle.uri, scenario.pickle.name);
  this.featureSlug = featureSlug;
  this.scenarioSlug = scenarioSlug;

  // Start observation session if a doorway is registered
  for (const [, doorway] of this.doorways) {
    try {
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
 * Capture per-scenario screenshots for all Playwright devices (universal, pass or fail).
 */
async function captureAllDeviceScreenshots(
  world: E2EWorld,
  featureSlug: string,
  scenarioSlug: string
): Promise<void> {
  for (const [name, human] of world.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        await captureVisualEvidence(device, featureSlug, scenarioSlug, name);
      }
    }
  }
}

/**
 * Capture failure-only artifacts for all Playwright devices (trace, console errors,
 * and error sidecar JSON). Called only when the scenario status is FAILED.
 */
async function captureAllFailureArtifacts(
  world: E2EWorld,
  featureSlug: string,
  scenarioSlug: string,
  safeName: string,
  failureMessage: string
): Promise<void> {
  for (const [name, human] of world.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        await captureFailureArtifactsExceptScreenshot(device, safeName, name);
        writeErrorSidecar(featureSlug, scenarioSlug, name, failureMessage);
      }
    }
  }
}

/**
 * Capture artifacts on failure, assert console cleanliness on pass, then run cleanup.
 */
After(async function (this: E2EWorld, scenario) {
  const featureSlug = this.featureSlug ?? 'unknown-feature';
  const scenarioSlug = this.scenarioSlug ?? 'unknown-scenario';

  // Universal capture: every Playwright device gets a screenshot regardless
  // of pass/fail outcome. Failures additionally get a sibling .error.json,
  // and the existing console-errors / trace artifacts (failure-only).
  await captureAllDeviceScreenshots(this, featureSlug, scenarioSlug);

  if (scenario.result?.status === Status.FAILED) {
    const safeName = scenario.pickle.name.replace(/[^a-zA-Z0-9]/g, '-');
    const failureMessage = scenario.result.message ?? 'unknown failure';
    // Write console-errors JSON and trace under reports/console and reports/traces.
    // We no longer write a FAIL- prefixed screenshot — the universal capture above
    // covered it and a sidecar .error.json carries the failure context.
    await captureAllFailureArtifacts(this, featureSlug, scenarioSlug, safeName, failureMessage);
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
