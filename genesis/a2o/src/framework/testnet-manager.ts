/**
 * Testnet Lifecycle Manager
 *
 * Session-scoped orchestration of persona testnet processes.
 * Wraps spawn-persona-testnet.sh and compute-budget.sh with
 * TypeScript lifecycle management for cucumber hooks.
 *
 * Every lifecycle transition produces CoordinationEnvelopes
 * (provision, sense, settle) in JSONL format — the same vocabulary
 * a protocol-native researcher would use to request community compute.
 */

import { execSync, spawn, type ChildProcess } from 'node:child_process';
import { StorageClient } from './storage-client.js';
import { readFileSync, existsSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const SIMULATION_DIR = resolve(__dirname, '../../../../elohim-node/simulation');
const SPAWN_SCRIPT = `${SIMULATION_DIR}/spawn-persona-testnet.sh`;
const BUDGET_SCRIPT = `${SIMULATION_DIR}/compute-budget.sh`;
const DEFAULT_TESTNET_DIR = '/tmp/elohim-persona-testnet';

export interface TestnetSession {
  personas: string[];
  requester: string;
  budgetWatcher: ChildProcess | null;
  startedAt: number;
  ttlSeconds: number;
  testnetDir: string;
  storageClient: StorageClient | null;
  commitmentIds: Map<string, string>;
  matthewCommitmentId: string | null;
}

export interface ComputeSummary {
  totalCpuSeconds: number;
  perPersona: Record<string, { cpuSeconds: number; action: string }>;
  budgetExceeded: string[];
  durationMs: number;
}

let activeSession: TestnetSession | null = null;

export function isTestnetActive(): boolean {
  return activeSession !== null;
}

export function getActiveSession(): TestnetSession | null {
  return activeSession;
}

export async function startTestnet(opts: {
  personas: string[];
  requester?: string;
  ttlSeconds?: number;
  killOnExceed?: boolean;
  testnetDir?: string;
  storageUrl?: string;
}): Promise<void> {
  if (activeSession) {
    // eslint-disable-next-line no-console
    console.log('Testnet already active, reusing session');
    return;
  }

  const requester = opts.requester ?? 'matthew';
  const ttl = opts.ttlSeconds ?? 1800;
  const testnetDir = opts.testnetDir ?? DEFAULT_TESTNET_DIR;
  const personaList = opts.personas.join(',');

  // eslint-disable-next-line no-console
  console.log(
    `\n  Starting testnet: ${personaList} (requester: ${requester}, TTL: ${ttl}s)\n`,
  );

  // Spawn subset via shell script
  execSync(`bash "${SPAWN_SCRIPT}" start-subset "${personaList}" "${requester}" --dir "${testnetDir}"`, {
    stdio: 'inherit',
    timeout: 60_000,
    env: { ...process.env, TESTNET_REQUESTER: requester },
  });

  // Start budget watcher in background
  const budgetWatcher = spawn(
    'bash',
    [BUDGET_SCRIPT, 'watch', '--dir', testnetDir, '--interval', '10'],
    {
      env: {
        ...process.env,
        COMPUTE_TTL_SECONDS: String(ttl),
        COMPUTE_KILL_ON_EXCEED: String(opts.killOnExceed ?? true),
        TESTNET_REQUESTER: requester,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
      detached: false,
    },
  );

  budgetWatcher.stdout?.on('data', (data: Buffer) => {
    const line = data.toString().trim();
    if (line.includes('WARN') || line.includes('KILL') || line.includes('TTL')) {
      // eslint-disable-next-line no-console
      console.log(`  [budget] ${line}`);
    }
  });

  budgetWatcher.stderr?.on('data', (data: Buffer) => {
    const line = data.toString().trim();
    if (line) {
      // eslint-disable-next-line no-console
      console.error(`  [budget:err] ${line}`);
    }
  });

  // Create REA Commitments if storage is available
  let storageClient: StorageClient | null = null;
  const commitmentIds = new Map<string, string>();
  let matthewCommitmentId: string | null = null;

  try {
    storageClient = new StorageClient(opts.storageUrl ?? 'http://localhost:8090');
    if (await storageClient.isHealthy()) {
      const now = new Date().toISOString();
      const totalBudget = opts.personas.length * 360;

      // Matthew's 'take' commitment
      const takeCommitment = await storageClient.createCommitment({
        action: 'take',
        provider: requester,
        receiver: requester,
        resourceClassifiedAs: ['compute'],
        resourceQuantity: { hasNumericalValue: totalBudget, hasUnit: 'cpu-second' },
        hasBeginning: now,
        mediumOfExchangeId: 'cpu-mutual-credit',
        note: `Compute allocation: ${opts.personas.length} peers for ${totalBudget} cpu-seconds`,
      });
      matthewCommitmentId = takeCommitment.id;

      // Per-persona 'give' commitments
      for (const persona of opts.personas) {
        const giveCommitment = await storageClient.createCommitment({
          action: 'give',
          provider: persona,
          receiver: requester,
          resourceClassifiedAs: ['compute'],
          resourceQuantity: { hasNumericalValue: 360, hasUnit: 'cpu-second' },
          effortQuantity: { hasNumericalValue: 150, hasUnit: 'megabyte' },
          hasBeginning: now,
          mediumOfExchangeId: 'cpu-mutual-credit',
          note: `Provide compute to ${requester}`,
        });
        commitmentIds.set(persona, giveCommitment.id);
      }

      // eslint-disable-next-line no-console
      console.log(`  REA: Created ${commitmentIds.size + 1} paired commitments`);
    } else {
      // eslint-disable-next-line no-console
      console.warn('  elohim-storage not available — skipping REA persistence');
    }
  } catch (err) {
    // eslint-disable-next-line no-console
    console.warn(`  REA commitment creation failed (non-fatal): ${err}`);
  }

  activeSession = {
    personas: opts.personas,
    requester,
    budgetWatcher,
    startedAt: Date.now(),
    ttlSeconds: ttl,
    testnetDir,
    storageClient,
    commitmentIds,
    matthewCommitmentId,
  };
}

export async function stopTestnet(): Promise<void> {
  if (!activeSession) return;

  const { testnetDir, budgetWatcher } = activeSession;

  // Kill budget watcher first
  if (budgetWatcher && !budgetWatcher.killed) {
    budgetWatcher.kill('SIGTERM');
  }

  // Settle via compute-budget.sh
  try {
    execSync(`bash "${BUDGET_SCRIPT}" settle --dir "${testnetDir}"`, {
      stdio: 'inherit',
      timeout: 30_000,
    });
  } catch {
    // eslint-disable-next-line no-console
    console.warn('  Settlement failed (non-fatal)');
  }

  // Stop all persona processes (emits settle envelopes)
  try {
    execSync(`bash "${SPAWN_SCRIPT}" stop --dir "${testnetDir}"`, {
      stdio: 'inherit',
      timeout: 30_000,
    });
  } catch {
    // eslint-disable-next-line no-console
    console.warn('  Testnet stop failed (non-fatal)');
  }

  // Persist REA settlement to elohim-storage
  const { storageClient: sc, commitmentIds: cIds, matthewCommitmentId: mId } = activeSession;
  if (sc && cIds.size > 0) {
    try {
      const summary = getComputeSummary();
      const now = new Date().toISOString();

      // Per-persona deliver-service events
      for (const [persona, commitmentId] of cIds) {
        const metrics = summary.perPersona[persona] ?? { cpuSeconds: 0, action: 'unknown' };
        const isExceeded = summary.budgetExceeded.includes(persona);

        await sc.createEconomicEvent({
          action: 'deliver-service',
          provider: persona,
          receiver: activeSession.requester,
          resourceClassifiedAs: ['compute'],
          resourceQuantityValue: metrics.cpuSeconds,
          resourceQuantityUnit: 'cpu-second',
          hasPointInTime: now,
          fulfills: [commitmentId],
          lamadEventType: 'compute-deliver',
          note: isExceeded ? 'Budget exceeded — partial delivery' : 'Compute delivered',
        });

        await sc.updateCommitmentState(
          commitmentId,
          isExceeded ? 'breached' : 'fulfilled',
          !isExceeded,
        );
      }

      // Matthew's aggregate take event
      if (mId) {
        await sc.createEconomicEvent({
          action: 'take',
          provider: activeSession.requester,
          receiver: activeSession.requester,
          resourceClassifiedAs: ['compute'],
          resourceQuantityValue: summary.totalCpuSeconds,
          resourceQuantityUnit: 'cpu-second',
          hasPointInTime: now,
          fulfills: [mId],
          lamadEventType: 'compute-take',
        });

        await sc.updateCommitmentState(mId, 'fulfilled', true);
      }

      // eslint-disable-next-line no-console
      console.log(`  REA: Persisted ${cIds.size + 1} economic events`);
    } catch (err) {
      // eslint-disable-next-line no-console
      console.warn(`  REA settlement failed (non-fatal): ${err}`);
    }
  }

  activeSession = null;
}

/**
 * Read all CoordinationEnvelopes from the JSONL ledger.
 */
export function getEnvelopes(testnetDir?: string): Record<string, unknown>[] {
  const dir = testnetDir ?? activeSession?.testnetDir ?? DEFAULT_TESTNET_DIR;
  const file = `${dir}/envelopes/envelopes.jsonl`;
  if (!existsSync(file)) return [];
  return readFileSync(file, 'utf-8')
    .trim()
    .split('\n')
    .filter((line) => line.startsWith('{'))
    .map((line) => JSON.parse(line) as Record<string, unknown>);
}

export function getEnvelopesByVerb(
  verb: string,
  testnetDir?: string,
): Record<string, unknown>[] {
  return getEnvelopes(testnetDir).filter((e) => e.verb === verb);
}

/**
 * Build a compute summary from settle envelopes.
 */
export function getComputeSummary(testnetDir?: string): ComputeSummary {
  const settles = getEnvelopesByVerb('settle', testnetDir);
  const perPersona: Record<string, { cpuSeconds: number; action: string }> = {};
  const budgetExceeded: string[] = [];
  let totalCpu = 0;

  for (const env of settles) {
    const payload = env.payload as {
      economicEvent?: {
        action?: string;
        provider?: string;
        resourceQuantity?: { value?: number };
      };
    } | undefined;
    const event = payload?.economicEvent;
    if (!event?.provider) continue;

    const cpu = event.resourceQuantity?.value ?? 0;
    const action = event.action ?? 'unknown';
    perPersona[event.provider] = { cpuSeconds: cpu, action };
    totalCpu += cpu;

    if (action === 'budget-exceeded') {
      budgetExceeded.push(event.provider);
    }
  }

  return {
    totalCpuSeconds: totalCpu,
    perPersona,
    budgetExceeded,
    durationMs: activeSession ? Date.now() - activeSession.startedAt : 0,
  };
}

/**
 * Write compute summary to reports/ and print a telemetry table.
 */
export function writeComputeReport(summary: ComputeSummary): void {
  const reportPath = 'reports/compute-summary.json';
  writeFileSync(reportPath, JSON.stringify(summary, null, 2));

  // eslint-disable-next-line no-console
  console.log('\n┌─────────────────────────────────────────────┐');
  // eslint-disable-next-line no-console
  console.log('│           COMPUTE TELEMETRY                 │');
  // eslint-disable-next-line no-console
  console.log('├─────────────────────────┬───────────────────┤');
  // eslint-disable-next-line no-console
  console.log(
    `│ Total CPU               │ ${String(summary.totalCpuSeconds).padStart(12)}s │`,
  );
  // eslint-disable-next-line no-console
  console.log(
    `│ Duration                │ ${String(Math.round(summary.durationMs / 1000)).padStart(12)}s │`,
  );
  // eslint-disable-next-line no-console
  console.log(
    `│ Budget exceeded         │ ${String(summary.budgetExceeded.length).padStart(12)}  │`,
  );
  // eslint-disable-next-line no-console
  console.log('├─────────────────────────┼───────────────────┤');

  for (const [persona, metrics] of Object.entries(summary.perPersona)) {
    const name = persona.replace('human-', '').substring(0, 23);
    const marker = metrics.action === 'budget-exceeded' ? ' !' : '  ';
    // eslint-disable-next-line no-console
    console.log(
      `│${marker}${name.padEnd(23)} │ ${String(metrics.cpuSeconds).padStart(12)}s │`,
    );
  }

  // eslint-disable-next-line no-console
  console.log('└─────────────────────────┴───────────────────┘');
  // eslint-disable-next-line no-console
  console.log(`  Written to: ${reportPath}\n`);
}
