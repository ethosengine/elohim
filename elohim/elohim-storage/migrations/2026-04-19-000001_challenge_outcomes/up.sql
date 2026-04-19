-- Source of truth: DHT (mishpat DNA, ChallengeOutcome entry, Category A notarized).
-- This table is a read-optimized projection populated by the post-commit signal
-- handler in src/signals.rs (MishpatSignal::ChallengeOutcomeCreated). Do not write
-- here directly — all writes flow from that signal. If this projection and the
-- DHT disagree, the DHT wins (rebuild from it).

CREATE TABLE challenge_outcomes (
    -- Identity
    app_id TEXT NOT NULL,                          -- Installed app ID (multi-tenant isolation)
    outcome_id TEXT NOT NULL,                      -- CID (self-addressing) — globally unique

    -- Outcome payload
    challenge_cid TEXT NOT NULL,                   -- CID of the GateDecisionChallenge closed by this outcome
    verdict TEXT NOT NULL,                         -- upheld | dismissed | superseded
    reviewer_consensus TEXT NOT NULL,              -- Comma-separated AgentPubKeys (base64) of reviewers
    reasoning_json TEXT NOT NULL,                  -- Full ConstitutionalReasoning (JSON)
    decided_at TEXT NOT NULL,                      -- ISO-8601 timestamp of decision
    indemnification_actions_json TEXT NOT NULL,    -- Indemnification actions (JSON array; "[]" if none)

    -- DHT provenance
    dht_anchor_hash TEXT NOT NULL,                 -- ActionHash (base64) of the upstream DHT entry

    -- Local bookkeeping
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    PRIMARY KEY (app_id, outcome_id)
);

-- Index: look up the outcome for a given challenge (at most one outcome per challenge)
CREATE INDEX idx_challenge_outcomes_challenge ON challenge_outcomes(challenge_cid);

-- Index: look up outcomes by verdict type
CREATE INDEX idx_challenge_outcomes_verdict ON challenge_outcomes(verdict);

-- Index: time-ordered queries (audit windows)
CREATE INDEX idx_challenge_outcomes_decided_at ON challenge_outcomes(decided_at);
