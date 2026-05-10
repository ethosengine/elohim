-- Extend peer_blob_inventory.source CHECK constraint to include 'self-custody'.
--
-- 'self-custody' records that THIS peer stored the bytes locally via the
-- seeder dual-write path (handle_put_blob). It is local evidence only — not
-- received from gossip, not from a fetch from another peer.
--
-- SQLite does not support ALTER COLUMN or DROP CONSTRAINT, so we recreate
-- the table with the extended CHECK constraint, copy all existing rows, and
-- drop the old table. The blake3_hash column (added in migration
-- 2026-05-08-033248) is preserved.
--
-- Source of truth: BlobStore filesystem scan (Category C operational;
-- rebuildable via backfill-blake3 binary).

PRAGMA foreign_keys = OFF;

CREATE TABLE peer_blob_inventory_new (
    peer_id      TEXT NOT NULL,
    blob_hash    TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    source       TEXT NOT NULL CHECK (source IN ('gossip-snapshot', 'gossip-delta', 'fetch-success', 'self-custody')),
    sequence     INTEGER NOT NULL,
    blake3_hash  TEXT,
    PRIMARY KEY (peer_id, blob_hash)
);

INSERT INTO peer_blob_inventory_new
    SELECT peer_id, blob_hash, last_seen_at, source, sequence, blake3_hash
    FROM peer_blob_inventory;

DROP TABLE peer_blob_inventory;

ALTER TABLE peer_blob_inventory_new RENAME TO peer_blob_inventory;

CREATE INDEX IF NOT EXISTS idx_peer_blob_inventory_blob   ON peer_blob_inventory(blob_hash);
CREATE INDEX IF NOT EXISTS idx_peer_blob_inventory_recent ON peer_blob_inventory(last_seen_at);
CREATE INDEX IF NOT EXISTS idx_peer_blob_inventory_blake3 ON peer_blob_inventory(blake3_hash) WHERE blake3_hash IS NOT NULL;

PRAGMA foreign_keys = ON;
