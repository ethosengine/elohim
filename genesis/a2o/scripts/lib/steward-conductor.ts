/**
 * Steward scripts — the conductor side: connect to a node's OWN conductor and call the
 * lamad `content_store` zome as that node's agent.
 *
 * Existing per-cell signing credentials are reused. Connecting never creates a
 * capability grant; missing credentials require an explicit device ceremony.
 */
import { createPrivateKey, createPublicKey } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

import {
  AdminWebsocket,
  AppWebsocket,
  encodeHashToBase64,
  decodeHashFromBase64,
  setSigningCredentials,
} from '@holochain/client';

import type { AppInfo, CellId, SigningCredentials } from '@holochain/client';

export interface HostedConductorReceipt {
  doorway: string;
  appToken: string;
  appPort: number;
  agentPubKey: string;
  installedAppId: string;
  cellIds: Record<string, [string, string]>;
  token: string;
  expiresAt: number;
}

export interface ConductorOptions {
  /** Existing gateway session, sent only to its issuing HTTPS origin. */
  gatewayAuth?: { doorway: string; authFile: string };
  /** Explicit ceremony output; ordinary connections never call /hc/connect. */
  hosted?: { doorway: string; receipt: string; authFile: string; signingFile: string };
  /** Conductor admin interface, e.g. ws://localhost:4444 */
  adminWs: string;
  /** Conductor app interface, e.g. ws://localhost:4445 */
  appWs: string;
  /** Installed app the storage bridge uses (`HOLOCHAIN_APP_ID`, `elohim` on the fleet). */
  appId: string;
  /** Role whose cell holds the content_store zome (`lamad`). */
  role: string;
  /** Coordinator for this role; publication uses content_store. */
  zome?: string;
  /** Existing storage closed-chain-fence credentials, indexed by exact cell id. */
  signingCredentialsDir?: string;
  /** Refuse accidental connection to a different device or DNA. */
  expectedAgent?: string;
  expectedDna?: string;
}

export interface Conductor {
  /** The cell's agent key, base64 (`uhCAk…`). */
  agent: string;
  dna: string;
  /** Call a `content_store` fn on the role's cell as this node's agent. */
  call<T>(fnName: string, payload: unknown, timeoutMs?: number): Promise<T>;
  close(): Promise<void>;
}

/** Keep gateway credentials out of URLs, logs and unrelated origins. This
 * authenticates transport only; it never authorizes a zome signing grant. */
export async function conductorSocketOptions(
  options: ConductorOptions,
  endpoint: string
): Promise<{ origin: string; headers?: Record<string, string> }> {
  if (!options.gatewayAuth) return { origin: options.appId };
  const refusal = 'gateway credential origin or receipt is invalid';
  try {
    const doorway = new URL(options.gatewayAuth.doorway);
    const socket = new URL(endpoint);
    if (
      doorway.protocol !== 'https:' ||
      socket.protocol !== 'wss:' ||
      doorway.host !== socket.host ||
      doorway.username ||
      doorway.password ||
      socket.username ||
      socket.password ||
      socket.searchParams.has('token')
    )
      throw new Error(refusal);
    const saved = JSON.parse(await readFile(options.gatewayAuth.authFile, 'utf8')) as {
      doorwayUrl?: string;
      token?: string;
      expiresAt?: number;
    };
    if (
      !saved.doorwayUrl ||
      new URL(saved.doorwayUrl).origin !== doorway.origin ||
      typeof saved.token !== 'string' ||
      !saved.token ||
      /[\r\n]/.test(saved.token) ||
      typeof saved.expiresAt !== 'number' ||
      !Number.isFinite(saved.expiresAt) ||
      saved.expiresAt * 1000 <= Date.now()
    )
      throw new Error(refusal);
    return { origin: doorway.origin, headers: { Authorization: `Bearer ${saved.token}` } };
  } catch {
    throw new Error(refusal);
  }
}

/** Find the provisioned cell of `role` in the installed app `appId`, or throw. */
async function findCell(admin: AdminWebsocket, appId: string, role: string): Promise<CellId> {
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === appId);
  if (!app) {
    throw new Error(
      `no installed app "${appId}" (have: ${apps.map(a => a.installed_app_id).join(', ')})`
    );
  }
  return provisionedCell(app, role);
}

function provisionedCell(app: AppInfo, role: string): CellId {
  let cellId: CellId | undefined;
  for (const [r, infos] of Object.entries(app.cell_info)) {
    if (r !== role) continue;
    for (const info of infos as { type: string; value?: { cell_id?: CellId } }[]) {
      if (info.type === 'provisioned' && info.value?.cell_id) cellId = info.value.cell_id;
    }
  }
  if (!cellId) {
    throw new Error(
      `app "${app.installed_app_id}" has no provisioned role "${role}" (roles: ${Object.keys(app.cell_info).join(', ')})`
    );
  }
  return cellId;
}

/** Read the existing storage credential format; never mint replacement keys. */
export async function loadSigningCredentials(
  dir: string,
  cell: CellId
): Promise<SigningCredentials> {
  const name = cell.map(hash => Buffer.from(hash).toString('hex')).join('-');
  let stored: Record<string, unknown>;
  try {
    stored = JSON.parse(await readFile(join(dir, `${name}.json`), 'utf8')) as Record<
      string,
      unknown
    >;
  } catch {
    throw new Error(
      'missing or unreadable signing credentials for this exact cell; explicit enrollment required'
    );
  }
  const bytes = (field: string, length: number): Uint8Array => {
    const value = stored?.[field];
    if (typeof value !== 'string' || !new RegExp(`^[0-9a-fA-F]{${length * 2}}$`).test(value)) {
      throw new Error(`invalid signing credential field: ${field}`);
    }
    return new Uint8Array(Buffer.from(value, 'hex'));
  };
  const seed = bytes('keypair', 32);
  // RFC 8410 PKCS#8 wrapper for the existing Ed25519 seed. No new key is generated.
  const privateKey = createPrivateKey({
    key: Buffer.concat([Buffer.from('302e020100300506032b657004220420', 'hex'), seed]),
    format: 'der',
    type: 'pkcs8',
  });
  const publicKey = new Uint8Array(
    createPublicKey(privateKey).export({ format: 'der', type: 'spki' }).subarray(-32)
  );
  const keyPair = {
    publicKey,
    privateKey: new Uint8Array(Buffer.concat([seed, publicKey])),
    keyType: 'ed25519' as const,
  };
  const signingKey = bytes('signingAgentKey', 39);
  if (!Buffer.from(signingKey.slice(3, 35)).equals(Buffer.from(keyPair.publicKey))) {
    throw new Error('signing credential public key does not match its seed');
  }
  return { signingKey, keyPair, capSecret: bytes('capSecret', 64) };
}

export async function connectConductor(o: ConductorOptions): Promise<Conductor> {
  const dir = o.signingCredentialsDir ?? process.env.STEWARD_SIGNING_CREDENTIALS_DIR;
  if (!dir) {
    throw new Error(
      'existing signing credentials required: set STEWARD_SIGNING_CREDENTIALS_DIR or --signing-credentials-dir'
    );
  }
  if (o.hosted) return connectHosted(o, dir);
  const admin = await AdminWebsocket.connect({
    url: new URL(o.adminWs),
    wsClientOptions: await conductorSocketOptions(o, o.adminWs),
    defaultTimeout: 120_000,
  });
  let appWs: AppWebsocket | undefined;
  try {
    const cell = await findCell(admin, o.appId, o.role);
    const agent = encodeHashToBase64(cell[1]);
    const dna = encodeHashToBase64(cell[0]);
    if (o.expectedAgent && agent !== o.expectedAgent)
      throw new Error('conductor device key mismatch');
    if (o.expectedDna && dna !== o.expectedDna) throw new Error('conductor DNA context mismatch');
    setSigningCredentials(cell, await loadSigningCredentials(dir, cell));
    const token = await admin.issueAppAuthenticationToken({ installed_app_id: o.appId });
    appWs = await AppWebsocket.connect({
      url: new URL(o.appWs),
      token: token.token,
      wsClientOptions: await conductorSocketOptions(o, o.appWs),
      defaultTimeout: 180_000,
    });
    const connected = appWs;
    return {
      agent,
      dna,
      async call<T>(fnName: string, payload: unknown, timeoutMs?: number): Promise<T> {
        return connected.callZome<T>(
          {
            cell_id: cell,
            zome_name: o.zome ?? 'content_store',
            fn_name: fnName,
            payload,
          },
          timeoutMs
        );
      },
      async close() {
        try {
          await (connected.client as unknown as { close(): Promise<unknown> }).close();
        } finally {
          await admin.client.close();
        }
      },
    };
  } catch (error) {
    if (appWs) await (appWs.client as unknown as { close(): Promise<unknown> }).close();
    await admin.client.close();
    throw error;
  }
}

async function connectHosted(o: ConductorOptions, dir: string): Promise<Conductor> {
  const hosted = o.hosted!;
  const receipt = JSON.parse(await readFile(hosted.receipt, 'utf8')) as HostedConductorReceipt;
  if (receipt.doorway !== hosted.doorway || receipt.installedAppId !== o.appId)
    throw new Error('Hosted receipt context mismatch');
  if (!Number.isFinite(receipt.expiresAt) || Date.now() >= receipt.expiresAt)
    throw new Error('Hosted app token expired; explicit credentials ceremony required');
  const pair = receipt.cellIds[o.role];
  if (!pair) throw new Error('Hosted receipt missing requested role');
  const cell: CellId = [decodeHashFromBase64(pair[0]), decodeHashFromBase64(pair[1])];
  const agent = encodeHashToBase64(cell[1]);
  const dna = encodeHashToBase64(cell[0]);
  if (!o.expectedAgent || agent !== o.expectedAgent || agent !== receipt.agentPubKey)
    throw new Error('Hosted device key mismatch');
  if (o.expectedDna && dna !== o.expectedDna) throw new Error('Hosted DNA context mismatch');
  setSigningCredentials(cell, await loadSigningCredentials(dir, cell));
  const url = new URL(`/hc/app/${receipt.appPort}`, hosted.doorway);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  url.searchParams.set('token', receipt.token);
  const app = await AppWebsocket.connect({
    url,
    token: Array.from(Buffer.from(receipt.appToken, 'base64')),
    wsClientOptions: { origin: new URL(hosted.doorway).origin },
    defaultTimeout: 180_000,
  }).catch(() => {
    throw new Error(
      'Hosted app connection failed; renew the explicit credentials ceremony if expired'
    );
  });
  try {
    const info = await app.appInfo();
    const actual = provisionedCell(info, o.role);
    if (
      info.installed_app_id !== o.appId ||
      encodeHashToBase64(actual[0]) !== dna ||
      encodeHashToBase64(actual[1]) !== agent
    )
      throw new Error('Hosted conductor differs from enrolled app/cell');
  } catch (error) {
    await (app.client as unknown as { close(): Promise<unknown> }).close();
    throw error;
  }
  return {
    agent,
    dna,
    async call<T>(fnName: string, payload: unknown, timeoutMs?: number): Promise<T> {
      return app.callZome<T>(
        { cell_id: cell, zome_name: o.zome ?? 'content_store', fn_name: fnName, payload },
        timeoutMs
      );
    },
    async close() {
      await (app.client as unknown as { close(): Promise<unknown> }).close();
    },
  };
}

/**
 * Read the agent key of `role`'s cell straight from a conductor's admin interface —
 * the same key `connectConductor` reports as `agent`, but read-only: it neither
 * authorizes signing credentials nor opens an app interface, so it writes nothing to
 * that conductor's chain. Used to learn a CO-STEWARD's key from its own conductor.
 */
export async function readConductorAgent(
  adminWs: string,
  appId: string,
  role: string,
  timeoutMs = 30_000
): Promise<string> {
  let admin: AdminWebsocket | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const read = (async () => {
    admin = await AdminWebsocket.connect({
      url: new URL(adminWs),
      wsClientOptions: { origin: appId },
      defaultTimeout: timeoutMs,
    });
    const cellId = await findCell(admin, appId, role);
    return encodeHashToBase64(cellId[1]);
  })();
  const deadline = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error(`no answer within ${timeoutMs / 1000}s`)), timeoutMs);
  });
  try {
    return await Promise.race([read, deadline]);
  } finally {
    clearTimeout(timer);
    read.catch(() => undefined);
    await admin?.client.close().catch(() => undefined);
  }
}
