/**
 * Read-only AgentPeerBinding provenance inventory.
 *
 * This utility invokes only `imagodei.get_agent_peer_bindings`; it never calls
 * create, update, delete, or supersede externs.  It enumerates the forward
 * links rooted at each supplied conductor identity, then reports bindings
 * whose `agentCid` names a different human than the anchor's conductor.
 *
 * The report deliberately does not decide whether the link anchor or
 * `agentCid` is authoritative.  Its role is evidence preparation for that
 * architecture decision.  `get_agent_peer_bindings` filters superseded rows,
 * so this is an inventory of currently-active bindings, not a full history.
 *
 * Required environment:
 *   CONDUCTOR_URLS  comma-separated app WebSocket URLs
 * Optional environment:
 *   INSTALLED_APP_ID  app id prefix (default: elohim)
 *   REPORT_PATH       local JSON destination; absent means stdout only
 *
 * Example:
 *   CONDUCTOR_URLS=ws://elohim-adam-alpha:4445,... \
 *     pnpm --dir genesis/seeder enumerate:agent-bindings
 */

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';

import {
  inventoryBindingProvenance,
  type AgentPeerBindingView,
  type AnchorBindings,
} from './agent-peer-binding-provenance.js';
import { conductorUrlForHuman } from './seed-conductor-identities.js';

interface Human {
  id: string;
  agencyPhase?: string;
}

interface HumansJson {
  humans: Human[];
}

interface ConductorSession {
  appWs: AppWebsocket;
  cellId: [Uint8Array, Uint8Array];
  agentPubKey: Uint8Array;
}

function toAdminUrl(appUrl: string): string {
  const url = new URL(appUrl);
  const appPort = Number.parseInt(url.port, 10);
  if (Number.isNaN(appPort)) {
    throw new Error(`Cannot derive admin port from app URL: ${appUrl}`);
  }
  url.port = String(appPort - 1);
  return url.toString();
}

/** Acquire an ephemeral app token, then perform only an app-interface query. */
async function connectForRead(
  appUrl: string,
  appIdPrefix: string,
): Promise<ConductorSession | null> {
  const adminWs = await AdminWebsocket.connect({
    url: new URL(toAdminUrl(appUrl)),
    wsClientOptions: { origin: 'http://localhost' },
  });

  try {
    const apps = await adminWs.listApps({});
    const app = apps.find(candidate => candidate.installed_app_id.startsWith(appIdPrefix));
    if (!app) return null;

    const cells = app.cell_info.imagodei;
    if (!cells?.length) {
      throw new Error(`App '${app.installed_app_id}' has no imagodei cell`);
    }
    const cell = cells.find(candidate => {
      const value = candidate as Record<string, unknown>;
      return (value.type === 'provisioned' && value.value) || value.provisioned;
    }) as Record<string, unknown> | undefined;
    if (!cell) throw new Error(`App '${app.installed_app_id}' imagodei cell is not provisioned`);

    const provisioned = (cell.value ?? cell.provisioned) as Record<string, unknown>;
    const cellId = provisioned.cell_id as [Uint8Array, Uint8Array];
    const token = await adminWs.issueAppAuthenticationToken({
      installed_app_id: app.installed_app_id,
      single_use: true,
      expiry_seconds: 300,
    });
    const appWs = await AppWebsocket.connect({
      url: new URL(appUrl),
      token: token.token,
      wsClientOptions: { origin: 'http://localhost' },
    });
    return { appWs, cellId, agentPubKey: cellId[1] };
  } finally {
    await adminWs.client.close();
  }
}

async function activeBindings(
  appWs: AppWebsocket,
  cellId: [Uint8Array, Uint8Array],
  agentPubKey: Uint8Array,
): Promise<AgentPeerBindingView[]> {
  return appWs.callZome({
    cell_id: cellId,
    zome_name: 'imagodei',
    fn_name: 'get_agent_peer_bindings',
    payload: agentPubKey,
  }) as Promise<AgentPeerBindingView[]>;
}

function expectedHumanId(url: string, humanIds: string[]): string | null {
  return humanIds.find(id => conductorUrlForHuman(id, [url]) === url) ?? null;
}

function readTargetHumanIds(): string[] {
  const jsonPath = resolve(dirname(fileURLToPath(import.meta.url)), '../../data/humans/humans.json');
  if (!existsSync(jsonPath)) {
    throw new Error(`${jsonPath} not found; run pnpm --dir genesis/seeder build:data first`);
  }
  const data: HumansJson = JSON.parse(readFileSync(jsonPath, 'utf8'));
  return data.humans
    .filter(human => ['node', 'device', 'doorway'].includes(human.agencyPhase ?? ''))
    .map(human => human.id);
}

export async function main(): Promise<void> {
  const urls = (process.env.CONDUCTOR_URLS ?? '')
    .split(',')
    .map(url => url.trim())
    .filter(Boolean);
  if (!urls.length) {
    throw new Error('CONDUCTOR_URLS is required (comma-separated app WebSocket URLs)');
  }

  const appIdPrefix = process.env.INSTALLED_APP_ID ?? 'elohim';
  const humanIds = readTargetHumanIds();
  const snapshots: AnchorBindings[] = [];
  const failures: Array<{ conductorUrl: string; error: string }> = [];

  for (const conductorUrl of urls) {
    let session: ConductorSession | null = null;
    try {
      session = await connectForRead(conductorUrl, appIdPrefix);
      if (!session) {
        failures.push({ conductorUrl, error: `no installed app beginning '${appIdPrefix}'` });
        continue;
      }
      const bindings = await activeBindings(session.appWs, session.cellId, session.agentPubKey);
      snapshots.push({
        conductorUrl,
        anchorAgentPubKey: encodeHashToBase64(session.agentPubKey),
        expectedHumanId: expectedHumanId(conductorUrl, humanIds),
        bindings,
      });
    } catch (error) {
      failures.push({
        conductorUrl,
        error: error instanceof Error ? error.message : String(error),
      });
    } finally {
      if (session) await session.appWs.client.close().catch(() => undefined);
    }
  }

  const inventory = inventoryBindingProvenance(snapshots);
  const report = {
    schemaVersion: '1',
    generatedAt: new Date().toISOString(),
    script: 'enumerate-agent-peer-bindings',
    readOnly: true,
    limitations: [
      'Only active rows are visible: get_agent_peer_bindings filters superseded bindings.',
      'A missing peer may mean an unintegrated DHT op, not that the binding never existed.',
      'This classifies the AgentToPeerBinding anchor versus agentCid; it does not choose authority.',
    ],
    queriedConductors: snapshots.map(({ bindings, ...anchor }) => ({
      ...anchor,
      activeBindings: bindings.length,
    })),
    queryFailures: failures,
    summary: inventory.summary,
    misProvenanced: inventory.rows.filter(row => row.classification === 'mis-provenanced'),
    correctlyAnchored: inventory.rows.filter(row => row.classification === 'correctly-anchored'),
    unattributedAnchors: inventory.rows.filter(row => row.classification === 'unattributed-anchor'),
  };

  const serialized = `${JSON.stringify(report, null, 2)}\n`;
  if (process.env.REPORT_PATH) writeFileSync(process.env.REPORT_PATH, serialized);
  process.stdout.write(serialized);
  if (failures.length > 0) process.exitCode = 2;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch(error => {
    console.error(error instanceof Error ? error.stack ?? error.message : error);
    process.exit(1);
  });
}
