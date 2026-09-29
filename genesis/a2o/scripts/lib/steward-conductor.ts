/**
 * Steward scripts — the conductor side: connect to a node's OWN conductor and call the
 * lamad `content_store` zome as that node's agent.
 *
 * The authority is the conductor, not a storage header: the admin websocket authorizes
 * signing credentials for the app's cell and issues an app token, and every call is
 * signed as the cell's agent. Shared by scripts/steward-publish.ts and
 * scripts/steward-grade.ts.
 *
 * Note: `authorizeSigningCredentials` writes a capability grant to the agent's source
 * chain, so a caller that must stay write-free connects only when it has to.
 */
import { AdminWebsocket, AppWebsocket, encodeHashToBase64 } from '@holochain/client';

import type { CellId } from '@holochain/client';

export interface ConductorOptions {
  /** Conductor admin interface, e.g. ws://localhost:4444 */
  adminWs: string;
  /** Conductor app interface, e.g. ws://localhost:4445 */
  appWs: string;
  /** Installed app the storage bridge uses (`HOLOCHAIN_APP_ID`, `elohim` on the fleet). */
  appId: string;
  /** Role whose cell holds the content_store zome (`lamad`). */
  role: string;
}

export interface Conductor {
  /** The cell's agent key, base64 (`uhCAk…`). */
  agent: string;
  /** Call a `content_store` fn on the role's cell as this node's agent. */
  call<T>(fnName: string, payload: unknown): Promise<T>;
  close(): Promise<void>;
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
  let cellId: CellId | undefined;
  for (const [r, infos] of Object.entries(app.cell_info)) {
    if (r !== role) continue;
    for (const info of infos as { type: string; value?: { cell_id?: CellId } }[]) {
      if (info.type === 'provisioned' && info.value?.cell_id) cellId = info.value.cell_id;
    }
  }
  if (!cellId) {
    throw new Error(
      `app "${appId}" has no provisioned role "${role}" (roles: ${Object.keys(app.cell_info).join(', ')})`
    );
  }
  return cellId;
}

export async function connectConductor(o: ConductorOptions): Promise<Conductor> {
  const admin = await AdminWebsocket.connect({
    url: new URL(o.adminWs),
    wsClientOptions: { origin: o.appId },
    defaultTimeout: 120_000,
  });
  let cellId: CellId;
  try {
    cellId = await findCell(admin, o.appId, o.role);
  } catch (e) {
    await admin.client.close();
    throw e;
  }
  await admin.authorizeSigningCredentials(cellId);
  const token = await admin.issueAppAuthenticationToken({ installed_app_id: o.appId });
  const appWs = await AppWebsocket.connect({
    url: new URL(o.appWs),
    token: token.token,
    wsClientOptions: { origin: o.appId },
    defaultTimeout: 180_000,
  });
  const cell = cellId;
  return {
    agent: encodeHashToBase64(cell[1]),
    async call<T>(fnName: string, payload: unknown): Promise<T> {
      const out: T = await appWs.callZome({
        cell_id: cell,
        zome_name: 'content_store',
        fn_name: fnName,
        payload,
      });
      return out;
    },
    async close() {
      await (appWs.client as unknown as { close(): Promise<unknown> }).close();
      await admin.client.close();
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
