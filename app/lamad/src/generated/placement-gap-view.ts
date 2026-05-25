/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/placement-gap-view.schema.json -- DO NOT EDIT */

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
