/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/topology-overview-view.schema.json -- DO NOT EDIT */

/**
 * Rolled-up topology for a single contributor DID — households, collectives, and inbound reciprocity. Source of truth: CozoDB graph projection via MEMBER_OF and RECIPROCATES_WITH edges (Operational, Category C). Reconstructed per request; not persisted.
 */
export interface TopologyOverviewView {
  /**
   * DID of the contributor this overview is scoped to.
   */
  contributorDid: string;
  /**
   * Households the contributor belongs to.
   */
  households: {
    householdCid: string;
    /**
     * Member DIDs in this household.
     */
    members?: string[];
    deviceCount?: number;
  }[];
  /**
   * Collectives the contributor is a member of.
   */
  collectives: {
    collectiveCid: string;
    memberCount?: number;
  }[];
  /**
   * Contributors sending reciprocity flows to this contributor.
   */
  reciprocityInbound?: {
    fromDid: string;
    weight: number;
  }[];
}
