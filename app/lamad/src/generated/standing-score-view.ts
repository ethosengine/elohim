/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/standing-score-view.schema.json -- DO NOT EDIT */

/**
 * Wire shape for GET /api/v1/standing/{agent_cid}?evaluator={evaluator_cid}. Returns the StandingScore tier (Floor/Low/Neutral/High/Trusted) for the (evaluator, subject) pair, plus recent FeedbackSignal evidence summary. Source of truth: local SQLite operational projection (standing_view table); recomputable from FeedbackSignal subgraph (Holochain DHT) via standing_projector.
 */
export interface StandingScoreView {
  evaluator_cid: string;
  subject_cid: string;
  /**
   * The 5-tier StandingScore enum (Unknown if cold-start).
   */
  score: 'Unknown' | 'Floor' | 'Low' | 'Neutral' | 'High' | 'Trusted';
  /**
   * Sum of weighted FeedbackSignal debits applied to this score within the window.
   */
  debit_weight_sum?: number;
  /**
   * Up to N most recent debit-class signals shaping this score.
   */
  recent_breaches: {
    signal_kind: string;
    emitted_at: string;
    weight: number;
    evidence_summary?: string;
  }[];
  /**
   * ISO 8601 timestamp when this score was last projected.
   */
  computed_at: string;
}
