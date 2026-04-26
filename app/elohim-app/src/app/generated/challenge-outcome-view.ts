/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/challenge-outcome-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: DHT (mishpat DNA, ChallengeOutcome entry, Notarized, Category A). Records the reviewer consensus verdict that closes a GateDecisionChallenge. This record is a read-optimised projection populated by the MishpatSignal::ChallengeOutcomeCreated post-commit signal. If this projection and the DHT disagree, the DHT wins.
 */
export interface ChallengeOutcomeView {
  /**
   * CID of this outcome (self-addressing, globally unique)
   */
  outcomeId: string;
  /**
   * CID of the GateDecisionChallenge this outcome closes
   */
  challengeCid: string;
  /**
   * Verdict closing the challenge
   */
  verdict: 'upheld' | 'dismissed' | 'superseded';
  /**
   * AgentPubKeys (base64, comma-separated) of reviewers who reached consensus
   */
  reviewerConsensus: string;
  /**
   * Full ConstitutionalReasoning (opaque JSON string)
   */
  reasoningJson: string;
  /**
   * ISO 8601 timestamp of when the verdict was decided
   */
  decidedAt: string;
  /**
   * Indemnification actions as JSON array (empty array '[]' if no action required)
   */
  indemnificationActionsJson: string;
  /**
   * ActionHash (base64) of the upstream DHT entry — client can use this to verify provenance
   */
  dhtAnchorHash: string;
  /**
   * ISO 8601 timestamp of when this projection row was inserted
   */
  createdAt: string;
}
