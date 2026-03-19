//! Distribution Planning Service
//!
//! Given resource supply at locations and demand at other locations, computes
//! an optimal distribution plan using greedy nearest-neighbor matching.
//!
//! This is the Cybersyn logistics layer — coordinating physical resource
//! movement across governed space.
//!
//! ## Algorithm
//!
//! MVP uses greedy nearest-neighbor: for each demand point, find the nearest
//! supply point with available quantity, compute route, allocate. More
//! sophisticated algorithms (min-cost flow, CVRP) are future work.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::services::routing::{self, ComputeRouteInputView, RouteResultView, TransportMode};
use crate::services::spatial::GeoPoint;

// ============================================================================
// Types
// ============================================================================

/// A point with available resource supply.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SupplyPoint {
    pub place_id: String,
    pub name: String,
    pub location: GeoPoint,
    pub available_quantity: f64,
    pub unit: String,
}

/// A point with resource demand.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DemandPoint {
    pub place_id: String,
    pub name: String,
    pub location: GeoPoint,
    pub needed_quantity: f64,
    pub unit: String,
}

/// Request to compute a distribution plan.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DistributionRequestView {
    pub resource_category: String,
    pub supply_points: Vec<SupplyPoint>,
    pub demand_points: Vec<DemandPoint>,
    #[serde(default = "default_transport_mode")]
    pub transport_mode: TransportMode,
}

fn default_transport_mode() -> TransportMode {
    TransportMode::Driving
}

/// A single planned route in a distribution plan.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PlannedRoute {
    /// Supply point place ID
    pub from_place_id: String,
    pub from_name: String,
    /// Demand point place ID
    pub to_place_id: String,
    pub to_name: String,
    /// Quantity to transport
    pub quantity: f64,
    pub unit: String,
    /// Computed route details
    pub route: RouteResultView,
}

/// Result of distribution planning.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DistributionPlanView {
    pub resource_category: String,
    /// Planned routes from supply to demand
    pub routes: Vec<PlannedRoute>,
    /// Total distance across all routes (km)
    pub total_distance_km: f64,
    /// Total duration across all routes (minutes)
    pub total_duration_minutes: f64,
    /// Demand points that couldn't be fully fulfilled
    pub unmet_demand: Vec<UnmetDemand>,
    /// Supply points with remaining unused quantity
    pub excess_supply: Vec<ExcessSupply>,
    /// ISO timestamp of plan computation
    pub computed_at: String,
}

/// Demand that could not be fulfilled.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UnmetDemand {
    pub place_id: String,
    pub name: String,
    pub needed_quantity: f64,
    pub fulfilled_quantity: f64,
    pub shortfall: f64,
    pub unit: String,
}

/// Supply that was not needed.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ExcessSupply {
    pub place_id: String,
    pub name: String,
    pub remaining_quantity: f64,
    pub unit: String,
}

// ============================================================================
// Core Algorithm
// ============================================================================

/// Compute a distribution plan using greedy nearest-neighbor matching.
///
/// For each demand point (sorted by quantity descending), find the nearest
/// supply point with available quantity, route it, and allocate.
pub async fn compute_distribution(input: &DistributionRequestView) -> DistributionPlanView {
    // Mutable copies for tracking remaining quantities
    let mut supply_remaining: Vec<(usize, f64)> = input
        .supply_points
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s.available_quantity))
        .collect();

    let mut demand_remaining: Vec<(usize, f64)> = input
        .demand_points
        .iter()
        .enumerate()
        .map(|(i, d)| (i, d.needed_quantity))
        .collect();

    let mut planned_routes = Vec::new();
    let mut total_distance = 0.0;
    let mut total_duration = 0.0;

    // Sort demand by quantity descending (fulfill largest needs first)
    demand_remaining.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for &mut (di, ref mut demand_left) in demand_remaining.iter_mut() {
        if *demand_left <= 0.0 {
            continue;
        }

        let demand_pt = &input.demand_points[di];

        // Find nearest supply points with available quantity
        // Sort supply by distance to this demand point
        let mut supply_by_distance: Vec<(usize, f64, f64)> = supply_remaining
            .iter()
            .filter(|(_, qty)| *qty > 0.0)
            .map(|&(si, qty)| {
                let supply_pt = &input.supply_points[si];
                let dist = routing::haversine_km(
                    supply_pt.location.latitude,
                    supply_pt.location.longitude,
                    demand_pt.location.latitude,
                    demand_pt.location.longitude,
                );
                (si, dist, qty)
            })
            .collect();

        supply_by_distance
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Allocate from nearest supply points until demand is met
        for (si, _dist, _avail) in &supply_by_distance {
            if *demand_left <= 0.0 {
                break;
            }

            let supply_entry = supply_remaining.iter_mut().find(|(i, _)| i == si);
            let Some((_, ref mut supply_qty)) = supply_entry else {
                continue;
            };

            if *supply_qty <= 0.0 {
                continue;
            }

            let supply_pt = &input.supply_points[*si];
            let allocate = demand_left.min(*supply_qty);

            // Compute route
            let route_input = ComputeRouteInputView {
                origin: supply_pt.location.clone(),
                destination: demand_pt.location.clone(),
                waypoints: vec![],
                transport_mode: input.transport_mode.clone(),
            };
            let route = routing::compute_route(&route_input).await;

            total_distance += route.distance_km;
            total_duration += route.duration_minutes;

            planned_routes.push(PlannedRoute {
                from_place_id: supply_pt.place_id.clone(),
                from_name: supply_pt.name.clone(),
                to_place_id: demand_pt.place_id.clone(),
                to_name: demand_pt.name.clone(),
                quantity: allocate,
                unit: demand_pt.unit.clone(),
                route,
            });

            *supply_qty -= allocate;
            *demand_left -= allocate;
        }
    }

    // Collect unmet demand
    let unmet_demand: Vec<UnmetDemand> = demand_remaining
        .iter()
        .filter(|(_, remaining)| *remaining > 0.001)
        .map(|&(di, remaining)| {
            let dp = &input.demand_points[di];
            UnmetDemand {
                place_id: dp.place_id.clone(),
                name: dp.name.clone(),
                needed_quantity: dp.needed_quantity,
                fulfilled_quantity: dp.needed_quantity - remaining,
                shortfall: remaining,
                unit: dp.unit.clone(),
            }
        })
        .collect();

    // Collect excess supply
    let excess_supply: Vec<ExcessSupply> = supply_remaining
        .iter()
        .filter(|(_, remaining)| *remaining > 0.001)
        .map(|&(si, remaining)| {
            let sp = &input.supply_points[si];
            ExcessSupply {
                place_id: sp.place_id.clone(),
                name: sp.name.clone(),
                remaining_quantity: remaining,
                unit: sp.unit.clone(),
            }
        })
        .collect();

    DistributionPlanView {
        resource_category: input.resource_category.clone(),
        routes: planned_routes,
        total_distance_km: total_distance,
        total_duration_minutes: total_duration,
        unmet_demand,
        excess_supply,
        computed_at: chrono::Utc::now().to_rfc3339(),
    }
}
