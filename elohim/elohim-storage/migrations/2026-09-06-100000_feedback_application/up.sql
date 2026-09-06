-- Source of truth: the FeedbackSignal subgraph on the DHT (Category C —
-- operational, rebuilt from the DHT per accountable-correction contract §7).
-- Neither table below is authoritative for anything: both are the per-generation
-- record of WHICH acts this peer has already folded into its standing view, so
-- replay after commit is a no-op and a rebuild can be replayed from scratch.
--
-- UNIT OF APPLICATION = THE OPERATION GROUP, NOT THE ACTION (§7 rev 3).
-- A correction's group key is its evidence action hash; an acceptance's group
-- key is the correction group it targets. Keying on the action instead would
-- mean a late-discovered same-operation act with a lower hash changes the
-- representative and, with it, an already-committed contribution — the
-- convergence hole the rev-2 review named. Keying on the group makes a second
-- act a MEMBER: the contribution is unchanged, and a peer with no access to the
-- origin outbox converges to the same aggregate.
CREATE TABLE feedback_application (
    generation_id INTEGER NOT NULL,
    -- Evidence action hash for a correction group. Stable across members.
    group_key TEXT NOT NULL,
    -- pending | applied | rejected
    --   pending  = a dependency (record, target, root lineage) is unfetchable.
    --              Retryable. NEVER shown as accepted.
    --   applied  = the contribution is committed to this generation's aggregate.
    --   rejected = a POSITIVE mismatch (wrong author, wrong type, request
    --              mismatch). Not a network failure; not retried.
    status TEXT NOT NULL,
    -- The debit weight this group contributed, under the generation's PINNED
    -- policy bytes. A correction alone contributes 0 (an allegation debits
    -- nobody); only an accepted correction moves the aggregate.
    contribution INTEGER NOT NULL DEFAULT 0,
    -- Subject = the TARGET's exact root author (never the signal's signer).
    -- 32-byte normalised ed25519 key; NULL while the group is pending.
    subject_pubkey BLOB,
    -- Max action timestamp among INCLUDED members (micros). `last_signal_at` on
    -- the aggregate is the maximum, never the last enumerated.
    max_included_at_micros BIGINT,
    accepted INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_retry_at TEXT,
    applied_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (generation_id, group_key)
);

CREATE INDEX idx_feedback_application_status
    ON feedback_application(generation_id, status, next_retry_at);

-- Source of truth: the FeedbackSignal subgraph on the DHT (Category C).
-- One row per DISCOVERED ACT. Identity is (origin DNA hash, action hash) — the
-- signed act, not its entry hash: two authors' identical corrections share an
-- entry hash and are two acts (§1, a labelled departure from cid = entry_hash).
CREATE TABLE feedback_application_member (
    generation_id INTEGER NOT NULL,
    origin_dna_hash TEXT NOT NULL,
    action_hash TEXT NOT NULL,
    -- The group this act belongs to. An act whose fields do not match the
    -- immutable request embedded in its evidence is a NON-MEMBER (§7): it lands
    -- with member_status = 'rejected' and contributes nothing.
    group_key TEXT NOT NULL,
    -- member | rejected | pending
    member_status TEXT NOT NULL,
    -- correction | acceptance
    member_role TEXT NOT NULL,
    -- Author recovered from the SIGNED action, normalised to 32 bytes.
    author_pubkey BLOB,
    action_timestamp_micros BIGINT,
    last_error TEXT,
    discovered_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (generation_id, origin_dna_hash, action_hash)
);

CREATE INDEX idx_feedback_application_member_group
    ON feedback_application_member(generation_id, group_key);
