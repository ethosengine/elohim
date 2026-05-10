-- Revert: restore the original CHECK constraint (without 'self-custody').
-- Any 'self-custody' rows are deleted before reverting.

PRAGMA foreign_keys = OFF;

DELETE FROM peer_blob_inventory WHERE source = 'self-custody';

CREATE TABLE peer_blob_inventory_old (
    peer_id      TEXT NOT NULL,
    blob_hash    TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    source       TEXT NOT NULL CHECK (source IN ('gossip-snapshot', 'gossip-delta', 'fetch-success')),
    sequence     INTEGER NOT NULL,
    blake3_hash  TEXT,
    PRIMARY KEY (peer_id, blob_hash)
);

INSERT INTO peer_blob_inventory_old
    SELECT peer_id, blob_hash, last_seen_at, source, sequence, blake3_hash
    FROM peer_blob_inventory;

DROP TABLE peer_blob_inventory;

ALTER TABLE peer_blob_inventory_old RENAME TO peer_blob_inventory;

CREATE INDEX IF NOT EXISTS idx_peer_blob_inventory_blob   ON peer_blob_inventory(blob_hash);
CREATE INDEX IF NOT EXISTS idx_peer_blob_inventory_recent ON peer_blob_inventory(last_seen_at);
CREATE INDEX IF NOT EXISTS idx_peer_blob_inventory_blake3 ON peer_blob_inventory(blake3_hash) WHERE blake3_hash IS NOT NULL;

PRAGMA foreign_keys = ON;
