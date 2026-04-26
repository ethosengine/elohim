-- Source of truth: Holochain DHT (PortalHost entry in imagodei DNA, Category A).
-- This table is a Category A projection rebuildable from signal replay.

CREATE TABLE portal_hosts (
    rowid               INTEGER PRIMARY KEY AUTOINCREMENT,
    human_id            TEXT NOT NULL,           -- mapped from signal's human_action_hash (b64)
    host_url            TEXT NOT NULL,
    label               TEXT,
    added_at            TEXT NOT NULL,
    last_reachable_at   TEXT,                    -- operational; NOT in DHT entry
    reach               TEXT NOT NULL,
    dht_anchor_hash     TEXT NOT NULL UNIQUE
);

CREATE INDEX idx_portal_hosts_human_id ON portal_hosts(human_id);
CREATE INDEX idx_portal_hosts_dht_anchor ON portal_hosts(dht_anchor_hash);
