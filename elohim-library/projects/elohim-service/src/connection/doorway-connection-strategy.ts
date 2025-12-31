/**
 * Doorway Connection Strategy
 *
 * Routes WebSocket connections through Doorway proxy.
 * Used in browser deployments where same-origin restrictions apply.
 *
 * Connection path:
 *   Browser → Doorway (wss://doorway-dev.elohim.host) → Conductor
 *
 * Blob storage:
 *   Uses Doorway's /api/blob/{hash} endpoint
 *
 * ContentResolver sources:
 *   indexeddb (Local) → projection (Doorway MongoDB) → conductor (Authoritative)
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

/**
 * Doorway Connection Strategy Implementation
 *
 * Connects to Holochain conductor through a Doorway proxy gateway.
 * Handles both Eclipse Che dev-proxy and deployed admin-proxy environments.
 */
export class DoorwayConnectionStrategy implements IConnectionStrategy {
  readonly name = 'doorway';
  readonly mode = 'doorway' as const;

  // Connection state
  private adminWs: AdminWebsocket | null = null;
  private appWs: AppWebsocket | null = null;
  private credentials: SigningCredentials | null = null;
  private connected = false;

  // ==========================================================================
  // Environment Detection
  // ==========================================================================

  isSupported(): boolean {
    // Doorway mode works everywhere (browser, Node, Tauri)
    // It's the fallback for any environment
    return true;
  }

  /**
   * Detect if running in Eclipse Che environment.
   * Checks for known Che URL patterns.
   */
  private isCheEnvironment(): boolean {
    if (typeof window === 'undefined') return false;
    return (
      window.location.hostname.includes('.devspaces.') ||
      window.location.hostname.includes('.code.ethosengine.com')
    );
  }

  /**
   * Get the dev proxy base URL in Che environment.
   * Converts angular-dev endpoint to hc-dev endpoint.
   */
  private getCheDevProxyUrl(): string | null {
    if (!this.isCheEnvironment()) return null;

    // In Che, each endpoint gets a unique URL like:
    // https://<workspace>-<endpoint>.code.ethosengine.com
    // We need to replace the endpoint suffix (angular-dev) with (hc-dev)
    const currentUrl = new URL(window.location.href);
    const hostname = currentUrl.hostname.replace(/-angular-dev\./, '-hc-dev.');

    console.log('[DoorwayStrategy] Che URL resolution:', {
      currentHostname: currentUrl.hostname,
      resolvedHostname: hostname,
      devProxyUrl: `wss://${hostname}`,
    });

    return `wss://${hostname}`;
  }

  // ==========================================================================
  // URL Resolution
  // ==========================================================================

  resolveAdminUrl(config: ConnectionConfig): string {
    const cheProxy = this.getCheDevProxyUrl();

    if (cheProxy && config.useLocalProxy) {
      // Che environment: use path-based routing through dev-proxy
      const url = `${cheProxy}/admin`;
      console.log('[DoorwayStrategy] Admin URL (Che dev-proxy):', url);
      return url;
    }

    // Deployed: use configured URL with optional API key
    const url = config.proxyApiKey
      ? `${config.adminUrl}?apiKey=${encodeURIComponent(config.proxyApiKey)}`
      : config.adminUrl;
    console.log('[DoorwayStrategy] Admin URL (direct):', url);
    return url;
  }

  resolveAppUrl(config: ConnectionConfig, port: number): string {
    const cheProxy = this.getCheDevProxyUrl();

    if (cheProxy && config.useLocalProxy) {
      // Che environment: use path-based routing with port through dev-proxy
      const url = `${cheProxy}/app/${port}`;
      console.log('[DoorwayStrategy] App URL (Che dev-proxy):', url);
      return url;
    }

    // Check if we have a configured admin URL (deployed environment)
    // Route app interface through admin-proxy using /app/:port path
    if (config.adminUrl && !config.adminUrl.includes('localhost')) {
      const baseUrl = config.adminUrl.replace(/\/$/, '');
      const apiKeyParam = config.proxyApiKey
        ? `?apiKey=${encodeURIComponent(config.proxyApiKey)}`
        : '';
      const url = `${baseUrl}/app/${port}${apiKeyParam}`;
      console.log('[DoorwayStrategy] App URL (admin-proxy):', url);
      return url;
    }

    // Fallback: direct localhost connection
    const url = `ws://localhost:${port}`;
    console.log('[DoorwayStrategy] App URL (localhost):', url);
    return url;
  }

  getBlobStorageUrl(config: ConnectionConfig, blobHash: string): string {
    // Convert WebSocket URL to HTTPS for blob API
    const baseUrl = config.adminUrl
      .replace('wss://', 'https://')
      .replace('ws://', 'http://')
      .replace(/\/$/, '');

    // Add API key if configured
    const apiKeyParam = config.proxyApiKey
      ? `?apiKey=${encodeURIComponent(config.proxyApiKey)}`
      : '';

    return `${baseUrl}/api/blob/${encodeURIComponent(blobHash)}${apiKeyParam}`;
  }

  // ==========================================================================
  // Content Source Configuration
  // ==========================================================================

  getContentSources(config: ConnectionConfig): ContentSourceConfig[] {
    const doorwayUrl = config.adminUrl
      .replace('wss://', 'https://')
      .replace('ws://', 'http://')
      .replace(/\/$/, '');

    return [
      {
        id: 'indexeddb',
        tier: SourceTier.Local,
        priority: 100,
        contentTypes: ['path', 'content', 'graph', 'assessment', 'profile'],
        available: true,
      },
      {
        id: 'projection',
        tier: SourceTier.Projection,
        priority: 80,
        contentTypes: ['path', 'content', 'graph', 'assessment', 'profile', 'blob', 'app'],
        baseUrl: doorwayUrl,
        available: true,
      },
      {
        id: 'conductor',
        tier: SourceTier.Authoritative,
        priority: 50,
        contentTypes: ['path', 'content', 'graph', 'assessment', 'profile', 'identity'],
        available: false, // Enabled after connection
      },
    ];
  }

  // ==========================================================================
  // Connection Lifecycle
  // ==========================================================================

  async connect(config: ConnectionConfig): Promise<ConnectionResult> {
    try {
      console.log('[DoorwayStrategy] Starting connection...');

      // Step 1: Connect to Admin WebSocket (through proxy or direct)
      const adminUrl = this.resolveAdminUrl(config);
      console.log('[DoorwayStrategy] Connecting to admin:', adminUrl);

      // Browser uses native WebSocket - wsClientOptions.origin is for Node.js only
      this.adminWs = await AdminWebsocket.connect({ url: new URL(adminUrl) });

      // Step 2: Generate signing credentials
      console.log('[DoorwayStrategy] Generating signing credentials...');
      const [keyPair, signingKey] = await generateSigningKeyPair();
      const capSecret = await randomCapSecret();

      this.credentials = { capSecret, keyPair, signingKey };

      // Step 3: Generate agent key
      const agentPubKey = await this.adminWs.generateAgentPubKey();
      console.log('[DoorwayStrategy] Agent key generated');

      // Step 4: Check if app is installed, install if needed
      let appInfo = await this.getInstalledApp(this.adminWs, config.appId);

      if (!appInfo) {
        console.log(`[DoorwayStrategy] App ${config.appId} not installed. Installing...`);

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

      console.log('[DoorwayStrategy] Found', cellIds.size, 'cells:', Array.from(cellIds.keys()));

      // Step 6: Grant zome call capability for ALL cells
      for (const [roleName, cellId] of cellIds) {
        await this.adminWs.grantZomeCallCapability({
          cell_id: cellId,
          cap_grant: {
            tag: `browser-signing-${roleName}`,
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
        console.log(`[DoorwayStrategy] Granted cap for role '${roleName}'`);
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
        console.log(`[DoorwayStrategy] Using existing app interface on port ${appPort}`);
      } else {
        // Create new interface with wildcard origins (proxy handles auth)
        const { port } = await this.adminWs.attachAppInterface({
          allowed_origins: '*',
        });
        appPort = port;
        console.log(`[DoorwayStrategy] Created new app interface on port ${appPort}`);
      }

      // Step 9: Authorize signing credentials for ALL cells
      for (const [roleName, cellId] of cellIds) {
        await this.adminWs.authorizeSigningCredentials(cellId);
        console.log(`[DoorwayStrategy] Authorized credentials for role '${roleName}'`);
      }

      // Step 10: Get app authentication token
      const issuedToken = await this.adminWs.issueAppAuthenticationToken({
        installed_app_id: config.appId,
        single_use: false,
        expiry_seconds: 3600, // 1 hour
      });

      // Step 11: Connect to App WebSocket
      const appUrl = this.resolveAppUrl(config, appPort);
      console.log('[DoorwayStrategy] Connecting to app interface:', appUrl);

      this.appWs = await AppWebsocket.connect({
        url: new URL(appUrl),
        token: issuedToken.token,
      });

      this.connected = true;

      console.log('[DoorwayStrategy] Connection successful', {
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
      console.error('[DoorwayStrategy] Connection failed:', errorMessage);

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
      console.warn('[DoorwayStrategy] Error during disconnect:', err);
    }

    this.credentials = null;
    this.connected = false;
    console.log('[DoorwayStrategy] Disconnected');
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
      return apps.find((app) => app.installed_app_id === appId) || null;
    } catch {
      return null;
    }
  }

  /**
   * Extract all cell IDs from app info, keyed by role name.
   *
   * For multi-DNA hApps like Elohim:
   * - 'lamad' → Content & Learning DNA
   * - 'infrastructure' → Doorway Federation DNA
   * - 'imagodei' → Identity & Relationships DNA
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
