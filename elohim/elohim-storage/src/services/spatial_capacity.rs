//! Spatial Carrying Capacity Enforcement Service
//!
//! Reads Place carrying capacity and enforces limits on resource allocation.
//! The chain: Resource → SpatialContext → Place → CarryingCapacity → enforcement.
//!
//! Planet-scale limits aggregate up the H3 hierarchy:
//! parcel → community → bioregion → global.
//!
//! Source of truth: Place.carrying_capacity_json (DHT-projected, Category A)

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::db::context::AppContext;
use crate::db::diesel_schema::{economic_events, places};
use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::spatial::CarryingCapacity;

/// Result of checking carrying capacity for a resource allocation.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CapacityCheckResult {
    /// Whether the allocation is allowed within carrying capacity
    pub is_allowed: bool,
    /// Current utilization ratio (0.0 to 1.0+)
    pub current_utilization: f64,
    /// Utilization ratio after the proposed allocation
    pub utilization_after: f64,
    /// Maximum sustainable yield for this resource category
    pub max_sustainable_yield: f64,
    /// Unit of measurement
    pub unit: String,
    /// Current total usage at this place
    pub current_usage: f64,
    /// Whether governance should be triggered (>80% threshold)
    pub trigger_governance: bool,
    /// Place ID checked
    pub place_id: String,
    /// Resource category checked
    pub resource_category: String,
}

/// Get the carrying capacity for a specific resource category at a Place.
///
/// Parses the Place's `carrying_capacity_json` and finds the entry
/// matching the requested resource category.
pub fn get_place_capacity(
    conn: &mut diesel::SqliteConnection,
    ctx: &AppContext,
    place_id: &str,
    resource_category: &str,
) -> Result<Option<CarryingCapacity>, StorageError> {
    // Load place
    let capacity_json: String = places::table
        .filter(places::id.eq(place_id))
        .filter(places::app_id.eq(ctx.app_id()))
        .select(places::carrying_capacity_json)
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                StorageError::NotFound(format!("Place not found: {}", place_id))
            }
            _ => StorageError::Internal(format!("Failed to fetch place capacity: {}", e)),
        })?;

    // Parse carrying capacity array
    let capacities: Vec<CarryingCapacity> =
        serde_json::from_str(&capacity_json).unwrap_or_default();

    // Find matching resource category
    Ok(capacities
        .into_iter()
        .find(|c| c.resource_category == resource_category))
}

/// Compute current resource usage at a Place by aggregating economic events.
///
/// Sums `resource_quantity_value` for consume/use events at the given location.
fn compute_current_usage(
    conn: &mut diesel::SqliteConnection,
    ctx: &AppContext,
    place_id: &str,
    resource_category: &str,
) -> Result<f64, StorageError> {
    use diesel::dsl::sum;

    // Sum resource quantities for consume/use events at this location
    // that match the resource category (via resource_conforms_to)
    let total: Option<f32> = economic_events::table
        .filter(economic_events::app_id.eq(ctx.app_id()))
        .filter(economic_events::at_location.eq(place_id))
        .filter(
            economic_events::action
                .eq("consume")
                .or(economic_events::action.eq("use")),
        )
        .filter(economic_events::resource_conforms_to.eq(resource_category))
        .select(sum(economic_events::resource_quantity_value))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to compute usage: {}", e)))?;

    Ok(total.unwrap_or(0.0) as f64)
}

/// Check whether a proposed resource allocation is within carrying capacity.
///
/// Returns a detailed result including current utilization and whether
/// governance should be triggered (>80% threshold).
pub fn check_carrying_capacity(
    conn: &mut diesel::SqliteConnection,
    ctx: &AppContext,
    place_id: &str,
    resource_category: &str,
    additional_amount: f64,
) -> Result<CapacityCheckResult, StorageError> {
    let capacity = get_place_capacity(conn, ctx, place_id, resource_category)?;

    match capacity {
        None => {
            // No carrying capacity defined — allow but warn
            Ok(CapacityCheckResult {
                is_allowed: true,
                current_utilization: 0.0,
                utilization_after: 0.0,
                max_sustainable_yield: 0.0,
                unit: "unknown".to_string(),
                current_usage: 0.0,
                trigger_governance: false,
                place_id: place_id.to_string(),
                resource_category: resource_category.to_string(),
            })
        }
        Some(cap) => {
            let current_usage = compute_current_usage(conn, ctx, place_id, resource_category)?;
            let usage_after = current_usage + additional_amount;
            let current_util = if cap.max_sustainable_yield > 0.0 {
                current_usage / cap.max_sustainable_yield
            } else {
                0.0
            };
            let util_after = if cap.max_sustainable_yield > 0.0 {
                usage_after / cap.max_sustainable_yield
            } else {
                0.0
            };

            Ok(CapacityCheckResult {
                is_allowed: util_after <= 1.0,
                current_utilization: current_util,
                utilization_after: util_after,
                max_sustainable_yield: cap.max_sustainable_yield,
                unit: cap.unit,
                current_usage,
                trigger_governance: util_after > 0.8,
                place_id: place_id.to_string(),
                resource_category: resource_category.to_string(),
            })
        }
    }
}
