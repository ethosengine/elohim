-- Source of truth: local (operational). Category C.
-- Tracks local Kademlia publish state for EPR Heads.
-- NULL = not yet published to libp2p Kad DHT. Set by the drain loop (p2p module).
-- Distinct from `dht_anchor_hash` which tracks Holochain notarization.
-- Reconstruction strategy: re-publish from the content table (put_record is idempotent);
-- losing this column only costs one extra drain pass.
ALTER TABLE content ADD COLUMN p2p_published_at TEXT;

-- Partial index over unpublished rows — the drain loop scans this frequently.
CREATE INDEX idx_content_p2p_unpublished
    ON content(h_app_id, id)
    WHERE p2p_published_at IS NULL;
