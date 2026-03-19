# Sprint 7: Risk + Resilience Mapping — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add hazard catalog, vulnerability assessment, and risk alerts to the spatial layer — the Cybersyn risk intelligence system.

**Architecture:** Three new Category C (operational) entities. Hazards are reported/auto-detected and stored in SQLite. Vulnerability is computed on-demand from hazards + weather + capacity + distribution data, cached in DashMap (15min TTL). Risk alerts are generated as side-effects of vulnerability computation when thresholds are crossed. All routes in elohim-storage, nothing in doorway.

**Tech Stack:** Rust (diesel, dashmap, chrono, serde, ts-rs, reqwest), Angular (thin HTTP clients), SQLite migrations.

**Design doc:** `genesis/plans/2026-03-19-geospatial-sprint7-risk-resilience-design.md`

---

### Task 1: Hazards Migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-19-400000_hazards/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-19-400000_hazards/down.sql`

**Step 1: Write up.sql**

```sql
-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- No dht_anchor_hash — ephemeral situational data, not notarized.
-- Reconstruction: re-fetch from external APIs + re-scan economic events.
CREATE TABLE hazards (
    id                TEXT PRIMARY KEY NOT NULL,
    app_id            TEXT NOT NULL DEFAULT 'lamad',
    place_id          TEXT NOT NULL,
    hazard_type       TEXT NOT NULL,
    severity          TEXT NOT NULL,
    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    reported_at       TEXT NOT NULL,
    projected_onset   TEXT,
    projected_end     TEXT,
    actual_onset      TEXT,
    resolved_at       TEXT,
    affected_h3_cells TEXT NOT NULL DEFAULT '[]',
    radius_km         REAL,
    source            TEXT NOT NULL,
    source_reference  TEXT,
    metadata_json     TEXT NOT NULL DEFAULT '{}',
    status            TEXT NOT NULL DEFAULT 'active',
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE INDEX idx_hazards_place ON hazards (place_id, app_id);
CREATE INDEX idx_hazards_status ON hazards (status, app_id);
CREATE INDEX idx_hazards_type ON hazards (hazard_type, app_id);
CREATE INDEX idx_hazards_onset ON hazards (projected_onset);
```

**Step 2: Write down.sql**

```sql
DROP TABLE IF EXISTS hazards;
```

**Step 3: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-03-19-400000_hazards/
git commit -m "feat(spatial): add hazards migration (Category C operational)"
```

---

### Task 2: Risk Alerts Migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-19-500000_risk_alerts/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-19-500000_risk_alerts/down.sql`

**Step 1: Write up.sql**

```sql
-- Source of truth: SQLite (operational, Category C). No DHT entry type.
-- No dht_anchor_hash — derived from threshold crossings.
-- Reconstruction: re-evaluate thresholds against current data.
CREATE TABLE risk_alerts (
    id                TEXT PRIMARY KEY NOT NULL,
    app_id            TEXT NOT NULL DEFAULT 'lamad',
    place_id          TEXT NOT NULL,
    alert_type        TEXT NOT NULL,
    severity          TEXT NOT NULL,
    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    trigger_hazard_id TEXT,
    trigger_data_json TEXT NOT NULL DEFAULT '{}',
    triggered_at      TEXT NOT NULL,
    lead_time_hours   REAL,
    expires_at        TEXT,
    status            TEXT NOT NULL DEFAULT 'active',
    acknowledged_by   TEXT,
    acknowledged_at   TEXT,
    resolved_at       TEXT,
    escalated_to      TEXT,
    metadata_json     TEXT NOT NULL DEFAULT '{}',
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE INDEX idx_risk_alerts_place ON risk_alerts (place_id, app_id);
CREATE INDEX idx_risk_alerts_status ON risk_alerts (status, app_id);
CREATE INDEX idx_risk_alerts_type ON risk_alerts (alert_type, app_id);
CREATE INDEX idx_risk_alerts_dedup ON risk_alerts (place_id, alert_type, trigger_hazard_id)
    WHERE status = 'active';
```

**Step 2: Write down.sql**

```sql
DROP TABLE IF EXISTS risk_alerts;
```

**Step 3: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-03-19-500000_risk_alerts/
git commit -m "feat(spatial): add risk_alerts migration (Category C operational)"
```

---

### Task 3: Diesel Schema + Models

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (append before `diesel::joinable!` block, ~line 940; add to `allow_tables_to_appear_in_same_query!` list)
- Modify: `elohim/elohim-storage/src/db/models.rs` (append at end)

**Step 1: Add `hazards` and `risk_alerts` table! macros to `diesel_schema.rs`**

Insert before the `diesel::joinable!` block (~line 940):

```rust
diesel::table! {
    hazards (id) {
        id -> Text,
        app_id -> Text,
        place_id -> Text,
        hazard_type -> Text,
        severity -> Text,
        title -> Text,
        description -> Text,
        reported_at -> Text,
        projected_onset -> Nullable<Text>,
        projected_end -> Nullable<Text>,
        actual_onset -> Nullable<Text>,
        resolved_at -> Nullable<Text>,
        affected_h3_cells -> Text,
        radius_km -> Nullable<Double>,
        source -> Text,
        source_reference -> Nullable<Text>,
        metadata_json -> Text,
        status -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    risk_alerts (id) {
        id -> Text,
        app_id -> Text,
        place_id -> Text,
        alert_type -> Text,
        severity -> Text,
        title -> Text,
        description -> Text,
        trigger_hazard_id -> Nullable<Text>,
        trigger_data_json -> Text,
        triggered_at -> Text,
        lead_time_hours -> Nullable<Double>,
        expires_at -> Nullable<Text>,
        status -> Text,
        acknowledged_by -> Nullable<Text>,
        acknowledged_at -> Nullable<Text>,
        resolved_at -> Nullable<Text>,
        escalated_to -> Nullable<Text>,
        metadata_json -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

Add `hazards` and `risk_alerts` to `allow_tables_to_appear_in_same_query!` (alphabetical order).

**Step 2: Add Hazard/NewHazard and RiskAlert/NewRiskAlert structs to `models.rs`**

Append at end of file:

```rust
// ============================================================================
// Hazard — Category C operational (Sprint 7: Risk + Resilience)
// ============================================================================

/// Hazard record (SELECT)
#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = hazards)]
pub struct Hazard {
    pub id: String,
    pub app_id: String,
    pub place_id: String,
    pub hazard_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub reported_at: String,
    pub projected_onset: Option<String>,
    pub projected_end: Option<String>,
    pub actual_onset: Option<String>,
    pub resolved_at: Option<String>,
    pub affected_h3_cells: String,
    pub radius_km: Option<f64>,
    pub source: String,
    pub source_reference: Option<String>,
    pub metadata_json: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// New hazard for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = hazards)]
pub struct NewHazard {
    pub id: String,
    pub app_id: String,
    pub place_id: String,
    pub hazard_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub reported_at: String,
    pub projected_onset: Option<String>,
    pub projected_end: Option<String>,
    pub actual_onset: Option<String>,
    pub resolved_at: Option<String>,
    pub affected_h3_cells: String,
    pub radius_km: Option<f64>,
    pub source: String,
    pub source_reference: Option<String>,
    pub metadata_json: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// RiskAlert — Category C operational (Sprint 7: Risk + Resilience)
// ============================================================================

/// Risk alert record (SELECT)
#[derive(Debug, Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = risk_alerts)]
pub struct RiskAlert {
    pub id: String,
    pub app_id: String,
    pub place_id: String,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub trigger_hazard_id: Option<String>,
    pub trigger_data_json: String,
    pub triggered_at: String,
    pub lead_time_hours: Option<f64>,
    pub expires_at: Option<String>,
    pub status: String,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<String>,
    pub resolved_at: Option<String>,
    pub escalated_to: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// New risk alert for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = risk_alerts)]
pub struct NewRiskAlert {
    pub id: String,
    pub app_id: String,
    pub place_id: String,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub trigger_hazard_id: Option<String>,
    pub trigger_data_json: String,
    pub triggered_at: String,
    pub lead_time_hours: Option<f64>,
    pub expires_at: Option<String>,
    pub status: String,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<String>,
    pub resolved_at: Option<String>,
    pub escalated_to: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}
```

**Step 3: Run cargo check**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`
Expected: compiles (may warn about unused structs — that's fine, they're used in next tasks)

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/db/diesel_schema.rs elohim/elohim-storage/src/db/models.rs
git commit -m "feat(spatial): add Hazard and RiskAlert diesel schema + models"
```

---

### Task 4: Hazards DB Module

**Files:**
- Create: `elohim/elohim-storage/src/db/hazards.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` (add `pub mod hazards;`)

**Step 1: Write `db/hazards.rs`**

Follow the pattern from `db/places.rs`:
- `HazardQuery` struct (deserializable from URL query params): `place_id`, `status`, `hazard_type`, `limit`
- `create_hazard(conn, ctx, NewHazard) -> Result<Hazard>`
- `get_hazard_by_id(conn, ctx, id) -> Result<Hazard>`
- `list_hazards(conn, ctx, &HazardQuery) -> Result<Vec<Hazard>>`
- `update_hazard_status(conn, ctx, id, status, resolved_at) -> Result<Hazard>`
- `update_hazard(conn, ctx, id, patch fields) -> Result<Hazard>`
- `list_active_hazards_for_place(conn, ctx, place_id) -> Result<Vec<Hazard>>` (convenience: status=active + monitoring)

**Step 2: Add `pub mod hazards;` to `db/mod.rs`**

**Step 3: Cargo check**

**Step 4: Commit**

---

### Task 5: Risk Alerts DB Module

**Files:**
- Create: `elohim/elohim-storage/src/db/risk_alerts.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` (add `pub mod risk_alerts;`)

**Step 1: Write `db/risk_alerts.rs`**

- `RiskAlertQuery` struct: `place_id`, `status`, `alert_type`, `limit`
- `upsert_risk_alert(conn, ctx, NewRiskAlert) -> Result<RiskAlert>` (dedup: ON CONFLICT on partial index place_id+alert_type+trigger_hazard_id WHERE status='active')
- `get_risk_alert_by_id(conn, ctx, id) -> Result<RiskAlert>`
- `list_risk_alerts(conn, ctx, &RiskAlertQuery) -> Result<Vec<RiskAlert>>`
- `update_risk_alert_status(conn, ctx, id, status, acknowledged_by, escalated_to) -> Result<RiskAlert>`

**Step 2: Add `pub mod risk_alerts;` to `db/mod.rs`**

**Step 3: Cargo check**

**Step 4: Commit**

---

### Task 6: View Types (Hazard + RiskAlert + Vulnerability)

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs` (append HazardView, CreateHazardInputView, RiskAlertView, UpdateRiskAlertInputView)
- Create: `elohim/elohim-storage/src/services/vulnerability.rs` (contains VulnerabilityAssessmentView, RiskTier, PreparationStatus, ReviewScheduleView, HazardSummary — these are the ts-rs-exported types)

**Step 1: Add HazardView + CreateHazardInputView to views.rs**

```rust
// ============================================================================
// Hazard Views (Sprint 7: Risk + Resilience)
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HazardView {
    pub id: String,
    pub place_id: String,
    pub hazard_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub reported_at: String,
    pub projected_onset: Option<String>,
    pub projected_end: Option<String>,
    pub actual_onset: Option<String>,
    pub resolved_at: Option<String>,
    pub affected_h3_cells: JsonVal,
    pub radius_km: Option<f64>,
    pub source: String,
    pub source_reference: Option<String>,
    pub metadata: Option<JsonVal>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::db::models::Hazard> for HazardView {
    fn from(h: crate::db::models::Hazard) -> Self {
        Self {
            id: h.id,
            place_id: h.place_id,
            hazard_type: h.hazard_type,
            severity: h.severity,
            title: h.title,
            description: h.description,
            reported_at: h.reported_at,
            projected_onset: h.projected_onset,
            projected_end: h.projected_end,
            actual_onset: h.actual_onset,
            resolved_at: h.resolved_at,
            affected_h3_cells: parse_json(&h.affected_h3_cells),
            radius_km: h.radius_km,
            source: h.source,
            source_reference: h.source_reference,
            metadata: parse_json_opt(&Some(h.metadata_json)),
            status: h.status,
            created_at: h.created_at,
            updated_at: h.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateHazardInputView {
    pub place_id: String,
    pub hazard_type: String,
    pub severity: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub projected_onset: Option<String>,
    pub projected_end: Option<String>,
    pub affected_h3_cells: Option<Vec<String>>,
    pub radius_km: Option<f64>,
    #[serde(default = "default_hazard_source")]
    pub source: String,
    pub source_reference: Option<String>,
    pub metadata: Option<JsonVal>,
}

fn default_hazard_source() -> String { "manual-report".to_string() }
```

**Step 2: Add RiskAlertView + UpdateRiskAlertInputView to views.rs**

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RiskAlertView {
    pub id: String,
    pub place_id: String,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub trigger_hazard_id: Option<String>,
    pub trigger_data: Option<JsonVal>,
    pub triggered_at: String,
    pub lead_time_hours: Option<f64>,
    pub expires_at: Option<String>,
    pub status: String,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<String>,
    pub resolved_at: Option<String>,
    pub escalated_to: Option<String>,
    pub metadata: Option<JsonVal>,
    pub created_at: String,
    pub updated_at: String,
}

// impl From<RiskAlert> for RiskAlertView { ... } (same pattern as HazardView)

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateRiskAlertInputView {
    pub status: Option<String>,
    pub acknowledged_by: Option<String>,
    pub escalated_to: Option<String>,
}
```

**Step 3: Cargo check + commit**

---

### Task 7: Hazard Service

**Files:**
- Create: `elohim/elohim-storage/src/services/hazard.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod hazard;`)

Contains:
- `HazardType` enum (with ts-rs export): Flood, Drought, Wildfire, Storm, Earthquake, SupplyDisruption, InfrastructureFailure, Epidemic, Custom
- `HazardSeverity` enum: Watch, Advisory, Warning, Emergency
- `HazardSource` enum: WeatherApi, ManualReport, CapacityBreach, DistributionGap, External
- `auto_detect_weather_hazards(conn, ctx, place_id, forecast) -> Vec<NewHazard>` — when weather forecast shows Severe/Warning, create hazard if none exists for that Place

**Cargo check + commit**

---

### Task 8: Vulnerability Assessment Service

**Files:**
- Create: `elohim/elohim-storage/src/services/vulnerability.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod vulnerability;`)

Contains:
- `RiskTier` enum (ts-rs): Low, Moderate, Elevated, Critical
- `PreparationStatus` enum (ts-rs): Clear, Monitoring, Preparing, Responding, Recovering
- `HazardSummary` struct (ts-rs): id, hazard_type, severity, title, projected_onset, lead_time_hours
- `ReviewScheduleView` struct (ts-rs): last_reviewed_at, next_review_due, is_overdue, rrule
- `VulnerabilityAssessmentView` struct (ts-rs): all fields from design
- `VulnerabilityCache` — DashMap with 15-minute TTL (same pattern as `WeatherCache` in `weather.rs`)
- `compute_vulnerability(conn, ctx, place_id) -> VulnerabilityAssessmentView`
  - Queries active hazards for place (db/hazards.rs)
  - Gets carrying capacity utilization (spatial_capacity.rs)
  - Gets weather forecast (weather.rs — already cached)
  - Computes component scores
  - Applies weight formula
  - Derives risk_tier, preparation_status, nearest_hazard_onset_hours
  - Checks schedule table for risk-review schedule on this place
  - Returns assessment

**Side-effect: alert generation** — after computing vulnerability, call `risk_alert.rs` functions to upsert alerts when thresholds are crossed.

**Cargo check + commit**

---

### Task 9: Risk Alert Service

**Files:**
- Create: `elohim/elohim-storage/src/services/risk_alert.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (add `pub mod risk_alert;`)

Contains:
- `evaluate_and_generate_alerts(conn, ctx, place_id, assessment) -> Vec<RiskAlertView>`
  - Check each threshold:
    - `capacity_stress > 0.9` → capacity-threshold alert
    - Hazard with projected_onset within 72h → hazard-approaching (with lead_time_hours)
    - `overall_vulnerability > 0.8` → vulnerability-critical
    - Multiple active hazards → multi-hazard-overlap
    - Weather severe → weather-severe
  - Upsert (dedup by place_id + alert_type + trigger_hazard_id)
  - Return newly created/updated alerts

**Cargo check + commit**

---

### Task 10: Hazards API Controller

**Files:**
- Create: `elohim/elohim-storage/src/api/hazards.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs` (add `pub mod hazards;` + dispatch)

Follows `api/places.rs` pattern:
- `POST ""` → create hazard (parse CreateHazardInputView, generate UUID, insert)
- `GET ""` → list hazards (HazardQuery from URL params)
- `GET "{id}"` → get by ID
- `PATCH "{id}"` → update (status, severity, actual_onset, resolved_at)

Add dispatch in `api/mod.rs`:
```rust
} else if sub_path.starts_with("hazards") {
    let resource_path = sub_path.strip_prefix("hazards").unwrap_or("");
    hazards::handle(req, method, resource_path, &pool, &app_ctx).await
```

**Cargo check + commit**

---

### Task 11: Risk API Controller

**Files:**
- Create: `elohim/elohim-storage/src/api/risk.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs` (add `pub mod risk;` + dispatch)

- `GET "vulnerability/{placeId}"` → compute vulnerability assessment (calls vulnerability service, which triggers alert generation as side-effect)
- `GET "alerts"` → list alerts (RiskAlertQuery from URL params)
- `PATCH "alerts/{id}"` → update alert status (acknowledge/resolve/escalate)

Add dispatch in `api/mod.rs`:
```rust
} else if sub_path.starts_with("risk") {
    let resource_path = sub_path.strip_prefix("risk").unwrap_or("");
    risk::handle(req, method, resource_path, &pool, &app_ctx).await
```

**Cargo check + commit**

---

### Task 12: Angular Thin HTTP Clients

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/hazard-api.service.ts`
- Create: `app/elohim-app/src/app/elohim/services/risk-api.service.ts`

**`hazard-api.service.ts`** — follows `places-api.service.ts` pattern:
```typescript
import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable } from 'rxjs';
import type { HazardView, CreateHazardInputView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class HazardApiService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  create(input: CreateHazardInputView): Observable<HazardView> { ... }
  getById(id: string): Observable<HazardView> { ... }
  list(query?: { placeId?: string; status?: string; hazardType?: string; limit?: number }): Observable<HazardView[]> { ... }
  updateStatus(id: string, patch: { status: string; acknowledgedBy?: string }): Observable<HazardView> { ... }
}
```

**`risk-api.service.ts`**:
```typescript
import type { VulnerabilityAssessmentView, RiskAlertView, UpdateRiskAlertInputView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class RiskApiService {
  getVulnerability(placeId: string): Observable<VulnerabilityAssessmentView> { ... }
  listAlerts(query?: { placeId?: string; status?: string; alertType?: string }): Observable<RiskAlertView[]> { ... }
  updateAlert(id: string, patch: UpdateRiskAlertInputView): Observable<RiskAlertView> { ... }
}
```

**Commit**

---

### Task 13: Final Verification

**Step 1:** `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`
Expected: compiles clean

**Step 2:** `cargo test export_bindings` — verify TS types generated for all new views

**Step 3:** Verify new `.ts` files exist in `elohim/sdk/storage-client-ts/src/generated/`:
- `HazardView.ts`, `CreateHazardInputView.ts`
- `RiskAlertView.ts`, `UpdateRiskAlertInputView.ts`
- `VulnerabilityAssessmentView.ts`, `RiskTier.ts`, `PreparationStatus.ts`
- `HazardSummary.ts`, `ReviewScheduleView.ts`
- `HazardType.ts`, `HazardSeverity.ts`, `HazardSource.ts`

**Step 4:** Update `elohim/sdk/storage-client-ts/src/generated/index.ts` to export new types

**Step 5:** Final commit
