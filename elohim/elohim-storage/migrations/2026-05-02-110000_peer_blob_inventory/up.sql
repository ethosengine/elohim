-- RECLASSIFICATION NOTE (Observation/Event Layer spec — Stage 8):
-- This table is the SQL projection of the 'infrastructure:blob-served' and
-- 'infrastructure:blob-hosted' observation kinds. The libp2p gossipsub topic
-- 'elohim/inventory/blob' is the legacy name for what is now formally the
-- observation cursor announcement stream for blob-served/blob-hosted.
-- See: genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md §10 Stage 8.
--
-- T12 — peer_blob_inventory: Reality projection of who currently hosts what blob.
--
-- Source of truth: libp2p gossipsub messages on topic 'elohim/inventory/blob'.
-- Category C operational projection rebuildable from gossip replay.
-- Manifest counterpart: rea_commitments(action='custody-blob') (DHT-notarized via T03d).
--
-- Timestamps stored as TEXT (ISO-8601) per elohim-storage conventions.
-- source discriminates evidence quality:
--   'gossip-snapshot' — peer broadcast a full inventory snapshot
--   'gossip-delta'    — peer broadcast a single add (deltas with 'removed' don't write rows; they delete)
--   'fetch-success'   — this peer successfully fetched the blob from the named peer (strongest evidence)
-- sequence is the per-peer monotonic counter from the gossip wire; used for gap-detect at receive time.

CREATE TABLE peer_blob_inventory (
    peer_id      TEXT NOT NULL,
    blob_hash    TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    source       TEXT NOT NULL CHECK (source IN ('gossip-snapshot', 'gossip-delta', 'fetch-success')),
    sequence     INTEGER NOT NULL,
    PRIMARY KEY (peer_id, blob_hash)
);

CREATE INDEX idx_peer_blob_inventory_blob ON peer_blob_inventory(blob_hash);
CREATE INDEX idx_peer_blob_inventory_recent ON peer_blob_inventory(last_seen_at);
