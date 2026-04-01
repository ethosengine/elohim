-- Rename app_id to h_app_id across all tables
--
-- The column stores the Holochain installed app ID (e.g., "lamad", "imagodei", "shefa"),
-- NOT the HTML5 app identifier (which lives in contentBody). Renaming to h_app_id
-- disambiguates between these two concepts.
--
-- Requires SQLite 3.25.0+ (ALTER TABLE RENAME COLUMN support)

-- Initial tables (2026-01-08)
ALTER TABLE content RENAME COLUMN app_id TO h_app_id;
ALTER TABLE content_tags RENAME COLUMN app_id TO h_app_id;
ALTER TABLE content_attestations RENAME COLUMN app_id TO h_app_id;
ALTER TABLE contributor_dashboards RENAME COLUMN app_id TO h_app_id;
ALTER TABLE apps RENAME COLUMN app_id TO h_app_id;
ALTER TABLE custodian RENAME COLUMN app_id TO h_app_id;
ALTER TABLE schema_version RENAME COLUMN app_id TO h_app_id;

-- Relationships (2026-01-10)
ALTER TABLE relationships RENAME COLUMN app_id TO h_app_id;
ALTER TABLE human_relationships RENAME COLUMN app_id TO h_app_id;
ALTER TABLE contributor_presences RENAME COLUMN app_id TO h_app_id;
ALTER TABLE economic_events RENAME COLUMN app_id TO h_app_id;
ALTER TABLE content_mastery RENAME COLUMN app_id TO h_app_id;
ALTER TABLE stewardship_allocations RENAME COLUMN app_id TO h_app_id;

-- Collectives (2026-03-03)
ALTER TABLE collectives RENAME COLUMN app_id TO h_app_id;
ALTER TABLE collective_participations RENAME COLUMN app_id TO h_app_id;

-- REA + Agreements (2026-03-10)
ALTER TABLE rea_commitments RENAME COLUMN app_id TO h_app_id;
ALTER TABLE agreements RENAME COLUMN app_id TO h_app_id;

-- Humans (2026-03-11)
ALTER TABLE humans RENAME COLUMN app_id TO h_app_id;

-- Stewarded nodes (2026-03-12)
ALTER TABLE stewarded_nodes RENAME COLUMN app_id TO h_app_id;

-- Knowledge maps + extensions (2026-03-13)
ALTER TABLE knowledge_maps RENAME COLUMN app_id TO h_app_id;

-- Steward affinity (2026-03-14)
ALTER TABLE steward_affinity RENAME COLUMN app_id TO h_app_id;

-- Imagodei observations (2026-03-14)
ALTER TABLE imagodei_observations RENAME COLUMN app_id TO h_app_id;

-- Comments (2026-03-15)
ALTER TABLE comments RENAME COLUMN app_id TO h_app_id;

-- Schedules (2026-03-16)
ALTER TABLE schedules RENAME COLUMN app_id TO h_app_id;

-- Spatial contexts (2026-03-19)
ALTER TABLE spatial_contexts RENAME COLUMN app_id TO h_app_id;
ALTER TABLE places RENAME COLUMN app_id TO h_app_id;

-- Spatial context history (2026-03-19)
-- Note: spatial_context_history table may not exist if created by a later migration
-- that already uses h_app_id. Check with: PRAGMA table_info(spatial_context_history);

-- Enum registry (2026-03-20)
ALTER TABLE enum_registry RENAME COLUMN app_id TO h_app_id;

-- Governance tables (signals, statements, votes, etc.)
-- These may have been created with h_app_id if added after the cleanup sprint.
-- SQLite ALTER TABLE RENAME COLUMN is a no-op if column doesn't exist —
-- but it will error. We wrap in a check below for safety.

-- Hazards and risk alerts (2026-03-19)
ALTER TABLE hazards RENAME COLUMN app_id TO h_app_id;
ALTER TABLE risk_alerts RENAME COLUMN app_id TO h_app_id;
