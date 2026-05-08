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
 *   [+] Created — Human profile was just created on the conductor
 *   [=] Exists  — Human profile already present (idempotent)
 *   [X] Failed  — Could not connect or create (see error)
 *
 * Exit codes:
 *   0 — all node/device humans created or already exist
 *   1 — one or more humans failed
 */

import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { AdminWebsocket, AppWebsocket, type AppInfo } from '@holochain/client';

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

type SeedResult = 'created' | 'exists' | 'failed';

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
// Per-human seeding logic
// =============================================================================

/**
 * Attempt to seed one human across the available conductor URLs.
 *
 * Iterates conductor URLs until one responds with a matching app. If none
 * match, the human is skipped (no conductor found for them).
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

  let lastError: string | undefined;

  for (const conductorUrl of conductorUrls) {
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
      // Check if Human already exists
      const existing = await getMyHuman(appWs, cellId);

      if (existing) {
        await appWs.client.close();
        return { ...base, conductorUrl, result: 'exists' };
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

  // Filter to node and device humans only
  const targets = humansJson.humans.filter(
    h => h.agencyPhase === 'node' || h.agencyPhase === 'device'
  );

  console.log('=== Seed Conductor Identities ===\n');
  console.log(`App ID prefix: ${appIdPrefix}`);
  console.log(
    `Conductors:    ${conductorUrls.length > 0 ? conductorUrls.join(', ') : '(none — set CONDUCTOR_URLS)'}`
  );
  console.log(`Humans:        ${targets.length} node/device of ${humansJson.humans.length} total`);
  console.log('');

  if (conductorUrls.length === 0) {
    console.error('ERROR: CONDUCTOR_URLS is not set. Set it to comma-separated conductor app WebSocket URLs.');
    console.error('  Example: CONDUCTOR_URLS=ws://elohim-adam-alpha:4445,ws://elohim-eve-alpha:4445');
    process.exit(1);
  }

  // Sort: node first, then device
  const phaseOrder: Record<string, number> = { node: 0, device: 1 };
  const sorted = [...targets].sort(
    (a, b) => (phaseOrder[a.agencyPhase ?? 'device'] ?? 99) - (phaseOrder[b.agencyPhase ?? 'device'] ?? 99)
  );

  const results: ConductorResult[] = [];

  for (const human of sorted) {
    const result = await seedHumanOnConductor(human, conductorUrls, appIdPrefix);
    results.push(result);

    const icon = result.result === 'created' ? '+' : result.result === 'exists' ? '=' : 'X';
    const phase = (human.agencyPhase ?? '').padEnd(6);
    const name = result.displayName.padEnd(16);
    const suffix = result.error ? ` (${result.error})` : '';
    console.log(`  [${icon}] ${name} ${phase} ${result.conductorUrl}${suffix}`);
  }

  const created = results.filter(r => r.result === 'created').length;
  const exists = results.filter(r => r.result === 'exists').length;
  const failed = results.filter(r => r.result === 'failed').length;

  console.log('');
  console.log(`=== Results: ${created} created, ${exists} existing, ${failed} failed ===`);

  if (failed > 0) {
    console.error('\nFailed humans:');
    for (const r of results.filter(r => r.result === 'failed')) {
      console.error(`  ${r.displayName} (${r.humanId}): ${r.error}`);
    }
    process.exit(1);
  }

  process.exit(0);
}

main();
