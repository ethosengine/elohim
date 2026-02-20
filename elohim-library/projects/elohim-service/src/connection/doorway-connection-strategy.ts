/**
 * Doorway Connection Strategy
 *
 * Routes WebSocket connections through Doorway proxy.
 * Used in browser deployments where same-origin restrictions apply.
 *
 * Connection path:
 *   Browser → Doorway (wss://doorway-alpha.elohim.host) → Conductor
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

import { ConsoleLogger } from './console-logger';

import type {
  IConnectionStrategy,
  ConnectionConfig,
  ConnectionResult,
  ContentSourceConfig,
  SigningCredentials,
  Logger,
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
  private logger: Logger = new ConsoleLogger('DoorwayStrategy');

  /** Resolve logger from config or use default */
  private resolveLogger(config: ConnectionConfig): Logger {
    if (config.logger) {
      this.logger = config.logger;
    } else if (config.logLevel) {
      this.logger = new ConsoleLogger('DoorwayStrategy', config.logLevel);
    }
    return this.logger;
  }

  // ==========================================================================
  // Retry Helper for Source Chain Operations
  // ==========================================================================

  /**
   * Retry an operation that may fail due to source chain conflicts.
   *
   * Holochain's source chain is single-threaded per agent. When multiple
   * operations try to commit simultaneously (e.g., cap grants for multiple
   * cells), they can race. This helper retries with backoff on known
   * conflict errors.
   */
  private async withSourceChainRetry<T>(
    operation: () => Promise<T>,
    description: string,
    maxRetries = 3
  ): Promise<T> {
    let lastError: Error | null = null;

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        return await operation();
      } catch (err) {
        lastError = err instanceof Error ? err : new Error(String(err));

        // Check if this is a source chain conflict error
        const isSourceChainConflict =
          lastError.message.includes('source chain head has moved') ||
          lastError.message.includes('HeadMoved');

        if (isSourceChainConflict && attempt < maxRetries) {
          // Exponential backoff: 100ms, 200ms, 400ms
          const delay = 100 * Math.pow(2, attempt - 1);
          this.logger.warn(
            `Source chain conflict during ${description}, retrying in ${delay}ms (attempt ${attempt}/${maxRetries})`
          );
          await new Promise(resolve => setTimeout(resolve, delay));
          continue;
        }

        // Not a retryable error or max retries reached
        throw lastError;
      }
    }

    // Should not reach here, but satisfy TypeScript
    throw lastError ?? new Error('Retry failed');
  }

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
    if (globalThis.window === undefined || !globalThis.location) return false;
    return (
      globalThis.location.hostname.includes('.devspaces.') ||
      globalThis.location.hostname.includes('.code.ethosengine.com')
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
    const currentUrl = new URL(globalThis.location!.href);
    const hostname = currentUrl.hostname.replace(/-angular-dev\./, '-hc-dev.');

    this.logger.debug('Che URL resolution', {
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
      const url = `${cheProxy}/hc/admin`;
      this.logger.debug('Admin URL (Che dev-proxy)', { url });
      return url;
    }

    // Deployed: use configured URL with /hc/admin path, optional API key, and JWT for affinity
    const baseUrl = config.adminUrl.replace(/\/$/, '');
    const params = this.buildQueryParams(config);
    const url = params ? `${baseUrl}/hc/admin?${params}` : `${baseUrl}/hc/admin`;
    this.logger.debug('Admin URL (doorway)', { url });
    return url;
  }

  resolveAppUrl(config: ConnectionConfig, port: number): string {
    const cheProxy = this.getCheDevProxyUrl();

    if (cheProxy && config.useLocalProxy) {
      // Che environment: use path-based routing with port through dev-proxy
      const url = `${cheProxy}/hc/app/${port}`;
      this.logger.debug('App URL (Che dev-proxy)', { url });
      return url;
    }

    // Check if we have a configured admin URL (deployed environment)
    // Route app interface through doorway using /hc/app/:port path
    if (config.adminUrl && !config.adminUrl.includes('localhost')) {
      const baseUrl = config.adminUrl.replace(/\/$/, '');
      const params = this.buildQueryParams(config);
      const url = params ? `${baseUrl}/hc/app/${port}?${params}` : `${baseUrl}/hc/app/${port}`;
      this.logger.debug('App URL (doorway)', { url });
      return url;
    }

    // Fallback: direct localhost connection
    const url = `ws://localhost:${port}`;
    this.logger.debug('App URL (localhost)', { url });
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

  getStorageBaseUrl(config: ConnectionConfig): string {
    // Check for Eclipse Che environment first
    if (this.isCheEnvironment() && config.useLocalProxy) {
      // In Che, return the Angular dev server's origin so URLs are absolute
      // but still same-origin. This allows:
      // 1. <img src="..."> tags to work (browser requests same origin)
      // 2. Angular proxy to intercept /api/* and /blob/* routes
      // 3. CORS issues to be avoided (same-origin requests)
      //
      // Note: Empty string doesn't work because Angular's proxy only intercepts
      // HttpClient requests, not browser resource loads like <img> tags.
      if (globalThis.window !== undefined && globalThis.location?.origin) {
        this.logger.debug('Storage base URL (Che via Angular proxy)', {
          url: globalThis.location.origin,
        });
        return globalThis.location.origin;
      }
      // SSR fallback - return empty and let requests be relative
      return '';
    }

    // Convert WebSocket URL to HTTPS for API access
    return config.adminUrl
      .replace('wss://', 'https://')
      .replace('ws://', 'http://')
      .replace(/\/$/, '');
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

  /**
   * Connect to Holochain conductor.
   *
   * Uses the Chaperone pattern (POST /hc/connect) in production when a
   * doorwayToken is present. Falls back to the admin WebSocket flow for
   * dev mode (Eclipse Che) or when no token is available.
   */
  async connect(config: ConnectionConfig): Promise<ConnectionResult> {
    this.resolveLogger(config);
    const useChaperone = !this.isCheEnvironment() && !!config.doorwayToken;

    if (useChaperone) {
      return this.connectViaChaperone(config);
    }

    // Dev mode / Eclipse Che: use admin WebSocket flow
    return this.connectViaAdminWs(config);
  }

  // ==========================================================================
  // Chaperone Connection (Production)
  // ==========================================================================

  /**
   * Connect via the Chaperone endpoint (POST /hc/connect).
   *
   * The browser generates signing keys locally, sends the public key +
   * cap secret to doorway, which handles all admin operations server-side.
   * Returns a single app auth token for the AppWebsocket connection.
   *
   * Eliminates: two-WebSocket routing, session affinity, admin protocol exposure.
   */
  private async connectViaChaperone(config: ConnectionConfig): Promise<ConnectionResult> {
    try {
      this.logger.info('Chaperone: starting connection...');

      // Step 1: Generate signing credentials locally
      const [keyPair, signingKey] = await generateSigningKeyPair();
      const capSecret = await randomCapSecret();
      this.credentials = { capSecret, keyPair, signingKey };

      // Step 2: Call POST /hc/connect
      const baseUrl = config.adminUrl
        .replace('wss://', 'https://')
        .replace('ws://', 'http://')
        .replace(/\/$/, '');

      const response = await fetch(`${baseUrl}/hc/connect`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${config.doorwayToken}`,
        },
        body: JSON.stringify({
          signingKey: this.toBase64(signingKey),
          capSecret: this.toBase64(capSecret),
        }),
      });

      if (!response.ok) {
        const errorBody = await response.text();
        throw new Error(`Chaperone failed (${response.status}): ${errorBody}`);
      }

      const data = (await response.json()) as {
        cellIds: Record<string, [string, string]>;
        token?: string;
        appToken: string;
        appPort: number;
        agentPubKey: string;
        conductorId: string;
      };

      // Step 3: Register signing credentials for each cell
      const cellIds = new Map<string, CellId>();
      for (const [roleName, [dnaHashB64, agentKeyB64]] of Object.entries(data.cellIds)) {
        const dnaHash = this.fromBase64(dnaHashB64);
        const agentKey = this.fromBase64(agentKeyB64);
        const cellId: CellId = [dnaHash, agentKey];

        cellIds.set(roleName, cellId);
        setSigningCredentials(cellId, { capSecret, keyPair, signingKey });
      }

      if (cellIds.size === 0) {
        throw new Error('Chaperone returned no cells');
      }

      // Step 4: Update stored JWT if chaperone returned a refreshed token
      if (data.token && data.token !== config.doorwayToken) {
        this.updateStoredToken(data.token);
      }

      // Step 5: Connect to App WebSocket (single connection)
      const appUrl = this.resolveAppUrl(config, data.appPort);
      this.logger.debug('Chaperone: connecting to app interface', { appUrl });

      // Decode the base64 app token back to bytes for AppWebsocket
      const appTokenBytes = this.fromBase64(data.appToken);
      this.appWs = await AppWebsocket.connect({
        url: new URL(appUrl),
        token: Array.from(appTokenBytes),
      });

      this.connected = true;

      const agentPubKey = this.fromBase64(data.agentPubKey) as AgentPubKey;

      this.logger.info('Chaperone: connected', {
        conductor: data.conductorId,
        cells: cellIds.size,
        appPort: data.appPort,
      });

      return {
        success: true,
        appWs: this.appWs,
        cellIds,
        agentPubKey,
        appPort: data.appPort,
      };
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown connection error';
      this.logger.error('Chaperone connection failed', errorMessage);

      return {
        success: false,
        error: errorMessage,
      };
    }
  }

  // ==========================================================================
  // Admin WebSocket Connection (Dev / Eclipse Che)
  // ==========================================================================

  /**
   * Connect via the full admin WebSocket flow.
   *
   * This is the legacy 11-step connection used in dev mode and Eclipse Che
   * where the admin WS proxy is available.
   */
  private async connectViaAdminWs(config: ConnectionConfig): Promise<ConnectionResult> {
    try {
      this.logger.info('AdminWS: starting connection...');

      // Step 1: Connect to Admin WebSocket (through proxy or direct)
      const adminUrl = this.resolveAdminUrl(config);
      this.logger.debug('Connecting to admin', { adminUrl });

      // Browser uses native WebSocket - wsClientOptions.origin is for Node.js only
      this.adminWs = await AdminWebsocket.connect({ url: new URL(adminUrl) });

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

      // If per-user app not found, fall back to base app name
      if (!appInfo && config.appId !== 'elohim') {
        this.logger.warn(`App '${config.appId}' not found, falling back to 'elohim'`);
        appInfo = await this.getInstalledApp(this.adminWs, 'elohim');
      }

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
      // Use retry helper since cap grants write to source chain and can race.
      // CellMissing errors are non-fatal — skip that cell and continue.
      const grantedCells = new Map<string, CellId>();
      for (const [roleName, cellId] of cellIds) {
        try {
          await this.withSourceChainRetry(
            async () =>
              this.adminWs!.grantZomeCallCapability({
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
              }),
            `cap grant for ${roleName}`
          );
          grantedCells.set(roleName, cellId);
          this.logger.debug(`Granted cap for role '${roleName}'`);
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          if (msg.includes('CellMissing')) {
            this.logger.warn(`Skipping cap grant for '${roleName}': CellMissing`);
          } else {
            throw err;
          }
        }
      }

      if (grantedCells.size === 0) {
        throw new Error('Failed to grant capabilities for any cell');
      }

      this.logger.debug(`Granted caps for ${grantedCells.size}/${cellIds.size} cells`);

      // Step 7: Register signing credentials for cells that got caps granted
      for (const [, cellId] of grantedCells) {
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
        // Create new interface with wildcard origins (proxy handles auth)
        const { port } = await this.adminWs.attachAppInterface({
          allowed_origins: '*',
        });
        appPort = port;
        this.logger.debug(`Created new app interface on port ${appPort}`);
      }

      // Step 9: Authorize signing credentials for cells that got caps granted
      // Use retry helper since authorization writes to source chain
      for (const [roleName, cellId] of grantedCells) {
        try {
          await this.withSourceChainRetry(
            async () => this.adminWs!.authorizeSigningCredentials(cellId),
            `authorize credentials for ${roleName}`
          );
          this.logger.debug(`Authorized credentials for role '${roleName}'`);
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          if (msg.includes('CellMissing')) {
            this.logger.warn(`Skipping authorize for '${roleName}': CellMissing`);
          } else {
            throw err;
          }
        }
      }

      // Step 10: Get app authentication token
      // Use the actual installed app ID (may differ from config.appId after fallback)
      const actualAppId = appInfo.installed_app_id;
      const issuedToken = await this.adminWs.issueAppAuthenticationToken({
        installed_app_id: actualAppId,
        single_use: false,
        expiry_seconds: 3600, // 1 hour
      });

      // Step 11: Connect to App WebSocket
      const appUrl = this.resolveAppUrl(config, appPort);
      this.logger.debug('Connecting to app interface', { appUrl });

      this.appWs = await AppWebsocket.connect({
        url: new URL(appUrl),
        token: issuedToken.token,
      });

      this.connected = true;

      this.logger.info('Connection successful', {
        appId: config.appId,
        cellCount: grantedCells.size,
        totalCells: cellIds.size,
      });

      return {
        success: true,
        adminWs: this.adminWs,
        appWs: this.appWs,
        cellIds: grantedCells,
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
   * Build query string parameters for WebSocket URLs.
   * Includes API key and JWT token for conductor affinity routing.
   */
  private buildQueryParams(config: ConnectionConfig): string {
    const params: string[] = [];
    if (config.proxyApiKey) {
      params.push(`apiKey=${encodeURIComponent(config.proxyApiKey)}`);
    }
    if (config.doorwayToken) {
      params.push(`token=${encodeURIComponent(config.doorwayToken)}`);
    }
    return params.join('&');
  }

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
   * Encode a Uint8Array as a base64 string.
   */
  private toBase64(bytes: Uint8Array): string {
    // Use btoa in browser environments
    if (typeof btoa === 'function') {
      let binary = '';
      for (const byte of bytes) {
        binary += String.fromCodePoint(byte);
      }
      return btoa(binary);
    }
    // Node.js fallback
    return Buffer.from(bytes).toString('base64');
  }

  /**
   * Decode a base64 string to a Uint8Array.
   */
  private fromBase64(str: string): Uint8Array {
    // Use atob in browser environments
    if (typeof atob === 'function') {
      const binary = atob(str);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.codePointAt(i)!;
      }
      return bytes;
    }
    // Node.js fallback
    return new Uint8Array(Buffer.from(str, 'base64'));
  }

  /**
   * Update the stored JWT token in localStorage after chaperone refresh.
   * The refreshed token contains conductor_id for future routing.
   */
  private updateStoredToken(token: string): void {
    if (typeof localStorage !== 'undefined' && localStorage) {
      try {
        localStorage.setItem('doorway_token', token);
        this.logger.debug('Updated stored JWT with conductor_id');
      } catch {
        this.logger.warn('Failed to update stored token');
      }
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
