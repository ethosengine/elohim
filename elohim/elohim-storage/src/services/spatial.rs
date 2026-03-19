//! Spatial primitives — geospatial types for the Elohim Protocol
//!
//! Provides structured spatial types that replace opaque `location: String` fields
//! throughout the protocol. Built on H3 hexagonal spatial indexing which maps
//! directly to the constitutional governance hierarchy.
//!
//! ## Two Spatial Concepts
//!
//! 1. **Place** — Named, governed spatial entity (city, watershed, parcel).
//!    Notarized on Mishpat DNA because boundary-drawing is governance.
//!    Sprint 2 deliverable.
//!
//! 2. **SpatialContext** — Where something IS (lat/lng attached to any entity).
//!    Polymorphic storage table like Schedule. Sprint 1 deliverable.
//!
//! ## H3 Resolution → Constitutional Layer
//!
//! ```text
//! Res 0-2  (~4.3M-87K km²)  → Global
//! Res 3    (~12K km²)        → Bioregional
//! Res 4    (~1,770 km²)      → Provincial
//! Res 5    (~253 km²)        → Municipal
//! Res 6-7  (~36-5.2 km²)    → Community/Neighborhood
//! Res 8-9  (~0.74-0.1 km²)  → Family/Local (parcel)
//! Res 10+  (<0.1 km²)       → Individual (sensor/device)
//! ```
//!
//! ## Source of Truth
//!
//! Value objects: inherited from parent entity.
//! SpatialContext table: SQLite (operational, Category C).
//! Place (future): Mishpat DHT (notarized, Category A).
//!
//! Protocol schemas: `elohim/sdk/schemas/v1/objects/geo-point.schema.json` etc.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ============================================================================
// Core Value Objects
// ============================================================================

/// WGS84 coordinate pair — the atomic spatial primitive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GeoPoint {
    /// Latitude in decimal degrees (-90 to 90)
    pub latitude: f64,
    /// Longitude in decimal degrees (-180 to 180)
    pub longitude: f64,
    /// Optional altitude in meters above WGS84 ellipsoid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude: Option<f64>,
    /// Optional accuracy radius in meters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
}

/// H3 hexagonal spatial index cell.
///
/// Maps to constitutional governance layers via resolution:
/// res 0-2 = global, res 3 = bioregional, res 5 = municipal,
/// res 7 = neighborhood, res 9 = parcel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct H3Cell {
    /// H3 index as 15-character hex string (e.g., "8928308280fffff")
    pub index: String,
    /// Resolution level 0-15
    pub resolution: u8,
}

// ============================================================================
// Enums
// ============================================================================

/// Classification of governed spatial entities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum PlaceType {
    /// City, county, state, nation (political boundary)
    Administrative,
    /// Watershed, bioregion, forest, ecosystem (ecological boundary)
    Bioregional,
    /// Land parcel or property
    Parcel,
    /// Solar farm, water treatment, school (built infrastructure)
    Infrastructure,
    /// Community center, market, park (social space)
    Gathering,
    /// Waterbody or hydrological catchment
    Watershed,
    /// Farmland, greenhouse, orchard
    Agricultural,
    /// Community-defined place type
    Custom,
}

/// Lifecycle status of a governed Place.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum PlaceStatus {
    /// Recognized boundary with active governance
    Active,
    /// Boundary under governance proposal review
    Proposed,
    /// Boundary under challenge
    Disputed,
    /// No longer recognized
    Dissolved,
}

/// OpenStreetMap element type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum OsmElementType {
    /// Point feature
    Node,
    /// Line or polygon
    Way,
    /// Complex boundary or route
    Relation,
}

/// Type of spatial context attached to an entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum SpatialContextType {
    /// Single location (lat/lng)
    Point,
    /// Polygon boundary (GeoJSON)
    Area,
    /// Path with waypoints (GeoJSON LineString)
    Route,
}

// ============================================================================
// Composite Value Objects
// ============================================================================

/// Pointer to an OpenStreetMap element.
/// OSM is reference data — the protocol references it, doesn't embed it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct OsmReference {
    pub osm_type: OsmElementType,
    pub osm_id: i64,
    /// Optional version for reproducibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
}

/// Ecological carrying capacity for a resource category within a Place.
/// Planet-scale limits aggregate up the H3 hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CarryingCapacity {
    /// ResourceCategory this limit applies to (energy, water, food, etc.)
    pub resource_category: String,
    /// Maximum sustainable yield before degradation
    pub max_sustainable_yield: f64,
    /// Unit of measurement
    pub unit: String,
    /// Current utilization (0.0 to 1.0+; >1.0 means exceeding capacity)
    pub current_utilization: f64,
    /// Confidence in the measurement
    pub data_quality: String,
    /// Source of the capacity estimate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// When last measured/estimated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measured_at: Option<String>,
}

// ============================================================================
// H3 ↔ Constitutional Layer Mapping
// ============================================================================

/// Map H3 resolution to constitutional layer.
///
/// This is the bridge between physical space and governance authority.
/// A parcel (res 9) falls under family governance. A watershed (res 3)
/// falls under bioregional governance. The mapping is the protocol's
/// answer to "who governs this space?"
pub fn h3_resolution_to_constitutional_layer(resolution: u8) -> &'static str {
    match resolution {
        0..=2 => "global",
        3 => "bioregional",
        4 => "nation-state",
        5 => "provincial",
        6 => "community",
        7 => "community",
        8..=9 => "family",
        _ => "individual",
    }
}

/// Map H3 resolution to reach level for content/resource visibility.
pub fn h3_resolution_to_reach(resolution: u8) -> &'static str {
    match resolution {
        0..=4 => "commons",
        5 => "municipal",
        6 => "neighborhood",
        7 => "neighborhood",
        8..=9 => "local",
        _ => "private",
    }
}

// ============================================================================
// H3 Computation Helpers
// ============================================================================

/// Compute H3 cell index from lat/lng at a given resolution.
///
/// Returns the H3 hex string (15 chars) or an error if coordinates are invalid.
pub fn lat_lng_to_h3(latitude: f64, longitude: f64, resolution: u8) -> Result<String, String> {
    use h3o::{CellIndex, LatLng, Resolution};

    let res = Resolution::try_from(resolution)
        .map_err(|_| format!("Invalid H3 resolution: {resolution} (must be 0-15)"))?;

    let ll = LatLng::new(latitude, longitude)
        .map_err(|e| format!("Invalid coordinates ({latitude}, {longitude}): {e}"))?;

    let cell: CellIndex = ll.to_cell(res);
    Ok(cell.to_string())
}

/// Compute H3 cells at three standard resolutions (5=municipal, 7=neighborhood, 9=parcel).
/// Returns (h3_res5, h3_res7, h3_res9) or None for each that fails.
pub fn compute_h3_cells(
    latitude: f64,
    longitude: f64,
) -> (Option<String>, Option<String>, Option<String>) {
    (
        lat_lng_to_h3(latitude, longitude, 5).ok(),
        lat_lng_to_h3(latitude, longitude, 7).ok(),
        lat_lng_to_h3(latitude, longitude, 9).ok(),
    )
}

// ============================================================================
// GeoPoint Helpers
// ============================================================================

impl GeoPoint {
    /// Compute the H3 cell for this point at a given resolution.
    pub fn to_h3(&self, resolution: u8) -> Result<H3Cell, String> {
        let index = lat_lng_to_h3(self.latitude, self.longitude, resolution)?;
        Ok(H3Cell { index, resolution })
    }

    /// Compute H3 cells at the three standard governance resolutions.
    pub fn to_governance_cells(&self) -> (Option<H3Cell>, Option<H3Cell>, Option<H3Cell>) {
        (
            self.to_h3(5).ok(), // Municipal
            self.to_h3(7).ok(), // Neighborhood
            self.to_h3(9).ok(), // Parcel
        )
    }
}
