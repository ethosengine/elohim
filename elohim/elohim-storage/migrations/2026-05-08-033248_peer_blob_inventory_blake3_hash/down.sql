DROP INDEX IF EXISTS idx_peer_blob_inventory_blake3;

-- SQLite ALTER TABLE DROP COLUMN was added in 3.35 (2021); diesel uses the
-- bundled rusqlite which targets a sufficient version. If this fails on an
-- older SQLite, the rollback path is a full table rebuild — not worth it
-- since cutover never rolls back a forward-then-soaked migration.
ALTER TABLE peer_blob_inventory DROP COLUMN blake3_hash;
