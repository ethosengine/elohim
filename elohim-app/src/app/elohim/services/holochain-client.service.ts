/**
 * Holochain Client Service
 *
 * Manages WebSocket connections to a Holochain conductor (Edge Node).
 * Handles authentication, signing credentials, and zome calls.
 *
 * Architecture:
 *   Browser → WebSocket → Edge Node (conductor) → DHT
 *
 * @see https://github.com/holochain/holochain-client-js
 */

import { Injectable, signal, computed } from '@angular/core';
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
  DEFAULT_HOLOCHAIN_CONFIG,
  INITIAL_CONNECTION_STATE,
  SIGNING_CREDENTIALS_KEY,
} from '../models/holochain-connection.model';

@Injectable({
  providedIn: 'root',
})
export class HolochainClientService {
  /** Current connection state */
  private readonly connectionSignal = signal<HolochainConnection>(INITIAL_CONNECTION_STATE);

  /** Expose connection state as readonly */
  readonly connection = this.connectionSignal.asReadonly();

  /** Computed convenience accessors */
  readonly state = computed(() => this.connectionSignal().state);
  readonly isConnected = computed(() => this.connectionSignal().state === 'connected');
  readonly error = computed(() => this.connectionSignal().error);

  /** Current configuration */
  private config: HolochainConfig = DEFAULT_HOLOCHAIN_CONFIG;

  /**
   * Connect to Holochain conductor
   *
   * Connection flow:
   * 1. Connect to AdminWebsocket
   * 2. Generate or restore signing credentials
   * 3. Install app (if not already installed)
   * 4. Attach app interface and get auth token
   * 5. Connect to AppWebsocket with token
   */
  async connect(config?: Partial<HolochainConfig>): Promise<void> {
    if (config) {
      this.config = { ...DEFAULT_HOLOCHAIN_CONFIG, ...config };
    }

    this.updateState({ state: 'connecting' });

    try {
      // Step 1: Connect to Admin WebSocket
      const adminWs = await AdminWebsocket.connect({
        url: new URL(this.config.adminUrl),
        wsClientOptions: {
          origin: this.config.origin,
        },
      });

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

      // Step 8: Attach app interface and get issued token
      const { port: appPort } = await adminWs.attachAppInterface({
        allowed_origins: this.config.origin,
      });

      const issuedToken = await adminWs.issueAppAuthenticationToken({
        installed_app_id: this.config.appId,
      });

      // Step 9: Connect to App WebSocket
      const appWs = await AppWebsocket.connect({
        url: new URL(`ws://localhost:${appPort}`),
        wsClientOptions: {
          origin: this.config.origin,
        },
        token: issuedToken.token,
      });

      // Success!
      this.updateState({
        state: 'connected',
        appWs,
        connectedAt: new Date(),
        error: undefined,
      });

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
   * Make a zome call
   */
  async callZome<T>(input: ZomeCallInput): Promise<ZomeCallResult<T>> {
    const { appWs, cellId } = this.connectionSignal();

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
      console.error(`Zome call failed: ${input.zomeName}.${input.fnName}`, err);

      return {
        success: false,
        error: errorMessage,
      };
    }
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
   */
  private extractCellId(appInfo: AppInfo): CellId | null {
    // Look for the first provisioned cell
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
   * Encode agent public key for display
   */
  private encodeAgentPubKey(key: AgentPubKey): string {
    return this.uint8ArrayToBase64(key).substring(0, 12) + '...';
  }

  /**
   * Convert Uint8Array to base64
   */
  private uint8ArrayToBase64(arr: Uint8Array): string {
    return btoa(String.fromCharCode.apply(null, Array.from(arr)));
  }
}
