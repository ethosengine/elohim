/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/p2p-status-view.schema.json -- DO NOT EDIT */

/**
 * NAT status detected by autonat
 */
export type NatStatus = 'unknown' | 'public' | 'private';
/**
 * Relay mode this node is running in
 */
export type RelayMode = 'disabled' | 'client' | 'server' | 'both';

/**
 * P2P node status for observability. Source of truth: libp2p Swarm state + in-memory replication tracker + SQLite drain query (Operational, Category C). Reconstructed per request. Not persisted.
 */
export interface P2PStatusView {
  /**
   * libp2p PeerId (base58 encoded)
   */
  peerId: string;
  /**
   * Multiaddrs this node is listening on
   */
  listenAddresses: string[];
  /**
   * Number of currently connected peers
   */
  connectedPeers: number;
  /**
   * Configured bootstrap node multiaddrs
   */
  bootstrapNodes: string[];
  /**
   * Number of Automerge sync documents
   */
  syncDocuments: number;
  natStatus: NatStatus;
  /**
   * Number of active relay reservations
   */
  relayReservations: number;
  /**
   * Addresses announced to the network
   */
  announceAddresses: string[];
  relayMode: RelayMode;
  replication: ReplicationStatusView;
  /**
   * Drain queue state. null when DB pool or query unavailable — treat as 'data not available', NOT 'caught up'
   */
  drain?: DrainStatusView | null;
  /**
   * True when sync/replication is paused for backpressure (bulk write in progress). Operators and elohim agents use this to understand node load state.
   */
  syncPaused: boolean;
  /**
   * D.7 dedup LRU: number of unique CIDs currently in the dedup window.
   */
  dedupUniqueLen: number;
  /**
   * D.7 dedup LRU: cumulative insert calls (new + duplicate). Ratio (dedupTotalSeen - dedupUniqueLen) / dedupTotalSeen approximates duplication rate.
   */
  dedupTotalSeen: number;
}
/**
 * Identity-driven content replication progress
 */
export interface ReplicationStatusView {
  /**
   * Content IDs discovered but not yet fetched
   */
  pending: number;
  /**
   * Content IDs successfully replicated
   */
  completed: number;
  /**
   * Content IDs that failed fetch (will retry)
   */
  failed: number;
  /**
   * True when all discovered content has been fetched or failed with max retries
   */
  caughtUp: boolean;
}
/**
 * Drain queue state for DHT publication. Source of truth: SQLite aggregate query over p2p_published_at column (Operational, Category C).
 */
export interface DrainStatusView {
  /**
   * Total rows in the local content projection (scoped to lamad app)
   */
  total: number;
  /**
   * Rows successfully published to libp2p Kad DHT
   */
  published: number;
  /**
   * Rows not yet drained. 0 and stable = caught up
   */
  pending: number;
}
