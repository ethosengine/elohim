-- Add DHT anchor hash to economic tables for cryptographic verification.
-- Records with non-null dht_anchor_hash are notarized on the DHT.
-- Records with null are storage-only (legacy/transitional).
ALTER TABLE economic_events ADD COLUMN dht_anchor_hash TEXT;
ALTER TABLE rea_commitments ADD COLUMN dht_anchor_hash TEXT;
