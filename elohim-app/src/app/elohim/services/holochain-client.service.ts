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

import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, signal, computed, inject } from '@angular/core';

// @coverage: 38.7% (2026-02-05)

import { AdminWebsocket, type AgentPubKey, type CellId, type AppInfo } from '@holochain/client';
import { firstValueFrom } from 'rxjs';

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
import { CONNECTION_STRATEGY } from '../providers/connection-strategy.provider';

import { LoggerService } from './logger.service';
import { PerformanceMetricsService } from './performance-metrics.service';

import type { ConnectionConfig } from '@elohim/service/connection';

@Injectable({
  providedIn: 'root',
})
export class HolochainClientService {
  /** HTTP client for REST API calls */
  private readonly http = inject(HttpClient);

  /** Performance metrics service for recording zome call metrics */
  private readonly metrics = inject(PerformanceMetricsService);

  /** Logger for structured logging with correlation IDs */
  private readonly logger = inject(LoggerService).createChild('HolochainClient');

  /** Connection strategy (injected based on environment) */
  private readonly strategy = inject(CONNECTION_STRATEGY);

  /** Current connection state */
  private readonly connectionSignal = signal<HolochainConnection>(INITIAL_CONNECTION_STATE);

  /** Track if connection error has been logged to avoid console spam */
  private connectionErrorLogged = false;

  /** Auto-reconnect configuration */
  private readonly reconnectConfig = {
    enabled: true,
    maxRetries: 5,
    baseDelayMs: 1000,
    maxDelayMs: 30000,
    currentRetries: 0,
  };

  /** Reconnect timeout handle */
  private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;

  /** Track if we're currently attempting to reconnect */
  private isReconnecting = false;

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

    this.logger.debug('Che URL resolution', {
      currentHostname: currentUrl.hostname,
      resolvedHostname: hostname,
      devProxyUrl: `wss://${hostname}`,
    });

    return `wss://${hostname}`;
  }

  /**
   * Resolve admin URL based on environment.
   * Uses strategy pattern for environment-specific URL resolution.
   */
  private resolveAdminUrl(): string {
    return this.strategy.resolveAdminUrl(this.buildConnectionConfig());
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
    this.logger.info('Testing admin connection', { mode });

    this.updateState({ state: 'connecting' });

    try {
      const adminUrl = this.resolveAdminUrl();

      // Browser uses native WebSocket - wsClientOptions.origin is for Node.js ws library only
      // In browser, omit wsClientOptions to avoid passing it as WebSocket subprotocol
      const adminWs = await AdminWebsocket.connect({ url: new URL(adminUrl) });

      this.updateState({ adminWs });

      // List apps to verify connection works
      const apps = await adminWs.listApps({});
      const appIds = apps.map(app => app.installed_app_id);

      this.updateState({
        state: 'connected',
        connectedAt: new Date(),
        error: undefined,
      });

      // Reset error logging flag on successful connection
      this.connectionErrorLogged = false;

      this.logger.info('Connection successful', { mode, url: adminUrl, apps: appIds });

      return { success: true, apps: appIds };
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Connection failed';
      this.logger.error('Connection failed', err, { mode });

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
      this.logger.info('Connecting', { strategy: this.strategy.name });

      this.updateState({ state: 'authenticating' });
      const result = await this.strategy.connect(connectionConfig);

      if (!result.success) {
        throw new Error(result.error ?? 'Connection failed');
      }

      // Update state from strategy result
      const firstCellId = result.cellIds?.values().next().value ?? null;

      this.updateState({
        state: 'connected',
        adminWs: result.adminWs ?? undefined,
        appWs: result.appWs ?? undefined,
        agentPubKey: result.agentPubKey ?? undefined,
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

      this.logger.info('Connected', {
        strategy: this.strategy.name,
        appId: this.config.appId,
        agentPubKey: result.agentPubKey ? this.encodeAgentPubKey(result.agentPubKey) : 'N/A',
        cellCount: result.cellIds?.size ?? 0,
        mode: this.strategy.mode,
      });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown connection error';
      this.logger.error('Connection failed', err, { strategy: this.strategy.name });

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
    // Cancel any pending reconnect
    this.cancelReconnect();

    try {
      await this.strategy.disconnect();
      this.logger.info('Disconnected', { strategy: this.strategy.name });
    } catch (err) {
      this.logger.warn('Error during disconnect', {
        error: err instanceof Error ? err.message : String(err),
      });
    }

    this.connectionSignal.set(INITIAL_CONNECTION_STATE);
  }

  // ==========================================================================
  // Auto-Reconnect Logic
  // ==========================================================================

  /**
   * Enable or disable auto-reconnect.
   */
  setAutoReconnect(enabled: boolean): void {
    this.reconnectConfig.enabled = enabled;
    if (!enabled) {
      this.cancelReconnect();
    }
  }

  /**
   * Get current reconnect status.
   */
  getReconnectStatus(): { isReconnecting: boolean; retryCount: number; maxRetries: number } {
    return {
      isReconnecting: this.isReconnecting,
      retryCount: this.reconnectConfig.currentRetries,
      maxRetries: this.reconnectConfig.maxRetries,
    };
  }

  /**
   * Schedule a reconnection attempt with exponential backoff.
   */
  private scheduleReconnect(): void {
    if (!this.reconnectConfig.enabled) {
      this.logger.debug('Auto-reconnect disabled, not scheduling');
      return;
    }

    if (this.reconnectConfig.currentRetries >= this.reconnectConfig.maxRetries) {
      this.logger.error('Max reconnect attempts reached', undefined, {
        attempts: this.reconnectConfig.currentRetries,
        maxRetries: this.reconnectConfig.maxRetries,
      });
      this.isReconnecting = false;
      return;
    }

    // Calculate delay with exponential backoff
    const delay = Math.min(
      this.reconnectConfig.baseDelayMs * Math.pow(2, this.reconnectConfig.currentRetries),
      this.reconnectConfig.maxDelayMs
    );

    this.reconnectConfig.currentRetries++;
    this.isReconnecting = true;

    this.logger.info('Scheduling reconnect', {
      attempt: this.reconnectConfig.currentRetries,
      maxRetries: this.reconnectConfig.maxRetries,
      delayMs: delay,
    });

    this.updateState({ state: 'reconnecting' as HolochainConnectionState });

    this.reconnectTimeout = setTimeout(async () => {
      try {
        await this.connect();
        // Success - reset retry count
        this.reconnectConfig.currentRetries = 0;
        this.isReconnecting = false;
        this.logger.info('Reconnection successful');
      } catch (err) {
        this.logger.warn('Reconnection attempt failed', {
          attempt: this.reconnectConfig.currentRetries,
          error: err instanceof Error ? err.message : String(err),
        });
        // Schedule another attempt
        this.scheduleReconnect();
      }
    }, delay);
  }

  /**
   * Cancel any pending reconnection attempt.
   */
  private cancelReconnect(): void {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
    this.isReconnecting = false;
    this.reconnectConfig.currentRetries = 0;
  }

  /**
   * Handle connection loss - trigger reconnect if enabled.
   */
  private handleConnectionLost(reason?: string): void {
    if (this.connectionSignal().state === 'disconnected') {
      return; // Already disconnected intentionally
    }

    this.logger.warn('Connection lost', { reason });
    this.updateState({
      state: 'error',
      error: reason ?? 'Connection lost',
    });

    // Trigger auto-reconnect
    if (this.reconnectConfig.enabled && !this.isReconnecting) {
      this.scheduleReconnect();
    }
  }

  /**
   * Wait for connection to be established (with timeout).
   * Returns true if connected, false if timed out.
   *
   * Default timeout is 30 seconds to accommodate doorway connections
   * which can take 15-20 seconds on first connect.
   */
  async waitForConnection(timeoutMs = 30000): Promise<boolean> {
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
    const { state } = this.connectionSignal();
    let { appWs, cellIds } = this.connectionSignal();

    // Generate correlation ID for this request (for tracing)
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const randomStr = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 6);
    const correlationId = `zome-${Date.now()}-${randomStr}`;
    const callContext = {
      correlationId,
      zomeName: input.zomeName,
      fnName: input.fnName,
      roleName: input.roleName ?? 'lamad',
    };

    // Use logger timer for automatic duration tracking
    const timer = this.logger.startTimer('zome-call');

    // Default to 'lamad' role if not specified (backwards compatibility)
    const roleName = input.roleName ?? 'lamad';

    // Return immediately if in disconnected or error state
    if (state === 'disconnected' || state === 'error') {
      timer.end({ ...callContext, success: false, reason: 'not_connected' });
      this.metrics.recordQuery(timer.elapsed(), false);
      return {
        success: false,
        error: 'Not connected to Holochain conductor',
      };
    }

    // If not connected yet, wait for connection (only if actively connecting)
    if ((!appWs || cellIds.size === 0) && (state === 'connecting' || state === 'authenticating')) {
      this.logger.debug('Waiting for connection before zome call', callContext);
      const connected = await this.waitForConnection();
      if (!connected) {
        timer.end({ ...callContext, success: false, reason: 'timeout' });
        this.metrics.recordQuery(timer.elapsed(), false);
        return {
          success: false,
          error: 'Connection timed out',
        };
      }
      // Re-read connection state after waiting
      ({ appWs, cellIds } = this.connectionSignal());
    }

    if (!appWs || cellIds.size === 0) {
      timer.end({ ...callContext, success: false, reason: 'no_connection' });
      this.metrics.recordQuery(timer.elapsed(), false);
      return {
        success: false,
        error: 'Not connected to Holochain conductor',
      };
    }

    // Look up the correct cell ID for this role
    const cellId = cellIds.get(roleName);
    if (!cellId) {
      const availableRoles = Array.from(cellIds.keys()).join(', ');
      timer.end({ ...callContext, success: false, reason: 'no_cell', availableRoles });
      this.metrics.recordQuery(timer.elapsed(), false);
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
      const duration = timer.elapsed();
      this.metrics.recordQuery(duration, true);
      timer.end({ ...callContext, success: true });

      return {
        success: true,
        data: result as T,
      };
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Zome call failed';
      const duration = timer.elapsed();
      this.metrics.recordQuery(duration, false);

      // Detect connection-related errors and only log once
      const isConnectionError =
        errorMessage.includes('Websocket') ||
        errorMessage.includes('InvalidToken') ||
        errorMessage.includes('not open') ||
        errorMessage.includes('not connected');

      if (isConnectionError) {
        if (!this.connectionErrorLogged) {
          this.logger.error('Connection error (subsequent errors suppressed)', err, callContext);
          this.connectionErrorLogged = true;
        }
      } else {
        this.logger.error('Zome call failed', err, callContext);
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
    const { state } = this.connectionSignal();
    let { cellIds } = this.connectionSignal();
    const startTime = Date.now();

    // Default to 'lamad' role if not specified
    const roleName = input.roleName ?? 'lamad';

    // Need cellIds for DNA hash lookup
    if (cellIds.size === 0) {
      // If not connected but connecting, wait for connection
      if (state === 'connecting' || state === 'authenticating') {
        const connected = await this.waitForConnection();
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
      return {
        success: false,
        error: `No cell found for role '${roleName}'. Available roles: ${availableRoles}`,
      };
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

      this.logger.error('REST call failed', undefined, {
        zomeName: input.zomeName,
        fnName: input.fnName,
        error: errorMessage,
      });
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
      return apps.find(app => app.installed_app_id === this.config.appId) ?? null;
    } catch {
      return null;
    }
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
      const cellArray = cells as { type: string; value: { cell_id: CellId } }[];
      for (const cell of cellArray) {
        if (cell.type === 'provisioned' && cell.value?.cell_id) {
          cellIds.set(roleName, cell.value.cell_id);
          this.logger.debug('Found cell for role', {
            roleName,
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
    this.connectionSignal.update(current => ({
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
      this.logger.warn('Could not store signing credentials', {
        error: err instanceof Error ? err.message : String(err),
      });
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
    return btoa(String.fromCodePoint(...Array.from(arr)));
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
    const networkSeed =
      (conn.appInfo as { manifest_network_seed?: string })?.manifest_network_seed ?? null;

    return {
      state: conn.state,
      mode: this.connectionMode,
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
