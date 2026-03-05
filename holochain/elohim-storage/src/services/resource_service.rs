//! Resource service - business logic for stewarded resource management
//!
//! Encapsulates entity construction, dashboard aggregation, constitutional
//! limits enforcement, and resource lifecycle operations.
//!
//! ## Architecture
//!
//! Controller (api/resources.rs) → **Service (this file)** → Model (db/stewardship_allocations.rs, db/economic_events.rs)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;

// =============================================================================
// Enums
// =============================================================================

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
    pub fn as_str(&self) -> &'static str {
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

    pub fn parse(s: &str) -> Option<Self> {
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

impl GovernanceLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            GovernanceLevel::Individual => "individual",
            GovernanceLevel::Household => "household",
            GovernanceLevel::Community => "community",
            GovernanceLevel::Network => "network",
            GovernanceLevel::Constitutional => "constitutional",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AllocationStrategy {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "automatic")]
    Automatic,
    #[serde(rename = "hybrid")]
    Hybrid,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceHealthStatus {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "critical")]
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrendDirection {
    #[serde(rename = "increasing")]
    Increasing,
    #[serde(rename = "stable")]
    Stable,
    #[serde(rename = "decreasing")]
    Decreasing,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "critical")]
    Critical,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EnforcementMethod {
    #[serde(rename = "voluntary")]
    Voluntary,
    #[serde(rename = "progressive")]
    Progressive,
    #[serde(rename = "hard")]
    Hard,
}

// =============================================================================
// Value Objects
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMeasure {
    pub value: f64,
    pub unit: String,
}

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
// Dashboard Types
// =============================================================================

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMetrics {
    pub total_resources_tracked: u32,
    pub categories_covered: u32,
    pub overall_utilization: f64,
    pub fully_allocated_count: u32,
    pub health_status: ResourceHealthStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StewardedResourceDashboard {
    pub steward_id: String,
    pub steward_name: String,
    pub governance_level: String,
    pub categories: Vec<CategorySummary>,
    pub metrics: DashboardMetrics,
    pub alerts: Vec<ResourceAlert>,
    pub insights: Vec<ResourceInsight>,
    pub recent_allocations: Vec<AllocationBlock>,
    pub recent_usage: Vec<UsageRecord>,
    pub last_updated_at: String,
}

// =============================================================================
// Constitutional Limits
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstitutionalLimit {
    pub id: String,
    pub resource_category: ResourceCategory,
    pub name: String,
    pub description: String,
    pub floor_value: f64,
    pub floor_unit: String,
    pub floor_rationale: String,
    pub floor_enforced: bool,
    pub ceiling_value: f64,
    pub ceiling_unit: String,
    pub ceiling_rationale: String,
    pub ceiling_enforced: bool,
    pub safe_min_value: f64,
    pub safe_max_value: f64,
    pub safe_zone_description: String,
    pub governance_level: String,
    pub constitutional_basis: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adoption_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_schedule: Option<String>,
    pub enforcement_method: EnforcementMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_deadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hard_stop_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Request Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResourceRequest {
    pub steward_id: String,
    pub category: ResourceCategory,
    pub subcategory: String,
    pub name: String,
    pub total_capacity: ResourceMeasure,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_level: Option<GovernanceLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observer_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permanent_reserve: Option<ResourceMeasure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAllocationRequest {
    pub label: String,
    pub allocated_amount: ResourceMeasure,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordUsageRequest {
    pub amount: ResourceMeasure,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocation_block_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observer_attestation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

// =============================================================================
// In-memory resource store (keyed by ID)
// Resources are a higher-level abstraction over allocations + events.
// They live in-process and are persisted as economic events.
// =============================================================================

use std::sync::{Mutex, OnceLock};

fn resource_store() -> &'static Mutex<HashMap<String, StewardedResource>> {
    static STORE: OnceLock<Mutex<HashMap<String, StewardedResource>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

// =============================================================================
// Service
// =============================================================================

/// Resource service for stewarded resource management.
pub struct ResourceService;

impl ResourceService {
    // -------------------------------------------------------------------------
    // CRUD
    // -------------------------------------------------------------------------

    /// List all resources for a steward (or all resources if no steward_id).
    pub fn list_resources(steward_id: Option<&str>) -> Vec<StewardedResource> {
        let store = resource_store().lock().unwrap();
        let mut resources: Vec<StewardedResource> = store.values().cloned().collect();
        if let Some(sid) = steward_id {
            resources.retain(|r| r.steward_id == sid);
        }
        resources.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        resources
    }

    /// Get a resource by ID.
    pub fn get_resource(id: &str) -> Option<StewardedResource> {
        let store = resource_store().lock().unwrap();
        store.get(id).cloned()
    }

    /// Create a new stewarded resource.
    pub fn create_resource(
        req: CreateResourceRequest,
        _pool: &DbPool,
        _ctx: &AppContext,
    ) -> Result<StewardedResource, StorageError> {
        let now = chrono::Utc::now().to_rfc3339();
        let id = make_id("res");
        let resource_number = format!("RES-{}", &id[4..16]);
        let dimension = dimension_for_category(&req.category);
        let unit = dimension.unit.clone();
        let allocatable = req.total_capacity.value
            - req
                .permanent_reserve
                .as_ref()
                .map(|r| r.value)
                .unwrap_or(0.0);

        let resource = StewardedResource {
            id: id.clone(),
            resource_number,
            steward_id: req.steward_id.clone(),
            category: req.category,
            subcategory: req.subcategory,
            name: req.name,
            description: req.description,
            dimension,
            total_capacity: req.total_capacity.clone(),
            permanent_reserve: req.permanent_reserve.clone(),
            allocatable_capacity: ResourceMeasure {
                value: allocatable,
                unit: unit.clone(),
            },
            total_allocated: ResourceMeasure {
                value: 0.0,
                unit: unit.clone(),
            },
            total_reserved: ResourceMeasure {
                value: 0.0,
                unit: unit.clone(),
            },
            total_used: ResourceMeasure {
                value: 0.0,
                unit: unit.clone(),
            },
            available: ResourceMeasure {
                value: allocatable,
                unit: unit.clone(),
            },
            allocations: vec![],
            allocation_strategy: AllocationStrategy::Manual,
            allocation_notes: None,
            governance_level: req.governance_level.unwrap_or(GovernanceLevel::Individual),
            governed_by: None,
            constitutional_basis: None,
            can_modify_allocations: true,
            requires_approval_for: None,
            observer_enabled: req.observer_enabled.unwrap_or(false),
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
            updated_at: now,
        };

        let mut store = resource_store().lock().unwrap();
        store.insert(id, resource.clone());
        Ok(resource)
    }

    // -------------------------------------------------------------------------
    // Allocations
    // -------------------------------------------------------------------------

    /// Create an allocation on a resource.
    pub fn create_allocation(
        resource_id: &str,
        req: CreateAllocationRequest,
        _pool: &DbPool,
        _ctx: &AppContext,
    ) -> Result<AllocationBlock, StorageError> {
        let now = chrono::Utc::now().to_rfc3339();
        let alloc_id = make_id("alloc");

        let mut store = resource_store().lock().unwrap();
        let resource = store
            .get_mut(resource_id)
            .ok_or_else(|| StorageError::NotFound(resource_id.to_string()))?;

        let unit = resource.allocatable_capacity.unit.clone();
        let amount = req.allocated_amount.value;

        // Validate capacity
        if amount > resource.available.value {
            return Err(StorageError::InvalidInput(format!(
                "Insufficient available capacity: requested {:.2}, available {:.2}",
                amount, resource.available.value
            )));
        }

        let utilization = utilization_percent(0.0, amount);

        let block = AllocationBlock {
            id: alloc_id,
            resource_id: resource_id.to_string(),
            label: req.label,
            description: req.description,
            allocated: ResourceMeasure {
                value: amount,
                unit: unit.clone(),
            },
            used: ResourceMeasure {
                value: 0.0,
                unit: unit.clone(),
            },
            reserved: ResourceMeasure {
                value: 0.0,
                unit: unit.clone(),
            },
            governance_level: req.governance_level,
            governed_by: None,
            commitment_id: None,
            priority: req.priority.unwrap_or(0),
            utilization,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        // Update resource totals
        resource.total_allocated.value += amount;
        resource.available.value -= amount;
        resource.updated_at = now;
        resource.allocations.push(block.clone());

        Ok(block)
    }

    // -------------------------------------------------------------------------
    // Usage
    // -------------------------------------------------------------------------

    /// Record usage against a resource (and optionally an allocation block).
    pub fn record_usage(
        resource_id: &str,
        req: RecordUsageRequest,
        pool: &DbPool,
        ctx: &AppContext,
    ) -> Result<UsageRecord, StorageError> {
        let now = chrono::Utc::now().to_rfc3339();
        let usage_id = make_id("usage");

        let amount = req.amount.value;

        // Record economic event
        let event_id = {
            use crate::db::economic_events::{record_event, CreateEconomicEventInput};
            let mut conn = pool.get().map_err(|e| {
                StorageError::Internal(format!("Failed to get DB connection: {}", e))
            })?;
            let event = record_event(
                &mut conn,
                ctx,
                CreateEconomicEventInput {
                    id: None,
                    action: "use".to_string(),
                    provider: resource_id.to_string(),
                    receiver: "system".to_string(),
                    resource_conforms_to: None,
                    resource_inventoried_as: Some(resource_id.to_string()),
                    resource_classified_as: vec!["stewarded-resource".to_string()],
                    resource_quantity_value: Some(amount as f32),
                    resource_quantity_unit: None,
                    effort_quantity_value: None,
                    effort_quantity_unit: None,
                    has_point_in_time: Some(now.clone()),
                    has_duration: None,
                    input_of: None,
                    output_of: None,
                    lamad_event_type: None,
                    content_id: None,
                    contributor_presence_id: None,
                    path_id: None,
                    triggered_by: None,
                    note: req.note.clone(),
                    metadata_json: None,
                },
            )?;
            event.id.clone()
        };

        let mut store = resource_store().lock().unwrap();
        let resource = store
            .get_mut(resource_id)
            .ok_or_else(|| StorageError::NotFound(resource_id.to_string()))?;

        let unit = resource.total_capacity.unit.clone();

        // Update allocation block if specified
        if let Some(ref alloc_id) = req.allocation_block_id {
            if let Some(block) = resource.allocations.iter_mut().find(|b| &b.id == alloc_id) {
                block.used.value += amount;
                block.utilization = utilization_percent(block.used.value, block.allocated.value);
                block.updated_at = now.clone();
            }
        }

        // Update resource totals
        resource.total_used.value += amount;
        resource.usage_event_ids.push(event_id.clone());
        resource.updated_at = now.clone();

        let record = UsageRecord {
            id: usage_id,
            resource_id: resource_id.to_string(),
            economic_event_id: Some(event_id),
            allocation_block_id: req.allocation_block_id,
            action: req.action.unwrap_or_else(|| "use".to_string()),
            quantity: ResourceMeasure {
                value: amount,
                unit,
            },
            duration: None,
            observer_attestation_id: req.observer_attestation_id,
            timestamp: now.clone(),
            note: req.note,
        };

        resource.recent_usage.push(record.clone());

        Ok(record)
    }

    // -------------------------------------------------------------------------
    // Dashboard
    // -------------------------------------------------------------------------

    /// Build a full dashboard for a steward.
    pub fn get_dashboard(steward_id: &str) -> StewardedResourceDashboard {
        let now = chrono::Utc::now().to_rfc3339();
        let resources = Self::list_resources(Some(steward_id));

        // Group by category
        let mut category_map: HashMap<String, Vec<StewardedResource>> = HashMap::new();
        for resource in &resources {
            category_map
                .entry(resource.category.as_str().to_string())
                .or_default()
                .push(resource.clone());
        }

        let mut categories: Vec<CategorySummary> = category_map
            .into_iter()
            .map(|(cat_str, cat_resources)| {
                let total_cap: f64 = cat_resources.iter().map(|r| r.total_capacity.value).sum();
                let total_alloc: f64 = cat_resources.iter().map(|r| r.total_allocated.value).sum();
                let total_used: f64 = cat_resources.iter().map(|r| r.total_used.value).sum();
                let util = utilization_percent(total_used, total_cap);
                let unit = cat_resources
                    .first()
                    .map(|r| r.total_capacity.unit.clone())
                    .unwrap_or_default();
                let category =
                    ResourceCategory::parse(&cat_str).unwrap_or(ResourceCategory::Inventory);

                CategorySummary {
                    category,
                    resources: cat_resources,
                    total_capacity: ResourceMeasure {
                        value: total_cap,
                        unit: unit.clone(),
                    },
                    total_allocated: ResourceMeasure {
                        value: total_alloc,
                        unit: unit.clone(),
                    },
                    total_used: ResourceMeasure {
                        value: total_used,
                        unit,
                    },
                    utilization_percent: util,
                    health_status: health_status(util),
                }
            })
            .collect();

        categories.sort_by(|a, b| a.category.as_str().cmp(b.category.as_str()));

        // Metrics
        let total_resources = resources.len() as u32;
        let categories_covered = categories.len() as u32;
        let overall_util = weighted_utilization(&categories);
        let fully_allocated = resources
            .iter()
            .filter(|r| r.available.value <= 0.0)
            .count() as u32;
        let overall_health = health_status(overall_util);

        let metrics = DashboardMetrics {
            total_resources_tracked: total_resources,
            categories_covered,
            overall_utilization: overall_util,
            fully_allocated_count: fully_allocated,
            health_status: overall_health,
        };

        let alerts = generate_alerts(&resources, &now);

        // Collect recent allocations and usage
        let recent_allocations: Vec<AllocationBlock> = resources
            .iter()
            .flat_map(|r| r.allocations.iter().cloned())
            .collect();
        let recent_usage: Vec<UsageRecord> = resources
            .iter()
            .flat_map(|r| r.recent_usage.iter().cloned())
            .collect();

        StewardedResourceDashboard {
            steward_id: steward_id.to_string(),
            steward_name: steward_id.to_string(),
            governance_level: "individual".to_string(),
            categories,
            metrics,
            alerts,
            insights: vec![],
            recent_allocations,
            recent_usage,
            last_updated_at: now,
        }
    }

    // -------------------------------------------------------------------------
    // Constitutional Limits
    // -------------------------------------------------------------------------

    /// Get the constitutional limit for a given category string.
    pub fn get_constitutional_limit(category: &str) -> Option<ConstitutionalLimit> {
        let cat = ResourceCategory::parse(category)?;
        constitutional_limit_for(&cat)
    }
}

// =============================================================================
// Private Helpers
// =============================================================================

fn random_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
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

fn utilization_percent(used: f64, capacity: f64) -> f64 {
    if capacity <= 0.0 {
        return 0.0;
    }
    (used / capacity * 100.0).min(100.0)
}

fn health_status(utilization: f64) -> ResourceHealthStatus {
    if utilization > 90.0 {
        ResourceHealthStatus::Critical
    } else if utilization > 75.0 {
        ResourceHealthStatus::Warning
    } else {
        ResourceHealthStatus::Healthy
    }
}

fn weighted_utilization(categories: &[CategorySummary]) -> f64 {
    let total_capacity: f64 = categories.iter().map(|c| c.total_capacity.value).sum();
    if total_capacity <= 0.0 {
        return 0.0;
    }
    let weighted_used: f64 = categories.iter().map(|c| c.total_used.value).sum();
    (weighted_used / total_capacity * 100.0).min(100.0)
}

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
                    unallocated / capacity * 100.0,
                    resource.name
                ),
                recommended_action: Some("Create allocation blocks for this resource.".into()),
                created_at: now.to_string(),
            });
        }
    }
    alerts
}
