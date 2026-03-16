-- Collectives: governance contexts with graduated participation
-- Unifies communities and organizations under a single model.
--
-- collectives: Source of truth: DHT (Collective entry in imagodei DNA).
-- Classification: A (Notarized) — collectives are community-visible governance contexts.
-- Note: dht_anchor_hash not yet added (pre-P2P-coherence table; tracked as gap).
--
-- collective_participations: Source of truth: DHT (derived from Collective via Link).
-- Classification: A2 (Derived) — participation is a witnessed act, linked to Collective ActionHash.
-- Note: dht_anchor_hash not yet added (pre-P2P-coherence table; tracked as gap).

CREATE TABLE collectives (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'qahal',
    name TEXT NOT NULL,
    description TEXT,
    governance_layer TEXT NOT NULL DEFAULT 'community',
    constitutional_parent_id TEXT,
    reach TEXT NOT NULL DEFAULT 'community',
    metadata_json TEXT,
    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    dissolved_at TEXT
);

CREATE TABLE collective_participations (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'qahal',
    collective_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    intimacy_level TEXT NOT NULL DEFAULT 'recognition',
    role_context TEXT,
    governance_weight REAL NOT NULL DEFAULT 1.0,
    consent_state TEXT NOT NULL DEFAULT 'pending',
    metadata_json TEXT,
    joined_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    departed_at TEXT,

    FOREIGN KEY (collective_id) REFERENCES collectives(id),
    UNIQUE(app_id, collective_id, human_id)
);

CREATE INDEX idx_collectives_app ON collectives(app_id);
CREATE INDEX idx_collectives_layer ON collectives(governance_layer);
CREATE INDEX idx_participations_app ON collective_participations(app_id);
CREATE INDEX idx_participations_collective ON collective_participations(app_id, collective_id);
CREATE INDEX idx_participations_human ON collective_participations(app_id, human_id);
