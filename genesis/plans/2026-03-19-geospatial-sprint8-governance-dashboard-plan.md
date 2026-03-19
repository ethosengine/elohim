# Sprint 8: Planet-Scale Governance Dashboard — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a backend aggregate endpoint and enhance the map component to render a governance dashboard scoped by constitutional layer — the Cybersyn control room.

**Architecture:** Single `GET /api/v1/dashboard/spatial` endpoint in elohim-storage composes Places + optional enrichments (hazards, vulnerability, weather, capacity, alerts, routes) into one payload. `include` query param controls which enrichments are computed. Angular `SpatialMapComponent` enhanced with risk heatmap, hazard markers, alert badges, weather overlay, and a dashboard sidebar panel.

**Tech Stack:** Rust (diesel, dashmap, serde, ts-rs), Angular 19 (signals, MapLibre GL), TypeScript.

**Design doc:** `genesis/plans/2026-03-19-geospatial-sprint8-governance-dashboard-design.md`

---

### Task 1: Dashboard View Types

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs` (append before `#[cfg(test)]`)

**Step 1: Add all dashboard view types to views.rs**

Append these types. All use `#[derive(Debug, Clone, Serialize, TS)]`, `#[serde(rename_all = "camelCase")]`, `#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]`. Use `JsonVal` for geometry_json.

```rust
// ============================================================================
// Spatial Dashboard Views (Sprint 8: Planet-Scale Governance Dashboard)
// ============================================================================

/// Summary of carrying capacity stress for a Place
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CapacitySummaryView {
    pub resource_count: u32,
    pub worst_utilization: f64,
    pub worst_category: String,
    pub trigger_governance: bool,
}

/// Summary of active hazards for a Place
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HazardEntrySummaryView {
    pub active_count: u32,
    pub worst_severity: String,
    pub nearest_onset_hours: Option<f64>,
    pub types: Vec<String>,
}

/// Summary of vulnerability for a Place
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct VulnerabilitySummaryView {
    pub overall_score: f64,
    pub risk_tier: String,
    pub preparation_status: String,
}

/// Summary of weather for a Place
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct WeatherEntrySummaryView {
    pub risk_level: String,
    pub temperature_c: f64,
    pub precipitation_mm: f64,
    pub wind_speed_kmh: f64,
}

/// Summary of active alerts for a Place
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AlertEntrySummaryView {
    pub active_count: u32,
    pub worst_severity: String,
    pub unacknowledged_count: u32,
    pub types: Vec<String>,
}

/// Summary of distribution routes touching a Place
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RouteEntrySummaryView {
    pub active_route_count: u32,
    pub total_distance_km: f64,
}

/// Per-Place entry in the spatial dashboard
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PlaceDashboardEntry {
    // Core (always included)
    pub id: String,
    pub name: String,
    pub place_type: String,
    pub constitutional_layer: String,
    pub h3_index: String,
    pub centroid_lat: f64,
    pub centroid_lng: f64,
    pub geometry_json: Option<JsonVal>,
    pub status: String,
    pub parent_place_id: Option<String>,

    // Optional enrichments (None if not requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<CapacitySummaryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hazards: Option<HazardEntrySummaryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vulnerability: Option<VulnerabilitySummaryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather: Option<WeatherEntrySummaryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alerts: Option<AlertEntrySummaryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<RouteEntrySummaryView>,
}

/// Risk tier distribution counts
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RiskTierDistribution {
    pub low: u32,
    pub moderate: u32,
    pub elevated: u32,
    pub critical: u32,
    pub unknown: u32,
}

/// Aggregate summary across all Places in the dashboard
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DashboardSummaryView {
    pub total_places: u32,
    pub places_by_risk_tier: RiskTierDistribution,
    pub total_active_hazards: u32,
    pub total_active_alerts: u32,
    pub total_unacknowledged_alerts: u32,
    pub worst_overall_risk: String,
}

/// Top-level spatial dashboard response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SpatialDashboardView {
    pub places: Vec<PlaceDashboardEntry>,
    pub summary: DashboardSummaryView,
    pub queried_at: String,
}
```

**Step 2: Cargo check**

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`

**Step 3: Commit**

---

### Task 2: Spatial Dashboard Service

**Files:**
- Create: `elohim/elohim-storage/src/services/spatial_dashboard.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod spatial_dashboard;`)

**Step 1: Write the service**

The service orchestrates the aggregate query. Key structure:

```rust
//! Spatial Dashboard Service
//!
//! Composes the governance dashboard view from Places + optional enrichments.
//! All composition logic in the backend — frontend is a thin rendering layer.
//!
//! Batch queries for hazards/alerts. Vulnerability/weather from DashMap caches.

use std::collections::{HashMap, HashSet};
use crate::db::{AppContext, DbPool};
use crate::db::places::PlaceQuery;
use crate::error::StorageError;
use crate::views::*;

/// Parsed include flags from query string
pub struct IncludeFlags {
    pub capacity: bool,
    pub hazards: bool,
    pub vulnerability: bool,
    pub weather: bool,
    pub alerts: bool,
    pub routes: bool,
}

impl IncludeFlags {
    /// Parse from comma-separated string. None/empty = all true (default).
    pub fn parse(include_str: Option<&str>) -> Self { ... }

    /// All flags enabled (default when include param omitted)
    pub fn all() -> Self { ... }
}

/// Dashboard query parameters
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardQuery {
    pub constitutional_layer: Option<String>,
    pub h3_index: Option<String>,
    pub h3_resolution: Option<i32>,
    pub parent_place_id: Option<String>,
    pub status: Option<String>,
    pub include: Option<String>,
    pub limit: Option<i64>,
}

/// Build the spatial dashboard view.
pub async fn build_dashboard(
    pool: &DbPool,
    ctx: &AppContext,
    query: &DashboardQuery,
) -> Result<SpatialDashboardView, StorageError> {
    let flags = IncludeFlags::parse(query.include.as_deref());

    // 1. Load places (always)
    let place_query = PlaceQuery {
        constitutional_layer: query.constitutional_layer.clone(),
        h3_index: query.h3_index.clone(),
        parent_place_id: query.parent_place_id.clone(),
        status: query.status.clone().or(Some("active".to_string())),
        limit: query.limit.or(Some(200)),
        ..Default::default()
    };
    let conn = pool.get().map_err(|e| StorageError::Internal(e.to_string()))?;
    let places = crate::db::places::list_places(&mut conn, ctx, &place_query)?;
    let place_ids: Vec<&str> = places.iter().map(|p| p.id.as_str()).collect();

    // 2. Batch-load enrichments
    let hazards_by_place = if flags.hazards {
        batch_load_hazards(&mut conn, ctx, &place_ids)?
    } else { HashMap::new() };

    let alerts_by_place = if flags.alerts {
        batch_load_alerts(&mut conn, ctx, &place_ids)?
    } else { HashMap::new() };

    // 3. Build per-place entries (vulnerability + weather are async/cached)
    let mut entries = Vec::with_capacity(places.len());
    let mut summary = DashboardSummaryView { ... defaults ... };

    for place in &places {
        let capacity = if flags.capacity {
            Some(compute_capacity_summary(place))
        } else { None };

        let hazard_summary = if flags.hazards {
            Some(compute_hazard_summary(hazards_by_place.get(&place.id)))
        } else { None };

        let vuln_summary = if flags.vulnerability {
            let v = crate::services::vulnerability::compute_vulnerability(pool, ctx, &place.id).await;
            Some(VulnerabilitySummaryView {
                overall_score: v.overall_vulnerability,
                risk_tier: format!("{:?}", v.risk_tier).to_lowercase(),
                preparation_status: format!("{:?}", v.preparation_status).to_lowercase(),
            })
        } else { None };

        let weather_summary = if flags.weather {
            let f = crate::services::weather::get_forecast(&place.id, place.centroid_lat, place.centroid_lng).await;
            Some(WeatherEntrySummaryView {
                risk_level: /* map WeatherRisk to string */,
                temperature_c: f.current.temperature_c,
                precipitation_mm: f.current.precipitation_mm,
                wind_speed_kmh: f.current.wind_speed_kmh,
            })
        } else { None };

        let alert_summary = if flags.alerts {
            Some(compute_alert_summary(alerts_by_place.get(&place.id)))
        } else { None };

        let route_summary = if flags.routes {
            Some(RouteEntrySummaryView { active_route_count: 0, total_distance_km: 0.0 })
            // MVP stub — needs SpatialContext route query
        } else { None };

        // Accumulate summary stats
        // ... update summary counters based on enrichments ...

        entries.push(PlaceDashboardEntry {
            id: place.id.clone(),
            name: place.name.clone(),
            place_type: place.place_type.clone(),
            constitutional_layer: place.constitutional_layer.clone(),
            h3_index: place.h3_index.clone(),
            centroid_lat: place.centroid_lat,
            centroid_lng: place.centroid_lng,
            geometry_json: Some(parse_json(&place.geometry_json)),
            status: place.status.clone(),
            parent_place_id: place.parent_place_id.clone(),
            capacity, hazards: hazard_summary, vulnerability: vuln_summary,
            weather: weather_summary, alerts: alert_summary, routes: route_summary,
        });
    }

    Ok(SpatialDashboardView {
        places: entries,
        summary,
        queried_at: chrono::Utc::now().to_rfc3339(),
    })
}
```

Batch helpers:
- `batch_load_hazards(conn, ctx, place_ids) -> HashMap<String, Vec<Hazard>>` — single SQL query with `WHERE place_id IN (...) AND status IN ('active', 'monitoring')`, group by place_id
- `batch_load_alerts(conn, ctx, place_ids) -> HashMap<String, Vec<RiskAlert>>` — same pattern with `WHERE status = 'active'`
- `compute_capacity_summary(place) -> CapacitySummaryView` — parse `carrying_capacity_json`, find worst utilization
- `compute_hazard_summary(hazards) -> HazardEntrySummaryView` — count, worst severity, nearest onset
- `compute_alert_summary(alerts) -> AlertEntrySummaryView` — count, worst severity, unacknowledged count

**Step 2: Add `pub mod spatial_dashboard;` to services/mod.rs**

**Step 3: Cargo check + commit**

---

### Task 3: Dashboard API Controller

**Files:**
- Create: `elohim/elohim-storage/src/api/dashboard.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs` (add `pub mod dashboard;` + dispatch)

**Step 1: Write the controller**

```rust
//! Dashboard API controller
//!
//! Routes:
//!   GET `/api/v1/dashboard/spatial` — aggregate governance dashboard

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::services::spatial_dashboard::DashboardQuery;

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        (&Method::GET, "spatial") | (&Method::GET, "spatial/") => {
            handle_spatial_dashboard(req, pool, ctx).await
        }
        _ => response::not_found(&format!(
            "Unknown dashboard route: {} /api/v1/dashboard/{}",
            method, path
        )),
    })
}

async fn handle_spatial_dashboard(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let query_str = req.uri().query().unwrap_or("");
    let query: DashboardQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    match crate::services::spatial_dashboard::build_dashboard(pool, ctx, &query).await {
        Ok(view) => response::ok(&view),
        Err(e) => response::error_response(e),
    }
}
```

**Step 2: Add to api/mod.rs**

Add `pub mod dashboard;` to declarations.

Add dispatch before `places`:
```rust
} else if sub_path.starts_with("dashboard") {
    let resource_path = sub_path.strip_prefix("dashboard").unwrap_or("");
    dashboard::handle(req, method, resource_path, &pool, &app_ctx).await
```

**Step 3: Cargo check + commit**

---

### Task 4: Batch Query Helpers for Hazards and Alerts

**Files:**
- Modify: `elohim/elohim-storage/src/db/hazards.rs` (add `list_hazards_for_places`)
- Modify: `elohim/elohim-storage/src/db/risk_alerts.rs` (add `list_alerts_for_places`)

**Step 1: Add batch query to hazards.rs**

```rust
/// Batch-load active hazards for multiple places in one query.
pub fn list_hazards_for_places(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    place_ids: &[&str],
) -> Result<Vec<Hazard>, StorageError> {
    use super::diesel_schema::hazards::dsl::*;

    hazards
        .filter(app_id.eq(ctx.app_id()))
        .filter(place_id.eq_any(place_ids))
        .filter(status.eq("active").or(status.eq("monitoring")))
        .order(place_id.asc())
        .load::<Hazard>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to batch-load hazards: {}", e)))
}
```

**Step 2: Add batch query to risk_alerts.rs**

```rust
/// Batch-load active alerts for multiple places in one query.
pub fn list_alerts_for_places(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    place_ids: &[&str],
) -> Result<Vec<RiskAlert>, StorageError> {
    use super::diesel_schema::risk_alerts::dsl::*;

    risk_alerts
        .filter(app_id.eq(ctx.app_id()))
        .filter(place_id.eq_any(place_ids))
        .filter(status.eq("active"))
        .order(place_id.asc())
        .load::<RiskAlert>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to batch-load alerts: {}", e)))
}
```

**Step 3: Cargo check + commit**

---

### Task 5: Angular Dashboard API Service

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/spatial-dashboard-api.service.ts`

**Step 1: Write the service**

```typescript
import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';

import type { SpatialDashboardView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class SpatialDashboardApiService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  getDashboard(query?: {
    constitutionalLayer?: string;
    h3Index?: string;
    h3Resolution?: number;
    parentPlaceId?: string;
    status?: string;
    include?: string;
    limit?: number;
  }): Observable<SpatialDashboardView> {
    let params = new HttpParams();
    if (query?.constitutionalLayer)
      params = params.set('constitutionalLayer', query.constitutionalLayer);
    if (query?.h3Index) params = params.set('h3Index', query.h3Index);
    if (query?.h3Resolution)
      params = params.set('h3Resolution', query.h3Resolution.toString());
    if (query?.parentPlaceId) params = params.set('parentPlaceId', query.parentPlaceId);
    if (query?.status) params = params.set('status', query.status);
    if (query?.include) params = params.set('include', query.include);
    if (query?.limit) params = params.set('limit', query.limit.toString());
    return this.http.get<SpatialDashboardView>(
      `${this.baseUrl}/api/v1/dashboard/spatial`,
      { params },
    );
  }
}
```

**Step 2: Commit**

---

### Task 6: Enhance SpatialMapComponent — Dashboard Mode

**Files:**
- Modify: `app/elohim-app/src/app/elohim/components/spatial-map/spatial-map.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/spatial-map/spatial-map.component.css`

**Step 1: Add dashboard data loading**

In the component:
- Inject `SpatialDashboardApiService`
- Add `dashboardMode = signal(false)` toggle
- Add `dashboardData = signal<SpatialDashboardView | null>(null)`
- Add `selectedLayer = signal<string>('all')` for constitutional layer filter
- Add `includeFlags = signal<string>('hazards,vulnerability,weather,capacity,alerts')` for include toggles
- When `dashboardMode` is true, call `getDashboard()` instead of `PlacesApiService.list()`
- Computed signals: `filteredPlaces`, `summaryStats`

**Step 2: Add risk heatmap layer**

Replace the constitutional-layer color scheme with risk-tier coloring when dashboard mode is active:
- `low` → `#4CAF50` (green)
- `moderate` → `#FFC107` (amber)
- `elevated` → `#FF9800` (orange)
- `critical` → `#F44336` (red)
- No vulnerability data → `#9E9E9E` (gray)

Use MapLibre data-driven styling:
```javascript
'fill-color': ['match', ['get', 'riskTier'],
  'low', '#4CAF50',
  'moderate', '#FFC107',
  'elevated', '#FF9800',
  'critical', '#F44336',
  '#9E9E9E'
]
```

**Step 3: Add hazard marker layer**

Add a symbol layer for Places with active hazards:
- Warning triangle icon (⚠) with count
- Pulsing animation for `emergency` severity
- Different icon colors by hazard type

**Step 4: Add alert badge layer**

Circle markers showing unacknowledged alert count per Place. Red for critical, orange for warning.

**Step 5: Add weather overlay**

Small weather icon per Place based on risk_level: ☀️ (none), ☁️ (advisory), 🌧️ (warning), ⛈️ (severe).

**Step 6: Add dashboard sidebar panel**

Panel contents:
- Toggle button: "Dashboard Mode" on/off
- Constitutional layer dropdown selector
- Summary stats: total Places, risk distribution as colored bar chart
- Active hazards count, active alerts count, unacknowledged alerts
- Include checkboxes (capacity, hazards, vulnerability, weather, alerts)
- Place list sorted by risk tier (critical first), each row shows name + risk tier badge + hazard count + alert count
- Click a Place row → fly to it on the map + select it

**Step 7: Add styles for dashboard panel, legend, badges**

CSS for:
- `.dashboard-panel` — fixed sidebar, 320px wide, scrollable
- `.risk-badge` — colored pill (green/amber/orange/red)
- `.summary-bar` — horizontal stacked bar for risk distribution
- `.place-list-item` — compact row with name + badges
- `.hazard-icon` — pulsing animation for emergency
- `.alert-badge` — red circle with count
- `.dashboard-toggle` — button in top bar

**Step 8: Commit**

---

### Task 7: Final Verification

**Step 1:** `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`
Expected: compiles clean

**Step 2:** `cargo test export_bindings` — verify TS types generated for all new views

**Step 3:** Verify new `.ts` files in `elohim/sdk/storage-client-ts/src/generated/`:
- `SpatialDashboardView.ts`
- `PlaceDashboardEntry.ts`
- `DashboardSummaryView.ts`
- `RiskTierDistribution.ts`
- `CapacitySummaryView.ts`
- `HazardEntrySummaryView.ts`
- `VulnerabilitySummaryView.ts`
- `WeatherEntrySummaryView.ts`
- `AlertEntrySummaryView.ts`
- `RouteEntrySummaryView.ts`

**Step 4:** Update `elohim/sdk/storage-client-ts/src/generated/index.ts` to export new types

**Step 5:** Final commit
