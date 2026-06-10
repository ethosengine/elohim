/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/accumulation-status.schema.json -- DO NOT EDIT */

/**
 * Source of truth: derived projection of FeedbackSignal × Manifest(kind: standing-policy) (Operational, Category C). Computed by elohim-storage on Signal::FeedbackSignalCreated and Signal::ManifestUpdated{kind: standing-policy} events. Not persisted as DHT; reconstructable from the underlying FeedbackSignal entries and standing-policy Manifest payload.
 */
export interface AccumulationStatusView {
  /**
   * The type of entity this accumulation status applies to (e.g. 'content', 'path')
   */
  entityType: string;
  /**
   * The identifier of the entity this accumulation status applies to
   */
  entityId: string;
  /**
   * Total count of FeedbackSignal entries for this (entityType, entityId)
   */
  totalSignals: number;
  /**
   * Count of distinct signer pubkeys across FeedbackSignal entries for this entity
   */
  uniqueParticipants: number;
  /**
   * Normalized consensus strength in [0, 1]. 1.0 = full consensus; 0.0 = maximum diversity. Derived from normalized Shannon entropy of signal_kind distribution.
   */
  consensusStrength: number;
  /**
   * Computed accumulation status: 'pending' means insufficient signals; other values derive from standing-policy Manifest threshold comparison.
   */
  status: 'pending' | 'ready_for_sensemaking' | 'controversy_detected' | 'settled';
  /**
   * True when totalSignals >= threshold.readyForSensemaking.minTotalSignals AND consensusStrength < threshold.readyForSensemaking.maxConsensusStrength
   */
  readyForSensemaking: boolean;
  /**
   * True when totalSignals >= threshold.controversyDetected.minTotalSignals AND consensusStrength < threshold.controversyDetected.maxConsensusStrength
   */
  controversyDetected: boolean;
  /**
   * True when totalSignals >= threshold.settled.minTotalSignals AND consensusStrength >= threshold.settled.maxConsensusStrength (read as minConsensusStrength for 'settled')
   */
  settled: boolean;
  /**
   * CID of the standing-policy Manifest whose thresholds were applied. Null when protocol defaults were used.
   */
  policyManifestCid?: string | null;
  /**
   * ISO-8601 timestamp when this projection was last computed
   */
  computedAt: string;
}
