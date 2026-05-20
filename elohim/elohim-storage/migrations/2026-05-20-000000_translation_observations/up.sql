-- Wave 3 M1 — learning ledger for the valueflows bridge.
-- See genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md §4.2
--
-- Each row is one TranslationPoint observation. End-of-Wave-3 (M5) aggregates
-- these to produce the upstream-contribution inventory + R&O compatibility report.
--
-- Source of truth: local write (Category C operational — produced by the
-- valueflows-bridge at translation call sites; aggregated at M5).
CREATE TABLE translation_observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    observed_at TEXT NOT NULL,                  -- ISO-8601 UTC
    block_height BIGINT,                        -- DHT block height; nullable (None during M1 fixture phase)
    direction TEXT NOT NULL,                    -- 'Read' | 'Write'
    vf_type TEXT NOT NULL,                      -- 'EconomicEvent', 'Proposal', ...
    elohim_source TEXT NOT NULL,                -- 'fixture' | 'hREA::EconomicEvent' | ...
    translation_kind TEXT NOT NULL,             -- 'IdentityShape' | 'FieldRename' | ...
    semantic_cost TEXT NOT NULL,                -- 'Mechanical' | 'JustifiedDistinct' | 'UnclearYet'
    ontological_commitment TEXT,                -- nullable; enum string when set
    client_capability TEXT NOT NULL,            -- 'StockVf' | 'ElohimAware'
    code_location TEXT NOT NULL,                -- file:line
    notes TEXT                                  -- free-form
);

CREATE INDEX idx_translation_observations_observed_at ON translation_observations(observed_at);
CREATE INDEX idx_translation_observations_vf_type ON translation_observations(vf_type);
CREATE INDEX idx_translation_observations_kind_cost ON translation_observations(translation_kind, semantic_cost);
