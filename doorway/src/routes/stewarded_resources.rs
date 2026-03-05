//! Stewarded Resources API — Resource stewardship management
//!
//! REST endpoints for creating, querying, and managing stewarded resources,
//! allocations, usage tracking, dashboards, and constitutional limits.
//!
//! ## Routes
//!
//! - `POST   /api/v1/stewarded-resources`                                      → create resource
//! - `GET    /api/v1/stewarded-resources/{id}`                                  → get resource by ID
//! - `GET    /api/v1/stewarded-resources?stewardId=`                            → list steward resources
//! - `POST   /api/v1/stewarded-resources/{id}/allocations`                      → create allocation
//! - `POST   /api/v1/stewarded-resources/{id}/allocations/{allocId}/usage`      → record usage
//! - `GET    /api/v1/stewarded-resources/{stewardId}/category-summary/{cat}`    → category summary
//! - `GET    /api/v1/stewarded-resources/{stewardId}/dashboard`                 → full dashboard
//! - `GET    /api/v1/stewarded-resources/constitutional-limits/{category}`      → constitutional limits
//!
//! ## Architecture
//!
//! Business logic (ID generation, dimension derivation, aggregations, constitutional
//! limits) lives in these handlers. Storage CRUD is delegated to elohim-storage via
//! `reqwest`. Economic events are fired-and-forgotten.

use serde::{Deserialize, Serialize};

// =============================================================================
// Enums
// =============================================================================

/// Resource category — the different types of resources humans steward.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceCategory {
    #[serde(rename = "energy")]
    Energy,
    #[serde(rename = "compute")]
    Compute,
    #[serde(rename = "water")]
    Water,
    #[serde(rename = "food")]
    Food,
    #[serde(rename = "shelter")]
    Shelter,
    #[serde(rename = "transportation")]
    Transportation,
    #[serde(rename = "property")]
    Property,
    #[serde(rename = "equipment")]
    Equipment,
    #[serde(rename = "inventory")]
    Inventory,
    #[serde(rename = "knowledge")]
    Knowledge,
    #[serde(rename = "reputation")]
    Reputation,
    #[serde(rename = "financial-asset")]
    FinancialAsset,
    #[serde(rename = "uba")]
    Uba,
}

impl ResourceCategory {
    fn as_str(&self) -> &'static str {
        match self {
            ResourceCategory::Energy => "energy",
            ResourceCategory::Compute => "compute",
            ResourceCategory::Water => "water",
            ResourceCategory::Food => "food",
            ResourceCategory::Shelter => "shelter",
            ResourceCategory::Transportation => "transportation",
            ResourceCategory::Property => "property",
            ResourceCategory::Equipment => "equipment",
            ResourceCategory::Inventory => "inventory",
            ResourceCategory::Knowledge => "knowledge",
            ResourceCategory::Reputation => "reputation",
            ResourceCategory::FinancialAsset => "financial-asset",
            ResourceCategory::Uba => "uba",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "energy" => Some(ResourceCategory::Energy),
            "compute" => Some(ResourceCategory::Compute),
            "water" => Some(ResourceCategory::Water),
            "food" => Some(ResourceCategory::Food),
            "shelter" => Some(ResourceCategory::Shelter),
            "transportation" => Some(ResourceCategory::Transportation),
            "property" => Some(ResourceCategory::Property),
            "equipment" => Some(ResourceCategory::Equipment),
            "inventory" => Some(ResourceCategory::Inventory),
            "knowledge" => Some(ResourceCategory::Knowledge),
            "reputation" => Some(ResourceCategory::Reputation),
            "financial-asset" => Some(ResourceCategory::FinancialAsset),
            "uba" => Some(ResourceCategory::Uba),
            _ => None,
        }
    }
}

/// Governance level — graduated governance from individual to constitutional.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GovernanceLevel {
    #[serde(rename = "individual")]
    Individual,
    #[serde(rename = "household")]
    Household,
    #[serde(rename = "community")]
    Community,
    #[serde(rename = "network")]
    Network,
    #[serde(rename = "constitutional")]
    Constitutional,
}

/// Allocation strategy — how a resource's capacity is divided.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AllocationStrategy {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "automatic")]
    Automatic,
    #[serde(rename = "hybrid")]
    Hybrid,
}

/// Resource visibility level — governance visibility scope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceVisibilityLevel {
    #[serde(rename = "private")]
    Private,
    #[serde(rename = "household")]
    Household,
    #[serde(rename = "community")]
    Community,
    #[serde(rename = "public")]
    Public,
}

/// Data quality — how the resource data was obtained.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataQuality {
    #[serde(rename = "measured")]
    Measured,
    #[serde(rename = "estimated")]
    Estimated,
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "mixed")]
    Mixed,
}

/// Resource health status indicator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceHealthStatus {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "critical")]
    Critical,
}

/// Trend direction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrendDirection {
    #[serde(rename = "increasing")]
    Increasing,
    #[serde(rename = "stable")]
    Stable,
    #[serde(rename = "decreasing")]
    Decreasing,
}

/// Trend period.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrendPeriod {
    #[serde(rename = "day")]
    Day,
    #[serde(rename = "week")]
    Week,
    #[serde(rename = "month")]
    Month,
    #[serde(rename = "quarter")]
    Quarter,
    #[serde(rename = "year")]
    Year,
}

/// Alert severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "critical")]
    Critical,
}

/// Insight type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InsightType {
    #[serde(rename = "trend")]
    Trend,
    #[serde(rename = "pattern")]
    Pattern,
    #[serde(rename = "opportunity")]
    Opportunity,
    #[serde(rename = "governance")]
    Governance,
}

/// Constitutional limit enforcement method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EnforcementMethod {
    /// Honor system, trust-based.
    #[serde(rename = "voluntary")]
    Voluntary,
    /// Incentive-based (positive rewards for compliance, gradually increasing).
    #[serde(rename = "progressive")]
    Progressive,
    /// Mandatory enforcement.
    #[serde(rename = "hard")]
    Hard,
}

// =============================================================================
// Value Objects
// =============================================================================

/// A quantity of resource with its unit of measure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMeasure {
    pub value: f64,
    pub unit: String,
}

/// How a resource is measured.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDimension {
    pub unit: String,
    pub unit_label: String,
    pub unit_abbreviation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversion_factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard_unit: Option<String>,
}

/// Historical trend data for a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTrend {
    pub period: TrendPeriod,
    pub used: ResourceMeasure,
    pub allocated: ResourceMeasure,
    pub utilization: f64,
    pub trend: TrendDirection,
    pub change_percent: f64,
}

// =============================================================================
// Core Entities
// =============================================================================

/// An allocation of resource to a purpose.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationBlock {
    pub id: String,
    pub resource_id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub allocated: ResourceMeasure,
    pub used: ResourceMeasure,
    pub reserved: ResourceMeasure,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governed_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment_id: Option<String>,
    pub priority: i32,
    pub utilization: f64,
    pub created_at: String,
    pub updated_at: String,
}

/// A single use event (usually from EconomicEvent).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRecord {
    pub id: String,
    pub resource_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub economic_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocation_block_id: Option<String>,
    pub action: String,
    pub quantity: ResourceMeasure,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observer_attestation_id: Option<String>,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A resource tracked and managed by a human.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StewardedResource {
    // Identity
    pub id: String,
    pub resource_number: String,
    pub steward_id: String,
    pub category: ResourceCategory,
    pub subcategory: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // Capacity & Measurement
    pub dimension: ResourceDimension,
    pub total_capacity: ResourceMeasure,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permanent_reserve: Option<ResourceMeasure>,
    pub allocatable_capacity: ResourceMeasure,

    // Current State
    pub total_allocated: ResourceMeasure,
    pub total_reserved: ResourceMeasure,
    pub total_used: ResourceMeasure,
    pub available: ResourceMeasure,

    // Allocations
    pub allocations: Vec<AllocationBlock>,
    pub allocation_strategy: AllocationStrategy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocation_notes: Option<String>,

    // Governance
    pub governance_level: GovernanceLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governed_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constitutional_basis: Option<String>,
    pub can_modify_allocations: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_approval_for: Option<Vec<String>>,

    // Tracking & Verification
    pub observer_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observer_agent_id: Option<String>,
    pub recent_usage: Vec<UsageRecord>,
    pub trends: Vec<ResourceTrend>,

    // Economic Integration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_spec_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commons_pool_id: Option<String>,
    pub allocation_event_ids: Vec<String>,
    pub usage_event_ids: Vec<String>,

    // Metadata
    pub is_shared: bool,
    pub visibility: ResourceVisibilityLevel,
    pub data_quality: DataQuality,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Dashboard Aggregations
// =============================================================================

/// Aggregation of all resources in one category.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySummary {
    pub category: ResourceCategory,
    pub resources: Vec<StewardedResource>,
    pub total_capacity: ResourceMeasure,
    pub total_allocated: ResourceMeasure,
    pub total_used: ResourceMeasure,
    pub utilization_percent: f64,
    pub health_status: ResourceHealthStatus,
}

/// Warning about resource state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAlert {
    pub id: String,
    pub severity: AlertSeverity,
    pub resource: String,
    pub title: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_action: Option<String>,
    pub created_at: String,
}

/// Observation about resource patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInsight {
    pub id: String,
    #[serde(rename = "type")]
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    pub based_on_period: String,
    pub confidence: f64,
}

/// Key metrics for the dashboard overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMetrics {
    pub total_resources_tracked: u32,
    pub categories_covered: u32,
    pub overall_utilization: f64,
    pub fully_allocated_count: u32,
    pub health_status: ResourceHealthStatus,
}

/// Complete resource overview for a steward.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StewardedResourceDashboard {
    pub steward_id: String,
    pub steward_name: String,
    pub governance_level: String,

    // Overview by category
    pub categories: Vec<CategorySummary>,

    // Key metrics
    pub metrics: DashboardMetrics,

    // Alerts and insights
    pub alerts: Vec<ResourceAlert>,
    pub insights: Vec<ResourceInsight>,

    // Recent activity
    pub recent_allocations: Vec<AllocationBlock>,
    pub recent_usage: Vec<UsageRecord>,

    pub last_updated_at: String,
}

// =============================================================================
// Constitutional Limits
// =============================================================================

/// Bounds on stewardship within the Elohim constitutional framework.
///
/// Implements Donut Economy thinking:
/// - FLOOR: Dignity minimum (what everyone needs)
/// - CEILING: Constitutional maximum (beyond which is excess)
/// - SAFE OPERATING SPACE: The zone between floor and ceiling
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstitutionalLimit {
    pub id: String,
    pub resource_category: ResourceCategory,
    pub name: String,
    pub description: String,

    // Floor (Dignity Minimum)
    pub floor_value: f64,
    pub floor_unit: String,
    pub floor_rationale: String,
    pub floor_enforced: bool,

    // Ceiling (Constitutional Maximum)
    pub ceiling_value: f64,
    pub ceiling_unit: String,
    pub ceiling_rationale: String,
    pub ceiling_enforced: bool,

    // Safe Operating Space
    pub safe_min_value: f64,
    pub safe_max_value: f64,
    pub safe_zone_description: String,

    // Governance
    pub governance_level: String,
    pub constitutional_basis: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adoption_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_schedule: Option<String>,

    // Enforcement & Transition
    pub enforcement_method: EnforcementMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_deadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hard_stop_date: Option<String>,

    // Metadata
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Request Types
// =============================================================================

/// Options for resource creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResourceOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_level: Option<GovernanceLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observer_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permanent_reserve: Option<ResourceMeasure>,
}

/// POST /api/v1/stewarded-resources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResourceRequest {
    pub steward_id: String,
    pub category: ResourceCategory,
    pub subcategory: String,
    pub name: String,
    pub total_capacity: ResourceMeasure,
    #[serde(flatten)]
    pub options: Option<CreateResourceOptions>,
}

/// Options for allocation creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAllocationOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

/// POST /api/v1/stewarded-resources/{id}/allocations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAllocationRequest {
    pub label: String,
    pub allocated_amount: ResourceMeasure,
    #[serde(flatten)]
    pub options: Option<CreateAllocationOptions>,
}

/// Options for usage recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordUsageOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observer_attestation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// POST /api/v1/stewarded-resources/{id}/allocations/{allocId}/usage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordUsageRequest {
    pub allocation_block_id: Option<String>,
    pub amount: ResourceMeasure,
    #[serde(flatten)]
    pub options: Option<RecordUsageOptions>,
}

/// Minimal economic event response from storage (only field we need).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EconomicEventResponse {
    pub id: String,
}

// =============================================================================
// Business Logic Helpers
// =============================================================================

/// Generate a pseudo-random suffix using the current timestamp and a thread-local counter.
/// Equivalent to the TS pattern: `(crypto.getRandomValues(...) / 2**32).toString(36).substring(2, 11)`.
fn random_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // XOR timestamp nanos with process id for entropy without pulling in rand
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let pid = std::process::id();
    let mixed = nanos ^ (pid << 7) ^ (pid >> 3);
    format!("{mixed:09x}")
}

fn make_id(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{prefix}-{ts}-{}", random_suffix())
}

/// Derive the standard `ResourceDimension` for a category (mirrors `getDimensionForCategory`).
fn dimension_for_category(category: &ResourceCategory) -> ResourceDimension {
    match category {
        ResourceCategory::Energy => ResourceDimension {
            unit: "hours".into(),
            unit_label: "hours".into(),
            unit_abbreviation: "h".into(),
            conversion_factor: None,
            standard_unit: Some("hours".into()),
        },
        ResourceCategory::Compute => ResourceDimension {
            unit: "percent".into(),
            unit_label: "percent".into(),
            unit_abbreviation: "%".into(),
            conversion_factor: None,
            standard_unit: Some("percent".into()),
        },
        ResourceCategory::Water => ResourceDimension {
            unit: "liters".into(),
            unit_label: "liters".into(),
            unit_abbreviation: "L".into(),
            conversion_factor: None,
            standard_unit: Some("liters".into()),
        },
        ResourceCategory::Food => ResourceDimension {
            unit: "calories".into(),
            unit_label: "calories".into(),
            unit_abbreviation: "kcal".into(),
            conversion_factor: None,
            standard_unit: Some("calories".into()),
        },
        ResourceCategory::Shelter => ResourceDimension {
            unit: "square-meters".into(),
            unit_label: "square meters".into(),
            unit_abbreviation: "m²".into(),
            conversion_factor: None,
            standard_unit: Some("square-meters".into()),
        },
        ResourceCategory::Transportation => ResourceDimension {
            unit: "kilometers".into(),
            unit_label: "kilometers".into(),
            unit_abbreviation: "km".into(),
            conversion_factor: None,
            standard_unit: Some("kilometers".into()),
        },
        ResourceCategory::Property | ResourceCategory::Equipment => ResourceDimension {
            unit: "count".into(),
            unit_label: "items".into(),
            unit_abbreviation: "#".into(),
            conversion_factor: None,
            standard_unit: Some("count".into()),
        },
        ResourceCategory::Inventory => ResourceDimension {
            unit: "units".into(),
            unit_label: "units".into(),
            unit_abbreviation: "u".into(),
            conversion_factor: None,
            standard_unit: Some("units".into()),
        },
        ResourceCategory::Knowledge => ResourceDimension {
            unit: "concepts".into(),
            unit_label: "concepts".into(),
            unit_abbreviation: "c".into(),
            conversion_factor: None,
            standard_unit: Some("concepts".into()),
        },
        ResourceCategory::Reputation => ResourceDimension {
            unit: "score".into(),
            unit_label: "reputation score".into(),
            unit_abbreviation: "pts".into(),
            conversion_factor: None,
            standard_unit: Some("score".into()),
        },
        ResourceCategory::FinancialAsset | ResourceCategory::Uba => ResourceDimension {
            unit: "currency".into(),
            unit_label: "currency units".into(),
            unit_abbreviation: "$".into(),
            conversion_factor: None,
            standard_unit: Some("currency".into()),
        },
    }
}

/// Compute utilization percent, clamped to [0, 100].
fn utilization_percent(used: f64, capacity: f64) -> f64 {
    if capacity <= 0.0 {
        return 0.0;
    }
    (used / capacity * 100.0).min(100.0)
}

/// Determine health status from utilization percentage.
fn health_status(utilization: f64) -> ResourceHealthStatus {
    if utilization > 90.0 {
        ResourceHealthStatus::Critical
    } else if utilization > 75.0 {
        ResourceHealthStatus::Warning
    } else {
        ResourceHealthStatus::Healthy
    }
}

/// Calculate weighted average utilization across categories.
fn weighted_utilization(categories: &[CategorySummary]) -> f64 {
    let total_capacity: f64 = categories.iter().map(|c| c.total_capacity.value).sum();
    if total_capacity <= 0.0 {
        return 0.0;
    }
    let weighted_used: f64 = categories.iter().map(|c| c.total_used.value).sum();
    (weighted_used / total_capacity * 100.0).min(100.0)
}

/// Return the hardcoded constitutional limit for a category, or None.
fn constitutional_limit_for(category: &ResourceCategory) -> Option<ConstitutionalLimit> {
    let now = chrono::Utc::now().to_rfc3339();

    match category {
        ResourceCategory::FinancialAsset => Some(ConstitutionalLimit {
            id: "limit-wealth-ceiling".into(),
            resource_category: ResourceCategory::FinancialAsset,
            name: "Wealth Ceiling (Limitarianism)".into(),
            description: "Constitutional maximum for net worth holding".into(),
            floor_value: 75_000.0,
            floor_unit: "USD".into(),
            floor_rationale: "Enables basic needs: food, shelter, healthcare, education, dignity"
                .into(),
            floor_enforced: true,
            ceiling_value: 10_000_000.0,
            ceiling_unit: "USD".into(),
            ceiling_rationale:
                "Beyond this, accumulation enables extraction. Supports community stewardship."
                    .into(),
            ceiling_enforced: false,
            safe_min_value: 75_000.0,
            safe_max_value: 10_000_000.0,
            safe_zone_description:
                "Flourishing stewardship - adequate for personal/family + community contribution"
                    .into(),
            governance_level: "Elohim-network".into(),
            constitutional_basis: "Part III: Constitutional Economics".into(),
            adoption_date: None,
            review_schedule: Some("annual".into()),
            enforcement_method: EnforcementMethod::Progressive,
            transition_deadline: Some("2035-12-31".into()),
            hard_stop_date: None,
            created_at: now.clone(),
            updated_at: now,
        }),
        ResourceCategory::Energy => Some(ConstitutionalLimit {
            id: "limit-time-ceiling".into(),
            resource_category: ResourceCategory::Energy,
            name: "Time Allocation Ceiling".into(),
            description: "Constitutional maximum for work/extraction of another's time".into(),
            floor_value: 40.0,
            floor_unit: "hours/week".into(),
            floor_rationale: "Enables human flourishing - rest, relationships, learning".into(),
            floor_enforced: true,
            ceiling_value: 100.0,
            ceiling_unit: "hours/week".into(),
            ceiling_rationale: "Beyond this is extraction. Community work respects boundaries."
                .into(),
            ceiling_enforced: false,
            safe_min_value: 40.0,
            safe_max_value: 60.0,
            safe_zone_description: "Balanced life - work, rest, relationships, contribution".into(),
            governance_level: "individual/household".into(),
            constitutional_basis: "Part II: Stewardship & Human Flourishing".into(),
            adoption_date: None,
            review_schedule: None,
            enforcement_method: EnforcementMethod::Voluntary,
            transition_deadline: None,
            hard_stop_date: None,
            created_at: now.clone(),
            updated_at: now,
        }),
        ResourceCategory::Compute => Some(ConstitutionalLimit {
            id: "limit-node-capacity".into(),
            resource_category: ResourceCategory::Compute,
            name: "Node Capacity Ceiling".into(),
            description: "Constitutional sharing of node resources".into(),
            floor_value: 10.0,
            floor_unit: "percent".into(),
            floor_rationale: "Minimum capacity for self-sovereign apps".into(),
            floor_enforced: true,
            ceiling_value: 80.0,
            ceiling_unit: "percent".into(),
            ceiling_rationale:
                "Beyond this, excess capacity returns to commons for community benefit".into(),
            ceiling_enforced: false,
            safe_min_value: 10.0,
            safe_max_value: 80.0,
            safe_zone_description: "Personal autonomy + community contribution balance".into(),
            governance_level: "network".into(),
            constitutional_basis: "Part IV: Autonomous Infrastructure".into(),
            adoption_date: None,
            review_schedule: None,
            enforcement_method: EnforcementMethod::Progressive,
            transition_deadline: None,
            hard_stop_date: None,
            created_at: now.clone(),
            updated_at: now,
        }),
        _ => None,
    }
}

/// Generate dashboard alerts from resource state.
fn generate_alerts(resources: &[StewardedResource], now: &str) -> Vec<ResourceAlert> {
    let mut alerts = Vec::new();
    for resource in resources {
        let utilization =
            utilization_percent(resource.total_used.value, resource.total_capacity.value);
        if utilization > 90.0 {
            alerts.push(ResourceAlert {
                id: make_id("alert"),
                severity: AlertSeverity::Critical,
                resource: resource.id.clone(),
                title: format!("Critical utilization: {}", resource.name),
                message: format!(
                    "{} is at {:.0}% utilization. Immediate action recommended.",
                    resource.name, utilization
                ),
                recommended_action: Some("Reduce usage or increase capacity.".into()),
                created_at: now.to_string(),
            });
        } else if utilization > 75.0 {
            alerts.push(ResourceAlert {
                id: make_id("alert"),
                severity: AlertSeverity::Warning,
                resource: resource.id.clone(),
                title: format!("High utilization: {}", resource.name),
                message: format!("{} is at {:.0}% utilization.", resource.name, utilization),
                recommended_action: Some("Monitor usage closely.".into()),
                created_at: now.to_string(),
            });
        }

        // Alert on unallocated capacity > 50%
        let unallocated = resource.available.value;
        let capacity = resource.total_capacity.value;
        if capacity > 0.0 && unallocated / capacity > 0.5 {
            alerts.push(ResourceAlert {
                id: make_id("alert"),
                severity: AlertSeverity::Info,
                resource: resource.id.clone(),
                title: format!("Unallocated capacity: {}", resource.name),
                message: format!(
                    "{:.0}% of {} is unallocated. Consider allocating for better stewardship.",
                    (unallocated / capacity * 100.0),
                    resource.name
                ),
                recommended_action: Some("Create allocation blocks for this resource.".into()),
                created_at: now.to_string(),
            });
        }
    }
    alerts
}

// =============================================================================
// Route Handlers
// =============================================================================

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::server::AppState;

fn json_response<T: Serialize>(status: StatusCode, body: &T) -> Response<Full<Bytes>> {
    match serde_json::to_vec(body) {
        Ok(bytes) => Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .header("Cross-Origin-Resource-Policy", "cross-origin")
            .body(Full::new(Bytes::from(bytes)))
            .unwrap(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Serialization error: {e}"),
        ),
    }
}

fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({ "error": message });
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(
            serde_json::to_vec(&body).unwrap_or_default(),
        )))
        .unwrap()
}

/// Forward a request to elohim-storage, remapping the path prefix.
///
/// `/api/v1/resources/...` → `{storage_url}/db/resources/...`
async fn forward_to_storage(
    req: Request<Incoming>,
    storage_url: &str,
    storage_path: &str,
) -> Response<Full<Bytes>> {
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), storage_path);

    let query = req.uri().query();
    let full_url = if let Some(q) = query {
        format!("{storage_endpoint}?{q}")
    } else {
        storage_endpoint
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding stewarded-resources request to elohim-storage");

    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::DELETE => client.delete(&full_url),
        Method::HEAD => client.head(&full_url),
        _ => {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap();
        }
    };

    // Forward content-type header if present
    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }

    // Forward authorization header if present
    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    // Forward body for POST/PUT
    if matches!(method, Method::POST | Method::PUT) {
        match req.collect().await {
            Ok(collected) => {
                let body_bytes = collected.to_bytes();
                builder = builder.body(body_bytes.to_vec());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read stewarded-resources request body");
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to read request body: {e}"}}"#
                    ))))
                    .unwrap();
            }
        }
    }

    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            match response.bytes().await {
                Ok(body) => {
                    info!(
                        status = %status,
                        size = body.len(),
                        path = %storage_path,
                        "Forwarded stewarded-resources response"
                    );
                    Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", content_type)
                        .header("Cross-Origin-Resource-Policy", "cross-origin")
                        .body(Full::new(Bytes::from(body.to_vec())))
                        .unwrap()
                }
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, url = %full_url, "Failed to forward stewarded-resources request to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

/// Parameters for creating an economic event via storage API.
struct EconomicEventParams<'a> {
    storage_url: &'a str,
    action: &'a str,
    provider_id: &'a str,
    receiver_id: &'a str,
    resource_conforms_to: Option<&'a str>,
    quantity_value: f64,
    quantity_unit: &'a str,
    note: &'a str,
}

/// Fire-and-forget: POST an economic event to storage. Errors are logged, not propagated.
async fn fire_economic_event(params: &EconomicEventParams<'_>) -> Option<String> {
    let url = format!(
        "{}/db/economic-events",
        params.storage_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "action": params.action,
        "providerId": params.provider_id,
        "receiverId": params.receiver_id,
        "resourceConformsTo": params.resource_conforms_to,
        "resourceQuantityValue": params.quantity_value,
        "resourceQuantityUnit": params.quantity_unit,
        "note": params.note,
    });

    let client = reqwest::Client::new();
    match client.post(&url).json(&body).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                resp.json::<EconomicEventResponse>()
                    .await
                    .ok()
                    .map(|e| e.id)
            } else {
                warn!(status = %resp.status(), "Economic event creation returned non-success");
                None
            }
        }
        Err(e) => {
            warn!(error = %e, "Failed to create economic event (fire-and-forget)");
            None
        }
    }
}

/// POST /api/v1/resources — create a new stewarded resource.
async fn handle_create_resource(
    req: Request<Incoming>,
    storage_url: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };

    let input: CreateResourceRequest = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid request body: {e}"),
            )
        }
    };

    let now = chrono::Utc::now().to_rfc3339();
    let ts = chrono::Utc::now().timestamp_millis();

    let dimension = dimension_for_category(&input.category);
    let options = input.options.as_ref();

    let permanent_reserve = options.and_then(|o| o.permanent_reserve.clone());
    let allocatable_value = match &permanent_reserve {
        Some(pr) if pr.value > 0.0 => input.total_capacity.value - pr.value,
        _ => input.total_capacity.value,
    };

    let governance_level = options
        .and_then(|o| o.governance_level.clone())
        .unwrap_or(GovernanceLevel::Individual);

    let resource = StewardedResource {
        id: make_id("res"),
        resource_number: format!("RES-{ts}"),
        steward_id: input.steward_id.clone(),
        category: input.category.clone(),
        subcategory: input.subcategory.clone(),
        name: input.name.clone(),
        description: options.and_then(|o| o.description.clone()),
        dimension,
        total_capacity: input.total_capacity.clone(),
        permanent_reserve: permanent_reserve.clone(),
        allocatable_capacity: ResourceMeasure {
            value: allocatable_value,
            unit: input.total_capacity.unit.clone(),
        },
        total_allocated: ResourceMeasure {
            value: 0.0,
            unit: input.total_capacity.unit.clone(),
        },
        total_reserved: ResourceMeasure {
            value: 0.0,
            unit: input.total_capacity.unit.clone(),
        },
        total_used: ResourceMeasure {
            value: 0.0,
            unit: input.total_capacity.unit.clone(),
        },
        available: ResourceMeasure {
            value: input.total_capacity.value,
            unit: input.total_capacity.unit.clone(),
        },
        allocations: vec![],
        allocation_strategy: AllocationStrategy::Manual,
        allocation_notes: None,
        governance_level,
        governed_by: None,
        constitutional_basis: None,
        can_modify_allocations: true,
        requires_approval_for: None,
        observer_enabled: options.and_then(|o| o.observer_enabled).unwrap_or(false),
        observer_agent_id: None,
        recent_usage: vec![],
        trends: vec![],
        resource_spec_id: None,
        commons_pool_id: None,
        allocation_event_ids: vec![],
        usage_event_ids: vec![],
        is_shared: false,
        visibility: ResourceVisibilityLevel::Private,
        data_quality: DataQuality::Manual,
        last_verified_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    // Persist to storage
    let client = reqwest::Client::new();
    let storage_resources_url = format!("{}/db/resources", storage_url.trim_end_matches('/'));
    match client
        .post(&storage_resources_url)
        .json(&resource)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            // Fire-and-forget economic event
            let category_str = input.category.as_str();
            let conforms_to = format!("{}/{}", category_str, input.subcategory);
            tokio::spawn({
                let storage_url = storage_url.to_string();
                let steward_id = input.steward_id.clone();
                let name = input.name.clone();
                let cap_value = input.total_capacity.value;
                let cap_unit = input.total_capacity.unit.clone();
                async move {
                    fire_economic_event(&EconomicEventParams {
                        storage_url: &storage_url,
                        action: "produce",
                        provider_id: &steward_id,
                        receiver_id: &steward_id,
                        resource_conforms_to: Some(&conforms_to),
                        quantity_value: cap_value,
                        quantity_unit: &cap_unit,
                        note: &format!("Created stewarded resource: {name}"),
                    })
                    .await;
                }
            });

            // Return the created resource (from our struct, not re-parsing storage response)
            json_response(StatusCode::CREATED, &resource)
        }
        Ok(resp) => {
            let status = resp.status();
            let msg = resp.text().await.unwrap_or_default();
            warn!(status = %status, "Storage rejected resource creation");
            error_response(
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                &format!("Storage error: {msg}"),
            )
        }
        Err(e) => {
            warn!(error = %e, "Failed to persist resource to storage");
            error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Storage unavailable: {e}"),
            )
        }
    }
}

/// POST /api/v1/resources/{id}/allocations — create an allocation for a resource.
async fn handle_create_allocation(
    req: Request<Incoming>,
    storage_url: &str,
    resource_id: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };

    let input: CreateAllocationRequest = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid request body: {e}"),
            )
        }
    };

    let now = chrono::Utc::now().to_rfc3339();
    let options = input.options.as_ref();

    let allocation = AllocationBlock {
        id: make_id("alloc"),
        resource_id: resource_id.to_string(),
        label: input.label.clone(),
        description: options.and_then(|o| o.description.clone()),
        allocated: input.allocated_amount.clone(),
        used: ResourceMeasure {
            value: 0.0,
            unit: input.allocated_amount.unit.clone(),
        },
        reserved: ResourceMeasure {
            value: 0.0,
            unit: input.allocated_amount.unit.clone(),
        },
        governance_level: options.and_then(|o| o.governance_level.clone()),
        governed_by: None,
        commitment_id: None,
        priority: options.and_then(|o| o.priority).unwrap_or(5),
        utilization: 0.0,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    // Persist to storage
    let client = reqwest::Client::new();
    let storage_url_alloc = format!(
        "{}/db/resources/{}/allocations",
        storage_url.trim_end_matches('/'),
        resource_id
    );
    match client
        .post(&storage_url_alloc)
        .json(&allocation)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            // Fire-and-forget economic event
            tokio::spawn({
                let storage_url = storage_url.to_string();
                let resource_id = resource_id.to_string();
                let label = input.label.clone();
                let qty_value = input.allocated_amount.value;
                let qty_unit = input.allocated_amount.unit.clone();
                async move {
                    fire_economic_event(&EconomicEventParams {
                        storage_url: &storage_url,
                        action: "transfer",
                        provider_id: &resource_id,
                        receiver_id: &resource_id,
                        resource_conforms_to: None,
                        quantity_value: qty_value,
                        quantity_unit: &qty_unit,
                        note: &format!("Allocated {qty_value} {qty_unit} to: {label}"),
                    })
                    .await;
                }
            });

            json_response(StatusCode::CREATED, &allocation)
        }
        Ok(resp) => {
            let status = resp.status();
            let msg = resp.text().await.unwrap_or_default();
            error_response(
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                &format!("Storage error: {msg}"),
            )
        }
        Err(e) => error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Storage unavailable: {e}"),
        ),
    }
}

/// POST /api/v1/resources/{id}/usage — record usage for a resource.
async fn handle_record_usage(
    req: Request<Incoming>,
    storage_url: &str,
    resource_id: &str,
) -> Response<Full<Bytes>> {
    let body_bytes = match req.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Failed to read body: {e}"),
            )
        }
    };

    let input: RecordUsageRequest = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid request body: {e}"),
            )
        }
    };

    let now = chrono::Utc::now().to_rfc3339();
    let options = input.options.as_ref();
    let action = options
        .and_then(|o| o.action.clone())
        .unwrap_or_else(|| "use".to_string());

    let mut usage = UsageRecord {
        id: make_id("use"),
        resource_id: resource_id.to_string(),
        economic_event_id: None,
        allocation_block_id: input.allocation_block_id.clone(),
        action: action.clone(),
        quantity: input.amount.clone(),
        duration: None,
        observer_attestation_id: options.and_then(|o| o.observer_attestation_id.clone()),
        timestamp: now.clone(),
        note: options.and_then(|o| o.note.clone()),
    };

    // Create economic event first to get its ID (not fire-and-forget — we need the event ID)
    let event_id = fire_economic_event(&EconomicEventParams {
        storage_url,
        action: &action,
        provider_id: resource_id,
        receiver_id: resource_id,
        resource_conforms_to: None,
        quantity_value: input.amount.value,
        quantity_unit: &input.amount.unit,
        note: options
            .and_then(|o| o.note.as_deref())
            .unwrap_or("Resource usage recorded"),
    })
    .await;

    usage.economic_event_id = event_id;

    // Persist usage record to storage
    let client = reqwest::Client::new();
    let storage_url_usage = format!(
        "{}/db/resources/{}/usage",
        storage_url.trim_end_matches('/'),
        resource_id
    );
    match client.post(&storage_url_usage).json(&usage).send().await {
        Ok(resp) if resp.status().is_success() => json_response(StatusCode::CREATED, &usage),
        Ok(resp) => {
            let status = resp.status();
            let msg = resp.text().await.unwrap_or_default();
            error_response(
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                &format!("Storage error: {msg}"),
            )
        }
        Err(e) => error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Storage unavailable: {e}"),
        ),
    }
}

/// GET /api/v1/resources/dashboard/{stewardId} — build aggregated dashboard.
async fn handle_get_dashboard(storage_url: &str, steward_id: &str) -> Response<Full<Bytes>> {
    let now = chrono::Utc::now().to_rfc3339();

    // Fetch all resources for this steward
    let client = reqwest::Client::new();
    let resources_url = format!(
        "{}/db/resources?stewardId={}",
        storage_url.trim_end_matches('/'),
        steward_id
    );

    let all_resources: Vec<StewardedResource> = match client.get(&resources_url).send().await {
        Ok(resp) if resp.status().is_success() => resp.json().await.unwrap_or_default(),
        Ok(resp) => {
            warn!(status = %resp.status(), steward_id, "Storage returned non-success for resource list");
            vec![]
        }
        Err(e) => {
            warn!(error = %e, "Failed to fetch resources from storage for dashboard");
            vec![]
        }
    };

    // Group by category and compute summaries
    let mut category_map: std::collections::HashMap<String, Vec<StewardedResource>> =
        std::collections::HashMap::new();
    for resource in &all_resources {
        category_map
            .entry(resource.category.as_str().to_string())
            .or_default()
            .push(resource.clone());
    }

    let mut categories: Vec<CategorySummary> = category_map
        .into_values()
        .map(|resources| {
            let total_capacity: f64 = resources.iter().map(|r| r.total_capacity.value).sum();
            let total_allocated: f64 = resources.iter().map(|r| r.total_allocated.value).sum();
            let total_used: f64 = resources.iter().map(|r| r.total_used.value).sum();
            let utilization = utilization_percent(total_used, total_capacity);
            let status = health_status(utilization);
            let unit = resources
                .first()
                .map(|r| r.total_capacity.unit.clone())
                .unwrap_or_default();

            // Clone resources into Vec<StewardedResource> for the summary
            let category = resources
                .first()
                .map(|r| r.category.clone())
                .unwrap_or(ResourceCategory::Energy);
            CategorySummary {
                category,
                resources,
                total_capacity: ResourceMeasure {
                    value: total_capacity,
                    unit: unit.clone(),
                },
                total_allocated: ResourceMeasure {
                    value: total_allocated,
                    unit: unit.clone(),
                },
                total_used: ResourceMeasure {
                    value: total_used,
                    unit: unit.clone(),
                },
                utilization_percent: utilization,
                health_status: status,
            }
        })
        .collect();

    // Sort categories by name for stable ordering
    categories.sort_by(|a, b| a.category.as_str().cmp(b.category.as_str()));

    // Metrics
    let total_resources_tracked = all_resources.len() as u32;
    let categories_covered = categories.len() as u32;
    let overall_utilization = weighted_utilization(&categories);
    let fully_allocated_count = all_resources
        .iter()
        .filter(|r| r.available.value <= 0.0)
        .count() as u32;
    let overall_health = health_status(overall_utilization);

    // Alerts
    let alerts = generate_alerts(&all_resources, &now);

    // Recent allocations (last 5)
    let mut recent_allocations: Vec<AllocationBlock> = all_resources
        .iter()
        .flat_map(|r| r.allocations.clone())
        .collect();
    recent_allocations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    recent_allocations.truncate(5);

    // Recent usage (last 10)
    let mut recent_usage: Vec<UsageRecord> = all_resources
        .iter()
        .flat_map(|r| r.recent_usage.clone())
        .collect();
    recent_usage.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    recent_usage.truncate(10);

    let dashboard = StewardedResourceDashboard {
        steward_id: steward_id.to_string(),
        steward_name: String::new(), // Would require a separate human service lookup
        governance_level: "individual".to_string(),
        categories,
        metrics: DashboardMetrics {
            total_resources_tracked,
            categories_covered,
            overall_utilization,
            fully_allocated_count,
            health_status: overall_health,
        },
        alerts,
        insights: vec![],
        recent_allocations,
        recent_usage,
        last_updated_at: now,
    };

    json_response(StatusCode::OK, &dashboard)
}

/// GET /api/v1/resources/constitutional-limits/{category} — return hardcoded limits.
fn handle_get_constitutional_limits(category_str: &str) -> Response<Full<Bytes>> {
    match ResourceCategory::from_str(category_str) {
        Some(cat) => match constitutional_limit_for(&cat) {
            Some(limit) => json_response(StatusCode::OK, &limit),
            None => error_response(
                StatusCode::NOT_FOUND,
                &format!("No constitutional limit defined for category: {category_str}"),
            ),
        },
        None => error_response(
            StatusCode::NOT_FOUND,
            &format!("Unknown resource category: {category_str}"),
        ),
    }
}

// =============================================================================
// Route Dispatcher
// =============================================================================

/// Handle all `/api/v1/resources/*` requests.
///
/// Routes with business logic are handled directly; others proxy to `/db/resources/*`
/// on elohim-storage.
pub async fn handle_stewarded_resources_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url.clone(),
        None => {
            warn!("Stewarded-resources proxy called but STORAGE_URL not configured");
            return Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Storage service not configured. Set STORAGE_URL env var."}"#,
                )))
                .unwrap();
        }
    };

    // Strip the /api/v1/resources prefix to get the sub-path
    let sub = path
        .strip_prefix("/api/v1/resources")
        .unwrap_or("")
        .trim_start_matches('/');

    let method = req.method().clone();

    // ─── Route matching ───────────────────────────────────────────────────────

    // POST /api/v1/resources  (create resource)
    if method == Method::POST && sub.is_empty() {
        return handle_create_resource(req, &storage_url).await;
    }

    // GET /api/v1/resources/constitutional-limits/{category}
    if method == Method::GET {
        if let Some(cat) = sub.strip_prefix("constitutional-limits/") {
            let cat = cat.trim_end_matches('/');
            return handle_get_constitutional_limits(cat);
        }
    }

    // Parse /{id}/... sub-paths
    // Split sub into segments: ["resource-id", "allocations", ...]
    let segments: Vec<&str> = sub.split('/').filter(|s| !s.is_empty()).collect();

    match (method.clone(), segments.as_slice()) {
        // GET /api/v1/resources/dashboard/{stewardId}
        // Note: "dashboard" appears as the *second* segment after stewardId.
        // Pattern: /{stewardId}/dashboard
        (Method::GET, [steward_id, "dashboard"]) => {
            handle_get_dashboard(&storage_url, steward_id).await
        }

        // POST /api/v1/resources/{id}/allocations
        (Method::POST, [resource_id, "allocations"]) => {
            handle_create_allocation(req, &storage_url, resource_id).await
        }

        // POST /api/v1/resources/{id}/usage
        (Method::POST, [resource_id, "usage"]) => {
            handle_record_usage(req, &storage_url, resource_id).await
        }

        // All other routes: proxy to /db/resources/...
        _ => {
            let storage_path = if sub.is_empty() {
                "/db/resources".to_string()
            } else {
                format!("/db/resources/{sub}")
            };
            forward_to_storage(req, &storage_url, &storage_path).await
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_category_serde_roundtrip() {
        let cat = ResourceCategory::FinancialAsset;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, r#""financial-asset""#);

        let parsed: ResourceCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ResourceCategory::FinancialAsset);
    }

    #[test]
    fn test_resource_measure_camel_case() {
        let measure = ResourceMeasure {
            value: 100.0,
            unit: "kWh".to_string(),
        };
        let json = serde_json::to_string(&measure).unwrap();
        assert!(json.contains("\"value\""));
        assert!(json.contains("\"unit\""));
    }

    #[test]
    fn test_create_resource_request_deserialize() {
        let json = r#"{
            "stewardId": "human-001",
            "category": "energy",
            "subcategory": "electricity",
            "name": "Home Solar",
            "totalCapacity": { "value": 500.0, "unit": "kWh" }
        }"#;
        let req: CreateResourceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.steward_id, "human-001");
        assert_eq!(req.category, ResourceCategory::Energy);
        assert_eq!(req.total_capacity.value, 500.0);
    }

    #[test]
    fn test_create_allocation_request_deserialize() {
        let json = r#"{
            "label": "Learning",
            "allocatedAmount": { "value": 10.0, "unit": "hours" },
            "priority": 5
        }"#;
        let req: CreateAllocationRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.label, "Learning");
        assert_eq!(req.allocated_amount.value, 10.0);
    }

    #[test]
    fn test_record_usage_request_deserialize() {
        let json = r#"{
            "amount": { "value": 2.5, "unit": "hours" },
            "action": "use",
            "note": "Study session"
        }"#;
        let req: RecordUsageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount.value, 2.5);
    }

    #[test]
    fn test_stewarded_resource_serializes_camel_case() {
        let resource = StewardedResource {
            id: "res-1".into(),
            resource_number: "RES-0001".into(),
            steward_id: "human-001".into(),
            category: ResourceCategory::Compute,
            subcategory: "processing".into(),
            name: "Node CPU".into(),
            description: None,
            dimension: ResourceDimension {
                unit: "percent".into(),
                unit_label: "percent".into(),
                unit_abbreviation: "%".into(),
                conversion_factor: None,
                standard_unit: Some("percent".into()),
            },
            total_capacity: ResourceMeasure {
                value: 100.0,
                unit: "percent".into(),
            },
            permanent_reserve: None,
            allocatable_capacity: ResourceMeasure {
                value: 100.0,
                unit: "percent".into(),
            },
            total_allocated: ResourceMeasure {
                value: 60.0,
                unit: "percent".into(),
            },
            total_reserved: ResourceMeasure {
                value: 10.0,
                unit: "percent".into(),
            },
            total_used: ResourceMeasure {
                value: 45.0,
                unit: "percent".into(),
            },
            available: ResourceMeasure {
                value: 40.0,
                unit: "percent".into(),
            },
            allocations: vec![],
            allocation_strategy: AllocationStrategy::Manual,
            allocation_notes: None,
            governance_level: GovernanceLevel::Individual,
            governed_by: None,
            constitutional_basis: None,
            can_modify_allocations: true,
            requires_approval_for: None,
            observer_enabled: false,
            observer_agent_id: None,
            recent_usage: vec![],
            trends: vec![],
            resource_spec_id: None,
            commons_pool_id: None,
            allocation_event_ids: vec![],
            usage_event_ids: vec![],
            is_shared: false,
            visibility: ResourceVisibilityLevel::Private,
            data_quality: DataQuality::Manual,
            last_verified_at: None,
            created_at: "2026-03-05T00:00:00Z".into(),
            updated_at: "2026-03-05T00:00:00Z".into(),
        };

        let json = serde_json::to_string(&resource).unwrap();
        // Verify camelCase field names in output
        assert!(json.contains("\"resourceNumber\""));
        assert!(json.contains("\"stewardId\""));
        assert!(json.contains("\"totalCapacity\""));
        assert!(json.contains("\"allocatableCapacity\""));
        assert!(json.contains("\"allocationStrategy\""));
        assert!(json.contains("\"governanceLevel\""));
        assert!(json.contains("\"canModifyAllocations\""));
        assert!(json.contains("\"observerEnabled\""));
        assert!(json.contains("\"isShared\""));
        assert!(json.contains("\"dataQuality\""));
        assert!(json.contains("\"createdAt\""));
    }

    #[test]
    fn test_constitutional_limit_serde() {
        let limit = ConstitutionalLimit {
            id: "cl-1".into(),
            resource_category: ResourceCategory::FinancialAsset,
            name: "Wealth Ceiling".into(),
            description: "Maximum individual financial stewardship".into(),
            floor_value: 2000.0,
            floor_unit: "USD/month".into(),
            floor_rationale: "Dignity minimum for basic needs".into(),
            floor_enforced: true,
            ceiling_value: 2_000_000.0,
            ceiling_unit: "USD".into(),
            ceiling_rationale: "Limitarianism: accumulation beyond this enables extraction".into(),
            ceiling_enforced: false,
            safe_min_value: 5000.0,
            safe_max_value: 1_500_000.0,
            safe_zone_description: "Comfortable stewardship range".into(),
            governance_level: "network".into(),
            constitutional_basis: "Elohim Constitution Art. 7".into(),
            adoption_date: Some("2026-01-01".into()),
            review_schedule: Some("annual".into()),
            enforcement_method: EnforcementMethod::Progressive,
            transition_deadline: None,
            hard_stop_date: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-03-05T00:00:00Z".into(),
        };

        let json = serde_json::to_string(&limit).unwrap();
        assert!(json.contains("\"resourceCategory\""));
        assert!(json.contains("\"floorValue\""));
        assert!(json.contains("\"ceilingValue\""));
        assert!(json.contains("\"enforcementMethod\""));
        assert!(json.contains("\"safeZoneDescription\""));

        // Roundtrip
        let parsed: ConstitutionalLimit = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "cl-1");
        assert_eq!(parsed.resource_category, ResourceCategory::FinancialAsset);
    }

    #[test]
    fn test_dimension_for_energy() {
        let dim = dimension_for_category(&ResourceCategory::Energy);
        assert_eq!(dim.unit, "hours");
        assert_eq!(dim.unit_abbreviation, "h");
    }

    #[test]
    fn test_dimension_for_compute() {
        let dim = dimension_for_category(&ResourceCategory::Compute);
        assert_eq!(dim.unit, "percent");
        assert_eq!(dim.unit_abbreviation, "%");
    }

    #[test]
    fn test_dimension_for_financial_asset() {
        let dim = dimension_for_category(&ResourceCategory::FinancialAsset);
        assert_eq!(dim.unit, "currency");
        assert_eq!(dim.unit_abbreviation, "$");
    }

    #[test]
    fn test_utilization_percent_zero_capacity() {
        assert_eq!(utilization_percent(10.0, 0.0), 0.0);
    }

    #[test]
    fn test_utilization_percent_clamped() {
        assert_eq!(utilization_percent(200.0, 100.0), 100.0);
    }

    #[test]
    fn test_health_status_thresholds() {
        assert_eq!(health_status(50.0), ResourceHealthStatus::Healthy);
        assert_eq!(health_status(80.0), ResourceHealthStatus::Warning);
        assert_eq!(health_status(95.0), ResourceHealthStatus::Critical);
    }

    #[test]
    fn test_constitutional_limit_financial_asset() {
        let limit = constitutional_limit_for(&ResourceCategory::FinancialAsset).unwrap();
        assert_eq!(limit.floor_value, 75_000.0);
        assert_eq!(limit.ceiling_value, 10_000_000.0);
        assert_eq!(limit.enforcement_method, EnforcementMethod::Progressive);
    }

    #[test]
    fn test_constitutional_limit_energy() {
        let limit = constitutional_limit_for(&ResourceCategory::Energy).unwrap();
        assert_eq!(limit.floor_value, 40.0);
        assert_eq!(limit.ceiling_value, 100.0);
        assert_eq!(limit.enforcement_method, EnforcementMethod::Voluntary);
    }

    #[test]
    fn test_constitutional_limit_compute() {
        let limit = constitutional_limit_for(&ResourceCategory::Compute).unwrap();
        assert_eq!(limit.floor_value, 10.0);
        assert_eq!(limit.ceiling_value, 80.0);
    }

    #[test]
    fn test_constitutional_limit_unknown_category() {
        assert!(constitutional_limit_for(&ResourceCategory::Water).is_none());
        assert!(constitutional_limit_for(&ResourceCategory::Food).is_none());
    }

    #[test]
    fn test_category_from_str() {
        assert_eq!(
            ResourceCategory::from_str("energy"),
            Some(ResourceCategory::Energy)
        );
        assert_eq!(
            ResourceCategory::from_str("financial-asset"),
            Some(ResourceCategory::FinancialAsset)
        );
        assert_eq!(
            ResourceCategory::from_str("uba"),
            Some(ResourceCategory::Uba)
        );
        assert_eq!(ResourceCategory::from_str("unknown"), None);
    }

    #[test]
    fn test_make_id_prefix() {
        let id = make_id("res");
        assert!(id.starts_with("res-"));
        let id2 = make_id("alloc");
        assert!(id2.starts_with("alloc-"));
    }

    #[test]
    fn test_weighted_utilization_empty() {
        assert_eq!(weighted_utilization(&[]), 0.0);
    }

    #[test]
    fn test_error_response_format() {
        let resp = error_response(StatusCode::NOT_FOUND, "Resource not found");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
