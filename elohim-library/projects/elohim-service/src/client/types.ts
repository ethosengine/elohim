/**
 * elohim-client types
 *
 * Type definitions for the Elohim Protocol client library.
 * Mirrors the Rust elohim-sdk patterns for consistency.
 */

/**
 * Client deployment mode
 *
 * Determines how the UI connects to backend services.
 * Content (heavy R/W) goes through elohim-storage → SQLite.
 * Agent data (attestations, identity) uses separate Holochain connection.
 */
export type ClientMode =
  | BrowserMode
  | TauriMode;

/**
 * Doorway configuration
 *
 * Doorways are public infrastructure providing:
 * - Projection store for browser clients
 * - Content sync relay
 * - Signal servers for peer discovery
 * - Holochain conductor WebSocket proxy
 */
export interface DoorwayConfig {
  /** Primary doorway URL */
  url: string;
  /** Fallback doorway URLs (tried in order if primary fails) */
  fallbacks?: string[];
  /** Optional API key for authenticated access */
  apiKey?: string;
}

/**
 * Node sync configuration
 *
 * elohim-nodes you operate for personal sync.
 * These are discovered/registered via DHT, not DNS.
 */
export interface NodeSyncConfig {
  /**
   * URLs of elohim-nodes you operate
   * These are personal nodes registered as yours in the DHT.
   * Can be local network addresses (no DNS required).
   */
  urls: string[];
  /**
   * Whether to prefer your nodes over public doorways for sync
   * @default true
   */
  preferOverDoorway?: boolean;
}

/**
 * Browser mode - Web app connecting via Doorway
 * No local storage, doorway-dependent, no offline capability
 */
export interface BrowserMode {
  type: 'browser';
  /** Doorway connection config */
  doorway: DoorwayConfig;
}

/**
 * Tauri mode - Desktop/mobile app with local storage
 * Full offline capability, syncs when online
 */
export interface TauriMode {
  type: 'tauri';
  /** Tauri invoke function for IPC calls to local elohim-storage */
  invoke: TauriInvoke;
  /**
   * Doorway config for public sync (optional)
   * Used when your personal nodes are unavailable.
   */
  doorway?: DoorwayConfig;
  /**
   * Personal elohim-nodes you operate (optional)
   * Discovered/registered via DHT by peers.
   * No DNS required - can be local network addresses.
   */
  nodes?: NodeSyncConfig;
}

/** Tauri invoke function signature */
export type TauriInvoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

/**
 * Holochain connection config (parallel to content, not a mode)
 * Used for agent-centric data: attestations, identity, points, consent
 *
 * Connection routing is mode-aware:
 * - Browser: WebSocket through Doorway proxy (wss://doorway.example.com/conductor)
 * - Tauri: Direct WebSocket to local conductor (ws://localhost:8888)
 *
 * The client automatically determines the connection method based on ClientMode.
 */
export interface HolochainConnection {
  /** App ID installed in conductor */
  appId: string;
  /** Whether connection is available */
  enabled: boolean;
  /**
   * Direct conductor URL (for Tauri mode)
   * In browser mode, this is ignored - connection goes through doorway
   */
  directConductorUrl?: string;
}

/**
 * Content type identifier
 * Used to route to the correct storage endpoint
 */
export type ContentType = 'content' | 'path' | 'blob' | 'progress' | string;

/**
 * Write operation priority
 */
export enum WritePriority {
  /** Flush immediately (user-initiated saves) */
  High = 0,
  /** Normal batching (background operations) */
  Normal = 1,
  /** Can wait for large batches (bulk imports) */
  Bulk = 2,
}

/**
 * Reach levels from most public to most private.
 *
 * The hierarchy is (most accessible to least accessible):
 * `commons` → `regional` → `bioregional` → `municipal` → `neighborhood` → `local` → `invited` → `private`
 *
 * Higher numeric values = more restricted access.
 */
export enum ReachLevel {
  /** Public commons - accessible to all (lowest barrier) */
  Commons = 0,
  /** Regional level */
  Regional = 1,
  /** Bioregional level */
  Bioregional = 2,
  /** Municipal level */
  Municipal = 3,
  /** Trusted neighborhood clusters */
  Neighborhood = 4,
  /** Local family/cluster only */
  Local = 5,
  /** Explicitly invited agents only */
  Invited = 6,
  /** Only the owner can access (highest barrier) */
  Private = 7,
}

/**
 * Write buffer configuration
 */
export interface WriteBufferConfig {
  /** Maximum items before auto-flush */
  maxItems: number;
  /** Maximum age (ms) before auto-flush */
  maxAgeMs: number;
  /** Backpressure threshold (0-100) */
  backpressureThreshold: number;
}

/**
 * Queued write operation
 */
export interface WriteOp {
  contentType: ContentType;
  id: string;
  data: unknown;
  priority: WritePriority;
  queuedAt: number;
}

/**
 * Client configuration
 */
export interface ElohimClientConfig {
  /** Client mode determines backend routing for content */
  mode: ClientMode;
  /** Write buffer configuration (optional, uses defaults) */
  writeBuffer?: Partial<WriteBufferConfig>;
  /** Agent's reach level for access control */
  agentReach?: ReachLevel;
  /** Holochain connection for agent-centric data (optional, parallel to content) */
  holochain?: HolochainConnection;
}

/**
 * Query options for batch content retrieval
 */
export interface ContentQuery {
  /** Filter by content type */
  contentType?: ContentType;
  /** Filter by tags (AND logic) */
  tags?: string[];
  /** Search in title/description */
  search?: string;
  /** Maximum results */
  limit?: number;
  /** Offset for pagination */
  offset?: number;
}

/**
 * Base interface for content that can be read
 */
export interface ContentReadable {
  /** Unique content ID */
  id: string;
  /** Optional reach level for access control */
  reach?: ReachLevel | string;
}

/**
 * Base interface for content that can be written
 */
export interface ContentWriteable extends ContentReadable {
  /** Validate before writing (throw on invalid) */
  validate?(): void;
}

/**
 * Default write buffer configurations
 */
export const WriteBufferDefaults = {
  interactive: {
    maxItems: 10,
    maxAgeMs: 1000,
    backpressureThreshold: 50,
  } as WriteBufferConfig,

  seeding: {
    maxItems: 100,
    maxAgeMs: 5000,
    backpressureThreshold: 80,
  } as WriteBufferConfig,

  default: {
    maxItems: 50,
    maxAgeMs: 2000,
    backpressureThreshold: 70,
  } as WriteBufferConfig,
};
