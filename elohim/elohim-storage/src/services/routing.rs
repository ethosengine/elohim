//! OSRM Routing Service
//!
//! Computes routes between spatial points using OSRM (Open Source Routing Machine).
//! Falls back to haversine great-circle estimation when OSRM is unavailable.
//!
//! ## External API
//!
//! OSRM public instance: `https://router.project-osrm.org/route/v1/{profile}/{coords}`
//! - Profiles: `car`, `bike`, `foot`
//! - Coords: `{lng},{lat};{lng},{lat}` (note: longitude first!)
//! - Response: GeoJSON geometry, distance (meters), duration (seconds)
//!
//! Rate-limited public instance; self-host for production.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::services::spatial::GeoPoint;
use crate::views::JsonVal;

// ============================================================================
// Types
// ============================================================================

/// Transport mode for routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum TransportMode {
    Walking,
    Cycling,
    Driving,
    Freight,
    Transit,
}

/// Source of the route computation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum RoutingSource {
    Osrm,
    Manual,
    Estimated,
}

/// Request to compute a route between points.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ComputeRouteInputView {
    pub origin: GeoPoint,
    pub destination: GeoPoint,
    #[serde(default)]
    pub waypoints: Vec<GeoPoint>,
    #[serde(default = "default_transport_mode")]
    pub transport_mode: TransportMode,
}

fn default_transport_mode() -> TransportMode {
    TransportMode::Driving
}

/// Result of a route computation.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RouteResultView {
    /// GeoJSON LineString geometry
    pub geometry: JsonVal,
    /// Total distance in kilometers
    pub distance_km: f64,
    /// Estimated duration in minutes
    pub duration_minutes: f64,
    /// Waypoints used (including origin and destination)
    pub waypoints: Vec<GeoPoint>,
    /// Transport mode used
    pub transport_mode: TransportMode,
    /// Source of the route computation
    pub routing_source: RoutingSource,
    /// ISO timestamp of computation
    pub computed_at: String,
}

// ============================================================================
// OSRM Response Types (internal)
// ============================================================================

#[derive(Debug, Deserialize)]
struct OsrmResponse {
    code: String,
    #[serde(default)]
    routes: Vec<OsrmRoute>,
}

#[derive(Debug, Deserialize)]
struct OsrmRoute {
    geometry: serde_json::Value,
    distance: f64, // meters
    duration: f64, // seconds
}

// ============================================================================
// Routing Error
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum RoutingError {
    #[error("OSRM request failed: {0}")]
    OsrmError(String),
    #[error("Invalid coordinates: {0}")]
    InvalidCoordinates(String),
}

// ============================================================================
// Core Functions
// ============================================================================

/// Compute a route between origin and destination, optionally through waypoints.
///
/// Attempts OSRM first; falls back to haversine straight-line estimate
/// if OSRM is unavailable or returns an error.
pub async fn compute_route(input: &ComputeRouteInputView) -> RouteResultView {
    // Validate coordinates
    if !is_valid_coord(input.origin.latitude, input.origin.longitude)
        || !is_valid_coord(input.destination.latitude, input.destination.longitude)
    {
        return haversine_fallback(input);
    }

    // Try OSRM
    match osrm_route(input).await {
        Ok(result) => result,
        Err(e) => {
            tracing::warn!("OSRM routing failed, using haversine fallback: {}", e);
            haversine_fallback(input)
        }
    }
}

/// Call OSRM public API to compute a route.
async fn osrm_route(input: &ComputeRouteInputView) -> Result<RouteResultView, RoutingError> {
    let profile = transport_mode_to_osrm_profile(&input.transport_mode);

    // Build coordinate string: lng,lat;lng,lat;...
    let mut coords = format!("{},{}", input.origin.longitude, input.origin.latitude);
    for wp in &input.waypoints {
        coords.push_str(&format!(";{},{}", wp.longitude, wp.latitude));
    }
    coords.push_str(&format!(
        ";{},{}",
        input.destination.longitude, input.destination.latitude
    ));

    let url = format!(
        "https://router.project-osrm.org/route/v1/{}/{}?geometries=geojson&overview=full",
        profile, coords
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| RoutingError::OsrmError(e.to_string()))?;

    let resp = client
        .get(&url)
        .header("User-Agent", "ElohimProtocol/1.0")
        .send()
        .await
        .map_err(|e| RoutingError::OsrmError(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(RoutingError::OsrmError(format!(
            "OSRM returned status {}",
            resp.status()
        )));
    }

    let osrm: OsrmResponse = resp
        .json()
        .await
        .map_err(|e| RoutingError::OsrmError(e.to_string()))?;

    if osrm.code != "Ok" || osrm.routes.is_empty() {
        return Err(RoutingError::OsrmError(format!(
            "OSRM response code: {}",
            osrm.code
        )));
    }

    let route = &osrm.routes[0];
    let mut waypoints = vec![input.origin.clone()];
    waypoints.extend(input.waypoints.clone());
    waypoints.push(input.destination.clone());

    Ok(RouteResultView {
        geometry: JsonVal(route.geometry.clone()),
        distance_km: route.distance / 1000.0,
        duration_minutes: route.duration / 60.0,
        waypoints,
        transport_mode: input.transport_mode.clone(),
        routing_source: RoutingSource::Osrm,
        computed_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Haversine great-circle fallback when OSRM is unavailable.
fn haversine_fallback(input: &ComputeRouteInputView) -> RouteResultView {
    let total_km = compute_total_haversine_distance(input);
    let speed_kmh = average_speed_kmh(&input.transport_mode);
    let duration_minutes = if speed_kmh > 0.0 {
        (total_km / speed_kmh) * 60.0
    } else {
        0.0
    };

    // Build a simple GeoJSON LineString from all points
    let mut coordinates: Vec<Vec<f64>> = vec![vec![input.origin.longitude, input.origin.latitude]];
    for wp in &input.waypoints {
        coordinates.push(vec![wp.longitude, wp.latitude]);
    }
    coordinates.push(vec![
        input.destination.longitude,
        input.destination.latitude,
    ]);

    let geometry = JsonVal(serde_json::json!({
        "type": "LineString",
        "coordinates": coordinates,
    }));

    let mut waypoints = vec![input.origin.clone()];
    waypoints.extend(input.waypoints.clone());
    waypoints.push(input.destination.clone());

    RouteResultView {
        geometry,
        distance_km: total_km,
        duration_minutes,
        waypoints,
        transport_mode: input.transport_mode.clone(),
        routing_source: RoutingSource::Estimated,
        computed_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Compute total haversine distance through all waypoints.
fn compute_total_haversine_distance(input: &ComputeRouteInputView) -> f64 {
    let mut points = vec![&input.origin];
    for wp in &input.waypoints {
        points.push(wp);
    }
    points.push(&input.destination);

    let mut total = 0.0;
    for i in 1..points.len() {
        total += haversine_km(
            points[i - 1].latitude,
            points[i - 1].longitude,
            points[i].latitude,
            points[i].longitude,
        );
    }
    total
}

/// Haversine formula — great-circle distance in km.
pub fn haversine_km(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();

    let a = (d_lat / 2.0).sin().powi(2) + lat1_r.cos() * lat2_r.cos() * (d_lng / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_KM * c
}

// ============================================================================
// Helpers
// ============================================================================

fn transport_mode_to_osrm_profile(mode: &TransportMode) -> &'static str {
    match mode {
        TransportMode::Walking => "foot",
        TransportMode::Cycling => "bike",
        TransportMode::Driving | TransportMode::Freight => "car",
        TransportMode::Transit => "car", // OSRM doesn't have transit; approximate
    }
}

fn average_speed_kmh(mode: &TransportMode) -> f64 {
    match mode {
        TransportMode::Walking => 5.0,
        TransportMode::Cycling => 15.0,
        TransportMode::Driving => 60.0,
        TransportMode::Freight => 40.0,
        TransportMode::Transit => 30.0,
    }
}

fn is_valid_coord(lat: f64, lng: f64) -> bool {
    (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lng)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_known_distance() {
        // London to Paris ~344 km
        let d = haversine_km(51.5074, -0.1278, 48.8566, 2.3522);
        assert!((d - 343.5).abs() < 5.0, "Expected ~344 km, got {}", d);
    }

    #[test]
    fn test_haversine_same_point() {
        let d = haversine_km(40.0, -74.0, 40.0, -74.0);
        assert!(d < 0.001);
    }

    #[test]
    fn test_fallback_produces_linestring() {
        let input = ComputeRouteInputView {
            origin: GeoPoint {
                latitude: 40.7128,
                longitude: -74.0060,
                altitude: None,
                accuracy: None,
            },
            destination: GeoPoint {
                latitude: 34.0522,
                longitude: -118.2437,
                altitude: None,
                accuracy: None,
            },
            waypoints: vec![],
            transport_mode: TransportMode::Driving,
        };

        let result = haversine_fallback(&input);
        assert_eq!(result.routing_source, RoutingSource::Estimated);
        assert!(result.distance_km > 3900.0); // NYC to LA ~3944 km great circle
        assert!(result.duration_minutes > 0.0);
        assert_eq!(result.geometry.0["type"].as_str().unwrap(), "LineString");
    }
}
