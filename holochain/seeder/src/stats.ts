/**
 * Lightweight stats check - just queries content count without seeding
 */

import { AdminWebsocket, AppWebsocket, encodeHashToBase64, CellId } from '@holochain/client';
import * as fs from 'fs';

const HC_PORTS_FILE = process.env.HC_PORTS_FILE || '/projects/elohim/holochain/local-dev/.hc_ports';
const APP_ID = 'lamad-spike';
const ROLE_NAME = 'lamad';
const ZOME_NAME = 'content_store';

function readHcPorts(): { adminPort: number; appPort: number } {
  try {
    const content = fs.readFileSync(HC_PORTS_FILE, 'utf-8');
    const adminMatch = content.match(/admin_port=(\d+)/);
    const appMatch = content.match(/app_port=(\d+)/);
    if (!adminMatch || !appMatch) throw new Error('Could not parse .hc_ports file');
    return { adminPort: parseInt(adminMatch[1], 10), appPort: parseInt(appMatch[1], 10) };
  } catch {
    return { adminPort: 4444, appPort: 4445 };
  }
}

const ports = readHcPorts();
const ADMIN_WS_URL = process.env.HOLOCHAIN_ADMIN_URL || `ws://localhost:${ports.adminPort}`;

function resolveAppUrl(adminUrl: string, port: number): string {
  if (!adminUrl.includes('localhost') && !adminUrl.includes('127.0.0.1')) {
    const url = new URL(adminUrl);
    const baseUrl = `${url.protocol}//${url.host}`;
    const apiKey = url.searchParams.get('apiKey');
    const apiKeyParam = apiKey ? `?apiKey=${encodeURIComponent(apiKey)}` : '';
    return `${baseUrl}/app/${port}${apiKeyParam}`;
  }
  return `ws://localhost:${port}`;
}

async function getStats() {
  try {
    const adminWs = await AdminWebsocket.connect({
      url: new URL(ADMIN_WS_URL),
      wsClientOptions: { origin: 'http://localhost' },
    });

    const apps = await adminWs.listApps({});
    const app = apps.find((a) => a.installed_app_id === APP_ID);
    if (!app) {
      console.log('total_count: 0');
      process.exit(0);
    }

    const cellInfo = app.cell_info[ROLE_NAME];
    if (!cellInfo || cellInfo.length === 0) {
      console.log('total_count: 0');
      process.exit(0);
    }

    const provisionedCell = cellInfo.find((c: any) => c.type === 'provisioned');
    if (!provisionedCell) {
      console.log('total_count: 0');
      process.exit(0);
    }

    const rawCellId = (provisionedCell as any).value.cell_id;
    function toUint8Array(val: any): Uint8Array {
      if (val instanceof Uint8Array) return val;
      if (val?.type === 'Buffer' && Array.isArray(val.data)) return new Uint8Array(val.data);
      if (ArrayBuffer.isView(val)) return new Uint8Array(val.buffer);
      throw new Error(`Cannot convert to Uint8Array`);
    }
    const cellId: CellId = [toUint8Array(rawCellId[0]), toUint8Array(rawCellId[1])];

    const token = await adminWs.issueAppAuthenticationToken({
      installed_app_id: APP_ID,
      single_use: false,
      expiry_seconds: 60,
    });

    await adminWs.authorizeSigningCredentials(cellId);

    const existingInterfaces = await adminWs.listAppInterfaces();
    const appPort = existingInterfaces.length > 0 ? existingInterfaces[0].port : ports.appPort;
    const appWsUrl = process.env.HOLOCHAIN_APP_URL || resolveAppUrl(ADMIN_WS_URL, appPort);

    const appWs = await AppWebsocket.connect({
      url: new URL(appWsUrl),
      wsClientOptions: { origin: 'http://localhost' },
      token: token.token,
    });

    // Get content stats
    const stats = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'get_content_stats',
      payload: null,
    }) as any;

    // Get path count
    const paths = await appWs.callZome({
      cell_id: cellId,
      zome_name: ZOME_NAME,
      fn_name: 'get_all_paths',
      payload: null,
    }) as any;

    const contentCount = stats?.total || stats?.count || 0;
    const pathCount = paths?.total_count || 0;

    console.log(`total_count: ${contentCount + pathCount}`);

    await adminWs.client.close();
    await appWs.client.close();
  } catch (error: any) {
    // If we can't connect, assume no content
    console.log('total_count: 0');
  }
}

getStats();
