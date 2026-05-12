//! Wire types for shefa (economics) domain coordinator functions.
//!
//! These types define the MessagePack-serialized inputs and outputs for
//! content_store zome calls related to REA economics, premium gating,
//! steward credentials, and custodian commitments. They are consumed by:
//! - The content_store coordinator zome (WASM target)
//! - Doorway gateway service (native target)
//! - elohim-storage (native target)
//!
//! This crate is an IoC artifact in `sdk/domains/shefa/`, alongside
//! the domain's schemas and manifest. It must NOT depend on HDK, HDI,
//! or any WASM-specific crates.

use holo_hash::{ActionHash, EntryHash};
use serde::{Deserialize, Serialize};

// =============================================================================
// REA Agreement Types
// =============================================================================

/// Input for content_store::create_agreement coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateAgreementInput {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Agreement wire type. Mirrors the integrity zome's Agreement entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Agreement {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: String,
}

/// Output from content_store::get_agreement coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AgreementOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub entry_hash: EntryHash,
    pub agreement: Agreement,
}

// =============================================================================
// REA Commitment Types
// =============================================================================

/// Input for content_store::create_rea_commitment coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateReaCommitmentInput {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_classified_as: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_beginning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub in_scope_of: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
}

/// Commitment wire type. Mirrors the integrity zome's Commitment entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Commitment {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_conforms_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_inventoried_as: Option<String>,
    pub resource_classified_as_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_point_in_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_beginning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clause_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreed_in: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub satisfies: Option<String>,
    pub in_scope_of_json: String,
    pub finished: bool,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from content_store::get_rea_commitment coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReaCommitmentOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub entry_hash: EntryHash,
    pub commitment: Commitment,
}

// =============================================================================
// REA Economic Event Types
// =============================================================================

/// Input for content_store::create_rea_economic_event coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateReaEconomicEventInput {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_classified_as: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_unit: Option<String>,
    pub has_point_in_time: String,
    #[serde(default)]
    pub fulfills: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realization_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lamad_event_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
    /// Substrate observation refs proving this event was graduated from observed data.
    /// Required for operational verbs (served-blob-summary, appreciation-summary,
    /// consumed-compute-summary). Omit for high-stakes authored verbs.
    /// See observation-event-layer spec §8.3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_refs: Option<Vec<String>>,
}

/// EconomicEvent wire type. Mirrors the integrity zome's EconomicEvent entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct EconomicEvent {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_conforms_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_inventoried_as: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_resource_inventoried_as: Option<String>,
    pub resource_classified_as_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_unit: Option<String>,
    pub has_point_in_time: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_duration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_of: Option<String>,
    pub fulfills_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realization_of: Option<String>,
    pub satisfies_json: String,
    pub in_scope_of_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggered_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lamad_event_type: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
}

/// Output from content_store::create_rea_economic_event coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReaEconomicEventOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub entry_hash: EntryHash,
    pub event: EconomicEvent,
    #[cfg_attr(feature = "ts", ts(type = "Array<string>"))]
    pub fulfillment_links: Vec<ActionHash>,
}

// =============================================================================
// Premium Gating Types
// =============================================================================

/// Input for content_store::create_steward_credential coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateStewardCredentialInput {
    pub steward_presence_id: String,
    pub tier: String,
    pub domain_tags: Vec<String>,
    pub mastery_content_ids: Vec<String>,
    pub mastery_level_achieved: String,
    pub peer_attestation_ids: Vec<String>,
    pub stewarded_presence_ids: Vec<String>,
    pub stewarded_content_ids: Vec<String>,
    pub stewarded_path_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// StewardCredential wire type. Mirrors the integrity zome's StewardCredential entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StewardCredential {
    pub id: String,
    pub steward_presence_id: String,
    pub agent_id: String,
    pub tier: String,
    pub stewarded_presence_ids_json: String,
    pub stewarded_content_ids_json: String,
    pub stewarded_path_ids_json: String,
    pub mastery_content_ids_json: String,
    pub mastery_level_achieved: String,
    pub qualification_verified_at: String,
    pub peer_attestation_ids_json: String,
    pub unique_attester_count: u32,
    pub attester_reputation_sum: f64,
    pub stewardship_quality_score: f64,
    pub total_learners_served: u32,
    pub total_content_improvements: u32,
    pub domain_tags_json: String,
    pub is_active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deactivation_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from content_store::create_steward_credential coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StewardCredentialOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub credential: StewardCredential,
}

/// Required attestation for gate access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RequiredAttestationInput {
    pub attestation_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation_id: Option<String>,
}

/// Required mastery for gate access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RequiredMasteryInput {
    pub content_id: String,
    pub min_level: String,
}

/// Required vouches for gate access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RequiredVouchesInput {
    pub min_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_tier: Option<String>,
}

/// Input for content_store::create_premium_gate coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreatePremiumGateInput {
    pub steward_credential_id: String,
    pub steward_presence_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributor_presence_id: Option<String>,
    pub gated_resource_type: String,
    pub gated_resource_ids: Vec<String>,
    pub gate_title: String,
    pub gate_description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_image: Option<String>,
    pub required_attestations: Vec<RequiredAttestationInput>,
    pub required_mastery: Vec<RequiredMasteryInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_vouches: Option<RequiredVouchesInput>,
    pub pricing_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_period_days: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<f64>,
    pub steward_share_percent: f64,
    pub commons_share_percent: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributor_share_percent: Option<f64>,
    pub scholarship_eligible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_scholarships_per_period: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scholarship_criteria_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// PremiumGate wire type. Mirrors the integrity zome's PremiumGate entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PremiumGate {
    pub id: String,
    pub steward_credential_id: String,
    pub steward_presence_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributor_presence_id: Option<String>,
    pub gated_resource_type: String,
    pub gated_resource_ids_json: String,
    pub gate_title: String,
    pub gate_description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_image: Option<String>,
    pub required_attestations_json: String,
    pub required_mastery_json: String,
    pub required_vouches_json: String,
    pub pricing_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_period_days: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<f64>,
    pub steward_share_percent: f64,
    pub commons_share_percent: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributor_share_percent: Option<f64>,
    pub scholarship_eligible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_scholarships_per_period: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scholarship_criteria_json: Option<String>,
    pub is_active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deactivation_reason: Option<String>,
    pub total_access_grants: u32,
    pub total_revenue_generated: f64,
    pub total_to_steward: f64,
    pub total_to_contributor: f64,
    pub total_to_commons: f64,
    pub total_scholarships_granted: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from content_store::create_premium_gate coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PremiumGateOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub gate: PremiumGate,
}

/// AccessGrant wire type. Mirrors the integrity zome's AccessGrant entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AccessGrant {
    pub id: String,
    pub gate_id: String,
    pub learner_agent_id: String,
    pub grant_type: String,
    pub granted_via: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scholarship_sponsor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scholarship_reason: Option<String>,
    pub granted_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renewal_due_at: Option<String>,
    pub is_active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoke_reason: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
}

/// Output from content_store::grant_access coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AccessGrantOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub grant: AccessGrant,
}

/// Input for content_store::grant_access coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GrantAccessInput {
    pub gate_id: String,
    pub grant_type: String,
    pub granted_via: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scholarship_sponsor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scholarship_reason: Option<String>,
}

/// StewardRevenue wire type. Mirrors the integrity zome's StewardRevenue entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StewardRevenue {
    pub id: String,
    pub access_grant_id: String,
    pub gate_id: String,
    pub from_learner_id: String,
    pub to_steward_presence_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_contributor_presence_id: Option<String>,
    pub gross_amount: f64,
    pub payment_unit: String,
    pub steward_amount: f64,
    pub contributor_amount: f64,
    pub commons_amount: f64,
    pub steward_economic_event_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributor_economic_event_id: Option<String>,
    pub commons_economic_event_id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
}

/// Output from steward revenue operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StewardRevenueOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    pub revenue: StewardRevenue,
}

/// Revenue summary per gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GateRevenueSummary {
    pub gate_id: String,
    pub gate_title: String,
    pub total_revenue: f64,
    pub grant_count: u32,
}

/// Steward revenue summary across all gates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct StewardRevenueSummary {
    pub steward_presence_id: String,
    pub total_revenue: f64,
    pub total_grants: u32,
    pub revenue_by_gate: Vec<GateRevenueSummary>,
}

// =============================================================================
// Contributor Dashboard Types
// =============================================================================

/// Impact summary per content piece.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ContentImpactSummary {
    pub content_id: String,
    pub recognition_points: i64,
    pub learners_reached: u32,
    pub mastery_count: u32,
}

/// Recent recognition event for the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RecognitionEventSummary {
    pub learner_id: String,
    pub content_id: String,
    pub flow_type: String,
    pub recognition_points: i32,
    pub occurred_at: String,
}

// =============================================================================
// Custodian Commitment Types
// =============================================================================

/// Input for content_store::create_custodian_commitment coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateCustodianCommitmentInput {
    pub custodian_agent_id: String,
    pub beneficiary_agent_id: String,
    pub commitment_type: String,
    pub basis: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship_id: Option<String>,
    pub category_override_json: String,
    pub content_filters_json: String,
    pub estimated_content_count: u32,
    pub estimated_size_mb: f64,
    pub shard_strategy: String,
    pub redundancy_factor: u32,
    pub shard_assignments_json: String,
    pub emergency_triggers_json: String,
    pub emergency_contacts_json: String,
    pub recovery_instructions_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_priority: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geographic_affinity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
}

/// CustodianCommitment wire type. Mirrors the integrity zome's CustodianCommitment entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CustodianCommitment {
    pub id: String,
    pub custodian_agent_id: String,
    pub beneficiary_agent_id: String,
    pub commitment_type: String,
    pub basis: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship_id: Option<String>,
    pub category_override_json: String,
    pub content_filters_json: String,
    pub estimated_content_count: u32,
    pub estimated_size_mb: f64,
    pub shard_strategy: String,
    pub redundancy_factor: u32,
    pub shard_assignments_json: String,
    pub emergency_triggers_json: String,
    pub emergency_contacts_json: String,
    pub recovery_instructions_json: String,
    pub cache_priority: u32,
    pub bandwidth_class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geographic_affinity: Option<String>,
    pub state: String,
    pub proposed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_verification_at: Option<String>,
    pub verification_failures_json: String,
    pub shards_stored_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_shard_update_at: Option<String>,
    pub total_restores_performed: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shefa_commitment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from content_store::create_custodian_commitment coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CustodianCommitmentOutput {
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub action_hash: ActionHash,
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub entry_hash: EntryHash,
    pub commitment: CustodianCommitment,
}

/// Input for querying custodian commitments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct QueryCommitmentsInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custodian_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beneficiary_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commitment_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Input for accepting a custodian commitment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AcceptCommitmentInput {
    pub commitment_id: String,
}

/// Input for batch accepting custodian commitments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BatchAcceptCommitmentsInput {
    pub commitment_ids: Vec<String>,
}

/// Output from batch accepting custodian commitments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BatchAcceptCommitmentsOutput {
    pub accepted_count: u32,
    pub failed_count: u32,
    pub errors: Vec<String>,
}

/// Input for batch updating custodian commitment metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct BatchUpdateCommitmentsInput {
    /// Vec of (commitment_id, new_cache_priority, new_bandwidth_class).
    pub updates: Vec<(String, u32, Option<String>)>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_agreement_input_msgpack_roundtrip() {
        let input = CreateAgreementInput {
            id: "agreement-1".into(),
            name: Some("Test Agreement".into()),
            note: None,
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateAgreementInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "agreement-1");
        assert_eq!(decoded.name, Some("Test Agreement".into()));
        assert_eq!(decoded.note, None);
    }

    #[test]
    fn create_rea_commitment_input_msgpack_roundtrip() {
        let input = CreateReaCommitmentInput {
            id: "commitment-1".into(),
            action: "transfer".into(),
            provider: "agent-a".into(),
            receiver: "agent-b".into(),
            resource_classified_as: vec!["learning-content".into()],
            resource_quantity_value: Some(1.0),
            resource_quantity_unit: Some("each".into()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: Some("2026-12-31".into()),
            clause_of: Some("agreement-1".into()),
            in_scope_of: vec![],
            note: Some("test commitment".into()),
            metadata_json: None,
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateReaCommitmentInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "commitment-1");
        assert_eq!(decoded.action, "transfer");
        assert_eq!(decoded.due, Some("2026-12-31".into()));
    }

    #[test]
    fn create_rea_economic_event_input_msgpack_roundtrip() {
        let input = CreateReaEconomicEventInput {
            id: "event-1".into(),
            action: "use".into(),
            provider: "agent-a".into(),
            receiver: "agent-b".into(),
            resource_classified_as: vec!["learning-content".into()],
            resource_quantity_value: Some(1.0),
            resource_quantity_unit: Some("each".into()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: "2026-04-05T12:00:00Z".into(),
            fulfills: vec!["commitment-1".into()],
            realization_of: None,
            lamad_event_type: Some("content_consumed".into()),
            note: None,
            metadata_json: None,
            observation_refs: None,
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateReaEconomicEventInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "event-1");
        assert_eq!(decoded.has_point_in_time, "2026-04-05T12:00:00Z");
        assert_eq!(decoded.fulfills, vec!["commitment-1".to_string()]);
    }

    #[test]
    fn create_steward_credential_input_msgpack_roundtrip() {
        let input = CreateStewardCredentialInput {
            steward_presence_id: "presence-1".into(),
            tier: "apprentice".into(),
            domain_tags: vec!["economics".into()],
            mastery_content_ids: vec!["content-1".into()],
            mastery_level_achieved: "apply".into(),
            peer_attestation_ids: vec![],
            stewarded_presence_ids: vec![],
            stewarded_content_ids: vec!["content-1".into()],
            stewarded_path_ids: vec![],
            note: None,
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateStewardCredentialInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.steward_presence_id, "presence-1");
        assert_eq!(decoded.tier, "apprentice");
    }

    #[test]
    fn create_premium_gate_input_msgpack_roundtrip() {
        let input = CreatePremiumGateInput {
            steward_credential_id: "cred-1".into(),
            steward_presence_id: "presence-1".into(),
            contributor_presence_id: None,
            gated_resource_type: "path".into(),
            gated_resource_ids: vec!["path-1".into()],
            gate_title: "Test Gate".into(),
            gate_description: "A test gate".into(),
            gate_image: None,
            required_attestations: vec![],
            required_mastery: vec![],
            required_vouches: None,
            pricing_model: "free".into(),
            price_amount: None,
            price_unit: None,
            subscription_period_days: None,
            min_amount: None,
            steward_share_percent: 85.0,
            commons_share_percent: 15.0,
            contributor_share_percent: None,
            scholarship_eligible: false,
            max_scholarships_per_period: None,
            scholarship_criteria_json: None,
            note: None,
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreatePremiumGateInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.gate_title, "Test Gate");
        assert_eq!(decoded.steward_share_percent, 85.0);
    }

    #[test]
    fn grant_access_input_msgpack_roundtrip() {
        let input = GrantAccessInput {
            gate_id: "gate-1".into(),
            grant_type: "payment".into(),
            granted_via: "payment".into(),
            payment_amount: Some(10.0),
            payment_unit: Some("elohim-credit".into()),
            scholarship_sponsor_id: None,
            scholarship_reason: None,
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: GrantAccessInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.gate_id, "gate-1");
        assert_eq!(decoded.payment_amount, Some(10.0));
    }

    #[test]
    fn create_custodian_commitment_input_msgpack_roundtrip() {
        let input = CreateCustodianCommitmentInput {
            custodian_agent_id: "agent-a".into(),
            beneficiary_agent_id: "agent-b".into(),
            commitment_type: "relationship".into(),
            basis: "intimate_relationship".into(),
            relationship_id: Some("rel-1".into()),
            category_override_json: "[]".into(),
            content_filters_json: "[]".into(),
            estimated_content_count: 100,
            estimated_size_mb: 50.0,
            shard_strategy: "full_replica".into(),
            redundancy_factor: 2,
            shard_assignments_json: "[]".into(),
            emergency_triggers_json: "[]".into(),
            emergency_contacts_json: "[]".into(),
            recovery_instructions_json: "{}".into(),
            cache_priority: Some(50),
            bandwidth_class: Some("medium".into()),
            geographic_affinity: None,
            note: None,
            metadata_json: None,
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateCustodianCommitmentInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.custodian_agent_id, "agent-a");
        assert_eq!(decoded.commitment_type, "relationship");
        assert_eq!(decoded.estimated_content_count, 100);
    }

    #[test]
    fn batch_accept_commitments_input_msgpack_roundtrip() {
        let input = BatchAcceptCommitmentsInput {
            commitment_ids: vec!["c1".into(), "c2".into()],
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: BatchAcceptCommitmentsInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.commitment_ids.len(), 2);
    }

    #[test]
    fn query_commitments_input_msgpack_roundtrip() {
        let input = QueryCommitmentsInput {
            custodian_agent_id: Some("agent-a".into()),
            beneficiary_agent_id: None,
            commitment_type: None,
            state: Some("accepted".into()),
            basis: None,
            limit: Some(10),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: QueryCommitmentsInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.custodian_agent_id, Some("agent-a".into()));
        assert_eq!(decoded.limit, Some(10));
    }
}


#[cfg(test)]
#[cfg(feature = "ts")]
mod ts_export {
    use super::*;
    use ts_rs::TS;

    #[test]
    fn export_bindings() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings");
        CreateAgreementInput::export_all_to(&out).unwrap();
        Agreement::export_all_to(&out).unwrap();
        AgreementOutput::export_all_to(&out).unwrap();
        CreateReaCommitmentInput::export_all_to(&out).unwrap();
        Commitment::export_all_to(&out).unwrap();
        ReaCommitmentOutput::export_all_to(&out).unwrap();
        CreateReaEconomicEventInput::export_all_to(&out).unwrap();
        EconomicEvent::export_all_to(&out).unwrap();
        ReaEconomicEventOutput::export_all_to(&out).unwrap();
        CreateStewardCredentialInput::export_all_to(&out).unwrap();
        StewardCredential::export_all_to(&out).unwrap();
        StewardCredentialOutput::export_all_to(&out).unwrap();
        RequiredAttestationInput::export_all_to(&out).unwrap();
        RequiredMasteryInput::export_all_to(&out).unwrap();
        RequiredVouchesInput::export_all_to(&out).unwrap();
        CreatePremiumGateInput::export_all_to(&out).unwrap();
        PremiumGate::export_all_to(&out).unwrap();
        PremiumGateOutput::export_all_to(&out).unwrap();
        AccessGrant::export_all_to(&out).unwrap();
        AccessGrantOutput::export_all_to(&out).unwrap();
        GrantAccessInput::export_all_to(&out).unwrap();
        StewardRevenue::export_all_to(&out).unwrap();
        StewardRevenueOutput::export_all_to(&out).unwrap();
        GateRevenueSummary::export_all_to(&out).unwrap();
        StewardRevenueSummary::export_all_to(&out).unwrap();
        ContentImpactSummary::export_all_to(&out).unwrap();
        RecognitionEventSummary::export_all_to(&out).unwrap();
        CreateCustodianCommitmentInput::export_all_to(&out).unwrap();
        CustodianCommitment::export_all_to(&out).unwrap();
        CustodianCommitmentOutput::export_all_to(&out).unwrap();
        QueryCommitmentsInput::export_all_to(&out).unwrap();
        AcceptCommitmentInput::export_all_to(&out).unwrap();
        BatchAcceptCommitmentsInput::export_all_to(&out).unwrap();
        BatchAcceptCommitmentsOutput::export_all_to(&out).unwrap();
        BatchUpdateCommitmentsInput::export_all_to(&out).unwrap();
    }
}
