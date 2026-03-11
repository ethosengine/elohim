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
  type CellId,
  type AppInfo,
} from '@holochain/client';

import { SourceTier } from '../cache/content-resolver';

import { ConsoleLogger } from './console-logger';

import type {
  IConnectionStrategy,
  ConnectionConfig,
  ConnectionResult,
  ContentSourceConfig,
  SigningCredentials,
  Logger,
} from './connection-strategy';

/** Default local conductor admin port */
const DEFAULT_ADMIN_PORT = 4444;

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
  private logger: Logger = new ConsoleLogger('DirectStrategy');

  /** Resolve logger from config or use default */
  private resolveLogger(config: ConnectionConfig): Logger {
    if (config.logger) {
      this.logger = config.logger;
    } else if (config.logLevel) {
      this.logger = new ConsoleLogger('DirectStrategy', config.logLevel);
    }
    return this.logger;
  }

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
    return globalThis.window !== undefined && '__TAURI__' in globalThis;
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
    this.logger.debug('Admin URL', { url });
    return url;
  }

  resolveAppUrl(_config: ConnectionConfig, port: number): string {
    // Direct mode: connect to localhost app port
    const url = `ws://localhost:${port}`;
    this.logger.debug('App URL', { url });
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
    this.resolveLogger(config);
    try {
      this.logger.info('Starting connection...');

      // Step 1: Connect to Admin WebSocket (direct to localhost)
      const adminUrl = this.resolveAdminUrl(config);
      this.logger.debug('Connecting to admin', { adminUrl });

      // In Node.js/Tauri, we can pass wsClientOptions
      // In browser, this would be ignored anyway
      this.adminWs = await AdminWebsocket.connect({
        url: new URL(adminUrl),
        wsClientOptions: {
          origin: config.origin || 'elohim-native',
        },
      });

      // Step 2: Generate signing credentials
      this.logger.debug('Generating signing credentials...');
      const [keyPair, signingKey] = await generateSigningKeyPair();
      const capSecret = await randomCapSecret();

      this.credentials = { capSecret, keyPair, signingKey };

      // Step 3: Generate agent key
      const agentPubKey = await this.adminWs.generateAgentPubKey();
      this.logger.debug('Agent key generated');

      // Step 4: Check if app is installed, install if needed
      let appInfo = await this.getInstalledApp(this.adminWs, config.appId);

      if (!appInfo) {
        this.logger.info(`App ${config.appId} not installed. Installing...`);

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
          throw new Error(`App ${config.appId} not installed and no happPath provided`);
        }
      }

      // Step 5: Extract all cell IDs (multi-DNA support)
      const cellIds = this.extractAllCellIds(appInfo);
      if (cellIds.size === 0) {
        throw new Error('Could not extract any cell IDs from app info');
      }

      this.logger.debug('Found cells', { count: cellIds.size, roles: Array.from(cellIds.keys()) });

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
        this.logger.debug(`Granted cap for role '${roleName}'`);
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
        this.logger.debug(`Using existing app interface on port ${appPort}`);
      } else {
        // Create new interface - in direct mode we can use localhost origin
        const { port } = await this.adminWs.attachAppInterface({
          allowed_origins: 'localhost',
        });
        appPort = port;
        this.logger.debug(`Created new app interface on port ${appPort}`);
      }

      // Step 9: Authorize signing credentials for ALL cells
      for (const [roleName, cellId] of cellIds) {
        await this.adminWs.authorizeSigningCredentials(cellId);
        this.logger.debug(`Authorized credentials for role '${roleName}'`);
      }

      // Step 10: Get app authentication token
      const issuedToken = await this.adminWs.issueAppAuthenticationToken({
        installed_app_id: config.appId,
        single_use: false,
        expiry_seconds: 86400, // 24 hours for native (longer session)
      });

      // Step 11: Connect to App WebSocket (direct to localhost)
      const appUrl = this.resolveAppUrl(config, appPort);
      this.logger.debug('Connecting to app interface', { appUrl });

      this.appWs = await AppWebsocket.connect({
        url: new URL(appUrl),
        token: issuedToken.token,
        wsClientOptions: {
          origin: config.origin || 'elohim-native',
        },
      });

      this.connected = true;

      this.logger.info('Connection successful', {
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
      this.logger.error('Connection failed', errorMessage);

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
      this.logger.warn('Error during disconnect', {
        error: err instanceof Error ? err.message : String(err),
      });
    }

    this.credentials = null;
    this.connected = false;
    this.logger.info('Disconnected');
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
  private async getInstalledApp(adminWs: AdminWebsocket, appId: string): Promise<AppInfo | null> {
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
      const cellArray = cells as { type: string; value: { cell_id: CellId } }[];
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
