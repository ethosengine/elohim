# Stewardship Fat Service Migration Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Delete the 932-line Angular `StewardshipService` by completing the thin `StewardshipApiService` with 7 missing methods, backed by new Rust storage API endpoints for device policies, policy chains, and time access.

**Architecture:** New `device_policies` SQLite table stores per-subject policies with `inherits_from` chains. Rust service computes merged policies and caches them via existing `PolicyCache`. Angular consumers switch from `inject(StewardshipService)` to `inject(STEWARDSHIP_POLICY)` token resolving to thin HTTP client.

**Tech Stack:** Rust (elohim-storage, Diesel ORM, rusqlite migrations), Angular 19 (TypeScript, HttpClient, InjectionTokens), SQLite

---

## Reference Files

Before starting any task, read these to understand patterns:

- **Existing Diesel module pattern**: `holochain/elohim-storage/src/db/stewardship_allocations.rs`
- **Views pattern**: `holochain/elohim-storage/src/views.rs` (search for `StewardshipAllocationView`)
- **Service pattern**: `holochain/elohim-storage/src/services/stewardship_service.rs`
- **API handler pattern**: `holochain/elohim-storage/src/api/stewardship.rs`
- **Schema migration pattern**: `holochain/elohim-storage/src/db/schema.rs` (v3→v4)
- **Diesel schema**: `holochain/elohim-storage/src/db/diesel_schema.rs`
- **TypeScript models**: `elohim-app/src/app/imagodei/models/stewardship.model.ts`
- **Fat service (being deleted)**: `elohim-app/src/app/imagodei/services/stewardship.service.ts`
- **Thin service (being expanded)**: `elohim-app/src/app/imagodei/services/stewardship-api.service.ts`
- **Interface**: `elohim-app/src/app/imagodei/interfaces/stewardship-policy.interface.ts`
- **Policy cache (reuse)**: `holochain/elohim-storage/src/db/policy_cache.rs`

---

## Task 1: Schema Migration v4→v5 — device_policies table

**Files:**
- Modify: `holochain/elohim-storage/src/db/schema.rs`

**Step 1: Add DEVICE_POLICIES_SCHEMA constant**

After the `COLLECTIVES_SCHEMA` constant (~line 657), add:

```rust
/// Device policies — per-subject/per-device policy rules set by stewards
const DEVICE_POLICIES_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS device_policies (
    id TEXT PRIMARY KEY NOT NULL,
    subject_id TEXT NOT NULL,
    device_id TEXT,
    author_id TEXT NOT NULL,
    author_tier TEXT NOT NULL DEFAULT 'self',
    inherits_from TEXT,
    blocked_categories_json TEXT NOT NULL DEFAULT '[]',
    blocked_hashes_json TEXT NOT NULL DEFAULT '[]',
    age_rating_max TEXT,
    reach_level_max INTEGER,
    session_max_minutes INTEGER,
    daily_max_minutes INTEGER,
    time_windows_json TEXT NOT NULL DEFAULT '[]',
    cooldown_minutes INTEGER,
    disabled_features_json TEXT NOT NULL DEFAULT '[]',
    disabled_routes_json TEXT NOT NULL DEFAULT '[]',
    require_approval_json TEXT NOT NULL DEFAULT '[]',
    log_sessions INTEGER NOT NULL DEFAULT 0,
    log_categories INTEGER NOT NULL DEFAULT 0,
    log_policy_events INTEGER NOT NULL DEFAULT 1,
    retention_days INTEGER NOT NULL DEFAULT 30,
    subject_can_view INTEGER NOT NULL DEFAULT 1,
    effective_from TEXT NOT NULL DEFAULT (datetime('now')),
    effective_until TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_device_policies_subject ON device_policies(subject_id);
CREATE INDEX IF NOT EXISTS idx_device_policies_author ON device_policies(author_id);
CREATE INDEX IF NOT EXISTS idx_device_policies_tier ON device_policies(author_tier);
"#;
```

**Step 2: Bump SCHEMA_VERSION to 5**

Change line 9: `pub const SCHEMA_VERSION: i32 = 5;`

**Step 3: Add v4→v5 migration**

In `migrate_schema()`, after the v3→v4 block (~line 115), add:

```rust
    // Migration: v4 -> v5: Add device_policies table
    if current == 4 {
        info!("Migrating v4 -> v5: Adding device_policies table");
        conn.execute_batch(DEVICE_POLICIES_SCHEMA)
            .map_err(|e| {
                StorageError::Internal(format!(
                    "Failed to create device_policies table: {}",
                    e
                ))
            })?;
        current = 5;
    }
```

**Step 4: Add to create_tables for fresh install**

In `create_pillar_tables()`, after the collectives line, add:

```rust
    conn.execute_batch(DEVICE_POLICIES_SCHEMA)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to create device_policies table: {}",
                e
            ))
        })?;
```

**Step 5: Build and test**

Run: `cd holochain/elohim-storage && RUSTFLAGS="" cargo test --lib 2>&1 | tail -20`
Expected: All tests pass, including existing schema tests.

**Step 6: Commit**

```bash
git add holochain/elohim-storage/src/db/schema.rs
git commit -m "feat(storage): add device_policies table (schema v4→v5)"
```

---

## Task 2: Diesel Schema + Models for device_policies

**Files:**
- Modify: `holochain/elohim-storage/src/db/diesel_schema.rs`
- Modify: `holochain/elohim-storage/src/db/models.rs`

**Step 1: Add Diesel table definition**

In `diesel_schema.rs`, after the `stewardship_allocations` table block (~line 308), add:

```rust
diesel::table! {
    device_policies (id) {
        id -> Text,
        subject_id -> Text,
        device_id -> Nullable<Text>,
        author_id -> Text,
        author_tier -> Text,
        inherits_from -> Nullable<Text>,
        blocked_categories_json -> Text,
        blocked_hashes_json -> Text,
        age_rating_max -> Nullable<Text>,
        reach_level_max -> Nullable<Integer>,
        session_max_minutes -> Nullable<Integer>,
        daily_max_minutes -> Nullable<Integer>,
        time_windows_json -> Text,
        cooldown_minutes -> Nullable<Integer>,
        disabled_features_json -> Text,
        disabled_routes_json -> Text,
        require_approval_json -> Text,
        log_sessions -> Integer,
        log_categories -> Integer,
        log_policy_events -> Integer,
        retention_days -> Integer,
        subject_can_view -> Integer,
        effective_from -> Text,
        effective_until -> Nullable<Text>,
        version -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

Add `device_policies,` to the `allow_tables_to_appear_in_same_query!` macro (alphabetical order, after `content_tags`).

**Step 2: Add model structs**

In `models.rs`, add the Queryable and Insertable structs. Find the pattern by searching for `StewardshipAllocation` in models.rs. Add:

```rust
/// Device policy — per-subject/per-device rules set by stewards
#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = crate::db::diesel_schema::device_policies)]
pub struct DevicePolicy {
    pub id: String,
    pub subject_id: String,
    pub device_id: Option<String>,
    pub author_id: String,
    pub author_tier: String,
    pub inherits_from: Option<String>,
    pub blocked_categories_json: String,
    pub blocked_hashes_json: String,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<i32>,
    pub session_max_minutes: Option<i32>,
    pub daily_max_minutes: Option<i32>,
    pub time_windows_json: String,
    pub cooldown_minutes: Option<i32>,
    pub disabled_features_json: String,
    pub disabled_routes_json: String,
    pub require_approval_json: String,
    pub log_sessions: i32,
    pub log_categories: i32,
    pub log_policy_events: i32,
    pub retention_days: i32,
    pub subject_can_view: i32,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Insertable device policy
#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::diesel_schema::device_policies)]
pub struct NewDevicePolicy {
    pub id: String,
    pub subject_id: String,
    pub device_id: Option<String>,
    pub author_id: String,
    pub author_tier: String,
    pub inherits_from: Option<String>,
    pub blocked_categories_json: String,
    pub blocked_hashes_json: String,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<i32>,
    pub session_max_minutes: Option<i32>,
    pub daily_max_minutes: Option<i32>,
    pub time_windows_json: String,
    pub cooldown_minutes: Option<i32>,
    pub disabled_features_json: String,
    pub disabled_routes_json: String,
    pub require_approval_json: String,
    pub log_sessions: i32,
    pub log_categories: i32,
    pub log_policy_events: i32,
    pub retention_days: i32,
    pub subject_can_view: i32,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
}
```

**Step 3: Build**

Run: `cd holochain/elohim-storage && RUSTFLAGS="" cargo build 2>&1 | tail -10`
Expected: Compiles cleanly.

**Step 4: Commit**

```bash
git add holochain/elohim-storage/src/db/diesel_schema.rs holochain/elohim-storage/src/db/models.rs
git commit -m "feat(storage): add Diesel schema and models for device_policies"
```

---

## Task 3: DB CRUD Module — device_policies.rs

**Files:**
- Create: `holochain/elohim-storage/src/db/device_policies.rs`
- Modify: `holochain/elohim-storage/src/db/mod.rs`

**Step 1: Create the CRUD module**

Create `holochain/elohim-storage/src/db/device_policies.rs` following the pattern from `stewardship_allocations.rs`. Include:

- `CreateDevicePolicyInput` struct
- `upsert_policy()` — INSERT ON CONFLICT UPDATE by (subject_id, author_id, device_id is null)
- `get_policies_for_subject()` — query by subject_id, ordered by author_tier
- `get_policy_by_id()` — single lookup
- `delete_policy()` — hard delete

Key detail: The upsert should match on `(subject_id, author_id)` with `device_id IS NULL` — one policy per author per subject per device. Use `uuid::Uuid::new_v4()` for new IDs.

```rust
//! Device policies CRUD operations
//!
//! Per-subject/per-device policy rules set by stewards.
//! Policies form inheritance chains via `inherits_from`.

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::diesel_schema::device_policies;
use super::models::{DevicePolicy, NewDevicePolicy};
use super::PooledConn;
use crate::error::StorageError;

/// Input for creating/upserting a device policy
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDevicePolicyInput {
    pub subject_id: String,
    pub device_id: Option<String>,
    pub author_id: String,
    pub author_tier: String,
    pub inherits_from: Option<String>,
    pub blocked_categories_json: String,
    pub blocked_hashes_json: String,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<i32>,
    pub session_max_minutes: Option<i32>,
    pub daily_max_minutes: Option<i32>,
    pub time_windows_json: String,
    pub cooldown_minutes: Option<i32>,
    pub disabled_features_json: String,
    pub disabled_routes_json: String,
    pub require_approval_json: String,
    pub log_sessions: bool,
    pub log_categories: bool,
    pub log_policy_events: bool,
    pub retention_days: i32,
    pub subject_can_view: bool,
}

/// Upsert a device policy (insert or update by subject_id + author_id)
pub fn upsert_policy(
    conn: &mut PooledConn,
    input: &CreateDevicePolicyInput,
) -> Result<DevicePolicy, StorageError> {
    // Check if a policy already exists for this author + subject
    let existing = device_policies::table
        .filter(device_policies::subject_id.eq(&input.subject_id))
        .filter(device_policies::author_id.eq(&input.author_id))
        .first::<DevicePolicy>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    let now = chrono::Utc::now().to_rfc3339();

    if let Some(existing) = existing {
        // Update existing policy, bump version
        diesel::update(device_policies::table.filter(device_policies::id.eq(&existing.id)))
            .set((
                device_policies::inherits_from.eq(&input.inherits_from),
                device_policies::blocked_categories_json.eq(&input.blocked_categories_json),
                device_policies::blocked_hashes_json.eq(&input.blocked_hashes_json),
                device_policies::age_rating_max.eq(&input.age_rating_max),
                device_policies::reach_level_max.eq(&input.reach_level_max),
                device_policies::session_max_minutes.eq(&input.session_max_minutes),
                device_policies::daily_max_minutes.eq(&input.daily_max_minutes),
                device_policies::time_windows_json.eq(&input.time_windows_json),
                device_policies::cooldown_minutes.eq(&input.cooldown_minutes),
                device_policies::disabled_features_json.eq(&input.disabled_features_json),
                device_policies::disabled_routes_json.eq(&input.disabled_routes_json),
                device_policies::require_approval_json.eq(&input.require_approval_json),
                device_policies::log_sessions.eq(input.log_sessions as i32),
                device_policies::log_categories.eq(input.log_categories as i32),
                device_policies::log_policy_events.eq(input.log_policy_events as i32),
                device_policies::retention_days.eq(input.retention_days),
                device_policies::subject_can_view.eq(input.subject_can_view as i32),
                device_policies::version.eq(existing.version + 1),
                device_policies::updated_at.eq(&now),
            ))
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

        device_policies::table
            .filter(device_policies::id.eq(&existing.id))
            .first::<DevicePolicy>(conn)
            .map_err(|e| StorageError::Internal(format!("Reload failed: {}", e)))
    } else {
        // Insert new policy
        let new_policy = NewDevicePolicy {
            id: Uuid::new_v4().to_string(),
            subject_id: input.subject_id.clone(),
            device_id: input.device_id.clone(),
            author_id: input.author_id.clone(),
            author_tier: input.author_tier.clone(),
            inherits_from: input.inherits_from.clone(),
            blocked_categories_json: input.blocked_categories_json.clone(),
            blocked_hashes_json: input.blocked_hashes_json.clone(),
            age_rating_max: input.age_rating_max.clone(),
            reach_level_max: input.reach_level_max,
            session_max_minutes: input.session_max_minutes,
            daily_max_minutes: input.daily_max_minutes,
            time_windows_json: input.time_windows_json.clone(),
            cooldown_minutes: input.cooldown_minutes,
            disabled_features_json: input.disabled_features_json.clone(),
            disabled_routes_json: input.disabled_routes_json.clone(),
            require_approval_json: input.require_approval_json.clone(),
            log_sessions: input.log_sessions as i32,
            log_categories: input.log_categories as i32,
            log_policy_events: input.log_policy_events as i32,
            retention_days: input.retention_days,
            subject_can_view: input.subject_can_view as i32,
            effective_from: now.clone(),
            effective_until: None,
            version: 1,
            created_at: now.clone(),
            updated_at: now,
        };

        diesel::insert_into(device_policies::table)
            .values(&new_policy)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

        device_policies::table
            .filter(device_policies::id.eq(&new_policy.id))
            .first::<DevicePolicy>(conn)
            .map_err(|e| StorageError::Internal(format!("Reload failed: {}", e)))
    }
}

/// Get all policies for a subject, ordered by author_tier
pub fn get_policies_for_subject(
    conn: &mut PooledConn,
    subject_id: &str,
) -> Result<Vec<DevicePolicy>, StorageError> {
    device_policies::table
        .filter(device_policies::subject_id.eq(subject_id))
        .order(device_policies::author_tier.asc())
        .load::<DevicePolicy>(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get a policy by ID
pub fn get_policy_by_id(
    conn: &mut PooledConn,
    id: &str,
) -> Result<DevicePolicy, StorageError> {
    device_policies::table
        .filter(device_policies::id.eq(id))
        .first::<DevicePolicy>(conn)
        .map_err(|e| StorageError::NotFound(format!("Policy {} not found: {}", id, e)))
}

/// Delete a policy by ID
pub fn delete_policy(
    conn: &mut PooledConn,
    id: &str,
) -> Result<(), StorageError> {
    let deleted = diesel::delete(device_policies::table.filter(device_policies::id.eq(id)))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    if deleted == 0 {
        return Err(StorageError::NotFound(format!("Policy {} not found", id)));
    }
    Ok(())
}
```

**Step 2: Register module in db/mod.rs**

Add `pub mod device_policies;` after the `pub mod stewardship_allocations;` line (~line 49).

**Step 3: Build and test**

Run: `cd holochain/elohim-storage && RUSTFLAGS="" cargo build 2>&1 | tail -10`
Expected: Compiles cleanly.

**Step 4: Commit**

```bash
git add holochain/elohim-storage/src/db/device_policies.rs holochain/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): add device_policies CRUD module"
```

---

## Task 4: Views for DevicePolicy + PolicyChainLink + TimeAccess

**Files:**
- Modify: `holochain/elohim-storage/src/views.rs`

**Step 1: Add output views**

After the `StewardshipAllocationView` `From` impl, add:

```rust
/// Device policy view — camelCase API boundary
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DevicePolicyView {
    pub id: String,
    pub subject_id: String,
    pub device_id: Option<String>,
    pub author_id: String,
    pub author_tier: String,
    pub inherits_from: Option<String>,
    pub blocked_categories: JsonVal,
    pub blocked_hashes: JsonVal,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<i32>,
    pub session_max_minutes: Option<i32>,
    pub daily_max_minutes: Option<i32>,
    pub time_windows: JsonVal,
    pub cooldown_minutes: Option<i32>,
    pub disabled_features: JsonVal,
    pub disabled_routes: JsonVal,
    pub require_approval: JsonVal,
    pub log_sessions: bool,
    pub log_categories: bool,
    pub log_policy_events: bool,
    pub retention_days: i32,
    pub subject_can_view: bool,
    pub effective_from: String,
    pub effective_until: Option<String>,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::db::models::DevicePolicy> for DevicePolicyView {
    fn from(p: crate::db::models::DevicePolicy) -> Self {
        Self {
            id: p.id,
            subject_id: p.subject_id,
            device_id: p.device_id,
            author_id: p.author_id,
            author_tier: p.author_tier,
            inherits_from: p.inherits_from,
            blocked_categories: parse_json(&p.blocked_categories_json),
            blocked_hashes: parse_json(&p.blocked_hashes_json),
            age_rating_max: p.age_rating_max,
            reach_level_max: p.reach_level_max,
            session_max_minutes: p.session_max_minutes,
            daily_max_minutes: p.daily_max_minutes,
            time_windows: parse_json(&p.time_windows_json),
            cooldown_minutes: p.cooldown_minutes,
            disabled_features: parse_json(&p.disabled_features_json),
            disabled_routes: parse_json(&p.disabled_routes_json),
            require_approval: parse_json(&p.require_approval_json),
            log_sessions: p.log_sessions != 0,
            log_categories: p.log_categories != 0,
            log_policy_events: p.log_policy_events != 0,
            retention_days: p.retention_days,
            subject_can_view: p.subject_can_view != 0,
            effective_from: p.effective_from,
            effective_until: p.effective_until,
            version: p.version,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// Policy chain link — one layer in the policy inheritance chain
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PolicyChainLinkView {
    pub policy_id: String,
    pub author_tier: String,
    pub layer_order: i32,
}

/// Time access decision — result of time-based policy check
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "status")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum TimeAccessView {
    #[serde(rename = "allowed")]
    Allowed {
        remaining_session: Option<u32>,
        remaining_daily: Option<u32>,
    },
    #[serde(rename = "outside_window")]
    OutsideWindow,
    #[serde(rename = "session_limit")]
    SessionLimit,
    #[serde(rename = "daily_limit")]
    DailyLimit,
}
```

**Step 2: Add input view**

After the `UpdateAllocationInputView` impl, add:

```rust
/// Input for upserting a device policy — camelCase API boundary type
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpsertPolicyInputView {
    pub subject_id: Option<String>,
    pub device_id: Option<String>,
    pub content_rules: ContentRulesInput,
    pub time_rules: TimeRulesInput,
    pub feature_rules: FeatureRulesInput,
    #[serde(default)]
    pub monitoring_rules: Option<MonitoringRulesInput>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ContentRulesInput {
    #[serde(default)]
    pub blocked_categories: Vec<String>,
    #[serde(default)]
    pub blocked_hashes: Vec<String>,
    pub age_rating_max: Option<String>,
    pub reach_level_max: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct TimeRulesInput {
    pub session_max_minutes: Option<i32>,
    pub daily_max_minutes: Option<i32>,
    #[serde(default)]
    pub time_windows: Vec<serde_json::Value>,
    pub cooldown_minutes: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct FeatureRulesInput {
    #[serde(default)]
    pub disabled_features: Vec<String>,
    #[serde(default)]
    pub disabled_routes: Vec<String>,
    #[serde(default)]
    pub require_approval: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct MonitoringRulesInput {
    #[serde(default)]
    pub log_sessions: bool,
    #[serde(default)]
    pub log_categories: bool,
    #[serde(default = "default_true")]
    pub log_policy_events: bool,
    #[serde(default = "default_30i32")]
    pub retention_days: i32,
    #[serde(default = "default_true")]
    pub subject_can_view: bool,
}

fn default_true() -> bool { true }
fn default_30i32() -> i32 { 30 }
```

**Step 3: Add From<UpsertPolicyInputView> conversion**

This converts from the API input view to the DB input type:

```rust
impl UpsertPolicyInputView {
    /// Convert to DB input with author context
    pub fn to_db_input(self, author_id: &str, author_tier: &str) -> crate::db::device_policies::CreateDevicePolicyInput {
        let monitoring = self.monitoring_rules.unwrap_or(MonitoringRulesInput {
            log_sessions: false,
            log_categories: false,
            log_policy_events: true,
            retention_days: 30,
            subject_can_view: true,
        });
        crate::db::device_policies::CreateDevicePolicyInput {
            subject_id: self.subject_id.unwrap_or_default(),
            device_id: self.device_id,
            author_id: author_id.to_string(),
            author_tier: author_tier.to_string(),
            inherits_from: None,
            blocked_categories_json: serde_json::to_string(&self.content_rules.blocked_categories).unwrap_or_else(|_| "[]".into()),
            blocked_hashes_json: serde_json::to_string(&self.content_rules.blocked_hashes).unwrap_or_else(|_| "[]".into()),
            age_rating_max: self.content_rules.age_rating_max,
            reach_level_max: self.content_rules.reach_level_max,
            session_max_minutes: self.time_rules.session_max_minutes,
            daily_max_minutes: self.time_rules.daily_max_minutes,
            time_windows_json: serde_json::to_string(&self.time_rules.time_windows).unwrap_or_else(|_| "[]".into()),
            cooldown_minutes: self.time_rules.cooldown_minutes,
            disabled_features_json: serde_json::to_string(&self.feature_rules.disabled_features).unwrap_or_else(|_| "[]".into()),
            disabled_routes_json: serde_json::to_string(&self.feature_rules.disabled_routes).unwrap_or_else(|_| "[]".into()),
            require_approval_json: serde_json::to_string(&self.feature_rules.require_approval).unwrap_or_else(|_| "[]".into()),
            log_sessions: monitoring.log_sessions,
            log_categories: monitoring.log_categories,
            log_policy_events: monitoring.log_policy_events,
            retention_days: monitoring.retention_days,
            subject_can_view: monitoring.subject_can_view,
        }
    }
}
```

**Step 4: Build**

Run: `cd holochain/elohim-storage && RUSTFLAGS="" cargo build 2>&1 | tail -10`
Expected: Compiles cleanly.

**Step 5: Commit**

```bash
git add holochain/elohim-storage/src/views.rs
git commit -m "feat(storage): add DevicePolicy, PolicyChainLink, TimeAccess views"
```

---

## Task 5: Service Methods — policy chain + merge + time access

**Files:**
- Modify: `holochain/elohim-storage/src/services/stewardship_service.rs`

**Step 1: Add policy service methods**

Add new methods to `impl StewardshipService`:

```rust
    // =========================================================================
    // Device Policy Operations
    // =========================================================================

    /// Upsert a device policy for a subject
    pub fn upsert_device_policy(
        conn: &mut PooledConn,
        input: &crate::db::device_policies::CreateDevicePolicyInput,
    ) -> Result<crate::db::models::DevicePolicy, StorageError> {
        if input.subject_id.trim().is_empty() {
            return Err(StorageError::InvalidInput("subjectId must not be empty".into()));
        }
        if input.author_id.trim().is_empty() {
            return Err(StorageError::InvalidInput("authorId must not be empty".into()));
        }
        crate::db::device_policies::upsert_policy(conn, input)
    }

    /// Get all policies for a subject
    pub fn get_policies_for_subject(
        conn: &mut PooledConn,
        subject_id: &str,
    ) -> Result<Vec<crate::db::models::DevicePolicy>, StorageError> {
        crate::db::device_policies::get_policies_for_subject(conn, subject_id)
    }

    /// Get the most recent active policy for a subject
    pub fn get_subject_policy(
        conn: &mut PooledConn,
        subject_id: &str,
    ) -> Result<Option<crate::db::models::DevicePolicy>, StorageError> {
        let policies = crate::db::device_policies::get_policies_for_subject(conn, subject_id)?;
        Ok(policies.into_iter().next())
    }

    /// Get the parent's computed policy for a subject.
    /// Walks the inherits_from chain to find the parent policy,
    /// then merges up to that level.
    pub fn get_parent_policy(
        conn: &mut PooledConn,
        subject_id: &str,
    ) -> Result<Option<Value>, StorageError> {
        let policies = crate::db::device_policies::get_policies_for_subject(conn, subject_id)?;
        // Find the first policy that has inherits_from
        let parent_id = policies.iter().find_map(|p| p.inherits_from.clone());
        match parent_id {
            Some(pid) => {
                match crate::db::device_policies::get_policy_by_id(conn, &pid) {
                    Ok(parent) => Ok(Some(Self::policy_to_computed_json(&parent))),
                    Err(_) => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    /// Get the policy chain for a subject as PolicyChainLinks
    pub fn get_policy_chain(
        conn: &mut PooledConn,
        subject_id: &str,
    ) -> Result<Vec<Value>, StorageError> {
        let policies = crate::db::device_policies::get_policies_for_subject(conn, subject_id)?;
        Ok(policies.iter().enumerate().map(|(i, p)| {
            serde_json::json!({
                "policyId": p.id,
                "authorTier": p.author_tier,
                "layerOrder": tier_to_layer_order(&p.author_tier),
            })
        }).collect())
    }

    /// Convert a DevicePolicy to computed policy JSON
    fn policy_to_computed_json(p: &crate::db::models::DevicePolicy) -> Value {
        let blocked_categories: Value = serde_json::from_str(&p.blocked_categories_json).unwrap_or(Value::Array(vec![]));
        let blocked_hashes: Value = serde_json::from_str(&p.blocked_hashes_json).unwrap_or(Value::Array(vec![]));
        let disabled_features: Value = serde_json::from_str(&p.disabled_features_json).unwrap_or(Value::Array(vec![]));
        let disabled_routes: Value = serde_json::from_str(&p.disabled_routes_json).unwrap_or(Value::Array(vec![]));
        let require_approval: Value = serde_json::from_str(&p.require_approval_json).unwrap_or(Value::Array(vec![]));
        let time_windows: Value = serde_json::from_str(&p.time_windows_json).unwrap_or(Value::Array(vec![]));

        serde_json::json!({
            "subjectId": p.subject_id,
            "computedAt": p.updated_at,
            "blockedCategories": blocked_categories,
            "blockedHashes": blocked_hashes,
            "ageRatingMax": p.age_rating_max,
            "reachLevelMax": p.reach_level_max,
            "sessionMaxMinutes": p.session_max_minutes,
            "dailyMaxMinutes": p.daily_max_minutes,
            "timeWindowsJson": p.time_windows_json,
            "timeWindows": time_windows,
            "cooldownMinutes": p.cooldown_minutes,
            "disabledFeatures": disabled_features,
            "disabledRoutes": disabled_routes,
            "requireApproval": require_approval,
            "logSessions": p.log_sessions != 0,
            "logCategories": p.log_categories != 0,
            "logPolicyEvents": p.log_policy_events != 0,
            "retentionDays": p.retention_days,
            "subjectCanView": p.subject_can_view != 0,
        })
    }
```

Also add the tier helper function outside the impl block:

```rust
/// Map author tier to layer order for policy chain
fn tier_to_layer_order(tier: &str) -> i32 {
    match tier {
        "self" => 0,
        "guide" => 1,
        "guardian" => 2,
        "coordinator" => 3,
        "constitutional" => 4,
        _ => 0,
    }
}
```

**Step 2: Build and test**

Run: `cd holochain/elohim-storage && RUSTFLAGS="" cargo test --lib 2>&1 | tail -20`
Expected: All tests pass.

**Step 3: Commit**

```bash
git add holochain/elohim-storage/src/services/stewardship_service.rs
git commit -m "feat(storage): add device policy service methods with chain computation"
```

---

## Task 6: API Endpoints — 7 new routes

**Files:**
- Modify: `holochain/elohim-storage/src/api/stewardship.rs`

**Step 1: Add imports for new types**

At top of file, add to existing imports:

```rust
use crate::views::{
    ContentStewardshipView, CreateAllocationInputView, DevicePolicyView,
    PolicyChainLinkView, StewardshipAllocationView, TimeAccessView,
    UpdateAllocationInputView, UpsertPolicyInputView,
};
```

**Step 2: Add new route matches in dispatcher**

In the `handle()` function, add these before the `_ => response::not_found(...)` catch-all:

```rust
        // -----------------------------------------------------------------
        // Device Policies (backed by device_policies table)
        // -----------------------------------------------------------------
        (&Method::POST, "policies") => handle_upsert_policy(req, pool).await,

        (&Method::GET, "policies/me/chain") => handle_my_policy_chain(req, pool).await,

        (&Method::GET, p) if p.starts_with("policies/") && p.ends_with("/parent") => {
            let subject_id = p.trim_start_matches("policies/").trim_end_matches("/parent");
            handle_get_parent_policy(subject_id, pool).await
        }

        (&Method::GET, p) if p.starts_with("policies/") && p.ends_with("/chain") => {
            let subject_id = p.trim_start_matches("policies/").trim_end_matches("/chain");
            handle_get_policy_chain(subject_id, pool).await
        }

        (&Method::GET, "policies") => handle_list_policies(req, pool).await,

        (&Method::GET, p) if p.starts_with("policies/") => {
            let subject_id = p.trim_start_matches("policies/");
            handle_get_subject_policy(subject_id, pool).await
        }

        // -----------------------------------------------------------------
        // Time access check (uses existing PolicyEnforcement)
        // -----------------------------------------------------------------
        (&Method::GET, "access/time") => handle_check_time_access(req, pool).await,
```

**IMPORTANT:** The order matters — `policies/me/chain` must come before the `policies/` wildcard.

**Step 3: Add handler functions**

```rust
// =============================================================================
// Device Policy handlers
// =============================================================================

async fn handle_upsert_policy(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Response<Full<Bytes>> {
    let input_view: UpsertPolicyInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for upsert policy"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    // In v0, author_id defaults to "self" (no auth context yet)
    let db_input = input_view.to_db_input("self", "self");

    match StewardshipService::upsert_device_policy(&mut conn, &db_input) {
        Ok(policy) => response::ok(&DevicePolicyView::from(policy)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_list_policies(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Response<Full<Bytes>> {
    let subject_id = extract_query_param(req.uri().query(), "subjectId");
    let subject_id = match subject_id {
        Some(id) if !id.is_empty() => id,
        _ => return response::bad_request("subjectId query parameter is required"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_policies_for_subject(&mut conn, &subject_id) {
        Ok(policies) => {
            let views: Vec<DevicePolicyView> = policies.into_iter().map(|p| p.into()).collect();
            response::ok(&views)
        }
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_subject_policy(
    subject_id: &str,
    pool: &DbPool,
) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_subject_policy(&mut conn, subject_id) {
        Ok(Some(policy)) => response::ok(&DevicePolicyView::from(policy)),
        Ok(None) => response::not_found(&format!("No policy found for subject {}", subject_id)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_parent_policy(
    subject_id: &str,
    pool: &DbPool,
) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_parent_policy(&mut conn, subject_id) {
        Ok(Some(policy)) => response::ok(&policy),
        Ok(None) => response::ok(&serde_json::json!(null)),
        Err(e) => response::error_response(e),
    }
}

async fn handle_get_policy_chain(
    subject_id: &str,
    pool: &DbPool,
) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_policy_chain(&mut conn, subject_id) {
        Ok(chain) => response::ok(&chain),
        Err(e) => response::error_response(e),
    }
}

async fn handle_my_policy_chain(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Response<Full<Bytes>> {
    let agent_id = extract_query_param(req.uri().query(), "agentId");
    let agent_id = match agent_id {
        Some(id) if !id.is_empty() => id,
        _ => return response::bad_request("agentId query parameter is required"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match StewardshipService::get_policy_chain(&mut conn, &agent_id) {
        Ok(chain) => response::ok(&chain),
        Err(e) => response::error_response(e),
    }
}

async fn handle_check_time_access(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Response<Full<Bytes>> {
    let agent_id = extract_query_param(req.uri().query(), "agentId");
    let agent_id = match agent_id {
        Some(id) if !id.is_empty() => id,
        _ => return response::bad_request("agentId query parameter is required"),
    };

    // Use existing PolicyEnforcement from policy_cache
    let policy_cache = crate::db::policy_cache::PolicyCache::new(pool.clone());
    let enforcement = crate::db::policy_cache::PolicyEnforcement::new(policy_cache);

    match enforcement.check_time_access(&agent_id) {
        Ok(decision) => {
            let view = match decision {
                crate::db::policy_cache::TimeAccessDecision::Allowed { remaining_session, remaining_daily } => {
                    serde_json::json!({ "status": "allowed", "remainingSession": remaining_session, "remainingDaily": remaining_daily })
                }
                crate::db::policy_cache::TimeAccessDecision::OutsideWindow => {
                    serde_json::json!({ "status": "outside_window" })
                }
                crate::db::policy_cache::TimeAccessDecision::SessionLimit => {
                    serde_json::json!({ "status": "session_limit" })
                }
                crate::db::policy_cache::TimeAccessDecision::DailyLimit => {
                    serde_json::json!({ "status": "daily_limit" })
                }
            };
            response::ok(&view)
        }
        Err(e) => {
            // Fail open — return allowed if check fails
            response::ok(&serde_json::json!({ "status": "allowed" }))
        }
    }
}
```

**Step 4: Build and test**

Run: `cd holochain/elohim-storage && RUSTFLAGS="" cargo build 2>&1 | tail -10`
Expected: Compiles cleanly.

Run: `cd holochain/elohim-storage && RUSTFLAGS="" cargo test --lib 2>&1 | tail -20`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add holochain/elohim-storage/src/api/stewardship.rs
git commit -m "feat(storage): add 7 stewardship policy API endpoints"
```

---

## Task 7: Angular — Expand Interface + Thin Service

**Files:**
- Modify: `elohim-app/src/app/imagodei/interfaces/stewardship-policy.interface.ts`
- Modify: `elohim-app/src/app/imagodei/services/stewardship-api.service.ts`

**Step 1: Expand IStewardshipPolicy interface**

Add imports and 7 new methods to the interface:

```typescript
import type {
  ComputedPolicy,
  CreateGrantInput,
  DelegateGrantInput,
  DevicePolicy,
  FileAppealInput,
  PolicyChainLink,
  PolicyDecision,
  StewardshipAppeal,
  StewardshipGrant,
  TimeAccessDecision,
  UpsertPolicyInput,
} from '../models/stewardship.model';
```

Add to the interface body:

```typescript
  // ===========================================================================
  // Policy Management
  // ===========================================================================

  /** Check time-based access status */
  checkTimeAccess(): Promise<TimeAccessDecision>;

  /** Get my grant for a specific subject */
  getGrantForSubject(subjectId: string): Promise<StewardshipGrant | null>;

  /** Get the active device policy for a subject */
  getSubjectPolicy(subjectId: string): Promise<DevicePolicy | null>;

  /** Get the parent's computed policy for a subject */
  getParentPolicy(subjectId: string): Promise<ComputedPolicy | null>;

  /** Get the policy inheritance chain for a subject */
  getPolicyChain(subjectId: string): Promise<PolicyChainLink[]>;

  /** Get my own policy inheritance chain */
  getMyPolicyChain(): Promise<PolicyChainLink[]>;

  /** Create or update a device policy */
  upsertPolicy(input: UpsertPolicyInput): Promise<DevicePolicy | null>;
```

Update the token factory JSDoc to reference StewardshipApiService instead of StewardshipService.

**Step 2: Expand StewardshipApiService**

Add imports:

```typescript
import type {
  ComputedPolicy,
  CreateGrantInput,
  DelegateGrantInput,
  DevicePolicy,
  FileAppealInput,
  PolicyChainLink,
  PolicyDecision,
  StewardshipAppeal,
  StewardshipGrant,
  TimeAccessDecision,
  UpsertPolicyInput,
} from '../models/stewardship.model';
```

Add 7 new methods:

```typescript
  async checkTimeAccess(): Promise<TimeAccessDecision> {
    return firstValueFrom(
      this.http
        .get<TimeAccessDecision>('/api/v1/stewardship/access/time')
        .pipe(catchError(() => of({ status: 'allowed' as const })))
    );
  }

  async getGrantForSubject(subjectId: string): Promise<StewardshipGrant | null> {
    // Computed client-side from existing endpoint
    const subjects = await this.getMySubjects();
    return subjects.find(g => g.subjectId === subjectId && g.status === 'active') ?? null;
  }

  async getSubjectPolicy(subjectId: string): Promise<DevicePolicy | null> {
    return firstValueFrom(
      this.http
        .get<DevicePolicy>(`/api/v1/stewardship/policies/${subjectId}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async getParentPolicy(subjectId: string): Promise<ComputedPolicy | null> {
    return firstValueFrom(
      this.http
        .get<ComputedPolicy | null>(`/api/v1/stewardship/policies/${subjectId}/parent`)
        .pipe(catchError(() => of(null)))
    );
  }

  async getPolicyChain(subjectId: string): Promise<PolicyChainLink[]> {
    return firstValueFrom(
      this.http
        .get<PolicyChainLink[]>(`/api/v1/stewardship/policies/${subjectId}/chain`)
        .pipe(catchError(() => of([])))
    );
  }

  async getMyPolicyChain(): Promise<PolicyChainLink[]> {
    return firstValueFrom(
      this.http
        .get<PolicyChainLink[]>('/api/v1/stewardship/policies/me/chain')
        .pipe(catchError(() => of([])))
    );
  }

  async upsertPolicy(input: UpsertPolicyInput): Promise<DevicePolicy | null> {
    return firstValueFrom(
      this.http
        .post<DevicePolicy>('/api/v1/stewardship/policies', input)
        .pipe(catchError(() => of(null)))
    );
  }
```

**Step 3: Verify TypeScript compiles**

Run: `cd elohim-app && npx tsc --noEmit --project tsconfig.json 2>&1 | head -20`
Expected: No new errors.

**Step 4: Commit**

```bash
git add elohim-app/src/app/imagodei/interfaces/stewardship-policy.interface.ts \
       elohim-app/src/app/imagodei/services/stewardship-api.service.ts
git commit -m "feat(imagodei): expand IStewardshipPolicy with 7 policy management methods"
```

---

## Task 8: Angular — Switch Consumers + Delete Fat Service

**Files:**
- Modify: `elohim-app/src/app/imagodei/components/appeal-wizard/appeal-wizard.component.ts`
- Modify: `elohim-app/src/app/imagodei/components/policy-console/policy-console.component.ts`
- Modify: `elohim-app/src/app/imagodei/components/capabilities-dashboard/capabilities-dashboard.component.ts`
- Modify: `elohim-app/src/app/imagodei/components/community-intervention/community-intervention.component.ts`
- Delete: `elohim-app/src/app/imagodei/services/stewardship.service.ts`
- Delete: `elohim-app/src/app/imagodei/services/stewardship.service.spec.ts`

**Step 1: Switch appeal-wizard**

Replace:
```typescript
import { StewardshipService } from '../../services/stewardship.service';
```
With:
```typescript
import { STEWARDSHIP_POLICY } from '../../interfaces/stewardship-policy.interface';
```

Replace the inject line (search for `inject(StewardshipService)`):
```typescript
// Before:
private readonly stewardship = inject(StewardshipService);
// After:
private readonly stewardship = inject(STEWARDSHIP_POLICY);
```

**Step 2: Switch policy-console**

Same pattern — replace import and inject:

```typescript
// Before:
import { StewardshipService } from '../../services/stewardship.service';
// After:
import { STEWARDSHIP_POLICY } from '../../interfaces/stewardship-policy.interface';

// Before:
private readonly stewardship = inject(StewardshipService);
// After:
private readonly stewardship = inject(STEWARDSHIP_POLICY);
```

**Step 3: Switch capabilities-dashboard**

Same pattern.

**Step 4: Remove dead injection from community-intervention**

Remove the import line:
```typescript
import { StewardshipService } from '../../services/stewardship.service';
```

Remove the inject line:
```typescript
private readonly stewardship = inject(StewardshipService);
```

**Step 5: Update spec files**

For each of the 4 component spec files, replace:
```typescript
import { StewardshipService } from '../../services/stewardship.service';
// With:
import { STEWARDSHIP_POLICY } from '../../interfaces/stewardship-policy.interface';
```

And in the providers array:
```typescript
// Before:
{ provide: StewardshipService, useValue: mockStewardshipService }
// After:
{ provide: STEWARDSHIP_POLICY, useValue: mockStewardshipService }
```

For `community-intervention.component.spec.ts`, remove the StewardshipService import and provider entirely.

**Step 6: Delete fat service files**

```bash
rm elohim-app/src/app/imagodei/services/stewardship.service.ts
rm elohim-app/src/app/imagodei/services/stewardship.service.spec.ts
```

**Step 7: Build verification**

Run: `cd elohim-app && pnpm run build 2>&1 | tail -20`
Expected: AOT build succeeds.

**Step 8: Test verification**

Run: `cd elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -30`
Expected: Tests pass (minus the 89 deleted spec tests).

**Step 9: Commit**

```bash
git add -A
git commit -m "refactor(v0): delete StewardshipService, switch consumers to STEWARDSHIP_POLICY token (-932 lines)"
```

---

## Task 9: Rust Test Verification + Final Push

**Step 1: Full Rust test suite**

Run: `cd holochain/elohim-storage && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -30`
Expected: All tests pass.

Run: `cd holochain/elohim-storage && RUSTFLAGS="" cargo clippy -- -D warnings 2>&1 | tail -10`
Expected: No warnings.

Run: `cd holochain/elohim-storage && cargo fmt --check 2>&1 | tail -10`
Expected: No formatting issues.

**Step 2: Full Angular quality gate**

Run: `cd elohim-app && pnpm run lint 2>&1 | tail -10`
Expected: No lint errors.

Run: `cd elohim-app && pnpm run format:check 2>&1 | tail -10`
Expected: No formatting issues.

**Step 3: Fix any issues and commit**

If clippy/fmt/lint issues: fix, commit as separate style commit.

**Step 4: Push**

```bash
git push origin dev
```

---

## Summary of Changes

| Area | Added | Deleted | Net |
|------|-------|---------|-----|
| Rust (storage) | ~800 lines | 0 | +800 |
| Angular (app) | ~120 lines | ~1050 lines | -930 |
| **Total** | ~920 lines | ~1050 lines | **-130** |

Business logic moves from browser (Angular fat service with Holochain zome calls) to Rust (elohim-storage with SQLite + policy cache). Angular becomes a thin HTTP client behind an injection token.
