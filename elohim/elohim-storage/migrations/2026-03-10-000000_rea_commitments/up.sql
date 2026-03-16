-- Source of truth: Holochain DHT (Commitment entry in lamad DNA)
-- Classification: A (Notarized) — dht_anchor_hash links to Commitment ActionHash

-- REA Commitment: a binding promise of future economic activity (ValueFlows)
-- Supports paired give/take actions for bilateral exchange.

CREATE TABLE rea_commitments (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',

    -- REA core: who promises what to whom
    action TEXT NOT NULL,
    provider TEXT NOT NULL,
    receiver TEXT NOT NULL,

    -- Resource specification
    resource_conforms_to TEXT,
    resource_classified_as TEXT,
    resource_quantity_value REAL,
    resource_quantity_unit TEXT,
    effort_quantity_value REAL,
    effort_quantity_unit TEXT,

    -- Timing
    has_beginning TEXT,
    has_end TEXT,
    due TEXT,

    -- Agreements and scoping
    clause_of TEXT,
    in_scope_of TEXT,

    -- Medium of exchange
    medium_of_exchange_id TEXT,

    -- Lifecycle
    state TEXT NOT NULL DEFAULT 'proposed',
    finished INTEGER NOT NULL DEFAULT 0,

    -- Metadata
    note TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_rea_commitment_app_id ON rea_commitments(app_id);
CREATE INDEX idx_rea_commitment_provider ON rea_commitments(app_id, provider);
CREATE INDEX idx_rea_commitment_receiver ON rea_commitments(app_id, receiver);
CREATE INDEX idx_rea_commitment_action ON rea_commitments(action);
CREATE INDEX idx_rea_commitment_state ON rea_commitments(state);
CREATE INDEX idx_rea_commitment_clause_of ON rea_commitments(clause_of);
CREATE INDEX idx_rea_commitment_medium ON rea_commitments(medium_of_exchange_id);
