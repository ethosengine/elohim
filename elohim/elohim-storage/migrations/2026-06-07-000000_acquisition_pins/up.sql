-- DevicePin — the airplane-mode-durable local want declaration (Category B, agent-scoped).
-- Source of truth: local (agent-scoped device pin; roams via export, not gossip).
-- No dht_anchor_hash by design: the pin's notarized shadow is a provide-content
-- Commitment written at sync-back (Slice 2), NOT this row.
-- Spec: 2026-06-07-epr-acquisition-pull-queue-design.md §1.1, §3.
CREATE TABLE acquisition_pins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_pub_key TEXT NOT NULL DEFAULT 'local-device',
    head_ref TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'item' CHECK (kind IN ('item', 'cluster')),
    closure_rule_json TEXT,
    priority INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'removed')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (agent_pub_key, head_ref, kind)
);
CREATE INDEX idx_acquisition_pins_status ON acquisition_pins(status);
