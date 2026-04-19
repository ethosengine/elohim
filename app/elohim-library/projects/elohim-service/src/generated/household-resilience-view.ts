/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/household-resilience-view.schema.json -- DO NOT EDIT */

/**
 * Per-content household-first resilience claim. Source of truth: computed projection aggregating stewardship_allocations (derived from Agreement DHT entries via stewardship-allocation link metadata, Category A2), peer_statuses (PeerStatus DHT entries in infrastructure DNA, Category A), and stewarded_nodes (NodeRegistration projection). Operational Category C — no persistence, no new DHT entry types.
 */
export interface HouseholdResilienceView {
  contentId: string;
  /**
   * Distinct households with at least one steward for this content.
   */
  householdsStewarding: number;
  /**
   * Households that both steward this and are stewarded by the viewer's household.
   */
  householdsReciprocated: number;
  protectionStatus: 'at-risk' | 'partial' | 'protected';
  details: {
    stewardHouseholds?: string[];
    onlinePeerCount?: number;
    healthScore?: number;
  };
}
