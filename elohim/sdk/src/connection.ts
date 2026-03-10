/**
 * Holochain Connection Manager
 *
 * Manages AdminWebsocket and AppWebsocket connections to the Holochain conductor.
 * Similar to a DataSource in Spring - handles connection lifecycle and provides
 * access to the underlying client for zome calls.
 */

import {
  AdminWebsocket,
  AppWebsocket,
  type AppInfo,
  type CellId,
} from '@holochain/client';
import { ConnectionConfig, DEFAULT_APP_ID, DEFAULT_ROLE_ID } from './types.js';

export interface ConnectionState {
  isConnected: boolean;
  adminUrl: string | null;
  appUrl: string | null;
  appId: string | null;
  cellId: CellId | null;
}

/**
 * Connection manager for Holochain conductor
 *
 * Handles:
 * - AdminWebsocket connection (for app installation, agent key generation)
 * - AppWebsocket connection (for zome calls)
 * - Cell ID resolution
 * - Reconnection logic
 */
export class HolochainConnection {
  private adminWs: AdminWebsocket | null = null;
  private appWs: AppWebsocket | null = null;
  private cellId: CellId | null = null;
  private config: ConnectionConfig;
  private connectionPromise: Promise<void> | null = null;

  constructor(config: ConnectionConfig) {
    this.config = {
      timeout: 30000,
      appId: DEFAULT_APP_ID,
      roleId: DEFAULT_ROLE_ID,
      ...config,
    };
  }

  /**
   * Connect to the Holochain conductor
   *
   * 1. Connects to admin interface
   * 2. Ensures app is installed
   * 3. Connects to app interface
   * 4. Resolves cell ID for zome calls
   */
  async connect(): Promise<void> {
    // Prevent multiple simultaneous connection attempts
    if (this.connectionPromise) {
      return this.connectionPromise;
    }

    this.connectionPromise = this._connect();

    try {
      await this.connectionPromise;
    } finally {
      this.connectionPromise = null;
    }
  }

  private async _connect(): Promise<void> {
    // Connect to admin interface
    console.log(`[SDK] Connecting to admin: ${this.config.adminUrl}`);
    this.adminWs = await AdminWebsocket.connect({
      url: new URL(this.config.adminUrl),
    });

    // List apps to check if our app is installed
    const apps = await this.adminWs.listApps({});
    const appInfo = apps.find(
      (app: AppInfo) => app.installed_app_id === this.config.appId
    );

    if (!appInfo) {
      throw new Error(
        `App '${this.config.appId}' not installed. Install the hApp first.`
      );
    }

    // Get app port from app info or config
    let appUrl = this.config.appUrl;
    if (!appUrl) {
      // Derive app URL from admin URL (assume same host, port 4445)
      const adminUrlObj = new URL(this.config.adminUrl);
      // Check if using dev-proxy pattern (/admin, /app/:port)
      if (adminUrlObj.pathname === '/admin') {
        appUrl = `${adminUrlObj.protocol}//${adminUrlObj.host}/app/4445`;
      } else {
        adminUrlObj.port = '4445';
        appUrl = adminUrlObj.toString();
      }
    }

    // Connect to app interface
    console.log(`[SDK] Connecting to app: ${appUrl}`);
    this.appWs = await AppWebsocket.connect({
      url: new URL(appUrl),
    });

    // Resolve cell ID
    const cellInfoArray = appInfo.cell_info[this.config.roleId!];
    if (!cellInfoArray || cellInfoArray.length === 0) {
      throw new Error(`No cells found for role '${this.config.roleId}'`);
    }

    // Get provisioned cell - CellInfo varies by Holochain client version
    // Cast through unknown to handle type variations
    let foundCellId: CellId | null = null;
    for (const info of cellInfoArray) {
      const infoAny = info as unknown as { type?: string; value?: { cell_id: CellId }; provisioned?: { cell_id: CellId } };

      // Handle different CellInfo formats
      if (infoAny.type === 'provisioned' && infoAny.value?.cell_id) {
        foundCellId = infoAny.value.cell_id;
        break;
      } else if (infoAny.provisioned?.cell_id) {
        foundCellId = infoAny.provisioned.cell_id;
        break;
      }
    }

    if (!foundCellId) {
      throw new Error(`No provisioned cell found for role '${this.config.roleId}'`);
    }

    this.cellId = foundCellId;
    console.log(`[SDK] Connected. Cell ID resolved.`);
  }

  /**
   * Disconnect from the conductor
   */
  async disconnect(): Promise<void> {
    if (this.appWs) {
      await this.appWs.client.close();
      this.appWs = null;
    }
    if (this.adminWs) {
      await this.adminWs.client.close();
      this.adminWs = null;
    }
    this.cellId = null;
    console.log('[SDK] Disconnected');
  }

  /**
   * Get current connection state
   */
  getState(): ConnectionState {
    return {
      isConnected: this.appWs !== null && this.cellId !== null,
      adminUrl: this.config.adminUrl,
      appUrl: this.config.appUrl ?? null,
      appId: this.config.appId ?? null,
      cellId: this.cellId,
    };
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.appWs !== null && this.cellId !== null;
  }

  /**
   * Get the AdminWebsocket (for admin operations)
   */
  getAdminWs(): AdminWebsocket {
    if (!this.adminWs) {
      throw new Error('Not connected. Call connect() first.');
    }
    return this.adminWs;
  }

  /**
   * Get the AppWebsocket (for zome calls)
   */
  getAppWs(): AppWebsocket {
    if (!this.appWs) {
      throw new Error('Not connected. Call connect() first.');
    }
    return this.appWs;
  }

  /**
   * Get the Cell ID (for zome calls)
   */
  getCellId(): CellId {
    if (!this.cellId) {
      throw new Error('Not connected. Call connect() first.');
    }
    return this.cellId;
  }

  /**
   * Call a zome function
   *
   * Low-level method - prefer using service classes for typed calls.
   */
  async callZome<T>(
    zomeName: string,
    fnName: string,
    payload: unknown
  ): Promise<T> {
    if (!this.appWs || !this.cellId) {
      throw new Error('Not connected. Call connect() first.');
    }

    const result = await this.appWs.callZome({
      cell_id: this.cellId,
      zome_name: zomeName,
      fn_name: fnName,
      payload,
    });

    return result as T;
  }
}

/**
 * Create a connection instance with default configuration
 */
export function createConnection(config: ConnectionConfig): HolochainConnection {
  return new HolochainConnection(config);
}
