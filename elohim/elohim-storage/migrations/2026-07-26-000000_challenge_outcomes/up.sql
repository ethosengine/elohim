-- Source of truth: DHT (mishpat DNA, ChallengeOutcome entry, Category A notarized).
-- This table is a read-optimized projection populated by the post-commit signal
-- handler in src/signals.rs (MishpatSignal::ChallengeOutcomeCreated). Do not write
-- here directly — all writes flow from that signal. If this projection and the
-- DHT disagree, the DHT wins (rebuild from it).
--
-- VERSION MOVED 2026-07-26 (was 2026-04-19-000001). That version collided with
-- `2026-04-19-000001_stewarded_nodes_add_archetype`; `embed_migrations!` keys on
-- the timestamp prefix, so it kept ONE of the pair and silently dropped this
-- one. The sibling won, meaning `challenge_outcomes` was NEVER created on any
-- node — `GET /db/challenge-outcomes` errored with "no such table" and the
-- ChallengeOutcomeCreated signal projection failed on every delivery. Re-versioned
-- past the newest migration so it runs; `IF NOT EXISTS` keeps it safe on any node
-- where the collision happened to resolve the other way.

CREATE TABLE IF NOT EXISTS challenge_outcomes (
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
CREATE INDEX IF NOT EXISTS idx_challenge_outcomes_challenge ON challenge_outcomes(challenge_cid);

-- Index: look up outcomes by verdict type
CREATE INDEX IF NOT EXISTS idx_challenge_outcomes_verdict ON challenge_outcomes(verdict);

-- Index: time-ordered queries (audit windows)
CREATE INDEX IF NOT EXISTS idx_challenge_outcomes_decided_at ON challenge_outcomes(decided_at);
