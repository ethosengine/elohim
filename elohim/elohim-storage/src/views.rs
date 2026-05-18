//! View types for HTTP API boundary
//!
//! These types use camelCase serialization for TypeScript clients.
//! Wire types in models.rs use snake_case for database compatibility.
//!
//! Pattern:
//! - Service layer returns Wire types (Content, Relationship, etc.)
//! - HTTP layer converts to View types (ContentView, RelationshipView, etc.)
//! - ts-rs generates camelCase TypeScript from View types
//!
//! Design principles:
//! - Boolean coercion: SQLite stores bools as i32. Views expose proper bools.
//! - JSON parsing: Internal *_json strings are parsed to serde_json::Value.
//!   This encapsulates storage format and provides typed objects to clients.
//!
//! InputView types (suffix InputView):
//! - Accept camelCase JSON from TypeScript with parsed Value objects
//! - Convert to internal DB Input types (snake_case with String fields)
//! - Encapsulate JSON serialization at the API boundary

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
// ts_rs is used transitively via elohim-views; keep for any remaining #[derive(TS)] in this file
#[allow(unused_imports)]
use ts_rs::TS;

use crate::db::contributors::ImpactSummary;
use crate::db::models::{
    AccessGrant, AgreementRow, App, Appeal, Challenge, Content, ContentAttestation, ContentMastery,
    ContentStewardship, ContentWithTags, ContributorDashboard, ContributorPresence,
    CustodianMetrics, Discussion, EconomicEvent, GovernanceDisposition, GovernanceSignal,
    GovernanceState, Human, HumanRelationship, LocalSession, NodeStewardship, ObservationEntry,
    Precedent, PremiumGate, Proposal, ProposalOption, RankedVote, ReaCommitment, Relationship,
    RelationshipWithContent, Schedule, Statement, StatementVote, StewardCredential, StewardedNode,
    StewardshipAllocation, StewardshipAllocationWithPresence, TokenBalance, TokenMintEvent,
    TokenTransfer, Vote,
};
use crate::db::steward_operations::RevenueSummary;

// ============================================================================
// App View
// ============================================================================


impl From<App> for AppView {
    fn from(a: App) -> Self {
        Self {
            id: a.id,
            name: a.name,
            description: a.description,
            created_at: a.created_at,
            enabled: a.enabled == 1,
        }
    }
}

// ============================================================================
// Content Views
// ============================================================================


impl From<Content> for ContentView {
    fn from(c: Content) -> Self {
        Self {
            id: c.id,
            h_app_id: c.h_app_id,
            title: c.title,
            description: c.description,
            content_type: c.content_type,
            content_format: c.content_format,
            blob_hash: c.blob_hash,
            blob_cid: c.blob_cid,
            content_size_bytes: c.content_size_bytes,
            metadata: parse_json_opt(&c.metadata_json),
            reach: c.reach,
            validation_status: c.validation_status,
            created_by: c.created_by,
            created_at: c.created_at,
            updated_at: c.updated_at,
            content_body: c.content_body,
            dht_anchor_hash: c.dht_anchor_hash,
        }
    }
}

/// Convert ContentWithTags → ContentView (strips tags, for backward compat)
impl From<ContentWithTags> for ContentView {
    fn from(c: ContentWithTags) -> Self {
        c.content.into()
    }
}

    /// Construct a minimal ContentView from an EPR Head resolved via P2P.
    /// Provides enough data for the frontend to render content metadata
    /// while the full content body is fetched asynchronously.
pub fn content_view_from_epr_head(head: &crate::epr_codec::EprHead) -> ContentView {
        ContentView {
            id: head.id.clone(),
            h_app_id: "lamad".to_string(),
            title: head.lamad.title.clone(),
            description: head.lamad.description.clone(),
            content_type: head.lamad.content_type.clone(),
            content_format: head
                .lamad
                .content_format
                .clone()
                .unwrap_or_else(|| "markdown".to_string()),
            blob_hash: None,
            blob_cid: if head.content.is_empty() {
                None
            } else {
                Some(head.content.clone())
            },
            content_size_bytes: None,
            metadata: None,
            reach: head
                .qahal
                .reach
                .clone()
                .unwrap_or_else(|| "commons".to_string()),
            validation_status: "valid".to_string(),
            created_by: head.author.clone(),
            created_at: head
                .updated
                .clone()
                .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string()),
            updated_at: head
                .updated
                .clone()
                .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string()),
            content_body: None,
            dht_anchor_hash: None,
        }
    }



impl From<ContentWithTags> for ContentWithTagsView {
    fn from(c: ContentWithTags) -> Self {
        Self {
            content: c.content.into(),
            tags: c.tags,
        }
    }
}

// ============================================================================
// Relationship Views
// ============================================================================


impl From<Relationship> for RelationshipView {
    fn from(r: Relationship) -> Self {
        Self {
            id: r.id,
            h_app_id: r.h_app_id,
            source_id: r.source_id,
            target_id: r.target_id,
            relationship_type: r.relationship_type,
            confidence: r.confidence,
            inference_source: r.inference_source,
            is_bidirectional: r.is_bidirectional == 1,
            inverse_relationship_id: r.inverse_relationship_id,
            provenance_chain: parse_json_opt(&r.provenance_chain_json),
            governance_layer: r.governance_layer,
            reach: r.reach,
            metadata: parse_json_opt(&r.metadata_json),
            created_at: r.created_at,
            updated_at: r.updated_at,
            dht_anchor_hash: r.dht_anchor_hash,
        }
    }
}


impl From<RelationshipWithContent> for RelationshipWithContentView {
    fn from(r: RelationshipWithContent) -> Self {
        Self {
            relationship: r.relationship.into(),
            source: r.source.map(|c| c.into()),
            target: r.target.map(|c| c.into()),
        }
    }
}

// ============================================================================
// Human Relationship Views
// ============================================================================


impl From<HumanRelationship> for HumanRelationshipView {
    fn from(h: HumanRelationship) -> Self {
        Self {
            id: h.id,
            h_app_id: h.h_app_id,
            party_a_id: h.party_a_id,
            party_b_id: h.party_b_id,
            relationship_type: h.relationship_type,
            intimacy_level: h.intimacy_level,
            is_bidirectional: h.is_bidirectional == 1,
            consent_given_by_a: h.consent_given_by_a == 1,
            consent_given_by_b: h.consent_given_by_b == 1,
            custody_enabled_by_a: h.custody_enabled_by_a == 1,
            custody_enabled_by_b: h.custody_enabled_by_b == 1,
            auto_custody_enabled: h.auto_custody_enabled == 1,
            emergency_access_enabled: h.emergency_access_enabled == 1,
            initiated_by: h.initiated_by,
            verified_at: h.verified_at,
            governance_layer: h.governance_layer,
            reach: h.reach,
            context: parse_json_opt(&h.context_json),
            created_at: h.created_at,
            updated_at: h.updated_at,
            expires_at: h.expires_at,
            dht_anchor_hash: h.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Contributor Presence Views
// ============================================================================


impl From<ContributorPresence> for ContributorPresenceView {
    fn from(c: ContributorPresence) -> Self {
        Self {
            id: c.id,
            h_app_id: c.h_app_id,
            display_name: c.display_name,
            presence_state: c.presence_state,
            external_identifiers: parse_json_opt(&c.external_identifiers_json),
            establishing_content_ids: parse_json(&c.establishing_content_ids_json),
            affinity_total: c.affinity_total,
            unique_engagers: c.unique_engagers,
            citation_count: c.citation_count,
            recognition_score: c.recognition_score,
            recognition_by_content: parse_json_opt(&c.recognition_by_content_json),
            last_recognition_at: c.last_recognition_at,
            steward_id: c.steward_id,
            stewardship_started_at: c.stewardship_started_at,
            stewardship_commitment_id: c.stewardship_commitment_id,
            stewardship_quality_score: c.stewardship_quality_score,
            claim_initiated_at: c.claim_initiated_at,
            claim_verified_at: c.claim_verified_at,
            claim_verification_method: c.claim_verification_method,
            claim_evidence: parse_json_opt(&c.claim_evidence_json),
            claimed_agent_id: c.claimed_agent_id,
            claim_recognition_transferred_value: c.claim_recognition_transferred_value,
            claim_facilitated_by: c.claim_facilitated_by,
            image: c.image,
            note: c.note,
            metadata: parse_json_opt(&c.metadata_json),
            created_at: c.created_at,
            updated_at: c.updated_at,
            dht_anchor_hash: c.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Economic Event Views
// ============================================================================


impl From<EconomicEvent> for EconomicEventView {
    fn from(e: EconomicEvent) -> Self {
        Self {
            id: e.id,
            h_app_id: e.h_app_id,
            action: e.action,
            provider: e.provider,
            receiver: e.receiver,
            resource_conforms_to: e.resource_conforms_to,
            resource_inventoried_as: e.resource_inventoried_as,
            resource_classified_as: parse_json_opt(&e.resource_classified_as_json),
            resource_quantity_value: e.resource_quantity_value,
            resource_quantity_unit: e.resource_quantity_unit,
            effort_quantity_value: e.effort_quantity_value,
            effort_quantity_unit: e.effort_quantity_unit,
            has_point_in_time: e.has_point_in_time,
            has_duration: e.has_duration,
            input_of: e.input_of,
            output_of: e.output_of,
            lamad_event_type: e.lamad_event_type,
            content_id: e.content_id,
            contributor_presence_id: e.contributor_presence_id,
            path_id: e.path_id,
            triggered_by: e.triggered_by,
            state: e.state,
            note: e.note,
            metadata: parse_json_opt(&e.metadata_json),
            dht_anchor_hash: e.dht_anchor_hash,
            created_at: e.created_at,
            at_location: e.at_location,
        }
    }
}

// ============================================================================
// REA Commitment Views
// ============================================================================



impl From<ReaCommitment> for ReaCommitmentView {
    fn from(c: ReaCommitment) -> Self {
        Self {
            id: c.id,
            action: c.action,
            provider: c.provider,
            receiver: c.receiver,
            resource_conforms_to: c.resource_conforms_to,
            resource_classified_as: c
                .resource_classified_as
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            resource_quantity: match (c.resource_quantity_value, &c.resource_quantity_unit) {
                (Some(v), Some(u)) => Some(MeasureView {
                    has_numerical_value: v,
                    has_unit: u.clone(),
                }),
                _ => None,
            },
            effort_quantity: match (c.effort_quantity_value, &c.effort_quantity_unit) {
                (Some(v), Some(u)) => Some(MeasureView {
                    has_numerical_value: v,
                    has_unit: u.clone(),
                }),
                _ => None,
            },
            has_beginning: c.has_beginning,
            has_end: c.has_end,
            due: c.due,
            clause_of: c.clause_of,
            in_scope_of: c
                .in_scope_of
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            medium_of_exchange_id: c.medium_of_exchange_id,
            state: c.state,
            finished: c.finished != 0,
            note: c.note,
            metadata: parse_json_opt(&c.metadata_json),
            dht_anchor_hash: c.dht_anchor_hash,
            created_at: c.created_at,
        }
    }
}

// ============================================================================
// Content Mastery Views
// ============================================================================


impl From<ContentMastery> for ContentMasteryView {
    fn from(m: ContentMastery) -> Self {
        Self {
            id: m.id,
            h_app_id: m.h_app_id,
            human_id: m.human_id,
            content_id: m.content_id,
            mastery_level: m.mastery_level,
            mastery_level_index: m.mastery_level_index,
            freshness_score: m.freshness_score,
            needs_refresh: m.needs_refresh == 1,
            engagement_count: m.engagement_count,
            last_engagement_type: m.last_engagement_type,
            last_engagement_at: m.last_engagement_at,
            level_achieved_at: m.level_achieved_at,
            content_version_at_mastery: m.content_version_at_mastery,
            assessment_evidence: parse_json_opt(&m.assessment_evidence_json),
            privileges: parse_json_opt(&m.privileges_json),
            created_at: m.created_at,
            updated_at: m.updated_at,
            dht_anchor_hash: m.dht_anchor_hash,
        }
    }
}

// ============================================================================
// Stewardship Allocation Views
// ============================================================================


impl From<StewardshipAllocation> for StewardshipAllocationView {
    fn from(a: StewardshipAllocation) -> Self {
        Self {
            id: a.id,
            h_app_id: a.h_app_id,
            content_id: a.content_id,
            steward_presence_id: a.steward_presence_id,
            allocation_ratio: a.allocation_ratio,
            allocation_method: a.allocation_method,
            contribution_type: a.contribution_type,
            contribution_evidence: parse_json_opt(&a.contribution_evidence_json),
            governance_state: a.governance_state,
            dispute_id: a.dispute_id,
            dispute_reason: a.dispute_reason,
            disputed_at: a.disputed_at,
            disputed_by: a.disputed_by,
            negotiation_session_id: a.negotiation_session_id,
            elohim_ratified_at: a.elohim_ratified_at,
            elohim_ratifier_id: a.elohim_ratifier_id,
            effective_from: a.effective_from,
            effective_until: a.effective_until,
            superseded_by: a.superseded_by,
            recognition_accumulated: a.recognition_accumulated,
            last_recognition_at: a.last_recognition_at,
            note: a.note,
            metadata: parse_json_opt(&a.metadata_json),
            created_at: a.created_at,
            updated_at: a.updated_at,
            dht_anchor_hash: a.dht_anchor_hash,
        }
    }
}


impl From<StewardshipAllocationWithPresence> for StewardshipAllocationWithPresenceView {
    fn from(a: StewardshipAllocationWithPresence) -> Self {
        Self {
            allocation: a.allocation.into(),
            steward: a.steward.map(|s| s.into()),
        }
    }
}


impl From<ContentStewardship> for ContentStewardshipView {
    fn from(s: ContentStewardship) -> Self {
        Self {
            content_id: s.content_id,
            allocations: s.allocations.into_iter().map(|a| a.into()).collect(),
            total_allocation: s.total_allocation,
            has_disputes: s.has_disputes,
            primary_steward: s.primary_steward.map(|a| a.into()),
        }
    }
}

// ============================================================================
// Comment Views
// ============================================================================


impl From<crate::db::models::Comment> for CommentView {
    fn from(c: crate::db::models::Comment) -> Self {
        Self {
            id: c.id,
            content_id: c.content_id,
            human_id: c.human_id,
            body: c.body,
            reach: c.reach,
            governance_state: c.governance_state,
            created_at: c.created_at,
        }
    }
}


// ============================================================================
// Device Policy Views (Stewardship v5)
// ============================================================================

use crate::db::models::DevicePolicy;


impl From<DevicePolicy> for DevicePolicyView {
    fn from(p: DevicePolicy) -> Self {
        Self {
            id: p.id,
            subject_id: p.subject_id,
            device_id: p.device_id,
            author_id: p.author_id,
            author_tier: p.author_tier,
            inherits_from: p.inherits_from,
            blocked_categories: parse_json(&p.blocked_categories_json),
            blocked_hashes: parse_json(&p.blocked_hashes_json),
            age_rating_max: p.age_rating_max,
            reach_level_max: p.reach_level_max,
            session_max_minutes: p.session_max_minutes,
            daily_max_minutes: p.daily_max_minutes,
            time_windows: parse_json(&p.time_windows_json),
            cooldown_minutes: p.cooldown_minutes,
            disabled_features: parse_json(&p.disabled_features_json),
            disabled_routes: parse_json(&p.disabled_routes_json),
            require_approval: parse_json(&p.require_approval_json),
            log_sessions: p.log_sessions != 0,
            log_categories: p.log_categories != 0,
            log_policy_events: p.log_policy_events != 0,
            retention_days: p.retention_days,
            subject_can_view: p.subject_can_view != 0,
            effective_from: p.effective_from,
            effective_until: p.effective_until,
            version: p.version,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}



// ============================================================================
// Local Session Views
// ============================================================================


impl From<LocalSession> for LocalSessionView {
    fn from(s: LocalSession) -> Self {
        Self {
            id: s.id,
            human_id: s.human_id,
            agent_pub_key: s.agent_pub_key,
            doorway_url: s.doorway_url,
            doorway_id: s.doorway_id,
            identifier: s.identifier,
            display_name: s.display_name,
            profile_image_hash: s.profile_image_hash,
            is_active: s.is_active == 1,
            created_at: s.created_at,
            updated_at: s.updated_at,
            last_synced_at: s.last_synced_at,
            bootstrap_url: s.bootstrap_url,
        }
    }
}

// ============================================================================
// Input View Types (API boundary for writes)
// ============================================================================
//
// These types accept camelCase JSON from TypeScript clients with parsed Value
// objects. They convert to internal DB Input types which use snake_case with
// String fields. This encapsulates JSON serialization at the API boundary.


// ============================================================================
// Content Input Views
// ============================================================================

use crate::db::content_diesel::CreateContentInput;


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
// Relationship Input Views
// ============================================================================

use crate::db::relationships_diesel::CreateRelationshipInput;


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
// Human Relationship Input Views
// ============================================================================

use crate::db::human_relationships::CreateHumanRelationshipInput;


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
// Contributor Presence Input Views
// ============================================================================

use crate::db::contributor_presences::{CreateContributorPresenceInput, InitiateClaimInput};


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
// Economic Event Input Views
// ============================================================================

use crate::db::economic_events::CreateEconomicEventInput;


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
// Stewardship Allocation Input Views
// ============================================================================

use crate::db::stewardship_allocations::{CreateAllocationInput, UpdateAllocationInput};


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
// Device Policy Input Views
// ============================================================================







/// Convert UpsertPolicyInputView to DB input with author context.
pub fn upsert_policy_to_db_input(
    input_view: UpsertPolicyInputView,
    author_id: &str,
    author_tier: &str,
) -> crate::db::device_policies::CreateDevicePolicyInput {
    let this = input_view;
        let monitoring = this.monitoring_rules.unwrap_or(MonitoringRulesInput {
            log_sessions: false,
            log_categories: false,
            log_policy_events: true,
            retention_days: 30,
            subject_can_view: true,
        });
        crate::db::device_policies::CreateDevicePolicyInput {
            subject_id: this.subject_id.unwrap_or_default(),
            device_id: this.device_id,
            author_id: author_id.to_string(),
            author_tier: author_tier.to_string(),
            inherits_from: None,
            blocked_categories_json: serde_json::to_string(&this.content_rules.blocked_categories)
                .unwrap_or_else(|_| "[]".into()),
            blocked_hashes_json: serde_json::to_string(&this.content_rules.blocked_hashes)
                .unwrap_or_else(|_| "[]".into()),
            age_rating_max: this.content_rules.age_rating_max,
            reach_level_max: this.content_rules.reach_level_max,
            session_max_minutes: this.time_rules.session_max_minutes,
            daily_max_minutes: this.time_rules.daily_max_minutes,
            time_windows_json: serde_json::to_string(
                &this
                    .time_rules
                    .time_windows
                    .iter()
                    .map(|v| &v.0)
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".into()),
            cooldown_minutes: this.time_rules.cooldown_minutes,
            disabled_features_json: serde_json::to_string(&this.feature_rules.disabled_features)
                .unwrap_or_else(|_| "[]".into()),
            disabled_routes_json: serde_json::to_string(&this.feature_rules.disabled_routes)
                .unwrap_or_else(|_| "[]".into()),
            require_approval_json: serde_json::to_string(&this.feature_rules.require_approval)
                .unwrap_or_else(|_| "[]".into()),
            log_sessions: monitoring.log_sessions,
            log_categories: monitoring.log_categories,
            log_policy_events: monitoring.log_policy_events,
            retention_days: monitoring.retention_days,
            subject_can_view: monitoring.subject_can_view,
        }
}


// ============================================================================
// Content Mastery Input View
// ============================================================================

use crate::db::content_mastery::CreateMasteryInput;


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
// Token Views (Shefa economy — elohim-token sprint 1)
// ============================================================================


impl From<TokenMintEvent> for TokenMintEventView {
    fn from(m: TokenMintEvent) -> Self {
        Self {
            id: m.id,
            h_app_id: m.h_app_id,
            amount: m.amount,
            provenance_event_id: m.provenance_event_id,
            mint_tier: m.mint_tier,
            source_epr_id: m.source_epr_id,
            agent_id: m.agent_id,
            constitutional_context: m.constitutional_context,
            elohim_attestation: m.elohim_attestation,
            reasoning_trace: m.reasoning_trace,
            dht_anchor_hash: m.dht_anchor_hash,
            created_at: m.created_at,
        }
    }
}


impl From<TokenBalance> for TokenBalanceView {
    fn from(b: TokenBalance) -> Self {
        Self {
            agent_id: b.agent_id,
            h_app_id: b.h_app_id,
            governance_layer: b.governance_layer,
            balance: b.balance,
            total_minted: b.total_minted,
            total_transferred_in: b.total_transferred_in,
            total_transferred_out: b.total_transferred_out,
            last_activity_at: b.last_activity_at,
            created_at: b.created_at,
        }
    }
}


impl From<TokenTransfer> for TokenTransferView {
    fn from(t: TokenTransfer) -> Self {
        Self {
            id: t.id,
            h_app_id: t.h_app_id,
            from_agent: t.from_agent,
            to_agent: t.to_agent,
            amount: t.amount,
            governance_layer: t.governance_layer,
            note: t.note,
            dht_anchor_hash: t.dht_anchor_hash,
            created_at: t.created_at,
        }
    }
}



// ============================================================================
// Responsibility Demand Config Views (Shefa — elohim-token sprint 2)
// ============================================================================

use crate::db::models::ResponsibilityDemandConfig;


impl From<ResponsibilityDemandConfig> for ResponsibilityDemandConfigView {
    fn from(c: ResponsibilityDemandConfig) -> Self {
        Self {
            id: c.id,
            governance_layer: c.governance_layer,
            dignity_floor: c.dignity_floor,
            median_estimate: c.median_estimate,
            soft_ceiling_multiplier: c.soft_ceiling_multiplier,
            hard_ceiling_multiplier: c.hard_ceiling_multiplier,
            social_contract_health: c.social_contract_health,
            enforcement_active: c.enforcement_active != 0,
            ratified_by: c.ratified_by,
            ratified_at: c.ratified_at,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}


// ============================================================================
// Token Decay Event Views (Shefa — elohim-token sprint 3)
// ============================================================================

use crate::db::models::TokenDecayEvent;


impl From<TokenDecayEvent> for TokenDecayEventView {
    fn from(d: TokenDecayEvent) -> Self {
        Self {
            id: d.id,
            agent_id: d.agent_id,
            governance_layer: d.governance_layer,
            balance_before: d.balance_before,
            balance_after: d.balance_after,
            decay_amount: d.decay_amount,
            obligation_level: d.obligation_level,
            dignity_floor: d.dignity_floor,
            created_at: d.created_at,
        }
    }
}


// ============================================================================
// Collective Views (Qahal - Governance Contexts)
// ============================================================================

use crate::db::models::{Collective, CollectiveParticipation};


impl From<Collective> for CollectiveView {
    fn from(c: Collective) -> Self {
        Self {
            id: c.id,
            name: c.name,
            description: c.description,
            governance_layer: c.governance_layer,
            constitutional_parent_id: c.constitutional_parent_id,
            reach: c.reach,
            metadata: parse_json_opt(&c.metadata_json),
            created_by: c.created_by,
            created_at: c.created_at,
            updated_at: c.updated_at,
            dissolved_at: c.dissolved_at,
        }
    }
}


impl From<CreateCollectiveInputView> for crate::db::collectives::CreateCollectiveInput {
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


impl From<CollectiveParticipation> for CollectiveParticipationView {
    fn from(p: CollectiveParticipation) -> Self {
        Self {
            id: p.id,
            collective_id: p.collective_id,
            human_id: p.human_id,
            intimacy_level: p.intimacy_level,
            role_context: p.role_context,
            governance_weight: p.governance_weight,
            consent_state: p.consent_state,
            metadata: parse_json_opt(&p.metadata_json),
            joined_at: p.joined_at,
            updated_at: p.updated_at,
            departed_at: p.departed_at,
        }
    }
}

// ============================================================================
// Account Package Views (Import/Export)
// ============================================================================











// ============================================================================
// EPR Head Views
// ============================================================================

use crate::epr_codec::{
    EprHead, EprLamadContext, EprQahalContext, EprRelationship, EprShefaContext,
};

/// EPR Head input — accepts camelCase JSON from TypeScript clients.
/// Converts to `EprHead` for DAG-CBOR encoding.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprHeadInputView {
    pub version: Option<u32>,
    pub id: String,
    pub content: String,
    pub lamad: EprLamadContextInputView,
    pub shefa: Option<EprShefaContextInputView>,
    pub qahal: Option<EprQahalContextInputView>,
    #[serde(default)]
    pub relationships: Vec<EprRelationshipInputView>,
    pub author: Option<String>,
    pub updated: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprLamadContextInputView {
    pub title: String,
    pub content_type: String,
    pub description: Option<String>,
    pub content_format: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprShefaContextInputView {
    #[serde(default)]
    pub stewards: Vec<String>,
    #[serde(default)]
    pub allocations: Vec<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprQahalContextInputView {
    pub reach: Option<String>,
    pub layer: Option<String>,
    #[serde(default)]
    pub attestation_requirements: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprRelationshipInputView {
    #[serde(rename = "type")]
    pub rel_type: String,
    pub target: String,
    pub target_cid: Option<String>,
}

impl From<EprHeadInputView> for EprHead {
    fn from(v: EprHeadInputView) -> Self {
        Self {
            version: v.version.unwrap_or(1),
            id: v.id,
            content: v.content,
            lamad: EprLamadContext {
                title: v.lamad.title,
                content_type: v.lamad.content_type,
                description: v.lamad.description,
                content_format: v.lamad.content_format,
                tags: v.lamad.tags,
            },
            shefa: v.shefa.map_or_else(
                || EprShefaContext {
                    stewards: vec![],
                    allocations: vec![],
                },
                |s| EprShefaContext {
                    stewards: s.stewards,
                    allocations: s.allocations,
                },
            ),
            qahal: v.qahal.map_or_else(
                || EprQahalContext {
                    reach: None,
                    layer: None,
                    attestation_requirements: vec![],
                },
                |q| EprQahalContext {
                    reach: q.reach,
                    layer: q.layer,
                    attestation_requirements: q.attestation_requirements,
                },
            ),
            relationships: v
                .relationships
                .into_iter()
                .map(|r| EprRelationship {
                    rel_type: r.rel_type,
                    target: r.target,
                    target_cid: r.target_cid,
                })
                .collect(),
            author: v.author,
            updated: v.updated,
        }
    }
}

/// EPR Head response — camelCase output for TypeScript clients.
///
/// **Note on distribution**: this is a *response wrapper*, not the canonical
/// EPR Head. The canonical [`EprHead`] (in `epr_codec`) is the deterministic
/// IPLD document whose CID is derived from its bytes — operational fields
/// like `distribution` MUST NOT contaminate it. The DAG-CBOR encoding path in
/// `handle_get_epr_head` serializes the canonical struct, so distribution is
/// only ever surfaced via this JSON view.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EprHeadView {
    pub version: u32,
    pub id: String,
    pub content: String,
    pub lamad: EprLamadContext,
    pub shefa: EprShefaContext,
    pub qahal: EprQahalContext,
    pub relationships: Vec<EprRelationship>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    /// CID of the DAG-CBOR encoded head (set after encoding)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
    /// Inline distribution summary (Phase 5 T34). Hydrated by the HTTP handler
    /// from `compose_distribution_summary` over the content's blob_hash.
    /// `None` when the content row has no blob_hash yet (pre-distribution),
    /// or when summary composition failed (best-effort hydration — distribution
    /// surfacing must never break the head response).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distribution: Option<DistributionSummary>,
}

impl From<EprHead> for EprHeadView {
    fn from(h: EprHead) -> Self {
        Self {
            version: h.version,
            id: h.id,
            content: h.content,
            lamad: h.lamad,
            shefa: h.shefa,
            qahal: h.qahal,
            relationships: h.relationships,
            author: h.author,
            updated: h.updated,
            cid: None,
            distribution: None,
        }
    }
}

// ============================================================================
// Human Identity Views (imagodei pillar)
// ============================================================================


impl From<Human> for HumanView {
    fn from(h: Human) -> Self {
        let affinities: Vec<String> = serde_json::from_str(&h.affinities).unwrap_or_default();
        Self {
            id: h.id,
            agent_pub_key: h.agent_pub_key,
            display_name: h.display_name,
            bio: h.bio,
            affinities,
            profile_reach: h.profile_reach,
            location: h.location,
            profile_photo_url: h.profile_photo_url,
            h_app_id: h.h_app_id,
            created_at: h.created_at,
            updated_at: h.updated_at,
            dht_anchor_hash: h.dht_anchor_hash,
        }
    }
}




// ============================================================================
// Custodian Metrics Views
// ============================================================================










impl From<CustodianMetrics> for CustodianMetricsView {
    fn from(m: CustodianMetrics) -> Self {
        // Metric groups are stored as JSON blobs; parse or fall back to defaults.
        let health: CustodianHealthView =
            serde_json::from_str(&m.health_json).unwrap_or(CustodianHealthView {
                uptime_percent: 0.0,
                availability: false,
                response_time_p50_ms: 0.0,
                response_time_p95_ms: 0.0,
                response_time_p99_ms: 0.0,
                error_rate: 0.0,
                sla_compliance: false,
            });
        let storage: CustodianStorageMetricsView = serde_json::from_str(&m.storage_json)
            .unwrap_or_else(|_| CustodianStorageMetricsView {
                total_capacity_bytes: 0,
                used_bytes: 0,
                free_bytes: 0,
                utilization_percent: 0.0,
                by_domain: None,
                full_replica_bytes: 0,
                threshold_bytes: 0,
                erasure_coded_bytes: 0,
            });
        let bandwidth: CustodianBandwidthView = serde_json::from_str(&m.bandwidth_json)
            .unwrap_or_else(|_| CustodianBandwidthView {
                declared_mbps: 0.0,
                current_usage_mbps: 0.0,
                peak_usage_mbps: 0.0,
                average_usage_mbps: 0.0,
                utilization_percent: 0.0,
                inbound_mbps: 0.0,
                outbound_mbps: 0.0,
                by_domain: None,
            });
        let computation: CustodianComputationView = serde_json::from_str(&m.computation_json)
            .unwrap_or(CustodianComputationView {
                cpu_cores: 0,
                cpu_usage_percent: 0.0,
                memory_gb: 0.0,
                memory_usage_percent: 0.0,
                zome_ops_per_second: 0.0,
                reconstruction_workload_percent: 0.0,
            });
        let reputation: CustodianReputationView = serde_json::from_str(&m.reputation_json)
            .unwrap_or(CustodianReputationView {
                reliability_rating: 0.0,
                speed_rating: 0.0,
                reputation_score: 0.0,
                specialization_bonus: 0.0,
                commitment_fulfillment: 0.0,
            });
        let economic: CustodianEconomicView =
            serde_json::from_str(&m.economic_json).unwrap_or(CustodianEconomicView {
                steward_tier: 0,
                price_per_gb: 0.0,
                monthly_earnings: 0.0,
                lifetime_earnings: 0.0,
                active_commitments: 0,
                total_committed_bytes: 0,
            });
        Self {
            custodian_id: m.custodian_id,
            tier: m.tier as u32,
            health,
            storage,
            bandwidth,
            computation,
            reputation,
            economic,
            collected_at: m.collected_at,
            last_updated_at: m.last_updated_at,
        }
    }
}


/// Convert ReportCustodianMetricsInputView to the insertable DB type.
pub fn report_custodian_metrics_into_upsert(
    view: ReportCustodianMetricsInputView,
    h_app_id: impl Into<String>,
    now_ms: i64,
) -> crate::db::models::UpsertCustodianMetrics {
        crate::db::models::UpsertCustodianMetrics {
            custodian_id: view.custodian_id,
            h_app_id: h_app_id.into(),
            tier: view.tier as i32,
            health_json: serde_json::to_string(&view.health).unwrap_or_default(),
            storage_json: serde_json::to_string(&view.storage).unwrap_or_default(),
            bandwidth_json: serde_json::to_string(&view.bandwidth).unwrap_or_default(),
            computation_json: serde_json::to_string(&view.computation).unwrap_or_default(),
            reputation_json: serde_json::to_string(&view.reputation).unwrap_or_default(),
            economic_json: serde_json::to_string(&view.economic).unwrap_or_default(),
            collected_at: view.collected_at.unwrap_or(now_ms),
            last_updated_at: now_ms,
        }
    }


// ============================================================================
// Data Protection Views
//
// Read-only aggregation views — assembled from custodian commitment data
// and DHT queries. No dedicated DB tables required.
// ============================================================================





// ============================================================================
// Shefa Dashboard Views
//
// Read-only aggregation views assembled from multiple sources by the
// compute handler. No dedicated DB tables.
// ============================================================================





























// ============================================================================
// Governance Views
// ============================================================================


impl From<GovernanceState> for GovernanceStateView {
    fn from(g: GovernanceState) -> Self {
        Self {
            id: g.id,
            entity_type: g.entity_type,
            entity_id: g.entity_id,
            reach: g.reach,
            labels: parse_json(&g.labels),
            voting_state: g.voting_state,
            signal_count: g.signal_count,
            created_at: g.created_at,
            updated_at: g.updated_at,
            dht_anchor_hash: g.dht_anchor_hash,
            status: None,
            status_basis: None,
            active_challenges: None,
            active_proposals: None,
            precedent_ids: None,
            last_updated: None,
            metadata: None,
        }
    }
}


impl From<Challenge> for ChallengeView {
    fn from(c: Challenge) -> Self {
        Self {
            id: c.id,
            entity_type: c.entity_type,
            entity_id: c.entity_id,
            challenger_id: c.challenger_id,
            standing_basis: c.standing_basis,
            grounds_primary: c.grounds_primary,
            grounds_secondary: c.grounds_secondary,
            evidence: parse_json(&c.evidence),
            requested_outcome: c.requested_outcome,
            state: c.state,
            response_outcome: c.response_outcome,
            response_reasoning: c.response_reasoning,
            response_actions: c.response_actions,
            response_by: c.response_by,
            sets_precedent: c.sets_precedent != 0,
            filed_at: c.filed_at,
            acknowledged_at: c.acknowledged_at,
            response_deadline: c.response_deadline,
            responded_at: c.responded_at,
            resolved_at: c.resolved_at,
            created_at: c.created_at,
            sla_status: String::new(),
            dht_anchor_hash: c.dht_anchor_hash,
            challenger_name: None,
            challenger_standing: None,
            grounds: None,
            description: None,
            status: None,
            priority: None,
            sla_deadline: None,
            assigned_elohim: None,
            resolution: None,
            updated_at: None,
            metadata: None,
        }
    }
}


impl From<Appeal> for AppealView {
    fn from(a: Appeal) -> Self {
        Self {
            id: a.id,
            challenge_id: a.challenge_id,
            appellant_id: a.appellant_id,
            grounds: a.grounds,
            additional_evidence: a.additional_evidence,
            state: a.state,
            escalation_level: a.escalation_level,
            decision: a.decision,
            decision_reasoning: a.decision_reasoning,
            decided_by: a.decided_by,
            filed_at: a.filed_at,
            decided_at: a.decided_at,
            created_at: a.created_at,
            dht_anchor_hash: a.dht_anchor_hash,
        }
    }
}





impl From<Proposal> for ProposalView {
    fn from(p: Proposal) -> Self {
        Self {
            id: p.id,
            content_id: p.content_id,
            proposer_presence_id: p.proposer_presence_id,
            proposal_type: p.proposal_type,
            title: p.title,
            body: p.body,
            status: p.status,
            votes_for: p.votes_for,
            votes_against: p.votes_against,
            voting_anonymous: p.voting_anonymous == 1,
            created_at: p.created_at,
            updated_at: p.updated_at,
            voting_mechanism: p.voting_mechanism,
            score_min: p.score_min,
            score_max: p.score_max,
            dots_per_voter: p.dots_per_voter,
            quorum_percentage: p.quorum_percentage,
            passage_threshold: p.passage_threshold,
            dht_anchor_hash: p.dht_anchor_hash,
            description: None,
            proposer_id: None,
            proposer_name: None,
            rationale: None,
            phase: None,
            amendments: None,
            voting_config: None,
            current_votes: None,
            outcome: None,
            related_entity_type: None,
            related_entity_id: None,
            metadata: None,
        }
    }
}


impl From<Precedent> for PrecedentView {
    fn from(p: Precedent) -> Self {
        Self {
            id: p.id,
            content_id: p.content_id,
            principle: p.principle,
            interpretation: p.interpretation,
            established_by: p.established_by,
            created_at: p.created_at,
            dht_anchor_hash: p.dht_anchor_hash,
            title: None,
            summary: None,
            full_reasoning: None,
            binding: None,
            scope: None,
            citations: None,
            status: None,
            established_at: None,
            superseded_by: None,
            updated_at: None,
            metadata: None,
        }
    }
}


impl From<Discussion> for DiscussionView {
    fn from(d: Discussion) -> Self {
        Self {
            id: d.id,
            content_id: d.content_id,
            author_presence_id: d.author_presence_id,
            body: d.body,
            parent_id: d.parent_id,
            created_at: d.created_at,
            updated_at: d.updated_at,
            entity_type: None,
            entity_id: None,
            category: None,
            title: None,
            messages: None,
            status: None,
            message_count: None,
            last_activity_at: None,
            metadata: None,
        }
    }
}





// ============================================================================
// Proposal Option Views (multi-mechanism voting)
// ============================================================================


impl From<ProposalOption> for ProposalOptionView {
    fn from(o: ProposalOption) -> Self {
        Self {
            id: o.id,
            proposal_id: o.proposal_id,
            label: o.label,
            description: o.description,
            position: o.position,
            source: o.source,
            source_justification: o.source_justification,
            created_at: o.created_at,
            dht_anchor_hash: o.dht_anchor_hash,
        }
    }
}


// ============================================================================
// Ranked Vote Views (multi-mechanism voting)
// ============================================================================




// ============================================================================
// Governance Signal Views
// ============================================================================


impl From<GovernanceSignal> for GovernanceSignalView {
    fn from(s: GovernanceSignal) -> Self {
        Self {
            id: s.id,
            entity_type: s.entity_type,
            entity_id: s.entity_id,
            human_id: s.human_id,
            signal_type: s.signal_type,
            signal_value: s.signal_value,
            mechanism_level: s.mechanism_level,
            proxy_elohim_id: s.proxy_elohim_id,
            created_at: s.created_at,
            dht_anchor_hash: s.dht_anchor_hash,
        }
    }
}



// ============================================================================
// Governance Disposition Views
// ============================================================================


impl From<GovernanceDisposition> for GovernanceDispositionView {
    fn from(d: GovernanceDisposition) -> Self {
        Self {
            id: d.id,
            human_id: d.human_id,
            risk_tolerance: d.risk_tolerance as f64,
            change_openness: d.change_openness as f64,
            consensus_preference: d.consensus_preference as f64,
            priority_values: parse_json(&d.priority_values),
            voting_pattern_summary: parse_json(&d.voting_pattern_summary),
            total_votes_cast: d.total_votes_cast,
            total_challenges_filed: d.total_challenges_filed,
            total_signals_recorded: d.total_signals_recorded,
            dht_anchor_hash: d.dht_anchor_hash,
            last_computed_at: d.last_computed_at,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}




// ============================================================================
// Governance Reaction Views (from qahal-types::GovernanceReaction)
// ============================================================================


// ============================================================================
// Graduated Feedback Views (from qahal-types::GraduatedFeedback)
// ============================================================================


// ============================================================================
// Attestation Views
// ============================================================================


impl From<ContentAttestation> for ContentAttestationView {
    fn from(c: ContentAttestation) -> Self {
        Self {
            id: c.id,
            content_id: c.content_id,
            attestor_presence_id: c.attestor_presence_id,
            scope: c.scope,
            attestation_type: c.attestation_type,
            evidence: parse_json_opt(&c.evidence),
            grantor: parse_json_opt(&c.grantor),
            is_revoked: c.is_revoked == 1,
            revocation: parse_json_opt(&c.revocation),
            created_at: c.created_at,
            updated_at: c.updated_at,
            dht_anchor_hash: c.dht_anchor_hash,
        }
    }
}



// ============================================================================
// Steward Views
// ============================================================================


impl From<StewardCredential> for StewardCredentialView {
    fn from(s: StewardCredential) -> Self {
        Self {
            id: s.id,
            presence_id: s.presence_id,
            content_id: s.content_id,
            affinity_coefficient: s.affinity_coefficient,
            credential_type: s.credential_type,
            status: s.status,
            dht_anchor_hash: s.dht_anchor_hash,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}


impl From<PremiumGate> for PremiumGateView {
    fn from(g: PremiumGate) -> Self {
        Self {
            id: g.id,
            steward_credential_id: g.steward_credential_id,
            steward_presence_id: g.steward_presence_id,
            gated_resource_type: g.gated_resource_type,
            gated_resource_ids: parse_json(&g.gated_resource_ids),
            gate_title: g.gate_title,
            gate_description: g.gate_description,
            dht_anchor_hash: g.dht_anchor_hash,
            created_at: g.created_at,
        }
    }
}


impl From<AccessGrant> for AccessGrantView {
    fn from(a: AccessGrant) -> Self {
        Self {
            id: a.id,
            gate_id: a.gate_id,
            grantee_presence_id: a.grantee_presence_id,
            contributor_presence_id: a.contributor_presence_id,
            granted_at: a.granted_at,
            expires_at: a.expires_at,
            status: a.status,
            dht_anchor_hash: a.dht_anchor_hash,
        }
    }
}


impl From<RevenueSummary> for StewardRevenueSummaryView {
    fn from(r: RevenueSummary) -> Self {
        Self {
            total_credentials: r.total_credentials,
            total_gates: r.total_gates,
            total_grants: r.total_grants,
        }
    }
}




// ============================================================================
// Contributor Views
// ============================================================================


impl From<ContributorDashboard> for ContributorDashboardView {
    fn from(d: ContributorDashboard) -> Self {
        Self {
            presence_id: d.presence_id,
            total_contributions: d.total_contributions,
            total_recognitions: d.total_recognitions,
            impact_score: d.impact_score,
            last_contribution_at: d.last_contribution_at,
            updated_at: d.updated_at,
        }
    }
}


impl From<ImpactSummary> for ContributorImpactView {
    fn from(i: ImpactSummary) -> Self {
        Self {
            presence_id: i.presence_id,
            total_events: i.total_events,
            unique_content_ids: i.unique_content_ids,
        }
    }
}


// ============================================================================
// REA Commitment Input Views
// ============================================================================

use crate::db::rea_commitments::{CreateReaCommitmentInput, UpdateReaCommitmentState};


impl From<CreateReaCommitmentInputView> for CreateReaCommitmentInput {
    fn from(v: CreateReaCommitmentInputView) -> Self {
        Self {
            id: v.id,
            action: v.action,
            provider: v.provider,
            receiver: v.receiver,
            resource_conforms_to: v.resource_conforms_to,
            resource_classified_as: v
                .resource_classified_as
                .map(|v| serde_json::to_string(&v).unwrap_or_default()),
            resource_quantity_value: v.resource_quantity.as_ref().map(|m| m.has_numerical_value),
            resource_quantity_unit: v.resource_quantity.map(|m| m.has_unit),
            effort_quantity_value: v.effort_quantity.as_ref().map(|m| m.has_numerical_value),
            effort_quantity_unit: v.effort_quantity.map(|m| m.has_unit),
            has_beginning: v.has_beginning,
            has_end: v.has_end,
            due: v.due,
            clause_of: v.clause_of,
            in_scope_of: v
                .in_scope_of
                .map(|v| serde_json::to_string(&v).unwrap_or_default()),
            medium_of_exchange_id: v.medium_of_exchange_id,
            note: v.note,
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}


impl From<UpdateReaCommitmentStateView> for UpdateReaCommitmentState {
    fn from(v: UpdateReaCommitmentStateView) -> Self {
        Self {
            state: v.state,
            finished: v.finished,
        }
    }
}

// ============================================================================
// Agreement Views
// ============================================================================


impl From<AgreementRow> for AgreementView {
    fn from(a: AgreementRow) -> Self {
        Self {
            id: a.id,
            name: a.name,
            note: a.note,
            dht_anchor_hash: a.dht_anchor_hash,
            metadata: parse_json_opt(&a.metadata_json),
            created_at: a.created_at,
        }
    }
}


impl From<CreateAgreementInputView> for crate::db::agreements::CreateAgreementInput {
    fn from(v: CreateAgreementInputView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            note: v.note,
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

// ============================================================================
// Stewarded Node Views
// ============================================================================


impl From<StewardedNode> for StewardedNodeView {
    fn from(n: StewardedNode) -> Self {
        Self {
            id: n.id,
            display_name: n.display_name,
            claim_status: n.claim_status,
            cpu_cores: n.cpu_cores,
            memory_gb: n.memory_gb,
            storage_tb: n.storage_tb,
            bandwidth_mbps: n.bandwidth_mbps,
            steward_tier: n.steward_tier,
            custodian_opt_in: n.custodian_opt_in == 1,
            region: n.region,
            context_epr_id: n.context_epr_id,
            dht_anchor_hash: n.dht_anchor_hash,
            created_at: n.created_at,
            updated_at: n.updated_at,
            stewards: vec![],
        }
    }
}






impl From<CreateStewardedNodeInputView> for crate::db::stewarded_nodes::CreateStewardedNodeInput {
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


impl From<CreateNodeStewardshipInputView>
    for crate::db::stewarded_nodes::CreateNodeStewardshipInput
{
    fn from(v: CreateNodeStewardshipInputView) -> Self {
        Self {
            node_id: v.node_id,
            human_id: v.human_id,
            affinity_score: v.affinity_score,
            relationship: v.relationship,
            context_epr_id: v.context_epr_id,
        }
    }
}

// ============================================================================
// Recognition Pipeline Views
// ============================================================================


impl From<RecognitionTriggerInputView>
    for crate::services::recognition_pipeline_service::RecognitionTrigger
{
    fn from(v: RecognitionTriggerInputView) -> Self {
        Self {
            content_id: v.content_id,
            event_type: v.event_type,
            raw_amount: v.raw_amount,
            triggered_by: v.triggered_by,
        }
    }
}


impl From<crate::services::recognition_pipeline_service::StageTrace> for StageTraceView {
    fn from(t: crate::services::recognition_pipeline_service::StageTrace) -> Self {
        Self {
            steward_presence_id: t.steward_presence_id,
            allocation_ratio: t.allocation_ratio,
            stored_affinity: t.stored_affinity,
            derived_affinity: t.derived_affinity,
            effective_ratio: t.effective_ratio,
            pre_limit_share: t.pre_limit_share,
            final_share: t.final_share,
            limit_reasons: t
                .limit_reasons
                .iter()
                .map(|r| JsonVal(serde_json::to_value(r).unwrap_or_default()))
                .collect(),
            economic_event_id: t.economic_event_id.unwrap_or_default(),
        }
    }
}


impl From<crate::services::recognition_pipeline_service::RecognitionDistributionResult>
    for RecognitionDistributionResultView
{
    fn from(
        r: crate::services::recognition_pipeline_service::RecognitionDistributionResult,
    ) -> Self {
        Self {
            content_id: r.content_id,
            trigger_event_type: r.trigger_event_type,
            raw_amount: r.raw_amount,
            weighted_amount: r.weighted_amount,
            distributions: r
                .distributions
                .into_iter()
                .map(StageTraceView::from)
                .collect(),
            economic_event_ids: r.economic_event_ids,
            limits_applied: r
                .limits_applied
                .iter()
                .map(|l| JsonVal(serde_json::to_value(l).unwrap_or_default()))
                .collect(),
        }
    }
}

// ============================================================================
// Steward Affinity Views
// ============================================================================


impl From<crate::db::models::StewardAffinity> for StewardAffinityView {
    fn from(a: crate::db::models::StewardAffinity) -> Self {
        Self {
            id: a.id,
            steward_id: a.steward_id,
            content_id: a.content_id,
            affinity_score: a.affinity_score,
            source: a.source,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}


impl From<CreateStewardAffinityInputView> for crate::db::steward_affinity::CreateAffinityInput {
    fn from(v: CreateStewardAffinityInputView) -> Self {
        Self {
            steward_id: v.steward_id,
            content_id: v.content_id,
            affinity_score: v.affinity_score,
            source: v.source.unwrap_or_else(|| "genesis_seed".to_string()),
        }
    }
}



// ============================================================================
// ElohimGate Views
// ============================================================================





// ============================================================================
// Sensemaking Statement Views
// ============================================================================


impl From<Statement> for StatementView {
    fn from(s: Statement) -> Self {
        Self {
            id: s.id,
            entity_type: s.entity_type,
            entity_id: s.entity_id,
            human_id: s.human_id,
            text: s.text,
            agree_count: s.agree_count,
            disagree_count: s.disagree_count,
            pass_count: s.pass_count,
            group_id: s.group_id,
            is_bridging: s.is_bridging != 0,
            created_at: s.created_at,
            dht_anchor_hash: s.dht_anchor_hash,
            context_id: None,
            author_id: None,
            status: None,
            vote_count: None,
            consensus_score: None,
            cluster: None,
            updated_at: None,
            metadata: None,
        }
    }
}


impl From<StatementVote> for StatementVoteView {
    fn from(v: StatementVote) -> Self {
        Self {
            id: v.id,
            statement_id: v.statement_id,
            human_id: v.human_id,
            vote: v.vote,
            created_at: v.created_at,
            dht_anchor_hash: v.dht_anchor_hash,
            voter_id: None,
            metadata: None,
        }
    }
}







// ============================================================================
// Schedule Views (Kairos temporal dimension)
// ============================================================================


impl From<Schedule> for ScheduleView {
    fn from(s: Schedule) -> Self {
        Self {
            id: s.id,
            entity_type: s.entity_type,
            entity_id: s.entity_id,
            scheduled_at: s.scheduled_at,
            expires_at: s.expires_at,
            rrule: s.rrule,
            next_occurrence_at: s.next_occurrence_at,
            occurrence_count: s.occurrence_count,
            created_at: s.created_at,
        }
    }
}



// ============================================================================
// Spatial Context Views
// ============================================================================


impl From<crate::db::models::SpatialContext> for SpatialContextView {
    fn from(s: crate::db::models::SpatialContext) -> Self {
        Self {
            id: s.id,
            entity_type: s.entity_type,
            entity_id: s.entity_id,
            latitude: s.latitude,
            longitude: s.longitude,
            altitude: s.altitude,
            accuracy: s.accuracy,
            h3_res5: s.h3_res5,
            h3_res7: s.h3_res7,
            h3_res9: s.h3_res9,
            place_id: s.place_id,
            osm_type: s.osm_type,
            osm_id: s.osm_id,
            label: s.label,
            context_type: s.context_type,
            geometry_json: parse_json_opt(&s.geometry_json),
            metadata: parse_json_opt(&s.metadata_json),
            observed_at: s.observed_at,
            created_at: s.created_at,
            updated_at: s.updated_at,
            is_current: s.is_current == 1,
        }
    }
}



// ============================================================================
// Place Views (governed spatial entity — DHT projection)
// ============================================================================


impl From<crate::db::models::Place> for PlaceView {
    fn from(p: crate::db::models::Place) -> Self {
        Self {
            id: p.id,
            dht_anchor_hash: p.dht_anchor_hash,
            name: p.name,
            place_type: p.place_type,
            constitutional_layer: p.constitutional_layer,
            h3_index: p.h3_index,
            h3_resolution: p.h3_resolution,
            geometry_json: Some(parse_json(&p.geometry_json)),
            centroid_lat: p.centroid_lat,
            centroid_lng: p.centroid_lng,
            parent_place_id: p.parent_place_id,
            osm_reference: parse_json_opt(&p.osm_reference_json),
            carrying_capacity: Some(parse_json(&p.carrying_capacity_json)),
            governing_collective_id: p.governing_collective_id,
            status: p.status,
            created_by: p.created_by,
            created_at: p.created_at,
            updated_at: p.updated_at,
            metadata: Some(parse_json(&p.metadata_json)),
        }
    }
}


// ============================================================================
// Hazard Views (Sprint 7 — Risk + Resilience Mapping)
// ============================================================================

use crate::db::models::{Hazard, RiskAlert};


impl From<Hazard> for HazardView {
    fn from(h: Hazard) -> Self {
        Self {
            id: h.id,
            h_app_id: h.h_app_id,
            place_id: h.place_id,
            hazard_type: h.hazard_type,
            severity: h.severity,
            title: h.title,
            description: h.description,
            reported_at: h.reported_at,
            projected_onset: h.projected_onset,
            projected_end: h.projected_end,
            actual_onset: h.actual_onset,
            resolved_at: h.resolved_at,
            affected_h3_cells: parse_json(&h.affected_h3_cells),
            radius_km: h.radius_km,
            source: h.source,
            source_reference: h.source_reference,
            metadata: parse_json_opt(&Some(h.metadata_json)),
            status: h.status,
            created_at: h.created_at,
            updated_at: h.updated_at,
        }
    }
}



// ============================================================================
// RiskAlert Views (Sprint 7 — Risk + Resilience Mapping)
// ============================================================================


impl From<RiskAlert> for RiskAlertView {
    fn from(r: RiskAlert) -> Self {
        Self {
            id: r.id,
            h_app_id: r.h_app_id,
            place_id: r.place_id,
            alert_type: r.alert_type,
            severity: r.severity,
            title: r.title,
            description: r.description,
            trigger_hazard_id: r.trigger_hazard_id,
            trigger_data: parse_json_opt(&Some(r.trigger_data_json)),
            triggered_at: r.triggered_at,
            lead_time_hours: r.lead_time_hours,
            expires_at: r.expires_at,
            status: r.status,
            acknowledged_by: r.acknowledged_by,
            acknowledged_at: r.acknowledged_at,
            resolved_at: r.resolved_at,
            escalated_to: r.escalated_to,
            metadata: parse_json_opt(&Some(r.metadata_json)),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}


// ============================================================================
// Schema Version Tests
// ============================================================================

// ============================================================================
// Spatial Dashboard Views (Sprint 8 — Planet-Scale Governance Dashboard)
// ============================================================================











// ============================================================================

#[cfg(test)]
mod schema_version_tests {
    use super::*;

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
        assert!(super::validate_schema_versions(&[1]).is_ok());
    }

    #[test]
    fn validate_unsupported_version_rejected() {
        let err = super::validate_schema_versions(&[99]).unwrap_err();
        assert!(err.contains("Unsupported schema version: 99"));
        assert!(err.contains("Supported:"));
    }

    #[test]
    fn validate_empty_batch_ok() {
        assert!(super::validate_schema_versions(&[]).is_ok());
    }

    #[test]
    fn supported_versions_includes_default() {
        assert!(super::SUPPORTED_SCHEMA_VERSIONS.contains(&super::default_schema_version()));
    }

    #[test]
    fn recognition_trigger_input_deserializes_camel_case() {
        let json = r#"{"contentId":"c-1","eventType":"mastery_completion","rawAmount":10.0}"#;
        let view: super::RecognitionTriggerInputView = serde_json::from_str(json).unwrap();
        assert_eq!(view.content_id, "c-1");
        assert_eq!(view.event_type, "mastery_completion");
        assert!((view.raw_amount - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_update_content_input_view_deserializes_partial() {
        let json = r#"{"metadata": {"status": "done"}}"#;
        let view: super::UpdateContentInputView = serde_json::from_str(json).unwrap();
        assert!(view.title.is_none());
        assert!(view.tags.is_none());
        let meta = view.metadata.unwrap();
        assert_eq!(meta.0["status"], "done");
    }

    #[test]
    fn test_update_content_input_view_empty_patch_deserializes() {
        let json = r#"{}"#;
        let view: super::UpdateContentInputView = serde_json::from_str(json).unwrap();
        assert!(view.title.is_none());
        assert!(view.metadata.is_none());
    }
}

// ============================================================================
// Observation Sessions — Views
// ============================================================================

/// Input for beginning a new observation session.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginObservationInputView {
    pub source: String,
    #[serde(default = "default_obs_ttl")]
    pub ttl_seconds: i32,
    #[serde(default)]
    pub metadata: Option<JsonVal>,
}


/// Response returned after beginning an observation session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginObservationResponseView {
    pub session_id: String,
    pub expires_at: String,
}

/// Input for appending a single entry to an observation session.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationEntryInputView {
    pub origin: String,
    pub category: String,
    #[serde(default = "default_obs_severity")]
    pub severity: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub status_code: Option<i32>,
    pub message: String,
    #[serde(default)]
    pub context: Option<JsonVal>,
}


/// A single observation entry as returned in a report.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationEntryView {
    pub timestamp: String,
    pub origin: String,
    pub category: String,
    pub severity: String,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<i32>,
    pub message: String,
    pub context: Option<JsonVal>,
}

impl From<ObservationEntry> for ObservationEntryView {
    fn from(e: ObservationEntry) -> Self {
        Self {
            timestamp: e.timestamp,
            origin: e.origin,
            category: e.category,
            severity: e.severity,
            method: e.method,
            path: e.path,
            status_code: e.status_code,
            message: e.message,
            context: parse_json_opt(&e.context_json),
        }
    }
}

/// A detected issue surfaced from observation entries.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationIssueView {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub entry_count: usize,
    pub related_content_ids: Vec<String>,
    pub suggested_cause: String,
}

/// Duration metadata for an observation report.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDurationView {
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: i64,
}

/// Aggregate counts across entries in a session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSummaryView {
    pub total_entries: usize,
    pub by_origin: std::collections::HashMap<String, usize>,
    pub by_severity: std::collections::HashMap<String, usize>,
    pub by_category: std::collections::HashMap<String, usize>,
}

/// Snapshot of relevant system health at report time.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSystemStateView {
    pub storage_healthy: bool,
    pub conductor_connected: bool,
    pub p2p_peer_count: usize,
}

/// Full observation report returned when a session is closed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationReportView {
    pub content_id: String,
    pub session_id: String,
    pub source: String,
    pub metadata: Option<JsonVal>,
    pub duration: ObservationDurationView,
    pub summary: ObservationSummaryView,
    pub issues: Vec<ObservationIssueView>,
    pub system_state: ObservationSystemStateView,
}

// =========================================================================
// Resilience Views
// =========================================================================









// ============================================================================
// Gate Decision Attestation View
// ============================================================================
//
// Source of truth: DHT (mishpat DNA, GateDecisionAttestation entry, Category A).
// This view is served from the read-optimised SQLite projection populated by
// `MishpatSignal::GateDecisionCreated`. If the projection and the DHT disagree,
// the DHT wins.
//
// JSON fields (requestRefJson, reasoningJson) are surfaced as opaque strings —
// the gate client owns parsing of those payloads.

use crate::db::gate_decision_attestations::GateDecisionAttestationRow;


impl From<GateDecisionAttestationRow> for GateDecisionAttestationView {
    fn from(row: GateDecisionAttestationRow) -> Self {
        Self {
            decision_id: row.decision_id,
            phase: row.phase,
            elohim_id: row.elohim_id,
            elohim_substance_cid: row.elohim_substance_cid,
            gate_name: row.gate_name,
            gate_process_cid: row.gate_process_cid,
            request_ref_json: row.request_ref_json,
            decision: row.decision,
            reasoning_json: row.reasoning_json,
            context_summary_cid: row.context_summary_cid,
            decided_at: row.decided_at,
            universal_band_cid: row.universal_band_cid,
            dht_anchor_hash: row.dht_anchor_hash,
            created_at: row.created_at,
        }
    }
}

// ============================================================================
// Gate Decision Challenge View
// ============================================================================
//
// Source of truth: DHT (mishpat DNA, GateDecisionChallenge entry, Category A).
// This view is served from the read-optimised SQLite projection populated by
// `MishpatSignal::GateDecisionChallengeCreated`. If the projection and the DHT
// disagree, the DHT wins.

use crate::db::gate_decision_challenges::GateDecisionChallengeRow;


impl From<GateDecisionChallengeRow> for GateDecisionChallengeView {
    fn from(row: GateDecisionChallengeRow) -> Self {
        Self {
            challenge_id: row.challenge_id,
            challenged_decision_cid: row.challenged_decision_cid,
            challenger_id: row.challenger_id,
            grounds: row.grounds,
            summary: row.summary,
            evidence_refs: row.evidence_refs,
            filed_at: row.filed_at,
            reach: row.reach,
            dht_anchor_hash: row.dht_anchor_hash,
            created_at: row.created_at,
        }
    }
}

// ============================================================================
// Challenge Outcome View
// ============================================================================
//
// Source of truth: DHT (mishpat DNA, ChallengeOutcome entry, Category A).
// This view is served from the read-optimised SQLite projection populated by
// `MishpatSignal::ChallengeOutcomeCreated`. If the projection and the DHT
// disagree, the DHT wins.
//
// JSON fields (reasoningJson, indemnificationActionsJson) are surfaced as
// opaque strings — the gate client owns parsing of those payloads.

use crate::db::challenge_outcomes::ChallengeOutcomeRow;


impl From<ChallengeOutcomeRow> for ChallengeOutcomeView {
    fn from(row: ChallengeOutcomeRow) -> Self {
        Self {
            outcome_id: row.outcome_id,
            challenge_cid: row.challenge_cid,
            verdict: row.verdict,
            reviewer_consensus: row.reviewer_consensus,
            reasoning_json: row.reasoning_json,
            decided_at: row.decided_at,
            indemnification_actions_json: row.indemnification_actions_json,
            dht_anchor_hash: row.dht_anchor_hash,
            created_at: row.created_at,
        }
    }
}

// ============================================================================
// Elohim Reputation Profile View
// ============================================================================
//
// Source of truth: computed aggregation over the mishpat DNA DHT outcome graph
// (GateDecisionAttestation + GateDecisionChallenge + ChallengeOutcome entries).
// This is NOT a direct table projection — it is computed by
// `crate::db::elohim_reputation::compute` at query time from the SQLite
// projections of those DHT entries. If the projections and the DHT disagree,
// the DHT wins.
//
// No scalar score is returned. Dimensions are raw counts; time-decay and
// severity weighting are deferred to Phase 11+ and applied by consumers.
// Wire format governed by:
//   `elohim/sdk/schemas/v1/views/elohim-reputation-profile-view.schema.json`

use crate::db::elohim_reputation::ReputationResult;


// ============================================================================
// Peer Status View
// ============================================================================
//
// Source of truth: Holochain infrastructure DNA DHT (Notarized, Category A).
// This view is a read-optimised SQLite projection populated by
// `InfrastructureSignal::PeerStatusRecorded` post-commit projections.
// If the projection and the DHT disagree, the DHT wins.
//
// Wire format governed by:
//   `elohim/sdk/schemas/v1/views/peer-status-view.schema.json`
//   `elohim/sdk/schemas/v1/views/elohim-capability-profile.schema.json`
//
// `elohimCapability` is operator-configured at startup (Phase 9). The field is
// optional: a peer that does not run an elohim-agent omits it. Phase 10+ may
// auto-detect capabilities from a live elohim-agent-service.

use crate::db::peer_statuses::PeerStatusRow;





/// Tier-2 extensions map. Keys are kebab-case capability names registered in
/// the capability registry. Validation checks shape only; consumers interpret content.
pub type CapabilityExtensions = std::collections::BTreeMap<String, CapabilityExtensionEntry>;



impl From<PeerStatusRow> for PeerStatusView {
    fn from(row: PeerStatusRow) -> Self {
        Self {
            peer_id: row.peer_id,
            status: row.status,
            general_pool_member: row.general_pool_member != 0,
            accepting_stewardship_reserves: row.accepting_stewardship_reserves != 0,
            archetype_class: row.archetype_class,
            timestamp: row.timestamp.to_string(),
            dht_anchor_hash: row.dht_anchor_hash,
            updated_at: row.updated_at.to_string(),
            elohim_capability: None, // Layered post-construction via build_peer_status_view()
            render_capability: None, // Layered post-construction via build_peer_status_view()
            extensions: None,        // Layered post-construction via build_peer_status_view()
        }
    }
}

/// Build a `PeerStatusView` from a projection row plus the operator-configured capability.
///
/// The capability is Category C — operational, local state, not stored in the projection
/// table. It is loaded once at startup from `ELOHIM_CAPABILITY_CONFIG_FILE` and layered
/// here so that all construction sites stay consistent.
///
/// Use this instead of `PeerStatusView::from(row)` in handlers and tests.
pub fn build_peer_status_view(
    row: PeerStatusRow,
    elohim_capability: Option<&ElohimCapabilityProfile>,
    render_capability: Option<&RenderCapabilityProfile>,
    extensions: Option<&CapabilityExtensions>,
) -> PeerStatusView {
    let mut view = PeerStatusView::from(row);
    view.elohim_capability = elohim_capability.cloned();
    view.render_capability = render_capability.cloned();
    view.extensions = extensions.cloned();
    view
}

/// Load the operator-configured `ElohimCapabilityProfile` from the path
/// given in `ELOHIM_CAPABILITY_CONFIG_FILE`.
///
/// Returns `None` (honest degradation) when:
/// - The env var is unset
/// - The file does not exist or is not readable
/// - The file contains invalid JSON or does not match the profile shape
///
/// Logged at `WARN` level on file/JSON errors so operators see actionable
/// diagnostics without aborting startup.
pub fn load_elohim_capability_from_env() -> Option<ElohimCapabilityProfile> {
    let path = std::env::var("ELOHIM_CAPABILITY_CONFIG_FILE").ok()?;
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                path = %path,
                error = %e,
                "ELOHIM_CAPABILITY_CONFIG_FILE unreadable — elohim_capability will be None"
            );
            return None;
        }
    };
    match serde_json::from_str::<ElohimCapabilityProfile>(&contents) {
        Ok(profile) => Some(profile),
        Err(e) => {
            tracing::warn!(
                path = %path,
                error = %e,
                "ELOHIM_CAPABILITY_CONFIG_FILE contains invalid JSON — elohim_capability will be None"
            );
            None
        }
    }
}

/// Load the render capability profile from a doorway's `/admin/capability` HTTP endpoint.
///
/// Uses the URL in `DOORWAY_CAPABILITY_URL`. Returns `None` (honest degradation) when:
/// - The env var is unset
/// - The URL is unreachable
/// - The response is non-2xx
/// - The body fails to parse as `RenderCapabilityProfile`
pub async fn load_render_capability_from_url() -> Option<RenderCapabilityProfile> {
    let url = std::env::var("DOORWAY_CAPABILITY_URL").ok()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                url = %url,
                error = %e,
                "DOORWAY_CAPABILITY_URL unreachable — render_capability will be None"
            );
            return None;
        }
    };
    if !resp.status().is_success() {
        tracing::warn!(
            url = %url,
            status = %resp.status(),
            "DOORWAY_CAPABILITY_URL returned non-success — render_capability will be None"
        );
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    match serde_json::from_slice::<RenderCapabilityProfile>(&bytes) {
        Ok(profile) => Some(profile),
        Err(e) => {
            tracing::warn!(
                url = %url,
                error = %e,
                "DOORWAY_CAPABILITY_URL response did not parse as RenderCapabilityProfile"
            );
            None
        }
    }
}

/// Synchronous wrapper for tests / non-async startup paths.
///
/// Returns `None` immediately if `DOORWAY_CAPABILITY_URL` is unset (avoiding
/// runtime startup overhead).
pub fn load_render_capability_from_url_blocking() -> Option<RenderCapabilityProfile> {
    if std::env::var("DOORWAY_CAPABILITY_URL").is_err() {
        return None;
    }
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    rt.block_on(load_render_capability_from_url())
}








// ============================================================================
// Placement Gap + Resilience Snapshot Views
// ============================================================================

use crate::db::models::PlacementGapRow;


impl From<PlacementGapRow> for PlacementGapView {
    fn from(r: PlacementGapRow) -> Self {
        Self {
            id: r.id,
            content_id: r.content_id,
            shard_hash: r.shard_hash,
            requested_steward_count: r.requested_steward_count,
            achieved_steward_count: r.achieved_steward_count,
            contract_coverage: r.contract_coverage,
            gap_kind: r.gap_kind,
            first_seen_at: r.first_seen_at,
            last_seen_at: r.last_seen_at,
        }
    }
}





// =============================================================================
// Recovery Protocol Phase 2 Views
// =============================================================================




impl From<crate::db::models::RecoveryWitnessRow> for RecoveryWitnessView {
    fn from(r: crate::db::models::RecoveryWitnessRow) -> Self {
        Self {
            dht_anchor_hash: r.dht_anchor_hash,
            recovery_request_hash: r.recovery_request_hash,
            witness_agent_id: r.witness_agent_id,
            human_id: r.human_id,
            note: r.note,
            submitted_at: r.submitted_at,
        }
    }
}

// =============================================================================
// Recovery Protocol Phase 2 — M4 Views
// Source of truth: DHT (imagodei KeyRevocation / RevocationVote entries).
// These tables are read-optimized projections rebuildable via signal replay.
// =============================================================================


impl From<crate::db::models::KeyRevocationRow> for KeyRevocationView {
    fn from(r: crate::db::models::KeyRevocationRow) -> Self {
        // dht_anchor_hash is BLOB (Vec<u8>) on the DB side; serialize to
        // hex for the wire so clients see a stable string identifier.
        Self {
            dht_anchor_hash: hex::encode(&r.dht_anchor_hash),
            id: r.id,
            subject_human_id: r.subject_human_id,
            revoked_key: r.revoked_key,
            reason: r.reason,
            trigger_type: r.trigger_type,
            initiated_by_cid: r.initiated_by_cid,
            required_votes: r.required_votes as u32,
            current_votes: r.current_votes as u32,
            threshold_reached: r.threshold_reached == 1,
            effective_at: r.effective_at,
            derived_compromise_at: r.derived_compromise_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}


impl From<crate::db::models::RevocationVoteRow> for RevocationVoteView {
    fn from(r: crate::db::models::RevocationVoteRow) -> Self {
        Self {
            dht_anchor_hash: r.dht_anchor_hash,
            id: r.id,
            revocation_dht_anchor_hash: r.revocation_dht_anchor_hash,
            revocation_id: r.revocation_id,
            steward_id: r.steward_id,
            approved: r.approved == 1,
            attestation: r.attestation,
            voted_at: r.voted_at,
        }
    }
}

// ============================================================================
// EPR views (Phase 2a)
// Source of truth: EPR atom (self-notarized via content-address + Ed25519).
// ============================================================================










// =============================================================================
// Recovery Protocol Phase 2 — M5 Views
// Auth Portal Convergence + Revocation UX + Stub Defender
// =============================================================================












// ───────────────────────────────────────────────────────────────────────────
// Light-Up-Topology Phase 1 — operational distribution + cluster + reciprocity
// ───────────────────────────────────────────────────────────────────────────
//
// Wire shapes for the light-up-topology epic. These are Operational (Category
// C) projections — composed per request from rea_commitments + economic_events
// + peer_identity_bindings + libp2p swarm state. None of them introduce a new
// DHT entry type; none of them are persisted on the elohim-storage side.

































#[cfg(test)]
mod federation_canonical_tests {
    use super::*;

    #[test]
    fn request_canonical_round_trips() {
        let req = ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent_test".to_string(),
            request_id: "req_001".to_string(),
        };
        let bytes = req.canonical_bytes();
        let back: ViewFederationRequest = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn slice_canonical_excludes_stale_since_ms() {
        // Two slices identical except for stale_since_ms must produce the SAME
        // canonical signing bytes — that's the contract that makes signatures
        // verifiable across receivers with drifting clocks.
        let base = ViewSlice {
            peer_id: "12D3KooWAAA".to_string(),
            view_kind: ViewKind::Cluster,
            freshness: Freshness {
                state: FreshnessState::Live,
                stale_since_ms: Some(12_345),
            },
            payload: JsonVal(serde_json::Value::Null),
            signature: String::new(),
        };
        let mut other = base.clone();
        other.freshness.stale_since_ms = Some(99_999);
        assert_eq!(
            base.canonical_bytes_for_signing(),
            other.canonical_bytes_for_signing(),
            "stale_since_ms must NOT influence the signing canonical"
        );

        // And signature must NOT influence the canonical either — that would
        // be circular (you can't sign something whose bytes depend on the
        // signature you're producing).
        let mut signed = base.clone();
        signed.signature = "fake-base64-signature".to_string();
        assert_eq!(
            base.canonical_bytes_for_signing(),
            signed.canonical_bytes_for_signing(),
            "signature field must NOT be self-referential in the canonical"
        );
    }

    #[test]
    fn slice_canonical_serializes_freshness_state_as_camel_case_field() {
        // Sanity check: confirm the canonical bytes parse back as a msgpack
        // map whose `freshnessState` value is "live" (snake_case enum
        // serialization, camelCase field name to match the wire format).
        let slice = ViewSlice {
            peer_id: "12D3KooWAAA".to_string(),
            view_kind: ViewKind::Cluster,
            freshness: Freshness {
                state: FreshnessState::Live,
                stale_since_ms: Some(12_345),
            },
            payload: JsonVal(serde_json::json!({"hello": "world"})),
            signature: String::new(),
        };
        let bytes = slice.canonical_bytes_for_signing();
        let parsed: serde_json::Value = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(parsed["freshnessState"], serde_json::json!("live"));
        assert_eq!(parsed["viewKind"], serde_json::json!("cluster"));
        assert_eq!(parsed["peerId"], serde_json::json!("12D3KooWAAA"));
        assert_eq!(parsed["payload"], serde_json::json!({"hello": "world"}));
        // staleSinceMs must NOT appear in the canonical.
        assert!(
            parsed.get("staleSinceMs").is_none(),
            "staleSinceMs must not appear in signing canonical"
        );
    }
}

// ============================================================================
// Peer Transport Manifest View (Phase 12)
// ============================================================================
//
// Source-of-truth pairing: peer_transport_manifest SQLite table,
// populated by p2p_iroh::peer_map record_* fns.





// ============================================================================

// Observation Views
// ============================================================================


impl From<crate::db::models::ObservationRow> for ObservationView {
    fn from(r: crate::db::models::ObservationRow) -> Self {
        Self {
            observer_cid: r.observer_cid,
            log_cid: r.log_cid,
            log_offset: r.log_offset,
            observed_at: r.observed_at,
            seq: r.seq,
            observation_kind: r.observation_kind,
            subject_cid: r.subject_cid,
            subject_kind: r.subject_kind,
            payload_json: r.payload_json,
            observer_household_cid: r.observer_household_cid,
            observer_collective_cid: r.observer_collective_cid,
            observer_region: r.observer_region,
            observer_archetype: r.observer_archetype,
            observer_compute_class: r.observer_compute_class,
            signature_b64: r.signature_b64,
        }
    }
}


impl From<crate::db::models::ObservationDiversitySummaryRow> for ObservationDiversitySummaryView {
    fn from(r: crate::db::models::ObservationDiversitySummaryRow) -> Self {
        Self {
            subject_cid: r.subject_cid,
            observation_kind: r.observation_kind,
            distinct_agents: r.distinct_agents,
            distinct_households: r.distinct_households,
            distinct_collectives: r.distinct_collectives,
            distinct_regions: r.distinct_regions,
            distinct_archetypes: r.distinct_archetypes,
            distinct_compute_classes: r.distinct_compute_classes,
            total_count: r.total_count,
            first_observed_at: r.first_observed_at,
            last_observed_at: r.last_observed_at,
        }
    }
}

// Unified Attestation View (Category A — source of truth: Holochain DHT)
// ============================================================================
//
// Source of truth: Holochain DHT (notarized Content entry with
// `content_type LIKE 'attestation:%'` in the elohim DNA). This view is served
// from the read-optimised SQLite projection populated by the
// `AttestationProjector` on post-commit signal. If this projection and the
// DHT disagree, the DHT wins. `dht_anchor_hash` is the ActionHash (hex) of
// the DHT entry — clients MAY verify provenance against the conductor.

use crate::db::models::AttestationRow;


impl From<AttestationRow> for AttestationView {
    fn from(row: AttestationRow) -> Self {
        Self {
            id: row.id,
            dht_anchor_hash: hex::encode(&row.dht_anchor_hash),
            attestation_kind: row.attestation_kind,
            subject_cid: row.subject_cid,
            subject_kind: row.subject_kind,
            issuer_cid: row.issuer_cid,
            parent_governance_action_cid: row.parent_governance_action_cid,
            vote_value: row.vote_value,
            vote_weight: row.vote_weight,
            proof_class: row.proof_class,
            proof_evidence_json: row.proof_evidence_json,
            evidence_json: row.evidence_json,
            expires_at: row.expires_at,
            supersedes_cid: row.supersedes_cid,
            revocation_reason: row.revocation_reason,
            revoked_at: row.revoked_at,
            created_at: row.created_at,
            manifest_ref: row.manifest_ref,
            title: row.title,
            description: row.description,
        }
    }
}

// ============================================================================
// Governance Action View (Category A — source of truth: Holochain DHT)
// ============================================================================
//
// Source of truth: Holochain DHT (notarized Content entry with
// `content_type LIKE 'governance-action:%'` in the elohim DNA). This view is
// a read-optimised projection populated by the `AttestationProjector` on
// post-commit signal. If this projection and the DHT disagree, the DHT wins.

use crate::db::models::GovernanceActionRow;


impl From<GovernanceActionRow> for GovernanceActionView {
    fn from(row: GovernanceActionRow) -> Self {
        Self {
            id: row.id,
            dht_anchor_hash: hex::encode(&row.dht_anchor_hash),
            governance_kind: row.governance_kind,
            subject_cid: row.subject_cid,
            proposer_cid: row.proposer_cid,
            threshold_json: row.threshold_json,
            eligibility_predicate_json: row.eligibility_predicate_json,
            ballot_format: row.ballot_format,
            closes_at: row.closes_at,
            parameters_json: row.parameters_json,
            title: row.title,
            description: row.description,
            created_at: row.created_at,
        }
    }
}

// ============================================================================
// Governance Action Tally View (Category C — local operational derived projection)
// ============================================================================
//
// Source of truth: local (operational) — recomputable at any time from
// governance_actions JOIN attestations. The `TallyProjector` maintains this
// derived table; the DHT does NOT store tallies directly. If the tally is
// stale, call `tally_projector::recompute(conn, parent_cid)` to rebuild.

use crate::db::models::GovernanceActionTallyRow;

// Re-export wire types from elohim-views so in-tree consumers' `crate::views::TypeName`
// continues to resolve. New code should import directly from `elohim_views::*`.
pub use elohim_views::shared::*;
pub use elohim_views::lamad::*;
pub use elohim_views::shefa::*;
pub use elohim_views::qahal::*;
pub use elohim_views::imagodei::*;
pub use elohim_views::infrastructure::*;
pub use elohim_views::epr::*;
pub use elohim_views::inputs::*;



impl From<GovernanceActionTallyRow> for GovernanceActionTallyView {
    fn from(row: GovernanceActionTallyRow) -> Self {
        Self {
            parent_cid: row.parent_cid,
            governance_kind: row.governance_kind,
            subject_cid: row.subject_cid,
            threshold_m: row.threshold_m,
            threshold_n: row.threshold_n,
            threshold_percentage: row.threshold_percentage,
            closes_at: row.closes_at,
            current_approve_count: row.current_approve_count,
            current_reject_count: row.current_reject_count,
            current_abstain_count: row.current_abstain_count,
            computed_status: row.computed_status,
            last_child_at: row.last_child_at,
            rebuilt_at: row.rebuilt_at,
        }
    }
}

// ============================================================================
// Free functions replacing inherent impls that use storage-internal types.
// These replace the `impl TypeName { pub fn ... }` pattern, which is
// forbidden by Rust's orphan rule when `TypeName` is defined in another crate.
// ============================================================================

/// Construct a VoteView from a DB Vote model.
pub fn vote_view_from_vote(v: crate::db::models::Vote, hide_identity: bool) -> VoteView {
    VoteView {
        id: v.id,
        proposal_id: v.proposal_id,
        human_id: if hide_identity { None } else { Some(v.human_id) },
        position: v.position,
        reason: v.reason,
        created_at: v.created_at,
        updated_at: v.updated_at,
        dht_anchor_hash: v.dht_anchor_hash,
        voter_id: None,
        voter_name: None,
        reasoning: None,
        version: None,
        previous_position: None,
        metadata: None,
    }
}

/// Construct a RankedVoteView from a DB RankedVote model.
pub fn ranked_vote_view_from_ranked_vote(v: crate::db::models::RankedVote, hide_identity: bool) -> RankedVoteView {
    RankedVoteView {
        id: v.id,
        proposal_id: v.proposal_id,
        human_id: if hide_identity { None } else { Some(v.human_id) },
        option_id: v.option_id,
        rank: v.rank,
        score: v.score,
        dots: v.dots,
        approved: v.approved.map(|a| a == 1),
        reasoning: v.reasoning,
        proxy_elohim_id: v.proxy_elohim_id,
        created_at: v.created_at,
        dht_anchor_hash: v.dht_anchor_hash,
    }
}

/// Build a NodeStewardshipView from DB model + joined display name.
pub fn node_stewardship_view_from_with_name(s: crate::db::models::NodeStewardship, display_name: String) -> NodeStewardshipView {
    NodeStewardshipView {
        human_id: s.human_id,
        display_name,
        affinity_score: s.affinity_score,
        relationship: s.relationship,
        context_epr_id: s.context_epr_id,
        granted_at: s.granted_at,
    }
}

/// Build an ElohimReputationProfileView from a computed aggregation result.
pub fn elohim_reputation_profile_view_from_result(
    elohim_id: String,
    window_start: String,
    window_end: String,
    r: crate::db::elohim_reputation::ReputationResult,
) -> ElohimReputationProfileView {
    use serde_json::Map;
    let grounds_map: Map<String, serde_json::Value> = r
        .challenges_by_grounds
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::Number(v.into())))
        .collect();
    let verdicts_map: Map<String, serde_json::Value> = r
        .outcomes_by_verdict
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::Number(v.into())))
        .collect();
    ElohimReputationProfileView {
        elohim_id,
        window_start,
        window_end,
        current_substance_cid: r.current_substance_cid,
        total_decisions: r.total_decisions,
        challenged_count: r.challenged_count,
        upheld_count: r.upheld_count,
        dismissed_count: r.dismissed_count,
        superseded_count: r.superseded_count,
        pending_count: r.pending_count,
        challenges_by_grounds: JsonVal(serde_json::Value::Object(grounds_map)),
        outcomes_by_verdict: JsonVal(serde_json::Value::Object(verdicts_map)),
    }
}

/// Build a PutBlobResponseView from an existing ShardManifest plus optional BLAKE3 hash.
pub fn put_blob_response_view_from_manifest(
    m: crate::sharding::ShardManifest,
    blake3_hash: Option<String>,
) -> PutBlobResponseView {
    PutBlobResponseView {
        blob_hash: m.blob_hash,
        total_size: m.total_size,
        mime_type: m.mime_type,
        encoding: m.encoding,
        data_shards: m.data_shards,
        total_shards: m.total_shards,
        shard_size: m.shard_size,
        shard_hashes: m.shard_hashes,
        reach: m.reach,
        author_id: m.author_id,
        created_at: m.created_at,
        verified_at: m.verified_at,
        blake3_hash,
    }
}
