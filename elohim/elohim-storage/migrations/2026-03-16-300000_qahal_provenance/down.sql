-- Reverse P2P Coherence Sprint 4: Qahal (Governance) Provenance
-- SQLite does not support DROP COLUMN directly in older versions,
-- but Diesel migrations use SQLite 3.35+ which supports it.

ALTER TABLE proposals DROP COLUMN dht_anchor_hash;
ALTER TABLE proposal_options DROP COLUMN dht_anchor_hash;
ALTER TABLE votes DROP COLUMN dht_anchor_hash;
ALTER TABLE ranked_votes DROP COLUMN dht_anchor_hash;
ALTER TABLE governance_signals DROP COLUMN dht_anchor_hash;
ALTER TABLE governance_states DROP COLUMN dht_anchor_hash;
ALTER TABLE challenges DROP COLUMN dht_anchor_hash;
ALTER TABLE appeals DROP COLUMN dht_anchor_hash;
ALTER TABLE statements DROP COLUMN dht_anchor_hash;
ALTER TABLE statement_votes DROP COLUMN dht_anchor_hash;
ALTER TABLE precedents DROP COLUMN dht_anchor_hash;
