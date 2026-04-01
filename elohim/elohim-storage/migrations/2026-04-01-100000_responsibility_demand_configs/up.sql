CREATE TABLE responsibility_demand_configs (
    id TEXT PRIMARY KEY NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    governance_layer TEXT NOT NULL,
    dignity_floor REAL NOT NULL DEFAULT 100.0,
    median_estimate REAL NOT NULL DEFAULT 1000.0,
    soft_ceiling_multiplier REAL NOT NULL DEFAULT 10.0,
    hard_ceiling_multiplier REAL NOT NULL DEFAULT 20.0,
    social_contract_health REAL NOT NULL DEFAULT 0.5,
    enforcement_active INTEGER NOT NULL DEFAULT 1,
    ratified_by TEXT,
    ratified_at TEXT,
    dht_anchor_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(h_app_id, governance_layer)
);

CREATE INDEX idx_rdc_h_app_id ON responsibility_demand_configs(h_app_id);
CREATE INDEX idx_rdc_governance_layer ON responsibility_demand_configs(governance_layer);
