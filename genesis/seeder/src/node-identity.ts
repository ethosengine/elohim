/**
 * Which agent a household node speaks as — read from that node's OWN conductor.
 *
 * "Which agent does this node speak as" is a fact about the node's own cell,
 * and the conductor is where that fact lives. Storage's `GET /auth/me` answers
 * a different question (which person is signed in here) and, since the
 * device-consent landing, only to a caller on the node's own machine — the
 * seeders were reading the first question through the second
 * (genesis/data/timeline/backlog/genesis-seeders-read-node-agent-key-from-closed-auth-me.md).
 *
 * The read adds no route, credential or roll: genesis CI already reaches every
 * household conductor through CONDUCTOR_URLS (genesis/Jenkinsfile
 * getConductorAppUrls), the same reach seed-conductor-identities uses.
 *
 * Affinity is NAME-ONLY: a human resolves through an explicit `name=url`
 * CONDUCTOR_URLS entry or the `elohim-<name>-<env>` hostname convention
 * (conductorUrlForHuman). A human with no affine conductor resolves to
 * nothing — never the first conductor that answers.
 *
 * Env: CONDUCTOR_URLS (required), INSTALLED_APP_ID (default `elohim`),
 * CONDUCTOR_CONNECT_TIMEOUT_MS (default 10000). Admin port = app port - 1.
 */

import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';
import { selectSeedCell } from './cell-target.js';
import {
  conductorUrlForHuman,
  extractHumanId,
  parseConductorUrls,
  selectStewardApp,
} from './seed-conductor-identities.js';

export interface NodeIdentity {
  humanId: string;
  conductorUrl: string;
  /** base64 AgentPubKey of the steward app on the human's own conductor. */
  agentPubKey: string;
  /** The Human id the steward cell embodies (`get_my_human`), or null if none yet. */
  embodiedHumanId: string | null;
}

/** What one conductor says about its steward app — the injectable seam. */
export interface StewardCellReading {
  agentPubKey: string;
  embodiedHumanId: string | null;
}

/**
 * Read the steward app on the conductor at `appUrl`. Returns null when the
 * conductor has no steward app; throws on a connect/timeout failure.
 */
export type ConductorReader = (
  appUrl: string,
  appIdPrefix: string,
  timeoutMs: number,
) => Promise<StewardCellReading | null>;

export interface NodeIdentityResolverOptions {
  conductorUrls?: string;
  appIdPrefix?: string;
  timeoutMs?: number;
  reader?: ConductorReader;
}

export interface NodeIdentityResolver {
  /** Resolve `humanId`'s node identity; throws a message naming the human and URL tried. */
  resolve(humanId: string): Promise<NodeIdentity>;
  /** The affine conductor URL for `humanId`, or null — no I/O. */
  conductorUrlFor(humanId: string): string | null;
}

function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${label} timed out after ${ms}ms`)), ms);
  });
  return Promise.race([promise, timeout]).finally(() => timer && clearTimeout(timer));
}

function adminUrlFor(appUrl: string): URL {
  const u = new URL(appUrl);
  const port = parseInt(u.port, 10);
  if (isNaN(port)) throw new Error(`cannot derive admin port from ${appUrl}`);
  u.port = String(port - 1);
  return u;
}

/** The real reader: AppInfo.agent_pub_key over admin, then get_my_human over app. */
export const readStewardCell: ConductorReader = async (appUrl, appIdPrefix, timeoutMs) => {
  const adminUrl = adminUrlFor(appUrl);
  const adminWs = await withTimeout(
    AdminWebsocket.connect({ url: adminUrl, wsClientOptions: { origin: 'http://localhost' } }),
    timeoutMs,
    `admin connect ${adminUrl}`,
  );
  let appWs: AppWebsocket | undefined;
  try {
    const app = selectStewardApp(await adminWs.listApps({}), appIdPrefix);
    if (!app) return null;
    const agentPubKey = encodeHashToBase64(app.agent_pub_key);
    const cellId = selectSeedCell(app.cell_info, 'imagodei');
    await adminWs.authorizeSigningCredentials(cellId);
    const { token } = await adminWs.issueAppAuthenticationToken({
      installed_app_id: app.installed_app_id,
      single_use: true,
      expiry_seconds: 60,
    });
    appWs = await withTimeout(
      AppWebsocket.connect({ url: new URL(appUrl), token, wsClientOptions: { origin: 'http://localhost' } }),
      timeoutMs,
      `app connect ${appUrl}`,
    );
    const human = await withTimeout(
      appWs.callZome({ cell_id: cellId, zome_name: 'imagodei', fn_name: 'get_my_human', payload: null }),
      timeoutMs,
      `get_my_human ${appUrl}`,
    );
    return { agentPubKey, embodiedHumanId: extractHumanId(human) ?? null };
  } finally {
    if (appWs) {
      await Promise.resolve((appWs.client as unknown as { close(): unknown }).close()).catch(
        () => undefined,
      );
    }
    await adminWs.client.close().catch(() => undefined);
  }
};

export function createNodeIdentityResolver(
  opts: NodeIdentityResolverOptions = {},
): NodeIdentityResolver {
  const entries = parseConductorUrls(opts.conductorUrls ?? process.env.CONDUCTOR_URLS ?? '');
  const appIdPrefix = opts.appIdPrefix ?? process.env.INSTALLED_APP_ID ?? 'elohim';
  const timeoutMs =
    opts.timeoutMs ?? parseInt(process.env.CONDUCTOR_CONNECT_TIMEOUT_MS ?? '10000', 10);
  const reader = opts.reader ?? readStewardCell;
  const cache = new Map<string, Promise<NodeIdentity>>();

  const conductorUrlFor = (humanId: string): string | null =>
    conductorUrlForHuman(humanId, entries);

  async function resolveUncached(humanId: string): Promise<NodeIdentity> {
    const url = conductorUrlFor(humanId);
    if (!url) {
      throw new Error(
        `${humanId}: no conductor of its own in CONDUCTOR_URLS ` +
          `(${entries.length} entr${entries.length === 1 ? 'y' : 'ies'}; needs name=url or elohim-<name>-<env>)`,
      );
    }
    let reading: StewardCellReading | null;
    try {
      reading = await reader(url, appIdPrefix, timeoutMs);
    } catch (err) {
      throw new Error(
        `${humanId}: conductor ${url} unreadable — ${err instanceof Error ? err.message : String(err)}`,
      );
    }
    if (!reading) {
      throw new Error(`${humanId}: conductor ${url} has no steward app '${appIdPrefix}'`);
    }
    if (!reading.agentPubKey.startsWith('uhCAk')) {
      throw new Error(`${humanId}: conductor ${url} did not return a Holochain agentPubKey`);
    }
    return { humanId, conductorUrl: url, ...reading };
  }

  return {
    conductorUrlFor,
    resolve(humanId: string): Promise<NodeIdentity> {
      let hit = cache.get(humanId);
      if (!hit) {
        hit = resolveUncached(humanId);
        cache.set(humanId, hit);
      }
      return hit;
    },
  };
}

let defaultResolver: NodeIdentityResolver | undefined;

/** Run-scoped default resolver built from env (one cache per process). */
export function defaultNodeIdentityResolver(): NodeIdentityResolver {
  defaultResolver ??= createNodeIdentityResolver();
  return defaultResolver;
}

/** Resolve through the run-scoped default resolver. */
export function resolveNodeIdentity(humanId: string): Promise<NodeIdentity> {
  return defaultNodeIdentityResolver().resolve(humanId);
}
