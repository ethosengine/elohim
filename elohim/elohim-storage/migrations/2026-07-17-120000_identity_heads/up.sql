-- Identity-head projection — the read-optimized cache of a `binds-identity`
-- Mishpat::Commitment (Wave B of the identity-head-key-lineage arc; design §2.2/§3).
-- Source of truth: Holochain DHT (mishpat DNA Commitment entry, action='binds-identity');
-- this row is the P1 reconciliation projection, populated from the create_commitment
-- post-commit signal. A NULL dht_anchor_hash means un-notarized/storage-only — the
-- did:elohim head resolver fail-closes on it (dht_anchor_hash IS NOT NULL).
-- Classification A (Notarized). `cid` is the Commitment entry_hash (NEVER action_hash).
--
-- The declaration: "identity chain `chain_root`'s current head is `head_key`;
-- controllers = {set}; controller-policy = self | steward-set | recovery-quorum(M,N)."
-- The chain_root is the stable identity identifier (imagodei genesis-key / CID),
-- unchanged across every key rotation — what other subsystems point at instead of a
-- rotation-fragile raw key. head_key is the current head (the agent_cid the did:elohim
-- resolver keys on). controllers_json is a JSON array of controller id strings;
-- controller_policy_json is the {kind, m?, n?} policy object.
--
-- HEAD SELECTION is DHT-canonical, node-independent: when >1 live+notarized head
-- exists for one head_key (out-of-order signal arrival / projector catch-up / a
-- stale re-projection landing after a legitimate rotation), the resolver picks the
-- newest by `(signed_at DESC, cid ASC)` — NEVER by local arrival (`created_at`,
-- which is `datetime('now')` at the moment THIS node processed the signal and so
-- differs per node). `signed_at` is the notarized caller-supplied signing time
-- carried on the Commitment envelope (deterministic, same on every node); `cid`
-- (the content-addressed entry_hash) is the deterministic tiebreak for a
-- signed_at collision. This is the design's "heads move by judgment over history,
-- never last-writer-wins" rule (design §1, kinship-lineage spec).
-- Spec: genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md.
CREATE TABLE identity_heads (
    cid TEXT PRIMARY KEY,                     -- = Commitment entry_hash (read key + canonical tiebreak)
    chain_root TEXT NOT NULL,                 -- stable identity-chain identifier (durable id)
    head_key TEXT NOT NULL,                   -- current head key of the chain (the agent_cid)
    controllers_json TEXT NOT NULL,           -- JSON array of controller id strings (non-empty)
    controller_policy_json TEXT NOT NULL,     -- JSON policy object {kind, m?, n?}
    signed_at TEXT NOT NULL DEFAULT '',       -- notarized envelope signing time (DHT-canonical head-order key)
    revoked_at TEXT,                          -- lifecycle: non-null excludes from the live head
    dht_anchor_hash TEXT,                     -- Classification A: NULL = un-notarized (fail-closed)
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_identity_heads_head_key ON identity_heads(head_key);
CREATE INDEX idx_identity_heads_chain_root ON identity_heads(chain_root);
