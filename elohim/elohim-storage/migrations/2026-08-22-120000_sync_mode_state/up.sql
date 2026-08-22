-- Sync-mode operational state (unified SyncGate).
--
-- Source of truth: local (operational, Category C). No peer validates this;
-- it is a node-local bandwidth-posture control, like rate-limit counters.
-- Reconstruction on loss: defaults mode='sync', network_class='unknown' —
-- exactly the pre-feature behavior. No dht_anchor_hash by design.
--
-- Design: genesis/data/timeline/backlog/p2p-sync-mode-unified-gate-design.md
-- (p2p-design-gate run 2026-08-22).

-- One-row table: the current operator mode + declared network class.
CREATE TABLE sync_mode_state (
    -- Always 1 — singleton per node.
    id INTEGER PRIMARY KEY CHECK (id = 1),
    -- 'sync' | 'paused' | 'wifi-only'
    mode TEXT NOT NULL DEFAULT 'sync',
    -- 'wifi' | 'cellular' | 'unknown' — declared by the device/steward layer
    -- via the API, never sniffed by the storage node.
    network_class TEXT NOT NULL DEFAULT 'unknown',
    updated_at TEXT NOT NULL
);

-- Bounded transition history for GET /p2p/sync-mode/history.
-- Operator-facing audit log (observability); capped at ~500 rows, oldest
-- pruned on insert. Loss is acceptable — not a protocol lie.
CREATE TABLE sync_mode_transitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- ISO 8601 UTC timestamp of the transition.
    at TEXT NOT NULL,
    from_mode TEXT NOT NULL,
    to_mode TEXT NOT NULL,
    -- 'operator' | 'network-change' | 'internal'
    source TEXT NOT NULL,
    reason TEXT NOT NULL
);
