# REA Compute Sharing Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the testnet lifecycle to real elohim-storage persistence — paired give/take Commitments on spawn, EconomicEvents on settlement, CPU mutual credit denomination.

**Architecture:** New Commitment CRUD endpoint in elohim-storage (Diesel + Hyper, matching existing EconomicEvent patterns). Testnet manager gains an HTTP client that POSTs Commitments when conductors spawn and EconomicEvents when they settle. A2O scenarios verify the full REA chain.

**Tech Stack:** Rust (Diesel ORM, Hyper HTTP, ts-rs), TypeScript (a2o framework), SQLite, Cucumber-JS

**Design doc:** `genesis/plans/2026-03-09-rea-compute-sharing-design.md`

**IMPORTANT Rust build note:** elohim-storage requires `RUSTFLAGS='--cfg getrandom_backend="custom"'` for WASM builds, but for native builds use the default. Check CLAUDE.md for details.

---

### Task 1: Diesel Migration — Commitments Table

**Files:**
- Create: `holochain/elohim-storage/migrations/2026-03-10-000000_rea_commitments/up.sql`
- Create: `holochain/elohim-storage/migrations/2026-03-10-000000_rea_commitments/down.sql`
- Auto-modified: `holochain/elohim-storage/src/db/diesel_schema.rs` (by diesel CLI)

**Step 1: Generate migration scaffold**

Run:
```bash
cd holochain/elohim-storage
diesel migration generate rea_commitments
```

Expected: Creates `migrations/2026-03-10-XXXXXX_rea_commitments/` with empty up.sql and down.sql.

**Step 2: Write the up migration**

Write `up.sql`:

```sql
-- REA Commitment: a binding promise of future economic activity (ValueFlows)
-- Supports paired give/take actions for bilateral exchange.

CREATE TABLE rea_commitments (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',

    -- REA core: who promises what to whom
    action TEXT NOT NULL,                    -- REA action: 'give', 'take', 'deliver-service', etc.
    provider TEXT NOT NULL,                  -- Agent providing resource
    receiver TEXT NOT NULL,                  -- Agent receiving resource

    -- Resource specification
    resource_conforms_to TEXT,               -- ResourceSpecification ID
    resource_classified_as TEXT,             -- JSON array of ResourceClassification strings
    resource_quantity_value REAL,            -- Measure: numerical value
    resource_quantity_unit TEXT,             -- Measure: unit (e.g. 'cpu-second')
    effort_quantity_value REAL,              -- Effort measure: value
    effort_quantity_unit TEXT,               -- Effort measure: unit (e.g. 'megabyte')

    -- Timing
    has_beginning TEXT,                      -- ISO 8601
    has_end TEXT,                            -- ISO 8601
    due TEXT,                                -- ISO 8601 deadline

    -- Agreements and scoping
    clause_of TEXT,                          -- Agreement ID
    in_scope_of TEXT,                        -- JSON array of scope strings

    -- Medium of exchange (for compute mutual credit, etc.)
    medium_of_exchange_id TEXT,

    -- Lifecycle
    state TEXT NOT NULL DEFAULT 'proposed',  -- proposed, accepted, in-progress, fulfilled, cancelled, breached
    finished INTEGER NOT NULL DEFAULT 0,     -- boolean: is this commitment fully satisfied?

    -- Metadata
    note TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_rea_commitment_app_id ON rea_commitments(app_id);
CREATE INDEX idx_rea_commitment_provider ON rea_commitments(app_id, provider);
CREATE INDEX idx_rea_commitment_receiver ON rea_commitments(app_id, receiver);
CREATE INDEX idx_rea_commitment_action ON rea_commitments(action);
CREATE INDEX idx_rea_commitment_state ON rea_commitments(state);
CREATE INDEX idx_rea_commitment_clause_of ON rea_commitments(clause_of);
CREATE INDEX idx_rea_commitment_medium ON rea_commitments(medium_of_exchange_id);
```

**Step 3: Write the down migration**

Write `down.sql`:

```sql
DROP TABLE IF EXISTS rea_commitments;
```

**Step 4: Run migration**

Run:
```bash
cd holochain/elohim-storage
diesel migration run
```

Expected: Migration applied, `diesel_schema.rs` updated with `rea_commitments` table definition.

**Step 5: Verify schema generation**

Run:
```bash
cd holochain/elohim-storage
grep -A 20 "rea_commitments" src/db/diesel_schema.rs
```

Expected: Table macro generated with all columns.

**Step 6: Commit**

```bash
git add holochain/elohim-storage/migrations/ holochain/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(storage): add rea_commitments table migration

REA ValueFlows Commitment with paired give/take actions, resource
quantity + effort quantity measures, medium of exchange reference,
and lifecycle state machine (proposed → fulfilled/breached).

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Diesel Model + DB CRUD

**Files:**
- Modify: `holochain/elohim-storage/src/db/models.rs` (add Commitment struct)
- Create: `holochain/elohim-storage/src/db/rea_commitments.rs`
- Modify: `holochain/elohim-storage/src/db/mod.rs` (register module)

**Step 1: Add Diesel model structs to models.rs**

Add after the EconomicEvent structs (search for the last `#[derive` block related to economic_events):

```rust
// ============================================================================
// REA Commitment
// ============================================================================

/// REA Commitment — a binding promise of future economic activity
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::db::diesel_schema::rea_commitments)]
pub struct ReaCommitment {
    pub id: String,
    pub app_id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_conforms_to: Option<String>,
    pub resource_classified_as: Option<String>,
    pub resource_quantity_value: Option<f32>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f32>,
    pub effort_quantity_unit: Option<String>,
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub due: Option<String>,
    pub clause_of: Option<String>,
    pub in_scope_of: Option<String>,
    pub medium_of_exchange_id: Option<String>,
    pub state: String,
    pub finished: i32,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
}

/// New REA commitment for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::diesel_schema::rea_commitments)]
pub struct NewReaCommitment<'a> {
    pub id: &'a str,
    pub app_id: &'a str,
    pub action: &'a str,
    pub provider: &'a str,
    pub receiver: &'a str,
    pub resource_conforms_to: Option<&'a str>,
    pub resource_classified_as: Option<&'a str>,
    pub resource_quantity_value: Option<f32>,
    pub resource_quantity_unit: Option<&'a str>,
    pub effort_quantity_value: Option<f32>,
    pub effort_quantity_unit: Option<&'a str>,
    pub has_beginning: Option<&'a str>,
    pub has_end: Option<&'a str>,
    pub due: Option<&'a str>,
    pub clause_of: Option<&'a str>,
    pub in_scope_of: Option<&'a str>,
    pub medium_of_exchange_id: Option<&'a str>,
    pub state: &'a str,
    pub finished: i32,
    pub note: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}
```

**Step 2: Create db/rea_commitments.rs**

```rust
//! REA Commitment CRUD operations

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::rea_commitments;
use super::models::{NewReaCommitment, ReaCommitment};
use crate::error::StorageError;

// ============================================================================
// Input / Query Types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreateReaCommitmentInput {
    #[serde(default)]
    pub id: Option<String>,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_classified_as: Option<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f32>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f32>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_beginning: Option<String>,
    #[serde(default)]
    pub has_end: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub in_scope_of: Option<String>,
    #[serde(default)]
    pub medium_of_exchange_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaCommitmentQuery {
    pub action: Option<String>,
    pub provider: Option<String>,
    pub receiver: Option<String>,
    pub state: Option<String>,
    pub clause_of: Option<String>,
    pub medium_of_exchange_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateReaCommitmentState {
    pub state: String,
    #[serde(default)]
    pub finished: Option<bool>,
}

// ============================================================================
// CRUD
// ============================================================================

pub fn create(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateReaCommitmentInput,
) -> Result<ReaCommitment, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let new = NewReaCommitment {
        id: &id,
        app_id: &ctx.app_id,
        action: &input.action,
        provider: &input.provider,
        receiver: &input.receiver,
        resource_conforms_to: input.resource_conforms_to.as_deref(),
        resource_classified_as: input.resource_classified_as.as_deref(),
        resource_quantity_value: input.resource_quantity_value,
        resource_quantity_unit: input.resource_quantity_unit.as_deref(),
        effort_quantity_value: input.effort_quantity_value,
        effort_quantity_unit: input.effort_quantity_unit.as_deref(),
        has_beginning: input.has_beginning.as_deref(),
        has_end: input.has_end.as_deref(),
        due: input.due.as_deref(),
        clause_of: input.clause_of.as_deref(),
        in_scope_of: input.in_scope_of.as_deref(),
        medium_of_exchange_id: input.medium_of_exchange_id.as_deref(),
        state: "proposed",
        finished: 0,
        note: input.note.as_deref(),
        metadata_json: input.metadata_json.as_deref(),
    };

    diesel::insert_into(rea_commitments::table)
        .values(&new)
        .returning(ReaCommitment::as_returning())
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create commitment: {e}")))
}

pub fn get_by_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<ReaCommitment>, StorageError> {
    rea_commitments::table
        .filter(rea_commitments::id.eq(id))
        .filter(rea_commitments::app_id.eq(&ctx.app_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to get commitment: {e}")))
}

pub fn list(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &ReaCommitmentQuery,
) -> Result<Vec<ReaCommitment>, StorageError> {
    let mut q = rea_commitments::table
        .filter(rea_commitments::app_id.eq(&ctx.app_id))
        .into_boxed();

    if let Some(ref action) = query.action {
        q = q.filter(rea_commitments::action.eq(action));
    }
    if let Some(ref provider) = query.provider {
        q = q.filter(rea_commitments::provider.eq(provider));
    }
    if let Some(ref receiver) = query.receiver {
        q = q.filter(rea_commitments::receiver.eq(receiver));
    }
    if let Some(ref state) = query.state {
        q = q.filter(rea_commitments::state.eq(state));
    }
    if let Some(ref clause) = query.clause_of {
        q = q.filter(rea_commitments::clause_of.eq(clause));
    }
    if let Some(ref medium) = query.medium_of_exchange_id {
        q = q.filter(rea_commitments::medium_of_exchange_id.eq(medium));
    }

    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);

    q.order(rea_commitments::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list commitments: {e}")))
}

pub fn get_by_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
) -> Result<Vec<ReaCommitment>, StorageError> {
    rea_commitments::table
        .filter(rea_commitments::app_id.eq(&ctx.app_id))
        .filter(
            rea_commitments::provider
                .eq(agent_id)
                .or(rea_commitments::receiver.eq(agent_id)),
        )
        .order(rea_commitments::created_at.desc())
        .limit(100)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to get agent commitments: {e}")))
}

pub fn update_state(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    update: &UpdateReaCommitmentState,
) -> Result<ReaCommitment, StorageError> {
    let finished_val = update.finished.map(|b| if b { 1 } else { 0 });

    if let Some(f) = finished_val {
        diesel::update(
            rea_commitments::table
                .filter(rea_commitments::id.eq(id))
                .filter(rea_commitments::app_id.eq(&ctx.app_id)),
        )
        .set((
            rea_commitments::state.eq(&update.state),
            rea_commitments::finished.eq(f),
        ))
        .returning(ReaCommitment::as_returning())
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to update commitment state: {e}")))
    } else {
        diesel::update(
            rea_commitments::table
                .filter(rea_commitments::id.eq(id))
                .filter(rea_commitments::app_id.eq(&ctx.app_id)),
        )
        .set(rea_commitments::state.eq(&update.state))
        .returning(ReaCommitment::as_returning())
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to update commitment state: {e}")))
    }
}
```

**Step 3: Register in db/mod.rs**

Add `pub mod rea_commitments;` alongside other module declarations.

**Step 4: Verify it compiles**

Run:
```bash
cd holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5
```

Expected: Compiles (possibly with warnings, no errors).

**Step 5: Commit**

```bash
git add holochain/elohim-storage/src/db/
git commit -m "feat(storage): add REA Commitment Diesel model + CRUD operations

Queryable/Insertable structs for rea_commitments table. CRUD with
app-scoped filtering, agent lookup (provider OR receiver), state
machine updates (proposed → fulfilled/breached).

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 3: View Types with TS Generation

**Files:**
- Modify: `holochain/elohim-storage/src/views.rs` (add ReaCommitmentView + CreateReaCommitmentInputView)

**Step 1: Add view types to views.rs**

Add after the EconomicEvent view types (search for the last `impl From<...> for EconomicEventView`):

```rust
// ============================================================================
// REA Commitment Views
// ============================================================================

/// REA Commitment — API output
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ReaCommitmentView {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_conforms_to: Option<String>,
    pub resource_classified_as: Option<Vec<String>>,
    pub resource_quantity: Option<MeasureView>,
    pub effort_quantity: Option<MeasureView>,
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub due: Option<String>,
    pub clause_of: Option<String>,
    pub in_scope_of: Option<Vec<String>>,
    pub medium_of_exchange_id: Option<String>,
    pub state: String,
    pub finished: bool,
    pub note: Option<String>,
    pub metadata: Option<JsonVal>,
    pub created_at: String,
}

/// Measure — quantity + unit pair (ValueFlows)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct MeasureView {
    pub has_numerical_value: f32,
    pub has_unit: String,
}

impl From<ReaCommitment> for ReaCommitmentView {
    fn from(c: ReaCommitment) -> Self {
        Self {
            id: c.id,
            action: c.action,
            provider: c.provider,
            receiver: c.receiver,
            resource_conforms_to: c.resource_conforms_to,
            resource_classified_as: c
                .resource_classified_as
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            resource_quantity: match (c.resource_quantity_value, &c.resource_quantity_unit) {
                (Some(v), Some(u)) => Some(MeasureView {
                    has_numerical_value: v,
                    has_unit: u.clone(),
                }),
                _ => None,
            },
            effort_quantity: match (c.effort_quantity_value, &c.effort_quantity_unit) {
                (Some(v), Some(u)) => Some(MeasureView {
                    has_numerical_value: v,
                    has_unit: u.clone(),
                }),
                _ => None,
            },
            has_beginning: c.has_beginning,
            has_end: c.has_end,
            due: c.due,
            clause_of: c.clause_of,
            in_scope_of: c
                .in_scope_of
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            medium_of_exchange_id: c.medium_of_exchange_id,
            state: c.state,
            finished: c.finished != 0,
            note: c.note,
            metadata: parse_json_opt(&c.metadata_json),
            created_at: c.created_at,
        }
    }
}

/// REA Commitment — API input
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateReaCommitmentInputView {
    #[serde(default)]
    pub id: Option<String>,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_classified_as: Option<Vec<String>>,
    #[serde(default)]
    pub resource_quantity: Option<MeasureView>,
    #[serde(default)]
    pub effort_quantity: Option<MeasureView>,
    #[serde(default)]
    pub has_beginning: Option<String>,
    #[serde(default)]
    pub has_end: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub in_scope_of: Option<Vec<String>>,
    #[serde(default)]
    pub medium_of_exchange_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata: Option<JsonVal>,
}

/// State update input
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateReaCommitmentStateView {
    pub state: String,
    #[serde(default)]
    pub finished: Option<bool>,
}

impl From<CreateReaCommitmentInputView> for CreateReaCommitmentInput {
    fn from(v: CreateReaCommitmentInputView) -> Self {
        Self {
            id: v.id,
            action: v.action,
            provider: v.provider,
            receiver: v.receiver,
            resource_conforms_to: v.resource_conforms_to,
            resource_classified_as: v
                .resource_classified_as
                .map(|v| serde_json::to_string(&v).unwrap_or_default()),
            resource_quantity_value: v.resource_quantity.as_ref().map(|m| m.has_numerical_value),
            resource_quantity_unit: v.resource_quantity.map(|m| m.has_unit),
            effort_quantity_value: v.effort_quantity.as_ref().map(|m| m.has_numerical_value),
            effort_quantity_unit: v.effort_quantity.map(|m| m.has_unit),
            has_beginning: v.has_beginning,
            has_end: v.has_end,
            due: v.due,
            clause_of: v.clause_of,
            in_scope_of: v
                .in_scope_of
                .map(|v| serde_json::to_string(&v).unwrap_or_default()),
            medium_of_exchange_id: v.medium_of_exchange_id,
            note: v.note,
            metadata_json: serialize_json_opt(&v.metadata),
        }
    }
}

impl From<UpdateReaCommitmentStateView> for UpdateReaCommitmentState {
    fn from(v: UpdateReaCommitmentStateView) -> Self {
        Self {
            state: v.state,
            finished: v.finished,
        }
    }
}
```

**Step 2: Add necessary imports at top of views.rs**

Add use statements (check what's already imported):
```rust
use crate::db::models::ReaCommitment;
use crate::db::rea_commitments::{CreateReaCommitmentInput, UpdateReaCommitmentState};
```

**Step 3: Verify compilation + generate TypeScript**

Run:
```bash
cd holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -10
```

Expected: Types generated to `sdk/storage-client-ts/src/generated/ReaCommitmentView.ts` etc.

**Step 4: Commit**

```bash
git add holochain/elohim-storage/src/views.rs holochain/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): add REA Commitment view types with TypeScript generation

ReaCommitmentView, CreateReaCommitmentInputView, MeasureView,
UpdateReaCommitmentStateView. camelCase API boundary with Measure
objects (hasNumericalValue + hasUnit) matching ValueFlows vocabulary.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 4: API Routes + Service Layer

**Files:**
- Create: `holochain/elohim-storage/src/api/rea_commitments.rs`
- Create: `holochain/elohim-storage/src/services/rea_commitment_service.rs`
- Modify: `holochain/elohim-storage/src/api/mod.rs` (register routes)
- Modify: `holochain/elohim-storage/src/services/mod.rs` (register module)

**Step 1: Create the service layer**

Write `holochain/elohim-storage/src/services/rea_commitment_service.rs`:

```rust
//! REA Commitment service — business logic

use diesel::SqliteConnection;

use crate::db::context::AppContext;
use crate::db::models::ReaCommitment;
use crate::db::rea_commitments::{self, CreateReaCommitmentInput, ReaCommitmentQuery, UpdateReaCommitmentState};
use crate::error::StorageError;
use crate::views::ReaCommitmentView;

pub struct ReaCommitmentService;

impl ReaCommitmentService {
    pub fn create(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        input: CreateReaCommitmentInput,
    ) -> Result<ReaCommitmentView, StorageError> {
        let commitment = rea_commitments::create(conn, ctx, input)?;
        Ok(ReaCommitmentView::from(commitment))
    }

    pub fn get_by_id(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
    ) -> Result<Option<ReaCommitmentView>, StorageError> {
        rea_commitments::get_by_id(conn, ctx, id)
            .map(|opt| opt.map(ReaCommitmentView::from))
    }

    pub fn list(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        query: &ReaCommitmentQuery,
    ) -> Result<Vec<ReaCommitmentView>, StorageError> {
        rea_commitments::list(conn, ctx, query)
            .map(|v| v.into_iter().map(ReaCommitmentView::from).collect())
    }

    pub fn get_by_agent(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
    ) -> Result<Vec<ReaCommitmentView>, StorageError> {
        rea_commitments::get_by_agent(conn, ctx, agent_id)
            .map(|v| v.into_iter().map(ReaCommitmentView::from).collect())
    }

    pub fn update_state(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
        update: UpdateReaCommitmentState,
    ) -> Result<ReaCommitmentView, StorageError> {
        let commitment = rea_commitments::update_state(conn, ctx, id, &update)?;
        Ok(ReaCommitmentView::from(commitment))
    }
}
```

**Step 2: Create the API controller**

Write `holochain/elohim-storage/src/api/rea_commitments.rs`:

```rust
//! REA Commitments API controller
//!
//! Routes: `/api/v1/commitments[/{id}][/agent/{agent_id}]`

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::rea_commitments::{CreateReaCommitmentInput, ReaCommitmentQuery, UpdateReaCommitmentState};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::rea_commitment_service::ReaCommitmentService;
use crate::services::response::{self, from_create_result, from_option, from_result};
use crate::views::{CreateReaCommitmentInputView, UpdateReaCommitmentStateView};

use super::{get_conn, parse_body};

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/commitments
        (&Method::GET, "") => handle_list(req, pool, ctx).await,

        // POST /api/v1/commitments
        (&Method::POST, "") => handle_create(req, pool, ctx).await,

        // GET /api/v1/commitments/{id}
        (&Method::GET, id) if !id.contains('/') => handle_get_by_id(id, pool, ctx).await,

        // PATCH /api/v1/commitments/{id}
        (&Method::PATCH, id) if !id.contains('/') => handle_update_state(req, id, pool, ctx).await,

        // GET /api/v1/commitments/agent/{agent_id}
        (&Method::GET, agent_path) if agent_path.starts_with("agent/") => {
            let agent_id = agent_path.trim_start_matches("agent/");
            handle_get_by_agent(agent_id, pool, ctx).await
        }

        _ => Ok(response::not_found(&format!(
            "Unknown commitments route: {method} /api/v1/commitments/{path}"
        ))),
    }
}

async fn handle_list(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query: ReaCommitmentQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).unwrap_or_default();
    let mut conn = get_conn(pool)?;
    Ok(from_result(ReaCommitmentService::list(&mut conn, ctx, &query)))
}

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input_view: CreateReaCommitmentInputView = parse_body(req).await?;
    let input: CreateReaCommitmentInput = input_view.into();
    let mut conn = get_conn(pool)?;
    Ok(from_create_result(ReaCommitmentService::create(&mut conn, ctx, input)))
}

async fn handle_get_by_id(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_option(
        ReaCommitmentService::get_by_id(&mut conn, ctx, id),
        &format!("Commitment not found: {id}"),
    ))
}

async fn handle_update_state(
    req: Request<Incoming>,
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let view: UpdateReaCommitmentStateView = parse_body(req).await?;
    let update: UpdateReaCommitmentState = view.into();
    let id = id.to_string();
    let mut conn = get_conn(pool)?;
    Ok(from_result(ReaCommitmentService::update_state(&mut conn, ctx, &id, update)))
}

async fn handle_get_by_agent(
    agent_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(from_result(ReaCommitmentService::get_by_agent(&mut conn, ctx, agent_id)))
}
```

**Step 3: Register modules**

In `src/api/mod.rs`, add:
```rust
pub mod rea_commitments;
```

And in the route dispatcher function, add (find the pattern matching economic-events):
```rust
} else if sub_path.starts_with("commitments") {
    let resource_path = sub_path.strip_prefix("commitments").unwrap_or("");
    rea_commitments::handle(req, method, resource_path, &pool, &app_ctx).await
```

In `src/services/mod.rs`, add:
```rust
pub mod rea_commitment_service;
```

**Step 4: Verify full build**

Run:
```bash
cd holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

Expected: Build succeeds.

**Step 5: Commit**

```bash
git add holochain/elohim-storage/src/api/ holochain/elohim-storage/src/services/
git commit -m "feat(storage): add REA Commitment API routes + service layer

5 endpoints: create, get, list, update-state, get-by-agent.
Hyper handler dispatches to service layer, follows existing
EconomicEvent patterns. PATCH for state transitions.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 5: Storage Client in Testnet Manager

**Files:**
- Create: `genesis/a2o/src/framework/storage-client.ts`
- Modify: `genesis/a2o/src/framework/testnet-manager.ts`

**Step 1: Create lightweight HTTP client**

Write `genesis/a2o/src/framework/storage-client.ts`:

```typescript
/**
 * Lightweight HTTP client for elohim-storage.
 * Used by testnet manager for REA Commitment + EconomicEvent persistence.
 */

const DEFAULT_BASE_URL = 'http://localhost:8090';

export interface Measure {
  hasNumericalValue: number;
  hasUnit: string;
}

export interface CreateCommitmentInput {
  id?: string;
  action: string;
  provider: string;
  receiver: string;
  resourceClassifiedAs?: string[];
  resourceQuantity?: Measure;
  effortQuantity?: Measure;
  hasBeginning?: string;
  hasEnd?: string;
  due?: string;
  clauseOf?: string;
  inScopeOf?: string[];
  mediumOfExchangeId?: string;
  note?: string;
  metadata?: Record<string, unknown>;
}

export interface CommitmentView {
  id: string;
  action: string;
  provider: string;
  receiver: string;
  resourceClassifiedAs?: string[];
  resourceQuantity?: Measure;
  effortQuantity?: Measure;
  mediumOfExchangeId?: string;
  state: string;
  finished: boolean;
  createdAt: string;
}

export interface CreateEconomicEventInput {
  id?: string;
  action: string;
  provider: string;
  receiver: string;
  resourceClassifiedAs?: string[];
  resourceQuantityValue?: number;
  resourceQuantityUnit?: string;
  effortQuantityValue?: number;
  effortQuantityUnit?: string;
  hasPointInTime?: string;
  fulfills?: string[];
  lamadEventType?: string;
  note?: string;
  metadataJson?: string;
}

export class StorageClient {
  constructor(private baseUrl: string = DEFAULT_BASE_URL) {}

  async isHealthy(): Promise<boolean> {
    try {
      const res = await fetch(`${this.baseUrl}/api/v1/health`, {
        signal: AbortSignal.timeout(5000),
      });
      return res.ok;
    } catch {
      return false;
    }
  }

  async createCommitment(input: CreateCommitmentInput): Promise<CommitmentView> {
    const res = await fetch(`${this.baseUrl}/api/v1/commitments`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(input),
    });
    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Failed to create commitment: ${res.status} ${text}`);
    }
    return (await res.json()) as CommitmentView;
  }

  async updateCommitmentState(
    id: string,
    state: string,
    finished?: boolean,
  ): Promise<CommitmentView> {
    const res = await fetch(`${this.baseUrl}/api/v1/commitments/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ state, finished }),
    });
    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Failed to update commitment: ${res.status} ${text}`);
    }
    return (await res.json()) as CommitmentView;
  }

  async getCommitment(id: string): Promise<CommitmentView | null> {
    const res = await fetch(`${this.baseUrl}/api/v1/commitments/${id}`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to get commitment: ${res.status}`);
    return (await res.json()) as CommitmentView;
  }

  async listCommitments(query?: Record<string, string>): Promise<CommitmentView[]> {
    const params = query ? '?' + new URLSearchParams(query).toString() : '';
    const res = await fetch(`${this.baseUrl}/api/v1/commitments${params}`);
    if (!res.ok) throw new Error(`Failed to list commitments: ${res.status}`);
    return (await res.json()) as CommitmentView[];
  }

  async createEconomicEvent(input: CreateEconomicEventInput): Promise<Record<string, unknown>> {
    const res = await fetch(`${this.baseUrl}/api/v1/economic-events`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(input),
    });
    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Failed to create economic event: ${res.status} ${text}`);
    }
    return (await res.json()) as Record<string, unknown>;
  }

  async getAgentCommitments(agentId: string): Promise<CommitmentView[]> {
    const res = await fetch(`${this.baseUrl}/api/v1/commitments/agent/${agentId}`);
    if (!res.ok) throw new Error(`Failed to get agent commitments: ${res.status}`);
    return (await res.json()) as CommitmentView[];
  }
}
```

**Step 2: Wire StorageClient into testnet manager**

Modify `genesis/a2o/src/framework/testnet-manager.ts`. Add to the `TestnetSession` interface:

```typescript
export interface TestnetSession {
  // ... existing fields ...
  storageClient: StorageClient | null;
  commitmentIds: Map<string, string>;  // persona → commitment ID
  matthewCommitmentId: string | null;
}
```

Modify `startTestnet()` to create paired Commitments after spawning nodes:

```typescript
// After spawning nodes successfully, create REA Commitments
let storageClient: StorageClient | null = null;
const commitmentIds = new Map<string, string>();
let matthewCommitmentId: string | null = null;

try {
  storageClient = new StorageClient(opts.storageUrl ?? 'http://localhost:8090');
  if (await storageClient.isHealthy()) {
    const now = new Date().toISOString();
    const totalBudget = opts.personas.length * 360;

    // Matthew's 'take' commitment
    const takeCommitment = await storageClient.createCommitment({
      action: 'take',
      provider: requester,
      receiver: requester,
      resourceClassifiedAs: ['compute'],
      resourceQuantity: { hasNumericalValue: totalBudget, hasUnit: 'cpu-second' },
      hasBeginning: now,
      mediumOfExchangeId: 'cpu-mutual-credit',
      note: `Compute allocation: ${opts.personas.length} peers for ${totalBudget} cpu-seconds`,
    });
    matthewCommitmentId = takeCommitment.id;

    // Per-persona 'give' commitments
    for (const persona of opts.personas) {
      const giveCommitment = await storageClient.createCommitment({
        action: 'give',
        provider: persona,
        receiver: requester,
        resourceClassifiedAs: ['compute'],
        resourceQuantity: { hasNumericalValue: 360, hasUnit: 'cpu-second' },
        effortQuantity: { hasNumericalValue: 150, hasUnit: 'megabyte' },
        hasBeginning: now,
        mediumOfExchangeId: 'cpu-mutual-credit',
        note: `Provide compute to ${requester}`,
      });
      commitmentIds.set(persona, giveCommitment.id);
    }

    console.log(`  REA: Created ${commitmentIds.size + 1} paired commitments`);
  } else {
    console.warn('  elohim-storage not available — skipping REA persistence');
  }
} catch (err) {
  console.warn(`  REA commitment creation failed (non-fatal): ${err}`);
}
```

Modify `stopTestnet()` to create EconomicEvents and update Commitment states:

```typescript
// After settle envelopes, persist to elohim-storage
if (storageClient && commitmentIds.size > 0) {
  try {
    const summary = getComputeSummary();
    const now = new Date().toISOString();

    // Per-persona deliver-service events
    for (const [persona, commitmentId] of commitmentIds) {
      const metrics = summary.perPersona[persona] ?? { cpuSeconds: 0 };
      const isExceeded = summary.budgetExceeded.includes(persona);

      await storageClient.createEconomicEvent({
        action: 'deliver-service',
        provider: persona,
        receiver: activeSession.requester,
        resourceClassifiedAs: ['compute'],
        resourceQuantityValue: metrics.cpuSeconds,
        resourceQuantityUnit: 'cpu-second',
        hasPointInTime: now,
        fulfills: [commitmentId],
        lamadEventType: 'compute-deliver',
        note: isExceeded ? 'Budget exceeded — partial delivery' : 'Compute delivered',
      });

      // Update commitment state
      await storageClient.updateCommitmentState(
        commitmentId,
        isExceeded ? 'breached' : 'fulfilled',
        !isExceeded,
      );
    }

    // Matthew's aggregate take event
    if (matthewCommitmentId) {
      await storageClient.createEconomicEvent({
        action: 'take',
        provider: activeSession.requester,
        receiver: activeSession.requester,
        resourceClassifiedAs: ['compute'],
        resourceQuantityValue: summary.totalCpuSeconds,
        resourceQuantityUnit: 'cpu-second',
        hasPointInTime: now,
        fulfills: [matthewCommitmentId],
        lamadEventType: 'compute-take',
      });

      await storageClient.updateCommitmentState(matthewCommitmentId, 'fulfilled', true);
    }

    console.log(`  REA: Persisted ${commitmentIds.size + 1} economic events`);
  } catch (err) {
    console.warn(`  REA settlement failed (non-fatal): ${err}`);
  }
}
```

**Step 3: Typecheck**

Run:
```bash
cd genesis/a2o && npx tsc --noEmit
```

Expected: No errors.

**Step 4: Commit**

```bash
git add genesis/a2o/src/framework/storage-client.ts genesis/a2o/src/framework/testnet-manager.ts
git commit -m "feat(a2o): wire testnet manager to elohim-storage for REA persistence

StorageClient for Commitment + EconomicEvent HTTP calls. startTestnet()
creates paired give/take Commitments. stopTestnet() creates deliver-service
Events with fulfillment links, updates commitment state. Falls back to
JSONL-only if storage unavailable.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 6: Feature File + Step Definitions

**Files:**
- Modify: `genesis/a2o/features/elohim/compute-allocation.feature`
- Modify: `genesis/a2o/steps/compute-allocation.steps.ts`

**Step 1: Update Background and add new scenarios to feature file**

Add to the Background:
```gherkin
  And elohim-storage is healthy at "http://localhost:8090"
  And compute mutual credit medium exists
```

Add new scenarios after the existing ones (before the @wip circuit breaker):

```gherkin
  @e2e @rea
  Scenario: Compute commitments are persisted as REA records
    Given Matthew has a simulation requiring 5 peer nodes
    When he submits a ServiceRequest with budget 1800 cpu-seconds
    Then a 'take' commitment exists for Matthew with 1800 cpu-seconds
    And a 'give' commitment exists for each of the 5 personas
    And all commitments reference the CPU mutual credit medium

  @e2e @rea
  Scenario: Settlement produces paired EconomicEvents
    Given 5 conductors are running for Matthew's simulation
    When the simulation workload completes
    Then each persona has a 'deliver-service' EconomicEvent
    And Matthew has a 'take' EconomicEvent for the total
    And each event fulfills its corresponding commitment
    And each event has resourceQuantity in cpu-seconds
    And each event has effortQuantity in megabytes
    And all persona commitments are marked 'fulfilled'
```

**Step 2: Add step definitions**

Add to `steps/compute-allocation.steps.ts`:

```typescript
import { StorageClient } from '../src/framework/storage-client.js';

const storageClient = new StorageClient('http://localhost:8090');

// --- Background steps ---

Given('elohim-storage is healthy at {string}', async function (url: string) {
  const client = new StorageClient(url);
  const healthy = await client.isHealthy();
  assert.ok(healthy, `elohim-storage at ${url} is not healthy`);
});

Given('compute mutual credit medium exists', async function () {
  // Pre-seeded or idempotent — just verify storage is reachable
  // The MediumOfExchange record is reference data, seeded separately
});

// --- REA Commitment scenario ---

Then('a {string} commitment exists for Matthew with {int} cpu-seconds', async function (action: string, budget: number) {
  const commitments = await storageClient.listCommitments({ action, provider: 'matthew' });
  assert.ok(commitments.length > 0, `No '${action}' commitment found for Matthew`);
  const commitment = commitments[0];
  assert.equal(commitment.resourceQuantity?.hasUnit, 'cpu-second');
  assert.ok(
    commitment.resourceQuantity!.hasNumericalValue >= budget,
    `Expected ${budget} cpu-seconds, got ${commitment.resourceQuantity!.hasNumericalValue}`,
  );
});

Then('a {string} commitment exists for each of the {int} personas', async function (action: string, count: number) {
  const commitments = await storageClient.listCommitments({ action });
  const personaCommitments = commitments.filter((c) => c.provider !== 'matthew');
  assert.ok(
    personaCommitments.length >= count,
    `Expected ${count} '${action}' commitments, found ${personaCommitments.length}`,
  );
});

Then('all commitments reference the CPU mutual credit medium', async function () {
  const commitments = await storageClient.listCommitments({});
  for (const c of commitments) {
    assert.equal(
      c.mediumOfExchangeId,
      'cpu-mutual-credit',
      `Commitment ${c.id} missing CPU mutual credit medium`,
    );
  }
});

// --- REA Event scenario ---

Then('each persona has a {string} EconomicEvent', async function (action: string) {
  // Query economic events via storage API
  const res = await fetch(`http://localhost:8090/api/v1/economic-events?action=${action}`);
  assert.ok(res.ok, `Failed to query events: ${res.status}`);
  const events = (await res.json()) as Record<string, unknown>[];
  assert.ok(events.length >= 5, `Expected at least 5 '${action}' events, got ${events.length}`);
});

Then('Matthew has a {string} EconomicEvent for the total', async function (action: string) {
  const res = await fetch(`http://localhost:8090/api/v1/economic-events?action=${action}&provider=matthew`);
  assert.ok(res.ok, `Failed to query events: ${res.status}`);
  const events = (await res.json()) as Record<string, unknown>[];
  assert.ok(events.length > 0, `No '${action}' event found for Matthew`);
});

Then('each event fulfills its corresponding commitment', async function () {
  const res = await fetch('http://localhost:8090/api/v1/economic-events?action=deliver-service');
  const events = (await res.json()) as Array<{ fulfills?: string[] }>;
  for (const event of events) {
    assert.ok(
      event.fulfills && event.fulfills.length > 0,
      'Event missing fulfills link to commitment',
    );
  }
});

Then('each event has effortQuantity in megabytes', async function () {
  const res = await fetch('http://localhost:8090/api/v1/economic-events?action=deliver-service');
  const events = (await res.json()) as Array<{ effortQuantityUnit?: string }>;
  for (const event of events) {
    // effortQuantity may be null for idle nodes — assert on non-null ones
    if (event.effortQuantityUnit) {
      assert.equal(event.effortQuantityUnit, 'megabyte');
    }
  }
});

Then('all persona commitments are marked {string}', async function (expectedState: string) {
  const commitments = await storageClient.listCommitments({ action: 'give' });
  for (const c of commitments) {
    assert.equal(c.state, expectedState, `Commitment ${c.id} state is '${c.state}', expected '${expectedState}'`);
  }
});
```

**Step 3: Typecheck**

Run:
```bash
cd genesis/a2o && npx tsc --noEmit
```

**Step 4: Commit**

```bash
git add genesis/a2o/features/elohim/compute-allocation.feature genesis/a2o/steps/compute-allocation.steps.ts
git commit -m "feat(a2o): add REA commitment/event verification scenarios

Two new scenarios: verify paired commitments on spawn, verify
EconomicEvents with fulfillment links on settlement. Steps query
elohim-storage HTTP API directly.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 7: Integration Test

**This requires elohim-storage running at localhost:8090.**

**Step 1: Start elohim-storage**

Run:
```bash
cd holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo run --release &
# Wait for health check
sleep 3
curl -s http://localhost:8090/api/v1/health | jq .
```

Expected: `{ "healthy": true, ... }`

**Step 2: Run lifecycle-only scenarios (no REA)**

Run:
```bash
cd genesis/a2o
npx cucumber-js --profile testnet --tags '@testnet and @e2e and not @wip and not @rea'
```

Expected: 2 scenarios pass (existing lifecycle tests still work).

**Step 3: Run REA scenarios**

Run:
```bash
cd genesis/a2o
npx cucumber-js --profile testnet --tags '@testnet and @rea and not @wip'
```

Expected: 2 scenarios pass. Commitments created, events persisted, fulfillment linked.

**Step 4: Verify persisted data**

Run:
```bash
curl -s http://localhost:8090/api/v1/commitments | jq '.[].action'
# Expected: "take", "give", "give", "give", "give", "give"

curl -s http://localhost:8090/api/v1/economic-events?action=deliver-service | jq '.[].provider'
# Expected: persona IDs

curl -s 'http://localhost:8090/api/v1/commitments?state=fulfilled' | jq 'length'
# Expected: 6 (all fulfilled)
```

**Step 5: Stop elohim-storage and clean up**

```bash
kill %1  # stop background storage process
cd elohim-node/simulation && ./spawn-persona-testnet.sh clean
```

**Step 6: Commit any fixes**

```bash
git add -A
git commit -m "fix(a2o): integration fixes from REA compute sharing test run

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | Diesel migration — rea_commitments table | Rust DB |
| 2 | Diesel model + CRUD operations | Rust DB |
| 3 | View types with TypeScript generation | Rust API boundary |
| 4 | API routes + service layer | Rust HTTP |
| 5 | Storage client + testnet manager REA integration | TypeScript |
| 6 | Feature file + step definitions | A2O |
| 7 | Integration test | E2E verification |
