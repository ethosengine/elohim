-- SQLite ALTER TABLE DROP COLUMN was added in 3.35 (2021); diesel uses the
-- bundled rusqlite which targets a sufficient version.
ALTER TABLE peer_blob_inventory DROP COLUMN transport_affinity;
