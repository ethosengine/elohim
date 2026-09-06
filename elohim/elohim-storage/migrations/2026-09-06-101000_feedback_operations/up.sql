-- Source of truth: THIS TABLE (Category B — local durable intent, NOT
-- rebuildable from the DHT). Accountable-correction contract §8.
--
-- An operation records what this cell INTENDED to do before the act exists on
-- chain. That is why it cannot be a projection: intent precedes the act. The
-- id is a client UUID, and it is bound on chain through the phase-1 evidence
-- (Content id `correction:<operation_id>` with the request embedded in its
-- body), so a peer with no access to this table still resolves the same group.
--
-- The guarantee is AT MOST ONE CONTRIBUTION PER OPERATION — not at most one
-- action, and explicitly not an exactly-once remote commit protocol.
CREATE TABLE feedback_operations (
    operation_id TEXT NOT NULL PRIMARY KEY,
    -- Pinned at creation. An operation is bound to ONE content space; an
    -- envelope naming a different DNA hash is rejected, never re-homed.
    origin_dna_hash TEXT NOT NULL,
    -- The SUBMITTING CONTENT-CELL AGENT. The zome derives the signer from
    -- `agent_initial_pubkey`, so this — not a human id — is what a phase-2
    -- recovery enumeration matches on. Residual, stated in §8: two humans
    -- behind one storage cell submitting under ONE operation id are conflated
    -- at the cell level; distinct operation ids are not conflated.
    submitting_cell_agent TEXT NOT NULL,
    -- Content-addressed digest of the canonical request bytes. Reuse of an
    -- operation_id with DIFFERENT bytes is refused: the request is immutable.
    request_bytes_cid TEXT NOT NULL,
    -- The canonical request bytes themselves, so a refusal can say precisely
    -- what differed and phase 2 can re-derive its fields after a restart.
    request_bytes TEXT NOT NULL,
    -- Denormalised from request_bytes for execution and for the phase-2
    -- recovery tuple (submitting_cell_agent, target, evidence action, kind).
    target_action_hash TEXT NOT NULL,
    signal_kind TEXT NOT NULL,
    standing_impact TEXT NOT NULL,
    vouch_kind TEXT,
    -- evidence | feedback | done
    -- Phase 1 authors the Correction EPR; phase 2 files the feedback against
    -- the phase-1 action. Each phase has its own uncertainty window.
    phase TEXT NOT NULL,
    -- Phase-1 result: the evidence ACTION hash (what `evidence_cid` on the
    -- entry actually stores — §8 binds to what is stored, not to a content CID).
    evidence_action_hash TEXT,
    -- Phase-2 result.
    feedback_action_hash TEXT,
    -- pending | unresolved | resolved | refused
    --   unresolved = an uncertain call whose recovery enumeration found ZERO
    --                matches. NEVER auto-resubmitted: absence from an
    --                eventually-consistent index does not authorise a second
    --                create. Explicit user resubmission reuses the operation id.
    status TEXT NOT NULL,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_feedback_operations_status ON feedback_operations(status, phase);
CREATE INDEX idx_feedback_operations_evidence ON feedback_operations(evidence_action_hash);
