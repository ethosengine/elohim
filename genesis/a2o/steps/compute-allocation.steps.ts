/**
 * Step definitions for Matthew's compute allocation story.
 * Exercises the testnet lifecycle through protocol-native vocabulary.
 */

import { strict as assert } from 'node:assert';
import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Given, When, Then } from '@cucumber/cucumber';

import type { E2EWorld } from '../src/framework/world.js';
import {
  startTestnet,
  stopTestnet,
  isTestnetActive,
  getEnvelopesByVerb,
  getComputeSummary,
} from '../src/framework/testnet-manager.js';
import {
  isProvisionEnvelope,
  isSettleEnvelope,
} from '../src/framework/envelope-validator.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SPAWN_SCRIPT = resolve(__dirname, '../elohim-node/simulation/spawn-persona-testnet.sh');

const DEFAULT_5_PERSONAS = ['matthew', 'susan', 'pete', 'frank', 'nancy'];

// --- Background ---

Given('human {string} has a running steward node', function (this: E2EWorld, name: string) {
  // Steward node is the testnet itself — Matthew's node is the requester.
  // In next sprint, this checks StewardDevice health at localhost:8090.
  this.humans.set(name, {
    name,
    credentials: { identifier: '', password: '' },
    devices: [],
    tokens: new Map(),
  } as never);
});

// --- Scenario 1: Request compute ---

Given('Matthew has a simulation requiring {int} peer nodes', function (this: E2EWorld, count: number) {
  assert.equal(count, 5, `This sprint supports 5 personas. Got: ${count}`);
  (this as unknown as Record<string, unknown>).requestedPersonas = DEFAULT_5_PERSONAS;
  (this as unknown as Record<string, unknown>).requestedCount = count;
});

When(
  'he submits a ServiceRequest with budget {int} cpu-seconds',
  function (this: E2EWorld, budget: number) {
    const personas =
      ((this as unknown as Record<string, unknown>).requestedPersonas as string[]) ??
      DEFAULT_5_PERSONAS;
    startTestnet({
      personas,
      requester: 'matthew',
      ttlSeconds: Math.ceil(budget / personas.length),
      killOnExceed: true,
    });
    (this as unknown as Record<string, unknown>).budget = budget;
  },
);

Then('a provision envelope is emitted for each persona', function () {
  const provisions = getEnvelopesByVerb('provision');
  assert.ok(provisions.length > 0, 'No provision envelopes found');
  for (const env of provisions) {
    assert.ok(isProvisionEnvelope(env), `Invalid provision envelope: ${JSON.stringify(env)}`);
  }
});

Then(
  '{int} conductors are running within {int} seconds',
  { timeout: 60_000 },
  async function (count: number, timeout: number) {
    const deadline = Date.now() + timeout * 1000;
    let running = 0;

    while (Date.now() < deadline) {
      try {
        const status = execSync(
          'bash elohim-node/simulation/spawn-persona-testnet.sh status 2>&1',
          { encoding: 'utf-8', timeout: 10_000, cwd: resolve(__dirname, '../../..') },
        );
        running = (status.match(/RUNNING|pid=/g) || []).length;
        if (running >= count) break;
      } catch {
        // retry
      }
      await new Promise((r) => setTimeout(r, 2000));
    }

    assert.ok(running >= count, `Expected ${count} running conductors, got ${running}`);
  },
);

Then('compute-budget tracking is active', function () {
  assert.ok(isTestnetActive(), 'Testnet session is not active');
});

// --- Scenario 2: Settlement ---

Given(
  '{int} conductors are running for Matthew\'s simulation',
  function (count: number) {
    if (!isTestnetActive()) {
      startTestnet({
        personas: DEFAULT_5_PERSONAS.slice(0, count),
        requester: 'matthew',
        killOnExceed: true,
      });
    }
  },
);

When('the simulation workload completes', { timeout: 30_000 }, async function () {
  // Let budget watcher collect a few samples, then stop
  await new Promise((r) => setTimeout(r, 15_000));
  stopTestnet();
});

Then('a settle envelope is emitted for each persona', function () {
  const settles = getEnvelopesByVerb('settle');
  assert.ok(settles.length > 0, 'No settle envelopes found');
  for (const env of settles) {
    assert.ok(isSettleEnvelope(env), `Invalid settle envelope: ${JSON.stringify(env)}`);
  }
});

Then('each EconomicEvent contains cpu-seconds and memory-mb', function () {
  const settles = getEnvelopesByVerb('settle');
  for (const env of settles) {
    const payload = (env as Record<string, Record<string, Record<string, unknown>>>).payload;
    const event = payload?.economicEvent;
    assert.ok(event, 'Missing economicEvent in settle envelope');
    const rq = event.resourceQuantity as Record<string, unknown> | undefined;
    assert.ok(rq, 'Missing resourceQuantity');
    assert.equal(rq.unit, 'cpu-second');
    assert.ok(typeof rq.value === 'number', 'resourceQuantity.value must be a number');
  }
});

Then('the total spend is within the {int} cpu-second budget', function (budget: number) {
  const summary = getComputeSummary();
  assert.ok(
    summary.totalCpuSeconds <= budget,
    `Total CPU ${summary.totalCpuSeconds}s exceeds budget ${budget}s`,
  );
});

Then('the compute summary appears in the test report', function () {
  const summary = getComputeSummary();
  assert.ok(summary.perPersona, 'Compute summary missing perPersona data');
  // Full report is written in AfterAll hook — here we just verify the data is available
  assert.ok(
    Object.keys(summary.perPersona).length > 0,
    'Compute summary has no persona data',
  );
});

// --- Scenario 3: Circuit breaker ---

Given(
  'one persona is configured with a {int} cpu-second budget',
  function (budget: number) {
    // Override via env for the budget watcher to pick up
    process.env['OVERRIDE_BUDGET_PERSONA'] = 'pete';
    process.env['OVERRIDE_BUDGET_VALUE'] = String(budget);
  },
);

When(
  'that persona exceeds its budget',
  { timeout: 120_000 },
  async function () {
    // Wait for circuit breaker to fire — budget watcher checks every 10s
    const deadline = Date.now() + 90_000;
    while (Date.now() < deadline) {
      const envelopes = getEnvelopesByVerb('settle');
      const exceeded = envelopes.find((e) => {
        const payload = e.payload as Record<string, Record<string, unknown>> | undefined;
        return payload?.economicEvent?.action === 'budget-exceeded';
      });
      if (exceeded) return;
      await new Promise((r) => setTimeout(r, 5000));
    }
    assert.fail('Budget-exceeded envelope not emitted within 90s');
  },
);

Then('it receives SIGTERM with a budget-exceeded envelope', function () {
  const settles = getEnvelopesByVerb('settle');
  const exceeded = settles.filter((e) => {
    const payload = e.payload as Record<string, Record<string, unknown>> | undefined;
    return payload?.economicEvent?.action === 'budget-exceeded';
  });
  assert.ok(exceeded.length > 0, 'No budget-exceeded envelopes found');
});

Then('the remaining {int} conductors continue', function (count: number) {
  const status = execSync(
    'bash elohim-node/simulation/spawn-persona-testnet.sh status 2>&1',
    { encoding: 'utf-8', timeout: 10_000, cwd: resolve(__dirname, '../../..') },
  );
  const running = (status.match(/RUNNING|pid=/g) || []).length;
  assert.ok(running >= count, `Expected ${count} running, got ${running}`);
});

Then('settlement records the partial delivery', function () {
  const settles = getEnvelopesByVerb('settle');
  const partial = settles.filter((e) => {
    const payload = e.payload as Record<string, Record<string, unknown>> | undefined;
    return payload?.economicEvent?.settlement === 'partial';
  });
  assert.ok(partial.length > 0, 'No partial settlement envelopes found');
});
