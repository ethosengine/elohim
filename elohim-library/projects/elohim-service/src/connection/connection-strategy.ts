/**
 * Connection Strategy Interface Definitions
 *
 * Defines the contract for connecting to Holochain conductor in different
 * deployment modes:
 *
 * 1. **Doorway Mode** (Web/Browser):
 *    - Routes through Doorway proxy due to WebSocket same-origin restrictions
 *    - Uses Projection tier (MongoDB cache) for fast reads
 *    - Blob storage via Doorway API
 *
 * 2. **Direct Mode** (Native/Tauri):
 *    - Connects directly to local conductor (lower latency)
 *    - Skips Projection tier, goes directly to Authoritative
 *    - Blob storage via elohim-storage sidecar
 *
 * Usage:
 * ```typescript
 * import { createConnectionStrategy } from './connection-strategy-factory';
 *
 * const strategy = createConnectionStrategy('auto');
 * const result = await strategy.connect(config);
 * ```
 *
 * @packageDocumentation
 */

import type {
  AdminWebsocket,
  AppWebsocket,
  AgentPubKey,
  CellId,
  AppInfo,
} from '@holochain/client';

import { SourceTier } from '../cache/content-resolver';

// ============================================================================
// Connection Mode Types
// ============================================================================

/** Connection mode - determines which strategy to use */
export type ConnectionMode = 'auto' | 'doorway' | 'direct';

// ============================================================================
// Configuration Types
// ============================================================================

/**
 * Connection configuration passed to strategy methods.
 */
export interface ConnectionConfig {
  /** Connection mode */
  mode: ConnectionMode;

  /** Admin WebSocket URL (Doorway: wss://doorway-alpha.elohim.host, Direct: ws://localhost:4444) */
  adminUrl: string;

  /** App WebSocket URL (may differ from adminUrl in direct mode) */
  appUrl: string;

  /** API key for Doorway authentication */
  proxyApiKey?: string;

  /** elohim-storage sidecar URL (Direct mode: http://localhost:8090) */
  storageUrl?: string;

  /** Installed app ID in Holochain conductor */
  appId: string;

  /** Path to hApp bundle (for installation) */
  happPath?: string;

  /** Origin header for WebSocket connections */
  origin?: string;

  /** Use local dev-proxy in Eclipse Che (auto-detected if true) */
  useLocalProxy?: boolean;

  /** Doorway JWT token for conductor affinity routing (multi-conductor) */
  doorwayToken?: string;
}

/**
 * Content source configuration for ContentResolver.
 */
export interface ContentSourceConfig {
  /** Unique source identifier */
  id: string;

  /** Source tier (Local, Projection, Authoritative, External) */
  tier: SourceTier;

  /** Priority within tier (higher = preferred) */
  priority: number;

  /** Content types this source can provide */
  contentTypes: string[];

  /** Base URL for URL-based sources */
  baseUrl?: string;

  /** Whether source is initially available */
  available: boolean;
}

// ============================================================================
// Connection Result Types
// ============================================================================

/**
 * Result of a connection attempt.
 */
export interface ConnectionResult {
  /** Whether connection succeeded */
  success: boolean;

  /** Admin WebSocket connection (for admin operations) */
  adminWs?: AdminWebsocket;

  /** App WebSocket connection (for zome calls) */
  appWs?: AppWebsocket;

  /** Cell IDs keyed by role name (for multi-DNA hApps) */
  cellIds?: Map<string, CellId>;

  /** Agent public key for this session */
  agentPubKey?: AgentPubKey;

  /** Installed app info */
  appInfo?: AppInfo;

  /** App interface port (for app WebSocket) */
  appPort?: number;

  /** Error message if connection failed */
  error?: string;
}

/**
 * Signing credentials for zome call authentication.
 */
export interface SigningCredentials {
  /** Capability secret */
  capSecret: Uint8Array;

  /** Ed25519 key pair */
  keyPair: {
    publicKey: Uint8Array;
    privateKey: Uint8Array;
  };

  /** Signing key (derived from key pair) */
  signingKey: Uint8Array;
}

// ============================================================================
// Connection Strategy Interface
// ============================================================================

/**
 * Connection Strategy Interface
 *
 * Implementations encapsulate the logic for connecting to Holochain conductor
 * in different deployment environments.
 */
export interface IConnectionStrategy {
  // ==========================================================================
  // Identity
  // ==========================================================================

  /** Strategy identifier (e.g., 'doorway', 'direct') */
  readonly name: string;

  /** Connection mode this strategy implements */
  readonly mode: Exclude<ConnectionMode, 'auto'>;

  // ==========================================================================
  // Environment Detection
  // ==========================================================================

  /**
   * Check if this strategy is supported in the current environment.
   * For example, DirectConnectionStrategy requires Tauri or Node.js.
   */
  isSupported(): boolean;

  // ==========================================================================
  // URL Resolution
  // ==========================================================================

  /**
   * Resolve admin WebSocket URL.
   *
   * - Doorway: wss://doorway-alpha.elohim.host?apiKey=...
   * - Direct: ws://localhost:4444
   * - Che: wss://{workspace}-hc-dev/admin
   */
  resolveAdminUrl(config: ConnectionConfig): string;

  /**
   * Resolve app WebSocket URL for a given port.
   *
   * - Doorway: wss://doorway-alpha.elohim.host/app/{port}?apiKey=...
   * - Direct: ws://localhost:{port}
   * - Che: wss://{workspace}-hc-dev/app/{port}
   */
  resolveAppUrl(config: ConnectionConfig, port: number): string;

  /**
   * Get blob storage URL for a given hash.
   *
   * - Doorway: https://doorway-alpha.elohim.host/api/blob/{hash}
   * - Direct: http://localhost:8090/store/{hash} (elohim-storage sidecar)
   */
  getBlobStorageUrl(config: ConnectionConfig, blobHash: string): string;

  /**
   * Get the base URL for storage API operations.
   *
   * Used by StorageClientService for blob and metadata operations.
   * - Doorway: https://doorway-alpha.elohim.host (HTTP base from adminUrl)
   * - Direct: http://localhost:8090 (elohim-storage sidecar)
   */
  getStorageBaseUrl(config: ConnectionConfig): string;

  // ==========================================================================
  // Content Source Configuration
  // ==========================================================================

  /**
   * Get content sources for ContentResolver based on this strategy.
   *
   * - Doorway: indexeddb → projection → conductor
   * - Direct: indexeddb → conductor → elohim-storage (blobs only)
   */
  getContentSources(config: ConnectionConfig): ContentSourceConfig[];

  // ==========================================================================
  // Connection Lifecycle
  // ==========================================================================

  /**
   * Connect to Holochain conductor.
   *
   * Full connection flow:
   * 1. Connect to AdminWebsocket
   * 2. Generate signing credentials
   * 3. Generate agent public key
   * 4. Check/install app
   * 5. Extract cell IDs (multi-DNA)
   * 6. Grant zome call capability for all cells
   * 7. Register signing credentials
   * 8. Find or create app interface
   * 9. Authorize signing credentials
   * 10. Issue app auth token
   * 11. Connect to AppWebsocket
   */
  connect(config: ConnectionConfig): Promise<ConnectionResult>;

  /**
   * Disconnect from conductor.
   * Closes admin and app WebSocket connections.
   */
  disconnect(): Promise<void>;

  // ==========================================================================
  // Connection State
  // ==========================================================================

  /**
   * Check if currently connected.
   */
  isConnected(): boolean;

  /**
   * Get current signing credentials (if connected).
   */
  getSigningCredentials(): SigningCredentials | null;
}
