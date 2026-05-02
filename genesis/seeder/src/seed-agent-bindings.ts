/**
 * Seed AgentPeerBindings — write multi-device peer bindings for demo humans.
 *
 * Each demo human's Agent EPR gets one or more AgentPeerBinding entries that
 * tie a libp2p peer_id to the agent_cid for a specific hardware archetype
 * (node | desktop | mobile | steward). These bindings let the topology view
 * aggregate multiple peer_ids into a single human/agent identity.
 *
 * Two demo humans receive multi-device plans (the multi-device-demo evidence):
 *   - Matthew (operator)  →  desktop + node + steward (3 bindings)
 *   - Jessica (steward)   →  desktop + mobile         (2 bindings)
 *   - Everyone else       →  desktop                  (1 binding)
 *
 * Runs AFTER seed-conductor-identities.ts (which creates the Agent EPR via
 * imagodei.create_human → create_agent). The signer-match gate in the
 * `create_agent_peer_binding` zome requires the calling pubkey to equal the
 * Agent EPR's holochain_agent_key — which holds because each human's seeder
 * call runs on their own conductor whose agent IS the human's agent.
 *
 * Idempotency: re-running this seeder creates additional binding entries —
 * there is no Stage-1 dedup check inside the zome. peer_id derivation is
 * deterministic from (humanId, archetype), so re-runs produce identical
 * peer_ids; the aggregated topology view will still group correctly even
 * with duplicate underlying entries. Stage-2 dedup is a future concern.
 *
 * Environment variables:
 *   CONDUCTOR_URLS    Comma-separated conductor app WebSocket URLs
 *                     e.g. ws://elohim-adam-alpha:4445,ws://elohim-eve-alpha:4445
 *   INSTALLED_APP_ID  Holochain app ID prefix (default: elohim)
 *
 * Output icons (mirrors seed-conductor-identities.ts):
 *   [+] created — binding(s) just written
 *   [=] exists  — (reserved for future dedup; unused at Stage 1)
 *   [X] failed  — connection or zome error
 *
 * Exit codes:
 *   0 — every targeted human's bindings written
 *   1 — at least one human failed
 */

import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { AdminWebsocket, AppWebsocket, type AppInfo } from '@holochain/client';

// =============================================================================
// Types — match seed-conductor-identities.ts (no shared types module exists)
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

type Archetype = 'node' | 'desktop' | 'mobile' | 'steward';

interface CreateAgentPeerBindingInput {
  peer_id: string;
  agent_cid: string;
  valid_from_micros: number;
  valid_until_micros: number | null;
  device_archetype: Archetype;
  signature: Uint8Array;
}

type SeedResult = 'created' | 'exists' | 'failed';

interface BindingResult {
  humanId: string;
  displayName: string;
  conductorUrl: string;
  plans: Archetype[];
  created: number;
  errors: string[];
  result: SeedResult;
}

// =============================================================================
// Plan: which archetypes each human gets
// =============================================================================

/**
 * Return the archetype list for a given human.
 *
 * Two humans get multi-device plans (Matthew = operator, Jessica = steward);
 * everyone else gets a single desktop binding so the dataset stays realistic.
 */
function plansForHuman(human: HumansJsonHuman): Archetype[] {
  switch (human.id) {
    case 'human-matthew-manager':
      return ['desktop', 'node', 'steward'];
    case 'human-jessica-spouse':
      return ['desktop', 'mobile'];
    default:
      return ['desktop'];
  }
}

/**
 * Deterministic peer_id derivation.
 *
 * Same (humanId, archetype) → same peer_id across re-runs. Uses the
 * `12D3KooW...` libp2p PeerId multibase prefix — sufficient for test
 * seeding; not cryptographically derived from a real keypair (Stage 2
 * concern).
 */
function deterministicPeerId(humanId: string, archetype: Archetype): string {
  const hash = createHash('sha256');
  hash.update(`${humanId}:${archetype}`, 'utf8');
  const bytes = hash.digest();
  // base64url, sliced to keep total length stable (12D3KooW + 38 = 46 chars)
  const suffix = bytes.toString('base64url').slice(0, 38);
  return `12D3KooW${suffix}`;
}

// =============================================================================
// Holochain helpers (focused copy of seed-conductor-identities.ts)
// =============================================================================

/**
 * Derive admin WebSocket URL from app WebSocket URL.
 * Convention (socat in K8s): admin port = app port - 1.
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
 * Connect to a conductor and find an installed app starting with the given prefix.
 * Returns null if no matching app is found (this conductor isn't for this human).
 */
async function connectToConductor(
  appUrl: string,
  appIdPrefix: string,
): Promise<ConductorSession | null> {
  const adminUrl = toAdminUrl(appUrl);

  let adminWs: AdminWebsocket;
  try {
    adminWs = await AdminWebsocket.connect({
      url: new URL(adminUrl),
      wsClientOptions: { origin: 'http://localhost' },
    });
  } catch (err) {
    throw new Error(
      `Admin connect failed (${adminUrl}): ${err instanceof Error ? err.message : err}`,
    );
  }

  try {
    const apps = await adminWs.listApps({});
    const matchingApp = apps.find(a => a.installed_app_id.startsWith(appIdPrefix));

    if (!matchingApp) {
      await adminWs.client.close();
      return null;
    }

    const imagodeiCells = matchingApp.cell_info['imagodei'];
    if (!imagodeiCells || imagodeiCells.length === 0) {
      await adminWs.client.close();
      throw new Error(`App '${matchingApp.installed_app_id}' has no imagodei cells`);
    }

    let cellId: [Uint8Array, Uint8Array] | null = null;
    for (const cell of imagodeiCells as unknown[]) {
      const c = cell as Record<string, unknown>;
      if (c['type'] === 'provisioned' && c['value']) {
        cellId = (c['value'] as Record<string, unknown>)['cell_id'] as [
          Uint8Array,
          Uint8Array,
        ];
        break;
      } else if (c['provisioned']) {
        cellId = (c['provisioned'] as Record<string, unknown>)['cell_id'] as [
          Uint8Array,
          Uint8Array,
        ];
        break;
      }
    }

    if (!cellId) {
      await adminWs.client.close();
      throw new Error(`App '${matchingApp.installed_app_id}' imagodei cell is not provisioned`);
    }

    await adminWs.authorizeSigningCredentials(cellId);

    const tokenResult = await adminWs.issueAppAuthenticationToken({
      installed_app_id: matchingApp.installed_app_id,
      single_use: true,
      expiry_seconds: 300,
    });

    await adminWs.client.close();

    const appWs = await AppWebsocket.connect({
      url: new URL(appUrl),
      token: tokenResult.token,
      wsClientOptions: { origin: 'http://localhost' },
    });

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
 * Call imagodei.create_agent_peer_binding on the given cell.
 *
 * Stage 1: signature is empty (the zome's signer-match gate is the only
 * authorization check; signature verification is Stage 2).
 */
async function callCreateBinding(
  appWs: AppWebsocket,
  cellId: [Uint8Array, Uint8Array],
  input: CreateAgentPeerBindingInput,
): Promise<void> {
  await appWs.callZome({
    cell_id: cellId,
    zome_name: 'imagodei',
    fn_name: 'create_agent_peer_binding',
    payload: input,
  });
}

// =============================================================================
// Per-human seeding
// =============================================================================

/**
 * Seed all bindings for one human across the available conductor URLs.
 *
 * Iterates conductor URLs until one responds with a matching app for this
 * human; then writes one AgentPeerBinding per archetype in the plan.
 */
async function seedBindingsForHuman(
  human: HumansJsonHuman,
  conductorUrls: string[],
  appIdPrefix: string,
): Promise<BindingResult> {
  const plans = plansForHuman(human);
  const base: Pick<BindingResult, 'humanId' | 'displayName' | 'plans'> = {
    humanId: human.id,
    displayName: human.displayName,
    plans,
  };

  let lastError: string | undefined;

  for (const conductorUrl of conductorUrls) {
    let session: ConductorSession | null = null;

    try {
      session = await connectToConductor(conductorUrl, appIdPrefix);
    } catch (err) {
      lastError = `Connect error (${conductorUrl}): ${err instanceof Error ? err.message : err}`;
      continue;
    }

    if (!session) {
      // No matching app on this conductor — try next.
      continue;
    }

    const { appWs, cellId } = session;
    const errors: string[] = [];
    let created = 0;

    try {
      const validFromMicros = Date.now() * 1000;

      for (const archetype of plans) {
        const peerId = deterministicPeerId(human.id, archetype);
        const input: CreateAgentPeerBindingInput = {
          peer_id: peerId,
          agent_cid: human.id,
          valid_from_micros: validFromMicros,
          valid_until_micros: null,
          device_archetype: archetype,
          signature: new Uint8Array(0),
        };

        try {
          await callCreateBinding(appWs, cellId, input);
          created += 1;
        } catch (err) {
          errors.push(
            `${archetype}: ${err instanceof Error ? err.message : String(err)}`,
          );
        }
      }
    } finally {
      try {
        await appWs.client.close();
      } catch {
        // ignore close errors
      }
    }

    return {
      ...base,
      conductorUrl,
      created,
      errors,
      result: errors.length === 0 ? 'created' : 'failed',
    };
  }

  return {
    ...base,
    conductorUrl: '(none)',
    created: 0,
    errors: [lastError ?? 'No conductor found with a matching app for this human'],
    result: 'failed',
  };
}

// =============================================================================
// Main
// =============================================================================

export async function main(): Promise<void> {
  const conductorUrlsRaw = process.env.CONDUCTOR_URLS ?? '';
  const appIdPrefix = process.env.INSTALLED_APP_ID ?? 'elohim';

  const conductorUrls = conductorUrlsRaw
    .split(',')
    .map(u => u.trim())
    .filter(Boolean);

  const __dirname = dirname(fileURLToPath(import.meta.url));
  const jsonPath = resolve(__dirname, '../../data/humans/humans.json');
  if (!existsSync(jsonPath)) {
    console.error(`ERROR: ${jsonPath} not found`);
    console.error('  humans.json is generated from markdown. Run:');
    console.error('  pnpm --filter genesis-seeder run build:data');
    process.exit(1);
  }
  const humansJson: HumansJson = JSON.parse(readFileSync(jsonPath, 'utf-8'));

  // Filter to humans that own a conductor: node, device, doorway.
  // (Hosted humans live on a shared conductor and are out of scope here.)
  const targets = humansJson.humans.filter(
    h =>
      h.agencyPhase === 'node' ||
      h.agencyPhase === 'device' ||
      h.agencyPhase === 'doorway',
  );

  console.log('=== Seed Agent Peer Bindings ===\n');
  console.log(`App ID prefix: ${appIdPrefix}`);
  console.log(
    `Conductors:    ${conductorUrls.length > 0 ? conductorUrls.join(', ') : '(none — set CONDUCTOR_URLS)'}`,
  );
  console.log(
    `Humans:        ${targets.length} node/device/doorway of ${humansJson.humans.length} total`,
  );
  console.log('');

  if (conductorUrls.length === 0) {
    console.error(
      'ERROR: CONDUCTOR_URLS is not set. Set it to comma-separated conductor app WebSocket URLs.',
    );
    console.error(
      "  Example: CONDUCTOR_URLS=ws://elohim-adam-alpha:4445,ws://elohim-eve-alpha:4445",
    );
    process.exit(1);
  }

  // Sort: node first, then doorway (operator), then device.
  const phaseOrder: Record<string, number> = { node: 0, doorway: 1, device: 2 };
  const sorted = [...targets].sort(
    (a, b) =>
      (phaseOrder[a.agencyPhase ?? 'device'] ?? 99) -
      (phaseOrder[b.agencyPhase ?? 'device'] ?? 99),
  );

  const results: BindingResult[] = [];

  for (const human of sorted) {
    const result = await seedBindingsForHuman(human, conductorUrls, appIdPrefix);
    results.push(result);

    const icon =
      result.result === 'created' ? '+' : result.result === 'exists' ? '=' : 'X';
    const phase = (human.agencyPhase ?? '').padEnd(7);
    const name = result.displayName.padEnd(16);
    const planStr = `[${result.plans.join('+')}]`.padEnd(28);
    const suffix = result.errors.length > 0 ? ` (${result.errors.join('; ')})` : '';
    console.log(
      `  [${icon}] ${name} ${phase} ${planStr} created=${result.created}/${result.plans.length} ${result.conductorUrl}${suffix}`,
    );
  }

  const totalCreated = results.reduce((sum, r) => sum + r.created, 0);
  const failed = results.filter(r => r.result === 'failed').length;
  const succeeded = results.length - failed;

  console.log('');
  console.log(
    `=== Results: ${totalCreated} bindings written, ${succeeded} humans succeeded, ${failed} humans failed ===`,
  );

  if (failed > 0) {
    console.error('\nFailed humans:');
    for (const r of results.filter(rr => rr.result === 'failed')) {
      console.error(`  ${r.displayName} (${r.humanId}): ${r.errors.join('; ')}`);
    }
    process.exit(1);
  }

  process.exit(0);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(err => {
    console.error('FATAL:', err instanceof Error ? err.stack ?? err.message : err);
    process.exit(1);
  });
}
