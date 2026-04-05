-- Squashed initial migration — single source of schema truth.
-- All tables that exist in the final schema (diesel_schema.rs) are created here.
-- Dropped tables (paths, steps, chapters, path_tags, path_extensions, path_attestations)
-- are intentionally absent — they were created and then dropped before this squash.
-- h_app_id is used everywhere (never app_id).
--
-- P2P classifications per table:
--   A  = Notarized (DHT is source of truth; this is a read-optimised projection)
--   A2 = Derived (DHT link from parent entry)
--   B  = Agent-scoped (private source chain)
--   B2 = Agent-scoped + Attestation
--   C  = Operational (SQLite-only; reconstructable)

-- ============================================================================
-- Schema Version
-- ============================================================================

-- Source of truth: SQLite (global, no h_app_id). Classification: C.
CREATE TABLE schema_version (
    version INTEGER NOT NULL
);
INSERT INTO schema_version (version) VALUES (2);

-- ============================================================================
-- App Registry
-- ============================================================================

-- Source of truth: SQLite (operational). Classification: C.
CREATE TABLE apps (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    enabled     INTEGER NOT NULL DEFAULT 1
);

-- Default apps: lamad for learning content, elohim for shared infrastructure
INSERT INTO apps (id, name, description) VALUES
    ('lamad',   'Lamad',   'Learning platform content - paths, concepts, quizzes, assessments'),
    ('elohim',  'Elohim',  'Shared infrastructure - resources, sensemaking, attestations, coordination');

-- ============================================================================
-- Content Pillar (Lamad DNA)
-- ============================================================================

-- Source of truth: DHT (Content entry in lamad DNA). Classification: A.
-- dht_anchor_hash links to Content ActionHash.
CREATE TABLE content (
    id                   TEXT PRIMARY KEY NOT NULL,
    h_app_id             TEXT NOT NULL DEFAULT 'lamad',
    title                TEXT NOT NULL,
    description          TEXT,
    content_type         TEXT NOT NULL DEFAULT 'concept',
    content_format       TEXT NOT NULL DEFAULT 'markdown',
    blob_hash            TEXT,
    blob_cid             TEXT,
    content_size_bytes   INTEGER,
    metadata_json        TEXT,
    reach                TEXT NOT NULL DEFAULT 'public',
    validation_status    TEXT NOT NULL DEFAULT 'valid',
    created_by           TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now')),
    content_body         TEXT,
    dht_anchor_hash      TEXT
);

CREATE INDEX idx_content_h_app_id     ON content(h_app_id);
CREATE UNIQUE INDEX idx_content_app_unique ON content(h_app_id, id);
CREATE INDEX idx_content_type         ON content(content_type);
CREATE INDEX idx_content_format       ON content(content_format);
CREATE INDEX idx_content_reach        ON content(reach);
CREATE INDEX idx_content_created_at   ON content(created_at);
CREATE INDEX idx_content_blob_hash    ON content(blob_hash);

-- Source of truth: SQLite (operational). Classification: C. Derived from content metadata.
CREATE TABLE content_tags (
    h_app_id   TEXT NOT NULL DEFAULT 'lamad',
    content_id TEXT NOT NULL,
    tag        TEXT NOT NULL,
    PRIMARY KEY (h_app_id, content_id, tag),
    FOREIGN KEY (content_id) REFERENCES content(id) ON DELETE CASCADE
);

CREATE INDEX idx_content_tags_app_tag ON content_tags(h_app_id, tag);

-- Source of truth: DHT (Attestation entry in imagodei DNA). Classification: A.
-- dht_anchor_hash links to Attestation ActionHash.
CREATE TABLE content_attestations (
    id                   TEXT PRIMARY KEY NOT NULL,
    content_id           TEXT NOT NULL,
    attestor_presence_id TEXT NOT NULL,
    scope                TEXT NOT NULL,
    attestation_type     TEXT NOT NULL,
    evidence             TEXT,
    grantor              TEXT,
    is_revoked           INTEGER NOT NULL DEFAULT 0,
    revocation           TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now')),
    dht_anchor_hash      TEXT
);

CREATE INDEX idx_content_attestations_content  ON content_attestations(content_id);
CREATE INDEX idx_content_attestations_attestor ON content_attestations(attestor_presence_id);
CREATE INDEX idx_content_attestations_type     ON content_attestations(attestation_type);

-- Source of truth: DHT (Relationship entry in lamad DNA). Classification: A.
-- dht_anchor_hash links to Relationship ActionHash.
CREATE TABLE relationships (
    id                    TEXT PRIMARY KEY NOT NULL,
    h_app_id              TEXT NOT NULL DEFAULT 'lamad',
    source_id             TEXT NOT NULL,
    target_id             TEXT NOT NULL,
    relationship_type     TEXT NOT NULL,
    confidence            REAL NOT NULL DEFAULT 1.0,
    inference_source      TEXT NOT NULL DEFAULT 'explicit',
    is_bidirectional      INTEGER NOT NULL DEFAULT 0,
    inverse_relationship_id TEXT,
    provenance_chain_json TEXT,
    governance_layer      TEXT,
    reach                 TEXT NOT NULL DEFAULT 'commons',
    metadata_json         TEXT,
    created_at            TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at            TEXT NOT NULL DEFAULT (datetime('now')),
    dht_anchor_hash       TEXT
);

CREATE UNIQUE INDEX idx_relationships_unique      ON relationships(h_app_id, source_id, target_id, relationship_type);
CREATE INDEX idx_relationships_h_app_id           ON relationships(h_app_id);
CREATE INDEX idx_relationships_source             ON relationships(source_id);
CREATE INDEX idx_relationships_target             ON relationships(target_id);
CREATE INDEX idx_relationships_type               ON relationships(relationship_type);
CREATE INDEX idx_relationships_source_type        ON relationships(source_id, relationship_type);
CREATE INDEX idx_relationships_inverse            ON relationships(inverse_relationship_id);
CREATE INDEX idx_relationships_inference          ON relationships(inference_source);

-- Source of truth: Private source chain (agent-scoped). Classification: B2.
-- dht_anchor_hash populated when mastery crosses threshold and Attestation is issued.
CREATE TABLE content_mastery (
    id                          TEXT PRIMARY KEY NOT NULL,
    h_app_id                    TEXT NOT NULL DEFAULT 'lamad',
    human_id                    TEXT NOT NULL,
    content_id                  TEXT NOT NULL,
    mastery_level               TEXT NOT NULL DEFAULT 'not_started',
    mastery_level_index         INTEGER NOT NULL DEFAULT 0,
    freshness_score             REAL NOT NULL DEFAULT 1.0,
    needs_refresh               INTEGER NOT NULL DEFAULT 0,
    engagement_count            INTEGER NOT NULL DEFAULT 0,
    last_engagement_type        TEXT,
    last_engagement_at          TEXT,
    level_achieved_at           TEXT,
    content_version_at_mastery  TEXT,
    assessment_evidence_json    TEXT,
    privileges_json             TEXT,
    created_at                  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at                  TEXT NOT NULL DEFAULT (datetime('now')),
    dht_anchor_hash             TEXT
);

CREATE UNIQUE INDEX idx_mastery_unique      ON content_mastery(h_app_id, human_id, content_id);
CREATE INDEX idx_mastery_h_app_id          ON content_mastery(h_app_id);
CREATE INDEX idx_mastery_human             ON content_mastery(h_app_id, human_id);
CREATE INDEX idx_mastery_content           ON content_mastery(content_id);
CREATE INDEX idx_mastery_level             ON content_mastery(mastery_level);
CREATE INDEX idx_mastery_needs_refresh     ON content_mastery(needs_refresh) WHERE needs_refresh = 1;
CREATE INDEX idx_mastery_freshness         ON content_mastery(freshness_score);

-- Source of truth: SQLite (operational). Classification: C.
-- Reconstructable from content relationships. Personal sensemaking.
CREATE TABLE knowledge_maps (
    id                TEXT PRIMARY KEY NOT NULL,
    h_app_id          TEXT NOT NULL DEFAULT 'lamad',
    map_type          TEXT NOT NULL,
    owner_id          TEXT NOT NULL,
    title             TEXT NOT NULL,
    description       TEXT,
    subject_type      TEXT NOT NULL,
    subject_id        TEXT NOT NULL,
    subject_name      TEXT NOT NULL,
    visibility        TEXT NOT NULL DEFAULT 'private',
    shared_with_json  TEXT,
    nodes_json        TEXT NOT NULL,
    path_ids_json     TEXT,
    overall_affinity  REAL NOT NULL DEFAULT 0.0,
    content_graph_id  TEXT,
    mastery_levels_json TEXT,
    goals_json        TEXT,
    metadata_json     TEXT,
    created_at        TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_knowledge_maps_h_app_id  ON knowledge_maps(h_app_id);
CREATE INDEX idx_knowledge_maps_owner     ON knowledge_maps(h_app_id, owner_id);
CREATE INDEX idx_knowledge_maps_type      ON knowledge_maps(h_app_id, map_type);
CREATE INDEX idx_knowledge_maps_subject   ON knowledge_maps(h_app_id, subject_id);
CREATE INDEX idx_knowledge_maps_visibility ON knowledge_maps(h_app_id, visibility);

-- Source of truth: SQLite (operational). Classification: C.
-- Reconstructable from economic_events curation acts.
CREATE TABLE steward_affinity (
    id             TEXT PRIMARY KEY NOT NULL,
    h_app_id       TEXT NOT NULL DEFAULT 'lamad',
    steward_id     TEXT NOT NULL,
    content_id     TEXT NOT NULL,
    affinity_score REAL NOT NULL DEFAULT 0.0,
    source         TEXT NOT NULL DEFAULT 'genesis_seed',
    created_at     TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_steward_affinity_unique   ON steward_affinity(h_app_id, steward_id, content_id);
CREATE INDEX idx_steward_affinity_h_app_id        ON steward_affinity(h_app_id);
CREATE INDEX idx_steward_affinity_steward         ON steward_affinity(h_app_id, steward_id);
CREATE INDEX idx_steward_affinity_content         ON steward_affinity(h_app_id, content_id);

-- Source of truth: SQLite (operational). Classification: B.
-- Personal scheduling data (RRULE patterns). Not shared with peers.
CREATE TABLE schedules (
    id                  TEXT PRIMARY KEY NOT NULL,
    h_app_id            TEXT NOT NULL DEFAULT 'lamad',
    entity_type         TEXT NOT NULL,
    entity_id           TEXT NOT NULL,
    scheduled_at        TEXT,
    expires_at          TEXT,
    rrule               TEXT,
    last_occurred_at    TEXT,
    next_occurrence_at  TEXT,
    occurrence_count    INTEGER NOT NULL DEFAULT 0,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    UNIQUE(entity_type, entity_id)
);

CREATE INDEX idx_schedules_entity    ON schedules(entity_type, entity_id);
CREATE INDEX idx_schedules_next      ON schedules(next_occurrence_at);
CREATE INDEX idx_schedules_scheduled ON schedules(scheduled_at);

-- ============================================================================
-- Identity Pillar (Imagodei DNA)
-- ============================================================================

-- Source of truth: DHT (Human entry in imagodei DNA). Classification: A.
-- dht_anchor_hash links to Human ActionHash.
CREATE TABLE humans (
    id                TEXT PRIMARY KEY NOT NULL,
    agent_pub_key     TEXT,
    display_name      TEXT NOT NULL,
    bio               TEXT,
    affinities        TEXT NOT NULL DEFAULT '[]',
    profile_reach     TEXT NOT NULL DEFAULT 'public',
    location          TEXT,
    profile_photo_url TEXT,
    h_app_id          TEXT NOT NULL DEFAULT 'imagodei',
    created_at        TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now')),
    dht_anchor_hash   TEXT
);

CREATE INDEX idx_humans_h_app_id      ON humans(h_app_id);
CREATE INDEX idx_humans_agent_pub_key ON humans(agent_pub_key);
CREATE INDEX idx_humans_display_name  ON humans(display_name);
CREATE INDEX idx_humans_profile_reach ON humans(profile_reach);

-- Source of truth: DHT (HumanRelationship entry in imagodei DNA). Classification: A.
-- dht_anchor_hash links to HumanRelationship ActionHash.
CREATE TABLE human_relationships (
    id                       TEXT PRIMARY KEY NOT NULL,
    h_app_id                 TEXT NOT NULL DEFAULT 'imagodei',
    party_a_id               TEXT NOT NULL,
    party_b_id               TEXT NOT NULL,
    relationship_type        TEXT NOT NULL,
    intimacy_level           TEXT NOT NULL DEFAULT 'recognition',
    is_bidirectional         INTEGER NOT NULL DEFAULT 0,
    consent_given_by_a       INTEGER NOT NULL DEFAULT 0,
    consent_given_by_b       INTEGER NOT NULL DEFAULT 0,
    custody_enabled_by_a     INTEGER NOT NULL DEFAULT 0,
    custody_enabled_by_b     INTEGER NOT NULL DEFAULT 0,
    auto_custody_enabled     INTEGER NOT NULL DEFAULT 0,
    emergency_access_enabled INTEGER NOT NULL DEFAULT 0,
    initiated_by             TEXT NOT NULL,
    verified_at              TEXT,
    governance_layer         TEXT,
    reach                    TEXT NOT NULL DEFAULT 'private',
    context_json             TEXT,
    created_at               TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at               TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at               TEXT,
    dht_anchor_hash          TEXT
);

CREATE UNIQUE INDEX idx_human_rel_unique   ON human_relationships(h_app_id, party_a_id, party_b_id, relationship_type);
CREATE INDEX idx_human_rel_h_app_id        ON human_relationships(h_app_id);
CREATE INDEX idx_human_rel_party_a         ON human_relationships(h_app_id, party_a_id);
CREATE INDEX idx_human_rel_party_b         ON human_relationships(h_app_id, party_b_id);
CREATE INDEX idx_human_rel_type            ON human_relationships(relationship_type);
CREATE INDEX idx_human_rel_intimacy        ON human_relationships(intimacy_level);
CREATE INDEX idx_human_rel_custody         ON human_relationships(auto_custody_enabled) WHERE auto_custody_enabled = 1;

-- Source of truth: DHT (ContributorPresence entry in imagodei DNA). Classification: A.
-- dht_anchor_hash links to ContributorPresence ActionHash.
CREATE TABLE contributor_presences (
    id                                  TEXT PRIMARY KEY NOT NULL,
    h_app_id                            TEXT NOT NULL DEFAULT 'lamad',
    display_name                        TEXT NOT NULL,
    presence_state                      TEXT NOT NULL DEFAULT 'unclaimed',
    external_identifiers_json           TEXT,
    establishing_content_ids_json       TEXT NOT NULL,
    affinity_total                      REAL NOT NULL DEFAULT 0.0,
    unique_engagers                     INTEGER NOT NULL DEFAULT 0,
    citation_count                      INTEGER NOT NULL DEFAULT 0,
    recognition_score                   REAL NOT NULL DEFAULT 0.0,
    recognition_by_content_json         TEXT,
    last_recognition_at                 TEXT,
    steward_id                          TEXT,
    stewardship_started_at              TEXT,
    stewardship_commitment_id           TEXT,
    stewardship_quality_score           REAL,
    claim_initiated_at                  TEXT,
    claim_verified_at                   TEXT,
    claim_verification_method           TEXT,
    claim_evidence_json                 TEXT,
    claimed_agent_id                    TEXT,
    claim_recognition_transferred_value REAL,
    claim_facilitated_by                TEXT,
    image                               TEXT,
    note                                TEXT,
    metadata_json                       TEXT,
    created_at                          TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at                          TEXT NOT NULL DEFAULT (datetime('now')),
    dht_anchor_hash                     TEXT
);

CREATE INDEX idx_presence_h_app_id    ON contributor_presences(h_app_id);
CREATE INDEX idx_presence_state       ON contributor_presences(h_app_id, presence_state);
CREATE INDEX idx_presence_steward     ON contributor_presences(steward_id);
CREATE INDEX idx_presence_claimed     ON contributor_presences(claimed_agent_id);
CREATE INDEX idx_presence_recognition ON contributor_presences(recognition_score DESC);

-- Source of truth: SQLite (operational). Classification: C.
-- Computed metrics snapshot, one row per contributor (upsert on report).
CREATE TABLE contributor_dashboards (
    presence_id           TEXT PRIMARY KEY NOT NULL,
    total_contributions   INTEGER NOT NULL DEFAULT 0,
    total_recognitions    INTEGER NOT NULL DEFAULT 0,
    impact_score          REAL NOT NULL DEFAULT 0.0,
    last_contribution_at  TEXT,
    updated_at            TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Source of truth: Private source chain (agent-scoped in imagodei DNA). Classification: B2.
-- dht_anchor_hash populated when trust attestation is issued.
CREATE TABLE imagodei_observations (
    id                      TEXT PRIMARY KEY NOT NULL,
    h_app_id                TEXT NOT NULL,
    human_id                TEXT NOT NULL,
    observed_at             TEXT NOT NULL,
    observation_type        TEXT NOT NULL,
    content                 TEXT NOT NULL,
    structured_signals_json TEXT,
    trust_delta             REAL NOT NULL DEFAULT 0.0,
    visibility_layer        TEXT NOT NULL DEFAULT 'individual',
    originating_elohim      TEXT NOT NULL,
    relevance_decay         REAL NOT NULL DEFAULT 0.0,
    superseded_by           TEXT,
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),
    dht_anchor_hash         TEXT
);

CREATE INDEX idx_imagodei_obs_human ON imagodei_observations(human_id);
CREATE INDEX idx_imagodei_obs_type  ON imagodei_observations(observation_type);

-- Source of truth: SQLite (device-local, ephemeral). Classification: C.
-- Reconstructable from identity handoff.
CREATE TABLE local_sessions (
    id                  TEXT PRIMARY KEY NOT NULL,
    human_id            TEXT NOT NULL,
    agent_pub_key       TEXT NOT NULL,
    doorway_url         TEXT NOT NULL,
    doorway_id          TEXT,
    identifier          TEXT NOT NULL,
    display_name        TEXT,
    profile_image_hash  TEXT,
    is_active           INTEGER NOT NULL DEFAULT 1,
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now')),
    last_synced_at      TEXT,
    bootstrap_url       TEXT,
    session_intent_json TEXT,
    intent_set_at       TEXT,
    UNIQUE(human_id, agent_pub_key)
);

CREATE INDEX idx_local_sessions_active  ON local_sessions(is_active) WHERE is_active = 1;
CREATE INDEX idx_local_sessions_human   ON local_sessions(human_id);
CREATE INDEX idx_local_sessions_doorway ON local_sessions(doorway_url);

-- ============================================================================
-- Economy Pillar (Shefa / ValueFlows)
-- ============================================================================

-- Source of truth: DHT (EconomicEvent entry in lamad DNA). Classification: A.
-- dht_anchor_hash links to EconomicEvent ActionHash.
CREATE TABLE economic_events (
    id                          TEXT PRIMARY KEY NOT NULL,
    h_app_id                    TEXT NOT NULL DEFAULT 'shefa',
    action                      TEXT NOT NULL,
    provider                    TEXT NOT NULL,
    receiver                    TEXT NOT NULL,
    resource_conforms_to        TEXT,
    resource_inventoried_as     TEXT,
    resource_classified_as_json TEXT,
    resource_quantity_value     REAL,
    resource_quantity_unit      TEXT,
    effort_quantity_value       REAL,
    effort_quantity_unit        TEXT,
    has_point_in_time           TEXT NOT NULL,
    has_duration                TEXT,
    input_of                    TEXT,
    output_of                   TEXT,
    lamad_event_type            TEXT,
    content_id                  TEXT,
    contributor_presence_id     TEXT,
    path_id                     TEXT,
    triggered_by                TEXT,
    state                       TEXT NOT NULL DEFAULT 'recorded',
    note                        TEXT,
    metadata_json               TEXT,
    dht_anchor_hash             TEXT,
    created_at                  TEXT NOT NULL DEFAULT (datetime('now')),
    at_location                 TEXT
);

CREATE INDEX idx_event_h_app_id   ON economic_events(h_app_id);
CREATE INDEX idx_event_provider   ON economic_events(h_app_id, provider);
CREATE INDEX idx_event_receiver   ON economic_events(h_app_id, receiver);
CREATE INDEX idx_event_action     ON economic_events(action);
CREATE INDEX idx_event_lamad_type ON economic_events(lamad_event_type);
CREATE INDEX idx_event_content    ON economic_events(content_id);
CREATE INDEX idx_event_presence   ON economic_events(contributor_presence_id);
CREATE INDEX idx_event_path       ON economic_events(path_id);
CREATE INDEX idx_event_time       ON economic_events(has_point_in_time);
CREATE INDEX idx_event_state      ON economic_events(state);
CREATE INDEX idx_econ_events_location ON economic_events(at_location);

-- Source of truth: DHT (Commitment entry in lamad DNA). Classification: A.
-- dht_anchor_hash links to Commitment ActionHash.
CREATE TABLE rea_commitments (
    id                       TEXT PRIMARY KEY NOT NULL,
    h_app_id                 TEXT NOT NULL DEFAULT 'lamad',
    action                   TEXT NOT NULL,
    provider                 TEXT NOT NULL,
    receiver                 TEXT NOT NULL,
    resource_conforms_to     TEXT,
    resource_classified_as   TEXT,
    resource_quantity_value  REAL,
    resource_quantity_unit   TEXT,
    effort_quantity_value    REAL,
    effort_quantity_unit     TEXT,
    has_beginning            TEXT,
    has_end                  TEXT,
    due                      TEXT,
    clause_of                TEXT,
    in_scope_of              TEXT,
    medium_of_exchange_id    TEXT,
    state                    TEXT NOT NULL DEFAULT 'proposed',
    finished                 INTEGER NOT NULL DEFAULT 0,
    note                     TEXT,
    metadata_json            TEXT,
    dht_anchor_hash          TEXT,
    created_at               TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_rea_commitment_h_app_id  ON rea_commitments(h_app_id);
CREATE INDEX idx_rea_commitment_provider  ON rea_commitments(h_app_id, provider);
CREATE INDEX idx_rea_commitment_receiver  ON rea_commitments(h_app_id, receiver);
CREATE INDEX idx_rea_commitment_action    ON rea_commitments(action);
CREATE INDEX idx_rea_commitment_state     ON rea_commitments(state);
CREATE INDEX idx_rea_commitment_clause_of ON rea_commitments(clause_of);
CREATE INDEX idx_rea_commitment_medium    ON rea_commitments(medium_of_exchange_id);

-- Source of truth: DHT (Agreement entry in lamad DNA). Classification: A.
-- dht_anchor_hash links to Agreement ActionHash.
CREATE TABLE agreements (
    id              TEXT PRIMARY KEY NOT NULL,
    h_app_id        TEXT NOT NULL DEFAULT 'lamad',
    name            TEXT,
    note            TEXT,
    dht_anchor_hash TEXT,
    metadata_json   TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_agreement_h_app_id ON agreements(h_app_id);

-- Source of truth: DHT (derived from Agreement via Link). Classification: A2.
-- dht_anchor_hash links to parent Agreement's ActionHash.
CREATE TABLE stewardship_allocations (
    id                          TEXT PRIMARY KEY NOT NULL,
    h_app_id                    TEXT NOT NULL DEFAULT 'lamad',
    content_id                  TEXT NOT NULL,
    steward_presence_id         TEXT NOT NULL,
    allocation_ratio            REAL NOT NULL DEFAULT 1.0,
    allocation_method           TEXT NOT NULL DEFAULT 'manual',
    contribution_type           TEXT NOT NULL DEFAULT 'original_creator',
    contribution_evidence_json  TEXT,
    governance_state            TEXT NOT NULL DEFAULT 'active',
    dispute_id                  TEXT,
    dispute_reason              TEXT,
    disputed_at                 TEXT,
    disputed_by                 TEXT,
    negotiation_session_id      TEXT,
    elohim_ratified_at          TEXT,
    elohim_ratifier_id          TEXT,
    effective_from              TEXT NOT NULL DEFAULT (datetime('now')),
    effective_until             TEXT,
    superseded_by               TEXT,
    recognition_accumulated     REAL NOT NULL DEFAULT 0.0,
    last_recognition_at         TEXT,
    note                        TEXT,
    metadata_json               TEXT,
    dht_anchor_hash             TEXT,
    created_at                  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at                  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_alloc_h_app_id   ON stewardship_allocations(h_app_id);
CREATE INDEX idx_alloc_content    ON stewardship_allocations(content_id);
CREATE INDEX idx_alloc_steward    ON stewardship_allocations(steward_presence_id);
CREATE INDEX idx_alloc_governance ON stewardship_allocations(governance_state);
CREATE INDEX idx_alloc_active     ON stewardship_allocations(content_id, governance_state, effective_until);
CREATE INDEX idx_alloc_disputed   ON stewardship_allocations(governance_state) WHERE governance_state = 'disputed';
CREATE UNIQUE INDEX idx_alloc_unique_active ON stewardship_allocations(h_app_id, content_id, steward_presence_id)
    WHERE effective_until IS NULL AND governance_state = 'active';

-- Source of truth: DHT (Attestation with type=credential in imagodei DNA). Classification: A.
CREATE TABLE steward_credentials (
    id                   TEXT PRIMARY KEY NOT NULL,
    presence_id          TEXT NOT NULL,
    content_id           TEXT NOT NULL,
    affinity_coefficient REAL NOT NULL DEFAULT 0.0,
    credential_type      TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'active',
    dht_anchor_hash      TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_steward_credentials_presence ON steward_credentials(presence_id);
CREATE INDEX idx_steward_credentials_content  ON steward_credentials(content_id);

-- Source of truth: DHT (Link on Content entry in lamad DNA). Classification: A.
CREATE TABLE premium_gates (
    id                   TEXT PRIMARY KEY NOT NULL,
    steward_credential_id TEXT NOT NULL REFERENCES steward_credentials(id),
    steward_presence_id  TEXT NOT NULL,
    gated_resource_type  TEXT NOT NULL,
    gated_resource_ids   TEXT NOT NULL,
    gate_title           TEXT NOT NULL,
    gate_description     TEXT,
    dht_anchor_hash      TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_premium_gates_credential ON premium_gates(steward_credential_id);
CREATE INDEX idx_premium_gates_presence   ON premium_gates(steward_presence_id);

-- Source of truth: DHT (Attestation with type=access in imagodei DNA). Classification: A.
CREATE TABLE access_grants (
    id                        TEXT PRIMARY KEY NOT NULL,
    gate_id                   TEXT NOT NULL REFERENCES premium_gates(id),
    grantee_presence_id       TEXT NOT NULL,
    contributor_presence_id   TEXT,
    granted_at                TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at                TEXT,
    status                    TEXT NOT NULL DEFAULT 'active',
    dht_anchor_hash           TEXT
);

CREATE INDEX idx_access_grants_gate    ON access_grants(gate_id);
CREATE INDEX idx_access_grants_grantee ON access_grants(grantee_presence_id);

-- Source of truth: DHT (StewardedResource entry in lamad DNA). Classification: A.
-- dht_anchor_hash links to StewardedResource ActionHash.
CREATE TABLE stewarded_nodes (
    id               TEXT PRIMARY KEY NOT NULL,
    display_name     TEXT NOT NULL,
    claim_status     TEXT NOT NULL DEFAULT 'unclaimed',
    cpu_cores        INTEGER NOT NULL DEFAULT 0,
    memory_gb        INTEGER NOT NULL DEFAULT 0,
    storage_tb       REAL NOT NULL DEFAULT 0.0,
    bandwidth_mbps   INTEGER NOT NULL DEFAULT 0,
    steward_tier     TEXT NOT NULL DEFAULT 'caretaker',
    custodian_opt_in INTEGER NOT NULL DEFAULT 1,
    region           TEXT,
    context_epr_id   TEXT,
    dht_anchor_hash  TEXT,
    h_app_id         TEXT NOT NULL DEFAULT 'shefa',
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_stewarded_nodes_claim_status ON stewarded_nodes(claim_status);
CREATE INDEX idx_stewarded_nodes_h_app_id     ON stewarded_nodes(h_app_id);

-- Source of truth: SQLite (operational). Classification: C.
-- Derived from stewarded_nodes relationships. No dht_anchor_hash needed.
CREATE TABLE node_stewardship (
    node_id        TEXT NOT NULL REFERENCES stewarded_nodes(id),
    human_id       TEXT NOT NULL REFERENCES humans(id),
    affinity_score REAL NOT NULL DEFAULT 0.0,
    relationship   TEXT NOT NULL DEFAULT 'primary',
    context_epr_id TEXT,
    granted_at     TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (node_id, human_id)
);

CREATE INDEX idx_node_stewardship_human ON node_stewardship(human_id);

-- Source of truth: SQLite (operational). Classification: C.
-- Custodian metrics snapshots — one row per custodian (upsert on report).
-- Metric groups are stored as JSON TEXT blobs to stay within Diesel's 32-column limit.
CREATE TABLE custodian_metrics (
    custodian_id    TEXT PRIMARY KEY NOT NULL,
    h_app_id        TEXT NOT NULL DEFAULT 'shefa',
    tier            INTEGER NOT NULL DEFAULT 1,
    health_json     TEXT NOT NULL DEFAULT '{}',
    storage_json    TEXT NOT NULL DEFAULT '{}',
    bandwidth_json  TEXT NOT NULL DEFAULT '{}',
    computation_json TEXT NOT NULL DEFAULT '{}',
    reputation_json TEXT NOT NULL DEFAULT '{}',
    economic_json   TEXT NOT NULL DEFAULT '{}',
    collected_at    INTEGER NOT NULL DEFAULT 0,
    last_updated_at INTEGER NOT NULL DEFAULT 0
);

-- Token tables (Shefa economic rail)

-- Source of truth: DHT (notarized). Classification: A.
-- Every mint is coupled to a witnessed REA event.
CREATE TABLE token_mint_events (
    id                    TEXT PRIMARY KEY NOT NULL,
    h_app_id              TEXT NOT NULL DEFAULT 'shefa',
    amount                REAL NOT NULL,
    provenance_event_id   TEXT NOT NULL,
    mint_tier             TEXT NOT NULL DEFAULT 'micro',
    source_epr_id         TEXT NOT NULL,
    agent_id              TEXT NOT NULL,
    constitutional_context TEXT,
    elohim_attestation    TEXT,
    reasoning_trace       TEXT,
    dht_anchor_hash       TEXT,
    created_at            TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_token_mint_events_h_app_id    ON token_mint_events(h_app_id);
CREATE INDEX idx_token_mint_events_agent_id    ON token_mint_events(agent_id);
CREATE INDEX idx_token_mint_events_provenance  ON token_mint_events(provenance_event_id);
CREATE INDEX idx_token_mint_events_source_epr  ON token_mint_events(source_epr_id);
CREATE INDEX idx_token_mint_events_tier        ON token_mint_events(mint_tier);
CREATE INDEX idx_token_mint_events_created     ON token_mint_events(created_at);

-- Source of truth: Agent-scoped. Classification: B.
-- Current holdings per agent per governance layer.
CREATE TABLE token_balances (
    agent_id              TEXT NOT NULL,
    h_app_id              TEXT NOT NULL DEFAULT 'shefa',
    governance_layer      TEXT NOT NULL DEFAULT 'individual',
    balance               REAL NOT NULL DEFAULT 0.0,
    total_minted          REAL NOT NULL DEFAULT 0.0,
    total_transferred_in  REAL NOT NULL DEFAULT 0.0,
    total_transferred_out REAL NOT NULL DEFAULT 0.0,
    last_activity_at      TEXT NOT NULL DEFAULT (datetime('now')),
    created_at            TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent_id, h_app_id, governance_layer)
);

CREATE INDEX idx_token_balances_h_app_id ON token_balances(h_app_id);
CREATE INDEX idx_token_balances_balance  ON token_balances(balance);

-- Source of truth: DHT (notarized). Classification: A.
-- Witnessed exchanges between agents.
CREATE TABLE token_transfers (
    id               TEXT PRIMARY KEY NOT NULL,
    h_app_id         TEXT NOT NULL DEFAULT 'shefa',
    from_agent       TEXT NOT NULL,
    to_agent         TEXT NOT NULL,
    amount           REAL NOT NULL,
    governance_layer TEXT NOT NULL DEFAULT 'individual',
    note             TEXT,
    dht_anchor_hash  TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_token_transfers_h_app_id ON token_transfers(h_app_id);
CREATE INDEX idx_token_transfers_from     ON token_transfers(from_agent);
CREATE INDEX idx_token_transfers_to       ON token_transfers(to_agent);
CREATE INDEX idx_token_transfers_created  ON token_transfers(created_at);

-- Source of truth: DHT. Classification: A.
-- ResponsibilityDemandParam config per governance layer.
CREATE TABLE responsibility_demand_configs (
    id                       TEXT PRIMARY KEY NOT NULL,
    h_app_id                 TEXT NOT NULL DEFAULT 'shefa',
    governance_layer         TEXT NOT NULL,
    dignity_floor            REAL NOT NULL DEFAULT 100.0,
    median_estimate          REAL NOT NULL DEFAULT 1000.0,
    soft_ceiling_multiplier  REAL NOT NULL DEFAULT 10.0,
    hard_ceiling_multiplier  REAL NOT NULL DEFAULT 20.0,
    social_contract_health   REAL NOT NULL DEFAULT 0.5,
    enforcement_active       INTEGER NOT NULL DEFAULT 1,
    ratified_by              TEXT,
    ratified_at              TEXT,
    dht_anchor_hash          TEXT,
    created_at               TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at               TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(h_app_id, governance_layer)
);

CREATE INDEX idx_rdc_h_app_id        ON responsibility_demand_configs(h_app_id);
CREATE INDEX idx_rdc_governance_layer ON responsibility_demand_configs(governance_layer);

-- Source of truth: SQLite (operational). Classification: C.
-- Demurrage records for obligation enforcement.
CREATE TABLE token_decay_events (
    id               TEXT PRIMARY KEY NOT NULL,
    h_app_id         TEXT NOT NULL DEFAULT 'shefa',
    agent_id         TEXT NOT NULL,
    governance_layer TEXT NOT NULL,
    balance_before   REAL NOT NULL,
    balance_after    REAL NOT NULL,
    decay_amount     REAL NOT NULL,
    obligation_level TEXT NOT NULL,
    dignity_floor    REAL NOT NULL,
    created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_token_decay_agent   ON token_decay_events(agent_id);
CREATE INDEX idx_token_decay_h_app   ON token_decay_events(h_app_id);
CREATE INDEX idx_token_decay_created ON token_decay_events(created_at);

-- ============================================================================
-- Governance Pillar (Qahal / Mishpat DNA)
-- ============================================================================

-- Source of truth: DHT (Collective entry in imagodei DNA). Classification: A.
CREATE TABLE collectives (
    id                       TEXT PRIMARY KEY NOT NULL,
    h_app_id                 TEXT NOT NULL DEFAULT 'qahal',
    name                     TEXT NOT NULL,
    description              TEXT,
    governance_layer         TEXT NOT NULL DEFAULT 'community',
    constitutional_parent_id TEXT,
    reach                    TEXT NOT NULL DEFAULT 'community',
    metadata_json            TEXT,
    created_by               TEXT,
    created_at               TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at               TEXT NOT NULL DEFAULT (datetime('now')),
    dissolved_at             TEXT
);

CREATE INDEX idx_collectives_app   ON collectives(h_app_id);
CREATE INDEX idx_collectives_layer ON collectives(governance_layer);

-- Source of truth: DHT (derived from Collective via Link). Classification: A2.
CREATE TABLE collective_participations (
    id               TEXT PRIMARY KEY NOT NULL,
    h_app_id         TEXT NOT NULL DEFAULT 'qahal',
    collective_id    TEXT NOT NULL,
    human_id         TEXT NOT NULL,
    intimacy_level   TEXT NOT NULL DEFAULT 'recognition',
    role_context     TEXT,
    governance_weight REAL NOT NULL DEFAULT 1.0,
    consent_state    TEXT NOT NULL DEFAULT 'pending',
    metadata_json    TEXT,
    joined_at        TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now')),
    departed_at      TEXT,
    FOREIGN KEY (collective_id) REFERENCES collectives(id),
    UNIQUE(h_app_id, collective_id, human_id)
);

CREATE INDEX idx_participations_app        ON collective_participations(h_app_id);
CREATE INDEX idx_participations_collective ON collective_participations(h_app_id, collective_id);
CREATE INDEX idx_participations_human      ON collective_participations(h_app_id, human_id);

-- Source of truth: DHT (governance entry in lamad/mishpat DNA). Classification: A.
-- dht_anchor_hash links to Proposal ActionHash.
CREATE TABLE proposals (
    id                   TEXT PRIMARY KEY NOT NULL,
    content_id           TEXT NOT NULL,
    proposer_presence_id TEXT NOT NULL,
    proposal_type        TEXT NOT NULL DEFAULT 'consent',
    title                TEXT NOT NULL,
    body                 TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'open',
    votes_for            INTEGER NOT NULL DEFAULT 0,
    votes_against        INTEGER NOT NULL DEFAULT 0,
    voting_anonymous     INTEGER NOT NULL DEFAULT 0,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL,
    voting_mechanism     TEXT NOT NULL DEFAULT 'consent',
    score_min            INTEGER,
    score_max            INTEGER,
    dots_per_voter       INTEGER,
    quorum_percentage    REAL,
    passage_threshold    REAL,
    dht_anchor_hash      TEXT
);

CREATE INDEX idx_proposals_content_id ON proposals(content_id);
CREATE INDEX idx_proposals_status     ON proposals(status);

-- Source of truth: DHT (derived from proposal via Link). Classification: A2.
CREATE TABLE proposal_options (
    id                   TEXT PRIMARY KEY NOT NULL,
    proposal_id          TEXT NOT NULL,
    label                TEXT NOT NULL,
    description          TEXT NOT NULL,
    position             INTEGER NOT NULL,
    source               TEXT,
    source_justification TEXT,
    created_at           TEXT NOT NULL,
    dht_anchor_hash      TEXT
);

-- Source of truth: DHT (derived from proposal lifecycle in mishpat DNA). Classification: A2.
CREATE TABLE governance_states (
    id           TEXT PRIMARY KEY NOT NULL,
    entity_type  TEXT NOT NULL,
    entity_id    TEXT NOT NULL,
    reach        TEXT NOT NULL DEFAULT 'close',
    labels       TEXT NOT NULL DEFAULT '[]',
    voting_state TEXT NOT NULL DEFAULT 'none',
    signal_count INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    dht_anchor_hash TEXT,
    UNIQUE(entity_type, entity_id)
);

CREATE INDEX idx_governance_states_entity ON governance_states(entity_type, entity_id);

-- Source of truth: Private source chain (agent-scoped ballot in mishpat DNA). Classification: B2.
-- dht_anchor_hash populated when tally Attestation is issued.
CREATE TABLE votes (
    id          TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    human_id    TEXT NOT NULL,
    position    TEXT NOT NULL,
    reason      TEXT,
    anonymous   INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    dht_anchor_hash TEXT,
    UNIQUE(proposal_id, human_id)
);

-- Source of truth: Private source chain (agent-scoped ballot in mishpat DNA). Classification: B2.
-- dht_anchor_hash populated when tally Attestation is issued.
CREATE TABLE ranked_votes (
    id                   TEXT PRIMARY KEY NOT NULL,
    proposal_id          TEXT NOT NULL,
    human_id             TEXT NOT NULL,
    option_id            TEXT NOT NULL,
    rank                 INTEGER,
    score                INTEGER,
    dots                 INTEGER,
    approved             INTEGER,
    reasoning            TEXT,
    proxy_elohim_id      TEXT,
    proxy_justification  TEXT,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL,
    dht_anchor_hash      TEXT,
    UNIQUE(proposal_id, human_id, option_id)
);

CREATE INDEX idx_ranked_votes_proposal ON ranked_votes(proposal_id);

-- Source of truth: Private source chain (agent-scoped reaction in mishpat DNA). Classification: B2.
-- dht_anchor_hash populated when aggregate Attestation is notarized.
CREATE TABLE governance_signals (
    id               TEXT PRIMARY KEY NOT NULL,
    entity_type      TEXT NOT NULL,
    entity_id        TEXT NOT NULL,
    human_id         TEXT NOT NULL,
    signal_type      TEXT NOT NULL,
    signal_value     TEXT NOT NULL,
    mechanism_level  INTEGER NOT NULL,
    proxy_elohim_id  TEXT,
    created_at       TEXT NOT NULL,
    dht_anchor_hash  TEXT
);

CREATE INDEX idx_governance_signals_entity ON governance_signals(entity_type, entity_id);

-- Source of truth: DHT (Challenge entry in mishpat DNA). Classification: A.
CREATE TABLE challenges (
    id                TEXT PRIMARY KEY NOT NULL,
    entity_type       TEXT NOT NULL,
    entity_id         TEXT NOT NULL,
    challenger_id     TEXT NOT NULL,
    standing_basis    TEXT NOT NULL,
    grounds_primary   TEXT NOT NULL,
    grounds_secondary TEXT,
    evidence          TEXT NOT NULL,
    requested_outcome TEXT,
    state             TEXT NOT NULL DEFAULT 'pending',
    response_outcome  TEXT,
    response_reasoning TEXT,
    response_actions  TEXT,
    response_by       TEXT,
    sets_precedent    INTEGER NOT NULL DEFAULT 0,
    filed_at          TEXT NOT NULL,
    acknowledged_at   TEXT,
    response_deadline TEXT NOT NULL,
    responded_at      TEXT,
    resolved_at       TEXT,
    created_at        TEXT NOT NULL,
    dht_anchor_hash   TEXT
);

CREATE INDEX idx_challenges_entity ON challenges(entity_type, entity_id);
CREATE INDEX idx_challenges_state  ON challenges(state);

-- Source of truth: DHT (Appeal entry in mishpat DNA). Classification: A.
CREATE TABLE appeals (
    id                 TEXT PRIMARY KEY NOT NULL,
    challenge_id       TEXT NOT NULL,
    appellant_id       TEXT NOT NULL,
    grounds            TEXT NOT NULL,
    additional_evidence TEXT,
    state              TEXT NOT NULL DEFAULT 'pending',
    escalation_level   TEXT,
    decision           TEXT,
    decision_reasoning TEXT,
    decided_by         TEXT,
    filed_at           TEXT NOT NULL,
    decided_at         TEXT,
    created_at         TEXT NOT NULL,
    dht_anchor_hash    TEXT
);

CREATE INDEX idx_appeals_challenge ON appeals(challenge_id);

-- Source of truth: DHT (Statement entry in mishpat DNA). Classification: A.
CREATE TABLE statements (
    id              TEXT PRIMARY KEY NOT NULL,
    entity_type     TEXT NOT NULL,
    entity_id       TEXT NOT NULL,
    human_id        TEXT NOT NULL,
    text            TEXT NOT NULL,
    agree_count     INTEGER NOT NULL DEFAULT 0,
    disagree_count  INTEGER NOT NULL DEFAULT 0,
    pass_count      INTEGER NOT NULL DEFAULT 0,
    group_id        TEXT,
    is_bridging     INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    dht_anchor_hash TEXT
);

CREATE INDEX idx_statements_entity ON statements(entity_type, entity_id);

-- Source of truth: Private source chain (agent-scoped stance in mishpat DNA). Classification: B2.
CREATE TABLE statement_votes (
    id           TEXT PRIMARY KEY NOT NULL,
    statement_id TEXT NOT NULL,
    human_id     TEXT NOT NULL,
    vote         TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    dht_anchor_hash TEXT,
    UNIQUE(statement_id, human_id),
    FOREIGN KEY (statement_id) REFERENCES statements(id)
);

CREATE INDEX idx_statement_votes_statement ON statement_votes(statement_id);

-- Source of truth: DHT (Precedent entry in mishpat DNA). Classification: A.
-- Immutable governance precedents.
CREATE TABLE precedents (
    id              TEXT PRIMARY KEY NOT NULL,
    content_id      TEXT NOT NULL,
    principle       TEXT NOT NULL,
    interpretation  TEXT NOT NULL,
    established_by  TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    dht_anchor_hash TEXT
);

CREATE INDEX idx_precedents_content_id ON precedents(content_id);

-- Source of truth: SQLite (operational). Classification: C.
-- Threaded conversation anchored to content or proposals.
CREATE TABLE discussions (
    id                   TEXT PRIMARY KEY NOT NULL,
    content_id           TEXT NOT NULL,
    author_presence_id   TEXT NOT NULL,
    body                 TEXT NOT NULL,
    parent_id            TEXT,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);

CREATE INDEX idx_discussions_content_id ON discussions(content_id);
CREATE INDEX idx_discussions_parent_id  ON discussions(parent_id);

-- Source of truth: SQLite (operational). Classification: C.
CREATE TABLE comments (
    id               TEXT PRIMARY KEY NOT NULL,
    h_app_id         TEXT NOT NULL DEFAULT 'lamad',
    content_id       TEXT NOT NULL,
    human_id         TEXT NOT NULL,
    body             TEXT NOT NULL,
    reach            TEXT NOT NULL DEFAULT 'close',
    governance_state TEXT NOT NULL DEFAULT 'active',
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL
);

CREATE INDEX idx_comments_content_id ON comments(content_id);
CREATE INDEX idx_comments_human_id   ON comments(human_id);
CREATE INDEX idx_comments_h_app_id   ON comments(h_app_id);

-- Source of truth: Agent source chain (private governance profile). Classification: B.
-- NOT published to DHT.
CREATE TABLE governance_dispositions (
    id                       TEXT PRIMARY KEY NOT NULL,
    human_id                 TEXT NOT NULL UNIQUE,
    risk_tolerance           REAL NOT NULL DEFAULT 0.5,
    change_openness          REAL NOT NULL DEFAULT 0.5,
    consensus_preference     REAL NOT NULL DEFAULT 0.5,
    priority_values          TEXT NOT NULL DEFAULT '[]',
    voting_pattern_summary   TEXT NOT NULL DEFAULT '{}',
    total_votes_cast         INTEGER NOT NULL DEFAULT 0,
    total_challenges_filed   INTEGER NOT NULL DEFAULT 0,
    total_signals_recorded   INTEGER NOT NULL DEFAULT 0,
    dht_anchor_hash          TEXT,
    last_computed_at         TEXT NOT NULL,
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL
);

CREATE INDEX idx_governance_dispositions_human ON governance_dispositions(human_id);

-- ============================================================================
-- Infrastructure & Node Management
-- ============================================================================

-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- Extensible vocabulary managed at storage layer. Seeded from JSON Schema on startup.
CREATE TABLE enum_registry (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    h_app_id    TEXT NOT NULL DEFAULT 'lamad',
    enum_name   TEXT NOT NULL,
    enum_value  TEXT NOT NULL,
    tier        TEXT NOT NULL DEFAULT 'extensible',
    added_by    TEXT,
    created_at  TEXT NOT NULL,
    UNIQUE(h_app_id, enum_name, enum_value)
);

CREATE INDEX idx_enum_registry_name ON enum_registry(enum_name, h_app_id);

-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- Device-level parental/institutional content control policies.
CREATE TABLE device_policies (
    id                    TEXT PRIMARY KEY NOT NULL,
    subject_id            TEXT NOT NULL,
    device_id             TEXT,
    author_id             TEXT NOT NULL,
    author_tier           TEXT NOT NULL,
    inherits_from         TEXT,
    blocked_categories_json TEXT NOT NULL DEFAULT '[]',
    blocked_hashes_json   TEXT NOT NULL DEFAULT '[]',
    age_rating_max        TEXT,
    reach_level_max       INTEGER,
    session_max_minutes   INTEGER,
    daily_max_minutes     INTEGER,
    time_windows_json     TEXT NOT NULL DEFAULT '[]',
    cooldown_minutes      INTEGER,
    disabled_features_json TEXT NOT NULL DEFAULT '[]',
    disabled_routes_json  TEXT NOT NULL DEFAULT '[]',
    require_approval_json TEXT NOT NULL DEFAULT '[]',
    log_sessions          INTEGER NOT NULL DEFAULT 0,
    log_categories        INTEGER NOT NULL DEFAULT 0,
    log_policy_events     INTEGER NOT NULL DEFAULT 0,
    retention_days        INTEGER NOT NULL DEFAULT 30,
    subject_can_view      INTEGER NOT NULL DEFAULT 1,
    effective_from        TEXT NOT NULL DEFAULT (datetime('now')),
    effective_until       TEXT,
    version               INTEGER NOT NULL DEFAULT 1,
    created_at            TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at            TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Geospatial
-- ============================================================================

-- Source of truth: DHT (Mishpat DNA). Classification: A.
-- Read-optimised projection of Place entries from the DHT.
CREATE TABLE places (
    id                      TEXT PRIMARY KEY NOT NULL,
    h_app_id                TEXT NOT NULL DEFAULT 'lamad',
    dht_anchor_hash         TEXT NOT NULL,
    name                    TEXT NOT NULL,
    place_type              TEXT NOT NULL,
    constitutional_layer    TEXT NOT NULL,
    h3_index                TEXT NOT NULL,
    h3_resolution           INTEGER NOT NULL,
    geometry_json           TEXT NOT NULL,
    centroid_lat            REAL NOT NULL,
    centroid_lng            REAL NOT NULL,
    parent_place_id         TEXT,
    osm_reference_json      TEXT,
    carrying_capacity_json  TEXT NOT NULL DEFAULT '[]',
    governing_collective_id TEXT,
    status                  TEXT NOT NULL DEFAULT 'proposed',
    created_by              TEXT NOT NULL,
    created_at              TEXT NOT NULL,
    updated_at              TEXT NOT NULL,
    metadata_json           TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_places_h3         ON places(h3_index);
CREATE INDEX idx_places_type       ON places(place_type, h_app_id);
CREATE INDEX idx_places_layer      ON places(constitutional_layer, h_app_id);
CREATE INDEX idx_places_parent     ON places(parent_place_id);
CREATE INDEX idx_places_status     ON places(status, h_app_id);
CREATE INDEX idx_places_collective ON places(governing_collective_id);
CREATE INDEX idx_places_dht        ON places(dht_anchor_hash);

-- Source of truth: SQLite (operational). Classification: C.
-- Geospatial context for any CID-addressed entity. Supports temporal history.
CREATE TABLE spatial_contexts (
    id            TEXT PRIMARY KEY NOT NULL,
    h_app_id      TEXT NOT NULL DEFAULT 'lamad',
    entity_type   TEXT NOT NULL,
    entity_id     TEXT NOT NULL,
    latitude      REAL,
    longitude     REAL,
    altitude      REAL,
    accuracy      REAL,
    h3_res5       TEXT,
    h3_res7       TEXT,
    h3_res9       TEXT,
    place_id      TEXT,
    osm_type      TEXT,
    osm_id        INTEGER,
    label         TEXT,
    context_type  TEXT NOT NULL DEFAULT 'point',
    geometry_json TEXT,
    metadata_json TEXT,
    observed_at   TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    is_current    INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_spatial_ctx_entity   ON spatial_contexts(entity_type, entity_id);
CREATE INDEX idx_spatial_ctx_h3_res5  ON spatial_contexts(h3_res5);
CREATE INDEX idx_spatial_ctx_h3_res7  ON spatial_contexts(h3_res7);
CREATE INDEX idx_spatial_ctx_h3_res9  ON spatial_contexts(h3_res9);
CREATE INDEX idx_spatial_ctx_place    ON spatial_contexts(place_id);
CREATE INDEX idx_spatial_ctx_type     ON spatial_contexts(entity_type, h_app_id);
CREATE INDEX idx_spatial_ctx_current  ON spatial_contexts(entity_type, entity_id, is_current);

-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- Ephemeral situational data; reconstructable from external APIs.
CREATE TABLE hazards (
    id                TEXT PRIMARY KEY NOT NULL,
    h_app_id          TEXT NOT NULL DEFAULT 'lamad',
    place_id          TEXT NOT NULL,
    hazard_type       TEXT NOT NULL,
    severity          TEXT NOT NULL,
    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    reported_at       TEXT NOT NULL,
    projected_onset   TEXT,
    projected_end     TEXT,
    actual_onset      TEXT,
    resolved_at       TEXT,
    affected_h3_cells TEXT NOT NULL DEFAULT '[]',
    radius_km         REAL,
    source            TEXT NOT NULL,
    source_reference  TEXT,
    metadata_json     TEXT NOT NULL DEFAULT '{}',
    status            TEXT NOT NULL DEFAULT 'active',
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE INDEX idx_hazards_place  ON hazards(place_id, h_app_id);
CREATE INDEX idx_hazards_status ON hazards(status, h_app_id);
CREATE INDEX idx_hazards_type   ON hazards(hazard_type, h_app_id);
CREATE INDEX idx_hazards_onset  ON hazards(projected_onset);

-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- Derived from threshold crossings on hazards + vulnerability data.
CREATE TABLE risk_alerts (
    id                TEXT PRIMARY KEY NOT NULL,
    h_app_id          TEXT NOT NULL DEFAULT 'lamad',
    place_id          TEXT NOT NULL,
    alert_type        TEXT NOT NULL,
    severity          TEXT NOT NULL,
    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    trigger_hazard_id TEXT,
    trigger_data_json TEXT NOT NULL DEFAULT '{}',
    triggered_at      TEXT NOT NULL,
    lead_time_hours   REAL,
    expires_at        TEXT,
    status            TEXT NOT NULL DEFAULT 'active',
    acknowledged_by   TEXT,
    acknowledged_at   TEXT,
    resolved_at       TEXT,
    escalated_to      TEXT,
    metadata_json     TEXT NOT NULL DEFAULT '{}',
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE INDEX idx_risk_alerts_place  ON risk_alerts(place_id, h_app_id);
CREATE INDEX idx_risk_alerts_status ON risk_alerts(status, h_app_id);
CREATE INDEX idx_risk_alerts_type   ON risk_alerts(alert_type, h_app_id);
CREATE INDEX idx_risk_alerts_dedup  ON risk_alerts(place_id, alert_type, trigger_hazard_id)
    WHERE status = 'active';

-- ============================================================================
-- Blob / Shard Storage
-- ============================================================================

-- Source of truth: Local SQLite (per-peer encoding state). Classification: C.
-- Rebuilt from local blob store; not shared via DHT.
CREATE TABLE shard_manifests (
    content_id          TEXT NOT NULL,
    h_app_id            TEXT NOT NULL DEFAULT 'lamad',
    blob_hash           TEXT NOT NULL,
    blob_cid            TEXT,
    encoding            TEXT NOT NULL DEFAULT 'none',
    data_shard_count    INTEGER NOT NULL DEFAULT 1,
    parity_shard_count  INTEGER NOT NULL DEFAULT 0,
    shard_hashes_json   TEXT NOT NULL DEFAULT '[]',
    total_size_bytes    INTEGER NOT NULL,
    shard_size_bytes    INTEGER NOT NULL,
    mime_type           TEXT NOT NULL DEFAULT 'application/octet-stream',
    reach               TEXT NOT NULL DEFAULT 'commons',
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (content_id, h_app_id)
);

CREATE INDEX idx_shard_manifests_blob     ON shard_manifests(blob_hash);
CREATE INDEX idx_shard_manifests_encoding ON shard_manifests(encoding);

-- Source of truth: Local SQLite (peer shard tracking). Classification: C.
-- Rebuilt from shard protocol ack events.
CREATE TABLE shard_locations (
    shard_hash    TEXT NOT NULL,
    peer_id       TEXT NOT NULL,
    h_app_id      TEXT NOT NULL DEFAULT 'lamad',
    status        TEXT NOT NULL DEFAULT 'announced',
    first_seen    TEXT NOT NULL DEFAULT (datetime('now')),
    last_verified TEXT,
    PRIMARY KEY (shard_hash, peer_id)
);

CREATE INDEX idx_shard_locations_peer   ON shard_locations(peer_id);
CREATE INDEX idx_shard_locations_status ON shard_locations(status);

-- ============================================================================
-- Observation Sessions (A2O Pipeline)
-- ============================================================================

-- Source of truth: SQLite (operational, Category C).
-- Sessions and entries are ephemeral working data; purgeable after report composition.
-- The composed report is persisted as a Content node (Category A via content table).
CREATE TABLE observation_sessions (
    id                TEXT PRIMARY KEY NOT NULL,
    started_at        TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at          TEXT,
    ttl_seconds       INTEGER NOT NULL DEFAULT 300,
    source            TEXT NOT NULL,
    metadata_json     TEXT,
    report_content_id TEXT
);

CREATE INDEX idx_obs_sessions_started ON observation_sessions(started_at);
CREATE INDEX idx_obs_sessions_source  ON observation_sessions(source);

CREATE TABLE observation_entries (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT NOT NULL REFERENCES observation_sessions(id),
    timestamp   TEXT NOT NULL DEFAULT (datetime('now')),
    origin      TEXT NOT NULL,
    category    TEXT NOT NULL,
    severity    TEXT NOT NULL DEFAULT 'info',
    method      TEXT,
    path        TEXT,
    status_code INTEGER,
    message     TEXT NOT NULL,
    context_json TEXT
);

CREATE INDEX idx_obs_entries_session ON observation_entries(session_id);
