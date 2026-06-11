/**
 * Seed Conductor Identities — create Human profiles directly on node/device conductors.
 *
 * Node and device humans own their own conductors (StatefulSets in K8s). Their
 * identity must be established directly on their conductor BEFORE they register
 * with doorway for ingress/recovery. This script models that real-world flow.
 *
 * Runs BEFORE seed-humans.ts (which registers with doorway).
 *
 * Environment variables:
 *   CONDUCTOR_URLS               Comma-separated conductor app WebSocket URLs
 *                                e.g. ws://elohim-adam-alpha:4445,ws://elohim-eve-alpha:4445
 *   INSTALLED_APP_ID             Holochain app ID prefix (default: elohim)
 *   CONDUCTOR_CONNECT_TIMEOUT_MS Per-connect timeout (default: 10000) — fail fast on
 *                                unreachable conductors so the stage's catchError can
 *                                soft-land before the pipeline's global timeout fires
 *
 * Output:
 *   [+] Created  — Human profile was just created on the conductor
 *   [=] Exists   — THIS human's profile already present (idempotent)
 *   [C] Conflict — the human's own conductor already embodies a DIFFERENT
 *                  human id (one agent = one Human; needs operator attention)
 *   [-] Skipped  — no conductor deployed for this human (soft, not a failure)
 *   [X] Failed   — could not connect or create (see error)
 *
 * Conductor affinity: a human seeds ONLY onto their own pod, resolved by the
 * `elohim-<name>-<env>` service naming convention — the same rule
 * genesis/Jenkinsfile uses to build CONDUCTOR_URLS. First-reachable-wins is
 * how genesis #1119 got the founder FATAL: every human probed matthew's
 * conductor first, saw SOME human existed (the old exists-check never compared
 * ids), and reported a no-op "exists" while matthew himself (doorway phase)
 * was filtered out entirely. When CONDUCTOR_URLS carries no name-affine URLs
 * (local dev), the legacy walk applies, but with the id-aware exists check.
 *
 *   seed-results-conductor-identities.json — structured per-human results,
 *     written next to the script. The Jenkinsfile reads this to emit a
 *     partial-vs-total UNSTABLE message and to archive the artifact for
 *     orchestrator-level reconciliation (Path C).
 *
 * Exit codes (partial-readiness aware — aligns with the "seed whoever is
 * ready" architecture: partial-cluster operation is the steady state):
 *   0 — all targeted humans created or already exist
 *   2 — partial: at least one human seeded, at least one failed
 *   1 — total failure: ZERO humans seeded (or pre-flight error)
 */

import { readFileSync, existsSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { AdminWebsocket, AppWebsocket, type AppInfo } from '@holochain/client';

// Canonical artifact filename from build-artifacts.json — single source of
// truth across Groovy + TypeScript + JS. Resolved once at module load so
// the writeFileSync below cannot drift from what genesis/Jenkinsfile reads.
const ARTIFACTS_MANIFEST_PATH = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '..', '..', 'orchestrator', 'build-artifacts.json',
);
const ARTIFACTS = JSON.parse(readFileSync(ARTIFACTS_MANIFEST_PATH, 'utf8'));
const SEED_RESULTS_FILE: string = ARTIFACTS.genesis.seedResultsConductorIdentities;

// =============================================================================
// Types
// =============================================================================

interface HumansJsonHuman {
  id: string;
  displayName: string;
  bio?: string;
  category: string;
  profileReach: string;
  affinities?: string[];
  agencyPhase?: string;
}

interface HumansJson {
  humans: HumansJsonHuman[];
}

interface CreateHumanInput {
  id: string;
  display_name: string;
  bio: string | null;
  affinities: string[];
  profile_reach: string;
  location: null;
}

interface HumanOutput {
  id: string;
  display_name: string;
}

type SeedResult = 'created' | 'exists' | 'conflict' | 'skipped' | 'failed';

interface ConductorResult {
  displayName: string;
  humanId: string;
  conductorUrl: string;
  result: SeedResult;
  error?: string;
}

// =============================================================================
// Holochain helpers
// =============================================================================

const CONNECT_TIMEOUT_MS = parseInt(
  process.env.CONDUCTOR_CONNECT_TIMEOUT_MS ?? '10000',
  10
);

async function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  label: string
): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(
      () => reject(new Error(`${label} timed out after ${timeoutMs}ms`)),
      timeoutMs
    );
  });
  try {
    return await Promise.race([promise, timeout]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/**
 * Derive admin WebSocket URL from app WebSocket URL.
 *
 * Convention (socat in K8s): admin port = app port - 1
 *   4445 (app) → 4444 (admin)
 */
function toAdminUrl(appUrl: string): string {
  const u = new URL(appUrl);
  const appPort = parseInt(u.port, 10);
  if (isNaN(appPort)) {
    throw new Error(`Cannot derive admin port from app URL: ${appUrl}`);
  }
  u.port = String(appPort - 1);
  return u.toString();
}

interface ConductorSession {
  appWs: AppWebsocket;
  cellId: [Uint8Array, Uint8Array];
  appInfo: AppInfo;
}

/**
 * Connect to a conductor and find an installed app that starts with the given prefix.
 * Returns null if no matching app is found (this conductor isn't for this human).
 */
async function connectToConductor(
  appUrl: string,
  appIdPrefix: string
): Promise<ConductorSession | null> {
  const adminUrl = toAdminUrl(appUrl);

  let adminWs: AdminWebsocket;
  try {
    adminWs = await withTimeout(
      AdminWebsocket.connect({
        url: new URL(adminUrl),
        wsClientOptions: { origin: 'http://localhost' },
      }),
      CONNECT_TIMEOUT_MS,
      `Admin connect ${adminUrl}`
    );
  } catch (err) {
    throw new Error(
      `Admin connect failed (${adminUrl}): ${err instanceof Error ? err.message : err}`
    );
  }

  try {
    const apps = await adminWs.listApps({});
    const matchingApp = apps.find(a => a.installed_app_id.startsWith(appIdPrefix));

    if (!matchingApp) {
      await adminWs.client.close();
      return null;
    }

    // Find the imagodei role's provisioned cell.
    // @holochain/client returns two possible cell formats depending on version:
    //   { type: "provisioned", value: { cell_id: [...] } }  — newer
    //   { provisioned: { cell_id: [...] } }                 — older
    const imagodeiCells = matchingApp.cell_info['imagodei'];
    if (!imagodeiCells || imagodeiCells.length === 0) {
      await adminWs.client.close();
      throw new Error(`App '${matchingApp.installed_app_id}' has no imagodei cells`);
    }

    let cellId: [Uint8Array, Uint8Array] | null = null;
    for (const cell of imagodeiCells as unknown[]) {
      const c = cell as Record<string, unknown>;
      if (c['type'] === 'provisioned' && c['value']) {
        cellId = (c['value'] as Record<string, unknown>)['cell_id'] as [Uint8Array, Uint8Array];
        break;
      } else if (c['provisioned']) {
        cellId = (c['provisioned'] as Record<string, unknown>)['cell_id'] as [Uint8Array, Uint8Array];
        break;
      }
    }

    if (!cellId) {
      await adminWs.client.close();
      throw new Error(
        `App '${matchingApp.installed_app_id}' imagodei cell is not provisioned`
      );
    }

    // Authorize signing credentials for this cell
    await adminWs.authorizeSigningCredentials(cellId);

    // Issue a short-lived app auth token
    const tokenResult = await adminWs.issueAppAuthenticationToken({
      installed_app_id: matchingApp.installed_app_id,
      single_use: true,
      expiry_seconds: 300,
    });

    await adminWs.client.close();

    // Connect to app interface
    const appWs = await withTimeout(
      AppWebsocket.connect({
        url: new URL(appUrl),
        token: tokenResult.token,
        wsClientOptions: { origin: 'http://localhost' },
      }),
      CONNECT_TIMEOUT_MS,
      `App connect ${appUrl}`
    );

    return { appWs, cellId, appInfo: matchingApp };
  } catch (err) {
    try {
      await adminWs.client.close();
    } catch {
      // ignore close errors
    }
    throw err;
  }
}

/**
 * Check whether the agent on this conductor already has a Human profile.
 */
async function getMyHuman(
  appWs: AppWebsocket,
  cellId: [Uint8Array, Uint8Array]
): Promise<HumanOutput | null> {
  const result = await appWs.callZome({
    cell_id: cellId,
    zome_name: 'imagodei',
    fn_name: 'get_my_human',
    payload: null,
  });
  return (result as HumanOutput | null) ?? null;
}

/**
 * Create a Human profile on the conductor for the given human.
 */
async function createHuman(
  appWs: AppWebsocket,
  cellId: [Uint8Array, Uint8Array],
  input: CreateHumanInput
): Promise<HumanOutput> {
  const result = await appWs.callZome({
    cell_id: cellId,
    zome_name: 'imagodei',
    fn_name: 'create_human',
    payload: input,
  });
  return result as HumanOutput;
}

// =============================================================================
// Conductor affinity + identity helpers (pure — exported for unit tests)
// =============================================================================

/** `human-matthew-manager` → `matthew` (genesis/Jenkinsfile's exact rule). */
export function humanShortName(humanId: string): string {
  return humanId.replace(/^human-/, '').split('-')[0];
}

/** A URL set is name-affine when any URL follows `elohim-<name>-<env>`. */
const NAME_AFFINE_RE = /elohim-[a-z0-9]+-/;

export function urlsAreNameAffine(conductorUrls: string[]): boolean {
  return conductorUrls.some(u => NAME_AFFINE_RE.test(u));
}

/**
 * Resolve the conductor URL that belongs to this human via the
 * `elohim-<name>-<env>` service convention. Null when the human has no
 * deployed conductor in the list.
 */
export function conductorUrlForHuman(
  humanId: string,
  conductorUrls: string[]
): string | null {
  const name = humanShortName(humanId);
  return conductorUrls.find(u => u.includes(`elohim-${name}-`)) ?? null;
}

/**
 * Normalize get_my_human's return across zome/client shapes:
 * `{ id }` (HumanOutput) or `{ human: { id } }` (wrapped).
 */
export function extractHumanId(result: unknown): string | undefined {
  if (!result || typeof result !== 'object') return undefined;
  const r = result as Record<string, unknown>;
  if (typeof r['id'] === 'string') return r['id'];
  const nested = r['human'];
  if (nested && typeof nested === 'object') {
    const nid = (nested as Record<string, unknown>)['id'];
    if (typeof nid === 'string') return nid;
  }
  return undefined;
}

// =============================================================================
// Per-human seeding logic
// =============================================================================

/**
 * Seed one human onto THEIR conductor (name-affine), or — when CONDUCTOR_URLS
 * has no name-affine URLs (local dev) — walk the list with the id-aware
 * exists check.
 */
async function seedHumanOnConductor(
  human: HumansJsonHuman,
  conductorUrls: string[],
  appIdPrefix: string
): Promise<ConductorResult> {
  const base: Pick<ConductorResult, 'displayName' | 'humanId'> = {
    displayName: human.displayName,
    humanId: human.id,
  };

  // Name-affine targeting: this human's own pod, or nothing. The legacy
  // walk survives only for non-affine URL sets (local dev).
  const affine = urlsAreNameAffine(conductorUrls);
  const ownUrl = conductorUrlForHuman(human.id, conductorUrls);
  const candidates = affine ? (ownUrl ? [ownUrl] : []) : conductorUrls;

  if (candidates.length === 0) {
    return {
      ...base,
      conductorUrl: '(none)',
      result: 'skipped',
      error: `no conductor deployed for this human (no elohim-${humanShortName(human.id)}-* in CONDUCTOR_URLS)`,
    };
  }

  let lastError: string | undefined;

  for (const conductorUrl of candidates) {
    let session: ConductorSession | null = null;

    try {
      session = await connectToConductor(conductorUrl, appIdPrefix);
    } catch (err) {
      // Connection failed — try next URL
      lastError = `Connect error (${conductorUrl}): ${err instanceof Error ? err.message : err}`;
      continue;
    }

    if (!session) {
      // No matching app on this conductor — try next
      continue;
    }

    const { appWs, cellId } = session;

    try {
      // Identity-aware exists check: "a human exists here" only counts when
      // it is THIS human. One agent = one Human — a different id on the
      // human's own conductor is a conflict, not an idempotent no-op.
      const existing = await getMyHuman(appWs, cellId);
      const existingId = extractHumanId(existing);

      if (existingId === human.id) {
        await appWs.client.close();
        return { ...base, conductorUrl, result: 'exists' };
      }

      if (existingId !== undefined) {
        await appWs.client.close();
        if (affine) {
          return {
            ...base,
            conductorUrl,
            result: 'conflict',
            error: `conductor already embodies '${existingId}' — expected '${human.id}'`,
          };
        }
        // Legacy walk: someone else's conductor — keep looking.
        lastError = `conductor at ${conductorUrl} embodies '${existingId}'`;
        continue;
      }

      // Create Human profile
      const input: CreateHumanInput = {
        id: human.id,
        display_name: human.displayName,
        bio: human.bio ?? null,
        affinities: human.affinities ?? [],
        profile_reach: human.profileReach,
        location: null,
      };

      await createHuman(appWs, cellId, input);
      await appWs.client.close();
      return { ...base, conductorUrl, result: 'created' };
    } catch (err) {
      try {
        await appWs.client.close();
      } catch {
        // ignore
      }
      lastError = err instanceof Error ? err.message : String(err);
      // Treat zome errors as failures for this human (don't try other conductors)
      return {
        ...base,
        conductorUrl,
        result: 'failed',
        error: lastError,
      };
    }
  }

  // No conductor matched this human
  return {
    ...base,
    conductorUrl: '(none)',
    result: 'failed',
    error: lastError ?? 'No conductor found with a matching app for this human',
  };
}

// =============================================================================
// Main
// =============================================================================

async function main(): Promise<void> {
  const conductorUrlsRaw = process.env.CONDUCTOR_URLS ?? '';
  const appIdPrefix = process.env.INSTALLED_APP_ID ?? 'elohim';

  const conductorUrls = conductorUrlsRaw
    .split(',')
    .map(u => u.trim())
    .filter(Boolean);

  // Load humans.json (generated artifact from genesis/data/humans/*.md)
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const jsonPath = resolve(__dirname, '../../data/humans/humans.json');
  if (!existsSync(jsonPath)) {
    console.error(`ERROR: ${jsonPath} not found`);
    console.error('  humans.json is generated from markdown. Run:');
    console.error('  pnpm --filter genesis-seeder run build:data');
    process.exit(1);
  }
  const humansJson: HumansJson = JSON.parse(readFileSync(jsonPath, 'utf-8'));

  // Node, device, AND doorway humans: doorway humans (matthew) host a full
  // conductor too — excluding them is how the household founder was never
  // seeded and formation FATALed (genesis #1119).
  const targets = humansJson.humans.filter(
    h => h.agencyPhase === 'node' || h.agencyPhase === 'device' || h.agencyPhase === 'doorway'
  );

  console.log('=== Seed Conductor Identities ===\n');
  console.log(`App ID prefix: ${appIdPrefix}`);
  console.log(
    `Conductors:    ${conductorUrls.length > 0 ? conductorUrls.join(', ') : '(none — set CONDUCTOR_URLS)'}`
  );
  console.log(`Humans:        ${targets.length} node/device/doorway of ${humansJson.humans.length} total`);
  console.log('');

  if (conductorUrls.length === 0) {
    console.error('ERROR: CONDUCTOR_URLS is not set. Set it to comma-separated conductor app WebSocket URLs.');
    console.error('  Example: CONDUCTOR_URLS=ws://elohim-adam-alpha:4445,ws://elohim-eve-alpha:4445');
    process.exit(1);
  }

  // Sort: doorway (founder hosts) first, then node, then device
  const phaseOrder: Record<string, number> = { doorway: 0, node: 1, device: 2 };
  const sorted = [...targets].sort(
    (a, b) => (phaseOrder[a.agencyPhase ?? 'device'] ?? 99) - (phaseOrder[b.agencyPhase ?? 'device'] ?? 99)
  );

  const results: ConductorResult[] = [];

  for (const human of sorted) {
    const result = await seedHumanOnConductor(human, conductorUrls, appIdPrefix);
    results.push(result);

    const icon =
      result.result === 'created' ? '+'
      : result.result === 'exists' ? '='
      : result.result === 'conflict' ? 'C'
      : result.result === 'skipped' ? '-'
      : 'X';
    const phase = (human.agencyPhase ?? '').padEnd(6);
    const name = result.displayName.padEnd(16);
    const suffix = result.error ? ` (${result.error})` : '';
    console.log(`  [${icon}] ${name} ${phase} ${result.conductorUrl}${suffix}`);
  }

  const created = results.filter(r => r.result === 'created').length;
  const exists = results.filter(r => r.result === 'exists').length;
  const conflict = results.filter(r => r.result === 'conflict').length;
  const skipped = results.filter(r => r.result === 'skipped').length;
  const failed = results.filter(r => r.result === 'failed').length + conflict;
  const succeeded = created + exists;

  console.log('');
  console.log(
    `=== Results: ${created} created, ${exists} existing, ${conflict} conflict, ${skipped} skipped, ${failed - conflict} failed ===`
  );

  // Structured artifact for Jenkinsfile + orchestrator-level reconciliation.
  // Schema kept stable so Path C (stageAnnotations in actual-build-graph.json)
  // can consume this without re-parsing console output.
  const report = {
    schemaVersion: '1',
    seededAt: new Date().toISOString(),
    script: 'seed-conductor-identities',
    counts: { created, exists, conflict, skipped, failed, succeeded, total: results.length },
    partial: succeeded > 0 && failed > 0,
    allSucceeded: failed === 0,
    allFailed: succeeded === 0 && failed > 0,
    results: results.map(r => ({
      humanId: r.humanId,
      displayName: r.displayName,
      result: r.result,
      conductorUrl: r.conductorUrl,
      error: r.error ?? null,
    })),
  };
  try {
    writeFileSync(SEED_RESULTS_FILE, JSON.stringify(report, null, 2));
  } catch (e) {
    console.error(`WARN: could not write ${SEED_RESULTS_FILE}:`, e);
  }

  if (failed > 0) {
    console.error('\nFailed humans:');
    for (const r of results.filter(r => r.result === 'failed' || r.result === 'conflict')) {
      console.error(`  ${r.displayName} (${r.humanId}): ${r.error}`);
    }
    // Partial-readiness aware: exit 2 if at least one succeeded, exit 1
    // only on total failure. The Jenkinsfile maps both to UNSTABLE today
    // but the distinction lets a future operator (or external Ralph
    // Wiggum loop) route partial vs total to different remediations.
    process.exit(succeeded > 0 ? 2 : 1);
  }

  process.exit(0);
}

// Only run when executed directly (same guard as seed-household-formation.ts) —
// the pure helpers above are imported by unit tests.
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
