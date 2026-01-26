/**
 * Holochain Client Service for Node.js CLI
 *
 * Manages WebSocket connections to a Holochain conductor.
 * Uses @holochain/client package for communication.
 *
 * This service is designed for CLI usage (Node.js) and connects
 * directly to the conductor's admin interface.
 */

import {
  AdminWebsocket,
  AppWebsocket,
  generateSigningKeyPair,
  randomCapSecret,
  setSigningCredentials,
  type AgentPubKey,
  type CellId,
  type AppInfo,
  type InstalledAppId,
} from '@holochain/client';

import {
  HolochainClientConfig,
  ZomeCallInput,
  ZomeCallResult,
} from '../models/holochain.model';

/**
 * Holochain Client Service
 *
 * Provides a simple interface for connecting to Holochain conductor
 * and making zome calls from Node.js CLI tools.
 *
 * Usage:
 * ```typescript
 * const client = new HolochainClientService({
 *   adminUrl: 'ws://localhost:4444',
 *   appId: 'elohim',
 * });
 *
 * await client.connect();
 * const result = await client.callZome({ zomeName: 'content_store', fnName: 'get_content_stats', payload: null });
 * await client.disconnect();
 * ```
 */
export class HolochainClientService {
  private adminWs: AdminWebsocket | null = null;
  private appWs: AppWebsocket | null = null;
  private cellId: CellId | null = null;
  private agentPubKey: AgentPubKey | null = null;
  private config: HolochainClientConfig;
  private isConnected = false;

  constructor(config: HolochainClientConfig) {
    this.config = config;
  }

  /**
   * Connect to the Holochain conductor
   *
   * Steps:
   * 1. Connect to admin WebSocket
   * 2. Generate signing credentials
   * 3. Check if app is installed, install if needed
   * 4. Grant zome call capability
   * 5. Attach app interface and connect
   */
  async connect(): Promise<void> {
    if (this.isConnected) {
      return;
    }

    try {
      console.log(`Connecting to Holochain conductor at ${this.config.adminUrl}...`);

      // Step 1: Connect to admin interface
      this.adminWs = await AdminWebsocket.connect({
        url: new URL(this.config.adminUrl),
      });

      // Step 2: Generate signing credentials
      const [keyPair, signingKey] = await generateSigningKeyPair();
      const capSecret = await randomCapSecret();

      // Step 3: Check if app is installed
      const apps = await this.adminWs.listApps({});
      let appInfo = apps.find((app) => app.installed_app_id === this.config.appId);

      if (!appInfo) {
        if (this.config.happPath) {
          console.log(`App ${this.config.appId} not found, installing from ${this.config.happPath}...`);

          // Generate agent key and install
          this.agentPubKey = await this.adminWs.generateAgentPubKey();
          appInfo = await this.adminWs.installApp({
            source: { type: 'path', value: this.config.happPath },
            installed_app_id: this.config.appId,
            agent_key: this.agentPubKey,
          });

          // Enable the app
          await this.adminWs.enableApp({ installed_app_id: this.config.appId });
          console.log(`App ${this.config.appId} installed and enabled`);
        } else {
          throw new Error(
            `App ${this.config.appId} not found and no happPath provided for installation`
          );
        }
      }

      // Step 4: Extract cell ID from app info
      this.cellId = this.extractCellId(appInfo);
      if (!this.cellId) {
        throw new Error('Could not extract cell ID from app info');
      }

      this.agentPubKey = this.cellId[1];

      // Step 5: Grant zome call capability
      await this.adminWs.grantZomeCallCapability({
        cell_id: this.cellId,
        cap_grant: {
          tag: 'cli-signing',
          functions: { type: 'all' },
          access: {
            type: 'assigned',
            value: { secret: capSecret, assignees: [signingKey] },
          },
        },
      });

      // Step 6: Register signing credentials
      setSigningCredentials(this.cellId, { capSecret, keyPair, signingKey });

      // Step 7: Attach app interface and connect
      const { port } = await this.adminWs.attachAppInterface({ allowed_origins: 'elohim-service' });
      const token = await this.adminWs.issueAppAuthenticationToken({
        installed_app_id: this.config.appId,
      });

      this.appWs = await AppWebsocket.connect({
        url: new URL(`ws://localhost:${port}`),
        token: token.token,
      });

      this.isConnected = true;
      console.log(`Connected to Holochain conductor (app: ${this.config.appId})`);
    } catch (error) {
      await this.disconnect();
      throw error;
    }
  }

  /**
   * Make a zome call
   *
   * @param input - Zome call parameters
   * @returns Promise with the call result
   */
  async callZome<T>(input: ZomeCallInput): Promise<T> {
    if (!this.appWs || !this.cellId) {
      throw new Error('Not connected to Holochain. Call connect() first.');
    }

    try {
      const result = await this.appWs.callZome({
        cell_id: this.cellId,
        zome_name: input.zomeName,
        fn_name: input.fnName,
        payload: input.payload,
      });

      return result as T;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`Zome call ${input.zomeName}/${input.fnName} failed: ${message}`);
    }
  }

  /**
   * Make a zome call with error handling wrapper
   *
   * @param input - Zome call parameters
   * @returns ZomeCallResult with success/error status
   */
  async callZomeSafe<T>(input: ZomeCallInput): Promise<ZomeCallResult<T>> {
    try {
      const data = await this.callZome<T>(input);
      return { success: true, data };
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return { success: false, error: message };
    }
  }

  /**
   * Disconnect from the conductor
   */
  async disconnect(): Promise<void> {
    if (this.appWs) {
      try {
        await this.appWs.client.close();
      } catch {
        // Ignore close errors
      }
      this.appWs = null;
    }

    if (this.adminWs) {
      try {
        await this.adminWs.client.close();
      } catch {
        // Ignore close errors
      }
      this.adminWs = null;
    }

    this.cellId = null;
    this.agentPubKey = null;
    this.isConnected = false;
  }

  /**
   * Check if connected to conductor
   */
  get connected(): boolean {
    return this.isConnected;
  }

  /**
   * Get the current agent public key
   */
  get agent(): AgentPubKey | null {
    return this.agentPubKey;
  }

  /**
   * Get the current cell ID
   */
  get cell(): CellId | null {
    return this.cellId;
  }

  /**
   * Test connection by making a simple zome call
   */
  async testConnection(): Promise<boolean> {
    try {
      await this.connect();
      // Try to get stats as a simple test
      await this.callZome({
        zomeName: 'content_store',
        fnName: 'get_content_stats',
        payload: null,
      });
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Extract cell ID from app info
   * Looks for the first provisioned cell in the app
   */
  private extractCellId(appInfo: AppInfo): CellId | null {
    const cellInfoEntries = Object.entries(appInfo.cell_info);
    for (const [, cells] of cellInfoEntries) {
      const cellArray = cells as Array<{ provisioned?: { cell_id: CellId } }>;
      for (const cell of cellArray) {
        if (cell.provisioned) {
          return cell.provisioned.cell_id;
        }
      }
    }
    return null;
  }
}

/**
 * Create a pre-configured client for the elohim app
 *
 * @param adminUrl - Admin WebSocket URL (default: wss://doorway-dev.elohim.host)
 * @param happPath - Optional path to .happ file for installation
 */
export function createElohimClient(
  adminUrl = 'wss://doorway-dev.elohim.host',
  happPath?: string
): HolochainClientService {
  return new HolochainClientService({
    adminUrl,
    appId: 'elohim',
    happPath,
  });
}
