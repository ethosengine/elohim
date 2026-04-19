-- Source of truth: DHT (mishpat DNA, GateDecisionChallenge entry, Category A notarized).
-- This table is a read-optimized projection populated by the post-commit signal
-- handler in src/signals.rs (MishpatSignal::GateDecisionChallengeCreated). Do not write
-- here directly — all writes flow from that signal. If this projection and the
-- DHT disagree, the DHT wins (rebuild from it).

CREATE TABLE gate_decision_challenges (
    -- Identity
    app_id TEXT NOT NULL,                      -- Installed app ID (multi-tenant isolation)
    challenge_id TEXT NOT NULL,                -- CID (self-addressing) — globally unique

    -- Challenge payload
    challenged_decision_cid TEXT NOT NULL,     -- CID of the GateDecisionAttestation challenged
    challenger_id TEXT NOT NULL,               -- AgentPubKey (base64) of the challenger
    grounds TEXT NOT NULL,                     -- factual-error | safety | policy | constitutional | indemnification-request
    summary TEXT NOT NULL,                     -- Challenger's articulation of the grievance
    evidence_refs TEXT NOT NULL,               -- Comma-separated CIDs (empty string if none)
    filed_at TEXT NOT NULL,                    -- ISO-8601 timestamp of filing
    reach TEXT NOT NULL,                       -- self | intimate | community | commons

    -- DHT provenance
    dht_anchor_hash TEXT NOT NULL,             -- ActionHash (base64) of the upstream DHT entry

    -- Local bookkeeping
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    PRIMARY KEY (app_id, challenge_id)
);

-- Index: look up all challenges filed by a given agent
CREATE INDEX idx_gate_challenges_challenger ON gate_decision_challenges(challenger_id);

-- Index: look up all challenges against a specific decision
CREATE INDEX idx_gate_challenges_decision ON gate_decision_challenges(challenged_decision_cid);

-- Index: look up challenges by grounds type
CREATE INDEX idx_gate_challenges_grounds ON gate_decision_challenges(grounds);

-- Index: time-ordered queries (audit windows, recent filings)
CREATE INDEX idx_gate_challenges_filed_at ON gate_decision_challenges(filed_at);
