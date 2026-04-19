/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/placement-gap-view.schema.json -- DO NOT EDIT */

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
