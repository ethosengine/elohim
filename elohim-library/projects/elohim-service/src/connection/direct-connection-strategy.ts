/**
 * Direct Connection Strategy
 *
 * Connects directly to local Holochain conductor.
 * Used in native (Tauri) deployments for lower latency.
 *
 * Connection path:
 *   Device → Local Conductor (ws://localhost:4444)
 *
 * Blob storage:
 *   Uses elohim-storage sidecar directly (http://localhost:8090)
 *   NOT through Doorway - Doorway is only for web 2.0 bridging
 *
 * ContentResolver sources:
 *   indexeddb (Local) → conductor (Authoritative) → elohim-storage (blobs)
 *   NOTE: Skips Projection tier (no Doorway in native P2P network)
 *
 * @packageDocumentation
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
} from '@holochain/client';

import { SourceTier } from '../cache/content-resolver';

import type {
  IConnectionStrategy,
  ConnectionConfig,
  ConnectionResult,
  ContentSourceConfig,
  SigningCredentials,
} from './connection-strategy';

/** Default local conductor admin port */
const DEFAULT_ADMIN_PORT = 4444;

/** Default local conductor app port */
const DEFAULT_APP_PORT = 4445;

/** Default elohim-storage sidecar port */
const DEFAULT_STORAGE_PORT = 8090;

/**
 * Direct Connection Strategy Implementation
 *
 * Connects directly to local Holochain conductor without any proxy.
 * Optimized for native deployments (Tauri, Electron, Node.js CLI).
 */
export class DirectConnectionStrategy implements IConnectionStrategy {
  readonly name = 'direct';
  readonly mode = 'direct' as const;

  // Connection state
  private adminWs: AdminWebsocket | null = null;
  private appWs: AppWebsocket | null = null;
  private credentials: SigningCredentials | null = null;
  private connected = false;

  // ==========================================================================
  // Environment Detection
  // ==========================================================================

  isSupported(): boolean {
    // Direct mode requires Tauri or Node.js
    return this.isTauriEnvironment() || this.isNodeEnvironment();
  }

  /**
   * Detect if running in Tauri native app.
   * Checks for __TAURI__ global injected by Tauri runtime.
   */
  private isTauriEnvironment(): boolean {
    return typeof window !== 'undefined' && '__TAURI__' in window;
  }

  /**
   * Detect if running in Node.js (CLI tools, tests).
   */
  private isNodeEnvironment(): boolean {
    return typeof process !== 'undefined' && process.versions?.node !== undefined;
  }

  // ==========================================================================
  // URL Resolution
  // ==========================================================================

  resolveAdminUrl(config: ConnectionConfig): string {
    // Direct mode: use localhost conductor
    const url = config.adminUrl || `ws://localhost:${DEFAULT_ADMIN_PORT}`;
    console.log('[DirectStrategy] Admin URL:', url);
    return url;
  }

  resolveAppUrl(_config: ConnectionConfig, port: number): string {
    // Direct mode: connect to localhost app port
    const url = `ws://localhost:${port}`;
    console.log('[DirectStrategy] App URL:', url);
    return url;
  }

  getBlobStorageUrl(config: ConnectionConfig, blobHash: string): string {
    // Direct to elohim-storage sidecar - NOT through Doorway
    // elohim-storage provides caching and R/W protection for the conductor
    const storageUrl = config.storageUrl || `http://localhost:${DEFAULT_STORAGE_PORT}`;
    return `${storageUrl}/store/${encodeURIComponent(blobHash)}`;
  }

  getStorageBaseUrl(config: ConnectionConfig): string {
    // Direct to elohim-storage sidecar
    return config.storageUrl || `http://localhost:${DEFAULT_STORAGE_PORT}`;
  }

  // ==========================================================================
  // Content Source Configuration
  // ==========================================================================

  getContentSources(config: ConnectionConfig): ContentSourceConfig[] {
    const storageUrl = config.storageUrl || `http://localhost:${DEFAULT_STORAGE_PORT}`;

    return [
      {
        id: 'indexeddb',
        tier: SourceTier.Local,
        priority: 100,
        contentTypes: ['path', 'content', 'graph', 'assessment', 'profile'],
        available: true,
      },
      // NOTE: Skip Projection tier - go directly to Authoritative
      // No Doorway proxy in native P2P network
      {
        id: 'conductor',
        tier: SourceTier.Authoritative,
        priority: 90, // Higher priority than in Doorway mode
        contentTypes: ['path', 'content', 'graph', 'assessment', 'profile', 'identity'],
        available: false, // Enabled after connection
      },
      {
        id: 'elohim-storage',
        tier: SourceTier.Authoritative,
        priority: 85,
        contentTypes: ['blob'],
        baseUrl: storageUrl,
        available: true,
      },
    ];
  }

  // ==========================================================================
  // Connection Lifecycle
  // ==========================================================================

  async connect(config: ConnectionConfig): Promise<ConnectionResult> {
    try {
      console.log('[DirectStrategy] Starting connection...');

      // Step 1: Connect to Admin WebSocket (direct to localhost)
      const adminUrl = this.resolveAdminUrl(config);
      console.log('[DirectStrategy] Connecting to admin:', adminUrl);

      // In Node.js/Tauri, we can pass wsClientOptions
      // In browser, this would be ignored anyway
      this.adminWs = await AdminWebsocket.connect({
        url: new URL(adminUrl),
        wsClientOptions: {
          origin: config.origin || 'elohim-native',
        },
      });

      // Step 2: Generate signing credentials
      console.log('[DirectStrategy] Generating signing credentials...');
      const [keyPair, signingKey] = await generateSigningKeyPair();
      const capSecret = await randomCapSecret();

      this.credentials = { capSecret, keyPair, signingKey };

      // Step 3: Generate agent key
      const agentPubKey = await this.adminWs.generateAgentPubKey();
      console.log('[DirectStrategy] Agent key generated');

      // Step 4: Check if app is installed, install if needed
      let appInfo = await this.getInstalledApp(this.adminWs, config.appId);

      if (!appInfo) {
        console.log(`[DirectStrategy] App ${config.appId} not installed. Installing...`);

        if (config.happPath) {
          appInfo = await this.adminWs.installApp({
            source: { type: 'path', value: config.happPath },
            installed_app_id: config.appId,
            agent_key: agentPubKey,
          });

          await this.adminWs.enableApp({
            installed_app_id: config.appId,
          });
        } else {
          throw new Error(
            `App ${config.appId} not installed and no happPath provided`
          );
        }
      }

      // Step 5: Extract all cell IDs (multi-DNA support)
      const cellIds = this.extractAllCellIds(appInfo);
      if (cellIds.size === 0) {
        throw new Error('Could not extract any cell IDs from app info');
      }

      console.log('[DirectStrategy] Found', cellIds.size, 'cells:', Array.from(cellIds.keys()));

      // Step 6: Grant zome call capability for ALL cells
      for (const [roleName, cellId] of cellIds) {
        await this.adminWs.grantZomeCallCapability({
          cell_id: cellId,
          cap_grant: {
            tag: `native-signing-${roleName}`,
            functions: { type: 'all' },
            access: {
              type: 'assigned',
              value: {
                secret: capSecret,
                assignees: [signingKey],
              },
            },
          },
        });
        console.log(`[DirectStrategy] Granted cap for role '${roleName}'`);
      }

      // Step 7: Register signing credentials for ALL cells
      for (const [, cellId] of cellIds) {
        setSigningCredentials(cellId, {
          capSecret,
          keyPair,
          signingKey,
        });
      }

      // Step 8: Find or create app interface
      const existingInterfaces = await this.adminWs.listAppInterfaces();
      let appPort: number;

      if (existingInterfaces.length > 0) {
        appPort = existingInterfaces[0].port;
        console.log(`[DirectStrategy] Using existing app interface on port ${appPort}`);
      } else {
        // Create new interface - in direct mode we can use localhost origin
        const { port } = await this.adminWs.attachAppInterface({
          allowed_origins: 'localhost',
        });
        appPort = port;
        console.log(`[DirectStrategy] Created new app interface on port ${appPort}`);
      }

      // Step 9: Authorize signing credentials for ALL cells
      for (const [roleName, cellId] of cellIds) {
        await this.adminWs.authorizeSigningCredentials(cellId);
        console.log(`[DirectStrategy] Authorized credentials for role '${roleName}'`);
      }

      // Step 10: Get app authentication token
      const issuedToken = await this.adminWs.issueAppAuthenticationToken({
        installed_app_id: config.appId,
        single_use: false,
        expiry_seconds: 86400, // 24 hours for native (longer session)
      });

      // Step 11: Connect to App WebSocket (direct to localhost)
      const appUrl = this.resolveAppUrl(config, appPort);
      console.log('[DirectStrategy] Connecting to app interface:', appUrl);

      this.appWs = await AppWebsocket.connect({
        url: new URL(appUrl),
        token: issuedToken.token,
        wsClientOptions: {
          origin: config.origin || 'elohim-native',
        },
      });

      this.connected = true;

      console.log('[DirectStrategy] Connection successful', {
        appId: config.appId,
        cellCount: cellIds.size,
      });

      return {
        success: true,
        adminWs: this.adminWs,
        appWs: this.appWs,
        cellIds,
        agentPubKey,
        appInfo,
        appPort,
      };
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown connection error';
      console.error('[DirectStrategy] Connection failed:', errorMessage);

      return {
        success: false,
        error: errorMessage,
      };
    }
  }

  async disconnect(): Promise<void> {
    try {
      if (this.appWs) {
        await this.appWs.client.close();
        this.appWs = null;
      }
      if (this.adminWs) {
        await this.adminWs.client.close();
        this.adminWs = null;
      }
    } catch (err) {
      console.warn('[DirectStrategy] Error during disconnect:', err);
    }

    this.credentials = null;
    this.connected = false;
    console.log('[DirectStrategy] Disconnected');
  }

  // ==========================================================================
  // Connection State
  // ==========================================================================

  isConnected(): boolean {
    return this.connected && this.appWs !== null;
  }

  getSigningCredentials(): SigningCredentials | null {
    return this.credentials;
  }

  // ==========================================================================
  // Helper Methods
  // ==========================================================================

  /**
   * Get installed app info from conductor.
   */
  private async getInstalledApp(
    adminWs: AdminWebsocket,
    appId: string
  ): Promise<AppInfo | null> {
    try {
      const apps = await adminWs.listApps({});
      return apps.find((app: AppInfo) => app.installed_app_id === appId) || null;
    } catch {
      return null;
    }
  }

  /**
   * Extract all cell IDs from app info, keyed by role name.
   */
  private extractAllCellIds(appInfo: AppInfo): Map<string, CellId> {
    const cellIds = new Map<string, CellId>();
    const cellInfoEntries = Object.entries(appInfo.cell_info);

    for (const [roleName, cells] of cellInfoEntries) {
      const cellArray = cells as Array<{ type: string; value: { cell_id: CellId } }>;
      for (const cell of cellArray) {
        if (cell.type === 'provisioned' && cell.value?.cell_id) {
          cellIds.set(roleName, cell.value.cell_id);
          break; // Only take first provisioned cell per role
        }
      }
    }

    return cellIds;
  }
}
