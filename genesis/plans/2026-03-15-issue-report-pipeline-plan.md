# Issue Report Pipeline — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a "Report Issue" entry to the governance feedback menu that silently collects diagnostics (logs, environment, health, route context), sends them through the gate conversation, and persists a structured issue report via a new `/db/issue-reports` endpoint.

**Architecture:** New `issue_reports` table + Diesel model + Rust CRUD + gated API route in elohim-storage. New `DiagnosticCollectorService` in Angular gathers runtime context. Extend `FeedbackType` to include `'report'`, wire modal to collect diagnostics on open and pass them through `contextMetadata`. Ensure `StorageApiService.handleError()` logs to `LoggerService` so HTTP failures appear in diagnostic bundles.

**Tech Stack:** Rust (Diesel ORM, hyper, serde, ts-rs), Angular 19 (signals, OnPush, Vitest), TypeScript type generation

**Design doc:** `genesis/plans/2026-03-15-issue-report-pipeline-design.md`

---

### Task 1: Diesel Migration — issue_reports table

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-15-200000_issue_reports/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-15-200000_issue_reports/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (auto-updated by diesel migration run)

**Step 1: Create migration directory**

```bash
mkdir -p elohim/elohim-storage/migrations/2026-03-15-200000_issue_reports
```

**Step 2: Write up.sql**

```sql
CREATE TABLE issue_reports (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    human_id TEXT NOT NULL,
    summary TEXT,
    description TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'bug',
    severity TEXT NOT NULL DEFAULT 'info',
    diagnostics TEXT NOT NULL,
    context_url TEXT,
    environment TEXT,
    avodah_context TEXT,
    resolution_status TEXT NOT NULL DEFAULT 'open',
    linked_github_url TEXT,
    linked_work_story_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_issue_reports_human_id ON issue_reports (human_id);
CREATE INDEX idx_issue_reports_resolution_status ON issue_reports (resolution_status);
CREATE INDEX idx_issue_reports_category ON issue_reports (category);
CREATE INDEX idx_issue_reports_app_id ON issue_reports (app_id);
```

**Step 3: Write down.sql**

```sql
DROP TABLE IF EXISTS issue_reports;
```

**Step 4: Run migration**

Run: `cd elohim/elohim-storage && diesel migration run`
Expected: Migration applied successfully. `diesel_schema.rs` updated with `issue_reports` table.

**Step 5: Verify diesel_schema.rs**

Check that `issue_reports` table block appears in `src/db/diesel_schema.rs`. Add `issue_reports` to the `allow_tables_to_appear_in_same_query!` macro if not already present.

**Step 6: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-03-15-200000_issue_reports/
git add elohim/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(storage): add issue_reports table migration"
```

---

### Task 2: Diesel models — IssueReport + NewIssueReport

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs`

**Step 1: Add model structs**

Append after the Comment models section (after `NewComment`):

```rust
// ============================================================================
// Issue Report Models
// ============================================================================

/// Issue report from the issue_reports table (Queryable)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = issue_reports)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct IssueReport {
    pub id: String,
    pub app_id: String,
    pub human_id: String,
    pub summary: Option<String>,
    pub description: String,
    pub category: String,
    pub severity: String,
    pub diagnostics: String,
    pub context_url: Option<String>,
    pub environment: Option<String>,
    pub avodah_context: Option<String>,
    pub resolution_status: String,
    pub linked_github_url: Option<String>,
    pub linked_work_story_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// New issue report for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = issue_reports)]
pub struct NewIssueReport<'a> {
    pub id: String,
    pub app_id: String,
    pub human_id: String,
    pub summary: Option<&'a str>,
    pub description: &'a str,
    pub category: &'a str,
    pub severity: &'a str,
    pub diagnostics: &'a str,
    pub context_url: Option<&'a str>,
    pub environment: Option<&'a str>,
    pub avodah_context: Option<&'a str>,
    pub resolution_status: &'a str,
    pub linked_github_url: Option<&'a str>,
    pub linked_work_story_id: Option<&'a str>,
    pub created_at: String,
    pub updated_at: String,
}
```

Also add `use super::diesel_schema::issue_reports;` at the top of the models file if not auto-imported.

**Step 2: Build to verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add elohim/elohim-storage/src/db/models.rs
git commit -m "feat(storage): add IssueReport and NewIssueReport diesel models"
```

---

### Task 3: View types — IssueReportView + CreateIssueReportInputView

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

**Step 1: Add view types**

Append after the Comment Views section:

```rust
// ============================================================================
// Issue Report Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct IssueReportView {
    pub id: String,
    pub human_id: String,
    pub summary: Option<String>,
    pub description: String,
    pub category: String,
    pub severity: String,
    pub diagnostics: Value,
    pub context_url: Option<String>,
    pub environment: Option<Value>,
    pub avodah_context: Option<Value>,
    pub resolution_status: String,
    pub linked_github_url: Option<String>,
    pub linked_work_story_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::db::models::IssueReport> for IssueReportView {
    fn from(r: crate::db::models::IssueReport) -> Self {
        Self {
            id: r.id,
            human_id: r.human_id,
            summary: r.summary,
            description: r.description,
            category: r.category,
            severity: r.severity,
            diagnostics: parse_json(&r.diagnostics),
            context_url: r.context_url,
            environment: r.environment.as_deref().map(parse_json),
            avodah_context: r.avodah_context.as_deref().map(parse_json),
            resolution_status: r.resolution_status,
            linked_github_url: r.linked_github_url,
            linked_work_story_id: r.linked_work_story_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateIssueReportInputView {
    pub description: String,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub diagnostics: Value,
    pub context_url: Option<String>,
    pub environment: Option<Value>,
    pub avodah_context: Option<Value>,
}
```

**Important:** Check whether `parse_json` (non-optional version) exists in views.rs. If only `parse_json_opt` exists, use:
```rust
diagnostics: serde_json::from_str(&r.diagnostics).unwrap_or(Value::Null),
```

**Step 2: Build to verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "feat(storage): add IssueReportView and CreateIssueReportInputView types"
```

---

### Task 4: CRUD functions — issue_reports

**Files:**
- Create: `elohim/elohim-storage/src/db/issue_reports.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` (add `pub mod issue_reports;`)

**Step 1: Create the CRUD module**

Follow the same pattern as `comments.rs`:

```rust
//! Issue Reports CRUD operations
//!
//! Diagnostic reports filed by humans through the gate feedback pipeline.

use diesel::prelude::*;
use serde::Deserialize;
use tracing::debug;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::issue_reports;
use super::models::{current_timestamp, IssueReport, NewIssueReport};
use crate::error::StorageError;

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating an issue report
#[derive(Debug, Clone, Deserialize)]
pub struct CreateIssueReportInput {
    pub human_id: String,
    pub description: String,
    pub category: String,
    pub severity: String,
    pub diagnostics: String,
    pub context_url: Option<String>,
    pub environment: Option<String>,
    pub avodah_context: Option<String>,
}

/// Query parameters for listing issue reports
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReportQuery {
    pub category: Option<String>,
    pub severity: Option<String>,
    pub resolution_status: Option<String>,
    pub human_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Create a new issue report
pub fn create_issue_report(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: &CreateIssueReportInput,
) -> Result<IssueReport, StorageError> {
    let id = Uuid::new_v4().to_string();
    let now = current_timestamp();

    let new = NewIssueReport {
        id: id.clone(),
        app_id: ctx.app_id().to_string(),
        human_id: input.human_id.clone(),
        summary: None,
        description: &input.description,
        category: &input.category,
        severity: &input.severity,
        diagnostics: &input.diagnostics,
        context_url: input.context_url.as_deref(),
        environment: input.environment.as_deref(),
        avodah_context: input.avodah_context.as_deref(),
        resolution_status: "open",
        linked_github_url: None,
        linked_work_story_id: None,
        created_at: now.clone(),
        updated_at: now,
    };

    diesel::insert_into(issue_reports::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create issue report: {}", e)))?;

    debug!("Created issue report {}", id);

    get_issue_report(conn, ctx, &id)
}

/// Get a single issue report by ID
pub fn get_issue_report(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<IssueReport, StorageError> {
    issue_reports::table
        .filter(issue_reports::id.eq(id))
        .filter(issue_reports::app_id.eq(ctx.app_id()))
        .first::<IssueReport>(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => StorageError::NotFound(id.to_string()),
            _ => StorageError::Internal(format!("Failed to get issue report: {}", e)),
        })
}

/// List issue reports with optional filters
pub fn list_issue_reports(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &IssueReportQuery,
) -> Result<Vec<IssueReport>, StorageError> {
    let mut q = issue_reports::table
        .filter(issue_reports::app_id.eq(ctx.app_id()))
        .order(issue_reports::created_at.desc())
        .into_boxed();

    if let Some(category) = &query.category {
        q = q.filter(issue_reports::category.eq(category));
    }
    if let Some(severity) = &query.severity {
        q = q.filter(issue_reports::severity.eq(severity));
    }
    if let Some(status) = &query.resolution_status {
        q = q.filter(issue_reports::resolution_status.eq(status));
    }
    if let Some(human_id) = &query.human_id {
        q = q.filter(issue_reports::human_id.eq(human_id));
    }
    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }
    if let Some(offset) = query.offset {
        q = q.offset(offset);
    }

    q.load::<IssueReport>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list issue reports: {}", e)))
}
```

**Step 2: Add module to db/mod.rs**

Add `pub mod issue_reports;` in the module list.

**Step 3: Build to verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/db/issue_reports.rs
git add elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): add issue_reports CRUD module"
```

---

### Task 5: API routes — POST and GET for issue-reports

**Files:**
- Create: `elohim/elohim-storage/src/api/issue_reports.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs` (add module + route dispatch)

**Step 1: Create the API controller**

Follow the same pattern as `api/comments.rs`. POST is gated through ElohimGate. GET endpoints are ungated.

```rust
//! Issue Reports API controller
//!
//! Routes: `/api/v1/issue-reports[/{id}]`
//!
//! POST is gated through ElohimGate (MutationType::IssueReport).
//! GET endpoints are ungated.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::issue_reports::{CreateIssueReportInput, IssueReportQuery};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::elohim_gate::{GateResult, MutationType};
use crate::services::response;
use crate::services::Services;
use crate::views::{CreateIssueReportInputView, IssueReportView};

use super::{get_conn, parse_body};

/// Handle `/api/v1/issue-reports*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        // POST /api/v1/issue-reports — gated create
        (&Method::POST, "") => handle_create(req, pool, ctx, services).await,

        // GET /api/v1/issue-reports — list with query params
        (&Method::GET, "") => handle_list(req, pool, ctx).await,

        // GET /api/v1/issue-reports/{id}
        (&Method::GET, id) if !id.is_empty() => handle_get(id, pool, ctx).await,

        _ => response::not_found(&format!(
            "Unknown issue-reports route: {} /api/v1/issue-reports/{}",
            method, path
        )),
    })
}

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Response<Full<Bytes>> {
    let input_view: CreateIssueReportInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body for create issue report"),
    };

    if input_view.description.trim().is_empty() {
        return response::bad_request("description must not be empty");
    }

    let human_id = "self".to_string();

    // Gate evaluation — uses MutationType::Comment for now (issue reports are
    // a form of community feedback). When a dedicated MutationType::IssueReport
    // variant is added, switch to that.
    let (gate_result, gate_view) = super::evaluate_gate(
        &services,
        pool,
        ctx,
        MutationType::Comment,
        serde_json::json!({
            "description": input_view.description,
            "category": input_view.category,
            "diagnostics": input_view.diagnostics,
        }),
        Some(&human_id),
    )
    .await;

    match &gate_result {
        GateResult::Pause {
            prompt,
            confirm_token,
            ..
        } => {
            return response::json_response(
                hyper::StatusCode::CONFLICT,
                &serde_json::json!({
                    "gate": gate_view,
                    "pausePrompt": prompt,
                    "confirmToken": confirm_token,
                }),
            );
        }
        GateResult::Settlement {
            boundary,
            appeal_path,
            ..
        } => {
            return response::forbidden(&serde_json::json!({
                "gate": gate_view,
                "boundary": boundary,
                "appealPath": appeal_path,
            }));
        }
        _ => {}
    }

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    let diagnostics_str = serde_json::to_string(&input_view.diagnostics).unwrap_or_default();
    let environment_str = input_view.environment.as_ref().map(|v| serde_json::to_string(v).unwrap_or_default());
    let avodah_str = input_view.avodah_context.as_ref().map(|v| serde_json::to_string(v).unwrap_or_default());

    let input = CreateIssueReportInput {
        human_id,
        description: input_view.description,
        category: input_view.category.unwrap_or_else(|| "bug".to_string()),
        severity: input_view.severity.unwrap_or_else(|| "info".to_string()),
        diagnostics: diagnostics_str,
        context_url: input_view.context_url,
        environment: environment_str,
        avodah_context: avodah_str,
    };

    match crate::db::issue_reports::create_issue_report(&mut conn, ctx, &input) {
        Ok(report) => response::created(&serde_json::json!({
            "data": IssueReportView::from(report),
            "gate": gate_view,
        })),
        Err(e) => response::error_response(e),
    }
}

async fn handle_list(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let query_str = req.uri().query().unwrap_or("");
    let query: IssueReportQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::issue_reports::list_issue_reports(&mut conn, ctx, &query) {
        Ok(items) => {
            let views: Vec<IssueReportView> = items.into_iter().map(|r| r.into()).collect();
            response::ok(&views)
        }
        Err(e) => response::error_response(e),
    }
}

async fn handle_get(id: &str, pool: &DbPool, ctx: &AppContext) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::issue_reports::get_issue_report(&mut conn, ctx, id) {
        Ok(report) => response::ok(&IssueReportView::from(report)),
        Err(e) => response::error_response(e),
    }
}
```

**Step 2: Wire into api/mod.rs**

Add `pub mod issue_reports;` and add a route dispatch arm in the main router. Find where `"comments"` is dispatched and add below it:

```rust
"issue-reports" => issue_reports::handle(req, method, resource_path, pool, ctx, services).await?,
```

**Step 3: Build and test**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5`
Expected: Compiles successfully

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -10`
Expected: All existing tests pass

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/api/issue_reports.rs
git add elohim/elohim-storage/src/api/mod.rs
git commit -m "feat(storage): add gated POST and ungated GET routes for issue-reports"
```

---

### Task 6: Generate TypeScript types

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/generated/` (auto-generated)
- Modify: `elohim/sdk/storage-client-ts/src/index.ts` (add re-exports)

**Step 1: Run type generation**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -5`
Expected: TypeScript types generated to `../../sdk/storage-client-ts/src/generated/`

**Step 2: Verify generated files**

Check that `IssueReportView.ts` and `CreateIssueReportInputView.ts` exist in the generated directory.

**Step 3: Add re-exports to index.ts**

In `elohim/sdk/storage-client-ts/src/index.ts`, add:

```typescript
export type { IssueReportView } from './generated/IssueReportView';
export type { CreateIssueReportInputView } from './generated/CreateIssueReportInputView';
```

**Step 4: Commit**

```bash
git add elohim/sdk/storage-client-ts/src/generated/
git add elohim/sdk/storage-client-ts/src/index.ts
git commit -m "feat(sdk): generate TypeScript types for IssueReportView"
```

---

### Task 7: StorageApiService — error logging + createIssueReport

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/storage-api.service.ts`

**Step 1: Add LoggerService to handleError**

Add `LoggerService` injection at the top of the class (it's `providedIn: 'root'` so just inject):

```typescript
private readonly logger = inject(LoggerService);
```

Update `handleError`:

```typescript
private handleError(operation: string, error: unknown): Observable<never> {
  const message = error instanceof Error ? error.message : String(error);
  const status = (error as Record<string, unknown>)['status'];
  const url = (error as Record<string, unknown>)['url'];

  this.logger.error(`${operation} failed`, error instanceof Error ? error : undefined, {
    operation,
    status: status as number,
    url: url as string,
  });

  return throwError(() => new Error(`${operation} failed: ${message}`));
}
```

**Step 2: Add createIssueReport method**

```typescript
/** Create an issue report with diagnostic payload. */
createIssueReport(input: CreateIssueReportInputView): Observable<IssueReportView> {
  return this.http
    .post<{ data: IssueReportView }>(`${this.baseUrl}/api/v1/issue-reports`, input)
    .pipe(
      map(response => response.data),
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('createIssueReport', error))
    );
}

/** List issue reports with optional filters. */
getIssueReports(filters?: {
  category?: string;
  severity?: string;
  resolutionStatus?: string;
  limit?: number;
}): Observable<IssueReportView[]> {
  let params = new HttpParams();
  if (filters?.category) params = params.set('category', filters.category);
  if (filters?.severity) params = params.set('severity', filters.severity);
  if (filters?.resolutionStatus) params = params.set('resolutionStatus', filters.resolutionStatus);
  if (filters?.limit) params = params.set('limit', filters.limit.toString());

  return this.http
    .get<IssueReportView[]>(`${this.baseUrl}/api/v1/issue-reports`, { params })
    .pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('getIssueReports', error))
    );
}
```

Add imports for `IssueReportView`, `CreateIssueReportInputView` from `@elohim/storage-client` and `LoggerService` from `./logger.service`.

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/storage-api.service.ts
git commit -m "feat(elohim): add error logging to handleError + createIssueReport on StorageApiService"
```

---

### Task 8: DiagnosticCollectorService

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/diagnostic-collector.service.ts`
- Create: `app/elohim-app/src/app/elohim/services/diagnostic-collector.service.spec.ts`

**Step 1: Write failing tests**

```typescript
import { TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { of, throwError } from 'rxjs';
import { vi } from 'vitest';

import { LoggerService, type LogEntry } from './logger.service';
import { DiagnosticCollectorService, type DiagnosticBundle } from './diagnostic-collector.service';

describe('DiagnosticCollectorService', () => {
  let service: DiagnosticCollectorService;
  let loggerMock: { getRecentLogs: ReturnType<typeof vi.fn> };
  let routerMock: { url: string };
  let httpMock: { get: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    loggerMock = {
      getRecentLogs: vi.fn().mockReturnValue([]),
    };
    routerMock = { url: '/learn/path/123/node/456' };
    httpMock = {
      get: vi.fn().mockReturnValue(of({ status: 'ok', blobs: 10, bytes: 1024 })),
    };

    TestBed.configureTestingModule({
      providers: [
        DiagnosticCollectorService,
        { provide: LoggerService, useValue: loggerMock },
        { provide: Router, useValue: routerMock },
        { provide: HttpClient, useValue: httpMock },
      ],
    });

    service = TestBed.inject(DiagnosticCollectorService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should include current route in context', async () => {
    const bundle = await service.collect();
    expect(bundle.context.url).toBe('/learn/path/123/node/456');
  });

  it('should include logs from LoggerService', async () => {
    const mockLogs: LogEntry[] = [
      { timestamp: '2026-03-15T10:00:00Z', level: 'error', message: 'test error' },
    ];
    loggerMock.getRecentLogs.mockReturnValue(mockLogs);

    const bundle = await service.collect();
    expect(bundle.logs).toEqual(mockLogs);
  });

  it('should filter logs to warn and error levels', async () => {
    const mockLogs: LogEntry[] = [
      { timestamp: '2026-03-15T10:00:00Z', level: 'debug', message: 'noise' },
      { timestamp: '2026-03-15T10:00:01Z', level: 'info', message: 'info' },
      { timestamp: '2026-03-15T10:00:02Z', level: 'warn', message: 'warning' },
      { timestamp: '2026-03-15T10:00:03Z', level: 'error', message: 'error' },
    ];
    loggerMock.getRecentLogs.mockReturnValue(mockLogs);

    const bundle = await service.collect();
    expect(bundle.logs.length).toBe(2);
    expect(bundle.logs[0].level).toBe('warn');
    expect(bundle.logs[1].level).toBe('error');
  });

  it('should include environment info', async () => {
    const bundle = await service.collect();
    expect(bundle.environment.platform).toBeDefined();
    expect(bundle.environment.userAgent).toBeDefined();
  });

  it('should fetch health snapshot', async () => {
    const bundle = await service.collect();
    expect(httpMock.get).toHaveBeenCalled();
    expect(bundle.environment.storageHealth).toEqual({ status: 'ok', blobs: 10, bytes: 1024 });
  });

  it('should handle health fetch failure gracefully', async () => {
    httpMock.get.mockReturnValue(throwError(() => new Error('network error')));

    const bundle = await service.collect();
    expect(bundle.environment.storageHealth).toBeNull();
  });

  it('should extract correlation IDs from logs', async () => {
    const mockLogs: LogEntry[] = [
      { timestamp: '2026-03-15T10:00:00Z', level: 'error', message: 'fail', correlationId: 'corr-1' },
      { timestamp: '2026-03-15T10:00:01Z', level: 'error', message: 'fail2', correlationId: 'corr-1' },
      { timestamp: '2026-03-15T10:00:02Z', level: 'warn', message: 'warn', correlationId: 'corr-2' },
    ];
    loggerMock.getRecentLogs.mockReturnValue(mockLogs);

    const bundle = await service.collect();
    expect(bundle.correlationIds).toEqual(['corr-1', 'corr-2']);
  });

  it('should include collectedAt timestamp', async () => {
    const bundle = await service.collect();
    expect(bundle.collectedAt).toBeDefined();
    expect(new Date(bundle.collectedAt).getTime()).toBeGreaterThan(0);
  });
});
```

**Step 2: Write the service**

```typescript
import { Injectable, inject } from '@angular/core';
import { Router } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom, timeout, catchError, of } from 'rxjs';

import { LoggerService, type LogEntry } from './logger.service';

export interface DiagnosticBundle {
  logs: LogEntry[];
  environment: {
    platform: 'browser' | 'tauri';
    userAgent: string;
    appVersion: string;
    storageHealth: Record<string, unknown> | null;
  };
  context: {
    url: string;
    eprId: string | null;
    avodahProject: string | null;
    avodahStory: string | null;
  };
  correlationIds: string[];
  collectedAt: string;
}

@Injectable({ providedIn: 'root' })
export class DiagnosticCollectorService {
  private readonly logger = inject(LoggerService);
  private readonly router = inject(Router);
  private readonly http = inject(HttpClient);

  async collect(): Promise<DiagnosticBundle> {
    const allLogs = this.logger.getRecentLogs();
    const logs = allLogs.filter(
      (l) => l.level === 'warn' || l.level === 'error',
    );

    const correlationIds = [
      ...new Set(
        logs
          .map((l) => l.correlationId)
          .filter((id): id is string => id != null),
      ),
    ];

    const isTauri = 'window' in globalThis && '__TAURI__' in (globalThis as Record<string, unknown>);

    let storageHealth: Record<string, unknown> | null = null;
    try {
      storageHealth = await firstValueFrom(
        this.http
          .get<Record<string, unknown>>('/health')
          .pipe(
            timeout(5000),
            catchError(() => of(null)),
          ),
      );
    } catch {
      // Health fetch failed — that's fine, it's diagnostic context not critical
    }

    return {
      logs,
      environment: {
        platform: isTauri ? 'tauri' : 'browser',
        userAgent: navigator.userAgent,
        appVersion: '0.1.0', // TODO: inject from build env
        storageHealth,
      },
      context: {
        url: this.router.url,
        eprId: null, // TODO: extract from route params when EPR routing lands
        avodahProject: null, // TODO: extract from Avodah context when available
        avodahStory: null,
      },
      correlationIds,
      collectedAt: new Date().toISOString(),
    };
  }
}
```

**Step 3: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "diagnostic-collector"`
Expected: All tests PASS

**Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/diagnostic-collector.service.ts
git add app/elohim-app/src/app/elohim/services/diagnostic-collector.service.spec.ts
git commit -m "feat(elohim): add DiagnosticCollectorService — gathers logs, health, route context"
```

---

### Task 9: Add service to barrel exports

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/index.ts`

**Step 1: Add exports**

Add to the services barrel:

```typescript
export { DiagnosticCollectorService } from './diagnostic-collector.service';
export type { DiagnosticBundle } from './diagnostic-collector.service';
```

**Step 2: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/index.ts
git commit -m "chore(elohim): export DiagnosticCollectorService from services barrel"
```

---

### Task 10: Extend FeedbackType + wire modal for 'report'

**Files:**
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.spec.ts`
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.spec.ts`

**Step 1: Extend FeedbackType**

In `gate-feedback-modal.component.ts`, change:

```typescript
export type FeedbackType = 'flag' | 'challenge' | 'feedback' | 'report';
```

Add to the maps:

```typescript
const TITLE_MAP: Record<string, string> = {
  flag: 'Flag Content',
  challenge: 'Challenge Content',
  feedback: 'Share Feedback',
  report: 'Report Issue',
};

const PLACEHOLDER_MAP: Record<string, string> = {
  flag: 'Describe the issue...',
  challenge: 'State your case...',
  feedback: 'Share your thoughts...',
  report: 'What happened?',
};
```

**Step 2: Add diagnostic collection to modal**

In the modal component, inject `DiagnosticCollectorService` and collect on init when `feedbackType()` is `'report'`:

```typescript
import { DiagnosticCollectorService, type DiagnosticBundle } from '../../services/diagnostic-collector.service';

// In the class:
private readonly diagnosticCollector = inject(DiagnosticCollectorService);
private diagnosticBundle: DiagnosticBundle | null = null;

constructor() {
  // Collect diagnostics when modal opens for report type
  effect(() => {
    if (this.feedbackType() === 'report') {
      this.diagnosticCollector.collect().then((bundle) => {
        this.diagnosticBundle = bundle;
      });
    }
  });
}
```

Update the `contextMetadata` computed to include diagnostics:

```typescript
readonly contextMetadata = computed(() => {
  const base: MutationContext = {
    contentId: this.contentId(),
    category: this.feedbackType(),
  };
  if (this.diagnosticBundle) {
    base['diagnostics'] = this.diagnosticBundle;
  }
  return base;
});
```

Update the `apiCall` to route 'report' type to `createIssueReport`:

```typescript
readonly apiCall = (text: string, context: MutationContext): Observable<unknown> => {
  if (context['category'] === 'report') {
    return this.storageApi.createIssueReport({
      description: text,
      category: 'bug',
      severity: 'info',
      diagnostics: context['diagnostics'] as unknown as import('serde_json').Value ?? {},
      contextUrl: context['contentId'] as string,
      environment: (context['diagnostics'] as DiagnosticBundle)?.environment ?? null,
      avodahContext: null,
    });
  }
  return this.storageApi.createComment(context['contentId'] as string, text);
};
```

**Important: Simplify the apiCall.** The `CreateIssueReportInputView` TypeScript type accepts `Value` for `diagnostics` — but in practice this is just a JSON object. Keep it clean:

```typescript
readonly apiCall = (text: string, context: MutationContext): Observable<unknown> => {
  if (context['category'] === 'report' && this.diagnosticBundle) {
    return this.storageApi.createIssueReport({
      description: text,
      diagnostics: this.diagnosticBundle,
      contextUrl: this.router.url,
      environment: this.diagnosticBundle.environment,
      avodahContext: this.diagnosticBundle.context.avodahProject
        ? { projectId: this.diagnosticBundle.context.avodahProject, storyId: this.diagnosticBundle.context.avodahStory }
        : null,
    });
  }
  return this.storageApi.createComment(context['contentId'] as string, text);
};
```

Add `Router` injection: `private readonly router = inject(Router);`

**Step 3: Add 'Report Issue' to trigger menu**

In `gate-feedback-trigger.component.ts`:

```typescript
const MENU_ITEMS: MenuItem[] = [
  { type: 'flag', label: 'Flag' },
  { type: 'challenge', label: 'Challenge' },
  { type: 'feedback', label: 'Feedback' },
  { type: 'report', label: 'Report Issue' },
];
```

**Step 4: Add tests**

In the modal spec, add:

```typescript
it('should render "Report Issue" title for report type', () => {
  fixture.componentRef.setInput('feedbackType', 'report');
  fixture.detectChanges();

  const title = fixture.nativeElement.querySelector('[data-testid="feedback-modal-title"]');
  expect(title.textContent.trim()).toBe('Report Issue');
});

it('should set placeholder to "What happened?" for report type', () => {
  fixture.componentRef.setInput('feedbackType', 'report');
  fixture.detectChanges();

  const textarea = fixture.nativeElement.querySelector('[data-testid="artifact-textarea"]');
  expect(textarea.getAttribute('placeholder')).toBe('What happened?');
});
```

In the trigger spec, update the menu items test:

```typescript
it('should show four menu items', () => {
  // ... click trigger ...
  const items = fixture.nativeElement.querySelectorAll('[data-testid^="feedback-menu-item-"]');
  expect(items.length).toBe(4);
});

it('should show Flag, Challenge, Feedback, Report Issue labels', () => {
  // ... click trigger ...
  const labels = Array.from(
    fixture.nativeElement.querySelectorAll('[data-testid^="feedback-menu-item-"]'),
  ).map((el: Element) => (el as HTMLElement).textContent?.trim());
  expect(labels).toEqual(['Flag', 'Challenge', 'Feedback', 'Report Issue']);
});
```

**Step 5: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-feedback"`
Expected: All tests pass (existing + new)

**Step 6: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/gate-feedback/
git commit -m "feat(elohim): add 'Report Issue' to feedback menu with diagnostic collection"
```

---

### Task 11: Run full test suite + lint

**Step 1: Run all tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts`
Expected: All existing tests pass, new tests pass

**Step 2: Run Rust tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test 2>&1 | tail -10`
Expected: All tests pass

**Step 3: Run lint**

Run: `cd app/elohim-app && pnpm run lint`
Expected: No new lint errors

**Step 4: Fix any issues, commit if needed**

---

## Future Seams (NOT implemented now, documented for reference)

### Screenshot Capture
- **Auto-capture (A):** `html2canvas` library, captures viewport on "Report Issue" click. Works in both browser and Tauri WebView. Store as base64 in diagnostics bundle or upload as blob.
- **User-provided (B):** Clipboard paste (`paste` event on textarea) or drag-and-drop. Upload as blob, include blob CID in diagnostics.
- Both should be available — AI agent reads auto-capture, human sees their own screenshot in the report.

### Agent Code Awareness
- Elohim agent has the codebase map as a tool, not payload on the wire
- Route-to-component mapping is the agent's investigation, not collection logic
- Agent queries backend logs via correlation IDs from the diagnostic bundle

### Avodah Integration
- Issue report → work-story creation (agent compute)
- REA event on resolution (links report → story → fix → recognition)
