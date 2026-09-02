#!/usr/bin/env npx tsx
/**
 * Seed standing spool-custody commitments for the canonical household.
 *
 * Every ordered pair is authored through the provider's own conductor. The
 * provider and receiver are Holochain agent keys from their lamad cells, never
 * storage/libp2p transport ids.
 *
 * Environment:
 *   CONDUCTOR_URLS   Named or hostname-affine app WebSocket URLs.
 *   INSTALLED_APP_ID Installed app id prefix (default: elohim).
 *   STORAGE_URL      Preferred projection used to resolve family-dowell.
 *   DOORWAY_URL      Fallback projection used to resolve family-dowell.
 */

import { createHash } from 'node:crypto';

import {
  AdminWebsocket,
  AppWebsocket,
  encodeHashToBase64,
  type AppInfo,
} from '@holochain/client';

import {
  extractHumanId,
  parseConductorUrls,
  resolveCandidateUrls,
} from './seed-conductor-identities.js';
import {
  HOUSEHOLD_MEMBERS,
  resolveExistingCollectiveCid,
  type HouseholdMember,
} from './seed-household-formation.js';

type CellId = [Uint8Array, Uint8Array];

export interface SpoolCustodyBounds {
  maxBytes: number;
  atomsPerHour: number;
  retentionDays: number;
}

export interface SpoolCustodyParams {
  providerAgent: string;
  receiverAgent: string;
  collectiveCid: string;
  bounds: SpoolCustodyBounds;
}

export interface SpoolCustodyInput {
  id: string;
  action: 'custody-spool';
  provider: string;
  receiver: string;
  resource_classified_as: string[];
  resource_quantity_value: number;
  resource_quantity_unit: 'B';
  in_scope_of: string[];
  note: string;
  metadata_json: string;
}

export const DEFAULT_SPOOL_CUSTODY_BOUNDS: SpoolCustodyBounds = {
  maxBytes: 64 << 20,
  atomsPerHour: 120,
  retentionDays: 90,
};

/**
 * Build the snake_case `content_store::create_rea_commitment` input.
 *
 * The id mirrors Rust's `deterministic_spool_custody_id(provider, receiver,
 * ward)`. A spool commitment's ward is its receiver, so the third digest
 * component is `spool:witness:<receiver>`.
 */
export function buildSpoolCustodyInput(
  p: SpoolCustodyParams,
): SpoolCustodyInput {
  const classification = `spool:witness:${p.receiverAgent}`;
  const digest = createHash('sha256')
    .update(`${p.providerAgent}|${p.receiverAgent}|${classification}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  return {
    id: `custody-spool-${digest}`,
    action: 'custody-spool',
    provider: p.providerAgent,
    receiver: p.receiverAgent,
    resource_classified_as: [classification],
    resource_quantity_value: p.bounds.maxBytes,
    resource_quantity_unit: 'B',
    in_scope_of: [p.collectiveCid],
    note: `spool custody: ${p.providerAgent} holds ${p.receiverAgent}'s witnesses`,
    metadata_json: JSON.stringify({
      seedGeneration: 'spool-custody',
      kind: 'custody-spool',
      bounds: p.bounds,
    }),
  };
}

const CONNECT_TIMEOUT_MS = Number.parseInt(
  process.env.CONDUCTOR_CONNECT_TIMEOUT_MS ?? '10000',
  10,
);

interface HouseholdSession {
  member: HouseholdMember;
  appWs: AppWebsocket;
  imagodeiCell: CellId;
  lamadCell: CellId;
  agentKey: string;
}

async function withTimeout<T>(promise: Promise<T>, label: string): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(
      () =>
        reject(new Error(`${label} timed out after ${CONNECT_TIMEOUT_MS}ms`)),
      CONNECT_TIMEOUT_MS,
    );
  });
  try {
    return await Promise.race([promise, timeout]);
  } finally {
    if (timer) clearTimeout(timer);
  }
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

function cellForRole(app: AppInfo, role: string): CellId | null {
  const cells = app.cell_info[role];
  if (!cells) return null;

  for (const cell of cells as unknown[]) {
    const candidate = cell as Record<string, unknown>;
    if (candidate['type'] === 'provisioned' && candidate['value']) {
      return (candidate['value'] as Record<string, unknown>)[
        'cell_id'
      ] as CellId;
    }
    if (candidate['provisioned']) {
      return (candidate['provisioned'] as Record<string, unknown>)[
        'cell_id'
      ] as CellId;
    }
  }
  return null;
}

async function connectToConductor(
  appUrl: string,
  appIdPrefix: string,
): Promise<Omit<HouseholdSession, 'member'> | null> {
  const adminUrl = toAdminUrl(appUrl);
  const adminWs = await withTimeout(
    AdminWebsocket.connect({
      url: new URL(adminUrl),
      wsClientOptions: { origin: 'http://localhost' },
    }),
    `Admin connect ${adminUrl}`,
  );

  try {
    const apps = await adminWs.listApps({});
    const app = apps.find((candidate) =>
      candidate.installed_app_id.startsWith(appIdPrefix),
    );
    if (!app) return null;

    const imagodeiCell = cellForRole(app, 'imagodei');
    const lamadCell = cellForRole(app, 'lamad');
    if (!imagodeiCell || !lamadCell) {
      throw new Error(
        `App '${app.installed_app_id}' lacks a provisioned imagodei or lamad cell`,
      );
    }

    await adminWs.authorizeSigningCredentials(imagodeiCell);
    await adminWs.authorizeSigningCredentials(lamadCell);
    const token = await adminWs.issueAppAuthenticationToken({
      installed_app_id: app.installed_app_id,
      single_use: true,
      expiry_seconds: 300,
    });

    const appWs = await withTimeout(
      AppWebsocket.connect({
        url: new URL(appUrl),
        token: token.token,
        wsClientOptions: { origin: 'http://localhost' },
      }),
      `App connect ${appUrl}`,
    );
    return {
      appWs,
      imagodeiCell,
      lamadCell,
      agentKey: encodeHashToBase64(lamadCell[1]),
    };
  } finally {
    await adminWs.client.close();
  }
}

async function findHouseholdSessions(
  rawConductorUrls: string,
  appIdPrefix: string,
): Promise<Map<string, HouseholdSession>> {
  const entries = parseConductorUrls(rawConductorUrls);
  const sessions = new Map<string, HouseholdSession>();

  for (const member of HOUSEHOLD_MEMBERS) {
    for (const conductorUrl of resolveCandidateUrls(member.humanId, entries)) {
      let connected: Omit<HouseholdSession, 'member'> | null = null;
      try {
        connected = await connectToConductor(conductorUrl, appIdPrefix);
        if (!connected) continue;
        const human = await connected.appWs.callZome({
          cell_id: connected.imagodeiCell,
          zome_name: 'imagodei',
          fn_name: 'get_my_human',
          payload: null,
        });
        if (extractHumanId(human) !== member.humanId) {
          await connected.appWs.client.close();
          continue;
        }
        sessions.set(member.humanId, { member, ...connected });
        console.log(
          `  [=] ${member.humanId.padEnd(22)} ${conductorUrl} ${connected.agentKey.slice(0, 16)}…`,
        );
        break;
      } catch (error) {
        if (connected) await connected.appWs.client.close();
        console.error(
          `  [X] ${member.humanId} at ${conductorUrl}: ${error instanceof Error ? error.message : error}`,
        );
      }
    }
  }

  return sessions;
}

async function closeSessions(
  sessions: Map<string, HouseholdSession>,
): Promise<void> {
  await Promise.allSettled(
    [...sessions.values()].map((session) => session.appWs.client.close()),
  );
}

async function main(): Promise<void> {
  const conductorUrls = process.env.CONDUCTOR_URLS ?? '';
  if (!conductorUrls) {
    throw new Error('CONDUCTOR_URLS is required');
  }

  const projectionUrl =
    process.env.STORAGE_URL ?? process.env.DOORWAY_URL ?? '';
  if (!projectionUrl) {
    throw new Error(
      'STORAGE_URL or DOORWAY_URL is required to resolve family-dowell',
    );
  }

  const collectiveCid = await resolveExistingCollectiveCid(projectionUrl);
  if (!collectiveCid) {
    throw new Error(
      `family-dowell collective is not projected at ${projectionUrl}`,
    );
  }

  console.log('=== Seed Spool Custody (provider-authored) ===\n');
  console.log(`Household: ${collectiveCid}`);
  console.log('Binding household conductors:');

  const sessions = await findHouseholdSessions(
    conductorUrls,
    process.env.INSTALLED_APP_ID ?? 'elohim',
  );

  let authored = 0;
  let existing = 0;
  let failed = 0;

  try {
    for (const provider of sessions.values()) {
      for (const receiver of sessions.values()) {
        if (provider.member.humanId === receiver.member.humanId) continue;

        const input = buildSpoolCustodyInput({
          providerAgent: provider.agentKey,
          receiverAgent: receiver.agentKey,
          collectiveCid,
          bounds: DEFAULT_SPOOL_CUSTODY_BOUNDS,
        });

        try {
          const prior = await provider.appWs.callZome({
            cell_id: provider.lamadCell,
            zome_name: 'content_store',
            fn_name: 'get_rea_commitment',
            payload: input.id,
          });
          if (prior) {
            existing += 1;
            console.log(
              `  [=] ${provider.member.humanId} -> ${receiver.member.humanId} (${input.id})`,
            );
            continue;
          }

          await provider.appWs.callZome({
            cell_id: provider.lamadCell,
            zome_name: 'content_store',
            fn_name: 'create_rea_commitment',
            payload: input,
          });
          authored += 1;
          console.log(
            `  [+] ${provider.member.humanId} -> ${receiver.member.humanId} (${input.id})`,
          );
        } catch (error) {
          failed += 1;
          console.error(
            `  [X] ${provider.member.humanId} -> ${receiver.member.humanId}: ${error instanceof Error ? error.message : error}`,
          );
        }
      }
    }
  } finally {
    await closeSessions(sessions);
  }

  const expected = HOUSEHOLD_MEMBERS.length * (HOUSEHOLD_MEMBERS.length - 1);
  const complete =
    sessions.size === HOUSEHOLD_MEMBERS.length &&
    authored + existing === expected;
  console.log(
    `\n=== Results: sessions=${sessions.size}/${HOUSEHOLD_MEMBERS.length}, authored=${authored}, existing=${existing}, failed=${failed} ===`,
  );
  process.exit(complete && failed === 0 ? 0 : 2);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(
      'FATAL:',
      error instanceof Error ? (error.stack ?? error.message) : error,
    );
    process.exit(1);
  });
}
