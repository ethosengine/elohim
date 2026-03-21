-- Source of truth: DHT (notarized, Category A). Has dht_anchor_hash (added by later migration).
-- Human identity — if centralized, someone becomes the identity provider.
CREATE TABLE humans (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'imagodei',
    agent_pub_key TEXT,
    display_name TEXT NOT NULL,
    bio TEXT,
    affinities TEXT NOT NULL DEFAULT '[]',
    profile_reach TEXT NOT NULL DEFAULT 'public',
    location TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_humans_app_id ON humans(app_id);
CREATE INDEX idx_humans_agent_pub_key ON humans(agent_pub_key);
CREATE INDEX idx_humans_display_name ON humans(display_name);
CREATE INDEX idx_humans_profile_reach ON humans(profile_reach);
