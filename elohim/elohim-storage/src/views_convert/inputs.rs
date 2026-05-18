//! Input Wire→DB converters.
//!
//! Converts InputView types (camelCase JSON from TypeScript clients) into
//! internal DB Input types (snake_case with String JSON fields).
//! All From impls that were authored directly against DB input structs
//! live here, organized by domain.

use elohim_views::shared::serialize_json_opt;
use elohim_views::{
    CreateAllocationInputView, CreateCollectiveInputView, CreateContributorPresenceInputView,
    CreateContentInputView, CreateEconomicEventInputView, CreateHumanRelationshipInputView,
    CreateMasteryInputView, CreateRelationshipInputView, CreateStewardedNodeInputView,
    InitiateClaimInputView, UpdateAllocationInputView,
};

use crate::db::collectives::CreateCollectiveInput;
use crate::db::content_diesel::CreateContentInput;
use crate::db::content_mastery::CreateMasteryInput;
use crate::db::contributor_presences::{CreateContributorPresenceInput, InitiateClaimInput};
use crate::db::economic_events::CreateEconomicEventInput;
use crate::db::human_relationships::CreateHumanRelationshipInput;
use crate::db::relationships_diesel::CreateRelationshipInput;
use crate::db::stewardship_allocations::{CreateAllocationInput, UpdateAllocationInput};
use crate::db::stewarded_nodes::CreateStewardedNodeInput;

// ============================================================================
// Content Input Views (lamad)
// ============================================================================

impl From<CreateContentInputView> for CreateContentInput {
    fn from(v: CreateContentInputView) -> Self {
        Self {
            id: v.id,
            title: v.title,
            description: v.description,
            content_type: v.content_type.unwrap_or_else(|| "concept".to_string()),
            content_format: v.content_format.unwrap_or_else(|| "markdown".to_string()),
            blob_hash: v.blob_hash,
            blob_cid: v.blob_cid,
            content_size_bytes: v.content_size_bytes.map(|s| s as i32),
            metadata_json: serialize_json_opt(&v.metadata),
            reach: v.reach.unwrap_or_else(|| "public".to_string()),
            created_by: v.created_by,
            tags: v.tags,
            content_body: v.content_body,
        }
    }
}

// ============================================================================
// Relationship Input Views (lamad)
// ============================================================================

impl From<CreateRelationshipInputView> for CreateRelationshipInput {
    fn from(v: CreateRelationshipInputView) -> Self {
        Self {
            id: v.id,
            source_id: v.source_id,
            target_id: v.target_id,
            relationship_type: v.relationship_type,
            confidence: v.confidence.unwrap_or(1.0) as f32,
            inference_source: v.inference_source.unwrap_or_else(|| "explicit".to_string()),
            is_bidirectional: false,
            provenance_chain_json: None,
            governance_layer: None,
            reach: "commons".to_string(),
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

// ============================================================================
// Human Relationship Input Views (imagodei)
// ============================================================================

impl From<CreateHumanRelationshipInputView> for CreateHumanRelationshipInput {
    fn from(v: CreateHumanRelationshipInputView) -> Self {
        Self {
            id: v.id,
            party_a_id: v.party_a_id,
            party_b_id: v.party_b_id,
            relationship_type: v.relationship_type,
            intimacy_level: v
                .intimacy_level
                .unwrap_or_else(|| "recognition".to_string()),
            is_bidirectional: v.is_bidirectional,
            consent_given_by_a: v.consent_given_by_a,
            consent_given_by_b: v.consent_given_by_b,
            initiated_by: v.initiated_by,
            governance_layer: v.governance_layer,
            reach: v.reach.unwrap_or_else(|| "private".to_string()),
            context_json: serialize_json_opt(&v.context),
            expires_at: v.expires_at,
        }
    }
}

// ============================================================================
// Contributor Presence Input Views (shefa)
// ============================================================================

impl From<CreateContributorPresenceInputView> for CreateContributorPresenceInput {
    fn from(v: CreateContributorPresenceInputView) -> Self {
        Self {
            id: v.id,
            display_name: v.display_name,
            external_identifiers_json: serialize_json_opt(&v.external_identifiers),
            establishing_content_ids: v.establishing_content_ids,
            image: v.image,
            note: v.note,
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

impl From<InitiateClaimInputView> for InitiateClaimInput {
    fn from(v: InitiateClaimInputView) -> Self {
        Self {
            claiming_agent_id: v.claiming_agent_id,
            verification_method: v.verification_method,
            evidence_json: serialize_json_opt(&v.evidence),
            facilitated_by: v.facilitated_by,
        }
    }
}

// ============================================================================
// Economic Event Input Views (shefa)
// ============================================================================

impl From<CreateEconomicEventInputView> for CreateEconomicEventInput {
    fn from(v: CreateEconomicEventInputView) -> Self {
        Self {
            id: v.id,
            action: v.action,
            provider: v.provider,
            receiver: v.receiver,
            resource_conforms_to: v.resource_conforms_to,
            resource_inventoried_as: v.resource_inventoried_as,
            resource_classified_as: v.resource_classified_as,
            resource_quantity_value: v.resource_quantity_value,
            resource_quantity_unit: v.resource_quantity_unit,
            effort_quantity_value: v.effort_quantity_value,
            effort_quantity_unit: v.effort_quantity_unit,
            has_point_in_time: v.has_point_in_time,
            has_duration: v.has_duration,
            input_of: v.input_of,
            output_of: v.output_of,
            lamad_event_type: v.lamad_event_type,
            content_id: v.content_id,
            contributor_presence_id: v.contributor_presence_id,
            path_id: v.path_id,
            triggered_by: v.triggered_by,
            note: v.note,
            metadata_json: serialize_json_opt(&v.metadata),
            at_location: v.at_location,
        }
    }
}

// ============================================================================
// Stewardship Allocation Input Views (shefa)
// ============================================================================

impl From<CreateAllocationInputView> for CreateAllocationInput {
    fn from(v: CreateAllocationInputView) -> Self {
        Self {
            content_id: v.content_id,
            steward_presence_id: v.steward_presence_id,
            allocation_ratio: v.allocation_ratio.unwrap_or(1.0),
            allocation_method: v.allocation_method.unwrap_or_else(|| "manual".to_string()),
            contribution_type: v
                .contribution_type
                .unwrap_or_else(|| "inherited".to_string()),
            contribution_evidence_json: serialize_json_opt(&v.contribution_evidence),
            note: v.note,
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

impl From<UpdateAllocationInputView> for UpdateAllocationInput {
    fn from(v: UpdateAllocationInputView) -> Self {
        Self {
            allocation_ratio: v.allocation_ratio,
            allocation_method: v.allocation_method,
            contribution_type: v.contribution_type,
            contribution_evidence_json: serialize_json_opt(&v.contribution_evidence),
            governance_state: v.governance_state,
            dispute_id: v.dispute_id,
            dispute_reason: v.dispute_reason,
            elohim_ratified_at: v.elohim_ratified_at,
            elohim_ratifier_id: v.elohim_ratifier_id,
            note: v.note,
        }
    }
}

// ============================================================================
// Content Mastery Input View (lamad)
// ============================================================================

impl From<CreateMasteryInputView> for CreateMasteryInput {
    fn from(v: CreateMasteryInputView) -> Self {
        Self {
            id: v.id,
            human_id: v.human_id,
            content_id: v.content_id,
            mastery_level: v.mastery_level.unwrap_or_else(|| "not_started".to_string()),
            content_version_at_mastery: v.content_version_at_mastery,
        }
    }
}

// ============================================================================
// Collective Input Views (qahal)
// ============================================================================

impl From<CreateCollectiveInputView> for CreateCollectiveInput {
    fn from(v: CreateCollectiveInputView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            description: v.description,
            governance_layer: v.governance_layer,
            constitutional_parent_id: v.constitutional_parent_id,
            reach: v.reach.unwrap_or_else(|| "community".to_string()),
            metadata_json: serialize_json_opt(&v.metadata),
            created_by: v.created_by,
        }
    }
}

// ============================================================================
// Stewarded Node Input Views (infrastructure/shefa)
// ============================================================================

impl From<CreateStewardedNodeInputView> for CreateStewardedNodeInput {
    fn from(v: CreateStewardedNodeInputView) -> Self {
        Self {
            id: v.id,
            display_name: v.display_name,
            claim_status: v.claim_status,
            cpu_cores: v.cpu_cores,
            memory_gb: v.memory_gb,
            storage_tb: v.storage_tb,
            bandwidth_mbps: v.bandwidth_mbps,
            steward_tier: v.steward_tier,
            custodian_opt_in: if v.custodian_opt_in { 1 } else { 0 },
            region: v.region,
            context_epr_id: v.context_epr_id,
            dht_anchor_hash: None,
            h_app_id: String::new(), // set by handler from AppContext
        }
    }
}

// ============================================================================
// Schema version tests (kept near the From impls they lint)
// ============================================================================

#[cfg(test)]
mod schema_version_tests {
    use elohim_views::{
        AccountPackageInputView, CreateAllocationInputView, CreateCollectiveInputView,
        CreateContributorPresenceInputView, CreateContentInputView, CreateEconomicEventInputView,
        CreateHumanRelationshipInputView, CreateMasteryInputView, CreateRelationshipInputView,
        InitiateClaimInputView, RecognitionTriggerInputView, UpdateAllocationInputView,
        UpdateContentInputView, UpsertPolicyInputView,
    };
    use elohim_views::shared::{
        default_schema_version, validate_schema_versions, SUPPORTED_SCHEMA_VERSIONS,
    };

    #[test]
    fn default_schema_version_is_one() {
        // Missing schemaVersion field defaults to 1
        let json = r#"{"id":"test","title":"Test"}"#;
        let view: CreateContentInputView = serde_json::from_str(json).unwrap();
        assert_eq!(view.schema_version, 1);
    }

    #[test]
    fn explicit_schema_version_is_preserved() {
        let json = r#"{"id":"test","title":"Test","schemaVersion":2}"#;
        let view: CreateContentInputView = serde_json::from_str(json).unwrap();
        assert_eq!(view.schema_version, 2);
    }

    #[test]
    fn unknown_fields_are_silently_ignored() {
        // Tolerant reader: future fields don't break deserialization
        let json = r#"{"id":"test","title":"Test","futureField":"ignored","anotherNew":42}"#;
        let view: CreateContentInputView = serde_json::from_str(json).unwrap();
        assert_eq!(view.id, "test");
        assert_eq!(view.schema_version, 1);
    }

    #[test]
    fn all_input_views_accept_schema_version() {
        // Verify schema_version works across representative InputView types
        let content: CreateContentInputView =
            serde_json::from_str(r#"{"id":"c","title":"T","schemaVersion":3}"#).unwrap();
        assert_eq!(content.schema_version, 3);

        let rel: CreateRelationshipInputView = serde_json::from_str(
            r#"{"sourceId":"a","targetId":"b","relationshipType":"relates","schemaVersion":2}"#,
        )
        .unwrap();
        assert_eq!(rel.schema_version, 2);

        let event: CreateEconomicEventInputView = serde_json::from_str(
            r#"{"action":"use","provider":"p","receiver":"r","schemaVersion":5}"#,
        )
        .unwrap();
        assert_eq!(event.schema_version, 5);
    }

    /// Compile-time lint: every InputView MUST have schema_version.
    /// If you add a new InputView struct without schema_version, this test
    /// will fail to compile. Add the field following the existing pattern:
    ///   #[serde(default = "default_schema_version")]
    ///   pub schema_version: u32,
    #[test]
    fn all_input_views_have_schema_version_field() {
        // Every InputView type must appear here. If you add a new one, add it below.
        let content: CreateContentInputView =
            serde_json::from_value(serde_json::json!({"id":"x","title":"x"})).unwrap();
        let rel: CreateRelationshipInputView = serde_json::from_value(
            serde_json::json!({"sourceId":"a","targetId":"b","relationshipType":"r"}),
        )
        .unwrap();
        let human_rel: CreateHumanRelationshipInputView = serde_json::from_value(
            serde_json::json!({"partyAId":"a","partyBId":"b","relationshipType":"r","initiatedBy":"a"})
        ).unwrap();
        let presence: CreateContributorPresenceInputView = serde_json::from_value(
            serde_json::json!({"displayName":"x","establishingContentIds":[]}),
        )
        .unwrap();
        let claim: InitiateClaimInputView = serde_json::from_value(
            serde_json::json!({"claimingAgentId":"a","verificationMethod":"m"}),
        )
        .unwrap();
        let event: CreateEconomicEventInputView = serde_json::from_value(
            serde_json::json!({"action":"use","provider":"p","receiver":"r"}),
        )
        .unwrap();
        let alloc: CreateAllocationInputView =
            serde_json::from_value(serde_json::json!({"contentId":"c","stewardPresenceId":"s"}))
                .unwrap();
        let update_alloc: UpdateAllocationInputView =
            serde_json::from_value(serde_json::json!({})).unwrap();
        let mastery: CreateMasteryInputView =
            serde_json::from_value(serde_json::json!({"humanId":"h","contentId":"c"})).unwrap();
        let account_pkg: AccountPackageInputView = serde_json::from_value(
            serde_json::json!({"identity":{"humanId":"h","displayName":"Test"}}),
        )
        .unwrap();
        let upsert_policy: UpsertPolicyInputView = serde_json::from_value(
            serde_json::json!({"contentRules":{"blockedCategories":[],"blockedHashes":[]},"timeRules":{},"featureRules":{}}),
        )
        .unwrap();

        // The lint: accessing .schema_version on each. Fails to compile if missing.
        assert_eq!(content.schema_version, 1);
        assert_eq!(rel.schema_version, 1);
        assert_eq!(human_rel.schema_version, 1);
        assert_eq!(presence.schema_version, 1);
        assert_eq!(claim.schema_version, 1);
        assert_eq!(event.schema_version, 1);
        assert_eq!(alloc.schema_version, 1);
        assert_eq!(update_alloc.schema_version, 1);
        assert_eq!(mastery.schema_version, 1);
        assert_eq!(account_pkg.schema_version, 1);
        assert_eq!(upsert_policy.schema_version, 1);
    }

    #[test]
    fn validate_supported_version_accepted() {
        assert!(validate_schema_versions(&[1]).is_ok());
    }

    #[test]
    fn validate_unsupported_version_rejected() {
        let err = validate_schema_versions(&[99]).unwrap_err();
        assert!(err.contains("Unsupported schema version: 99"));
        assert!(err.contains("Supported:"));
    }

    #[test]
    fn validate_empty_batch_ok() {
        assert!(validate_schema_versions(&[]).is_ok());
    }

    #[test]
    fn supported_versions_includes_default() {
        assert!(SUPPORTED_SCHEMA_VERSIONS.contains(&default_schema_version()));
    }

    #[test]
    fn recognition_trigger_input_deserializes_camel_case() {
        let json = r#"{"contentId":"c-1","eventType":"mastery_completion","rawAmount":10.0}"#;
        let view: RecognitionTriggerInputView = serde_json::from_str(json).unwrap();
        assert_eq!(view.content_id, "c-1");
        assert_eq!(view.event_type, "mastery_completion");
        assert!((view.raw_amount - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_update_content_input_view_deserializes_partial() {
        let json = r#"{"metadata": {"status": "done"}}"#;
        let view: UpdateContentInputView = serde_json::from_str(json).unwrap();
        assert!(view.title.is_none());
        assert!(view.tags.is_none());
        let meta = view.metadata.unwrap();
        assert_eq!(meta.0["status"], "done");
    }

    #[test]
    fn test_update_content_input_view_empty_patch_deserializes() {
        let json = r#"{}"#;
        let view: UpdateContentInputView = serde_json::from_str(json).unwrap();
        assert!(view.title.is_none());
        assert!(view.metadata.is_none());
    }

    #[test]
    fn collective_input_round_trips() {
        let json = r#"{"id":"c1","name":"Test Collective","governanceLayer":"community","reach":"commons","createdBy":"agent1"}"#;
        let view: CreateCollectiveInputView = serde_json::from_str(json).unwrap();
        assert_eq!(view.id, "c1");
        assert_eq!(view.name, "Test Collective");
    }
}
