//! Wire types for avodah (work/services) domain coordinator functions.
//!
//! These types define the MessagePack-serialized inputs and outputs for
//! content_store zome calls related to service marketplace, flow planning,
//! and insurance mutual. They are consumed by:
//! - The content_store coordinator zome (WASM target)
//! - Doorway gateway service (native target)
//! - elohim-storage (native target)
//!
//! This crate is an IoC artifact in `sdk/domains/avodah/`, alongside
//! the domain's schemas and manifest. It must NOT depend on HDK, HDI,
//! or any WASM-specific crates.

use holo_hash::{ActionHash, EntryHash};
use serde::{Deserialize, Serialize};

// =============================================================================
// Service Marketplace: ServiceRequest
// =============================================================================

/// Input for content_store::create_service_request coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateServiceRequestInput {
    pub id: String,
    pub request_number: String,
    pub requester_id: String,
    pub title: String,
    pub description: String,
    pub contact_preference: String,
    pub contact_value: String,
    pub time_zone: String,
    pub time_preference: String,
    pub interaction_type: String,
    pub date_range_start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_range_end: Option<String>,
    pub service_type_ids_json: String,
    pub required_skills_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_unit: Option<String>,
    pub medium_of_exchange_ids_json: String,
    pub status: String,
    pub is_public: bool,
    pub links_json: String,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// ServiceRequest wire type. Mirrors the integrity zome's ServiceRequest entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServiceRequest {
    pub id: String,
    pub request_number: String,
    pub requester_id: String,
    pub title: String,
    pub description: String,
    pub contact_preference: String,
    pub contact_value: String,
    pub time_zone: String,
    pub time_preference: String,
    pub interaction_type: String,
    pub date_range_start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_range_end: Option<String>,
    pub service_type_ids_json: String,
    pub required_skills_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_unit: Option<String>,
    pub medium_of_exchange_ids_json: String,
    pub status: String,
    pub is_public: bool,
    pub links_json: String,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from content_store::get_service_request coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServiceRequestOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub request: ServiceRequest,
}

/// Query input for service requests by requester.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetServiceRequestInput {
    pub request_id: String,
}

// =============================================================================
// Service Marketplace: ServiceOffer
// =============================================================================

/// Input for content_store::create_service_offer coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateServiceOfferInput {
    pub id: String,
    pub offer_number: String,
    pub offeror_id: String,
    pub title: String,
    pub description: String,
    pub contact_preference: String,
    pub contact_value: String,
    pub time_zone: String,
    pub time_preference: String,
    pub interaction_type: String,
    pub hours_per_week: f64,
    pub date_range_start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_range_end: Option<String>,
    pub service_type_ids_json: String,
    pub offered_skills_json: String,
    pub rate_value: f64,
    pub rate_unit: String,
    pub rate_per: String,
    pub medium_of_exchange_ids_json: String,
    pub accepts_alternative_payment: bool,
    pub status: String,
    pub is_public: bool,
    pub links_json: String,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// ServiceOffer wire type. Mirrors the integrity zome's ServiceOffer entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServiceOffer {
    pub id: String,
    pub offer_number: String,
    pub offeror_id: String,
    pub title: String,
    pub description: String,
    pub contact_preference: String,
    pub contact_value: String,
    pub time_zone: String,
    pub time_preference: String,
    pub interaction_type: String,
    pub hours_per_week: f64,
    pub date_range_start: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_range_end: Option<String>,
    pub service_type_ids_json: String,
    pub offered_skills_json: String,
    pub rate_value: f64,
    pub rate_unit: String,
    pub rate_per: String,
    pub medium_of_exchange_ids_json: String,
    pub accepts_alternative_payment: bool,
    pub status: String,
    pub is_public: bool,
    pub links_json: String,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from content_store::get_service_offer coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServiceOfferOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub offer: ServiceOffer,
}

/// Query input for service offers by ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetServiceOfferInput {
    pub offer_id: String,
}

// =============================================================================
// Service Marketplace: ServiceMatch
// =============================================================================

/// Input for content_store::create_service_match coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateServiceMatchInput {
    pub id: String,
    pub request_id: String,
    pub offer_id: String,
    pub match_reason: String,
    pub match_quality: u32,
    pub shared_service_types_json: String,
    pub time_compatible: bool,
    pub interaction_compatible: bool,
    pub exchange_compatible: bool,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commitment_id: Option<String>,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// ServiceMatch wire type. Mirrors the integrity zome's ServiceMatch entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServiceMatch {
    pub id: String,
    pub request_id: String,
    pub offer_id: String,
    pub match_reason: String,
    pub match_quality: u32,
    pub shared_service_types_json: String,
    pub time_compatible: bool,
    pub interaction_compatible: bool,
    pub exchange_compatible: bool,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commitment_id: Option<String>,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from content_store::get_service_match coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ServiceMatchOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub service_match: ServiceMatch,
}

/// Query input for service matches by ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetServiceMatchInput {
    pub match_id: String,
}

// =============================================================================
// Flow Planning: FlowPlan
// =============================================================================

/// Input for content_store::create_flow_plan coordinator function.
///
/// Note: `steward_id` and timestamps are represented as String/i64
/// because this crate cannot depend on HDK types (AgentPubKey, Timestamp).
/// The zome converts at the construction site.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateFlowPlanInput {
    pub id: String,
    pub plan_number: String,
    pub steward_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub time_horizon: String,
    pub plan_period_start: i64,
    pub plan_period_end: i64,
    #[serde(default)]
    pub resource_scopes: Vec<String>,
    #[serde(default)]
    pub included_resource_ids: Vec<String>,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub milestones: Vec<String>,
    #[serde(default)]
    pub budgets: Vec<String>,
    pub status: String,
    pub confidence_score: u8,
    pub completion_percent: u8,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reviewed_at: Option<i64>,
    pub next_review_due: i64,
    pub plan_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
}

/// FlowPlan wire type. Mirrors the integrity zome's FlowPlan entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlowPlan {
    pub id: String,
    pub plan_number: String,
    pub steward_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub time_horizon: String,
    pub plan_period_start: i64,
    pub plan_period_end: i64,
    #[serde(default)]
    pub resource_scopes: Vec<String>,
    #[serde(default)]
    pub included_resource_ids: Vec<String>,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub milestones: Vec<String>,
    #[serde(default)]
    pub budgets: Vec<String>,
    pub status: String,
    pub confidence_score: u8,
    pub completion_percent: u8,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reviewed_at: Option<i64>,
    pub next_review_due: i64,
    pub plan_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
}

/// Output from content_store::get_flow_plan coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlowPlanOutput {
    pub action_hash: ActionHash,
    pub plan: FlowPlan,
}

/// Query input for flow plans by steward.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetPlansForStewardInput {
    pub steward_id: String,
}

// =============================================================================
// Flow Planning: FlowBudget
// =============================================================================

/// Input for content_store::create_flow_budget coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateFlowBudgetInput {
    pub id: String,
    pub budget_number: String,
    pub plan_id: String,
    pub steward_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub budget_period: String,
    pub period_start: i64,
    pub period_end: i64,
    pub categories_json: String,
    pub total_planned: f64,
    pub total_actual: f64,
    pub variance: f64,
    pub variance_percent: f64,
    pub status: String,
    pub health_status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_reconciled: i64,
    pub budget_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
}

/// FlowBudget wire type. Mirrors the integrity zome's FlowBudget entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlowBudget {
    pub id: String,
    pub budget_number: String,
    pub plan_id: String,
    pub steward_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub budget_period: String,
    pub period_start: i64,
    pub period_end: i64,
    pub categories_json: String,
    pub total_planned: f64,
    pub total_actual: f64,
    pub variance: f64,
    pub variance_percent: f64,
    pub status: String,
    pub health_status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_reconciled: i64,
    pub budget_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
}

// =============================================================================
// Flow Planning: FlowGoal
// =============================================================================

/// Input for content_store::create_flow_goal coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateFlowGoalInput {
    pub id: String,
    pub goal_number: String,
    pub plan_id: String,
    pub steward_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub goal_type: String,
    pub target_metric: String,
    pub target_value: f64,
    pub target_unit: String,
    pub current_value: f64,
    pub starting_value: f64,
    pub deadline: i64,
    pub progress_percent: u8,
    pub on_track: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_completion_date: Option<i64>,
    pub linked_resource_ids_json: String,
    pub linked_budget_ids_json: String,
    pub blocked_by_json: String,
    pub status: String,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    pub goal_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
}

/// FlowGoal wire type. Mirrors the integrity zome's FlowGoal entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlowGoal {
    pub id: String,
    pub goal_number: String,
    pub plan_id: String,
    pub steward_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub goal_type: String,
    pub target_metric: String,
    pub target_value: f64,
    pub target_unit: String,
    pub current_value: f64,
    pub starting_value: f64,
    pub deadline: i64,
    pub progress_percent: u8,
    pub on_track: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_completion_date: Option<i64>,
    pub linked_resource_ids_json: String,
    pub linked_budget_ids_json: String,
    pub blocked_by_json: String,
    pub status: String,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    pub goal_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
}

// =============================================================================
// Flow Planning: FlowMilestone
// =============================================================================

/// Input for content_store::create_flow_milestone coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateFlowMilestoneInput {
    pub id: String,
    pub milestone_number: String,
    pub plan_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub target_date: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_date: Option<i64>,
    pub success_criteria_json: String,
    pub all_criteria_met: bool,
    pub depends_on_goals_json: String,
    pub depends_on_milestones_json: String,
    pub blocks_goals_json: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achieved_at: Option<i64>,
    pub milestone_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
}

/// FlowMilestone wire type. Mirrors the integrity zome's FlowMilestone entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlowMilestone {
    pub id: String,
    pub milestone_number: String,
    pub plan_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub target_date: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_date: Option<i64>,
    pub success_criteria_json: String,
    pub all_criteria_met: bool,
    pub depends_on_goals_json: String,
    pub depends_on_milestones_json: String,
    pub blocks_goals_json: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achieved_at: Option<i64>,
    pub milestone_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
}

// =============================================================================
// Flow Planning: FlowScenario
// =============================================================================

/// Input for content_store::create_flow_scenario coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateFlowScenarioInput {
    pub id: String,
    pub scenario_number: String,
    pub plan_id: String,
    pub steward_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub scenario_type: String,
    pub changes_json: String,
    pub projections_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_scenario_id: Option<String>,
    pub delta_metrics_json: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulated_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub scenario_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
}

/// FlowScenario wire type. Mirrors the integrity zome's FlowScenario entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlowScenario {
    pub id: String,
    pub scenario_number: String,
    pub plan_id: String,
    pub steward_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub scenario_type: String,
    pub changes_json: String,
    pub projections_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_scenario_id: Option<String>,
    pub delta_metrics_json: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulated_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub scenario_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
}

// =============================================================================
// Flow Planning: FlowProjection
// =============================================================================

/// Input for content_store::create_flow_projection coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateFlowProjectionInput {
    pub id: String,
    pub projection_number: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_id: Option<String>,
    pub steward_id: String,
    pub resource_category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    pub projection_start: i64,
    pub projection_end: i64,
    pub projection_horizon: String,
    pub data_points_json: String,
    pub confidence_level: String,
    pub confidence_percent: u8,
    pub projection_method: String,
    pub assumptions_json: String,
    pub breakpoints_json: String,
    pub created_at: i64,
    pub schema_version: u32,
    pub validation_status: String,
}

/// FlowProjection wire type. Mirrors the integrity zome's FlowProjection entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FlowProjection {
    pub id: String,
    pub projection_number: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_id: Option<String>,
    pub steward_id: String,
    pub resource_category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    pub projection_start: i64,
    pub projection_end: i64,
    pub projection_horizon: String,
    pub data_points_json: String,
    pub confidence_level: String,
    pub confidence_percent: u8,
    pub projection_method: String,
    pub assumptions_json: String,
    pub breakpoints_json: String,
    pub created_at: i64,
    pub schema_version: u32,
    pub validation_status: String,
}

// =============================================================================
// Flow Planning: RecurringPattern
// =============================================================================

/// Input for content_store::create_recurring_pattern coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateRecurringPatternInput {
    pub id: String,
    pub pattern_number: String,
    pub steward_id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub frequency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_value: Option<u32>,
    pub expected_amount: f64,
    pub expected_unit: String,
    pub variance_expected: u8,
    pub start_date: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<i64>,
    pub next_due_date: i64,
    pub resource_category: String,
    pub pattern_type: String,
    pub auto_generate: bool,
    pub historical_occurrences_json: String,
    pub missed_occurrences: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average_actual_amount: Option<f64>,
    pub reliability: u8,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub schema_version: u32,
    pub validation_status: String,
}

/// RecurringPattern wire type. Mirrors the integrity zome's RecurringPattern entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RecurringPattern {
    pub id: String,
    pub pattern_number: String,
    pub steward_id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub frequency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_value: Option<u32>,
    pub expected_amount: f64,
    pub expected_unit: String,
    pub variance_expected: u8,
    pub start_date: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<i64>,
    pub next_due_date: i64,
    pub resource_category: String,
    pub pattern_type: String,
    pub auto_generate: bool,
    pub historical_occurrences_json: String,
    pub missed_occurrences: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average_actual_amount: Option<f64>,
    pub reliability: u8,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub schema_version: u32,
    pub validation_status: String,
}

// =============================================================================
// Insurance Mutual: MemberRiskProfile
// =============================================================================

/// Input for content_store::create_member_risk_profile coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateMemberRiskProfileInput {
    pub id: String,
    pub member_id: String,
    pub risk_type: String,
    pub care_maintenance_score: f64,
    pub community_connectedness_score: f64,
    pub historical_claims_rate: f64,
    pub risk_score: f64,
    pub risk_tier: String,
    pub risk_tier_rationale: String,
    pub evidence_event_ids_json: String,
    pub evidence_breakdown_json: String,
    pub risk_trend_direction: String,
    pub last_risk_score: f64,
    pub assessed_at: String,
    pub last_assessment_at: String,
    pub next_assessment_due: String,
    pub assessment_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// MemberRiskProfile wire type. Mirrors the integrity zome's MemberRiskProfile entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MemberRiskProfile {
    pub id: String,
    pub member_id: String,
    pub risk_type: String,
    pub care_maintenance_score: f64,
    pub community_connectedness_score: f64,
    pub historical_claims_rate: f64,
    pub risk_score: f64,
    pub risk_tier: String,
    pub risk_tier_rationale: String,
    pub evidence_event_ids_json: String,
    pub evidence_breakdown_json: String,
    pub risk_trend_direction: String,
    pub last_risk_score: f64,
    pub assessed_at: String,
    pub last_assessment_at: String,
    pub next_assessment_due: String,
    pub assessment_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from content_store::get_member_risk_profile coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct MemberRiskProfileOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub profile: MemberRiskProfile,
}

// =============================================================================
// Insurance Mutual: CoveragePolicy
// =============================================================================

/// Input for content_store::create_coverage_policy coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateCoveragePolicyInput {
    pub id: String,
    pub member_id: String,
    pub coverage_level: String,
    pub governed_at: String,
    pub covered_risks_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deductible_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deductible_unit: Option<String>,
    pub coinsurance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_pocket_maximum_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_pocket_maximum_unit: Option<String>,
    pub effective_from: String,
    pub renewal_terms: String,
    pub renewal_due_at: String,
    pub constitutional_basis: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_premium_event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_premium_paid_at: Option<String>,
    pub schema_version: u32,
    pub validation_status: String,
    pub modification_event_ids_json: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// CoveragePolicy wire type. Mirrors the integrity zome's CoveragePolicy entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CoveragePolicy {
    pub id: String,
    pub member_id: String,
    pub coverage_level: String,
    pub governed_at: String,
    pub covered_risks_json: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deductible_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deductible_unit: Option<String>,
    pub coinsurance: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_pocket_maximum_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_of_pocket_maximum_unit: Option<String>,
    pub effective_from: String,
    pub renewal_terms: String,
    pub renewal_due_at: String,
    pub constitutional_basis: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_premium_event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_premium_paid_at: Option<String>,
    pub schema_version: u32,
    pub validation_status: String,
    pub modification_event_ids_json: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from content_store::get_coverage_policy coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CoveragePolicyOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub policy: CoveragePolicy,
}

// =============================================================================
// Insurance Mutual: InsuranceClaim
// =============================================================================

/// Input for content_store::create_insurance_claim coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateInsuranceClaimInput {
    pub id: String,
    pub claim_number: String,
    pub policy_id: String,
    pub member_id: String,
    pub filed_date: String,
    pub filed_by: String,
    pub loss_type: String,
    pub loss_date: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_amount_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_amount_unit: Option<String>,
    pub observer_attestation_ids_json: String,
    pub member_document_ids_json: String,
    pub status: String,
    pub status_history_json: String,
    pub adjustment_event_ids_json: String,
    pub appeal_event_ids_json: String,
    pub settlement_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// InsuranceClaim wire type. Mirrors the integrity zome's InsuranceClaim entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InsuranceClaim {
    pub id: String,
    pub claim_number: String,
    pub policy_id: String,
    pub member_id: String,
    pub filed_date: String,
    pub filed_by: String,
    pub loss_type: String,
    pub loss_date: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_amount_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_amount_unit: Option<String>,
    pub observer_attestation_ids_json: String,
    pub member_document_ids_json: String,
    pub status: String,
    pub status_history_json: String,
    pub adjustment_event_ids_json: String,
    pub appeal_event_ids_json: String,
    pub settlement_event_ids_json: String,
    pub schema_version: u32,
    pub validation_status: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Output from content_store::get_insurance_claim coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct InsuranceClaimOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub claim: InsuranceClaim,
}

// =============================================================================
// Insurance Mutual: AdjustmentReasoning
// =============================================================================

/// Input for content_store::create_adjustment_reasoning coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateAdjustmentReasoningInput {
    pub id: String,
    pub claim_id: String,
    pub adjuster_id: String,
    pub coverage_decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_amount_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_amount_unit: Option<String>,
    pub plain_language_explanation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpretation_notes: Option<String>,
    pub applied_generosity_principle: bool,
    pub constitutional_basis_documents_json: String,
    pub policy_citations_json: String,
    pub flagged_for_governance: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_review_reason: Option<String>,
    pub adjustment_date: String,
    pub created_at: String,
}

/// AdjustmentReasoning wire type. Mirrors the integrity zome's AdjustmentReasoning entry type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AdjustmentReasoning {
    pub id: String,
    pub claim_id: String,
    pub adjuster_id: String,
    pub coverage_decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_amount_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_amount_unit: Option<String>,
    pub plain_language_explanation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpretation_notes: Option<String>,
    pub applied_generosity_principle: bool,
    pub constitutional_basis_documents_json: String,
    pub policy_citations_json: String,
    pub flagged_for_governance: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_review_reason: Option<String>,
    pub adjustment_date: String,
    pub created_at: String,
}

/// Output from content_store::get_adjustment_reasoning coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct AdjustmentReasoningOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub reasoning: AdjustmentReasoning,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_request_msgpack_roundtrip() {
        let input = CreateServiceRequestInput {
            id: "sr-001".into(),
            request_number: "REQ-0000000001".into(),
            requester_id: "agent-001".into(),
            title: "Need plumbing help".into(),
            description: "Kitchen sink is leaking".into(),
            contact_preference: "message".into(),
            contact_value: "agent-001".into(),
            time_zone: "America/New_York".into(),
            time_preference: "morning".into(),
            interaction_type: "in-person".into(),
            date_range_start: "2026-04-01".into(),
            date_range_end: None,
            service_type_ids_json: "[]".into(),
            required_skills_json: "[\"plumbing\"]".into(),
            budget_value: Some(100.0),
            budget_unit: Some("USD".into()),
            medium_of_exchange_ids_json: "[]".into(),
            status: "pending".into(),
            is_public: false,
            links_json: "[]".into(),
            schema_version: 1,
            validation_status: "Valid".into(),
            metadata_json: "{}".into(),
            created_at: "2026-04-01T00:00:00Z".into(),
            updated_at: "2026-04-01T00:00:00Z".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateServiceRequestInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "sr-001");
        assert_eq!(decoded.budget_value, Some(100.0));
    }

    #[test]
    fn service_offer_msgpack_roundtrip() {
        let input = CreateServiceOfferInput {
            id: "so-001".into(),
            offer_number: "OFR-0000000001".into(),
            offeror_id: "agent-002".into(),
            title: "Plumbing services".into(),
            description: "Licensed plumber".into(),
            contact_preference: "phone".into(),
            contact_value: "555-1234".into(),
            time_zone: "America/New_York".into(),
            time_preference: "any".into(),
            interaction_type: "in-person".into(),
            hours_per_week: 20.0,
            date_range_start: "2026-04-01".into(),
            date_range_end: None,
            service_type_ids_json: "[]".into(),
            offered_skills_json: "[\"plumbing\"]".into(),
            rate_value: 75.0,
            rate_unit: "USD".into(),
            rate_per: "hour".into(),
            medium_of_exchange_ids_json: "[]".into(),
            accepts_alternative_payment: true,
            status: "active".into(),
            is_public: true,
            links_json: "[]".into(),
            schema_version: 1,
            validation_status: "Valid".into(),
            metadata_json: "{}".into(),
            created_at: "2026-04-01T00:00:00Z".into(),
            updated_at: "2026-04-01T00:00:00Z".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateServiceOfferInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "so-001");
        assert_eq!(decoded.rate_value, 75.0);
    }

    #[test]
    fn service_match_msgpack_roundtrip() {
        let input = CreateServiceMatchInput {
            id: "sm-001".into(),
            request_id: "sr-001".into(),
            offer_id: "so-001".into(),
            match_reason: "Skills overlap".into(),
            match_quality: 85,
            shared_service_types_json: "[\"plumbing\"]".into(),
            time_compatible: true,
            interaction_compatible: true,
            exchange_compatible: true,
            status: "suggested".into(),
            proposal_id: None,
            commitment_id: None,
            schema_version: 1,
            validation_status: "Valid".into(),
            metadata_json: "{}".into(),
            created_at: "2026-04-01T00:00:00Z".into(),
            updated_at: "2026-04-01T00:00:00Z".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateServiceMatchInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "sm-001");
        assert_eq!(decoded.match_quality, 85);
    }

    #[test]
    fn flow_plan_msgpack_roundtrip() {
        let input = CreateFlowPlanInput {
            id: "fp-001".into(),
            plan_number: "FP-0000000001".into(),
            steward_id: "agent-001".into(),
            name: "Monthly budget plan".into(),
            description: Some("Track monthly spending".into()),
            time_horizon: "monthly".into(),
            plan_period_start: 1711929600000000,
            plan_period_end: 1714521600000000,
            resource_scopes: vec!["energy".into()],
            included_resource_ids: vec![],
            goals: vec![],
            milestones: vec![],
            budgets: vec![],
            status: "draft".into(),
            confidence_score: 70,
            completion_percent: 0,
            created_at: 1711929600000000,
            activated_at: None,
            completed_at: None,
            last_reviewed_at: None,
            next_review_due: 1714521600000000,
            plan_event_ids_json: "[]".into(),
            schema_version: 1,
            validation_status: "Valid".into(),
            metadata_json: "{}".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateFlowPlanInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "fp-001");
        assert_eq!(decoded.confidence_score, 70);
    }

    #[test]
    fn flow_budget_msgpack_roundtrip() {
        let input = CreateFlowBudgetInput {
            id: "fb-001".into(),
            budget_number: "FB-0000000001".into(),
            plan_id: "fp-001".into(),
            steward_id: "agent-001".into(),
            name: "Utilities budget".into(),
            description: None,
            budget_period: "monthly".into(),
            period_start: 1711929600000000,
            period_end: 1714521600000000,
            categories_json: "[]".into(),
            total_planned: 500.0,
            total_actual: 0.0,
            variance: 0.0,
            variance_percent: 0.0,
            status: "active".into(),
            health_status: "healthy".into(),
            created_at: 1711929600000000,
            updated_at: 1711929600000000,
            last_reconciled: 1711929600000000,
            budget_event_ids_json: "[]".into(),
            schema_version: 1,
            validation_status: "Valid".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateFlowBudgetInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "fb-001");
        assert_eq!(decoded.total_planned, 500.0);
    }

    #[test]
    fn member_risk_profile_msgpack_roundtrip() {
        let input = CreateMemberRiskProfileInput {
            id: "mrp-001".into(),
            member_id: "agent-001".into(),
            risk_type: "health".into(),
            care_maintenance_score: 85.0,
            community_connectedness_score: 70.0,
            historical_claims_rate: 0.1,
            risk_score: 25.0,
            risk_tier: "low".into(),
            risk_tier_rationale: "Good preventive care".into(),
            evidence_event_ids_json: "[]".into(),
            evidence_breakdown_json: "{}".into(),
            risk_trend_direction: "improving".into(),
            last_risk_score: 30.0,
            assessed_at: "2026-04-01T00:00:00Z".into(),
            last_assessment_at: "2026-03-01T00:00:00Z".into(),
            next_assessment_due: "2026-07-01T00:00:00Z".into(),
            assessment_event_ids_json: "[]".into(),
            schema_version: 1,
            validation_status: "Valid".into(),
            metadata_json: "{}".into(),
            created_at: "2026-04-01T00:00:00Z".into(),
            updated_at: "2026-04-01T00:00:00Z".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateMemberRiskProfileInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "mrp-001");
        assert_eq!(decoded.care_maintenance_score, 85.0);
    }

    #[test]
    fn coverage_policy_msgpack_roundtrip() {
        let input = CreateCoveragePolicyInput {
            id: "cp-001".into(),
            member_id: "agent-001".into(),
            coverage_level: "individual".into(),
            governed_at: "community-001".into(),
            covered_risks_json: "[]".into(),
            deductible_value: Some(500.0),
            deductible_unit: Some("USD".into()),
            coinsurance: 20.0,
            out_of_pocket_maximum_value: Some(5000.0),
            out_of_pocket_maximum_unit: Some("USD".into()),
            effective_from: "2026-01-01".into(),
            renewal_terms: "annual".into(),
            renewal_due_at: "2027-01-01".into(),
            constitutional_basis: "health-coverage-v1".into(),
            last_premium_event_id: None,
            last_premium_paid_at: None,
            schema_version: 1,
            validation_status: "Valid".into(),
            modification_event_ids_json: "[]".into(),
            metadata_json: "{}".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateCoveragePolicyInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "cp-001");
        assert_eq!(decoded.coinsurance, 20.0);
    }

    #[test]
    fn insurance_claim_msgpack_roundtrip() {
        let input = CreateInsuranceClaimInput {
            id: "ic-001".into(),
            claim_number: "CLM-0000000001".into(),
            policy_id: "cp-001".into(),
            member_id: "agent-001".into(),
            filed_date: "2026-04-01".into(),
            filed_by: "agent-001".into(),
            loss_type: "Emergency Medical".into(),
            loss_date: "2026-03-28".into(),
            description: "Emergency room visit".into(),
            estimated_amount_value: Some(2500.0),
            estimated_amount_unit: Some("USD".into()),
            observer_attestation_ids_json: "[]".into(),
            member_document_ids_json: "[]".into(),
            status: "filed".into(),
            status_history_json: "[]".into(),
            adjustment_event_ids_json: "[]".into(),
            appeal_event_ids_json: "[]".into(),
            settlement_event_ids_json: "[]".into(),
            schema_version: 1,
            validation_status: "Valid".into(),
            metadata_json: "{}".into(),
            created_at: "2026-04-01T00:00:00Z".into(),
            updated_at: "2026-04-01T00:00:00Z".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateInsuranceClaimInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "ic-001");
        assert_eq!(decoded.estimated_amount_value, Some(2500.0));
    }

    #[test]
    fn adjustment_reasoning_msgpack_roundtrip() {
        let input = CreateAdjustmentReasoningInput {
            id: "ar-001".into(),
            claim_id: "ic-001".into(),
            adjuster_id: "agent-002".into(),
            coverage_decision: "approved".into(),
            approved_amount_value: Some(2000.0),
            approved_amount_unit: Some("USD".into()),
            plain_language_explanation: "Covered under emergency medical".into(),
            interpretation_notes: None,
            applied_generosity_principle: true,
            constitutional_basis_documents_json: "[]".into(),
            policy_citations_json: "[]".into(),
            flagged_for_governance: false,
            governance_review_reason: None,
            adjustment_date: "2026-04-05".into(),
            created_at: "2026-04-05T00:00:00Z".into(),
        };
        let bytes = rmp_serde::to_vec_named(&input).unwrap();
        let decoded: CreateAdjustmentReasoningInput = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.id, "ar-001");
        assert_eq!(decoded.approved_amount_value, Some(2000.0));
    }
}
