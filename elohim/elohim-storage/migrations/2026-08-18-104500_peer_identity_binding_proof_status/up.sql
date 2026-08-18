-- Give `peer_identity_bindings` the two columns a cross-signature needs: the
-- proof itself, and this node's fail-closed classification of it.
--
-- Why it exists (habit `identity-cross-signed`): an AgentPeerBinding asserts
-- "agent X owns transport endpoint Y". Today that assertion is SELF-ASSERTED —
-- the gossip publisher writes STAGE1_SIGNATURE_SENTINEL, and the imagodei
-- integrity validator checks only that the signature field is non-empty, which a
-- sentinel satisfies. A gossiped spoof (agent_cid = attacker, transport_id =
-- victim) therefore passes validation, and any join that credits transport
-- activity to an agent through such a binding credits the wrong peer. Worse, the
-- projection did not even RETAIN the signature: the field was dropped at
-- projection time, so no consumer could have verified anything if it wanted to.
--
-- `signature` closes that: the proof travels with the row it is about.
-- `proof_status` is this node's classification of it, written ONLY by the
-- chokepoint `p2p::binding_proof_wire::classify_binding_signature` — the sole
-- function that can produce the 'cross_signed' value (the status type's inner
-- enum is private, so no other writer can mint one by assigning a string).
--
-- DEFAULT 'unverified' NOT NULL is the load-bearing part, in both directions:
-- every existing row backfills to unverified (correct — none of them carries a
-- proof), and every future writer that forgets this column gets the safe value
-- rather than an attributable one. Read gates MUST positive-match
-- (`proof_status = 'cross_signed'`), never `!= 'unverified'`, which would admit
-- any status string a later slice invents before anyone has reasoned about it.
--
-- Source of truth: DHT (imagodei AgentPeerBinding entry). Classification: C
-- (operational projection, rebuildable from signal replay) — so a rebuild
-- re-derives `proof_status` from `signature` through the same chokepoint, and
-- these columns never become a second truth.
--
-- Backlog: genesis/data/timeline/backlog/agent-peer-binding-signing.md (C2-S4)
--          genesis/data/timeline/backlog/agent-peer-binding-cross-signed-proof.md
ALTER TABLE peer_identity_bindings ADD COLUMN signature TEXT NOT NULL DEFAULT '';
ALTER TABLE peer_identity_bindings ADD COLUMN proof_status TEXT NOT NULL DEFAULT 'unverified';

-- Partial index for the attribution cut: the consumer query is always
-- "cross-signed bindings for this agent", and today (and for as long as minting
-- is incomplete) that set is a tiny fraction of the table.
CREATE INDEX IF NOT EXISTS idx_peer_identity_bindings_cross_signed
    ON peer_identity_bindings (agent_cid, observed_at)
    WHERE proof_status = 'cross_signed';
