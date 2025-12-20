/**
 * Holochain Client Service
 *
 * Manages WebSocket connections to a Holochain conductor (Edge Node).
 * Handles authentication, signing credentials, and zome calls.
 *
 * Architecture supports multiple environments:
 *
 * 1. Eclipse Che (dev-proxy):
 *    Browser → Dev Proxy (wss://hc-dev-...devspaces/) → Conductor (local)
 *    Routes: /admin → :4444, /app/:port → :port
 *
 * 2. Deployed (admin-proxy):
 *    Browser → Admin Proxy (wss://holochain-*.elohim.host) → Conductor → DHT
 *
 * 3. Local development:
 *    Browser → Conductor (ws://localhost:4444)
 *
 * @see https://github.com/holochain/holochain-client-js
 */

import { Injectable, signal, computed, inject } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
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

import {
  type HolochainConnection,
  type HolochainConfig,
  type HolochainConnectionState,
  type ZomeCallResult,
  type ZomeCallInput,
  type StoredSigningCredentials,
  type EdgeNodeDisplayInfo,
  DEFAULT_HOLOCHAIN_CONFIG,
  INITIAL_CONNECTION_STATE,
  SIGNING_CREDENTIALS_KEY,
} from '../models/holochain-connection.model';

@Injectable({
  providedIn: 'root',
})
export class HolochainClientService {
  /** HTTP client for REST API calls */
  private readonly http = inject(HttpClient);

  /** Current connection state */
  private readonly connectionSignal = signal<HolochainConnection>(INITIAL_CONNECTION_STATE);

  /** Track if connection error has been logged to avoid console spam */
  private connectionErrorLogged = false;

  /** Expose connection state as readonly */
  readonly connection = this.connectionSignal.asReadonly();

  /** Computed convenience accessors */
  readonly state = computed(() => this.connectionSignal().state);
  readonly isConnected = computed(() => this.connectionSignal().state === 'connected');
  readonly error = computed(() => this.connectionSignal().error);

  /** Current configuration */
  private config: HolochainConfig = DEFAULT_HOLOCHAIN_CONFIG;

  /**
   * Detect if running in Eclipse Che environment.
   * Checks for known Che URL patterns.
   */
  private isCheEnvironment(): boolean {
    return (
      window.location.hostname.includes('.devspaces.') ||
      window.location.hostname.includes('.code.ethosengine.com')
    );
  }

  /**
   * Get the dev proxy base URL in Che environment.
   * Looks for the hc-dev endpoint URL.
   */
  private getCheDevProxyUrl(): string | null {
    if (!this.isCheEnvironment()) return null;

    // In Che, each endpoint gets a unique URL like:
    // https://<workspace>-<endpoint>.code.ethosengine.com
    // Example: mbd06b-gmail-com-elohim-devspace-angular-dev.code.ethosengine.com
    // We need to replace the endpoint suffix (angular-dev) with (hc-dev)
    const currentUrl = new URL(window.location.href);

    // Replace '-angular-dev' suffix with '-hc-dev' in the hostname
    const hostname = currentUrl.hostname.replace(/-angular-dev\./, '-hc-dev.');

    console.log('[Holochain] Che URL resolution:', {
      currentHostname: currentUrl.hostname,
      resolvedHostname: hostname,
      devProxyUrl: `wss://${hostname}`,
    });

    return `wss://${hostname}`;
  }

  /**
   * Resolve admin URL based on environment.
   */
  private resolveAdminUrl(): string {
    const cheProxy = this.getCheDevProxyUrl();

    if (cheProxy && this.config.useLocalProxy) {
      // Che environment: use path-based routing through dev-proxy
      const url = `${cheProxy}/admin`;
      console.log('[Holochain] Admin URL resolved (Che dev-proxy):', url);
      return url;
    }

    // Default: use configured URL with optional API key
    const url = this.config.proxyApiKey
      ? `${this.config.adminUrl}?apiKey=${encodeURIComponent(this.config.proxyApiKey)}`
      : this.config.adminUrl;
    console.log('[Holochain] Admin URL resolved (direct):', url);
    return url;
  }

  /**
   * Resolve app interface URL based on environment and port.
   */
  private resolveAppUrl(port: number): string {
    const cheProxy = this.getCheDevProxyUrl();

    if (cheProxy && this.config.useLocalProxy) {
      // Che environment: use path-based routing with port through dev-proxy
      const url = `${cheProxy}/app/${port}`;
      console.log('[Holochain] App URL resolved (Che dev-proxy):', url);
      return url;
    }

    // Check if we have a configured admin URL (deployed environment)
    // Route app interface through admin-proxy using /app/:port path
    if (this.config.adminUrl && !this.config.adminUrl.includes('localhost')) {
      // Use admin URL base with /app/:port path
      const baseUrl = this.config.adminUrl.replace(/\/$/, ''); // Remove trailing slash
      const apiKeyParam = this.config.proxyApiKey
        ? `?apiKey=${encodeURIComponent(this.config.proxyApiKey)}`
        : '';
      const url = `${baseUrl}/app/${port}${apiKeyParam}`;
      console.log('[Holochain] App URL resolved (admin-proxy):', url);
      return url;
    }

    // Default: direct localhost connection (works for local dev)
    const url = `ws://localhost:${port}`;
    console.log('[Holochain] App URL resolved (direct):', url);
    return url;
  }

  /**
   * Test connection to admin proxy (Phase 1)
   *
   * Verifies we can reach the proxy and list apps.
   * Does NOT establish app interface (that's Phase 2).
   */
  async testAdminConnection(config?: Partial<HolochainConfig>): Promise<{
    success: boolean;
    apps?: string[];
    error?: string;
  }> {
    if (config) {
      this.config = { ...DEFAULT_HOLOCHAIN_CONFIG, ...config };
    }

    const isChe = this.isCheEnvironment();
    const mode = isChe && this.config.useLocalProxy ? 'Che dev-proxy' : 'direct';
    console.log(`[Holochain] Testing admin connection (${mode})...`);

    this.updateState({ state: 'connecting' });

    try {
      const adminUrl = this.resolveAdminUrl();

      // Browser uses native WebSocket - wsClientOptions.origin is for Node.js ws library only
      // In browser, omit wsClientOptions to avoid passing it as WebSocket subprotocol
      const adminWs = await AdminWebsocket.connect({ url: new URL(adminUrl) });

      this.updateState({ adminWs });

      // List apps to verify connection works
      const apps = await adminWs.listApps({});
      const appIds = apps.map((app) => app.installed_app_id);

      this.updateState({
        state: 'connected',
        connectedAt: new Date(),
        error: undefined,
      });

      // Reset error logging flag on successful connection
      this.connectionErrorLogged = false;

      console.log(`[Holochain] Connection successful (${mode})`, {
        url: adminUrl,
        apps: appIds,
      });

      return { success: true, apps: appIds };
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Connection failed';
      console.error(`[Holochain] Connection failed (${mode}):`, errorMessage);

      this.updateState({
        state: 'error',
        error: errorMessage,
      });

      return { success: false, error: errorMessage };
    }
  }

  /**
   * Connect to Holochain conductor (full flow)
   *
   * Connection flow:
   * 1. Connect to AdminWebsocket
   * 2. Generate or restore signing credentials
   * 3. Install app (if not already installed)
   * 4. Attach app interface and get auth token
   * 5. Connect to AppWebsocket with token
   *
   * NOTE: Step 5 currently fails from browser because attachAppInterface
   * returns a localhost port. Phase 2 will add app interface proxying.
   */
  async connect(config?: Partial<HolochainConfig>): Promise<void> {
    if (config) {
      this.config = { ...DEFAULT_HOLOCHAIN_CONFIG, ...config };
    }

    this.updateState({ state: 'connecting' });

    try {
      // Step 1: Connect to Admin WebSocket (through proxy or direct)
      const adminUrl = this.resolveAdminUrl();
      console.log('Connecting to admin:', adminUrl, { isChe: this.isCheEnvironment() });

      // Browser uses native WebSocket - wsClientOptions.origin is for Node.js ws library only
      // In browser, omit wsClientOptions to avoid passing it as WebSocket subprotocol
      const adminWs = await AdminWebsocket.connect({ url: new URL(adminUrl) });

      this.updateState({ adminWs });

      // Step 2: Generate signing credentials
      this.updateState({ state: 'authenticating' });

      const [keyPair, signingKey] = await generateSigningKeyPair();
      const capSecret = await randomCapSecret();

      // Step 3: Generate agent key
      const agentPubKey = await adminWs.generateAgentPubKey();
      this.updateState({ agentPubKey });

      // Step 4: Check if app is installed, install if needed
      let appInfo = await this.getInstalledApp(adminWs);

      if (!appInfo) {
        // App not installed - need to install it
        // Note: In production, the hApp should be pre-installed in the Edge Node
        console.log(`App ${this.config.appId} not installed. Attempting installation...`);

        if (this.config.happPath) {
          appInfo = await adminWs.installApp({
            source: { type: 'path', value: this.config.happPath },
            installed_app_id: this.config.appId,
            agent_key: agentPubKey,
          });

          await adminWs.enableApp({
            installed_app_id: this.config.appId,
          });
        } else {
          throw new Error(
            `App ${this.config.appId} not installed and no happPath provided for installation`
          );
        }
      }

      this.updateState({ appInfo });

      // Step 5: Get cell ID from app info
      const cellId = this.extractCellId(appInfo);
      if (!cellId) {
        throw new Error('Could not extract cell ID from app info');
      }
      this.updateState({ cellId });

      // Step 6: Grant zome call capability
      await adminWs.grantZomeCallCapability({
        cell_id: cellId,
        cap_grant: {
          tag: 'browser-signing',
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

      // Step 7: Register signing credentials with client
      setSigningCredentials(cellId, {
        capSecret,
        keyPair,
        signingKey,
      });

      // Store credentials for persistence (optional)
      this.storeSigningCredentials({ capSecret, keyPair, signingKey });

      // Step 8: Find or create app interface
      // First check for existing app interfaces to avoid creating duplicates
      const existingInterfaces = await adminWs.listAppInterfaces();
      let appPort: number;

      if (existingInterfaces.length > 0) {
        // Use the first existing app interface
        appPort = existingInterfaces[0].port;
        console.log(`[Holochain] Using existing app interface on port ${appPort}`);
      } else {
        // No existing interface, create one
        // Use "*" for allowed_origins because the proxy handles authentication
        // and may connect with different origins (browser origin, localhost, etc.)
        const { port } = await adminWs.attachAppInterface({
          allowed_origins: '*',
        });
        appPort = port;
        console.log(`[Holochain] Created new app interface on port ${appPort}`);
      }

      // Step 9: Authorize signing credentials for the cell
      await adminWs.authorizeSigningCredentials(cellId);
      console.log('[Holochain] Signing credentials authorized');

      // Step 10: Get app authentication token
      const issuedToken = await adminWs.issueAppAuthenticationToken({
        installed_app_id: this.config.appId,
        single_use: false,
        expiry_seconds: 3600, // 1 hour
      });

      // Step 11: Connect to App WebSocket
      // In Che: routes through dev-proxy path-based URL
      // Local: connects directly to localhost:port
      const appUrl = this.resolveAppUrl(appPort);
      console.log('[Holochain] Connecting to app interface:', appUrl);

      const appWs = await AppWebsocket.connect({
        url: new URL(appUrl),
        token: issuedToken.token,
      });

      // Success!
      this.updateState({
        state: 'connected',
        appWs,
        connectedAt: new Date(),
        error: undefined,
      });

      // Reset error logging flag on successful connection
      this.connectionErrorLogged = false;

      console.log('Connected to Holochain conductor', {
        appId: this.config.appId,
        agentPubKey: this.encodeAgentPubKey(agentPubKey),
      });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown connection error';
      console.error('Holochain connection failed:', errorMessage);

      this.updateState({
        state: 'error',
        error: errorMessage,
      });

      throw err;
    }
  }

  /**
   * Disconnect from Holochain conductor
   */
  async disconnect(): Promise<void> {
    const { adminWs, appWs } = this.connectionSignal();

    try {
      if (appWs) {
        await appWs.client.close();
      }
      if (adminWs) {
        await adminWs.client.close();
      }
    } catch (err) {
      console.warn('Error during disconnect:', err);
    }

    this.connectionSignal.set(INITIAL_CONNECTION_STATE);
  }

  /**
   * Wait for connection to be established (with timeout).
   * Returns true if connected, false if timed out.
   */
  async waitForConnection(timeoutMs: number = 10000): Promise<boolean> {
    const startTime = Date.now();
    const pollInterval = 100;

    while (Date.now() - startTime < timeoutMs) {
      const { state, appWs, cellId } = this.connectionSignal();

      if (state === 'connected' && appWs && cellId) {
        return true;
      }

      if (state === 'error') {
        return false;
      }

      await new Promise(resolve => setTimeout(resolve, pollInterval));
    }

    return false;
  }

  /**
   * Make a zome call (waits for connection if not yet established)
   */
  async callZome<T>(input: ZomeCallInput): Promise<ZomeCallResult<T>> {
    let { appWs, cellId, state } = this.connectionSignal();

    // Return immediately if in disconnected or error state
    if (state === 'disconnected' || state === 'error') {
      return {
        success: false,
        error: 'Not connected to Holochain conductor',
      };
    }

    // If not connected yet, wait for connection (only if actively connecting)
    if (!appWs || !cellId) {
      if (state === 'connecting' || state === 'authenticating') {
        console.log('[HolochainClient] Waiting for connection before zome call...');
        const connected = await this.waitForConnection();
        if (!connected) {
          return {
            success: false,
            error: 'Connection timed out',
          };
        }
        // Re-read connection state after waiting
        ({ appWs, cellId } = this.connectionSignal());
      }
    }

    if (!appWs || !cellId) {
      return {
        success: false,
        error: 'Not connected to Holochain conductor',
      };
    }

    try {
      const result = await appWs.callZome({
        cell_id: cellId,
        zome_name: input.zomeName,
        fn_name: input.fnName,
        payload: input.payload,
      });

      return {
        success: true,
        data: result as T,
      };
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Zome call failed';

      // Detect connection-related errors and only log once
      const isConnectionError = errorMessage.includes('Websocket') ||
                                errorMessage.includes('InvalidToken') ||
                                errorMessage.includes('not open') ||
                                errorMessage.includes('not connected');

      if (isConnectionError) {
        if (!this.connectionErrorLogged) {
          console.error(`[Holochain] Connection error: ${errorMessage} - subsequent errors will be suppressed`);
          this.connectionErrorLogged = true;
        }
      } else {
        // Log non-connection errors normally
        console.error(`Zome call failed: ${input.zomeName}.${input.fnName}`, err);
      }

      return {
        success: false,
        error: errorMessage,
      };
    }
  }

  /**
   * Make a zome call via REST API (uses Doorway cache for reads)
   *
   * This is preferred for read-only calls that benefit from caching.
   * The Doorway gateway caches responses based on zome-defined cache rules.
   *
   * Endpoint: POST /api/v1/zome/{dna_hash}/{zome_name}/{fn_name}
   */
  async callZomeRest<T>(input: ZomeCallInput): Promise<ZomeCallResult<T>> {
    const { cellId, state } = this.connectionSignal();

    // Need cellId for DNA hash
    if (!cellId) {
      // If not connected but connecting, wait briefly
      if (state === 'connecting' || state === 'authenticating') {
        const connected = await this.waitForConnection(5000);
        if (!connected) {
          return { success: false, error: 'Connection timed out' };
        }
      } else {
        return { success: false, error: 'Not connected - no cell ID available' };
      }
    }

    // Get current cellId after potential wait
    const currentCellId = this.connectionSignal().cellId;
    if (!currentCellId) {
      return { success: false, error: 'No cell ID available' };
    }

    // Build REST API URL
    const dnaHash = this.uint8ArrayToBase64(currentCellId[0]);
    const restUrl = this.resolveRestUrl(dnaHash, input.zomeName, input.fnName);

    try {
      const response = await firstValueFrom(
        this.http.post<T>(restUrl, input.payload ?? null, {
          headers: {
            'Content-Type': 'application/json',
          },
        })
      );

      return { success: true, data: response };
    } catch (err) {
      let errorMessage = 'REST call failed';
      if (err instanceof HttpErrorResponse) {
        if (err.error?.error) {
          errorMessage = err.error.error;
        } else if (err.message) {
          errorMessage = err.message;
        }
      } else if (err instanceof Error) {
        errorMessage = err.message;
      }

      console.error(`[Holochain REST] ${input.zomeName}.${input.fnName} failed:`, errorMessage);
      return { success: false, error: errorMessage };
    }
  }

  /**
   * Resolve REST API URL for zome calls
   * Converts WebSocket config to HTTP endpoint
   */
  private resolveRestUrl(dnaHash: string, zomeName: string, fnName: string): string {
    // Use the admin URL base, but convert to HTTPS for REST
    const baseUrl = this.config.adminUrl
      .replace('wss://', 'https://')
      .replace('ws://', 'http://')
      .replace(/\/$/, ''); // Remove trailing slash

    // Add API key if configured
    const apiKeyParam = this.config.proxyApiKey
      ? `?apiKey=${encodeURIComponent(this.config.proxyApiKey)}`
      : '';

    // Endpoint: /api/v1/zome/{dna_hash}/{zome_name}/{fn_name}
    return `${baseUrl}/api/v1/zome/${encodeURIComponent(dnaHash)}/${encodeURIComponent(zomeName)}/${encodeURIComponent(fnName)}${apiKeyParam}`;
  }

  /**
   * Get installed app info
   */
  private async getInstalledApp(adminWs: AdminWebsocket): Promise<AppInfo | null> {
    try {
      const apps = await adminWs.listApps({});
      return apps.find((app) => app.installed_app_id === this.config.appId) || null;
    } catch {
      return null;
    }
  }

  /**
   * Extract cell ID from app info
   *
   * Holochain cell_info structure (0.6.x):
   * { "role_name": [{ type: "provisioned", value: { cell_id: {...} } }] }
   */
  private extractCellId(appInfo: AppInfo): CellId | null {
    const cellInfoEntries = Object.entries(appInfo.cell_info);
    for (const [, cells] of cellInfoEntries) {
      const cellArray = cells as Array<{ type: string; value: { cell_id: CellId } }>;
      for (const cell of cellArray) {
        if (cell.type === 'provisioned' && cell.value?.cell_id) {
          return cell.value.cell_id;
        }
      }
    }
    return null;
  }

  /**
   * Update connection state
   */
  private updateState(partial: Partial<HolochainConnection>): void {
    this.connectionSignal.update((current) => ({
      ...current,
      ...partial,
    }));
  }

  /**
   * Store signing credentials in localStorage
   */
  private storeSigningCredentials(credentials: StoredSigningCredentials): void {
    try {
      // Convert Uint8Arrays to base64 for storage
      const serialized = {
        capSecret: this.uint8ArrayToBase64(credentials.capSecret),
        keyPair: {
          publicKey: this.uint8ArrayToBase64(credentials.keyPair.publicKey),
          privateKey: this.uint8ArrayToBase64(credentials.keyPair.privateKey),
        },
        signingKey: this.uint8ArrayToBase64(credentials.signingKey),
      };

      localStorage.setItem(SIGNING_CREDENTIALS_KEY, JSON.stringify(serialized));
    } catch (err) {
      console.warn('Could not store signing credentials:', err);
    }
  }

  /**
   * Encode agent public key for display (truncated)
   */
  private encodeAgentPubKey(key: AgentPubKey): string {
    return this.uint8ArrayToBase64(key).substring(0, 12) + '...';
  }

  /**
   * Convert Uint8Array to base64
   */
  public uint8ArrayToBase64(arr: Uint8Array): string {
    return btoa(String.fromCharCode.apply(null, Array.from(arr)));
  }

  // =========================================================================
  // Public Utility Methods for UI Display
  // =========================================================================

  /**
   * Get current configuration (for displaying URLs in UI)
   */
  public getConfig(): HolochainConfig {
    return this.config;
  }

  /**
   * Check if signing credentials exist in localStorage
   */
  public hasStoredCredentials(): boolean {
    try {
      const stored = localStorage.getItem(SIGNING_CREDENTIALS_KEY);
      return stored !== null;
    } catch {
      return false;
    }
  }

  /**
   * Get display-friendly connection info for UI rendering
   */
  public getDisplayInfo(): EdgeNodeDisplayInfo {
    const conn = this.connectionSignal();
    const config = this.config;

    let cellIdDisplay: { dnaHash: string; agentPubKey: string } | null = null;
    let dnaHash: string | null = null;

    if (conn.cellId) {
      dnaHash = this.uint8ArrayToBase64(conn.cellId[0]);
      cellIdDisplay = {
        dnaHash,
        agentPubKey: this.uint8ArrayToBase64(conn.cellId[1]),
      };
    }

    // Extract network seed from appInfo if available
    // AppInfo structure may have network_seed in manifest_network_seed
    const networkSeed = (conn.appInfo as { manifest_network_seed?: string })?.manifest_network_seed ?? null;

    return {
      state: conn.state,
      adminUrl: config.adminUrl,
      appUrl: config.appUrl,
      agentPubKey: conn.agentPubKey ? this.uint8ArrayToBase64(conn.agentPubKey) : null,
      cellId: cellIdDisplay,
      appId: config.appId,
      dnaHash,
      connectedAt: conn.connectedAt ?? null,
      hasStoredCredentials: this.hasStoredCredentials(),
      networkSeed,
      error: conn.error ?? null,
    };
  }
}
