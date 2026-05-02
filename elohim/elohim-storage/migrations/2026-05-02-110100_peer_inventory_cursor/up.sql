-- T12 — peer_inventory_cursor: per-peer sequence high-watermark.
-- Survives restart so projection writer's gap-detect doesn't false-fire on restart.
-- One row per peer; updated on each successful apply_snapshot or apply_delta.

CREATE TABLE peer_inventory_cursor (
    peer_id        TEXT NOT NULL PRIMARY KEY,
    last_sequence  INTEGER NOT NULL,
    last_updated   TEXT NOT NULL
);
