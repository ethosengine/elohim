-- Token mint events: immutable record of every token minted
-- Category A (notarized) — every mint is coupled to a witnessed REA event
CREATE TABLE IF NOT EXISTS token_mint_events (
    id TEXT PRIMARY KEY NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    amount REAL NOT NULL,
    provenance_event_id TEXT NOT NULL,
    mint_tier TEXT NOT NULL DEFAULT 'micro',
    source_epr_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    constitutional_context TEXT,
    elohim_attestation TEXT,
    reasoning_trace TEXT,
    dht_anchor_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_token_mint_events_h_app_id ON token_mint_events(h_app_id);
CREATE INDEX IF NOT EXISTS idx_token_mint_events_agent_id ON token_mint_events(agent_id);
CREATE INDEX IF NOT EXISTS idx_token_mint_events_provenance ON token_mint_events(provenance_event_id);
CREATE INDEX IF NOT EXISTS idx_token_mint_events_source_epr ON token_mint_events(source_epr_id);
CREATE INDEX IF NOT EXISTS idx_token_mint_events_tier ON token_mint_events(mint_tier);
CREATE INDEX IF NOT EXISTS idx_token_mint_events_created ON token_mint_events(created_at);

-- Token balances: current holdings per agent per governance layer
-- Category B (agent-scoped)
CREATE TABLE IF NOT EXISTS token_balances (
    agent_id TEXT NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    governance_layer TEXT NOT NULL DEFAULT 'individual',
    balance REAL NOT NULL DEFAULT 0.0,
    total_minted REAL NOT NULL DEFAULT 0.0,
    total_transferred_in REAL NOT NULL DEFAULT 0.0,
    total_transferred_out REAL NOT NULL DEFAULT 0.0,
    last_activity_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent_id, h_app_id, governance_layer)
);

CREATE INDEX IF NOT EXISTS idx_token_balances_h_app_id ON token_balances(h_app_id);
CREATE INDEX IF NOT EXISTS idx_token_balances_balance ON token_balances(balance);

-- Token transfers: witnessed exchanges between agents
-- Category A (notarized)
CREATE TABLE IF NOT EXISTS token_transfers (
    id TEXT PRIMARY KEY NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    from_agent TEXT NOT NULL,
    to_agent TEXT NOT NULL,
    amount REAL NOT NULL,
    governance_layer TEXT NOT NULL DEFAULT 'individual',
    note TEXT,
    dht_anchor_hash TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_token_transfers_h_app_id ON token_transfers(h_app_id);
CREATE INDEX IF NOT EXISTS idx_token_transfers_from ON token_transfers(from_agent);
CREATE INDEX IF NOT EXISTS idx_token_transfers_to ON token_transfers(to_agent);
CREATE INDEX IF NOT EXISTS idx_token_transfers_created ON token_transfers(created_at);
