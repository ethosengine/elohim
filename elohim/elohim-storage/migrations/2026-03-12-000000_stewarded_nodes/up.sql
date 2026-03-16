-- Source of truth: Holochain DHT (StewardedResource entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to StewardedResource ActionHash
-- node_stewardship: Source of truth: SQLite (operational). Classification: C.

-- StewardedNode: a physical node registered on the DHT and projected to storage
CREATE TABLE stewarded_nodes (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    claim_status TEXT NOT NULL DEFAULT 'unclaimed',
    cpu_cores INTEGER NOT NULL DEFAULT 0,
    memory_gb INTEGER NOT NULL DEFAULT 0,
    storage_tb REAL NOT NULL DEFAULT 0.0,
    bandwidth_mbps INTEGER NOT NULL DEFAULT 0,
    steward_tier TEXT NOT NULL DEFAULT 'caretaker',
    custodian_opt_in INTEGER NOT NULL DEFAULT 1,
    region TEXT,
    context_epr_id TEXT,
    dht_anchor_hash TEXT,
    app_id TEXT NOT NULL DEFAULT 'shefa',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_stewarded_nodes_claim_status ON stewarded_nodes(claim_status);
CREATE INDEX idx_stewarded_nodes_app_id ON stewarded_nodes(app_id);

-- NodeStewardship: many-to-many relationship between humans and nodes
CREATE TABLE node_stewardship (
    node_id TEXT NOT NULL REFERENCES stewarded_nodes(id),
    human_id TEXT NOT NULL REFERENCES humans(id),
    affinity_score REAL NOT NULL DEFAULT 0.0,
    relationship TEXT NOT NULL DEFAULT 'primary',
    context_epr_id TEXT,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (node_id, human_id)
);

CREATE INDEX idx_node_stewardship_human ON node_stewardship(human_id);
