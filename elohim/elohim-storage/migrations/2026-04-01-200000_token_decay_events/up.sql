CREATE TABLE token_decay_events (
    id TEXT PRIMARY KEY NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    agent_id TEXT NOT NULL,
    governance_layer TEXT NOT NULL,
    balance_before REAL NOT NULL,
    balance_after REAL NOT NULL,
    decay_amount REAL NOT NULL,
    obligation_level TEXT NOT NULL,
    dignity_floor REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_token_decay_agent ON token_decay_events(agent_id);
CREATE INDEX idx_token_decay_h_app ON token_decay_events(h_app_id);
CREATE INDEX idx_token_decay_created ON token_decay_events(created_at);
