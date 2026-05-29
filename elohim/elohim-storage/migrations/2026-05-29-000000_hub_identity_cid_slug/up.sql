-- Source of truth: DHT (Collective/Membership entries). Classification: A2.
-- Wave 2 T1: hub identity schema — canonical CID + slug alias on collectives,
-- member_cid + member_kind + dht_anchor_hash on collective_participations.
-- Existing slug-keyed rows continue to work; new columns are nullable pre-coherence.

ALTER TABLE collectives ADD COLUMN collective_cid TEXT;
ALTER TABLE collectives ADD COLUMN slug TEXT;

ALTER TABLE collective_participations ADD COLUMN member_cid TEXT;
ALTER TABLE collective_participations ADD COLUMN member_kind TEXT NOT NULL DEFAULT 'person';
ALTER TABLE collective_participations ADD COLUMN dht_anchor_hash TEXT;

CREATE INDEX idx_collectives_cid ON collectives(collective_cid);
CREATE INDEX idx_collective_participations_member_cid ON collective_participations(member_cid);
