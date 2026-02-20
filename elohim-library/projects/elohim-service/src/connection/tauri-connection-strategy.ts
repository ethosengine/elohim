/**
 * Tauri Connection Strategy
 *
 * Connection strategy for Tauri native apps with local session management.
 * Extends DirectConnectionStrategy with OAuth handoff and session persistence.
 *
 * Flow:
 * 1. Check for existing local session in elohim-storage
 * 2. If session exists, restore connection state
 * 3. If no session, wait for OAuth handoff from doorway
 * 4. After handoff, store session and link agent to Human identity
 *
 * Session storage:
 *   Uses elohim-storage SQLite via HTTP API (http://localhost:8090/session)
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
  Logger,
} from './connection-strategy';
import { ConsoleLogger } from './console-logger';

/** Default elohim-storage sidecar port */
const DEFAULT_STORAGE_PORT = 8090;

/**
 * Local session stored in elohim-storage SQLite
 */
export interface LocalSession {
  id: string;
  humanId: string;
  agentPubKey: string;
  doorwayUrl: string;
  doorwayId?: string;
  identifier: string;
  displayName?: string;
  profileImageHash?: string;
  isActive: number;
  createdAt: string;
  updatedAt: string;
  lastSyncedAt?: string;
  bootstrapUrl?: string;
}

/**
 * Input for creating a local session
 */
export interface CreateSessionInput {
  id?: string;
  humanId: string;
  agentPubKey: string;
  doorwayUrl: string;
  doorwayId?: string;
  identifier: string;
  displayName?: string;
  profileImageHash?: string;
  bootstrapUrl?: string;
}

/**
 * Native handoff response from doorway
 */
export interface NativeHandoffResponse {
  humanId: string;
  identifier: string;
  doorwayId: string;
  doorwayUrl: string;
  displayName?: string;
  profileImageHash?: string;
  bootstrapUrl?: string;
}

/**
 * OAuth callback payload from Tauri deep link handler
 */
export interface OAuthCallbackPayload {
  code: string;
  state?: string;
  url: string;
}

/**
 * Tauri Connection Strategy Implementation
 *
 * Manages local sessions and connects to Holochain with session persistence.
 * Used for Tauri native apps that need to survive app restarts.
 */
export class TauriConnectionStrategy implements IConnectionStrategy {
  readonly name = 'tauri';
  readonly mode = 'direct' as const;

  // Connection state
  private adminWs: AdminWebsocket | null = null;
  private appWs: AppWebsocket | null = null;
  private credentials: SigningCredentials | null = null;
  private connected = false;
  private currentSession: LocalSession | null = null;
  private logger: Logger = new ConsoleLogger('TauriStrategy');

  /** Resolve logger from config or use default */
  private resolveLogger(config: ConnectionConfig): Logger {
    if (config.logger) {
      this.logger = config.logger;
    } else if (config.logLevel) {
      this.logger = new ConsoleLogger('TauriStrategy', config.logLevel);
    }
    return this.logger;
  }

  // ==========================================================================
  // Environment Detection
  // ==========================================================================

  isSupported(): boolean {
    return this.isTauriEnvironment();
  }

  /**
   * Detect if running in Tauri native app.
   */
  private isTauriEnvironment(): boolean {
    return typeof window !== 'undefined' && '__TAURI__' in window;
  }

  // ==========================================================================
  // Session Management (via elohim-storage HTTP API)
  // ==========================================================================

  /**
   * Get the storage API base URL.
   */
  private getStorageApiUrl(config: ConnectionConfig): string {
    return config.storageUrl || `http://localhost:${DEFAULT_STORAGE_PORT}`;
  }

  /**
   * Check if there's an active local session.
   */
  async hasLocalSession(config: ConnectionConfig): Promise<boolean> {
    try {
      const session = await this.getActiveSession(config);
      return session !== null;
    } catch {
      return false;
    }
  }

  /**
   * Get the active local session from elohim-storage.
   */
  async getActiveSession(config: ConnectionConfig): Promise<LocalSession | null> {
    try {
      const storageUrl = this.getStorageApiUrl(config);
      const response = await fetch(`${storageUrl}/session`);

      if (response.status === 404) {
        return null;
      }

      if (!response.ok) {
        throw new Error(`Session API error: ${response.status}`);
      }

      const session = await response.json();
      // Convert snake_case to camelCase
      return {
        id: session.id,
        humanId: session.human_id,
        agentPubKey: session.agent_pub_key,
        doorwayUrl: session.doorway_url,
        doorwayId: session.doorway_id,
        identifier: session.identifier,
        displayName: session.display_name,
        profileImageHash: session.profile_image_hash,
        isActive: session.is_active,
        createdAt: session.created_at,
        updatedAt: session.updated_at,
        lastSyncedAt: session.last_synced_at,
        bootstrapUrl: session.bootstrap_url,
      };
    } catch (err) {
      this.logger.warn('Failed to get session', { error: err instanceof Error ? err.message : String(err) });
      return null;
    }
  }

  /**
   * Create a new local session in elohim-storage.
   */
  async createSession(
    config: ConnectionConfig,
    input: CreateSessionInput
  ): Promise<LocalSession> {
    const storageUrl = this.getStorageApiUrl(config);

    // Convert camelCase to snake_case for API
    const body = {
      id: input.id,
      human_id: input.humanId,
      agent_pub_key: input.agentPubKey,
      doorway_url: input.doorwayUrl,
      doorway_id: input.doorwayId,
      identifier: input.identifier,
      display_name: input.displayName,
      profile_image_hash: input.profileImageHash,
      bootstrap_url: input.bootstrapUrl,
    };

    const response = await fetch(`${storageUrl}/session`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to create session: ${error}`);
    }

    const session = await response.json();
    return {
      id: session.id,
      humanId: session.human_id,
      agentPubKey: session.agent_pub_key,
      doorwayUrl: session.doorway_url,
      doorwayId: session.doorway_id,
      identifier: session.identifier,
      displayName: session.display_name,
      profileImageHash: session.profile_image_hash,
      isActive: session.is_active,
      createdAt: session.created_at,
      updatedAt: session.updated_at,
      lastSyncedAt: session.last_synced_at,
      bootstrapUrl: session.bootstrap_url,
    };
  }

  /**
   * Delete the active session (logout).
   */
  async deleteSession(config: ConnectionConfig): Promise<boolean> {
    const storageUrl = this.getStorageApiUrl(config);

    const response = await fetch(`${storageUrl}/session`, {
      method: 'DELETE',
    });

    if (!response.ok) {
      this.logger.warn('Failed to delete session');
      return false;
    }

    const result = await response.json();
    return result.deleted === true;
  }

  // ==========================================================================
  // OAuth Handoff
  // ==========================================================================

  /**
   * Exchange OAuth code for tokens and get native handoff data.
   *
   * Called after receiving elohim://auth/callback?code=... deep link.
   */
  async performNativeHandoff(
    doorwayUrl: string,
    authCode: string,
    accessToken: string
  ): Promise<NativeHandoffResponse> {
    const response = await fetch(`${doorwayUrl}/auth/native-handoff`, {
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Native handoff failed: ${error}`);
    }

    return response.json();
  }

  /**
   * Exchange authorization code for access token.
   */
  async exchangeCodeForToken(
    doorwayUrl: string,
    code: string,
    redirectUri: string
  ): Promise<string> {
    const response = await fetch(`${doorwayUrl}/auth/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        code,
        redirect_uri: redirectUri,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Token exchange failed: ${error}`);
    }

    const data = await response.json();
    return data.access_token;
  }

  // ==========================================================================
  // URL Resolution
  // ==========================================================================

  resolveAdminUrl(_config: ConnectionConfig): string {
    // In Tauri mode, always connect to the local embedded conductor.
    // config.adminUrl may contain a doorway URL from environment config,
    // which is not appropriate for direct conductor access.
    const url = 'ws://localhost:4444';
    this.logger.debug('Admin URL', { url });
    return url;
  }

  resolveAppUrl(_config: ConnectionConfig, port: number): string {
    const url = `ws://localhost:${port}`;
    this.logger.debug('App URL', { url });
    return url;
  }

  getBlobStorageUrl(config: ConnectionConfig, blobHash: string): string {
    const storageUrl = config.storageUrl || `http://localhost:${DEFAULT_STORAGE_PORT}`;
    return `${storageUrl}/store/${encodeURIComponent(blobHash)}`;
  }

  getStorageBaseUrl(config: ConnectionConfig): string {
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
      {
        id: 'conductor',
        tier: SourceTier.Authoritative,
        priority: 90,
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

      // Step 1: Connect to Admin WebSocket
      const adminUrl = this.resolveAdminUrl(config);
      this.logger.debug('Connecting to admin', { adminUrl });

      this.adminWs = await AdminWebsocket.connect({
        url: new URL(adminUrl),
        wsClientOptions: {
          origin: config.origin || 'elohim-tauri',
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

      // Step 4: Check if app is installed
      let appInfo = await this.getInstalledApp(this.adminWs, config.appId);

      if (!appInfo) {
        this.logger.info(`App ${config.appId} not installed`);

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

      // Step 5: Extract all cell IDs
      const cellIds = this.extractAllCellIds(appInfo);
      if (cellIds.size === 0) {
        throw new Error('Could not extract any cell IDs from app info');
      }

      this.logger.debug('Found cells', { count: cellIds.size });

      // Step 6: Grant zome call capability for ALL cells
      for (const [roleName, cellId] of cellIds) {
        await this.adminWs.grantZomeCallCapability({
          cell_id: cellId,
          cap_grant: {
            tag: `tauri-signing-${roleName}`,
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
      }

      // Step 7: Register signing credentials
      for (const [, cellId] of cellIds) {
        setSigningCredentials(cellId, { capSecret, keyPair, signingKey });
      }

      // Step 8: Find or create app interface
      const existingInterfaces = await this.adminWs.listAppInterfaces();
      let appPort: number;

      if (existingInterfaces.length > 0) {
        appPort = existingInterfaces[0].port;
      } else {
        const { port } = await this.adminWs.attachAppInterface({
          allowed_origins: 'localhost',
        });
        appPort = port;
      }

      // Step 9: Authorize signing credentials
      for (const [, cellId] of cellIds) {
        await this.adminWs.authorizeSigningCredentials(cellId);
      }

      // Step 10: Get app authentication token
      const issuedToken = await this.adminWs.issueAppAuthenticationToken({
        installed_app_id: config.appId,
        single_use: false,
        expiry_seconds: 86400,
      });

      // Step 11: Connect to App WebSocket
      const appUrl = this.resolveAppUrl(config, appPort);
      this.logger.debug('Connecting to app interface', { appUrl });

      this.appWs = await AppWebsocket.connect({
        url: new URL(appUrl),
        token: issuedToken.token,
        wsClientOptions: {
          origin: config.origin || 'elohim-tauri',
        },
      });

      this.connected = true;

      this.logger.info('Connection successful');

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
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';
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
      this.logger.warn('Error during disconnect', { error: err instanceof Error ? err.message : String(err) });
    }

    this.credentials = null;
    this.connected = false;
    this.currentSession = null;
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

  getCurrentSession(): LocalSession | null {
    return this.currentSession;
  }

  // ==========================================================================
  // Helper Methods
  // ==========================================================================

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

  private extractAllCellIds(appInfo: AppInfo): Map<string, CellId> {
    const cellIds = new Map<string, CellId>();
    const cellInfoEntries = Object.entries(appInfo.cell_info);

    for (const [roleName, cells] of cellInfoEntries) {
      const cellArray = cells as Array<{ type: string; value: { cell_id: CellId } }>;
      for (const cell of cellArray) {
        if (cell.type === 'provisioned' && cell.value?.cell_id) {
          cellIds.set(roleName, cell.value.cell_id);
          break;
        }
      }
    }

    return cellIds;
  }
}
