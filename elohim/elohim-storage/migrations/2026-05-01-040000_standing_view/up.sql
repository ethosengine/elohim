-- Source of truth: FeedbackSignal subgraph (Category C operational — recomputable).
-- This table is a local per-evaluator standing projection. It is NOT a stored
-- authoritative score. Different evaluators project different views of the same
-- subject (pluralism property per brainstorm §4.2). Recomputable by replaying the
-- FeedbackSignal subgraph through the evaluator's manifest debit weights.
CREATE TABLE standing_view (
    evaluator_pubkey BLOB NOT NULL,
    subject_pubkey BLOB NOT NULL,
    score TEXT NOT NULL,
    debit_weight_sum INTEGER NOT NULL DEFAULT 0,
    last_signal_at TEXT NOT NULL,
    manifest_cid TEXT NOT NULL,
    PRIMARY KEY (evaluator_pubkey, subject_pubkey)
);
CREATE INDEX idx_standing_view_subject ON standing_view(subject_pubkey);
