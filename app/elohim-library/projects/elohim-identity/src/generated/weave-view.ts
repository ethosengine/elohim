/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/weave-view.schema.json -- DO NOT EDIT */

/**
 * Cluster-scoped operational weave projection: placement/coverage/capacity folded per-shard → per-node → per-cluster. Each lens field is optional — a facing carries only the lenses it selected (the not-selected-field contract). Operational Category C — no DHT entry; projection only.
 */
export interface WeaveView {
  /**
   * Number of open shard placement gaps (under-replicated shards) at the time of measurement.
   */
  placementGapCount: number;
  /**
   * ISO-8601 timestamp at which this view was assembled.
   */
  measuredAt: string;
  /**
   * Mean RS contract-coverage across open placement gaps. 1.0 when there are no gaps (fully covered). Optional — absent when the lens was not selected.
   */
  rsCoverage?: number;
  clusterCapacity?: ComputeTriptych;
  /**
   * Risk-tier occupancy distribution (deferred — Slice 5 follow-on). Absent in Slice 4 responses.
   */
  tierOccupancy?: Record<string, unknown>;
  /**
   * Regional occupancy distribution (deferred — Slice 5 follow-on). Absent in Slice 4 responses.
   */
  regionOccupancy?: Record<string, unknown>;
}
/**
 * Aggregated storage capacity (free/used/stewarded bytes) summed across all custodian nodes in the cluster weave.
 */
export interface ComputeTriptych {
  /**
   * Bytes available on device blob filesystem (capacity - used)
   */
  free: number | null;
  /**
   * Bytes occupied by blob storage on device
   */
  used: number | null;
  /**
   * Bytes committed via rea_commitments where this peer is provider (action=custody-blob)
   */
  stewarded: number | null;
}
