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

export async function connectConductor(o: ConductorOptions): Promise<Conductor> {
  const admin = await AdminWebsocket.connect({
    url: new URL(o.adminWs),
    wsClientOptions: { origin: o.appId },
    defaultTimeout: 120_000,
  });
  const apps = await admin.listApps({});
  const app = apps.find(a => a.installed_app_id === o.appId);
  if (!app) {
    await admin.client.close();
    throw new Error(
      `no installed app "${o.appId}" (have: ${apps.map(a => a.installed_app_id).join(', ')})`
    );
  }
  let cellId: CellId | undefined;
  for (const [role, infos] of Object.entries(app.cell_info)) {
    if (role !== o.role) continue;
    for (const info of infos as { type: string; value?: { cell_id?: CellId } }[]) {
      if (info.type === 'provisioned' && info.value?.cell_id) cellId = info.value.cell_id;
    }
  }
  if (!cellId) {
    await admin.client.close();
    throw new Error(
      `app "${o.appId}" has no provisioned role "${o.role}" (roles: ${Object.keys(app.cell_info).join(', ')})`
    );
  }
  await admin.authorizeSigningCredentials(cellId);
  const token = await admin.issueAppAuthenticationToken({ installed_app_id: app.installed_app_id });
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
