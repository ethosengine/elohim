-- Local sessions for Tauri native handoff
-- Stores identity info from doorway after OAuth authentication
-- Enables offline access without re-authenticating with doorway

CREATE TABLE local_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    human_id TEXT NOT NULL,
    agent_pub_key TEXT NOT NULL,
    doorway_url TEXT NOT NULL,
    doorway_id TEXT,
    identifier TEXT NOT NULL,
    display_name TEXT,
    profile_image_hash TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_synced_at TEXT,

    -- Bootstrap info for P2P discovery (from handoff response)
    bootstrap_url TEXT,

    UNIQUE(human_id, agent_pub_key)
);

-- Index for finding active session quickly
CREATE INDEX idx_local_sessions_active ON local_sessions(is_active) WHERE is_active = 1;

-- Index for finding sessions by human
CREATE INDEX idx_local_sessions_human ON local_sessions(human_id);

-- Index for finding sessions by doorway
CREATE INDEX idx_local_sessions_doorway ON local_sessions(doorway_url);
