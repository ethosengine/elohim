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
   * True when the unified SyncGate holds sync for ANY reason (operator pause, wifi-only on cellular, import/bulk backpressure, drain backlog). Operators and elohim agents use this to understand node load state; syncReasons names what holds it.
   */
  syncPaused: boolean;
  /**
   * Operator/device sync mode from the unified SyncGate. Sticky across restart (SQLite-persisted operational state, Category C).
   */
  syncMode: 'sync' | 'paused' | 'wifi-only';
  /**
   * Device-declared network class (declared via the API — never sniffed by the storage node). 'unknown' is honest and permissive: it never suppresses under wifi-only.
   */
  networkClass: 'wifi' | 'cellular' | 'unknown';
  /**
   * Reasons currently holding sync (kebab-case). Empty exactly when syncPaused is false.
   */
  syncReasons: (
    | 'operator-paused'
    | 'wifi-only-on-cellular'
    | 'import-in-progress'
    | 'bulk-write-in-progress'
    | 'drain-backlog'
  )[];
  /**
   * D.7 dedup LRU: number of unique CIDs currently in the dedup window.
   */
  dedupUniqueLen: number;
  /**
   * D.7 dedup LRU: cumulative insert calls (new + duplicate). Ratio (dedupTotalSeen - dedupUniqueLen) / dedupTotalSeen approximates duplication rate.
   */
  dedupTotalSeen: number;
  /**
   * T23 custody reconcile: total reconcile passes run since startup. CI asserts a custody reconcile pass ran without log scraping.
   */
  reconcilePassesTotal: number;
  /**
   * T23 custody reconcile: total fetch kicks fired across all passes. Non-zero confirms the reconciler emitted fetch requests for gaps.
   */
  kicksFiredTotal: number;
  /**
   * T23 custody reconcile: total placement gaps emitted across all passes.
   */
  placementGapsEmittedTotal: number;
  /**
   * Acquisition pull-queue rollup. null until a local active-pin × presence reconcile completes — treat as 'keep waiting', NEVER as caught up. Once non-null, total=0 is an observed empty desired set (spec §4.3).
   */
  pull?: {
    total: number;
    fetched: number;
    pending: number;
    failed: number;
    caughtUp: boolean;
  } | null;
  /**
   * P1 projection-reconcile stream status. null when the reconcile task is not running (missing lamad HcClient / pool / disabled).
   */
  projectionReconcile?: {
    pending: number;
    completed: number;
    failed: number;
    /**
     * This SWEEP ended: every discovered gap was healed OR exhausted its retry budget. NOT a convergence signal — use `converged`.
     */
    caughtUp: boolean;
    peersAsked: number;
    divergentAnchor: number;
    healedTotal: number;
    sweeps: number;
    /**
     * Gaps abandoned at max_retries this sweep — healed nothing, retried no more.
     */
    exhausted: number;
    /**
     * This peer holds what its peers advertised: pending==0 AND exhausted==0 AND divergentAnchor==0. Strictly stronger than caughtUp; an SLO may be offered over this field, not that one.
     */
    converged: boolean;
  } | null;
  /**
   * Provide-loop + re-anchor-backfill observability. null when the holder is not wired (e.g. a P2PNode built without the builder).
   */
  provideLoop?: {
    /**
     * Where self_cid came from at boot. 'unset' means the provide-loop cannot spawn (the dark-card root cause).
     */
    selfCidSource: 'env' | 'derived-libp2p-peer-id' | 'unset';
    /**
     * True once the Slice-2b provide-loop authoring tick has spawned. False = dormant = card cannot light its content:<reach> counts.
     */
    active: boolean;
    /**
     * Content rows still lacking dht_anchor_hash after the last re-anchor sweep.
     */
    reanchorPending: number;
    /**
     * Content rows re-authored (anchor stamped) this process lifetime.
     */
    reanchorCompleted: number;
    /**
     * Re-author failures (non-fatal, retried on a future boot's sweep).
     */
    reanchorFailed: number;
    /**
     * True when the re-anchor backfill ran and found no NULL-anchor rows left. False before the first run or while candidates remain.
     */
    reanchorCaughtUp: boolean;
  } | null;
  /**
   * The co-resident iroh node's NodeId (64-char hex), when this node runs the iroh transport stack alongside libp2p. Additive/optional — OMITTED from the wire on a libp2p-only node (dual-stack boot is mode-exclusive today). Set at the main.rs dual block from iroh_n.node_id().
   */
  irohNodeId?: string;
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
