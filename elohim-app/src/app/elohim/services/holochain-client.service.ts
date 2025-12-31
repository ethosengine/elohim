/**
 * Holochain Client Service
 *
 * Manages WebSocket connections to a Holochain conductor (Edge Node).
 * Handles authentication, signing credentials, and zome calls.
 *
 * Architecture supports multiple deployment modes via ConnectionStrategy:
 *
 * 1. Doorway Mode (browser):
 *    Browser → Doorway Proxy (wss://doorway-dev.elohim.host) → Conductor
 *    Uses Projection tier for fast cached reads
 *
 * 2. Direct Mode (native/Tauri):
 *    Device → Local Conductor (ws://localhost:4444)
 *    Uses elohim-storage sidecar for blob storage
 *
 * @see https://github.com/holochain/holochain-client-js
 */

import { Injectable, signal, computed, inject } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import {
  AdminWebsocket,
  AppWebsocket,
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

import { PerformanceMetricsService } from './performance-metrics.service';
import { CONNECTION_STRATEGY } from '../providers/connection-strategy.provider';
import type { IConnectionStrategy, ConnectionConfig } from '../../../../../elohim-library/projects/elohim-service/src/connection';

@Injectable({
  providedIn: 'root',
})
export class HolochainClientService {
  /** HTTP client for REST API calls */
  private readonly http = inject(HttpClient);

  /** Performance metrics service for recording zome call metrics */
  private readonly metrics = inject(PerformanceMetricsService);

  /** Connection strategy (injected based on environment) */
  private readonly strategy = inject(CONNECTION_STRATEGY);

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

  // ==========================================================================
  // Strategy Accessors
  // ==========================================================================

  /** Get the current connection strategy name (e.g., 'doorway', 'direct') */
  get strategyName(): string {
    return this.strategy.name;
  }

  /** Get the current connection mode */
  get connectionMode(): 'doorway' | 'direct' {
    return this.strategy.mode;
  }

  /**
   * Get blob storage URL for a given hash.
   * Routes to appropriate storage based on connection mode:
   * - Doorway: https://doorway-dev.elohim.host/api/blob/{hash}
   * - Direct: http://localhost:8090/store/{hash}
   */
  getBlobUrl(blobHash: string): string {
    return this.strategy.getBlobStorageUrl(this.buildConnectionConfig(), blobHash);
  }

  /**
   * Get content sources for ContentResolver based on connection strategy.
   */
  getContentSources() {
    return this.strategy.getContentSources(this.buildConnectionConfig());
  }

  /**
   * Build ConnectionConfig from current HolochainConfig.
   */
  private buildConnectionConfig(): ConnectionConfig {
    return {
      mode: this.strategy.mode,
      adminUrl: this.config.adminUrl,
      appUrl: this.config.appUrl,
      proxyApiKey: this.config.proxyApiKey,
      storageUrl: this.config.storageUrl,
      appId: this.config.appId,
      happPath: this.config.happPath,
      origin: this.config.origin,
      useLocalProxy: this.config.useLocalProxy,
    };
  }

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
   * @deprecated Use strategy.resolveAdminUrl() instead. Kept for testAdminConnection().
   */
  private resolveAdminUrl(): string {
    return this.strategy.resolveAdminUrl(this.buildConnectionConfig());
  }

  /**
   * Resolve app interface URL based on environment and port.
   * @deprecated Use strategy.resolveAppUrl() instead. Kept for testAdminConnection().
   */
  private resolveAppUrl(port: number): string {
    return this.strategy.resolveAppUrl(this.buildConnectionConfig(), port);
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
   * Connect to Holochain conductor using the injected connection strategy.
   *
   * The strategy handles the full 11-step connection flow:
   * 1. Connect to AdminWebsocket (through proxy or direct)
   * 2. Generate signing credentials
   * 3. Generate agent key
   * 4. Check/install app
   * 5. Extract cell IDs (multi-DNA support)
   * 6. Grant zome call capabilities
   * 7. Register signing credentials
   * 8. Find/create app interface
   * 9. Authorize signing credentials
   * 10. Issue app auth token
   * 11. Connect to AppWebsocket
   *
   * The service manages Angular state updates based on strategy results.
   */
  async connect(config?: Partial<HolochainConfig>): Promise<void> {
    if (config) {
      this.config = { ...DEFAULT_HOLOCHAIN_CONFIG, ...config };
    }

    this.updateState({ state: 'connecting' });

    try {
      // Delegate to connection strategy
      const connectionConfig = this.buildConnectionConfig();
      console.log(`[HolochainClient] Connecting via ${this.strategy.name} strategy...`);

      this.updateState({ state: 'authenticating' });
      const result = await this.strategy.connect(connectionConfig);

      if (!result.success) {
        throw new Error(result.error || 'Connection failed');
      }

      // Update state from strategy result
      const firstCellId = result.cellIds?.values().next().value ?? null;

      this.updateState({
        state: 'connected',
        adminWs: result.adminWs ?? null,
        appWs: result.appWs ?? null,
        agentPubKey: result.agentPubKey ?? null,
        cellId: firstCellId,
        cellIds: result.cellIds ?? new Map(),
        appInfo: result.appInfo ?? null,
        connectedAt: new Date(),
        error: undefined,
      });

      // Store signing credentials from strategy for persistence
      const credentials = this.strategy.getSigningCredentials();
      if (credentials) {
        this.storeSigningCredentials(credentials);
      }

      // Reset error logging flag on successful connection
      this.connectionErrorLogged = false;

      console.log(`[HolochainClient] Connected via ${this.strategy.name}`, {
        appId: this.config.appId,
        agentPubKey: result.agentPubKey ? this.encodeAgentPubKey(result.agentPubKey) : 'N/A',
        cellCount: result.cellIds?.size ?? 0,
        mode: this.strategy.mode,
      });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown connection error';
      console.error(`[HolochainClient] Connection failed (${this.strategy.name}):`, errorMessage);

      this.updateState({
        state: 'error',
        error: errorMessage,
      });

      throw err;
    }
  }

  /**
   * Disconnect from Holochain conductor using the connection strategy.
   */
  async disconnect(): Promise<void> {
    try {
      await this.strategy.disconnect();
      console.log(`[HolochainClient] Disconnected (${this.strategy.name})`);
    } catch (err) {
      console.warn('[HolochainClient] Error during disconnect:', err);
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
      const { state, appWs, cellIds } = this.connectionSignal();

      if (state === 'connected' && appWs && cellIds.size > 0) {
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
   * Records performance metrics for all calls.
   *
   * For multi-DNA hApps, use input.roleName to specify which DNA to call:
   * - 'lamad' (default) - Content & Learning DNA (content_store zome)
   * - 'infrastructure' - Doorway Federation DNA
   * - 'imagodei' - Identity & Relationships DNA
   */
  async callZome<T>(input: ZomeCallInput): Promise<ZomeCallResult<T>> {
    let { appWs, cellIds, state } = this.connectionSignal();

    // Record start time for metrics
    const startTime = Date.now();

    // Default to 'lamad' role if not specified (backwards compatibility)
    const roleName = input.roleName ?? 'lamad';

    // Return immediately if in disconnected or error state
    if (state === 'disconnected' || state === 'error') {
      const duration = Date.now() - startTime;
      this.metrics.recordQuery(duration, false);
      return {
        success: false,
        error: 'Not connected to Holochain conductor',
      };
    }

    // If not connected yet, wait for connection (only if actively connecting)
    if (!appWs || cellIds.size === 0) {
      if (state === 'connecting' || state === 'authenticating') {
        console.log('[HolochainClient] Waiting for connection before zome call...');
        const connected = await this.waitForConnection();
        if (!connected) {
          const duration = Date.now() - startTime;
          this.metrics.recordQuery(duration, false);
          return {
            success: false,
            error: 'Connection timed out',
          };
        }
        // Re-read connection state after waiting
        ({ appWs, cellIds } = this.connectionSignal());
      }
    }

    if (!appWs || cellIds.size === 0) {
      const duration = Date.now() - startTime;
      this.metrics.recordQuery(duration, false);
      return {
        success: false,
        error: 'Not connected to Holochain conductor',
      };
    }

    // Look up the correct cell ID for this role
    const cellId = cellIds.get(roleName);
    if (!cellId) {
      const duration = Date.now() - startTime;
      this.metrics.recordQuery(duration, false);
      const availableRoles = Array.from(cellIds.keys()).join(', ');
      return {
        success: false,
        error: `No cell found for role '${roleName}'. Available roles: ${availableRoles}`,
      };
    }

    try {
      const result = await appWs.callZome({
        cell_id: cellId,
        zome_name: input.zomeName,
        fn_name: input.fnName,
        payload: input.payload,
      });

      // Record successful query
      const duration = Date.now() - startTime;
      this.metrics.recordQuery(duration, true);

      return {
        success: true,
        data: result as T,
      };
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Zome call failed';

      // Record failed query
      const duration = Date.now() - startTime;
      this.metrics.recordQuery(duration, false);

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
   * Records performance metrics for all calls.
   *
   * Endpoint: POST /api/v1/zome/{dna_hash}/{zome_name}/{fn_name}
   */
  async callZomeRest<T>(input: ZomeCallInput): Promise<ZomeCallResult<T>> {
    let { cellIds, state } = this.connectionSignal();
    const startTime = Date.now();

    // Default to 'lamad' role if not specified
    const roleName = input.roleName ?? 'lamad';

    // Need cellIds for DNA hash lookup
    if (cellIds.size === 0) {
      // If not connected but connecting, wait briefly
      if (state === 'connecting' || state === 'authenticating') {
        const connected = await this.waitForConnection(5000);
        if (!connected) {
          const duration = Date.now() - startTime;
          this.metrics.recordQuery(duration, false);
          return { success: false, error: 'Connection timed out' };
        }
        // Re-read connection state after waiting
        ({ cellIds } = this.connectionSignal());
      } else {
        const duration = Date.now() - startTime;
        this.metrics.recordQuery(duration, false);
        return { success: false, error: 'Not connected - no cell IDs available' };
      }
    }

    // Look up the correct cell ID for this role
    const cellId = cellIds.get(roleName);
    if (!cellId) {
      const duration = Date.now() - startTime;
      this.metrics.recordQuery(duration, false);
      const availableRoles = Array.from(cellIds.keys()).join(', ');
      return { success: false, error: `No cell found for role '${roleName}'. Available roles: ${availableRoles}` };
    }

    // Build REST API URL
    const dnaHash = this.uint8ArrayToBase64(cellId[0]);
    const restUrl = this.resolveRestUrl(dnaHash, input.zomeName, input.fnName);

    try {
      const response = await firstValueFrom(
        this.http.post<T>(restUrl, input.payload ?? null, {
          headers: {
            'Content-Type': 'application/json',
          },
        })
      );

      // Record successful REST call
      const duration = Date.now() - startTime;
      this.metrics.recordQuery(duration, true);

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

      // Record failed REST call
      const duration = Date.now() - startTime;
      this.metrics.recordQuery(duration, false);

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
   * Extract cell ID from app info (DEPRECATED - use extractAllCellIds)
   *
   * Holochain cell_info structure (0.6.x):
   * { "role_name": [{ type: "provisioned", value: { cell_id: {...} } }] }
   *
   * @deprecated Use extractAllCellIds for multi-DNA hApps
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
   * Extract all cell IDs from app info, keyed by role name.
   *
   * For multi-DNA hApps like Elohim, this returns a map:
   * - 'lamad' → CellId for content/learning DNA
   * - 'infrastructure' → CellId for doorway/network DNA
   * - 'imagodei' → CellId for identity DNA
   *
   * Holochain cell_info structure (0.6.x):
   * { "role_name": [{ type: "provisioned", value: { cell_id: [...] } }] }
   */
  private extractAllCellIds(appInfo: AppInfo): Map<string, CellId> {
    const cellIds = new Map<string, CellId>();
    const cellInfoEntries = Object.entries(appInfo.cell_info);

    for (const [roleName, cells] of cellInfoEntries) {
      const cellArray = cells as Array<{ type: string; value: { cell_id: CellId } }>;
      for (const cell of cellArray) {
        if (cell.type === 'provisioned' && cell.value?.cell_id) {
          cellIds.set(roleName, cell.value.cell_id);
          console.log(`[Holochain] Found cell for role '${roleName}':`, {
            dnaHash: this.uint8ArrayToBase64(cell.value.cell_id[0]).slice(0, 12) + '...',
          });
          break; // Only take first provisioned cell per role
        }
      }
    }

    return cellIds;
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
