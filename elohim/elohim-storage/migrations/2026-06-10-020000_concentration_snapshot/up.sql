-- Per-layer concentration snapshot — the governor's measured state (spec §11 land 3).
-- Source of truth: NONE — Classification C (Operational): recomputed-on-read
-- aggregate, rebuildable by event replay. DELIBERATELY NOT DHT-ANCHORED (no
-- dht_anchor_hash by design): anchoring would notarize what must stay
-- operational — spec §4.4's A/C seam invariant ("computed edges/values are
-- never given a dht_anchor_hash"). CARRIES NO PER-AGENT IDENTITY (k>=5
-- firewall: writer refuses n<5).
-- v1 computes over token balances (spec §11.3); generalizes per-substrate once
-- EconomicEvent.substrate_signal coverage widens.
CREATE TABLE concentration_snapshots (
    id TEXT PRIMARY KEY,                -- slug: {substrate_signal}:{governance_layer}:{computed_at}
    h_app_id TEXT NOT NULL DEFAULT 'shefa',
    substrate_signal TEXT NOT NULL DEFAULT 'attention',
    governance_layer TEXT NOT NULL,
    n INTEGER NOT NULL,                 -- population size (>=5 enforced by writer)
    mu REAL NOT NULL,                   -- distribution mean (b_hat denominator)
    ge REAL NOT NULL,                   -- raw GE(alpha)
    ge_squashed REAL NOT NULL,          -- squash(GE) = GE/(1+GE)
    top_share REAL NOT NULL,            -- S_q
    gini REAL NOT NULL,                 -- diagnostic only, never a driver
    c_composite REAL NOT NULL,          -- w_e*ge_squashed + w_s*top_share
    alpha REAL NOT NULL,
    top_q REAL NOT NULL,                -- renamed from q: Diesel reserves 'q' as a macro identifier
    computed_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_concentration_snapshots_lookup
    ON concentration_snapshots(h_app_id, substrate_signal, governance_layer, computed_at);
