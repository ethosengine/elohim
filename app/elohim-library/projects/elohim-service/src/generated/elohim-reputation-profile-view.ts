/* Generated from protocol schema: views/elohim-reputation-profile-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: computed aggregation over the DHT-notarized outcome graph (mishpat DNA — GateDecisionAttestation + GateDecisionChallenge + ChallengeOutcome entries). This view is NOT a direct projection of any single table — it is derived from the attestation and challenge tables scoped to a time window. No elohim can assert its own reputation; it emerges from the accumulated public record. Dimensions are raw counts; weighting (time-decay, severity) is deferred to Phase 11+ and applied by consumers. If the projection tables and the DHT disagree, the DHT wins.
 */
export interface ElohimReputationProfileView {
  /**
   * AgentPubKey (base64) of the queried elohim agent
   */
  elohimId: string;
  /**
   * ISO 8601 timestamp — start of the query window (inclusive)
   */
  windowStart: string;
  /**
   * ISO 8601 timestamp — end of the query window (inclusive)
   */
  windowEnd: string;
  /**
   * CID of the most-recently-used ElohimSubstance in the window (null if no decisions in window)
   */
  currentSubstanceCid?: string | null;
  /**
   * Count of GateDecisionAttestations where phase=elohim-active within the window. Dev-context decisions are excluded — they carry no reputation weight.
   */
  totalDecisions: number;
  /**
   * Count of distinct decisions (within window) that attracted at least one GateDecisionChallenge
   */
  challengedCount: number;
  /**
   * Count of ChallengeOutcomes with verdict=upheld for challenges against this elohim's window decisions
   */
  upheldCount: number;
  /**
   * Count of ChallengeOutcomes with verdict=dismissed for challenges against this elohim's window decisions
   */
  dismissedCount: number;
  /**
   * Count of ChallengeOutcomes with verdict=superseded for challenges against this elohim's window decisions
   */
  supersededCount: number;
  /**
   * Count of challenges against this elohim's window decisions that have no ChallengeOutcome yet
   */
  pendingCount: number;
  /**
   * Histogram of ChallengeGrounds values → count for all challenges in window. Keys are grounds strings (factual-error, safety, policy, constitutional, indemnification-request). Missing keys indicate zero challenges on those grounds.
   */
  challengesByGrounds: {
    [k: string]: number;
  };
  /**
   * Histogram of ChallengeVerdict values → count for all outcomes in window. Keys are verdict strings (upheld, dismissed, superseded). Missing keys indicate zero outcomes of that verdict.
   */
  outcomesByVerdict: {
    [k: string]: number;
  };
}
