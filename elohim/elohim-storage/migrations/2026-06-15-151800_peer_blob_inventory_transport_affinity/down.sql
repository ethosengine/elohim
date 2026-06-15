-- SQLite ALTER TABLE DROP COLUMN was added in 3.35 (2021); diesel-sqlite links
-- the bundled libsqlite3-sys (SQLite >= 3.50 in Cargo.lock), so DROP COLUMN is
-- supported.
ALTER TABLE peer_blob_inventory DROP COLUMN transport_affinity;
