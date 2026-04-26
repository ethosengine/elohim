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
    comments (id) {
        id -> Text,
        h_app_id -> Text,
        content_id -> Text,
        human_id -> Text,
        body -> Text,
        reach -> Text,
        governance_state -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    collectives (id) {
        id -> Text,
        h_app_id -> Text,
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
        region -> Nullable<Text>,
    }
}

diesel::table! {
    collective_participations (id) {
        id -> Text,
        h_app_id -> Text,
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
    content (id) {
        id -> Text,
        h_app_id -> Text,
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
        dht_anchor_hash -> Nullable<Text>,
        p2p_published_at -> Nullable<Text>,
    }
}

diesel::table! {
    content_mastery (id) {
        id -> Text,
        h_app_id -> Text,
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
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    content_tags (h_app_id, content_id, tag) {
        h_app_id -> Text,
        content_id -> Text,
        tag -> Text,
    }
}

diesel::table! {
    contributor_presences (id) {
        id -> Text,
        h_app_id -> Text,
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
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    enum_registry (id) {
        id -> Integer,
        h_app_id -> Text,
        enum_name -> Text,
        enum_value -> Text,
        tier -> Text,
        added_by -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    economic_events (id) {
        id -> Text,
        h_app_id -> Text,
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
        at_location -> Nullable<Text>,
        verified_at -> Nullable<Text>,
    }
}

diesel::table! {
    human_relationships (id) {
        id -> Text,
        h_app_id -> Text,
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
        dht_anchor_hash -> Nullable<Text>,
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
        session_intent_json -> Nullable<Text>,
        intent_set_at -> Nullable<Text>,
    }
}

diesel::table! {
    gate_decision_attestations (app_id, decision_id) {
        app_id -> Text,
        decision_id -> Text,
        phase -> Text,
        elohim_id -> Text,
        elohim_substance_cid -> Text,
        gate_name -> Text,
        gate_process_cid -> Text,
        request_ref_json -> Text,
        decision -> Text,
        reasoning_json -> Text,
        context_summary_cid -> Text,
        decided_at -> Text,
        universal_band_cid -> Text,
        dht_anchor_hash -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    gate_decision_challenges (app_id, challenge_id) {
        app_id -> Text,
        challenge_id -> Text,
        challenged_decision_cid -> Text,
        challenger_id -> Text,
        grounds -> Text,
        summary -> Text,
        evidence_refs -> Text,
        filed_at -> Text,
        reach -> Text,
        dht_anchor_hash -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    challenge_outcomes (app_id, outcome_id) {
        app_id -> Text,
        outcome_id -> Text,
        challenge_cid -> Text,
        verdict -> Text,
        reviewer_consensus -> Text,
        reasoning_json -> Text,
        decided_at -> Text,
        indemnification_actions_json -> Text,
        dht_anchor_hash -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    peer_statuses (peer_id) {
        peer_id -> Text,
        status -> Text,
        general_pool_member -> Integer,
        accepting_stewardship_reserves -> Integer,
        archetype_class -> Nullable<Text>,
        timestamp -> BigInt,
        dht_anchor_hash -> Text,
        updated_at -> BigInt,
    }
}

diesel::table! {
    relationships (id) {
        id -> Text,
        h_app_id -> Text,
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
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    schedules (id) {
        id -> Text,
        h_app_id -> Text,
        entity_type -> Text,
        entity_id -> Text,
        scheduled_at -> Nullable<Text>,
        expires_at -> Nullable<Text>,
        rrule -> Nullable<Text>,
        last_occurred_at -> Nullable<Text>,
        next_occurrence_at -> Nullable<Text>,
        occurrence_count -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    places (id) {
        id -> Text,
        h_app_id -> Text,
        dht_anchor_hash -> Text,
        name -> Text,
        place_type -> Text,
        constitutional_layer -> Text,
        h3_index -> Text,
        h3_resolution -> Integer,
        geometry_json -> Text,
        centroid_lat -> Double,
        centroid_lng -> Double,
        parent_place_id -> Nullable<Text>,
        osm_reference_json -> Nullable<Text>,
        carrying_capacity_json -> Text,
        governing_collective_id -> Nullable<Text>,
        status -> Text,
        created_by -> Text,
        created_at -> Text,
        updated_at -> Text,
        metadata_json -> Text,
    }
}

diesel::table! {
    spatial_contexts (id) {
        id -> Text,
        h_app_id -> Text,
        entity_type -> Text,
        entity_id -> Text,
        latitude -> Nullable<Double>,
        longitude -> Nullable<Double>,
        altitude -> Nullable<Double>,
        accuracy -> Nullable<Double>,
        h3_res5 -> Nullable<Text>,
        h3_res7 -> Nullable<Text>,
        h3_res9 -> Nullable<Text>,
        place_id -> Nullable<Text>,
        osm_type -> Nullable<Text>,
        osm_id -> Nullable<Integer>,
        label -> Nullable<Text>,
        context_type -> Text,
        geometry_json -> Nullable<Text>,
        metadata_json -> Nullable<Text>,
        observed_at -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
        is_current -> Integer,
    }
}

diesel::table! {
    schema_version (rowid) {
        rowid -> Integer,
        version -> Integer,
    }
}

diesel::table! {
    shard_locations (shard_hash, peer_id) {
        shard_hash -> Text,
        peer_id -> Text,
        h_app_id -> Text,
        status -> Text,
        first_seen -> Text,
        last_verified -> Nullable<Text>,
    }
}

diesel::table! {
    shard_manifests (content_id, h_app_id) {
        content_id -> Text,
        h_app_id -> Text,
        blob_hash -> Text,
        blob_cid -> Nullable<Text>,
        encoding -> Text,
        data_shard_count -> Integer,
        parity_shard_count -> Integer,
        shard_hashes_json -> Text,
        total_size_bytes -> BigInt,
        shard_size_bytes -> BigInt,
        mime_type -> Text,
        reach -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    stewardship_allocations (id) {
        id -> Text,
        h_app_id -> Text,
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
        dht_anchor_hash -> Nullable<Text>,
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
        profile_photo_url -> Nullable<Text>,
        h_app_id -> Text,
        created_at -> Text,
        updated_at -> Text,
        dht_anchor_hash -> Nullable<Text>,
        household_id -> Nullable<Text>,
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
        h_app_id -> Text,
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
    governance_dispositions (id) {
        id -> Text,
        human_id -> Text,
        risk_tolerance -> Float,
        change_openness -> Float,
        consensus_preference -> Float,
        priority_values -> Text,
        voting_pattern_summary -> Text,
        total_votes_cast -> Integer,
        total_challenges_filed -> Integer,
        total_signals_recorded -> Integer,
        dht_anchor_hash -> Nullable<Text>,
        last_computed_at -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    governance_signals (id) {
        id -> Text,
        entity_type -> Text,
        entity_id -> Text,
        human_id -> Text,
        signal_type -> Text,
        signal_value -> Text,
        mechanism_level -> Integer,
        proxy_elohim_id -> Nullable<Text>,
        created_at -> Text,
        dht_anchor_hash -> Nullable<Text>,
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
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    challenges (id) {
        id -> Text,
        entity_type -> Text,
        entity_id -> Text,
        challenger_id -> Text,
        standing_basis -> Text,
        grounds_primary -> Text,
        grounds_secondary -> Nullable<Text>,
        evidence -> Text,
        requested_outcome -> Nullable<Text>,
        state -> Text,
        response_outcome -> Nullable<Text>,
        response_reasoning -> Nullable<Text>,
        response_actions -> Nullable<Text>,
        response_by -> Nullable<Text>,
        sets_precedent -> Integer,
        filed_at -> Text,
        acknowledged_at -> Nullable<Text>,
        response_deadline -> Text,
        responded_at -> Nullable<Text>,
        resolved_at -> Nullable<Text>,
        created_at -> Text,
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    appeals (id) {
        id -> Text,
        challenge_id -> Text,
        appellant_id -> Text,
        grounds -> Text,
        additional_evidence -> Nullable<Text>,
        state -> Text,
        escalation_level -> Nullable<Text>,
        decision -> Nullable<Text>,
        decision_reasoning -> Nullable<Text>,
        decided_by -> Nullable<Text>,
        filed_at -> Text,
        decided_at -> Nullable<Text>,
        created_at -> Text,
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    proposal_options (id) {
        id -> Text,
        proposal_id -> Text,
        label -> Text,
        description -> Text,
        position -> Integer,
        source -> Nullable<Text>,
        source_justification -> Nullable<Text>,
        created_at -> Text,
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    ranked_votes (id) {
        id -> Text,
        proposal_id -> Text,
        human_id -> Text,
        option_id -> Text,
        rank -> Nullable<Integer>,
        score -> Nullable<Integer>,
        dots -> Nullable<Integer>,
        approved -> Nullable<Integer>,
        reasoning -> Nullable<Text>,
        proxy_elohim_id -> Nullable<Text>,
        proxy_justification -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
        dht_anchor_hash -> Nullable<Text>,
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
        voting_anonymous -> Integer,
        created_at -> Text,
        updated_at -> Text,
        voting_mechanism -> Text,
        score_min -> Nullable<Integer>,
        score_max -> Nullable<Integer>,
        dots_per_voter -> Nullable<Integer>,
        quorum_percentage -> Nullable<Double>,
        passage_threshold -> Nullable<Double>,
        dht_anchor_hash -> Nullable<Text>,
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
        dht_anchor_hash -> Nullable<Text>,
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
    votes (id) {
        id -> Text,
        proposal_id -> Text,
        human_id -> Text,
        position -> Text,
        reason -> Nullable<Text>,
        anonymous -> Integer,
        created_at -> Text,
        updated_at -> Text,
        dht_anchor_hash -> Nullable<Text>,
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
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    steward_affinity (id) {
        id -> Text,
        h_app_id -> Text,
        steward_id -> Text,
        content_id -> Text,
        affinity_score -> Float,
        source -> Text,
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
        dht_anchor_hash -> Nullable<Text>,
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
        dht_anchor_hash -> Nullable<Text>,
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
        dht_anchor_hash -> Nullable<Text>,
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
        h_app_id -> Text,
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
        h_app_id -> Text,
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
        h_app_id -> Text,
        created_at -> Text,
        updated_at -> Text,
        device_archetype_id -> Nullable<Text>,
        household_id -> Nullable<Text>,
        hostname -> Nullable<Text>,
        node_role -> Nullable<Text>,
        capability_level -> Nullable<Integer>,
        can_steward -> Integer,
        can_infer -> Integer,
        can_doorway -> Integer,
        signature -> Nullable<Text>,
        signed_at -> Nullable<Text>,
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
        h_app_id -> Text,
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
    imagodei_observations (id) {
        id -> Text,
        h_app_id -> Text,
        human_id -> Text,
        observed_at -> Text,
        observation_type -> Text,
        content -> Text,
        structured_signals_json -> Nullable<Text>,
        trust_delta -> Float,
        visibility_layer -> Text,
        originating_elohim -> Text,
        relevance_decay -> Float,
        superseded_by -> Nullable<Text>,
        created_at -> Text,
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    statements (id) {
        id -> Text,
        entity_type -> Text,
        entity_id -> Text,
        human_id -> Text,
        text -> Text,
        agree_count -> Integer,
        disagree_count -> Integer,
        pass_count -> Integer,
        group_id -> Nullable<Text>,
        is_bridging -> Integer,
        created_at -> Text,
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    statement_votes (id) {
        id -> Text,
        statement_id -> Text,
        human_id -> Text,
        vote -> Text,
        created_at -> Text,
        dht_anchor_hash -> Nullable<Text>,
    }
}

diesel::table! {
    hazards (id) {
        id -> Text,
        h_app_id -> Text,
        place_id -> Text,
        hazard_type -> Text,
        severity -> Text,
        title -> Text,
        description -> Text,
        reported_at -> Text,
        projected_onset -> Nullable<Text>,
        projected_end -> Nullable<Text>,
        actual_onset -> Nullable<Text>,
        resolved_at -> Nullable<Text>,
        affected_h3_cells -> Text,
        radius_km -> Nullable<Double>,
        source -> Text,
        source_reference -> Nullable<Text>,
        metadata_json -> Text,
        status -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    risk_alerts (id) {
        id -> Text,
        h_app_id -> Text,
        place_id -> Text,
        alert_type -> Text,
        severity -> Text,
        title -> Text,
        description -> Text,
        trigger_hazard_id -> Nullable<Text>,
        trigger_data_json -> Text,
        triggered_at -> Text,
        lead_time_hours -> Nullable<Double>,
        expires_at -> Nullable<Text>,
        status -> Text,
        acknowledged_by -> Nullable<Text>,
        acknowledged_at -> Nullable<Text>,
        resolved_at -> Nullable<Text>,
        escalated_to -> Nullable<Text>,
        metadata_json -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    token_mint_events (id) {
        id -> Text,
        h_app_id -> Text,
        amount -> Float,
        provenance_event_id -> Text,
        mint_tier -> Text,
        source_epr_id -> Text,
        agent_id -> Text,
        constitutional_context -> Nullable<Text>,
        elohim_attestation -> Nullable<Text>,
        reasoning_trace -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    token_balances (agent_id, h_app_id, governance_layer) {
        agent_id -> Text,
        h_app_id -> Text,
        governance_layer -> Text,
        balance -> Float,
        total_minted -> Float,
        total_transferred_in -> Float,
        total_transferred_out -> Float,
        last_activity_at -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    token_transfers (id) {
        id -> Text,
        h_app_id -> Text,
        from_agent -> Text,
        to_agent -> Text,
        amount -> Float,
        governance_layer -> Text,
        note -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    responsibility_demand_configs (id) {
        id -> Text,
        h_app_id -> Text,
        governance_layer -> Text,
        dignity_floor -> Float,
        median_estimate -> Float,
        soft_ceiling_multiplier -> Float,
        hard_ceiling_multiplier -> Float,
        social_contract_health -> Float,
        enforcement_active -> Integer,
        ratified_by -> Nullable<Text>,
        ratified_at -> Nullable<Text>,
        dht_anchor_hash -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    token_decay_events (id) {
        id -> Text,
        h_app_id -> Text,
        agent_id -> Text,
        governance_layer -> Text,
        balance_before -> Float,
        balance_after -> Float,
        decay_amount -> Float,
        obligation_level -> Text,
        dignity_floor -> Float,
        created_at -> Text,
    }
}

diesel::table! {
    observation_sessions (id) {
        id -> Text,
        started_at -> Text,
        ended_at -> Nullable<Text>,
        ttl_seconds -> Integer,
        source -> Text,
        metadata_json -> Nullable<Text>,
        report_content_id -> Nullable<Text>,
    }
}

diesel::table! {
    observation_entries (id) {
        id -> Integer,
        session_id -> Text,
        timestamp -> Text,
        origin -> Text,
        category -> Text,
        severity -> Text,
        method -> Nullable<Text>,
        path -> Nullable<Text>,
        status_code -> Nullable<Integer>,
        message -> Text,
        context_json -> Nullable<Text>,
    }
}

diesel::joinable!(observation_entries -> observation_sessions (session_id));

diesel::table! {
    placement_gaps (id) {
        id -> Text,
        content_id -> Text,
        shard_hash -> Text,
        h_app_id -> Text,
        requested_steward_count -> Integer,
        achieved_steward_count -> Integer,
        contract_coverage -> Float,
        gap_kind -> Text,
        first_seen_at -> Text,
        last_seen_at -> Text,
    }
}

// Recovery Protocol Phase 2 — DHT projection tables (imagodei DNA)
// Source of truth: DHT. These tables are read-optimized projections.

diesel::table! {
    recovery_requests (dht_anchor_hash) {
        dht_anchor_hash -> Text,
        human_agent_pubkey -> Text,
        new_agent_pubkey -> Text,
        hosting_doorway_pubkey -> Text,
        proposed_authority_kind -> Text,
        proposed_authority_json -> Text,
        request_nonce -> Binary,
        human_id -> Nullable<Text>,
        required_witness_count -> Integer,
        created_at -> Text,
    }
}

diesel::table! {
    key_rotations (dht_anchor_hash) {
        dht_anchor_hash -> Text,
        human_agent_pubkey -> Text,
        new_agent_pubkey -> Text,
        superseded_agent_pubkey -> Text,
        recovery_request_hash -> Text,
        authority_kind -> Text,
        authority_json -> Text,
        rotated_at -> Text,
    }
}

// Recovery Protocol Phase 2 — M3 witness projection.
// Source of truth: DHT. HumanityWitness entries (linked via
// RecoveryRequestToHumanityWitness) are the notary. This table is a
// read-optimized projection rebuildable from signal replay.
diesel::table! {
    recovery_witnesses (dht_anchor_hash) {
        dht_anchor_hash -> Text,
        recovery_request_hash -> Text,
        witness_agent_id -> Text,
        human_id -> Text,
        note -> Nullable<Text>,
        submitted_at -> Text,
    }
}

// EPR Phase 2B — peer_identity_bindings
// Source of truth: Holochain DHT (imagodei AgentPeerBinding entry — Task A.2).
// This table is a Category C operational projection rebuildable from signal replay.
// Timestamps stored as TEXT (ISO-8601) per elohim-storage conventions.

diesel::table! {
    peer_identity_bindings (peer_id, dht_anchor_hash) {
        peer_id         -> Text,
        agent_cid       -> Text,
        dht_anchor_hash -> Text,
        valid_from      -> Text,
        valid_until     -> Nullable<Text>,
        observed_at     -> Text,
        source          -> Text,
    }
}

// Recovery Protocol Phase 2 — M4 revocation projection tables.
// Source of truth: DHT (imagodei KeyRevocation + RevocationVote entries).
// These tables are read-optimized projections rebuildable via signal replay.

diesel::table! {
    key_revocations (dht_anchor_hash) {
        dht_anchor_hash   -> Text,
        id                -> Text,
        human_id          -> Text,
        revoked_key       -> Text,
        reason            -> Text,
        trigger_type      -> Text,
        initiated_by      -> Text,
        required_votes    -> Integer,
        current_votes     -> Integer,
        threshold_reached -> Integer,
        effective_at      -> Nullable<Text>,
        created_at        -> Text,
        updated_at        -> Text,
    }
}

diesel::table! {
    revocation_votes (dht_anchor_hash) {
        dht_anchor_hash            -> Text,
        id                         -> Text,
        revocation_dht_anchor_hash -> Text,
        revocation_id              -> Text,
        steward_id                 -> Text,
        approved                   -> Integer,
        attestation                -> Text,
        voted_at                   -> Text,
    }
}

// Recovery Protocol Phase 2 — M5 portal host projection (imagodei PortalHost entry)
// Source of truth: Holochain DHT (PortalHost entry in imagodei DNA, Category A).
diesel::table! {
    portal_hosts (rowid) {
        rowid               -> Integer,
        human_id            -> Text,
        host_url            -> Text,
        label               -> Nullable<Text>,
        added_at            -> Text,
        last_reachable_at   -> Nullable<Text>,
        reach               -> Text,
        dht_anchor_hash     -> Text,
    }
}

// EPR storage layer — Phase 2a
// Source of truth: EPR atoms (self-notarized via content-derived CID + Ed25519).
// Timestamps stored as TEXT (ISO-8601). Binary blobs stored as Binary/BLOB.

diesel::table! {
    epr_atoms (cid) {
        cid -> Text,
        kind -> Text,
        schema_ref -> Text,
        schema_key -> Text,
        reach -> Text,
        issued_at -> Text,
        signer_cid -> Text,
        supersedes -> Nullable<Text>,
        canonical_bytes -> Binary,
        payload_bytes -> Binary,
        proof_bytes -> Binary,
        proof_algorithm -> Text,
        /// UTC ISO-8601 timestamp at which resolver-backed Ed25519 verify succeeded.
        /// NULL for atoms ingested before A.7 or whose signer has no timeline entry.
        verified_at -> Nullable<Text>,
        /// blake3-128-prefix of the 32-byte ed25519 pubkey that signed this atom
        /// (first 16 bytes = 32 hex chars). NULL when verified_at is NULL.
        verified_signer_fingerprint -> Nullable<Text>,
    }
}

diesel::table! {
    epr_coupling (epr_cid, leg) {
        epr_cid -> Text,
        leg -> Text,
        target_cid -> Text,
    }
}

diesel::table! {
    epr_claims (epr_cid, claim_cid) {
        epr_cid -> Text,
        claim_cid -> Text,
    }
}

diesel::table! {
    epr_supersedence (predecessor, successor) {
        predecessor -> Text,
        successor -> Text,
        attested_by -> Text,
        attested_at -> Text,
    }
}

// EPR Phase 2B — B.3: projector cursor (Category C operational)
diesel::table! {
    projector_cursor (pillar, kind) {
        pillar -> Text,
        kind -> Text,
        last_epr_cid -> Nullable<Text>,
        last_issued_at -> Nullable<Text>,
        updated_at -> Text,
    }
}

diesel::joinable!(epr_coupling -> epr_atoms (epr_cid));
diesel::joinable!(epr_claims -> epr_atoms (epr_cid));

diesel::joinable!(statement_votes -> statements (statement_id));
diesel::joinable!(collective_participations -> collectives (collective_id));
diesel::joinable!(content_tags -> content (content_id));
diesel::joinable!(node_stewardship -> stewarded_nodes (node_id));
diesel::joinable!(node_stewardship -> humans (human_id));

diesel::allow_tables_to_appear_in_same_query!(
    access_grants,
    agreements,
    appeals,
    apps,
    challenge_outcomes,
    challenges,
    comments,
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
    enum_registry,
    epr_atoms,
    epr_claims,
    epr_coupling,
    epr_supersedence,
    gate_decision_attestations,
    gate_decision_challenges,
    governance_dispositions,
    governance_signals,
    governance_states,
    hazards,
    human_relationships,
    humans,
    imagodei_observations,
    knowledge_maps,
    local_sessions,
    node_stewardship,
    observation_entries,
    observation_sessions,
    peer_identity_bindings,
    peer_statuses,
    placement_gaps,
    portal_hosts,
    precedents,
    premium_gates,
    proposal_options,
    projector_cursor,
    proposals,
    ranked_votes,
    key_revocations,
    key_rotations,
    rea_commitments,
    recovery_requests,
    recovery_witnesses,
    relationships,
    revocation_votes,
    responsibility_demand_configs,
    risk_alerts,
    schema_version,
    schedules,
    shard_locations,
    shard_manifests,
    statement_votes,
    statements,
    steward_affinity,
    steward_credentials,
    stewarded_nodes,
    stewardship_allocations,
    token_balances,
    token_decay_events,
    token_mint_events,
    token_transfers,
    votes,
);
