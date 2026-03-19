# Sprint 8: Planet-Scale Governance Dashboard — Design

## Context

Sprints 1-7 built the complete spatial stack: H3 hexagonal indexing, Places with carrying capacity, SpatialContexts, constitutional coupling, map component, OSRM routing, distribution planning, weather risk, hazard catalog, vulnerability assessment, risk alerts. Sprint 8 composes everything into a single governance dashboard — the Cybersyn control room, scoped by the viewer's constitutional layer.

## Key Design Decision: Backend Aggregation

All composition logic lives in **elohim-storage** (not doorway, not Angular). A single aggregate endpoint returns the complete dashboard payload. The frontend is a thin rendering layer.

**Why:** Business logic belongs in the backend. Frontend composition creates chatty APIs, duplicates logic, and makes the system harder to reason about. One request, one response, backend does the work.

## P2P Design Gate

**No new entities.** This sprint is pure read-only composition of existing data. No new tables, no new migrations, no DHT pressure. The aggregate endpoint is a service-layer orchestration pattern.

## Deliverables

### 8a. Aggregate Dashboard Endpoint

```
GET /api/v1/dashboard/spatial
    ?constitutionalLayer=bioregional
    &h3Index=852a100bfffffff
    &h3Resolution=5
    &parentPlaceId=...
    &status=active
    &include=hazards,vulnerability,weather,capacity,alerts,routes
    &limit=100
```

**Scoping** — the dashboard adapts to governance level:
- Family steward: `?constitutionalLayer=family` or `?parentPlaceId={their-place}`
- Community council: `?constitutionalLayer=community`
- Bioregional council: `?constitutionalLayer=bioregional`
- Global layer: no scope (all Places)

**Include flags** — client specifies which enrichments to load:
- `capacity` — carrying capacity summary per Place
- `hazards` — active hazard count + worst severity + nearest onset
- `vulnerability` — risk tier + overall score + preparation status
- `weather` — current weather risk level
- `alerts` — active alert count + worst severity
- `routes` — active distribution routes touching this Place
- Default (omit `include` param): all enrichments

### 8b. Response Types

```rust
// Top-level response
SpatialDashboardView {
    places: Vec<PlaceDashboardEntry>,
    summary: DashboardSummaryView,
    queried_at: String,
}

// Per-place entry with optional enrichments
PlaceDashboardEntry {
    // Core (always)
    id, name, place_type, constitutional_layer,
    h3_index, centroid_lat, centroid_lng,
    geometry_json, status, parent_place_id,

    // Optional (null if not in include)
    capacity: Option<CapacitySummary>,
    hazards: Option<HazardEntrySummary>,
    vulnerability: Option<VulnerabilitySummary>,
    weather: Option<WeatherEntrySummary>,
    alerts: Option<AlertEntrySummary>,
    routes: Option<RouteEntrySummary>,
}

// Summaries
CapacitySummary {
    resource_count, worst_utilization, worst_category, trigger_governance
}

HazardEntrySummary {
    active_count, worst_severity, nearest_onset_hours, types: Vec<String>
}

VulnerabilitySummary {
    overall_score, risk_tier, preparation_status
}

WeatherEntrySummary {
    risk_level, temperature_c, precipitation_mm, wind_speed_kmh
}

AlertEntrySummary {
    active_count, worst_severity, unacknowledged_count, types: Vec<String>
}

RouteEntrySummary {
    active_route_count, total_distance_km
}

// Aggregate stats
DashboardSummaryView {
    total_places,
    places_by_risk_tier: { low, moderate, elevated, critical },
    total_active_hazards, total_active_alerts,
    total_unacknowledged_alerts, worst_overall_risk
}
```

### 8c. Backend Service Architecture

```
api/dashboard.rs (controller)
       ↓ parse query, parse include flags
services/spatial_dashboard.rs (orchestrator)
       ├── db::places::list_places()              — always (filtered by query)
       ├── db::hazards batch query                 — if include=hazards
       ├── vulnerability::compute_vulnerability()   — if include=vulnerability
       ├── weather::get_forecast()                  — if include=weather
       ├── db::risk_alerts batch query              — if include=alerts
       ├── capacity from Place.carrying_capacity_json — if include=capacity
       └── aggregate into SpatialDashboardView
```

**Batch queries**: Hazards and alerts are loaded with `WHERE place_id IN (...)` — one query per enrichment, not N+1. Capacity is parsed from the Place record (already loaded). Vulnerability and weather hit their DashMap caches (15min / 1hr TTL).

### 8d. Angular Dashboard Enhancement

Enhance existing `SpatialMapComponent` (already at `/map` route):

**New map layers** (togglable):
- Risk heatmap: Place fill color by risk tier (green/yellow/orange/red)
- Hazard markers: Warning icons, pulsing for emergency
- Alert badges: Unacknowledged count on Places
- Weather overlay: Small weather icon per Place

**Dashboard panel** (sidebar):
- Summary stats bar (total Places, risk distribution, hazard/alert counts)
- Constitutional layer selector (scopes the view)
- Include toggles (which enrichments to show)
- Risk tier filter
- Place list sorted by risk (critical first), clickable to fly-to

**New Angular service**: `spatial-dashboard-api.service.ts`

### 8e. Angular Service

```typescript
@Injectable({ providedIn: 'root' })
export class SpatialDashboardApiService {
  getDashboard(query?: {
    constitutionalLayer?: string;
    h3Index?: string;
    h3Resolution?: number;
    parentPlaceId?: string;
    status?: string;
    include?: string;   // comma-separated: 'hazards,vulnerability,weather,...'
    limit?: number;
  }): Observable<SpatialDashboardView>
}
```

## Files to Create/Modify

### New Rust files
| File | Purpose |
|------|---------|
| `services/spatial_dashboard.rs` | Orchestrator — batch queries, compose aggregate view |
| `api/dashboard.rs` | HTTP controller for `/api/v1/dashboard/spatial` |

### Modified Rust files
| File | Change |
|------|--------|
| `services/mod.rs` | Add `pub mod spatial_dashboard;` |
| `api/mod.rs` | Add `pub mod dashboard;` + dispatch |
| `views.rs` | Add SpatialDashboardView and all summary types |

### New Angular files
| File | Purpose |
|------|---------|
| `services/spatial-dashboard-api.service.ts` | HTTP client for aggregate endpoint |

### Modified Angular files
| File | Change |
|------|--------|
| `spatial-map.component.ts` | Dashboard mode with risk layers, panel, filters |
| `spatial-map.component.css` | Dashboard panel styles, legends, badges |

### No new migrations
Pure read-only composition.

## Verification

1. `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check` — compiles
2. `cargo test export_bindings` — TS types generated
3. Manual: `GET /api/v1/dashboard/spatial` → returns all Places with all enrichments
4. Manual: `GET /api/v1/dashboard/spatial?include=hazards,alerts` → only hazard/alert fields populated
5. Manual: `GET /api/v1/dashboard/spatial?constitutionalLayer=bioregional` → scoped to bioregional Places
6. Map: Places colored by risk tier, hazard icons visible, dashboard panel shows summary

## Notes for Fresh Session

- All backend logic in elohim-storage. Nothing in doorway.
- No new tables or migrations — pure composition of existing data.
- Include flags default to all enrichments if omitted. Empty `include=` returns bare Places.
- Batch queries for hazards/alerts (WHERE place_id IN). Vulnerability/weather hit caches.
- Existing SpatialMapComponent at `/map` route. Enhance, don't replace.
- Dashboard scopes by constitutional layer — the same endpoint serves family, community, bioregional, and global views.
