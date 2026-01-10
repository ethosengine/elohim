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

diesel::joinable!(chapters -> paths (path_id));
diesel::joinable!(content_tags -> content (content_id));
diesel::joinable!(path_attestations -> paths (path_id));
diesel::joinable!(path_tags -> paths (path_id));
diesel::joinable!(steps -> chapters (chapter_id));
diesel::joinable!(steps -> paths (path_id));

diesel::allow_tables_to_appear_in_same_query!(
    apps,
    chapters,
    content,
    content_mastery,
    content_tags,
    contributor_presences,
    economic_events,
    human_relationships,
    path_attestations,
    path_tags,
    paths,
    relationships,
    schema_version,
    stewardship_allocations,
    steps,
);
