/**
 * Holochain Connection Utilities
 *
 * # RNA Metaphor: Cell Membrane
 *
 * The cell membrane controls what enters and exits the cell.
 * These utilities manage the connection boundary between the
 * migration tool and Holochain cells.
 */

import {
  AdminWebsocket,
  AppWebsocket,
  CellId,
  encodeHashToBase64,
} from '@holochain/client';
import * as fs from 'fs';
import { ConnectionConfig } from './config.js';

/**
 * Port configuration from .hc_ports file
 */
export interface PortsConfig {
  adminPort: number;
  appPort: number;
}

/**
 * Active Holochain connection with both websockets and cell IDs
 */
export interface HolochainConnection {
  /** Admin WebSocket for conductor management */
  adminWs: AdminWebsocket;
  /** App WebSocket for zome calls */
  appWs: AppWebsocket;
  /** Cell ID of the source DNA */
  sourceCellId: CellId;
  /** Cell ID of the target DNA */
  targetCellId: CellId;
}

/**
 * Read Holochain ports from .hc_ports file
 *
 * The .hc_ports file is created by `hc sandbox` and contains:
 * ```
 * admin_port=4444
 * app_port=4445
 * ```
 */
export function readPorts(portsFile: string): PortsConfig {
  try {
    const content = fs.readFileSync(portsFile, 'utf-8');
    const adminMatch = content.match(/admin_port=(\d+)/);
    const appMatch = content.match(/app_port=(\d+)/);

    if (!adminMatch || !appMatch) {
      throw new Error('Could not parse .hc_ports file');
    }

    return {
      adminPort: parseInt(adminMatch[1], 10),
      appPort: parseInt(appMatch[1], 10),
    };
  } catch (error) {
    console.warn(`Could not read ${portsFile}, using defaults`);
    return { adminPort: 4444, appPort: 4445 };
  }
}

/**
 * Resolve app WebSocket URL based on admin URL
 *
 * For local connections: `ws://localhost:{port}`
 * For remote connections (proxy): `{baseUrl}/app/{port}`
 */
export function resolveAppUrl(adminUrl: string, port: number): string {
  if (!adminUrl.includes('localhost') && !adminUrl.includes('127.0.0.1')) {
    // Remote connection - route through proxy
    const url = new URL(adminUrl);
    const baseUrl = `${url.protocol}//${url.host}`;
    const apiKey = url.searchParams.get('apiKey');
    const apiKeyParam = apiKey ? `?apiKey=${encodeURIComponent(apiKey)}` : '';
    return `${baseUrl}/app/${port}${apiKeyParam}`;
  }
  // Local connection - direct WebSocket
  return `ws://localhost:${port}`;
}

/**
 * Extract CellId from cell info structure
 *
 * Handles both provisioned and cloned cells.
 */
export function extractCellId(cellInfo: unknown): CellId | null {
  if (Array.isArray(cellInfo)) {
    for (const info of cellInfo) {
      if (info && typeof info === 'object') {
        if ('provisioned' in info && info.provisioned) {
          return (info.provisioned as { cell_id: CellId }).cell_id;
        }
        if ('cloned' in info && info.cloned) {
          return (info.cloned as { cell_id: CellId }).cell_id;
        }
      }
    }
  }
  return null;
}

/**
 * Format CellId for display (truncated hash)
 */
export function formatCellId(cellId: CellId): string {
  return `${encodeHashToBase64(cellId[0]).slice(0, 12)}...`;
}

/**
 * Connect to Holochain and get both source and target cell connections
 *
 * # Example
 *
 * ```typescript
 * const conn = await connect({
 *   adminUrl: 'ws://localhost:4444',
 *   appId: 'my-app',
 *   sourceRole: 'my-dna-v1',
 *   targetRole: 'my-dna-v2',
 * });
 *
 * // Use conn.appWs to make zome calls
 * // Use conn.sourceCellId and conn.targetCellId for cell targeting
 * ```
 */
export async function connect(config: {
  adminUrl: string;
  appId: string;
  sourceRole: string;
  targetRole: string;
}): Promise<HolochainConnection> {
  // Connect to admin WebSocket
  console.log(`Connecting to admin: ${config.adminUrl}`);
  const adminWs = await AdminWebsocket.connect({
    url: new URL(config.adminUrl),
  });

  // List apps and find ours
  const apps = await adminWs.listApps({});
  const app = apps.find((a) => a.installed_app_id === config.appId);

  if (!app) {
    const available = apps.map((a) => a.installed_app_id).join(', ');
    throw new Error(
      `App "${config.appId}" not found. Available: ${available || 'none'}`
    );
  }

  // Get cell info for both roles
  const cells = app.cell_info;
  const sourceCell = cells[config.sourceRole];
  const targetCell = cells[config.targetRole];

  if (!sourceCell) {
    const available = Object.keys(cells).join(', ');
    throw new Error(
      `Source role "${config.sourceRole}" not found. Available: ${available}`
    );
  }

  if (!targetCell) {
    const available = Object.keys(cells).join(', ');
    throw new Error(
      `Target role "${config.targetRole}" not found. Available: ${available}`
    );
  }

  // Extract cell IDs
  const sourceCellId = extractCellId(sourceCell);
  const targetCellId = extractCellId(targetCell);

  if (!sourceCellId) {
    throw new Error(`Could not extract cell ID for source role "${config.sourceRole}"`);
  }
  if (!targetCellId) {
    throw new Error(`Could not extract cell ID for target role "${config.targetRole}"`);
  }

  console.log(`Source cell: ${formatCellId(sourceCellId)}`);
  console.log(`Target cell: ${formatCellId(targetCellId)}`);

  // Get app auth token
  const token = await adminWs.issueAppAuthenticationToken({
    installed_app_id: config.appId,
    single_use: false,
    expiry_seconds: 3600,
  });

  // Authorize signing credentials for target cell (we'll be writing there)
  await adminWs.authorizeSigningCredentials(targetCellId);

  // Get or create app interface
  const interfaces = await adminWs.listAppInterfaces();
  let appPort: number;

  if (interfaces.length > 0) {
    appPort = interfaces[0].port;
    console.log(`Using existing app interface on port ${appPort}`);
  } else {
    const { port } = await adminWs.attachAppInterface({
      allowed_origins: '*',
    });
    appPort = port;
    console.log(`Created app interface on port ${appPort}`);
  }

  // Connect to app WebSocket
  const appWsUrl = resolveAppUrl(config.adminUrl, appPort);
  console.log(`Connecting to app: ${appWsUrl}`);

  const appWs = await AppWebsocket.connect({
    url: new URL(appWsUrl),
    wsClientOptions: { origin: 'http://localhost' },
    token: token.token,
  });

  return { adminWs, appWs, sourceCellId, targetCellId };
}

/**
 * Close both WebSocket connections
 */
export async function disconnect(conn: HolochainConnection): Promise<void> {
  await conn.appWs.client.close();
  await conn.adminWs.client.close();
}
