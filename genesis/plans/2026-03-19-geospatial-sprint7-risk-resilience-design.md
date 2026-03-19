# Sprint 7: Risk + Resilience Mapping — Design

## Context

Sprints 1-6 built the spatial foundation: H3 hexagonal indexing, Places with carrying capacity, SpatialContext with history, constitutional coupling, map component, OSRM routing, distribution planning, weather risk layer. Sprint 7 adds the risk intelligence layer — how hazards are cataloged, how vulnerability is assessed, and how preparatory alerts enable governance response before impact.

This is the "Cybersyn risk layer" — real-time situational awareness for governed physical space, informed by underwriting principles: scheduled re-evaluation on cadence, temporal hazard horizons for preparatory mobilization.

## P2P Design Gate

All three new entities are **Category C (Operational)** — pure computation and projection, no DHT entry types needed. No pressure on Lamad (~73/~100) or Mishpat (~11/~100).

- **Hazard**: Ephemeral situational report, often from external APIs. Reconstructable from source data.
- **VulnerabilityAssessment**: Computed composite score, cached in-memory (not persisted).
- **RiskAlert**: Derived from threshold crossings on operational data. If escalated to governance, creates a Proposal via existing Mishpat entry type.

## Architecture: On-Demand Computation

Vulnerability scores and risk alerts are computed **when requested** via API calls. No background jobs for MVP. Weather + capacity data is already cached (1hr / instant). Schedules (avodah cadence) provide re-evaluation triggers.

All routes live in **elohim-storage** (`/api/v1/*`). Nothing in doorway — doorway only proxies.

## Deliverables

### 7a. Hazard Entity + Migration

**New table**: `hazards` (Category C — operational, reconstructable)

```sql
-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- No dht_anchor_hash — this is ephemeral situational data, not notarized.
-- Reconstruction: re-fetch from external APIs + re-scan economic events.
CREATE TABLE hazards (
    id                TEXT PRIMARY KEY,  -- UUID, no DHT anchor (Category C operational)
    app_id            TEXT NOT NULL,
    place_id          TEXT NOT NULL,
    hazard_type       TEXT NOT NULL,  -- flood, drought, wildfire, storm, earthquake,
                                     -- supply-disruption, infrastructure-failure, epidemic, custom
    severity          TEXT NOT NULL,  -- watch, advisory, warning, emergency
    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',

    -- Temporal horizon (underwriter renewal + hurricane prep)
    reported_at       TEXT NOT NULL,
    projected_onset   TEXT,           -- when impact begins (null = immediate)
    projected_end     TEXT,           -- when impact expected to clear (null = unknown)
    actual_onset      TEXT,           -- filled when it actually starts
    resolved_at       TEXT,           -- filled when cleared

    -- Spatial scope
    affected_h3_cells TEXT NOT NULL DEFAULT '[]',  -- JSON array of H3 indices
    radius_km         REAL,

    -- Source + provenance
    source            TEXT NOT NULL,  -- weather-api, manual-report, capacity-breach,
                                     -- distribution-gap, external
    source_reference  TEXT,           -- external API ID, reporter human_id, etc.
    metadata_json     TEXT NOT NULL DEFAULT '{}',

    status            TEXT NOT NULL DEFAULT 'active',  -- active, monitoring, resolved, expired
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);
-- Source of truth: SQLite (operational). Reconstruction: re-fetch from external APIs + economic events.
```

**Hazard types** (enum, extensible via `custom`):
- **Natural**: flood, drought, wildfire, storm, earthquake
- **Systemic**: supply-disruption, infrastructure-failure, epidemic
- **Custom**: community-defined

**Auto-detection**: Weather service can auto-create hazards when `WeatherRisk::Severe` or `WeatherRisk::Warning` is detected for a Place.

### 7b. Vulnerability Assessment Service

**Not a table** — computed on demand, cached in DashMap (15-minute TTL).

```rust
pub struct VulnerabilityAssessmentView {
    pub place_id: String,
    pub assessed_at: String,

    // Component scores (0.0 to 1.0, higher = more vulnerable)
    pub hazard_exposure: f64,
    pub capacity_stress: f64,
    pub supply_chain_risk: f64,
    pub weather_risk: f64,
    pub environmental_fragility: f64,

    // Composite
    pub overall_vulnerability: f64,
    pub risk_tier: RiskTier,  // low, moderate, elevated, critical

    // Preparedness window
    pub nearest_hazard_onset_hours: Option<f64>,
    pub preparation_status: PreparationStatus,  // clear, monitoring, preparing, responding, recovering

    // Active hazard summary
    pub active_hazard_count: u32,
    pub active_hazards: Vec<HazardSummary>,

    // Schedule integration (avodah cadence)
    pub review_schedule: Option<ReviewScheduleView>,
}
```

**Weight formula**:
```
overall = 0.30 * hazard_exposure
        + 0.25 * capacity_stress
        + 0.20 * supply_chain_risk
        + 0.15 * weather_risk
        + 0.10 * environmental_fragility
```

**Inputs**:
- Active hazards for this Place → hazard_exposure
- Carrying capacity utilization (spatial_capacity.rs) → capacity_stress
- Weather forecast (weather.rs) → weather_risk
- Place characteristics (place_type, carrying_capacity_json) → environmental_fragility
- Supply chain risk is derived from resource nature + active hazards on supply routes

**Review schedule**: Checks existing Schedule table for `entity_type='place', schedule_type='risk-review'`. Returns last review time, next due, whether overdue.

### 7c. Risk Alert Entity + Migration

**New table**: `risk_alerts` (Category C — operational)

```sql
-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- No dht_anchor_hash — derived from threshold crossings on operational data.
-- Reconstruction: re-evaluate thresholds against current hazards + vulnerability.
CREATE TABLE risk_alerts (
    id                TEXT PRIMARY KEY,  -- UUID, no DHT anchor (Category C operational)
    app_id            TEXT NOT NULL,
    place_id          TEXT NOT NULL,
    alert_type        TEXT NOT NULL,  -- capacity-threshold, hazard-approaching,
                                     -- supply-disruption, weather-severe,
                                     -- multi-hazard-overlap, vulnerability-critical
    severity          TEXT NOT NULL,  -- info, warning, critical, emergency
    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',

    -- Trigger context
    trigger_hazard_id TEXT,           -- FK → hazards (if hazard-triggered)
    trigger_data_json TEXT NOT NULL DEFAULT '{}',  -- vulnerability snapshot at trigger time

    -- Temporal
    triggered_at      TEXT NOT NULL,
    lead_time_hours   REAL,           -- hours before projected impact
    expires_at        TEXT,           -- auto-expire if not acknowledged

    -- Lifecycle
    status            TEXT NOT NULL DEFAULT 'active',  -- active, acknowledged, escalated, resolved, expired
    acknowledged_by   TEXT,           -- human_id
    acknowledged_at   TEXT,
    resolved_at       TEXT,
    escalated_to      TEXT,           -- proposal_id if escalated to governance

    metadata_json     TEXT NOT NULL DEFAULT '{}',
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);
-- Source of truth: SQLite (operational). Reconstruction: re-evaluate thresholds against current data.
```

**Alert generation** (side-effect of vulnerability computation):
- `capacity_stress > 0.9` → capacity-threshold alert
- Hazard with `projected_onset` within 72h → hazard-approaching alert (lead_time_hours populated)
- `overall_vulnerability > 0.8` → vulnerability-critical alert
- Multiple active hazards on same Place → multi-hazard-overlap alert
- `WeatherRisk::Severe` → weather-severe alert

**Deduplication**: One active alert per (place_id, alert_type, trigger_hazard_id). Upsert pattern.

**Governance bridge**: `status = 'escalated'` + `escalated_to = proposal_id`. Sprint 7 provides the trigger data — existing governance API creates the Proposal.

### 7d. Schedule Integration (Avodah Cadence)

No new tables or schemas. Convention for existing Schedule (source of truth: existing Schedule table):
- `entity_type = 'place'`, `entity_id = {place_id}`, `schedule_type = 'risk-review'`
- RRULE defines cadence (weekly, monthly, quarterly — governance decides)
- Vulnerability endpoint checks if review is overdue and includes schedule info in response

### 7e. Angular Services (thin HTTP clients)

**`hazard-api.service.ts`**: create, list (by place/status), getById, update status
**`risk-api.service.ts`**: getVulnerability(placeId), listAlerts(placeId, status), updateAlert(id, patch)

## HTTP Routes (all in elohim-storage, Category C operational — no DHT entry types)

All routes serve operational Category C projections. No coordinator zome functions —
these entities are SQLite-only with documented reconstruction strategies.

| Method | Path | Category C Table | Notes |
|--------|------|-----------------|-------|
| POST | `/api/v1/hazards` | hazards | Report hazard |
| GET | `/api/v1/hazards?placeId=&status=` | hazards | List hazards |
| GET | `/api/v1/hazards/{id}` | hazards | Get hazard |
| PATCH | `/api/v1/hazards/{id}` | hazards | Update status/severity |
| GET | `/api/v1/risk/vulnerability/{placeId}` | (computed, not stored) | On-demand scoring |
| GET | `/api/v1/risk/alerts?placeId=&status=` | risk_alerts | List alerts |
| PATCH | `/api/v1/risk/alerts/{id}` | risk_alerts | Acknowledge/resolve/escalate |

## Files to Create/Modify

### New Rust files
| File | Purpose |
|------|---------|
| `services/hazard.rs` | Hazard CRUD + auto-detection from weather |
| `services/vulnerability.rs` | Vulnerability scoring engine + DashMap cache |
| `services/risk_alert.rs` | Alert generation, dedup, lifecycle |
| `api/hazards.rs` | HTTP controller for hazards CRUD |
| `api/risk.rs` | HTTP controller for vulnerability + alerts |
| `db/hazards.rs` | Diesel CRUD for hazards table |
| `db/risk_alerts.rs` | Diesel CRUD for risk_alerts table |
| `migrations/2026-03-19-400000_hazards/up.sql` | hazards table |
| `migrations/2026-03-19-400000_hazards/down.sql` | drop hazards |
| `migrations/2026-03-19-500000_risk_alerts/up.sql` | risk_alerts table |
| `migrations/2026-03-19-500000_risk_alerts/down.sql` | drop risk_alerts |

### New Angular files
| File | Purpose |
|------|---------|
| `services/hazard-api.service.ts` | Hazard HTTP client |
| `services/risk-api.service.ts` | Vulnerability + alerts HTTP client |

### Modified Rust files
| File | Change |
|------|--------|
| `services/mod.rs` | Add hazard, vulnerability, risk_alert modules |
| `api/mod.rs` | Add hazards, risk dispatchers |
| `db/mod.rs` | Add hazards, risk_alerts modules |
| `db/diesel_schema.rs` | Add hazards, risk_alerts table! macros |
| `db/models.rs` | Add Hazard, NewHazard, RiskAlert, NewRiskAlert structs |

## Verification

1. `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check` — compiles
2. `cargo test export_bindings` — TS types generated
3. Manual: `POST /api/v1/hazards` with flood hazard → created
4. Manual: `GET /api/v1/risk/vulnerability/{placeId}` → returns composite score
5. Manual: Vulnerability >0.8 → risk alert auto-generated
6. Manual: `PATCH /api/v1/risk/alerts/{id}` acknowledge → status updated
7. Hazard with `projected_onset` 48h out → alert with lead_time_hours populated

## Notes for Fresh Session

- All routes in elohim-storage, nothing in doorway. Doorway only proxies.
- Vulnerability assessment is NOT persisted — computed on demand, cached 15min in DashMap.
- Hazard auto-detection from weather: when weather service returns Severe/Warning, auto-create hazard if none exists for that Place + weather event.
- Schedule integration uses existing Schedule table with `schedule_type = 'risk-review'` convention.
- Alert dedup: one active alert per (place_id, alert_type, trigger_hazard_id). Upsert, don't spam.
- Governance escalation: alert carries trigger data, existing Proposal API does the rest.
