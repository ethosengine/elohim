/**
 * elohim-client
 *
 * Mode-aware content client library for the Elohim Protocol.
 * Mirrors the Rust elohim-sdk patterns for consistency across platforms.
 *
 * ## Client Modes (how UI connects)
 *
 * - **browser**: Web app via Doorway (no offline capability)
 * - **tauri**: Desktop/mobile with local elohim-storage (full offline)
 *
 * ## Sync Configuration
 *
 * - **DoorwayConfig**: Public doorways with fallbacks (DNS-based)
 * - **NodeSyncConfig**: Personal elohim-nodes you operate (DHT-registered)
 *
 * ## Parallel Connections (not modes)
 *
 * - **HolochainConnection**: For agent-centric data (attestations, identity, points)
 *
 * @example Browser mode (doorway-dependent, no offline)
 * ```typescript
 * import { ElohimClient } from 'elohim-service/client';
 *
 * const client = ElohimClient.anonymousBrowser('https://doorway.example.com');
 * const content = await client.get<Content>('content', 'manifesto');
 * ```
 *
 * @example Browser with doorway fallbacks
 * ```typescript
 * const client = new ElohimClient({
 *   mode: {
 *     type: 'browser',
 *     doorway: {
 *       url: 'https://doorway.example.com',
 *       fallbacks: ['https://backup1.example.com', 'https://backup2.example.com'],
 *       apiKey: 'optional-key',
 *     },
 *   },
 * });
 * ```
 *
 * @example Tauri mode (full offline, syncs when online)
 * ```typescript
 * import { ElohimClient } from 'elohim-service/client';
 * import { invoke } from '@tauri-apps/api/tauri';
 *
 * const client = new ElohimClient({
 *   mode: {
 *     type: 'tauri',
 *     invoke,
 *     // Public doorway for sync
 *     doorway: { url: 'https://doorway.example.com' },
 *     // Personal nodes (DHT-registered, no DNS needed)
 *     nodes: {
 *       urls: ['http://192.168.1.100:8090', 'http://my-node.local:8090'],
 *       preferOverDoorway: true,
 *     },
 *   },
 * });
 * ```
 *
 * @example With Holochain for agent data
 * ```typescript
 * const client = new ElohimClient({
 *   mode: { type: 'browser', doorway: { url: 'https://doorway.example.com' } },
 *   holochain: {
 *     appId: 'elohim',
 *     enabled: true,
 *     // Only used in Tauri mode (browser proxies through doorway)
 *     directConductorUrl: 'ws://localhost:8888',
 *   },
 * });
 * ```
 */

// Main client
export { ElohimClient, WriteBuffer, ReachEnforcer } from './elohim-client';

// Type exports (interfaces, type aliases) - must use 'export type' with isolatedModules
export type {
  // Modes
  ClientMode,
  BrowserMode,
  TauriMode,
  TauriInvoke,

  // Sync configuration
  DoorwayConfig,
  NodeSyncConfig,

  // Holochain (parallel connection, not a mode)
  HolochainConnection,

  // Content types
  ContentType,
  ContentReadable,
  ContentWriteable,
  ContentQuery,

  // Write buffer
  WriteOp,
  WriteBufferConfig,

  // Config
  ElohimClientConfig,
} from './types';

// Value exports (enums, consts) - normal export
export {
  WritePriority,
  WriteBufferDefaults,
  ReachLevel,
} from './types';

// Angular integration
export {
  ELOHIM_CLIENT,
  ELOHIM_CLIENT_CONFIG,
  provideElohimClient,
  provideAnonymousBrowserClient,
  detectClientMode,
} from './angular-provider';
