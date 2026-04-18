-- Source of truth: DHT (mishpat DNA, GateDecisionAttestation entry, Category A notarized).
-- This table is a read-optimized projection populated by the post-commit signal
-- handler in src/signals.rs (MishpatSignal::GateDecisionCreated). Do not write
-- here directly — all writes flow from that signal. If this projection and the
-- DHT disagree, the DHT wins (rebuild from it).

CREATE TABLE gate_decision_attestations (
    -- Identity
    app_id TEXT NOT NULL,                  -- Installed app ID (multi-tenant isolation)
    decision_id TEXT NOT NULL,             -- CID (self-addressing) — globally unique

    -- Gate evaluation provenance
    phase TEXT NOT NULL,                   -- "dev-context" | "elohim-active"
    elohim_id TEXT NOT NULL,               -- AgentPubKey (base64) of the deciding elohim
    elohim_substance_cid TEXT NOT NULL,    -- CID of substance declaration (model+constitution)
    gate_name TEXT NOT NULL,               -- e.g. "discernment-gate-v1-mechanical"
    gate_process_cid TEXT NOT NULL,        -- CID of GateProcessDeclaration DAG executed

    -- Decision payload
    request_ref_json TEXT NOT NULL,        -- Serialized RequestRef (JSON)
    decision TEXT NOT NULL,                -- "allow" | "decline" | "escalate" | "verdict"
    reasoning_json TEXT NOT NULL,          -- Full ConstitutionalReasoning (JSON)
    context_summary_cid TEXT NOT NULL,     -- CID of privacy-respecting GateContext snapshot
    decided_at TEXT NOT NULL,              -- ISO-8601 timestamp

    -- Universal band provenance
    universal_band_cid TEXT NOT NULL,      -- CID of universal-band DAG declaration

    -- DHT provenance
    dht_anchor_hash TEXT NOT NULL,         -- ActionHash (base64) of the upstream DHT entry

    -- Local bookkeeping
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    PRIMARY KEY (app_id, decision_id)
);

-- Index: look up all decisions for a given elohim agent
CREATE INDEX idx_gate_decisions_elohim ON gate_decision_attestations(elohim_id);

-- Index: look up all decisions produced by a specific gate
CREATE INDEX idx_gate_decisions_gate ON gate_decision_attestations(gate_name);

-- Index: look up decisions by deployment phase
CREATE INDEX idx_gate_decisions_phase ON gate_decision_attestations(phase);

-- Index: look up decisions that ran under a given gate process manifest
CREATE INDEX idx_gate_decisions_manifest ON gate_decision_attestations(gate_process_cid);

-- Index: time-ordered queries (e.g. recent decisions, audit windows)
CREATE INDEX idx_gate_decisions_decided_at ON gate_decision_attestations(decided_at);
