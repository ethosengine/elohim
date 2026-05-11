/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/governance-action-tally-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: local (operational, Category C per p2p-design-gate). Recomputable from governance_actions JOIN attestations at any time via tally_projector::recompute. This record is NOT notarized on the DHT — only the constituent vote attestations are.
 */
export interface GovernanceActionTallyView {
  /**
   * CID of the parent governance-action
   */
  parentCid: string;
  /**
   * Governance kind discriminator (copied from parent)
   */
  governanceKind: string;
  /**
   * CID of the subject being acted upon (copied from parent)
   */
  subjectCid: string;
  /**
   * Required count for M-of-N quorum (threshold.m)
   */
  thresholdM: number;
  /**
   * Optional N for M-of-N quorum (threshold.n)
   */
  thresholdN?: number | null;
  /**
   * Optional percentage for percentage-threshold ballots
   */
  thresholdPercentage?: number | null;
  /**
   * ISO 8601 close time (copied from parent)
   */
  closesAt: string;
  /**
   * Current count of approve votes
   */
  currentApproveCount: number;
  /**
   * Current count of reject votes
   */
  currentRejectCount: number;
  /**
   * Current count of abstain votes
   */
  currentAbstainCount: number;
  /**
   * Computed status of the vote
   */
  computedStatus: 'pending' | 'reached-quorum' | 'rejected' | 'expired' | 'tied';
  /**
   * ISO 8601 timestamp of the most recent child vote, if any
   */
  lastChildAt?: string | null;
  /**
   * ISO 8601 timestamp when this tally row was last recomputed
   */
  rebuiltAt: string;
}
