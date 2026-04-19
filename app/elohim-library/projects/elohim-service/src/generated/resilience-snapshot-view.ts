/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/resilience-snapshot-view.schema.json -- DO NOT EDIT */

/**
 * Enriched household-first resilience claim consumed by <elohim-resilience-snapshot>. Extends HouseholdResilienceView with commitment-backed counts, diversity score, regional distribution, and placement-gap rollup. Source of truth: computed projection. Operational Category C.
 */
export interface ResilienceSnapshotView {
  contentId: string;
  householdsStewarding: number;
  /**
   * Households with an active REA commitment covering this content.
   */
  commitmentBackedHouseholds: number;
  /**
   * 0..1 — ratio of achieved distinct-household placements to requested.
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
  householdsReciprocated?: number;
  details?: {
    stewardHouseholds?: string[];
    onlinePeerCount?: number;
    healthScore?: number;
  };
}
/**
 * Structured shefa signal: a content item's achieved placement falls short of its requested household diversity. Source of truth: computed projection from shard_locations + rea_commitments + humans.household_id. Operational Category C — no DHT entry.
 */
export interface PlacementGapView {
  /**
   * UUID of this gap record.
   */
  id: string;
  contentId: string;
  shardHash: string;
  requestedHouseholdCount: number;
  achievedHouseholdCount: number;
  /**
   * Fraction of requested diversity backed by active REA commitments.
   */
  contractCoverage: number;
  gapKind: 'under-committed' | 'contracts-short' | 'peers-unavailable';
  firstSeenAt: string;
  lastSeenAt: string;
}
