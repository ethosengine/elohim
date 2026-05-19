//! shefa view types — migrated from elohim-storage/src/views.rs (VIEWS.T2).

use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json::Value;
use ts_rs::TS;
use crate::shared::*;
#[allow(unused_imports)]
use crate::imagodei::*;
#[allow(unused_imports)]
use crate::infrastructure::*;
#[allow(unused_imports)]
use crate::lamad::*;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EconomicEventView {
    pub id: String,
    pub h_app_id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_conforms_to: Option<String>,
    pub resource_inventoried_as: Option<String>,
    /// Parsed resource classification (was resource_classified_as_json string in storage)
    pub resource_classified_as: Option<JsonVal>,
    pub resource_quantity_value: Option<f32>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f32>,
    pub effort_quantity_unit: Option<String>,
    pub has_point_in_time: String,
    pub has_duration: Option<String>,
    pub input_of: Option<String>,
    pub output_of: Option<String>,
    pub lamad_event_type: Option<String>,
    pub content_id: Option<String>,
    pub contributor_presence_id: Option<String>,
    pub path_id: Option<String>,
    pub triggered_by: Option<String>,
    pub state: String,
    pub note: Option<String>,
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<JsonVal>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
    /// Place ID where this event occurred (spatial grounding)
    pub at_location: Option<String>,
}

/// Measure — quantity + unit pair (ValueFlows)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct MeasureView {
    pub has_numerical_value: f32,
    pub has_unit: String,
}

/// REA Commitment — API output
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ReaCommitmentView {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_conforms_to: Option<String>,
    pub resource_classified_as: Option<Vec<String>>,
    pub resource_quantity: Option<MeasureView>,
    pub effort_quantity: Option<MeasureView>,
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub due: Option<String>,
    pub clause_of: Option<String>,
    pub in_scope_of: Option<Vec<String>>,
    pub medium_of_exchange_id: Option<String>,
    pub state: String,
    pub finished: bool,
    pub note: Option<String>,
    pub metadata: Option<JsonVal>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
}

/// Denormalized read shape for active doorway-operator authority. Wire
/// contract: elohim/sdk/schemas/v1/views/doorway-operator-binding-view.schema.json
///
/// Source of truth: derived from rea_commitments where action='operate-doorway'
/// and state='active' (Operational, Category C — reconstructed from the
/// notarized Commitment entry; no separate persistent table). Capability set
/// is parsed from resource_classified_as; successionRole + reachScope from
/// metadata_json (per operator-classification.schema.json).
///
/// Custody + steward chain: schemaVersion=2 carries custodyAttestationHash and
/// stewardAttestationHash references — the two-layer attestation chain ABOVE
/// the operator commitment. v1 commitments have these None during the
/// transition window. Auth chain resolution (verify_custody_chain) walks
/// commitment -> steward attestation -> custody attestation, returning
/// orphaned if any link has been superseded.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DoorwayOperatorBindingView {
    pub commitment_id: String,
    pub doorway_id: String,
    pub operator_agent: String,
    pub capabilities: Vec<String>,
    pub succession_role: String,
    pub reach_scope: String,
    pub state: String,
    pub agreement_id: String,
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub dht_anchor_hash: String,
    pub custody_attestation_hash: Option<String>,
    pub steward_attestation_hash: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardshipAllocationView {
    pub id: String,
    pub h_app_id: String,
    pub content_id: String,
    pub steward_presence_id: String,
    pub allocation_ratio: f32,
    pub allocation_method: String,
    pub contribution_type: String,
    /// Parsed contribution evidence (was contribution_evidence_json string in storage)
    pub contribution_evidence: Option<JsonVal>,
    pub governance_state: String,
    pub dispute_id: Option<String>,
    pub dispute_reason: Option<String>,
    pub disputed_at: Option<String>,
    pub disputed_by: Option<String>,
    pub negotiation_session_id: Option<String>,
    pub elohim_ratified_at: Option<String>,
    pub elohim_ratifier_id: Option<String>,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub superseded_by: Option<String>,
    pub recognition_accumulated: f32,
    pub last_recognition_at: Option<String>,
    pub note: Option<String>,
    /// Parsed metadata object (was metadata_json string in storage)
    pub metadata: Option<JsonVal>,
    pub created_at: String,
    pub updated_at: String,
    pub dht_anchor_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardshipAllocationWithPresenceView {
    #[serde(flatten)]
    pub allocation: StewardshipAllocationView,
    pub steward: Option<ContributorPresenceView>,
}

/// Input for initiating a claim - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct InitiateClaimInputView {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub claiming_agent_id: String,
    pub verification_method: String,
    /// Parsed evidence object (serialized to JSON string for DB)
    #[serde(default)]
    pub evidence: Option<JsonVal>,
    #[serde(default)]
    pub facilitated_by: Option<String>,
}

/// Input for creating an economic event - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateEconomicEventInputView {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_inventoried_as: Option<String>,
    #[serde(default)]
    pub resource_classified_as: Vec<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f32>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f32>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_point_in_time: Option<String>,
    #[serde(default)]
    pub has_duration: Option<String>,
    #[serde(default)]
    pub input_of: Option<String>,
    #[serde(default)]
    pub output_of: Option<String>,
    #[serde(default)]
    pub lamad_event_type: Option<String>,
    #[serde(default)]
    pub content_id: Option<String>,
    #[serde(default)]
    pub contributor_presence_id: Option<String>,
    #[serde(default)]
    pub path_id: Option<String>,
    #[serde(default)]
    pub triggered_by: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<JsonVal>,
    /// Place ID where this event occurred (spatial grounding)
    #[serde(default)]
    pub at_location: Option<String>,
}

/// Input for creating a stewardship allocation - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateAllocationInputView {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub content_id: String,
    pub steward_presence_id: String,
    #[serde(default)]
    pub allocation_ratio: Option<f32>,
    #[serde(default)]
    pub allocation_method: Option<String>,
    #[serde(default)]
    pub contribution_type: Option<String>,
    /// Parsed contribution evidence (serialized to JSON string for DB)
    #[serde(default)]
    pub contribution_evidence: Option<JsonVal>,
    #[serde(default)]
    pub note: Option<String>,
    /// Parsed metadata object (serialized to JSON string for DB)
    #[serde(default)]
    pub metadata: Option<JsonVal>,
}

/// Input for updating a stewardship allocation - camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateAllocationInputView {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub allocation_ratio: Option<f32>,
    #[serde(default)]
    pub allocation_method: Option<String>,
    #[serde(default)]
    pub contribution_type: Option<String>,
    /// Parsed contribution evidence (serialized to JSON string for DB)
    #[serde(default)]
    pub contribution_evidence: Option<JsonVal>,
    #[serde(default)]
    pub governance_state: Option<String>,
    #[serde(default)]
    pub dispute_id: Option<String>,
    #[serde(default)]
    pub dispute_reason: Option<String>,
    #[serde(default)]
    pub elohim_ratified_at: Option<String>,
    #[serde(default)]
    pub elohim_ratifier_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// API view for a token mint event.
/// Mirrors `TokenMintEvent` with camelCase fields for TypeScript clients.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TokenMintEventView {
    pub id: String,
    pub h_app_id: String,
    pub amount: f32,
    pub provenance_event_id: String,
    pub mint_tier: String,
    pub source_epr_id: String,
    pub agent_id: String,
    pub constitutional_context: Option<String>,
    pub elohim_attestation: Option<String>,
    pub reasoning_trace: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
}

/// API view for a token balance ledger entry.
/// One row per agent per governance layer.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TokenBalanceView {
    pub agent_id: String,
    pub h_app_id: String,
    pub governance_layer: String,
    pub balance: f32,
    pub total_minted: f32,
    pub total_transferred_in: f32,
    pub total_transferred_out: f32,
    pub last_activity_at: String,
    pub created_at: String,
}

/// API view for a token transfer event.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TokenTransferView {
    pub id: String,
    pub h_app_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub amount: f32,
    pub governance_layer: String,
    pub note: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
}

/// Input view for creating a token transfer — camelCase API boundary type.
/// Accepted by HTTP handlers; converted to internal DB types by the service layer.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateTokenTransferInputView {
    /// Optional client-supplied ID. If absent, the service generates a UUID.
    #[serde(default)]
    pub id: Option<String>,
    pub from_agent: String,
    pub to_agent: String,
    pub amount: f32,
    /// Governance layer for this transfer (e.g. `"individual"`, `"household"`).
    #[serde(default = "default_governance_layer")]
    pub governance_layer: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// API view for a responsibility demand curve config.
///
/// Encodes the per-layer parameters that couple token accumulation with
/// obligation. `enforcementActive` is coerced from SQLite INTEGER (0/1) to bool.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ResponsibilityDemandConfigView {
    pub id: String,
    pub governance_layer: String,
    pub dignity_floor: f32,
    pub median_estimate: f32,
    pub soft_ceiling_multiplier: f32,
    pub hard_ceiling_multiplier: f32,
    pub social_contract_health: f32,
    /// Coerced from INTEGER (0/1) to bool at the API boundary.
    pub enforcement_active: bool,
    pub ratified_by: Option<String>,
    pub ratified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input view for creating a responsibility demand config — camelCase API boundary type.
///
/// All curve parameters are optional; the service layer applies protocol defaults
/// when absent (dignity_floor=100, median_estimate=1000, multipliers=10/20,
/// social_contract_health=0.5, enforcement_active=true).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateResponsibilityDemandConfigInputView {
    /// Governance layer this config applies to (e.g. `"individual"`, `"household"`).
    pub governance_layer: String,
    pub dignity_floor: Option<f32>,
    pub median_estimate: Option<f32>,
    pub soft_ceiling_multiplier: Option<f32>,
    pub hard_ceiling_multiplier: Option<f32>,
    pub social_contract_health: Option<f32>,
    /// Whether the curve is actively enforced. Defaults to `true`.
    pub enforcement_active: Option<bool>,
}

/// API view for a token decay event.
///
/// Each decay event records one periodic balance reduction applied to an agent,
/// including the before/after balances, the decay amount, the obligation level
/// that triggered the decay, and the dignity floor that was enforced.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TokenDecayEventView {
    pub id: String,
    pub agent_id: String,
    pub governance_layer: String,
    pub balance_before: f32,
    pub balance_after: f32,
    pub decay_amount: f32,
    pub obligation_level: String,
    pub dignity_floor: f32,
    pub created_at: String,
}

/// Input view for the discernment mint pathway.
///
/// Discernment mints are elohim-attested awards for demonstrated judgment —
/// qualitative recognition that cannot be reduced to an REA provenance event.
/// The elohim attestation and reasoning trace are mandatory: they form the
/// audit record that allows constitutional review of elohim mint decisions.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DiscernmentMintInputView {
    /// Agent receiving the discernment mint.
    pub agent_id: String,
    /// Governance layer for the mint (e.g. `"individual"`, `"household"`).
    /// Defaults to `"individual"` when absent.
    #[serde(default)]
    pub governance_layer: Option<String>,
    /// Token amount to mint.
    pub amount: f32,
    /// Identifier of the elohim agent making this attestation.
    pub elohim_attestation: String,
    /// Free-form reasoning trace from the elohim agent explaining the award.
    pub reasoning_trace: String,
    /// Optional EPR content reference that grounded the discernment decision.
    #[serde(default)]
    pub source_epr_id: Option<String>,
}

/// Collective response view
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CollectiveView {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub governance_layer: String,
    pub constitutional_parent_id: Option<String>,
    pub reach: String,
    pub metadata: Option<JsonVal>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub dissolved_at: Option<String>,
}

/// Create collective input view
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateCollectiveInputView {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub governance_layer: String,
    #[serde(default)]
    pub constitutional_parent_id: Option<String>,
    #[serde(default)]
    pub reach: Option<String>,
    #[serde(default)]
    pub metadata: Option<JsonVal>,
    #[serde(default)]
    pub created_by: Option<String>,
}

/// Collective participation response view
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CollectiveParticipationView {
    pub id: String,
    pub collective_id: String,
    pub human_id: String,
    pub intimacy_level: String,
    pub role_context: Option<String>,
    pub governance_weight: f32,
    pub consent_state: String,
    pub metadata: Option<JsonVal>,
    pub joined_at: String,
    pub updated_at: String,
    pub departed_at: Option<String>,
}

/// Relationship seed within an account package
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RelationshipSeedView {
    pub target_id: String,
    pub relationship_type: String,
    pub intimacy_level: String,
    #[serde(default)]
    pub is_bidirectional: bool,
    #[serde(default)]
    pub reach: Option<String>,
}

/// Stewardship seed within an account package
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardshipSeedView {
    pub content_category: String,
    pub allocation_ratio: f32,
    #[serde(default)]
    pub contribution_type: Option<String>,
}

/// Collective seed within an account package
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CollectiveSeedView {
    pub collective_id: String,
    #[serde(default)]
    pub role_context: Option<String>,
    #[serde(default)]
    pub intimacy_level: Option<String>,
}

/// Resource allocation percentages for a single governance level.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GovernanceLevelAllocationView {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub storage_percent: f64,
    pub bandwidth_percent: f64,
}

/// A specific allocation block for a purpose (e.g., "10% CPU for Lamad family learning").
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AllocationBlockView {
    pub id: String,
    pub label: String,
    /// "individual" | "household" | "community" | "network"
    pub governance_level: String,
    pub priority: u32,
    pub cpu_cores: f64,
    pub cpu_percent: f64,
    pub memory_gb: f64,
    pub memory_percent: f64,
    pub storage_gb: f64,
    pub storage_percent: f64,
    pub bandwidth_mbps: f64,
    pub bandwidth_percent: f64,
    pub utilized_cpu_percent: f64,
    pub utilized_memory_percent: f64,
    pub utilized_storage_percent: f64,
    pub utilized_bandwidth_percent: f64,
    pub commitment_id: Option<String>,
    pub related_agents: Vec<String>,
}

/// How much compute is allocated to family-community.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AllocationSnapshotView {
    pub by_governance_individual: GovernanceLevelAllocationView,
    pub by_governance_household: GovernanceLevelAllocationView,
    pub by_governance_community: GovernanceLevelAllocationView,
    pub by_governance_network: GovernanceLevelAllocationView,
    pub total_allocated_cpu_percent: f64,
    pub total_allocated_memory_percent: f64,
    pub total_allocated_storage_percent: f64,
    pub total_allocated_bandwidth_percent: f64,
    pub allocation_blocks: Vec<AllocationBlockView>,
}

/// Infrastructure-token balance and earnings.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct InfrastructureTokenBalanceView {
    pub balance_tokens: f64,
    pub balance_estimated_value: f64,
    pub balance_currency: String,
    pub earning_rate_tokens_per_hour: f64,
    pub earning_rate_cpu_allocation: f64,
    pub earning_rate_storage_allocation: f64,
    pub earning_rate_bandwidth_allocation: f64,
    pub earning_rate_estimated_monthly: f64,
    pub decay_demurrage_rate: f64,
    pub decay_last_calculated: String,
    pub decay_projected_next_month_tokens: f64,
    pub decay_projected_next_month_value_usd: f64,
    pub token_history_last_24h: f64,
    pub token_history_last_7d: f64,
    pub token_history_last_30d: f64,
    pub token_history_all_time: f64,
    pub transactions: Vec<TokenTransactionView>,
    pub exchange_rates: Vec<ExchangeRateView>,
}

/// A token earning or spending event.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TokenTransactionView {
    pub id: String,
    pub timestamp: String,
    /// "earned" | "transferred" | "exchanged" | "decayed" | "claimed"
    pub transaction_type: String,
    pub amount: f64,
    pub related_agent: Option<String>,
    pub description: String,
    pub economic_event_id: Option<String>,
}

/// Cross-swimlane token exchange rate.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ExchangeRateView {
    pub from: String,
    pub to: String,
    pub rate: f64,
    /// "market" | "consensus" | "algorithm"
    pub source: String,
    pub last_updated: String,
}

/// A single compute-related hREA economic event.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecentEconomicEventView {
    pub id: String,
    pub timestamp: String,
    pub event_type: String,
    pub provider: Option<String>,
    pub receiver: Option<String>,
    pub quantity_has_unit: String,
    pub quantity_has_numerical_value: f64,
    pub tokens_minted: Option<f64>,
    pub note: String,
}

/// Dignity floor enforcement status.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DignityFloorView {
    pub compute_min_cores: f64,
    pub compute_min_memory_gb: f64,
    pub compute_min_storage_gb: f64,
    pub compute_min_bandwidth_mbps: f64,
    /// "met" | "warning" | "breached"
    pub status: String,
    pub percent_of_floor: f64,
    /// "voluntary" | "progressive" | "hard"
    pub enforcement: String,
}

/// Ceiling limit enforcement status.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CeilingLimitView {
    pub compute_max_cores: f64,
    pub compute_max_memory_gb: f64,
    pub compute_max_storage_gb: f64,
    pub compute_max_bandwidth_mbps: f64,
    pub token_accumulation_ceiling: f64,
    pub current_accumulation: f64,
    pub percent_of_ceiling: f64,
    /// "safe" | "warning" | "breached"
    pub status: String,
    /// "voluntary" | "progressive" | "hard"
    pub enforcement: String,
}

/// A constitutional limit violation alert.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ConstitutionalAlertView {
    pub id: String,
    /// "info" | "warning" | "critical"
    pub severity: String,
    /// "floor-breach" | "ceiling-breach" | "redistribution-required"
    pub alert_type: String,
    pub message: String,
    pub affected_resource: String,
    pub current_value: f64,
    pub threshold: f64,
    pub recommended_action: String,
    pub timestamp: String,
}

/// Dignity floor and ceiling enforcement status.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ConstitutionalLimitsStatusView {
    pub dignity_floor: DignityFloorView,
    pub ceiling_limit: CeilingLimitView,
    pub safe_zone_cpu: f64,
    pub safe_zone_memory: f64,
    pub safe_zone_storage: f64,
    pub safe_zone_bandwidth: f64,
    pub safe_zone_tokens: f64,
    pub alerts: Vec<ConstitutionalAlertView>,
}

/// Complete state for the operator's Shefa compute dashboard.
///
/// Assembled server-side from compute metrics, allocations, protection
/// status, token economics, and constitutional limits. Angular is a thin
/// display client — no aggregation happens in TypeScript.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SheafaDashboardStateView {
    // Identity
    pub operator_id: String,
    pub operator_name: String,
    pub stewarded_resource_id: String,
    pub node_id: String,
    pub node_location_region: Option<String>,
    pub node_location_country: Option<String>,
    pub node_location_latitude: Option<f64>,
    pub node_location_longitude: Option<f64>,
    // Status
    /// "online" | "offline" | "degraded" | "maintenance"
    pub status: String,
    pub last_heartbeat: String,
    pub uptime: UpTimeMetricsView,
    // Compute
    pub compute_metrics: ComputeMetricsView,
    pub allocations: AllocationSnapshotView,
    // Protection
    pub family_community_protection: FamilyCommunityProtectionStatusView,
    // Economics
    pub infrastructure_tokens: InfrastructureTokenBalanceView,
    pub economic_events: Vec<RecentEconomicEventView>,
    // Constitutional
    pub constitutional_limits: ConstitutionalLimitsStatusView,
    // Timestamps
    pub last_updated: String,
    pub update_frequency_ms: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardCredentialView {
    pub id: String,
    pub presence_id: String,
    pub content_id: String,
    pub affinity_coefficient: f32,
    pub credential_type: String,
    pub status: String,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PremiumGateView {
    pub id: String,
    pub steward_credential_id: String,
    pub steward_presence_id: String,
    pub gated_resource_type: String,
    pub gated_resource_ids: JsonVal,
    pub gate_title: String,
    pub gate_description: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AccessGrantView {
    pub id: String,
    pub gate_id: String,
    pub grantee_presence_id: String,
    pub contributor_presence_id: Option<String>,
    pub granted_at: String,
    pub expires_at: Option<String>,
    pub status: String,
    pub dht_anchor_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardRevenueSummaryView {
    pub total_credentials: i64,
    pub total_gates: i64,
    pub total_grants: i64,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateCredentialInputView {
    pub presence_id: String,
    pub content_id: String,
    pub affinity_coefficient: f32,
    pub credential_type: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateGateInputView {
    pub steward_credential_id: String,
    pub steward_presence_id: String,
    pub gated_resource_type: String,
    pub gated_resource_ids: JsonVal,
    pub gate_title: String,
    pub gate_description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateGrantInputView {
    pub gate_id: String,
    pub grantee_presence_id: String,
    pub contributor_presence_id: Option<String>,
    pub expires_at: Option<String>,
}

/// REA Commitment — API input
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateReaCommitmentInputView {
    #[serde(default)]
    pub id: Option<String>,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_classified_as: Option<Vec<String>>,
    #[serde(default)]
    pub resource_quantity: Option<MeasureView>,
    #[serde(default)]
    pub effort_quantity: Option<MeasureView>,
    #[serde(default)]
    pub has_beginning: Option<String>,
    #[serde(default)]
    pub has_end: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub in_scope_of: Option<Vec<String>>,
    #[serde(default)]
    pub medium_of_exchange_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata: Option<JsonVal>,
}

/// State update input
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateReaCommitmentStateView {
    pub state: String,
    #[serde(default)]
    pub finished: Option<bool>,
}

/// REA Agreement — API output
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AgreementView {
    pub id: String,
    pub name: Option<String>,
    pub note: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub metadata: Option<JsonVal>,
    pub created_at: String,
}

/// Create agreement — API input
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateAgreementInputView {
    #[serde(default)]
    pub id: Option<String>,
    pub name: Option<String>,
    pub note: Option<String>,
    pub metadata: Option<JsonVal>,
}

/// API output for a stewarded node, with stewards joined in.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardedNodeView {
    pub id: String,
    pub display_name: String,
    pub claim_status: String,
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub storage_tb: f64,
    pub bandwidth_mbps: i32,
    pub steward_tier: String,
    pub custodian_opt_in: bool,
    pub region: Option<String>,
    pub context_epr_id: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub stewards: Vec<NodeStewardshipView>,
}

/// API output for a single node–human stewardship record.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeStewardshipView {
    pub human_id: String,
    pub display_name: String,
    pub affinity_score: f64,
    pub relationship: String,
    pub context_epr_id: Option<String>,
    pub granted_at: String,
}

/// API input for registering a new stewarded node.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateStewardedNodeInputView {
    pub id: String,
    pub display_name: String,
    #[serde(default = "default_claim_status")]
    pub claim_status: String,
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub storage_tb: f64,
    pub bandwidth_mbps: i32,
    #[serde(default = "default_steward_tier")]
    pub steward_tier: String,
    #[serde(default = "default_true")]
    pub custodian_opt_in: bool,
    pub region: Option<String>,
    pub context_epr_id: Option<String>,
}

/// API input for adding a stewardship relationship to a node.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateNodeStewardshipInputView {
    pub node_id: String,
    pub human_id: String,
    pub affinity_score: f64,
    #[serde(default = "default_relationship")]
    pub relationship: String,
    pub context_epr_id: Option<String>,
}

/// API input for triggering recognition distribution
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecognitionTriggerInputView {
    pub content_id: String,
    pub event_type: String,
    pub raw_amount: f64,
    #[serde(default)]
    pub triggered_by: Option<String>,
}

/// Per-steward trace in the distribution result
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StageTraceView {
    pub steward_presence_id: String,
    pub allocation_ratio: f32,
    pub stored_affinity: f64,
    pub derived_affinity: f64,
    pub effective_ratio: f64,
    pub pre_limit_share: f64,
    pub final_share: f64,
    pub limit_reasons: Vec<JsonVal>,
    pub economic_event_id: String,
}

/// Full pipeline result
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RecognitionDistributionResultView {
    pub content_id: String,
    pub trigger_event_type: String,
    pub raw_amount: f64,
    pub weighted_amount: f64,
    pub distributions: Vec<StageTraceView>,
    pub economic_event_ids: Vec<String>,
    pub limits_applied: Vec<JsonVal>,
}

/// Steward affinity output view
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardAffinityView {
    pub id: String,
    pub steward_id: String,
    pub content_id: String,
    pub affinity_score: f32,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for creating steward affinity via API
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateStewardAffinityInputView {
    pub steward_id: String,
    pub content_id: String,
    #[serde(default)]
    pub affinity_score: f32,
    #[serde(default)]
    pub source: Option<String>,
}

/// Bulk create input
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BulkCreateStewardAffinityInputView {
    pub affinities: Vec<CreateStewardAffinityInputView>,
}

/// Per-counterparty row in a reciprocity view's inflow/outflow ledger.
///
/// `honored_percent` is `f64` so PartialEq is impl'd but Eq is not — matches
/// JSON Schema's `number` with `minimum: 0`.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ReciprocityRow {
    pub counterparty_household_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub committed_bytes: u64,
    pub delivered_bytes: u64,
    pub honored_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub online: Option<bool>,
}

/// Per-agent reciprocity ledger.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct ReciprocityView {
    pub agent_cid: String,
    pub inflow: Vec<ReciprocityRow>,
    pub outflow: Vec<ReciprocityRow>,
    pub net_hosted_bytes: i64,
    pub capacity_available_bytes: u64,
}

/// Per-storage-steward row in a doorway dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
#[serde(rename_all = "camelCase")]
pub struct DashboardSteward {
    pub peer_id: String,
    pub archetype: DeviceArchetype,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub online: bool,
    pub hosting_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hop_hint: Option<u32>,
}

