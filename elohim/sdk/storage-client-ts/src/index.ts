/**
 * @elohim/storage-client
 *
 * TypeScript client for elohim-storage sync API
 *
 * @example
 * ```typescript
 * import { StorageClient, AutomergeSync } from '@elohim/storage-client';
 *
 * // Create client
 * const client = new StorageClient({
 *   baseUrl: 'http://localhost:8080',
 *   appId: 'lamad',
 * });
 *
 * // List documents
 * const { documents } = await client.listDocuments({ prefix: 'graph' });
 *
 * // Use Automerge sync helper
 * const sync = new AutomergeSync(client);
 * let doc = await sync.load('graph:my-doc');
 * doc = Automerge.change(doc, d => { d.title = 'Updated'; });
 * await sync.save('graph:my-doc', doc);
 * ```
 */

// Types
export {
  StorageConfig,
  StorageError,
  DocumentInfo,
  ListDocumentsResponse,
  GetDocumentResponse,
  GetHeadsResponse,
  GetChangesResponse,
  ApplyChangesResponse,
  BlobResult,
  BlobManifest,
  ListOptions,
} from './types';

// HTTP Client
export { StorageClient } from './client';

// Dataplane read/diagnostics facade (per-peer node ops; composes generated
// View types, never duplicates the Dataplane Read API's typed getters)
export {
  DataplaneApi,
  streamSyncState,
  VersionInfo,
  InventoryParity,
  ReaCommitmentQuery,
  EconomicEventQuery,
  StreamName,
  StreamSyncState,
} from './api/dataplane';

// Doorway admin/federation HTTP client (one client per doorway)
export { DoorwayClient, DoorwayConfig, SeedBlobResult } from './doorway';

// Multi-peer dataplane fleet, built from a compact CSV env-var shape
export { DataplaneFleet, FleetPeer } from './fleet';

// Automerge Sync
export { AutomergeSync, SyncResult, createSync } from './sync';

// Generated types from Rust via ts-rs
export * from './generated/index.js';
export type { NetworkPostureView } from './generated/NetworkPostureView.js';
export type { PeerListView } from './generated/PeerListView.js';
export type { PeerStatusView } from './generated/PeerStatusView.js';

// GraphQL query documents + response types (viewer.hub, viewer.peers)
export * from './graphql/index.js';

// Protocol wire-format types (Slice 2.5 — migrated from elohim-app pillar)
export * from './protocol-core.model.js';
export * from './zome-wire-types.js';
export * from './epr-head.model.js';
export * from './source-chain.model.js';
export type { IIntegrityAnchor } from './integrity-anchor.interface';
