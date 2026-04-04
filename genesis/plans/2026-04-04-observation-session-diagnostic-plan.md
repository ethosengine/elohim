# Observation Session Diagnostic System — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add protocol-native observation sessions to elohim-storage so a2o tests (and future consumers) can activate request-level diagnostics and receive a correlated report as a ContentNode.

**Architecture:** Storage owns observation state via two operational SQLite tables. An `X-Observation-Id` header acts as a cross-cutting aspect — storage middleware auto-appends entries for each observed request. Doorway forwards the header and contributes its own entries for errors it generates before reaching storage. The report endpoint composes, persists as a Content node, and returns the artifact.

**Tech Stack:** Rust (Diesel/SQLite, hyper), TypeScript (a2o cucumber framework, undici)

**Spec:** `genesis/plans/2026-04-04-observation-session-diagnostic-design.md`

---

## File Map

### elohim-storage (Rust)

| File | Action | Responsibility |
|------|--------|----------------|
| `migrations/2026-04-04-120000_observation_sessions/up.sql` | Create | DDL for observation_sessions + observation_entries tables |
| `migrations/2026-04-04-120000_observation_sessions/down.sql` | Create | DROP tables |
| `src/db/observation_sessions.rs` | Create | CRUD functions: begin, append_entry, get_entries, close |
| `src/db/models.rs` | Modify | Add Diesel Queryable/Insertable structs |
| `src/db/mod.rs` | Modify | Add `pub mod observation_sessions;` |
| `src/schema.rs` | Auto | Diesel auto-generates from migration |
| `src/views.rs` | Modify | Add InputView and View types (camelCase) |
| `src/http.rs` | Modify | Add route handlers, observation middleware aspect, manifest entries |

### doorway (Rust)

| File | Action | Responsibility |
|------|--------|----------------|
| `src/routes/storage_proxy.rs` | Modify | Forward X-Observation-Id header to storage |
| `src/server/http.rs` | Modify | Contribute observation entries on doorway-originated errors |

### genesis/a2o (TypeScript)

| File | Action | Responsibility |
|------|--------|----------------|
| `src/framework/api/doorway-client.ts` | Modify | Add observationId property, begin/report methods, header injection |
| `steps/common.steps.ts` | Modify | Before hook starts observation, After hook fetches report |

---

## Task 1: Diesel Migration

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-04-120000_observation_sessions/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-04-120000_observation_sessions/down.sql`

- [ ] **Step 1: Create migration directory**

```bash
mkdir -p elohim/elohim-storage/migrations/2026-04-04-120000_observation_sessions
```

- [ ] **Step 2: Write up.sql**

```sql
-- Source of truth: SQLite (operational, Category C).
-- Sessions and entries are ephemeral working data — purgeable after report composition.
-- The composed report is persisted as a Content node (Category A, dht_anchor_hash via content table).
-- No dht_anchor_hash on these tables: observations are not notarized.

CREATE TABLE observation_sessions (
    -- Source of truth: SQLite (operational)
    id TEXT PRIMARY KEY NOT NULL,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    ttl_seconds INTEGER NOT NULL DEFAULT 300,
    source TEXT NOT NULL,
    metadata_json TEXT,
    report_content_id TEXT
);

CREATE INDEX idx_obs_sessions_started ON observation_sessions(started_at);
CREATE INDEX idx_obs_sessions_source ON observation_sessions(source);

CREATE TABLE observation_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES observation_sessions(id),
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    origin TEXT NOT NULL,
    category TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'info',
    method TEXT,
    path TEXT,
    status_code INTEGER,
    message TEXT NOT NULL,
    context_json TEXT
);

CREATE INDEX idx_obs_entries_session ON observation_entries(session_id);
```

- [ ] **Step 3: Write down.sql**

```sql
DROP TABLE IF EXISTS observation_entries;
DROP TABLE IF EXISTS observation_sessions;
```

- [ ] **Step 4: Run migration**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

Expected: Build succeeds. Diesel auto-generates schema updates.

- [ ] **Step 5: Verify schema.rs was updated**

Check that `elohim/elohim-storage/src/schema.rs` contains `observation_sessions` and `observation_entries` table definitions.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-04-120000_observation_sessions/
git add elohim/elohim-storage/src/schema.rs
git commit -m "feat(storage): add observation_sessions migration"
```

---

## Task 2: Diesel Models

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs`

- [ ] **Step 1: Add Queryable struct for ObservationSession**

Add after the existing model structs (near the end of the file):

```rust
// ============================================================================
// Observation Sessions (Operational — Category C)
// ============================================================================

/// Observation session row from SQLite.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::observation_sessions)]
pub struct ObservationSession {
    pub id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub ttl_seconds: i32,
    pub source: String,
    pub metadata_json: Option<String>,
    pub report_content_id: Option<String>,
}

/// New observation session for INSERT.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::observation_sessions)]
pub struct NewObservationSession<'a> {
    pub id: &'a str,
    pub ttl_seconds: i32,
    pub source: &'a str,
    pub metadata_json: Option<&'a str>,
}

/// Observation entry row from SQLite.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::observation_entries)]
pub struct ObservationEntry {
    pub id: i32,
    pub session_id: String,
    pub timestamp: String,
    pub origin: String,
    pub category: String,
    pub severity: String,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<i32>,
    pub message: String,
    pub context_json: Option<String>,
}

/// New observation entry for INSERT.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::observation_entries)]
pub struct NewObservationEntry<'a> {
    pub session_id: &'a str,
    pub origin: &'a str,
    pub category: &'a str,
    pub severity: &'a str,
    pub method: Option<&'a str>,
    pub path: Option<&'a str>,
    pub status_code: Option<i32>,
    pub message: &'a str,
    pub context_json: Option<&'a str>,
}
```

- [ ] **Step 2: Verify build**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/db/models.rs
git commit -m "feat(storage): add observation session Diesel models"
```

---

## Task 3: DB Functions

**Files:**
- Create: `elohim/elohim-storage/src/db/observation_sessions.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

- [ ] **Step 1: Add module to mod.rs**

In `elohim/elohim-storage/src/db/mod.rs`, add:

```rust
pub mod observation_sessions;
```

- [ ] **Step 2: Write observation_sessions.rs**

```rust
//! Observation session CRUD — operational (Category C).
//!
//! Sessions and entries are ephemeral working data for protocol-native diagnostics.
//! The composed report is persisted as a Content node (Category A).

use diesel::prelude::*;
use uuid::Uuid;

use crate::db::models::{
    NewObservationEntry, NewObservationSession, ObservationEntry, ObservationSession,
};
use crate::schema::{observation_entries, observation_sessions};
use crate::StorageError;

/// Begin a new observation session.
pub fn begin_session(
    conn: &mut SqliteConnection,
    source: &str,
    ttl_seconds: i32,
    metadata_json: Option<&str>,
) -> Result<ObservationSession, StorageError> {
    let id = Uuid::new_v4().to_string();

    let new = NewObservationSession {
        id: &id,
        ttl_seconds,
        source,
        metadata_json,
    };

    diesel::insert_into(observation_sessions::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert session failed: {}", e)))?;

    get_session(conn, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created session".into()))
}

/// Get a session by ID.
pub fn get_session(
    conn: &mut SqliteConnection,
    session_id: &str,
) -> Result<Option<ObservationSession>, StorageError> {
    observation_sessions::table
        .filter(observation_sessions::id.eq(session_id))
        .first::<ObservationSession>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query session failed: {}", e)))
}

/// Check if a session is active (exists, not ended, not expired).
pub fn is_session_active(
    conn: &mut SqliteConnection,
    session_id: &str,
) -> Result<bool, StorageError> {
    let session = get_session(conn, session_id)?;
    match session {
        Some(s) if s.ended_at.is_some() => Ok(false),
        Some(_) => Ok(true), // TODO: check TTL expiry against started_at
        None => Ok(false),
    }
}

/// Append an observation entry to a session.
pub fn append_entry(
    conn: &mut SqliteConnection,
    session_id: &str,
    origin: &str,
    category: &str,
    severity: &str,
    method: Option<&str>,
    path: Option<&str>,
    status_code: Option<i32>,
    message: &str,
    context_json: Option<&str>,
) -> Result<(), StorageError> {
    let new = NewObservationEntry {
        session_id,
        origin,
        category,
        severity,
        method,
        path,
        status_code,
        message,
        context_json,
    };

    diesel::insert_into(observation_entries::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert entry failed: {}", e)))?;

    Ok(())
}

/// Append multiple entries in a single transaction.
pub fn append_entries_batch(
    conn: &mut SqliteConnection,
    entries: &[NewObservationEntry<'_>],
) -> Result<(), StorageError> {
    diesel::insert_into(observation_entries::table)
        .values(entries)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Batch insert entries failed: {}", e)))?;

    Ok(())
}

/// Get all entries for a session, ordered by timestamp.
pub fn get_entries(
    conn: &mut SqliteConnection,
    session_id: &str,
) -> Result<Vec<ObservationEntry>, StorageError> {
    observation_entries::table
        .filter(observation_entries::session_id.eq(session_id))
        .order(observation_entries::timestamp.asc())
        .load::<ObservationEntry>(conn)
        .map_err(|e| StorageError::Internal(format!("Query entries failed: {}", e)))
}

/// Close a session (set ended_at) and record the report content ID.
pub fn close_session(
    conn: &mut SqliteConnection,
    session_id: &str,
    report_content_id: &str,
) -> Result<(), StorageError> {
    diesel::update(observation_sessions::table.filter(observation_sessions::id.eq(session_id)))
        .set((
            observation_sessions::ended_at.eq(chrono::Utc::now().to_rfc3339()),
            observation_sessions::report_content_id.eq(report_content_id),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Close session failed: {}", e)))?;

    Ok(())
}

/// Purge entries for a closed session.
pub fn purge_entries(
    conn: &mut SqliteConnection,
    session_id: &str,
) -> Result<usize, StorageError> {
    diesel::delete(
        observation_entries::table.filter(observation_entries::session_id.eq(session_id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Purge entries failed: {}", e)))
}
```

- [ ] **Step 3: Verify build**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/db/observation_sessions.rs elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): add observation session DB functions"
```

---

## Task 4: View Types

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1: Add InputView and response View types**

Add at the end of `views.rs`:

```rust
// ============================================================================
// Observation Sessions — Views
// ============================================================================

/// Input for POST /api/v1/observations/begin
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginObservationInputView {
    pub source: String,
    #[serde(default = "default_obs_ttl")]
    pub ttl_seconds: i32,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn default_obs_ttl() -> i32 {
    300
}

/// Response for POST /api/v1/observations/begin
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginObservationResponseView {
    pub session_id: String,
    pub expires_at: String,
}

/// Input for POST /api/v1/observations/{id}/entries
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationEntryInputView {
    pub origin: String,
    pub category: String,
    #[serde(default = "default_severity")]
    pub severity: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub status_code: Option<i32>,
    pub message: String,
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}

fn default_severity() -> String {
    "info".to_string()
}

/// An entry in the observation report.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationEntryView {
    pub timestamp: String,
    pub origin: String,
    pub category: String,
    pub severity: String,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status_code: Option<i32>,
    pub message: String,
    pub context: Option<serde_json::Value>,
}

impl From<crate::db::models::ObservationEntry> for ObservationEntryView {
    fn from(e: crate::db::models::ObservationEntry) -> Self {
        Self {
            timestamp: e.timestamp,
            origin: e.origin,
            category: e.category,
            severity: e.severity,
            method: e.method,
            path: e.path,
            status_code: e.status_code,
            message: e.message,
            context: e
                .context_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

/// A correlated issue in the observation report.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationIssueView {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub entry_count: usize,
    pub related_content_ids: Vec<String>,
    pub suggested_cause: String,
}

/// The composed observation report.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationReportView {
    pub content_id: String,
    pub session_id: String,
    pub source: String,
    pub metadata: Option<serde_json::Value>,
    pub duration: ObservationDurationView,
    pub summary: ObservationSummaryView,
    pub issues: Vec<ObservationIssueView>,
    pub system_state: ObservationSystemStateView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDurationView {
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSummaryView {
    pub total_entries: usize,
    pub by_origin: std::collections::HashMap<String, usize>,
    pub by_severity: std::collections::HashMap<String, usize>,
    pub by_category: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSystemStateView {
    pub storage_healthy: bool,
    pub conductor_connected: bool,
    pub p2p_peer_count: usize,
}
```

- [ ] **Step 2: Verify build**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "feat(storage): add observation session view types"
```

---

## Task 5: Route Handlers + Observation Middleware

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`

This is the largest task. It adds three route handlers and the observation middleware aspect.

- [ ] **Step 1: Add route dispatch in handle_request()**

In the main `handle_request()` function (around line 356), add a new match arm for `/api/v1/observations/` **before** the generic `/api/v1/` handler:

```rust
// Observation sessions
(method, p) if p.starts_with("/api/v1/observations/") || p == "/api/v1/observations" => {
    if let Some(ref pool) = self.db_pool {
        self.handle_observation_request(req, method, &path).await
    } else {
        Ok(response::service_unavailable("Database not available"))
    }
}
```

- [ ] **Step 2: Add the observation request dispatcher**

Add a new method to the `HttpServer` impl block:

```rust
/// Route dispatcher for /api/v1/observations/*
async fn handle_observation_request(
    &self,
    req: Request<Incoming>,
    method: Method,
    path: &str,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let sub = path
        .strip_prefix("/api/v1/observations")
        .unwrap_or("");

    // POST /api/v1/observations/begin
    if sub == "/begin" && method == Method::POST {
        return self.handle_observation_begin(req).await;
    }

    // Strip leading slash for ID extraction
    let sub = sub.strip_prefix('/').unwrap_or(sub);

    // POST /api/v1/observations/{id}/entries
    if let Some(session_id) = sub.strip_suffix("/entries") {
        if method == Method::POST {
            return self.handle_observation_add_entries(req, session_id).await;
        }
        return Ok(response::method_not_allowed());
    }

    // GET /api/v1/observations/{id}/report
    if let Some(session_id) = sub.strip_suffix("/report") {
        if method == Method::GET {
            return self.handle_observation_report(session_id).await;
        }
        return Ok(response::method_not_allowed());
    }

    Ok(response::not_found("Unknown observation endpoint"))
}
```

- [ ] **Step 3: Implement handle_observation_begin**

```rust
/// POST /api/v1/observations/begin — start a new observation session.
async fn handle_observation_begin(
    &self,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body = req
        .collect()
        .await
        .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
    let input: crate::views::BeginObservationInputView =
        serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

    let metadata_str = input
        .metadata
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default());

    let mut conn = self.get_diesel_conn()?;
    let session = crate::db::observation_sessions::begin_session(
        &mut conn,
        &input.source,
        input.ttl_seconds,
        metadata_str.as_deref(),
    )?;

    let expires_at = chrono::NaiveDateTime::parse_from_str(&session.started_at, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt + chrono::Duration::seconds(session.ttl_seconds as i64))
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let resp = crate::views::BeginObservationResponseView {
        session_id: session.id,
        expires_at,
    };

    Ok(response::created(&resp))
}
```

- [ ] **Step 4: Implement handle_observation_add_entries**

```rust
/// POST /api/v1/observations/{id}/entries — append entries (doorway contribution).
async fn handle_observation_add_entries(
    &self,
    req: Request<Incoming>,
    session_id: &str,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = self.get_diesel_conn()?;

    if !crate::db::observation_sessions::is_session_active(&mut conn, session_id)? {
        return Ok(response::not_found(&format!("Session {} not found or inactive", session_id)));
    }

    let body = req
        .collect()
        .await
        .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
    let bytes = body.to_bytes();

    // Accept single entry or array
    let inputs: Vec<crate::views::ObservationEntryInputView> =
        if bytes.starts_with(b"[") {
            serde_json::from_slice(&bytes)
                .map_err(|e| StorageError::Parse(format!("Invalid JSON array: {}", e)))?
        } else {
            let single: crate::views::ObservationEntryInputView =
                serde_json::from_slice(&bytes)
                    .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
            vec![single]
        };

    for input in &inputs {
        let ctx_str = input.context.as_ref().map(|v| serde_json::to_string(v).unwrap_or_default());
        crate::db::observation_sessions::append_entry(
            &mut conn,
            session_id,
            &input.origin,
            &input.category,
            &input.severity,
            input.method.as_deref(),
            input.path.as_deref(),
            input.status_code,
            &input.message,
            ctx_str.as_deref(),
        )?;
    }

    Ok(Response::builder()
        .status(hyper::StatusCode::CREATED)
        .body(Full::new(Bytes::new()))
        .unwrap())
}
```

- [ ] **Step 5: Implement handle_observation_report**

```rust
/// GET /api/v1/observations/{id}/report — compose and persist the report.
async fn handle_observation_report(
    &self,
    session_id: &str,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = self.get_diesel_conn()?;

    let session = crate::db::observation_sessions::get_session(&mut conn, session_id)?
        .ok_or_else(|| StorageError::NotFound(format!("Session {} not found", session_id)))?;

    // Idempotent: if already reported, return existing report
    if let Some(ref content_id) = session.report_content_id {
        // Fetch the existing content node and return it
        if let Ok(Some(content)) = crate::db::content::get_content_by_id(&mut conn, content_id) {
            if let Some(body) = &content.content_body {
                let report: crate::views::ObservationReportView = serde_json::from_str(body)
                    .map_err(|e| StorageError::Internal(format!("Stored report parse failed: {}", e)))?;
                return Ok(response::ok(&report));
            }
        }
    }

    let entries = crate::db::observation_sessions::get_entries(&mut conn, session_id)?;
    let now = chrono::Utc::now().to_rfc3339();

    // Build summary
    let mut by_origin: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut by_severity: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut by_category: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for e in &entries {
        *by_origin.entry(e.origin.clone()).or_default() += 1;
        *by_severity.entry(e.severity.clone()).or_default() += 1;
        *by_category.entry(e.category.clone()).or_default() += 1;
    }

    // Correlate issues
    let issues = correlate_issues(&entries);

    // Build system state snapshot
    let system_state = crate::views::ObservationSystemStateView {
        storage_healthy: true,
        conductor_connected: self.services.is_some(),
        p2p_peer_count: 0, // TODO: read from P2P health if available
    };

    let metadata: Option<serde_json::Value> = session
        .metadata_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let duration_ms = chrono::NaiveDateTime::parse_from_str(&session.started_at, "%Y-%m-%d %H:%M:%S")
        .ok()
        .and_then(|start| {
            chrono::DateTime::parse_from_rfc3339(&now).ok().map(|end| {
                (end.naive_utc() - start).num_milliseconds()
            })
        })
        .unwrap_or(0);

    // Generate a content ID for the report
    let content_id = format!("obs-report-{}", session_id);

    let report = crate::views::ObservationReportView {
        content_id: content_id.clone(),
        session_id: session_id.to_string(),
        source: session.source.clone(),
        metadata,
        duration: crate::views::ObservationDurationView {
            started_at: session.started_at.clone(),
            ended_at: now.clone(),
            duration_ms,
        },
        summary: crate::views::ObservationSummaryView {
            total_entries: entries.len(),
            by_origin,
            by_severity,
            by_category,
        },
        issues,
        system_state,
    };

    // Persist as content node
    let report_json = serde_json::to_string(&report)
        .map_err(|e| StorageError::Internal(format!("Report serialization failed: {}", e)))?;

    let scenario_name = report.metadata
        .as_ref()
        .and_then(|m| m.get("scenario"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let content_input = crate::db::content::CreateContentInput {
        id: Some(content_id.clone()),
        content_type: "observation-report".to_string(),
        content_format: "json".to_string(),
        title: format!("Observation: {}", scenario_name),
        description: Some(format!(
            "{} entries, {} errors",
            report.summary.total_entries,
            report.summary.by_severity.get("error").unwrap_or(&0)
        )),
        content_body: Some(report_json),
        tags: Some(vec![
            "observation-report".to_string(),
            report.source.clone(),
        ]),
        ..Default::default()
    };

    let _ = crate::db::content::upsert_content(&mut conn, content_input);

    // Close session and record report content ID
    crate::db::observation_sessions::close_session(&mut conn, session_id, &content_id)?;

    // Purge entries (working data no longer needed)
    crate::db::observation_sessions::purge_entries(&mut conn, session_id)?;

    Ok(response::ok(&report))
}
```

- [ ] **Step 6: Implement issue correlation**

Add as a free function in `http.rs` (or a separate module if preferred):

```rust
/// Correlate observation entries into issues by grouping on status code + path prefix.
fn correlate_issues(
    entries: &[crate::db::models::ObservationEntry],
) -> Vec<crate::views::ObservationIssueView> {
    use std::collections::HashMap;

    let error_entries: Vec<_> = entries
        .iter()
        .filter(|e| e.severity == "error" || e.severity == "warning")
        .collect();

    if error_entries.is_empty() {
        return vec![];
    }

    // Group by (status_code, path_prefix)
    let mut groups: HashMap<String, Vec<&crate::db::models::ObservationEntry>> = HashMap::new();

    for e in &error_entries {
        let key = match (e.status_code, e.path.as_deref()) {
            (Some(401), _) => "auth-401".to_string(),
            (Some(403), _) => "auth-403".to_string(),
            (Some(404), Some(p)) => {
                // Extract content ID from path if possible
                let content_id = extract_content_id_from_path(p);
                format!("not-found-{}", content_id.unwrap_or_else(|| p.to_string()))
            }
            (Some(405), Some(p)) => format!("method-not-allowed-{}", p),
            (Some(503), _) => "service-unavailable".to_string(),
            (Some(code), _) => format!("http-{}", code),
            (None, _) => format!("category-{}", e.category),
        };
        groups.entry(key).or_default().push(e);
    }

    groups
        .into_iter()
        .map(|(id, group)| {
            let first = group[0];
            let content_ids: Vec<String> = group
                .iter()
                .filter_map(|e| e.path.as_deref().and_then(extract_content_id_from_path))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            let (title, cause) = suggest_cause(&id, &group);

            crate::views::ObservationIssueView {
                id,
                category: first.category.clone(),
                severity: first.severity.clone(),
                title,
                entry_count: group.len(),
                related_content_ids: content_ids,
                suggested_cause: cause,
            }
        })
        .collect()
}

/// Extract a content ID from a request path like /db/content/{id} or /db/allocations/content/{id}.
fn extract_content_id_from_path(path: &str) -> Option<String> {
    if let Some(rest) = path.strip_prefix("/db/content/") {
        return Some(rest.split('/').next().unwrap_or(rest).to_string());
    }
    if let Some(rest) = path.strip_prefix("/db/allocations/content/") {
        return Some(rest.split('/').next().unwrap_or(rest).to_string());
    }
    None
}

/// Generate a human-readable title and suggested cause for an issue group.
fn suggest_cause(
    issue_id: &str,
    entries: &[&crate::db::models::ObservationEntry],
) -> (String, String) {
    let count = entries.len();

    if issue_id == "auth-401" {
        return (
            format!("Auth failed for {} request(s) — credentials invalid or humans not registered", count),
            "Fixture humans may not be registered on this environment. Check if registration endpoint is functional (imagodei DNA must be installed).".to_string(),
        );
    }
    if issue_id == "service-unavailable" {
        let has_imagodei = entries.iter().any(|e| {
            e.message.contains("imagodei")
        });
        if has_imagodei {
            return (
                "Identity service unavailable — imagodei DNA missing".to_string(),
                "Conductor only has lamad DNA installed. Install imagodei DNA to enable registration and identity.".to_string(),
            );
        }
        return (
            format!("Service unavailable for {} request(s)", count),
            "One or more backend services are not responding.".to_string(),
        );
    }
    if issue_id.starts_with("not-found-") {
        let content = issue_id.strip_prefix("not-found-").unwrap_or("unknown");
        return (
            format!("Content '{}' not found — {} lookup(s) failed", content, count),
            format!("Content '{}' is not seeded in this environment.", content),
        );
    }
    if issue_id.starts_with("method-not-allowed-") {
        let path = issue_id.strip_prefix("method-not-allowed-").unwrap_or("unknown");
        return (
            format!("Method not allowed on {}", path),
            "Route may not support this HTTP method. Check manifest route registration.".to_string(),
        );
    }

    // Generic fallback
    (
        format!("{} error(s) in category '{}'", count, entries[0].category),
        "Review the individual entries for details.".to_string(),
    )
}
```

- [ ] **Step 7: Add the observation middleware aspect**

This is the core of the aspect pattern. Add a helper method that the main `handle_request()` calls after producing a response:

```rust
/// Observation middleware: if X-Observation-Id is present, append an entry for this request.
fn maybe_observe_request(
    &self,
    req_method: &str,
    req_path: &str,
    status_code: u16,
    observation_id: &str,
    response_body_hint: Option<&str>,
) {
    // Only observe non-2xx or explicitly interesting responses
    let severity = match status_code {
        200..=299 => return, // Don't observe successful requests
        400..=499 => "warning",
        _ => "error",
    };

    let category = if req_path.starts_with("/auth") {
        "auth"
    } else if req_path.starts_with("/db/content") || req_path.starts_with("/api/v1/cache") {
        "content"
    } else if req_path.starts_with("/db/events") || req_path.starts_with("/api/v1/economic") {
        "http"
    } else {
        "http"
    };

    let message = format!("{} {} → {}", req_method, req_path, status_code);

    let context = response_body_hint.map(|hint| {
        serde_json::json!({ "responseHint": hint }).to_string()
    });

    if let Ok(mut conn) = self.get_diesel_conn() {
        let _ = crate::db::observation_sessions::append_entry(
            &mut conn,
            observation_id,
            "storage",
            category,
            severity,
            Some(req_method),
            Some(req_path),
            Some(status_code as i32),
            &message,
            context.as_deref(),
        );
    }
}
```

Then in `handle_request()`, after the main match produces a response, add the observation hook:

```rust
// After the main match block, before returning the response:
if let Some(obs_id) = req.headers().get("x-observation-id").and_then(|h| h.to_str().ok()) {
    let status = response.status().as_u16();
    self.maybe_observe_request(
        req.method().as_str(),
        req.uri().path(),
        status,
        obs_id,
        None, // response body hint — could extract from error responses
    );
}
```

Note: You'll need to capture the method and path before the match consumes the request. The existing code already does this (`let path = req.uri().path().to_string(); let method = req.method().clone();`).

- [ ] **Step 8: Register in manifest**

In `build_manifest()`, add the observation routes:

```rust
// =====================================================================
// /api/v1/observations — Protocol-native diagnostics
// =====================================================================
.route(
    Route::post("/api/v1/observations/begin")
        .handler("begin_observation")
        .build(),
)
.route(
    Route::post("/api/v1/observations/{id}/entries")
        .handler("add_observation_entries")
        .build(),
)
.route(
    Route::get("/api/v1/observations/{id}/report")
        .handler("get_observation_report")
        .build(),
)
```

- [ ] **Step 9: Verify build**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -10
```

- [ ] **Step 10: Commit**

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add observation session endpoints and middleware aspect"
```

---

## Task 6: Doorway Header Forwarding + Contribution

**Files:**
- Modify: `doorway/doorway-service/src/routes/storage_proxy.rs`
- Modify: `doorway/doorway-service/src/server/http.rs`

- [ ] **Step 1: Forward X-Observation-Id in storage_proxy.rs**

In `forward_to_storage()` (around line 67, after the `authorization` header block), add:

```rust
// Forward observation session header if present
if let Some(obs_id) = req.headers().get("x-observation-id") {
    if let Ok(obs_str) = obs_id.to_str() {
        builder = builder.header("X-Observation-Id", obs_str);
    }
}
```

- [ ] **Step 2: Add observation contribution on doorway-originated errors**

In `http.rs`, add a helper function:

```rust
/// Fire-and-forget: contribute a doorway observation entry to storage.
fn maybe_contribute_observation(
    observation_id: &str,
    storage_url: &str,
    origin_category: &str,
    severity: &str,
    method: &str,
    path: &str,
    status_code: u16,
    message: &str,
) {
    let url = format!(
        "{}/api/v1/observations/{}/entries",
        storage_url.trim_end_matches('/'),
        observation_id
    );
    let body = serde_json::json!({
        "origin": "doorway",
        "category": origin_category,
        "severity": severity,
        "method": method,
        "path": path,
        "statusCode": status_code,
        "message": message
    });

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let _ = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await;
    });
}
```

- [ ] **Step 3: Hook contribution into the request handler**

In `handle_request()`, after the main match block produces a response, add:

```rust
// Contribute doorway observations for errors that doorway generated (not proxied from storage)
if let Some(obs_id) = req_headers.get("x-observation-id").and_then(|h| h.to_str().ok()) {
    let status = response.status().as_u16();
    // Only contribute for doorway-originated errors (not proxied storage errors)
    if status >= 400 && !was_proxied_to_storage {
        if let Some(storage_url) = &state.storage_url {
            maybe_contribute_observation(
                obs_id,
                storage_url,
                "route",
                if status >= 500 { "error" } else { "warning" },
                method.as_str(),
                &path,
                status,
                &format!("Doorway returned {} before reaching storage", status),
            );
        }
    }
}
```

Note: You'll need to track whether the request was proxied to storage. A simple boolean flag set when `forward_to_storage` is called works. Alternatively, contribute for ALL non-2xx responses and let the deduplication in report composition handle it (storage will also observe if it received the request).

- [ ] **Step 4: Verify build**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo build --release 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/routes/storage_proxy.rs doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): forward X-Observation-Id and contribute doorway observations"
```

---

## Task 7: A2O DoorwayClient Integration

**Files:**
- Modify: `genesis/a2o/src/framework/api/doorway-client.ts`

- [ ] **Step 1: Add observationId property and header injection**

Add property after the constructor:

```typescript
export class DoorwayClient {
  /** Active observation session ID, if any. */
  observationId: string | null = null;

  constructor(
    private readonly baseUrl: string,
    private token?: string
  ) {}
```

Update `headers()` to include the observation ID:

```typescript
private headers(): Record<string, string> {
  const h: Record<string, string> = {};
  if (this.token) h['authorization'] = `Bearer ${this.token}`;
  if (this.observationId) h['x-observation-id'] = this.observationId;
  return h;
}
```

- [ ] **Step 2: Add begin and report methods**

Add after the existing public methods:

```typescript
// ===========================================================================
// Observation Sessions
// ===========================================================================

/**
 * Begin an observation session. All subsequent requests will carry
 * X-Observation-Id and the infrastructure will auto-observe.
 */
async beginObservation(metadata?: Record<string, unknown>): Promise<string> {
  const resp = await this.post<{ sessionId: string; expiresAt: string }>(
    '/api/v1/observations/begin',
    { source: 'a2o', ttlSeconds: 300, metadata }
  );
  this.observationId = resp.sessionId;
  return resp.sessionId;
}

/**
 * Fetch the composed observation report and clear the session.
 */
async getObservationReport(): Promise<ObservationReport> {
  if (!this.observationId) {
    throw new Error('No active observation session');
  }
  const report = await this.get<ObservationReport>(
    `/api/v1/observations/${this.observationId}/report`
  );
  this.observationId = null;
  return report;
}
```

- [ ] **Step 3: Add the ObservationReport type**

Add at the top of the file or in a separate types section:

```typescript
export interface ObservationReport {
  contentId: string;
  sessionId: string;
  source: string;
  metadata?: Record<string, unknown>;
  duration: {
    startedAt: string;
    endedAt: string;
    durationMs: number;
  };
  summary: {
    totalEntries: number;
    byOrigin: Record<string, number>;
    bySeverity: Record<string, number>;
    byCategory: Record<string, number>;
  };
  issues: Array<{
    id: string;
    category: string;
    severity: string;
    title: string;
    entryCount: number;
    relatedContentIds: string[];
    suggestedCause: string;
  }>;
  systemState: {
    storageHealthy: boolean;
    conductorConnected: boolean;
    p2pPeerCount: number;
  };
}
```

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/src/framework/api/doorway-client.ts
git commit -m "feat(a2o): add observation session support to DoorwayClient"
```

---

## Task 8: A2O Before/After Hook Integration

**Files:**
- Modify: `genesis/a2o/steps/common.steps.ts`

- [ ] **Step 1: Add observation start to Before hook**

Update the Before hook:

```typescript
Before(async function (this: E2EWorld, scenario) {
  // Clear Playwright capture state
  for (const [, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        device.clearCapture();
      }
    }
  }

  // Start observation session if a doorway is registered
  for (const [, doorway] of this.doorways) {
    try {
      await doorway.client.beginObservation({
        scenario: scenario.pickle.name,
        tags: scenario.pickle.tags.map(t => t.name),
        feature: scenario.pickle.uri,
      });
    } catch {
      // Observation is best-effort — don't block scenarios if storage is down
    }
    break; // Only observe on the first doorway
  }
});
```

- [ ] **Step 2: Add observation report to After hook**

Add observation report collection after the existing browser error checks, before `runCleanup()`:

```typescript
After(async function (this: E2EWorld, scenario) {
  // ... existing failure artifact capture ...

  if (scenario.result?.status === Status.FAILED) {
    const safeName = scenario.pickle.name.replace(/[^a-zA-Z0-9]/g, '-');

    for (const [name, human] of this.humans) {
      for (const device of human.devices) {
        if (device instanceof PlaywrightDevice) {
          await captureFailureArtifacts(device, safeName, name);
        }
      }
    }
  }

  // ... existing browser error assertion ...

  if (scenario.result?.status === Status.PASSED) {
    const errorReport = collectBrowserErrors(this, scenario.pickle.name);
    if (errorReport.length) {
      throw new Error(
        `Scenario passed but had ${errorReport.length} browser error(s):\n` +
          errorReport.map(e => `  ${e}`).join('\n')
      );
    }
  }

  // Collect observation report (works for both HTTP and Playwright modes)
  const safeName = scenario.pickle.name.replace(/[^a-zA-Z0-9]/g, '-');
  for (const [, doorway] of this.doorways) {
    if (doorway.client.observationId) {
      try {
        const report = await doorway.client.getObservationReport();
        const errorCount = report.summary.bySeverity['error'] ?? 0;
        if (errorCount > 0 || scenario.result?.status === Status.FAILED) {
          mkdirSync('reports/observations', { recursive: true });
          writeFileSync(
            `reports/observations/${safeName}.json`,
            JSON.stringify(report, null, 2)
          );
        }
      } catch {
        // Best-effort — don't mask the real test result
      }
    }
    break;
  }

  await this.runCleanup();
});
```

- [ ] **Step 3: Add import for mkdirSync**

Ensure `mkdirSync` is imported at the top of `common.steps.ts`:

```typescript
import { mkdirSync, writeFileSync } from 'node:fs';
```

- [ ] **Step 4: Verify a2o builds**

```bash
cd genesis/a2o && npx tsc --noEmit 2>&1 | tail -10
```

Expected: No type errors.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/steps/common.steps.ts
git commit -m "feat(a2o): activate observation sessions in Before/After hooks"
```

---

## Task 9: Smoke Test

- [ ] **Step 1: Run a2o delivery tests and verify observation reports are generated**

```bash
cd genesis/a2o && npm run test:delivery 2>&1 | tail -30
```

- [ ] **Step 2: Check for observation report files**

```bash
ls -la genesis/a2o/reports/observations/ 2>/dev/null
```

Expected: JSON files for failed scenarios.

- [ ] **Step 3: Inspect one report for correct structure**

```bash
cat genesis/a2o/reports/observations/*.json | head -1 | python3 -m json.tool | head -30
```

Expected: Report with `sessionId`, `summary`, `issues`, `systemState` fields.

- [ ] **Step 4: Final commit with any fixes**

```bash
git add -A
git commit -m "feat: observation session diagnostic system — smoke tested"
```
