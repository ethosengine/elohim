-- Source of truth: gossip claims (custody announcements, salvage ads, inventory
-- envelopes) whose signature verified under their author's own transport key but
-- whose author no cross-signed chain binds to an agent YET (its AgentPeerBinding not
-- yet minted or received, or its transport link not yet received). Category C,
-- local, operational: bounded per topic, expired after a TTL, never projected and
-- never counted. When a binding or link for the author is recorded, each held claim
-- is replayed through full admission and deleted; one that still fails is dropped.
-- Forged or badly signed claims are refused and never land here.
CREATE TABLE held_unbound_claims (
    id               INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    topic            TEXT NOT NULL,
    author_peer      TEXT NOT NULL,
    -- 'libp2p' or 'iroh': the plane it arrived on, so replay re-runs the same witness.
    plane            TEXT NOT NULL,
    -- gossipsub `message.source` when it arrived on libp2p (NULL otherwise).
    gossipsub_source TEXT,
    bytes            BLOB NOT NULL,
    held_at_ms       BIGINT NOT NULL
);
CREATE INDEX idx_held_unbound_claims_author ON held_unbound_claims(author_peer);
CREATE INDEX idx_held_unbound_claims_topic ON held_unbound_claims(topic, id);
