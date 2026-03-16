-- P2P Coherence Sprint 3: Imagodei (Identity) Provenance
-- Adds dht_anchor_hash to identity pillar tables.
-- After this migration, every human, relationship, presence, and attestation
-- links back to its DHT proof in the imagodei DNA.

-- humans: Source of truth: DHT (Human entry in imagodei DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to Human ActionHash
ALTER TABLE humans ADD COLUMN dht_anchor_hash TEXT;

-- human_relationships: Source of truth: DHT (HumanRelationship entry in imagodei DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to HumanRelationship ActionHash
ALTER TABLE human_relationships ADD COLUMN dht_anchor_hash TEXT;

-- contributor_presences: Source of truth: DHT (ContributorPresence entry in imagodei DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to ContributorPresence ActionHash
ALTER TABLE contributor_presences ADD COLUMN dht_anchor_hash TEXT;

-- path_attestations: Source of truth: DHT (Attestation entry in imagodei DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to Attestation ActionHash
ALTER TABLE path_attestations ADD COLUMN dht_anchor_hash TEXT;
