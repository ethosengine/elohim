-- Reverse of squashed initial migration — drops all tables in reverse dependency order.

-- Observation sessions (no dependents)
DROP TABLE IF EXISTS observation_entries;
DROP TABLE IF EXISTS observation_sessions;

-- Shard storage (no dependents)
DROP TABLE IF EXISTS shard_locations;
DROP TABLE IF EXISTS shard_manifests;

-- Geospatial (no dependents)
DROP TABLE IF EXISTS risk_alerts;
DROP TABLE IF EXISTS hazards;
DROP TABLE IF EXISTS spatial_contexts;
DROP TABLE IF EXISTS places;

-- Infrastructure
DROP TABLE IF EXISTS device_policies;
DROP TABLE IF EXISTS enum_registry;

-- Governance pillar
DROP TABLE IF EXISTS governance_dispositions;
DROP TABLE IF EXISTS comments;
DROP TABLE IF EXISTS discussions;
DROP TABLE IF EXISTS precedents;
DROP TABLE IF EXISTS statement_votes;
DROP TABLE IF EXISTS statements;
DROP TABLE IF EXISTS appeals;
DROP TABLE IF EXISTS challenges;
DROP TABLE IF EXISTS governance_signals;
DROP TABLE IF EXISTS ranked_votes;
DROP TABLE IF EXISTS votes;
DROP TABLE IF EXISTS governance_states;
DROP TABLE IF EXISTS proposal_options;
DROP TABLE IF EXISTS proposals;
DROP TABLE IF EXISTS collective_participations;
DROP TABLE IF EXISTS collectives;

-- Economy pillar
DROP TABLE IF EXISTS token_decay_events;
DROP TABLE IF EXISTS responsibility_demand_configs;
DROP TABLE IF EXISTS token_transfers;
DROP TABLE IF EXISTS token_balances;
DROP TABLE IF EXISTS token_mint_events;
DROP TABLE IF EXISTS custodian_metrics;
DROP TABLE IF EXISTS node_stewardship;
DROP TABLE IF EXISTS stewarded_nodes;
DROP TABLE IF EXISTS access_grants;
DROP TABLE IF EXISTS premium_gates;
DROP TABLE IF EXISTS steward_credentials;
DROP TABLE IF EXISTS stewardship_allocations;
DROP TABLE IF EXISTS agreements;
DROP TABLE IF EXISTS rea_commitments;
DROP TABLE IF EXISTS economic_events;

-- Identity pillar
DROP TABLE IF EXISTS local_sessions;
DROP TABLE IF EXISTS imagodei_observations;
DROP TABLE IF EXISTS contributor_dashboards;
DROP TABLE IF EXISTS contributor_presences;
DROP TABLE IF EXISTS human_relationships;
DROP TABLE IF EXISTS humans;

-- Content pillar
DROP TABLE IF EXISTS schedules;
DROP TABLE IF EXISTS steward_affinity;
DROP TABLE IF EXISTS knowledge_maps;
DROP TABLE IF EXISTS content_mastery;
DROP TABLE IF EXISTS relationships;
DROP TABLE IF EXISTS content_attestations;
DROP TABLE IF EXISTS content_tags;
DROP TABLE IF EXISTS content;

-- App registry and schema version
DROP TABLE IF EXISTS apps;
DROP TABLE IF EXISTS schema_version;
