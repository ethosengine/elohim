-- Reverse of the witness column: the projection forgets which witness admitted a row.
ALTER TABLE peer_blob_inventory DROP COLUMN witness;
