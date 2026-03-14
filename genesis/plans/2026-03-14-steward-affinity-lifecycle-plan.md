# Steward Affinity Lifecycle Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire real steward affinity data through the recognition pipeline, replacing hardcoded 1.0 values, with a mastery gate that ensures only proven learners can build stewardship standing.

**Architecture:** New `steward_affinity` table stores per-steward-per-content affinity scores. Genesis seeds initial values matching a2o scenarios. The recognition pipeline's Stage 2 queries real affinity instead of defaulting to 1.0. Increment 2 adds a mastery gate (must reach mastery on practicable content before curation acts build affinity). Increment 3 implements constitutional limits in Stage 4.

**Tech Stack:** Rust (diesel, hyper, serde), SQLite, TypeScript (genesis seeder), ts-rs for type generation

---

## Increment 1: Storage + Pipeline Wiring

### Task 1: Create `steward_affinity` diesel migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-14-000000_steward_affinity/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-14-000000_steward_affinity/down.sql`

**Step 1: Create migration directory**

```bash
mkdir -p elohim/elohim-storage/migrations/2026-03-14-000000_steward_affinity
```

**Step 2: Write up.sql**

```sql
CREATE TABLE IF NOT EXISTS steward_affinity (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    steward_id TEXT NOT NULL,
    content_id TEXT NOT NULL,
    affinity_score REAL NOT NULL DEFAULT 0.0,
    source TEXT NOT NULL DEFAULT 'genesis_seed',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_steward_affinity_unique
    ON steward_affinity(app_id, steward_id, content_id);
CREATE INDEX IF NOT EXISTS idx_steward_affinity_app_id
    ON steward_affinity(app_id);
CREATE INDEX IF NOT EXISTS idx_steward_affinity_steward
    ON steward_affinity(app_id, steward_id);
CREATE INDEX IF NOT EXISTS idx_steward_affinity_content
    ON steward_affinity(app_id, content_id);
```

**Step 3: Write down.sql**

```sql
DROP TABLE IF EXISTS steward_affinity;
```

**Step 4: Run migration to verify**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

Expected: Build succeeds (diesel auto-runs migrations on startup)

**Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-03-14-000000_steward_affinity/
git commit -m "feat(storage): add steward_affinity table migration"
```

---

### Task 2: Add diesel schema and model for `steward_affinity`

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`
- Modify: `elohim/elohim-storage/src/db/models.rs`

**Step 1: Add table definition to diesel_schema.rs**

Add after the `stewardship_allocations` table block (after line 309):

```rust
diesel::table! {
    steward_affinity (id) {
        id -> Text,
        app_id -> Text,
        steward_id -> Text,
        content_id -> Text,
        affinity_score -> Float,
        source -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

Add `steward_affinity` to the `diesel::allow_tables_to_appear_in_same_query!()` macro (alphabetically, between `steward_credentials` and `stewardship_allocations`).

**Step 2: Add model structs to models.rs**

Add near the other stewardship models:

```rust
// ============================================================================
// Steward Affinity
// ============================================================================

/// Steward affinity source types
pub mod affinity_sources {
    pub const GENESIS_SEED: &str = "genesis_seed";
    pub const MASTERY_GATE: &str = "mastery_gate";
    pub const CURATION_EDIT: &str = "curation_edit";
    pub const CURATION_REVIEW: &str = "curation_review";
    pub const DISPUTE_RESOLUTION: &str = "dispute_resolution";

    pub fn is_valid(source: &str) -> bool {
        matches!(
            source,
            GENESIS_SEED | MASTERY_GATE | CURATION_EDIT | CURATION_REVIEW | DISPUTE_RESOLUTION
        )
    }
}

/// Steward affinity record — tracks a steward's earned relationship to content
#[derive(Debug, Clone, Queryable, Serialize)]
#[diesel(table_name = crate::db::diesel_schema::steward_affinity)]
pub struct StewardAffinity {
    pub id: String,
    pub app_id: String,
    pub steward_id: String,
    pub content_id: String,
    pub affinity_score: f32,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Insertable steward affinity record
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::diesel_schema::steward_affinity)]
pub struct NewStewardAffinity<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub steward_id: &'a str,
    pub content_id: &'a str,
    pub affinity_score: f32,
    pub source: &'a str,
}
```

**Step 3: Build to verify**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

Expected: Build succeeds

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/db/diesel_schema.rs elohim/elohim-storage/src/db/models.rs
git commit -m "feat(storage): add StewardAffinity model and diesel schema"
```

---

### Task 3: Create `steward_affinity` CRUD module

**Files:**
- Create: `elohim/elohim-storage/src/db/steward_affinity.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` (add `pub mod steward_affinity;`)

**Step 1: Write the CRUD module**

Follow the pattern from `stewardship_allocations.rs`:

```rust
//! Steward affinity CRUD operations
//!
//! Tracks earned steward-content affinity. Affinity grows through curation
//! acts (edit, review, dispute resolution) and is gated by mastery.
//! Seeded via genesis for initial state.

use diesel::prelude::*;
use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::steward_affinity;
use super::models::{current_timestamp, affinity_sources, NewStewardAffinity, StewardAffinity};
use crate::error::StorageError;

// ============================================================================
// Input Types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAffinityInput {
    pub steward_id: String,
    pub content_id: String,
    #[serde(default)]
    pub affinity_score: f32,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String {
    affinity_sources::GENESIS_SEED.to_string()
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffinityQuery {
    pub steward_id: Option<String>,
    pub content_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Create a new steward affinity record
pub fn create_affinity(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: &CreateAffinityInput,
) -> Result<StewardAffinity, StorageError> {
    if !affinity_sources::is_valid(&input.source) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid affinity source: {}",
            input.source
        )));
    }

    if input.affinity_score < 0.0 || input.affinity_score > 1.0 {
        return Err(StorageError::InvalidInput(format!(
            "Affinity score must be between 0.0 and 1.0, got: {}",
            input.affinity_score
        )));
    }

    let id = Uuid::new_v4().to_string();

    let new_affinity = NewStewardAffinity {
        id: &id,
        app_id: ctx.app_id(),
        steward_id: &input.steward_id,
        content_id: &input.content_id,
        affinity_score: input.affinity_score,
        source: &input.source,
    };

    diesel::insert_into(steward_affinity::table)
        .values(&new_affinity)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create affinity: {}", e)))?;

    debug!(
        "Created steward affinity {} for steward {} on content {}",
        id, input.steward_id, input.content_id
    );

    get_affinity_by_id(conn, ctx, &id)
}

/// Get affinity by ID
pub fn get_affinity_by_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<StewardAffinity, StorageError> {
    steward_affinity::table
        .filter(steward_affinity::id.eq(id))
        .filter(steward_affinity::app_id.eq(ctx.app_id()))
        .first::<StewardAffinity>(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => StorageError::NotFound(id.to_string()),
            _ => StorageError::Internal(format!("Failed to get affinity: {}", e)),
        })
}

/// Get affinity for a specific steward-content pair
pub fn get_affinity_for_steward_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    steward_id: &str,
    content_id: &str,
) -> Result<Option<StewardAffinity>, StorageError> {
    steward_affinity::table
        .filter(steward_affinity::app_id.eq(ctx.app_id()))
        .filter(steward_affinity::steward_id.eq(steward_id))
        .filter(steward_affinity::content_id.eq(content_id))
        .first::<StewardAffinity>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List affinities with optional filters
pub fn list_affinities(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &AffinityQuery,
) -> Result<Vec<StewardAffinity>, StorageError> {
    let mut q = steward_affinity::table
        .filter(steward_affinity::app_id.eq(ctx.app_id()))
        .into_boxed();

    if let Some(steward_id) = &query.steward_id {
        q = q.filter(steward_affinity::steward_id.eq(steward_id));
    }

    if let Some(content_id) = &query.content_id {
        q = q.filter(steward_affinity::content_id.eq(content_id));
    }

    q = q.order(steward_affinity::affinity_score.desc());

    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    if let Some(offset) = query.offset {
        q = q.offset(offset);
    }

    q.load::<StewardAffinity>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list affinities: {}", e)))
}

/// Update affinity score and source for a steward-content pair
pub fn update_affinity_score(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    steward_id: &str,
    content_id: &str,
    delta: f32,
    source: &str,
) -> Result<StewardAffinity, StorageError> {
    let existing = get_affinity_for_steward_content(conn, ctx, steward_id, content_id)?;
    let now = current_timestamp();

    match existing {
        Some(record) => {
            let new_score = (record.affinity_score + delta).clamp(0.0, 1.0);
            diesel::update(steward_affinity::table)
                .filter(steward_affinity::id.eq(&record.id))
                .filter(steward_affinity::app_id.eq(ctx.app_id()))
                .set((
                    steward_affinity::affinity_score.eq(new_score),
                    steward_affinity::source.eq(source),
                    steward_affinity::updated_at.eq(&now),
                ))
                .execute(conn)
                .map_err(|e| {
                    StorageError::Internal(format!("Failed to update affinity: {}", e))
                })?;
            get_affinity_by_id(conn, ctx, &record.id)
        }
        None => Err(StorageError::NotFound(format!(
            "No affinity record for steward {} on content {}",
            steward_id, content_id
        ))),
    }
}

/// Bulk create affinities (for genesis seeding)
pub fn bulk_create_affinities(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    inputs: &[CreateAffinityInput],
) -> Result<(usize, Vec<String>), StorageError> {
    let mut created = 0;
    let mut errors = Vec::new();

    for input in inputs {
        match create_affinity(conn, ctx, input) {
            Ok(_) => created += 1,
            Err(e) => errors.push(format!(
                "steward={} content={}: {}",
                input.steward_id, input.content_id, e
            )),
        }
    }

    Ok((created, errors))
}
```

**Step 2: Register module in mod.rs**

Add `pub mod steward_affinity;` to `elohim/elohim-storage/src/db/mod.rs` (alphabetically near other modules).

**Step 3: Build to verify**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/db/steward_affinity.rs elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): add steward_affinity CRUD module"
```

---

### Task 4: Add View types and API routes for steward affinity

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`
- Create: `elohim/elohim-storage/src/api/steward_affinity.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

**Step 1: Add View types to views.rs**

Add after the Recognition Pipeline Views section:

```rust
// ============================================================================
// Steward Affinity Views
// ============================================================================

/// Steward affinity output view
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardAffinityView {
    pub id: String,
    pub steward_id: String,
    pub content_id: String,
    pub affinity_score: f32,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::db::models::StewardAffinity> for StewardAffinityView {
    fn from(a: crate::db::models::StewardAffinity) -> Self {
        Self {
            id: a.id,
            steward_id: a.steward_id,
            content_id: a.content_id,
            affinity_score: a.affinity_score,
            source: a.source,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

/// Input for creating steward affinity via API
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateStewardAffinityInputView {
    pub steward_id: String,
    pub content_id: String,
    #[serde(default)]
    pub affinity_score: f32,
    #[serde(default)]
    pub source: Option<String>,
}

impl From<CreateStewardAffinityInputView> for crate::db::steward_affinity::CreateAffinityInput {
    fn from(v: CreateStewardAffinityInputView) -> Self {
        Self {
            steward_id: v.steward_id,
            content_id: v.content_id,
            affinity_score: v.affinity_score,
            source: v.source.unwrap_or_else(|| "genesis_seed".to_string()),
        }
    }
}

/// Bulk create input
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct BulkCreateStewardAffinityInputView {
    pub affinities: Vec<CreateStewardAffinityInputView>,
}
```

**Step 2: Create API controller**

Create `elohim/elohim-storage/src/api/steward_affinity.rs`:

```rust
//! Steward Affinity API controller
//!
//! Routes: `/api/v1/steward-affinity[/*]`

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::steward_affinity::{self, AffinityQuery};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{
    BulkCreateStewardAffinityInputView, CreateStewardAffinityInputView, StewardAffinityView,
};

use super::{get_conn, parse_body, parse_query};

/// Handle `/api/v1/steward-affinity*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        (&Method::GET, "") => handle_list(req, pool, ctx).await,
        (&Method::POST, "") => handle_create(req, pool, ctx).await,
        (&Method::POST, "bulk") => handle_bulk_create(req, pool, ctx).await,
        (&Method::GET, id) => handle_get_by_id(id, pool, ctx).await,

        _ => Ok(response::not_found(&format!(
            "Unknown steward-affinity route: {} /api/v1/steward-affinity/{}",
            method, path
        ))),
    }
}

async fn handle_list(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query: AffinityQuery = parse_query(req.uri())?;
    let mut conn = get_conn(pool)?;
    let affinities = steward_affinity::list_affinities(&mut conn, ctx, &query)?;
    let views: Vec<StewardAffinityView> = affinities.into_iter().map(Into::into).collect();
    Ok(response::ok(&views))
}

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: CreateStewardAffinityInputView = parse_body(req).await?;
    let db_input = input.into();
    let mut conn = get_conn(pool)?;
    let affinity = steward_affinity::create_affinity(&mut conn, ctx, &db_input)?;
    let view = StewardAffinityView::from(affinity);
    Ok(response::created(&view))
}

async fn handle_bulk_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: BulkCreateStewardAffinityInputView = parse_body(req).await?;
    let db_inputs: Vec<_> = input.affinities.into_iter().map(Into::into).collect();
    let mut conn = get_conn(pool)?;
    let (created, errors) = steward_affinity::bulk_create_affinities(&mut conn, ctx, &db_inputs)?;
    Ok(response::ok(&serde_json::json!({
        "created": created,
        "errors": errors,
    })))
}

async fn handle_get_by_id(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let affinity = steward_affinity::get_affinity_by_id(&mut conn, ctx, id)?;
    let view = StewardAffinityView::from(affinity);
    Ok(response::ok(&view))
}
```

**Step 3: Register in api/mod.rs**

Add `pub mod steward_affinity;`

**Step 4: Wire routes in http.rs**

Find the recognition routes section (~line 5355) and add after it:

```rust
        // =====================================================================
        // /api/v1/steward-affinity — Steward affinity lifecycle
        // =====================================================================
        .route(
            Route::get("/api/v1/steward-affinity")
                .handler("list_steward_affinity")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/steward-affinity")
                .handler("create_steward_affinity")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/steward-affinity/bulk")
                .handler("bulk_create_steward_affinity")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward-affinity/{id}")
                .handler("get_steward_affinity")
                .cache_ttl(60)
                .build(),
        )
```

Also add the dispatch match in the main handler function, following the pattern of other `/api/v1/` routes:

```rust
        ["api", "v1", "steward-affinity", rest @ ..] => {
            api::steward_affinity::handle(req, method, &rest.join("/"), pool, ctx).await
        }
```

**Step 5: Build to verify**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -10
```

**Step 6: Commit**

```bash
git add elohim/elohim-storage/src/views.rs elohim/elohim-storage/src/api/steward_affinity.rs elohim/elohim-storage/src/api/mod.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add steward-affinity API routes and views"
```

---

### Task 5: Wire Stage 2 to query real affinity

**Files:**
- Modify: `elohim/elohim-storage/src/services/recognition_pipeline_service.rs`

**Step 1: Write failing test**

Add to the `tests` module at the bottom of `recognition_pipeline_service.rs`:

```rust
    #[test]
    fn resolve_with_affinity_uses_provided_scores() {
        // When affinity data is provided, it should be used instead of 1.0
        let allocations = vec![
            make_allocation("alloc-1", "steward-1", 0.6, "active"),
            make_allocation("alloc-2", "steward-2", 0.4, "active"),
        ];
        let affinity_map = vec![
            ("steward-1".to_string(), 0.85_f64),
            ("steward-2".to_string(), 0.50_f64),
        ]
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();

        let resolved = resolve_from_allocations_with_affinity(&allocations, &affinity_map);
        assert_eq!(resolved.len(), 2);
        assert!((resolved[0].stored_affinity - 0.85).abs() < f64::EPSILON);
        assert!((resolved[1].stored_affinity - 0.50).abs() < f64::EPSILON);
    }

    #[test]
    fn resolve_with_affinity_defaults_to_zero_when_missing() {
        let allocations = vec![
            make_allocation("alloc-1", "steward-1", 0.6, "active"),
        ];
        let affinity_map = std::collections::HashMap::new(); // No affinity data

        let resolved = resolve_from_allocations_with_affinity(&allocations, &affinity_map);
        assert_eq!(resolved.len(), 1);
        assert!((resolved[0].stored_affinity - 0.0).abs() < f64::EPSILON);
    }
```

**Step 2: Run tests to verify they fail**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib recognition_pipeline 2>&1 | tail -10
```

Expected: FAIL — `resolve_from_allocations_with_affinity` not found

**Step 3: Implement `resolve_from_allocations_with_affinity`**

Add to the Stage 2 section of `recognition_pipeline_service.rs`, after `resolve_from_allocations`:

```rust
/// Stage 2 (pure): Build resolved stewards with real affinity data.
/// Falls back to 0.0 for stewards without affinity records (no affinity = no share).
pub fn resolve_from_allocations_with_affinity(
    allocations: &[StewardshipAllocation],
    affinity_map: &std::collections::HashMap<String, f64>,
) -> Vec<ResolvedSteward> {
    allocations
        .iter()
        .filter(|a| a.governance_state == "active")
        .map(|a| {
            let stored_affinity = affinity_map
                .get(&a.steward_presence_id)
                .copied()
                .unwrap_or(0.0);
            ResolvedSteward {
                allocation_id: a.id.clone(),
                steward_presence_id: a.steward_presence_id.clone(),
                allocation_ratio: a.allocation_ratio,
                stored_affinity,
                derived_affinity: 1.0,
                contribution_type: a.contribution_type.clone(),
            }
        })
        .collect()
}
```

**Step 4: Run tests to verify they pass**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib recognition_pipeline 2>&1 | tail -10
```

Expected: All tests pass

**Step 5: Update `resolve_stewards` DB function to use real affinity**

Add import at top of file:

```rust
use crate::db::steward_affinity;
```

Replace the `resolve_stewards` function:

```rust
/// Stage 2 (DB): Query allocations for content, then resolve with real affinity.
pub fn resolve_stewards(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    content_id: &str,
) -> Result<Vec<ResolvedSteward>, StorageError> {
    let allocations = stewardship_allocations::get_allocations_for_content(conn, ctx, content_id)?;

    // Build affinity map from DB
    let affinities = steward_affinity::list_affinities(
        conn,
        ctx,
        &steward_affinity::AffinityQuery {
            content_id: Some(content_id.to_string()),
            ..Default::default()
        },
    )?;

    let affinity_map: std::collections::HashMap<String, f64> = affinities
        .into_iter()
        .map(|a| (a.steward_id, a.affinity_score as f64))
        .collect();

    Ok(resolve_from_allocations_with_affinity(&allocations, &affinity_map))
}
```

**Step 6: Build and run all tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib recognition_pipeline 2>&1 | tail -10
```

**Step 7: Commit**

```bash
git add elohim/elohim-storage/src/services/recognition_pipeline_service.rs
git commit -m "feat(storage): wire Stage 2 to query real steward affinity"
```

---

### Task 6: Add steward affinity seeding to genesis

**Files:**
- Modify: `genesis/seeder/src/seed-stewardship.ts`

**Step 1: Add affinity seeding after allocation seeding**

Add a `CATEGORY_AFFINITY_MAP` that maps category → steward affinity scores, derived from the existing `CATEGORY_STEWARD_MAP` ratios. The affinity score represents curation relationship strength, distinct from allocation ratio.

Add after the `CATEGORY_STEWARD_MAP` definition (~line 193):

```typescript
// =============================================================================
// Category-to-Steward Affinity Scores
//
// Affinity represents earned stewardship standing through curation.
// Higher affinity = deeper relationship with the content domain.
// These are initial seeds — real activity will update them over time.
// =============================================================================

interface StewardAffinityEntry {
  stewardId: string;
  affinityScore: number;
}

const CATEGORY_AFFINITY_MAP: Record<string, StewardAffinityEntry[]> = {
  'public-observer': [
    { stewardId: 'eve-firstwoman', affinityScore: 0.85 },
    { stewardId: 'nancy-neighbor', affinityScore: 0.60 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'scripture': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'fct': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'fct-media': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'fct-practice': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'fct-narrative': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'fct-activity': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'value-scanner': [
    { stewardId: 'adam-firstman', affinityScore: 0.75 },
    { stewardId: 'jessica-spouse', affinityScore: 0.55 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
    { stewardId: 'frank-farmer', affinityScore: 0.45 },
  ],
  'governance': [
    { stewardId: 'nancy-neighbor', affinityScore: 0.70 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
    { stewardId: 'eve-firstwoman', affinityScore: 0.55 },
  ],
  'social-medium': [
    { stewardId: 'eve-firstwoman', affinityScore: 0.80 },
    { stewardId: 'jessica-spouse', affinityScore: 0.55 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'autonomous-entity': [
    { stewardId: 'meriadoc-moneybags', affinityScore: 0.65 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
    { stewardId: 'frank-farmer', affinityScore: 0.45 },
  ],
  'economic-coordination': [
    { stewardId: 'meriadoc-moneybags', affinityScore: 0.65 },
    { stewardId: 'frank-farmer', affinityScore: 0.60 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'community': [
    { stewardId: 'nancy-neighbor', affinityScore: 0.70 },
    { stewardId: 'adam-firstman', affinityScore: 0.55 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'local-economy': [
    { stewardId: 'frank-farmer', affinityScore: 0.70 },
    { stewardId: 'meriadoc-moneybags', affinityScore: 0.55 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'foundation': [
    { stewardId: 'dan-developer', affinityScore: 0.75 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'contributor': [
    { stewardId: 'dan-developer', affinityScore: 0.75 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'general': [
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
    { stewardId: 'dan-developer', affinityScore: 0.50 },
  ],
  'landing-page-concept': [
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'algorithmic-bias': [
    { stewardId: 'eve-firstwoman', affinityScore: 0.75 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
};
```

**Step 2: Add affinity seeding methods to StewardshipClient**

Add to the `StewardshipClient` class:

```typescript
  async bulkCreateAffinities(
    affinities: Array<{ stewardId: string; contentId: string; affinityScore: number; source: string }>
  ): Promise<{ created: number; errors: string[] }> {
    const response = await this.fetch('/api/v1/steward-affinity/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ affinities }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to bulk create affinities: ${error}`);
    }

    return response.json();
  }
```

**Step 3: Add affinity seeding step to main()**

After the allocation seeding loop (after ~line 557), add:

```typescript
  // Step 9: Seed steward affinities
  console.log();
  console.log('Seeding steward affinities...');

  const affinityInputs: Array<{
    stewardId: string;
    contentId: string;
    affinityScore: number;
    source: string;
  }> = [];

  for (const contentId of contentNeedingAllocations) {
    const category = categoryMap.get(contentId);
    const affinityEntries = category && CATEGORY_AFFINITY_MAP[category]
      ? CATEGORY_AFFINITY_MAP[category]
      : [{ stewardId: 'matthew-dowell', affinityScore: 0.70 }];

    for (const entry of affinityEntries) {
      affinityInputs.push({
        stewardId: entry.stewardId,
        contentId,
        affinityScore: entry.affinityScore,
        source: 'genesis_seed',
      });
    }
  }

  console.log(`   Generated ${affinityInputs.length} affinity records`);

  // Batch seed affinities
  for (let i = 0; i < affinityInputs.length; i += BATCH_SIZE) {
    const batch = affinityInputs.slice(i, i + BATCH_SIZE);
    const batchNum = Math.floor(i / BATCH_SIZE) + 1;
    const totalBatches = Math.ceil(affinityInputs.length / BATCH_SIZE);

    try {
      const result = await client.bulkCreateAffinities(batch);
      console.log(
        `   Affinity batch ${batchNum}/${totalBatches}: ${result.created} created, ${result.errors.length} errors`
      );
    } catch (error) {
      console.error(`   ERROR in affinity batch ${batchNum}: ${error}`);
    }
  }
```

**Step 4: Commit**

```bash
git add genesis/seeder/src/seed-stewardship.ts
git commit -m "feat(genesis): seed steward affinity alongside stewardship allocations"
```

---

### Task 7: Generate TypeScript types and verify end-to-end

**Files:**
- Auto-generated: `elohim/sdk/storage-client-ts/src/generated/`

**Step 1: Export TypeScript bindings**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -5
```

**Step 2: Verify generated types exist**

```bash
ls elohim/sdk/storage-client-ts/src/generated/ | grep -i affinity
```

Expected: `StewardAffinityView.ts`, `CreateStewardAffinityInputView.ts`, `BulkCreateStewardAffinityInputView.ts`

**Step 3: Run all storage tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -10
```

**Step 4: Run clippy**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -10
```

**Step 5: Commit generated types**

```bash
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "chore: regenerate TypeScript types for steward affinity"
```

---

## Increment 2: Mastery Gate + Curation Mutations

### Task 8: Add mastery gate check function

**Files:**
- Modify: `elohim/elohim-storage/src/services/recognition_pipeline_service.rs` (or create new service file)

**Step 1: Write failing test**

Add a new service file `elohim/elohim-storage/src/services/steward_affinity_service.rs`:

```rust
//! Steward affinity service — mastery gate and curation mutation logic
//!
//! The mastery gate ensures only learners who have demonstrated mastery
//! can build stewardship standing through curation acts.

use diesel::SqliteConnection;

use crate::db::content_mastery::{self, MasteryQuery};
use crate::db::models::mastery_levels;
use crate::db::steward_affinity::{self, CreateAffinityInput};
use crate::db::AppContext;
use crate::error::StorageError;

/// Mastery level index threshold for stewardship eligibility.
/// APPLY (index 4) = you can apply the knowledge = ready to steward.
const MASTERY_GATE_THRESHOLD: i32 = 4; // mastery_levels::APPLY

/// Check if a human has reached mastery on content (or any child content).
/// Returns true if any mastery record for this human+content is at or above APPLY level.
pub fn check_mastery_gate(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
    content_id: &str,
) -> Result<bool, StorageError> {
    let mastery_records = content_mastery::list_mastery(
        conn,
        ctx,
        &MasteryQuery {
            human_id: Some(human_id.to_string()),
            content_id: Some(content_id.to_string()),
            ..Default::default()
        },
    )?;

    Ok(mastery_records
        .iter()
        .any(|m| m.mastery_level_index >= MASTERY_GATE_THRESHOLD))
}

/// Curation activity types and their affinity deltas
pub fn curation_delta(activity_type: &str) -> Option<f32> {
    match activity_type {
        "edit" => Some(0.10),
        "review" => Some(0.05),
        "dispute_resolution" => Some(0.15),
        _ => None,
    }
}

/// Record a curation activity and update steward affinity.
/// Returns error if mastery gate is not met.
pub fn record_curation_activity(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    steward_id: &str,
    content_id: &str,
    activity_type: &str,
) -> Result<crate::db::models::StewardAffinity, StorageError> {
    // Validate activity type
    let delta = curation_delta(activity_type).ok_or_else(|| {
        StorageError::InvalidInput(format!("Unknown curation activity type: {}", activity_type))
    })?;

    // Check mastery gate
    if !check_mastery_gate(conn, ctx, steward_id, content_id)? {
        return Err(StorageError::Forbidden(format!(
            "Mastery gate not met: {} has not reached mastery on content {}",
            steward_id, content_id
        )));
    }

    // Map activity to affinity source
    let source = match activity_type {
        "edit" => "curation_edit",
        "review" => "curation_review",
        "dispute_resolution" => "dispute_resolution",
        _ => "curation_edit",
    };

    // Check if affinity record exists; create if first curation act
    let existing =
        steward_affinity::get_affinity_for_steward_content(conn, ctx, steward_id, content_id)?;

    if existing.is_some() {
        steward_affinity::update_affinity_score(conn, ctx, steward_id, content_id, delta, source)
    } else {
        // First curation act after mastery — create initial affinity
        let input = CreateAffinityInput {
            steward_id: steward_id.to_string(),
            content_id: content_id.to_string(),
            affinity_score: delta,
            source: source.to_string(),
        };
        steward_affinity::create_affinity(conn, ctx, &input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curation_delta_edit() {
        assert!((curation_delta("edit").unwrap() - 0.10).abs() < f32::EPSILON);
    }

    #[test]
    fn curation_delta_review() {
        assert!((curation_delta("review").unwrap() - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn curation_delta_dispute() {
        assert!((curation_delta("dispute_resolution").unwrap() - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn curation_delta_unknown_returns_none() {
        assert!(curation_delta("unknown").is_none());
    }
}
```

**Step 2: Register module**

Add `pub mod steward_affinity_service;` to `elohim/elohim-storage/src/services/mod.rs`.

**Step 3: Check `StorageError` has a `Forbidden` variant**

If not, add it to `elohim/elohim-storage/src/error.rs`:

```rust
Forbidden(String),
```

And handle it in the `impl` block that maps errors to HTTP responses (return 403).

**Step 4: Run tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib steward_affinity_service 2>&1 | tail -10
```

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/steward_affinity_service.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): add steward affinity service with mastery gate"
```

---

### Task 9: Add curation event API endpoint

**Files:**
- Modify: `elohim/elohim-storage/src/api/steward_affinity.rs`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

**Step 1: Add curation event view types to views.rs**

```rust
/// Input for recording a curation activity
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CurationEventInputView {
    pub steward_id: String,
    pub content_id: String,
    pub activity_type: String,
}
```

**Step 2: Add handler to steward_affinity.rs API controller**

Add to the match block:

```rust
        (&Method::POST, "curation-event") => handle_curation_event(req, pool, ctx).await,
```

Add handler:

```rust
async fn handle_curation_event(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: CurationEventInputView = parse_body(req).await?;
    let mut conn = get_conn(pool)?;

    let result = crate::services::steward_affinity_service::record_curation_activity(
        &mut conn,
        ctx,
        &input.steward_id,
        &input.content_id,
        &input.activity_type,
    )?;

    let view = StewardAffinityView::from(result);
    Ok(response::created(&view))
}
```

Add import at top: `use crate::views::CurationEventInputView;`

**Step 3: Wire route in http.rs**

```rust
        .route(
            Route::post("/api/v1/steward-affinity/curation-event")
                .handler("steward_affinity_curation_event")
                .auth_required()
                .build(),
        )
```

**Step 4: Build and test**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/steward_affinity.rs elohim/elohim-storage/src/views.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add POST /steward-affinity/curation-event endpoint with mastery gate"
```

---

## Increment 3: Constitutional Limits (Stage 4)

### Task 10: Implement floor/ceiling limits in Stage 4

**Files:**
- Modify: `elohim/elohim-storage/src/services/recognition_pipeline_service.rs`

**Step 1: Write failing tests**

Add to the `tests` module:

```rust
    #[test]
    fn limits_apply_ceiling() {
        // Two stewards: one gets 8.0, one gets 2.0
        // Ceiling at 0.70 (70% of total = 7.0)
        let shares = vec![
            WeightedShare {
                allocation_id: "a1".to_string(),
                steward_presence_id: "p1".to_string(),
                effective_ratio: 0.8,
                share_amount: 8.0,
            },
            WeightedShare {
                allocation_id: "a2".to_string(),
                steward_presence_id: "p2".to_string(),
                effective_ratio: 0.2,
                share_amount: 2.0,
            },
        ];

        let limited = apply_limits_with_config(&shares, 10.0, None, Some(0.70));
        assert_eq!(limited.len(), 2);
        // p1 capped at 7.0, excess 1.0 redistributed to p2
        assert!((limited[0].final_amount - 7.0).abs() < 1e-10);
        assert!((limited[1].final_amount - 3.0).abs() < 1e-10);
        assert!(!limited[0].limit_reasons.is_empty());
    }

    #[test]
    fn limits_apply_floor() {
        let shares = vec![
            WeightedShare {
                allocation_id: "a1".to_string(),
                steward_presence_id: "p1".to_string(),
                effective_ratio: 0.95,
                share_amount: 9.5,
            },
            WeightedShare {
                allocation_id: "a2".to_string(),
                steward_presence_id: "p2".to_string(),
                effective_ratio: 0.05,
                share_amount: 0.5,
            },
        ];

        // Floor at 0.10 (10% of total = 1.0)
        let limited = apply_limits_with_config(&shares, 10.0, Some(0.10), None);
        assert!((limited[1].final_amount - 1.0).abs() < 1e-10);
        assert!((limited[0].final_amount - 9.0).abs() < 1e-10);
    }

    #[test]
    fn limits_passthrough_when_no_config() {
        let shares = vec![
            WeightedShare {
                allocation_id: "a1".to_string(),
                steward_presence_id: "p1".to_string(),
                effective_ratio: 0.7,
                share_amount: 7.0,
            },
        ];

        let limited = apply_limits_with_config(&shares, 10.0, None, None);
        assert!((limited[0].final_amount - 7.0).abs() < f64::EPSILON);
        assert!(limited[0].limit_reasons.is_empty());
    }
```

**Step 2: Run tests to verify they fail**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib recognition_pipeline 2>&1 | tail -10
```

Expected: FAIL — `apply_limits_with_config` not found

**Step 3: Implement `apply_limits_with_config`**

Replace the Stage 4 section with:

```rust
// =============================================================================
// Stage 4: Apply Limits
// =============================================================================

/// Stage 4: Apply floor/ceiling limits to weighted shares.
///
/// - `floor_ratio`: minimum share as ratio of total (e.g., 0.10 = 10%)
/// - `ceiling_ratio`: maximum share as ratio of total (e.g., 0.70 = 70%)
///
/// Excess from ceiling enforcement is redistributed proportionally
/// to non-capped stewards. Floor is applied after ceiling redistribution.
pub fn apply_limits_with_config(
    shares: &[WeightedShare],
    total_amount: f64,
    floor_ratio: Option<f64>,
    ceiling_ratio: Option<f64>,
) -> Vec<LimitedShare> {
    if shares.is_empty() {
        return Vec::new();
    }

    // If no limits configured, passthrough
    if floor_ratio.is_none() && ceiling_ratio.is_none() {
        return apply_limits(shares);
    }

    let mut results: Vec<LimitedShare> = shares
        .iter()
        .map(|s| LimitedShare {
            allocation_id: s.allocation_id.clone(),
            steward_presence_id: s.steward_presence_id.clone(),
            pre_limit_amount: s.share_amount,
            final_amount: s.share_amount,
            limit_reasons: Vec::new(),
        })
        .collect();

    // Apply ceiling
    if let Some(ceiling) = ceiling_ratio {
        let ceiling_amount = total_amount * ceiling;
        let mut excess_total = 0.0;
        let mut capped_indices = Vec::new();

        for (i, result) in results.iter_mut().enumerate() {
            if result.final_amount > ceiling_amount {
                let excess = result.final_amount - ceiling_amount;
                excess_total += excess;
                result.final_amount = ceiling_amount;
                result.limit_reasons.push(LimitReason::CeilingApplied {
                    ceiling: ceiling_amount,
                    excess,
                });
                capped_indices.push(i);
            }
        }

        // Redistribute excess proportionally to non-capped stewards
        if excess_total > 0.0 {
            let uncapped_total: f64 = results
                .iter()
                .enumerate()
                .filter(|(i, _)| !capped_indices.contains(i))
                .map(|(_, r)| r.final_amount)
                .sum();

            if uncapped_total > 0.0 {
                for (i, result) in results.iter_mut().enumerate() {
                    if !capped_indices.contains(&i) {
                        let redistribution =
                            excess_total * (result.final_amount / uncapped_total);
                        result.final_amount += redistribution;
                        if redistribution > f64::EPSILON {
                            result
                                .limit_reasons
                                .push(LimitReason::ExcessRedistributed {
                                    from_steward: "ceiling_excess".to_string(),
                                    amount: redistribution,
                                });
                        }
                    }
                }
            }
        }
    }

    // Apply floor
    if let Some(floor) = floor_ratio {
        let floor_amount = total_amount * floor;

        for result in results.iter_mut() {
            if result.final_amount < floor_amount && result.final_amount > 0.0 {
                let boost = floor_amount - result.final_amount;
                result.limit_reasons.push(LimitReason::FloorApplied {
                    floor: floor_amount,
                    original: result.final_amount,
                });
                result.final_amount = floor_amount;

                // Deduct boost proportionally from others above floor
                let above_floor_total: f64 = results
                    .iter()
                    .filter(|r| {
                        r.steward_presence_id != result.steward_presence_id
                            && r.final_amount > floor_amount
                    })
                    .map(|r| r.final_amount - floor_amount)
                    .sum();

                if above_floor_total > 0.0 {
                    let steward_id = result.steward_presence_id.clone();
                    for other in results.iter_mut() {
                        if other.steward_presence_id != steward_id
                            && other.final_amount > floor_amount
                        {
                            let deduction =
                                boost * ((other.final_amount - floor_amount) / above_floor_total);
                            other.final_amount -= deduction;
                        }
                    }
                }
            }
        }
    }

    results
}

/// Stage 4: Apply floor/ceiling limits to weighted shares.
///
/// v0 implementation: passthrough — final_amount equals share_amount with no
/// limit reasons applied. Use `apply_limits_with_config` for actual enforcement.
pub fn apply_limits(shares: &[WeightedShare]) -> Vec<LimitedShare> {
    shares
        .iter()
        .map(|s| LimitedShare {
            allocation_id: s.allocation_id.clone(),
            steward_presence_id: s.steward_presence_id.clone(),
            pre_limit_amount: s.share_amount,
            final_amount: s.share_amount,
            limit_reasons: Vec::new(),
        })
        .collect()
}
```

**Step 4: Run tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib recognition_pipeline 2>&1 | tail -10
```

Expected: All pass

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/services/recognition_pipeline_service.rs
git commit -m "feat(storage): implement constitutional limits in Stage 4 (floor/ceiling)"
```

---

### Task 11: Wire limits config into distribute pipeline

**Files:**
- Modify: `elohim/elohim-storage/src/services/recognition_pipeline_service.rs`

**Step 1: Add limit config to distribute function**

Update `distribute()` to accept optional limit config and use `apply_limits_with_config` when present:

```rust
/// Limit configuration for constitutional constraints
#[derive(Debug, Clone, Default)]
pub struct LimitConfig {
    pub floor_ratio: Option<f64>,
    pub ceiling_ratio: Option<f64>,
}

/// Run the full recognition distribution pipeline with optional limit config.
pub fn distribute_with_limits(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    trigger: RecognitionTrigger,
    limits: &LimitConfig,
) -> Result<RecognitionDistributionResult, StorageError> {
    // Stage 1-3 same as distribute()...
    let normalized =
        normalize_trigger(&trigger.content_id, &trigger.event_type, trigger.raw_amount);

    if normalized.weighted_amount <= 0.0 {
        return Ok(RecognitionDistributionResult {
            content_id: trigger.content_id,
            trigger_event_type: trigger.event_type,
            raw_amount: trigger.raw_amount,
            weighted_amount: 0.0,
            distributions: vec![],
            economic_event_ids: vec![],
            limits_applied: vec![],
        });
    }

    let stewards = resolve_stewards(conn, ctx, &trigger.content_id)?;

    if stewards.is_empty() {
        return Ok(RecognitionDistributionResult {
            content_id: trigger.content_id,
            trigger_event_type: trigger.event_type,
            raw_amount: trigger.raw_amount,
            weighted_amount: normalized.weighted_amount,
            distributions: vec![],
            economic_event_ids: vec![],
            limits_applied: vec![],
        });
    }

    let shares = apply_weights(&stewards, normalized.weighted_amount);

    // Stage 4: Apply limits if configured
    let limited = if limits.floor_ratio.is_some() || limits.ceiling_ratio.is_some() {
        apply_limits_with_config(
            &shares,
            normalized.weighted_amount,
            limits.floor_ratio,
            limits.ceiling_ratio,
        )
    } else {
        apply_limits(&shares)
    };

    // Stage 5: Settle
    let mut traces = settle(conn, ctx, &limited, &trigger, normalized.weighted_amount)?;

    for (trace, steward) in traces.iter_mut().zip(stewards.iter()) {
        trace.allocation_ratio = steward.allocation_ratio;
        trace.stored_affinity = steward.stored_affinity;
        trace.derived_affinity = steward.derived_affinity;
    }
    for (trace, share) in traces.iter_mut().zip(shares.iter()) {
        trace.effective_ratio = share.effective_ratio;
    }

    let event_ids: Vec<String> = traces
        .iter()
        .filter_map(|t| t.economic_event_id.clone())
        .collect();
    let all_limits: Vec<LimitReason> = limited
        .iter()
        .flat_map(|l| l.limit_reasons.clone())
        .collect();

    Ok(RecognitionDistributionResult {
        content_id: trigger.content_id,
        trigger_event_type: trigger.event_type,
        raw_amount: trigger.raw_amount,
        weighted_amount: normalized.weighted_amount,
        distributions: traces,
        economic_event_ids: event_ids,
        limits_applied: all_limits,
    })
}
```

Update existing `distribute()` to delegate:

```rust
pub fn distribute(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    trigger: RecognitionTrigger,
) -> Result<RecognitionDistributionResult, StorageError> {
    distribute_with_limits(conn, ctx, trigger, &LimitConfig::default())
}
```

**Step 2: Build and run all tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib recognition_pipeline 2>&1 | tail -10
```

**Step 3: Run clippy**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -10
```

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/services/recognition_pipeline_service.rs
git commit -m "feat(storage): wire constitutional limits into recognition distribute pipeline"
```

---

### Task 12: Final verification and type regeneration

**Step 1: Regenerate TypeScript types**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -5
```

**Step 2: Run full test suite**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -20
```

**Step 3: Run clippy and fmt**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -10
cd elohim/elohim-storage && cargo fmt --check 2>&1 | tail -5
```

**Step 4: Commit any generated type changes**

```bash
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "chore: regenerate TypeScript types after steward affinity lifecycle"
```

---

## Reference

### Key Files

| File | Purpose |
|------|---------|
| `elohim/elohim-storage/src/services/recognition_pipeline_service.rs` | Recognition pipeline (5 stages) |
| `elohim/elohim-storage/src/services/steward_affinity_service.rs` | Mastery gate + curation mutations |
| `elohim/elohim-storage/src/db/steward_affinity.rs` | Steward affinity CRUD |
| `elohim/elohim-storage/src/db/stewardship_allocations.rs` | Allocation CRUD (existing) |
| `elohim/elohim-storage/src/db/content_mastery.rs` | Mastery records (existing, used by gate) |
| `elohim/elohim-storage/src/views.rs` | API boundary types |
| `elohim/elohim-storage/src/api/steward_affinity.rs` | HTTP route handlers |
| `elohim/elohim-storage/src/http.rs` | Route wiring |
| `genesis/seeder/src/seed-stewardship.ts` | Genesis seed script |
| `genesis/a2o/features/content/stewardship-allocation.feature` | A2O scenarios |

### Build Commands

```bash
# Build storage
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release

# Test storage
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test

# Lint
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings

# Regenerate TypeScript types
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings

# Run seeder (local dev)
cd genesis/seeder && DOORWAY_URL=http://localhost:8888 npx tsx src/seed-stewardship.ts --dry-run
```

### Design Document

`genesis/plans/2026-03-14-steward-affinity-lifecycle-design.md`
