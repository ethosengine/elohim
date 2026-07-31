-- Admit 'retired' into the acquisition_pins.status CHECK constraint.
--
-- Independent review found the pin-retirement path (commit 192f77cd0,
-- src/p2p/mod.rs retire_exhausted_pins / readmit_cooled_down_pins /
-- readmit_pins_named_by_inventory, acquisition::PIN_STATUS_RETIRED) inert:
-- the original CHECK (status IN ('active', 'paused', 'removed')) rejects
-- 'retired' at the SQLite engine, so every set_pin_status(..., "retired")
-- call fails with a CheckViolation that was being swallowed at debug! level.
--
-- SQLite cannot ALTER a CHECK constraint, so this is the table-rebuild
-- pattern (same shape used by the 2026-06-10-000000_acquisition_pin_commitment_cid
-- down.sql): reproduce the CURRENT full shape of acquisition_pins — the
-- original 2026-06-07-000000_acquisition_pins columns plus the commitment_cid
-- column added by 2026-06-10-000000_acquisition_pin_commitment_cid — changing
-- only the status CHECK to admit 'retired'.
CREATE TABLE acquisition_pins_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_pub_key TEXT NOT NULL DEFAULT 'local-device',
    head_ref TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'item' CHECK (kind IN ('item', 'cluster')),
    closure_rule_json TEXT,
    priority INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'removed', 'retired')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    commitment_cid TEXT,
    UNIQUE (agent_pub_key, head_ref, kind)
);
INSERT INTO acquisition_pins_new
    (id, agent_pub_key, head_ref, kind, closure_rule_json, priority, status, created_at, updated_at, commitment_cid)
    SELECT id, agent_pub_key, head_ref, kind, closure_rule_json, priority, status, created_at, updated_at, commitment_cid
    FROM acquisition_pins;
DROP TABLE acquisition_pins;
ALTER TABLE acquisition_pins_new RENAME TO acquisition_pins;
CREATE INDEX idx_acquisition_pins_status ON acquisition_pins(status);
