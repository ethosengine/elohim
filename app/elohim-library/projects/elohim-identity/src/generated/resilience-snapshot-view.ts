/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/resilience-snapshot-view.schema.json -- DO NOT EDIT */

/**
 * Resilience claim consumed by <elohim-resilience-snapshot>. Collective-general: stewards of the content can be any collective kind (household, church, patron-circle, DAO, …) — any group that holds DHT-notarized REA commitments under the social-compute epic. Source of truth: computed projection. Operational Category C.
 */
export interface ResilienceSnapshotView {
  contentId: string;
  /**
   * Distinct collectives (of any kind) with at least one peer holding this content.
   */
  stewardingCollectives: number;
  /**
   * Distinct collectives with an active REA commitment covering this content.
   */
  commitmentBackedCollectives: number;
  /**
   * 0..1 — ratio of achieved distinct-collective placements to requested.
   */
  diversityScore: number;
  regionalDistribution: {
    local: number;
    regional: number;
    global: number;
    unknown: number;
  };
  placementGaps: PlacementGapView[];
  protectionStatus: 'at-risk' | 'partial' | 'protected';
  /**
   * Collectives that both steward this content AND are stewarded by the viewer's collective (mutual reciprocation). Optional; omitted when no viewer context is supplied.
   */
  reciprocatingCollectives?: number;
  details?: {
    /**
     * Collectives currently stewarding this content, with their kind so the UI can label per-kind ('household', 'church', 'patron-circle', etc.).
     */
    stewardingCollectives?: {
      id: string;
      /**
       * Collective kind: 'household', 'church', 'patron-circle', 'dao', or any future collective kind declared in the collectives table.
       */
      kind: string;
      /**
       * Optional human-readable display label.
       */
      label?: string;
    }[];
    onlinePeerCount?: number;
    healthScore?: number;
  };
}
/**
 * Structured shefa signal: a content item's achieved placement falls short of its requested stewarding-collective diversity. The 'steward' can be any collective kind (household, church, patron-circle, DAO, …) — any group that can hold DHT-notarized REA commitments. Source of truth: computed projection from shard_locations + rea_commitments + humans → collectives. Operational Category C — no DHT entry.
 */
export interface PlacementGapView {
  /**
   * UUID of this gap record.
   */
  id: string;
  contentId: string;
  shardHash: string;
  /**
   * Target number of distinct stewarding collectives (any kind) the placement should reach.
   */
  requestedStewardCount: number;
  /**
   * Actual number of distinct stewarding collectives that accepted the shard.
   */
  achievedStewardCount: number;
  /**
   * Fraction of requested diversity backed by active REA commitments.
   */
  contractCoverage: number;
  gapKind: 'under-committed' | 'contracts-short' | 'peers-unavailable';
  firstSeenAt: string;
  lastSeenAt: string;
}
