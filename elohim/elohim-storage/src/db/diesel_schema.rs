// @generated automatically by Diesel CLI.

diesel::table! {
    apps (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        created_at -> Text,
        enabled -> Integer,
    }
}

diesel::table! {
    collectives (id) {
        id -> Text,
        app_id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        governance_layer -> Text,
        constitutional_parent_id -> Nullable<Text>,
        reach -> Text,
        metadata_json -> Nullable<Text>,
        created_by -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
        dissolved_at -> Nullable<Text>,
    }
}

diesel::table! {
    collective_participations (id) {
        id -> Text,
        app_id -> Text,
        collective_id -> Text,
        human_id -> Text,
        intimacy_level -> Text,
        role_context -> Nullable<Text>,
        governance_weight -> Float,
        consent_state -> Text,
        metadata_json -> Nullable<Text>,
        joined_at -> Text,
        updated_at -> Text,
        departed_at -> Nullable<Text>,
    }
}

diesel::table! {
    chapters (id) {
        id -> Text,
        app_id -> Text,
        path_id -> Text,
        title -> Text,
        description -> Nullable<Text>,
        order_index -> Integer,
        estimated_duration -> Nullable<Text>,
    }
}

diesel::table! {
    content (id) {
        id -> Text,
        app_id -> Text,
        title -> Text,
        description -> Nullable<Text>,
        content_type -> Text,
        content_format -> Text,
        blob_hash -> Nullable<Text>,
        blob_cid -> Nullable<Text>,
        content_size_bytes -> Nullable<Integer>,
        metadata_json -> Nullable<Text>,
        reach -> Text,
        validation_status -> Text,
        created_by -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
        content_body -> Nullable<Text>,
    }
}

diesel::table! {
    content_mastery (id) {
        id -> Text,
        app_id -> Text,
        human_id -> Text,
        content_id -> Text,
        mastery_level -> Text,
        mastery_level_index -> Integer,
        freshness_score -> Float,
        needs_refresh -> Integer,
        engagement_count -> Integer,
        last_engagement_type -> Nullable<Text>,
        last_engagement_at -> Nullable<Text>,
        level_achieved_at -> Nullable<Text>,
        content_version_at_mastery -> Nullable<Text>,
        assessment_evidence_json -> Nullable<Text>,
        privileges_json -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    content_tags (app_id, content_id, tag) {
        app_id -> Text,
        content_id -> Text,
        tag -> Text,
    }
}

diesel::table! {
    contributor_presences (id) {
        id -> Text,
        app_id -> Text,
        display_name -> Text,
        presence_state -> Text,
        external_identifiers_json -> Nullable<Text>,
        establishing_content_ids_json -> Text,
        affinity_total -> Float,
        unique_engagers -> Integer,
        citation_count -> Integer,
        recognition_score -> Float,
        recognition_by_content_json -> Nullable<Text>,
        last_recognition_at -> Nullable<Text>,
        steward_id -> Nullable<Text>,
        stewardship_started_at -> Nullable<Text>,
        stewardship_commitment_id -> Nullable<Text>,
        stewardship_quality_score -> Nullable<Float>,
        claim_initiated_at -> Nullable<Text>,
        claim_verified_at -> Nullable<Text>,
        claim_verification_method -> Nullable<Text>,
        claim_evidence_json -> Nullable<Text>,
        claimed_agent_id -> Nullable<Text>,
        claim_recognition_transferred_value -> Nullable<Float>,
        claim_facilitated_by -> Nullable<Text>,
        image -> Nullable<Text>,
        note -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    economic_events (id) {
        id -> Text,
        app_id -> Text,
        action -> Text,
        provider -> Text,
        receiver -> Text,
        resource_conforms_to -> Nullable<Text>,
        resource_inventoried_as -> Nullable<Text>,
        resource_classified_as_json -> Nullable<Text>,
        resource_quantity_value -> Nullable<Float>,
        resource_quantity_unit -> Nullable<Text>,
        effort_quantity_value -> Nullable<Float>,
        effort_quantity_unit -> Nullable<Text>,
        has_point_in_time -> Text,
        has_duration -> Nullable<Text>,
        input_of -> Nullable<Text>,
        output_of -> Nullable<Text>,
        lamad_event_type -> Nullable<Text>,
        content_id -> Nullable<Text>,
        contributor_presence_id -> Nullable<Text>,
        path_id -> Nullable<Text>,
        triggered_by -> Nullable<Text>,
        state -> Text,
        note -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    human_relationships (id) {
        id -> Text,
        app_id -> Text,
        party_a_id -> Text,
        party_b_id -> Text,
        relationship_type -> Text,
        intimacy_level -> Text,
        is_bidirectional -> Integer,
        consent_given_by_a -> Integer,
        consent_given_by_b -> Integer,
        custody_enabled_by_a -> Integer,
        custody_enabled_by_b -> Integer,
        auto_custody_enabled -> Integer,
        emergency_access_enabled -> Integer,
        initiated_by -> Text,
        verified_at -> Nullable<Text>,
        governance_layer -> Nullable<Text>,
        reach -> Text,
        context_json -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
        expires_at -> Nullable<Text>,
    }
}

diesel::table! {
    local_sessions (id) {
        id -> Text,
        human_id -> Text,
        agent_pub_key -> Text,
        doorway_url -> Text,
        doorway_id -> Nullable<Text>,
        identifier -> Text,
        display_name -> Nullable<Text>,
        profile_image_hash -> Nullable<Text>,
        is_active -> Integer,
        created_at -> Text,
        updated_at -> Text,
        last_synced_at -> Nullable<Text>,
        bootstrap_url -> Nullable<Text>,
    }
}

diesel::table! {
    path_attestations (app_id, path_id, attestation_type) {
        app_id -> Text,
        path_id -> Text,
        attestation_type -> Text,
        attestation_name -> Text,
    }
}

diesel::table! {
    path_tags (app_id, path_id, tag) {
        app_id -> Text,
        path_id -> Text,
        tag -> Text,
    }
}

diesel::table! {
    paths (id) {
        id -> Text,
        app_id -> Text,
        title -> Text,
        description -> Nullable<Text>,
        path_type -> Text,
        difficulty -> Nullable<Text>,
        estimated_duration -> Nullable<Text>,
        thumbnail_url -> Nullable<Text>,
        thumbnail_alt -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
        visibility -> Text,
        created_by -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    relationships (id) {
        id -> Text,
        app_id -> Text,
        source_id -> Text,
        target_id -> Text,
        relationship_type -> Text,
        confidence -> Float,
        inference_source -> Text,
        is_bidirectional -> Integer,
        inverse_relationship_id -> Nullable<Text>,
        provenance_chain_json -> Nullable<Text>,
        governance_layer -> Nullable<Text>,
        reach -> Text,
        metadata_json -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    schema_version (rowid) {
        rowid -> Integer,
        version -> Integer,
    }
}

diesel::table! {
    stewardship_allocations (id) {
        id -> Text,
        app_id -> Text,
        content_id -> Text,
        steward_presence_id -> Text,
        allocation_ratio -> Float,
        allocation_method -> Text,
        contribution_type -> Text,
        contribution_evidence_json -> Nullable<Text>,
        governance_state -> Text,
        dispute_id -> Nullable<Text>,
        dispute_reason -> Nullable<Text>,
        disputed_at -> Nullable<Text>,
        disputed_by -> Nullable<Text>,
        negotiation_session_id -> Nullable<Text>,
        elohim_ratified_at -> Nullable<Text>,
        elohim_ratifier_id -> Nullable<Text>,
        effective_from -> Text,
        effective_until -> Nullable<Text>,
        superseded_by -> Nullable<Text>,
        recognition_accumulated -> Float,
        last_recognition_at -> Nullable<Text>,
        note -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    device_policies (id) {
        id -> Text,
        subject_id -> Text,
        device_id -> Nullable<Text>,
        author_id -> Text,
        author_tier -> Text,
        inherits_from -> Nullable<Text>,
        blocked_categories_json -> Text,
        blocked_hashes_json -> Text,
        age_rating_max -> Nullable<Text>,
        reach_level_max -> Nullable<Integer>,
        session_max_minutes -> Nullable<Integer>,
        daily_max_minutes -> Nullable<Integer>,
        time_windows_json -> Text,
        cooldown_minutes -> Nullable<Integer>,
        disabled_features_json -> Text,
        disabled_routes_json -> Text,
        require_approval_json -> Text,
        log_sessions -> Integer,
        log_categories -> Integer,
        log_policy_events -> Integer,
        retention_days -> Integer,
        subject_can_view -> Integer,
        effective_from -> Text,
        effective_until -> Nullable<Text>,
        version -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    humans (id) {
        id -> Text,
        agent_pub_key -> Nullable<Text>,
        display_name -> Text,
        bio -> Nullable<Text>,
        affinities -> Text,
        profile_reach -> Text,
        location -> Nullable<Text>,
        app_id -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    steps (id) {
        id -> Text,
        app_id -> Text,
        path_id -> Text,
        chapter_id -> Nullable<Text>,
        title -> Text,
        description -> Nullable<Text>,
        step_type -> Text,
        resource_id -> Nullable<Text>,
        resource_type -> Nullable<Text>,
        order_index -> Integer,
        estimated_duration -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
    }
}

diesel::table! {
    /// Custodian metrics snapshots — one row per custodian (upsert on report).
    ///
    /// Metric groups are stored as JSON TEXT blobs to stay within Diesel's
    /// 32-column limit. The service layer deserialises them into typed structs
    /// before building the CustodianMetricsView.
    custodian_metrics (custodian_id) {
        custodian_id -> Text,
        app_id -> Text,
        tier -> Integer,
        health_json -> Text,
        storage_json -> Text,
        bandwidth_json -> Text,
        computation_json -> Text,
        reputation_json -> Text,
        economic_json -> Text,
        collected_at -> BigInt,
        last_updated_at -> BigInt,
    }
}

diesel::table! {
    governance_states (id) {
        id -> Text,
        entity_type -> Text,
        entity_id -> Text,
        reach -> Text,
        labels -> Text,
        voting_state -> Text,
        signal_count -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    challenges (id) {
        id -> Text,
        content_id -> Text,
        challenger_presence_id -> Text,
        reason -> Text,
        status -> Text,
        evidence -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    proposals (id) {
        id -> Text,
        content_id -> Text,
        proposer_presence_id -> Text,
        proposal_type -> Text,
        title -> Text,
        body -> Text,
        status -> Text,
        votes_for -> Integer,
        votes_against -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    precedents (id) {
        id -> Text,
        content_id -> Text,
        principle -> Text,
        interpretation -> Text,
        established_by -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    discussions (id) {
        id -> Text,
        content_id -> Text,
        author_presence_id -> Text,
        body -> Text,
        parent_id -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    content_attestations (id) {
        id -> Text,
        content_id -> Text,
        attestor_presence_id -> Text,
        scope -> Text,
        attestation_type -> Text,
        evidence -> Nullable<Text>,
        grantor -> Nullable<Text>,
        is_revoked -> Integer,
        revocation -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    steward_credentials (id) {
        id -> Text,
        presence_id -> Text,
        content_id -> Text,
        affinity_coefficient -> Float,
        credential_type -> Text,
        status -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    premium_gates (id) {
        id -> Text,
        steward_credential_id -> Text,
        steward_presence_id -> Text,
        gated_resource_type -> Text,
        gated_resource_ids -> Text,
        gate_title -> Text,
        gate_description -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    access_grants (id) {
        id -> Text,
        gate_id -> Text,
        grantee_presence_id -> Text,
        contributor_presence_id -> Nullable<Text>,
        granted_at -> Text,
        expires_at -> Nullable<Text>,
        status -> Text,
    }
}

diesel::table! {
    contributor_dashboards (presence_id) {
        presence_id -> Text,
        total_contributions -> Integer,
        total_recognitions -> Integer,
        impact_score -> Float,
        last_contribution_at -> Nullable<Text>,
        updated_at -> Text,
    }
}

diesel::table! {
    rea_commitments (id) {
        id -> Text,
        app_id -> Text,
        action -> Text,
        provider -> Text,
        receiver -> Text,
        resource_conforms_to -> Nullable<Text>,
        resource_classified_as -> Nullable<Text>,
        resource_quantity_value -> Nullable<Float>,
        resource_quantity_unit -> Nullable<Text>,
        effort_quantity_value -> Nullable<Float>,
        effort_quantity_unit -> Nullable<Text>,
        has_beginning -> Nullable<Text>,
        has_end -> Nullable<Text>,
        due -> Nullable<Text>,
        clause_of -> Nullable<Text>,
        in_scope_of -> Nullable<Text>,
        medium_of_exchange_id -> Nullable<Text>,
        state -> Text,
        finished -> Integer,
        note -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    agreements (id) {
        id -> Text,
        app_id -> Text,
        name -> Nullable<Text>,
        note -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    stewarded_nodes (id) {
        id -> Text,
        display_name -> Text,
        claim_status -> Text,
        cpu_cores -> Integer,
        memory_gb -> Integer,
        storage_tb -> Double,
        bandwidth_mbps -> Integer,
        steward_tier -> Text,
        custodian_opt_in -> Integer,
        region -> Nullable<Text>,
        context_epr_id -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        app_id -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    node_stewardship (node_id, human_id) {
        node_id -> Text,
        human_id -> Text,
        affinity_score -> Double,
        relationship -> Text,
        context_epr_id -> Nullable<Text>,
        granted_at -> Text,
    }
}

diesel::table! {
    knowledge_maps (id) {
        id -> Text,
        app_id -> Text,
        map_type -> Text,
        owner_id -> Text,
        title -> Text,
        description -> Nullable<Text>,
        subject_type -> Text,
        subject_id -> Text,
        subject_name -> Text,
        visibility -> Text,
        shared_with_json -> Nullable<Text>,
        nodes_json -> Text,
        path_ids_json -> Nullable<Text>,
        overall_affinity -> Float,
        content_graph_id -> Nullable<Text>,
        mastery_levels_json -> Nullable<Text>,
        goals_json -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    path_extensions (id) {
        id -> Text,
        app_id -> Text,
        base_path_id -> Text,
        base_path_version -> Text,
        extended_by -> Text,
        title -> Text,
        description -> Nullable<Text>,
        insertions_json -> Nullable<Text>,
        annotations_json -> Nullable<Text>,
        reorderings_json -> Nullable<Text>,
        exclusions_json -> Nullable<Text>,
        visibility -> Text,
        shared_with_json -> Nullable<Text>,
        forked_from -> Nullable<Text>,
        forks_json -> Nullable<Text>,
        upstream_proposal_json -> Nullable<Text>,
        stats_json -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::joinable!(chapters -> paths (path_id));
diesel::joinable!(collective_participations -> collectives (collective_id));
diesel::joinable!(content_tags -> content (content_id));
diesel::joinable!(node_stewardship -> stewarded_nodes (node_id));
diesel::joinable!(node_stewardship -> humans (human_id));
diesel::joinable!(path_attestations -> paths (path_id));
diesel::joinable!(path_tags -> paths (path_id));
diesel::joinable!(steps -> chapters (chapter_id));
diesel::joinable!(path_extensions -> paths (base_path_id));
diesel::joinable!(steps -> paths (path_id));

diesel::allow_tables_to_appear_in_same_query!(
    access_grants,
    agreements,
    apps,
    challenges,
    chapters,
    collective_participations,
    collectives,
    content,
    content_attestations,
    content_mastery,
    content_tags,
    contributor_dashboards,
    contributor_presences,
    custodian_metrics,
    device_policies,
    discussions,
    economic_events,
    governance_states,
    human_relationships,
    humans,
    knowledge_maps,
    local_sessions,
    node_stewardship,
    path_attestations,
    path_extensions,
    path_tags,
    paths,
    precedents,
    premium_gates,
    proposals,
    rea_commitments,
    relationships,
    schema_version,
    steward_credentials,
    stewarded_nodes,
    stewardship_allocations,
    steps,
);
