-- Phase 4 of iroh parallel stack — peer_blob_inventory.blake3_hash column.
--
-- Source of truth: iroh-gossip messages on the iroh-side equivalent of the
-- libp2p `elohim/inventory/blob` topic (mapped via BLAKE3(name)[..32]).
-- Category C operational projection — same Source-of-Truth as blob_hash.
--
-- During hybrid mode (libp2p + iroh both active), peers may hold the same
-- bytes under both addressing schemes. blob_hash is the SHA256/CIDv1 hash
-- (libp2p side); blake3_hash is the iroh-side native address. Either can
-- be NULL during transition; cutover (Phase 11) drops blob_hash.
--
-- See genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md
-- (Phase 4 — Gossip plane).

ALTER TABLE peer_blob_inventory ADD COLUMN blake3_hash TEXT NULL;

CREATE INDEX idx_peer_blob_inventory_blake3 ON peer_blob_inventory(blake3_hash);
