-- Reverse: rebuild acquisition_pins back to the CHECK that excludes 'retired'.
--
-- Explicit downgrade policy: any row currently holding status='retired' maps
-- to 'active' on the way down (not 'removed' — retirement is a substrate HOLD
-- on a pin the person still wants, so downgrading it to the person's
-- revocation status would misrepresent intent; 'active' is the closest
-- pre-'retired' meaning: a pin still wanted and eligible for reconcile).
CREATE TABLE acquisition_pins_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_pub_key TEXT NOT NULL DEFAULT 'local-device',
    head_ref TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'item' CHECK (kind IN ('item', 'cluster')),
    closure_rule_json TEXT,
    priority INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'removed')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    commitment_cid TEXT,
    UNIQUE (agent_pub_key, head_ref, kind)
);
INSERT INTO acquisition_pins_old
    (id, agent_pub_key, head_ref, kind, closure_rule_json, priority, status, created_at, updated_at, commitment_cid)
    SELECT id, agent_pub_key, head_ref, kind, closure_rule_json, priority,
           CASE WHEN status = 'retired' THEN 'active' ELSE status END,
           created_at, updated_at, commitment_cid
    FROM acquisition_pins;
DROP TABLE acquisition_pins;
ALTER TABLE acquisition_pins_old RENAME TO acquisition_pins;
CREATE INDEX idx_acquisition_pins_status ON acquisition_pins(status);
