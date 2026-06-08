-- SQLite cannot DROP COLUMN before 3.35; rebuild the table without the column.
CREATE TABLE acquisition_pins_down (
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
INSERT INTO acquisition_pins_down
    (id, agent_pub_key, head_ref, kind, closure_rule_json, priority, status, created_at, updated_at)
    SELECT id, agent_pub_key, head_ref, kind, closure_rule_json, priority, status, created_at, updated_at
    FROM acquisition_pins;
DROP TABLE acquisition_pins;
ALTER TABLE acquisition_pins_down RENAME TO acquisition_pins;
CREATE INDEX idx_acquisition_pins_status ON acquisition_pins(status);
