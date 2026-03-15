# Gate Comment Wiring — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the gate artifact card to real HTTP calls end-to-end — build comment persistence in Rust, generate TypeScript types, connect the Angular card via injectable API callback.

**Architecture:** New `comments` table + Diesel model + gated REST routes in elohim-storage. Angular gets `StorageApiService.createComment()`, `GateInteractionService.submitWithApi()`, and `GateArtifactCardComponent` accepts a `gateApiCall` input. GateCommentComponent wires it together.

**Tech Stack:** Rust (Diesel ORM, hyper, serde, ts-rs), Angular 19 (signals, OnPush, Vitest), TypeScript type generation.

---

## Task 1: Diesel Migration — Comments Table

### Files
- Create: `elohim/elohim-storage/migrations/2026-03-15-100000_comments/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-15-100000_comments/down.sql`

### Step 1: Create migration directory

```bash
mkdir -p elohim/elohim-storage/migrations/2026-03-15-100000_comments
```

### Step 2: Write up.sql

```sql
CREATE TABLE comments (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    content_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    body TEXT NOT NULL,
    reach TEXT NOT NULL DEFAULT 'close',
    governance_state TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_comments_content_id ON comments (content_id);
CREATE INDEX idx_comments_human_id ON comments (human_id);
CREATE INDEX idx_comments_app_id ON comments (app_id);
```

### Step 3: Write down.sql

```sql
DROP TABLE IF EXISTS comments;
```

### Step 4: Run migration to regenerate diesel_schema.rs

```bash
cd elohim/elohim-storage && diesel migration run
```

This updates `src/db/diesel_schema.rs` with the `comments` table definition.

### Step 5: Verify diesel_schema.rs has the new table

Check that `diesel_schema.rs` now contains:
```rust
diesel::table! {
    comments (id) {
        id -> Text,
        app_id -> Text,
        content_id -> Text,
        human_id -> Text,
        body -> Text,
        reach -> Text,
        governance_state -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

### Step 6: Commit

```bash
git add elohim/elohim-storage/migrations/2026-03-15-100000_comments/ \
       elohim/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(storage): add comments table migration"
```

---

## Task 2: Diesel Model — Comment + NewComment

### Files
- Modify: `elohim/elohim-storage/src/db/models.rs` (add Comment + NewComment structs)
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (already updated by migration — just verify import)

### Step 1: Add import of `comments` table to models.rs

In the `use super::diesel_schema::{...}` block at the top of `models.rs`, add `comments` to the list.

### Step 2: Add Comment model structs

Add at the end of models.rs (before any closing brace), following the existing pattern:

```rust
// ============================================================================
// Comment Models
// ============================================================================

/// Comment from the comments table (Queryable)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = comments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Comment {
    pub id: String,
    pub app_id: String,
    pub content_id: String,
    pub human_id: String,
    pub body: String,
    pub reach: String,
    pub governance_state: String,
    pub created_at: String,
    pub updated_at: String,
}

/// New comment for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = comments)]
pub struct NewComment {
    pub id: String,
    pub app_id: String,
    pub content_id: String,
    pub human_id: String,
    pub body: String,
    pub reach: String,
    pub governance_state: String,
    pub created_at: String,
    pub updated_at: String,
}
```

### Step 3: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

### Step 4: Commit

```bash
git add elohim/elohim-storage/src/db/models.rs
git commit -m "feat(storage): add Comment and NewComment diesel models"
```

---

## Task 3: View Types — CommentView + CreateCommentInputView

### Files
- Modify: `elohim/elohim-storage/src/views.rs` (add CommentView, CreateCommentInputView)

### Step 1: Add CommentView

Add after the existing view structs (e.g. after `StewardshipAllocationWithPresenceView`), following the exact pattern:

```rust
// ============================================================================
// Comment Views
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CommentView {
    pub id: String,
    pub content_id: String,
    pub human_id: String,
    pub body: String,
    pub reach: String,
    pub governance_state: String,
    pub created_at: String,
}

impl From<crate::db::models::Comment> for CommentView {
    fn from(c: crate::db::models::Comment) -> Self {
        Self {
            id: c.id,
            content_id: c.content_id,
            human_id: c.human_id,
            body: c.body,
            reach: c.reach,
            governance_state: c.governance_state,
            created_at: c.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateCommentInputView {
    pub content_id: String,
    pub body: String,
}
```

### Step 2: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

### Step 3: Commit

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "feat(storage): add CommentView and CreateCommentInputView"
```

---

## Task 4: Comment DB Module — CRUD Operations

### Files
- Create: `elohim/elohim-storage/src/db/comments.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` (add `pub mod comments;`)

### Step 1: Create comments.rs

Follow the `stewardship_allocations.rs` pattern:

```rust
//! Comments CRUD operations

use diesel::prelude::*;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::comments;
use super::models::{current_timestamp, Comment, NewComment};
use crate::error::StorageError;

/// Input for creating a comment (from controller after gate check)
pub struct CreateCommentInput {
    pub content_id: String,
    pub human_id: String,
    pub body: String,
    pub reach: String,
}

/// Query parameters for listing comments
#[derive(Debug, Clone, Default)]
pub struct CommentQuery {
    pub content_id: Option<String>,
    pub human_id: Option<String>,
    pub limit: Option<i64>,
}

/// Create a new comment
pub fn create_comment(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateCommentInput,
) -> Result<Comment, StorageError> {
    let now = current_timestamp();
    let new = NewComment {
        id: Uuid::new_v4().to_string(),
        app_id: ctx.app_id.clone(),
        content_id: input.content_id,
        human_id: input.human_id,
        body: input.body,
        reach: input.reach,
        governance_state: "active".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    diesel::insert_into(comments::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create comment: {}", e)))?;

    comments::table
        .filter(comments::id.eq(&new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to fetch created comment: {}", e)))
}

/// Get a single comment by ID
pub fn get_comment(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Comment, StorageError> {
    comments::table
        .filter(comments::id.eq(id))
        .filter(comments::app_id.eq(&ctx.app_id))
        .first(conn)
        .map_err(|e| StorageError::NotFound(format!("Comment not found: {}", e)))
}

/// List comments with optional filters
pub fn list_comments(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: CommentQuery,
) -> Result<Vec<Comment>, StorageError> {
    let mut q = comments::table
        .filter(comments::app_id.eq(&ctx.app_id))
        .order(comments::created_at.desc())
        .into_boxed();

    if let Some(content_id) = &query.content_id {
        q = q.filter(comments::content_id.eq(content_id));
    }
    if let Some(human_id) = &query.human_id {
        q = q.filter(comments::human_id.eq(human_id));
    }
    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    q.load::<Comment>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list comments: {}", e)))
}
```

### Step 2: Add module to db/mod.rs

Add `pub mod comments;` in the domain modules section.

### Step 3: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

### Step 4: Commit

```bash
git add elohim/elohim-storage/src/db/comments.rs elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): add comments CRUD module"
```

---

## Task 5: Comment API Routes — Gated POST, Ungated GET

### Files
- Create: `elohim/elohim-storage/src/api/comments.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs` (add `pub mod comments;` + route dispatch)

### Step 1: Create comments.rs API controller

Follow the `stewardship.rs` gated create pattern exactly:

```rust
//! Comments API controller
//!
//! Routes: `/api/v1/comments[/{id}]`

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::comments::{self, CommentQuery, CreateCommentInput};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::elohim_gate::MutationType;
use crate::services::{response, Services};
use crate::views::{CommentView, CreateCommentInputView};

use super::{get_conn, parse_body};

/// URL query params for listing comments
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListCommentsQuery {
    pub content_id: Option<String>,
    pub human_id: Option<String>,
    pub limit: Option<i64>,
}

/// Handle all `/api/v1/comments*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Strip leading slash: "/foo" -> "foo", "" -> ""
    let path = resource_path.strip_prefix('/').unwrap_or(resource_path);

    match (&method, path) {
        // POST /api/v1/comments — gated create
        (&Method::POST, "") => handle_create_comment(req, pool, ctx, services).await,

        // GET /api/v1/comments — list with query params
        (&Method::GET, "") => handle_list_comments(req, pool, ctx).await,

        // GET /api/v1/comments/{id}
        (&Method::GET, id) if !id.is_empty() => handle_get_comment(id, pool, ctx).await,

        _ => Ok(response::not_found(&format!(
            "Unknown comments route: {} /api/v1/comments{}",
            method, resource_path
        ))),
    }
}

async fn handle_create_comment(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input_view: CreateCommentInputView = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return Ok(response::bad_request("Invalid JSON body for create comment")),
    };

    // Gate evaluation — use the comment body as mutation content
    let (gate_result, gate_view) = super::evaluate_gate(
        &services,
        pool,
        ctx,
        MutationType::Comment,
        serde_json::json!({
            "contentId": input_view.content_id,
            "body": input_view.body,
        }),
        None, // TODO: extract human_id from auth context
    )
    .await;

    // Handle pause/settlement
    match &gate_result {
        crate::services::elohim_gate::GateResult::Pause {
            prompt,
            confirm_token,
            ..
        } => {
            return Ok(response::json_response(
                hyper::StatusCode::CONFLICT,
                &serde_json::json!({
                    "gate": gate_view,
                    "pausePrompt": prompt,
                    "confirmToken": confirm_token,
                }),
            ));
        }
        crate::services::elohim_gate::GateResult::Settlement {
            boundary,
            appeal_path,
            ..
        } => {
            return Ok(response::forbidden(&serde_json::json!({
                "gate": gate_view,
                "boundary": boundary,
                "appealPath": appeal_path,
            })));
        }
        _ => {}
    }

    // Gate passed — create the comment
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return Ok(response::error_response(e)),
    };

    // Compute reach from trust context
    let reach = match &gate_view {
        Some(gv) => {
            let trust = gv.trust_context.composite_trust;
            if trust >= 0.85 {
                "constitutional"
            } else if trust >= 0.6 {
                "network"
            } else if trust >= 0.3 {
                "community"
            } else {
                "close"
            }
        }
        None => "close",
    };

    let input = CreateCommentInput {
        content_id: input_view.content_id,
        human_id: "anonymous".to_string(), // TODO: extract from auth context
        body: input_view.body,
        reach: reach.to_string(),
    };

    match comments::create_comment(&mut conn, ctx, input) {
        Ok(c) => Ok(response::created(&serde_json::json!({
            "data": CommentView::from(c),
            "gate": gate_view,
        }))),
        Err(e) => Ok(response::error_response(e)),
    }
}

async fn handle_list_comments(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query_str = req.uri().query().unwrap_or("");
    let params: ListCommentsQuery =
        serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return Ok(response::error_response(e)),
    };

    let query = CommentQuery {
        content_id: params.content_id,
        human_id: params.human_id,
        limit: params.limit,
    };

    match comments::list_comments(&mut conn, ctx, query) {
        Ok(list) => {
            let views: Vec<CommentView> = list.into_iter().map(CommentView::from).collect();
            Ok(response::ok(&views))
        }
        Err(e) => Ok(response::error_response(e)),
    }
}

async fn handle_get_comment(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return Ok(response::error_response(e)),
    };

    match comments::get_comment(&mut conn, ctx, id) {
        Ok(c) => Ok(response::ok(&CommentView::from(c))),
        Err(e) => Ok(response::error_response(e)),
    }
}
```

### Step 2: Add `pub mod comments;` to api/mod.rs

Add it alphabetically with the other modules (after `pub mod attestations;`).

### Step 3: Add route dispatch to `handle_api_request`

In the `handle_api_request` function in `api/mod.rs`, add before the `else` fallback:

```rust
} else if sub_path.starts_with("comments") {
    let resource_path = sub_path.strip_prefix("comments").unwrap_or("");
    comments::handle(req, method, resource_path, &pool, &app_ctx, services).await
```

### Step 4: Add `Comment` MutationType variant

In `elohim/elohim-storage/src/services/elohim_gate.rs`, find the `MutationType` enum and add `Comment` variant. If the enum doesn't have it, add:

```rust
Comment,
```

### Step 5: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

### Step 6: Run tests

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -20
```

### Step 7: Commit

```bash
git add elohim/elohim-storage/src/api/comments.rs \
       elohim/elohim-storage/src/api/mod.rs \
       elohim/elohim-storage/src/services/elohim_gate.rs
git commit -m "feat(storage): add gated comment API routes

POST /api/v1/comments — gated create with evaluate_gate()
GET /api/v1/comments — list by contentId
GET /api/v1/comments/{id} — get single comment
Reach computed from trust context composite score."
```

---

## Task 6: Generate TypeScript Types

### Step 1: Run type generation

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -10
```

### Step 2: Verify generated files exist

```bash
ls elohim/sdk/storage-client-ts/src/generated/CommentView.ts
ls elohim/sdk/storage-client-ts/src/generated/CreateCommentInputView.ts
```

### Step 3: Add exports to generated index.ts

Check if `elohim/sdk/storage-client-ts/src/generated/index.ts` auto-includes the new types. If not, add:

```typescript
export type { CommentView } from './CommentView';
export type { CreateCommentInputView } from './CreateCommentInputView';
```

### Step 4: Commit

```bash
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(sdk): generate CommentView and CreateCommentInputView types"
```

---

## Task 7: StorageApiService.createComment()

### Files
- Modify: `app/elohim-app/src/app/elohim/services/storage-api.service.ts`
- Create: `app/elohim-app/src/app/elohim/services/storage-api-comments.service.spec.ts` (focused test)

### Step 1: Write the failing test

```typescript
// storage-api-comments.service.spec.ts
import { TestBed } from '@angular/core/testing';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { of, throwError } from 'rxjs';
import { vi, describe, it, expect, beforeEach } from 'vitest';

import { StorageApiService } from '@app/elohim/services/storage-api.service';
import { GateService } from '@app/elohim/services/gate.service';

describe('StorageApiService - Comments', () => {
  let service: StorageApiService;
  let httpMock: { post: ReturnType<typeof vi.fn>; get: ReturnType<typeof vi.fn> };
  let gateService: GateService;

  beforeEach(() => {
    httpMock = {
      post: vi.fn().mockReturnValue(of({})),
      get: vi.fn().mockReturnValue(of([])),
    };

    TestBed.configureTestingModule({
      providers: [
        StorageApiService,
        { provide: HttpClient, useValue: httpMock },
      ],
    });
    service = TestBed.inject(StorageApiService);
    gateService = TestBed.inject(GateService);
  });

  it('should POST to /api/v1/comments', () => {
    const response = {
      data: { id: 'c-1', contentId: 'n-1', body: 'Hello' },
      gate: {
        tier: 'standard',
        trustContext: { compositeTrust: 0.5 },
        pausePrompt: null,
        confirmToken: null,
        settlementBoundary: null,
        appealPath: null,
      },
    };
    httpMock.post.mockReturnValue(of(response));

    service.createComment('n-1', 'Hello').subscribe(result => {
      expect(result).toBeTruthy();
    });

    expect(httpMock.post).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/comments'),
      { contentId: 'n-1', body: 'Hello' },
    );
  });

  it('should extract gate from response', () => {
    const gate = {
      tier: 'standard',
      trustContext: {
        compositeTrust: 0.5,
        masteryDepth: 0.5,
        stewardStanding: 0.5,
        relationshipDensity: 0.3,
        governanceHealth: 0.8,
        behavioralTrust: 0.7,
        intentDivergence: 0.1,
        declaredIntent: null,
      },
      pausePrompt: null,
      confirmToken: null,
      settlementBoundary: null,
      appealPath: null,
    };
    httpMock.post.mockReturnValue(of({ data: { id: 'c-1' }, gate }));
    const spy = vi.spyOn(gateService, 'handleGateResponse');

    service.createComment('n-1', 'Test').subscribe();

    expect(spy).toHaveBeenCalledWith(gate);
  });

  it('should handle 409 pause error', () => {
    const error = new HttpErrorResponse({
      status: 409,
      error: {
        gate: {
          tier: 'standard',
          trustContext: { compositeTrust: 0.5 },
          pausePrompt: 'Consider rephrasing.',
          confirmToken: 'tok-1',
          settlementBoundary: null,
          appealPath: null,
        },
      },
    });
    httpMock.post.mockReturnValue(throwError(() => error));

    service.createComment('n-1', 'Harsh').subscribe({
      error: (err) => {
        expect(err.status).toBe(409);
        expect(gateService.isPaused()).toBe(true);
      },
    });
  });

  it('should GET comments by contentId', () => {
    httpMock.get.mockReturnValue(of([{ id: 'c-1', body: 'Hello' }]));

    service.getComments('n-1').subscribe(result => {
      expect(result).toHaveLength(1);
    });

    expect(httpMock.get).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/comments'),
      expect.objectContaining({ params: expect.anything() }),
    );
  });
});
```

### Step 2: Run tests to verify they fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "storage-api-comments"
```

### Step 3: Add methods to StorageApiService

Add to `storage-api.service.ts` in the service class:

```typescript
// ============================================================================
// Comments
// ============================================================================

createComment(contentId: string, body: string): Observable<unknown> {
  return this.http
    .post<unknown>(`${this.baseUrl}/api/v1/comments`, { contentId, body })
    .pipe(
      timeout(this.defaultTimeoutMs),
      tap(response => {
        const gate = extractGateFromResponse(response);
        if (gate) {
          this.gateService.handleGateResponse(gate);
        }
      }),
      map((response: unknown) => (response as Record<string, unknown>)?.['data'] ?? response),
      catchError(error => handleGateError(error, this.gateService)),
      catchError(error => this.handleError('createComment', error)),
    );
}

getComments(contentId: string): Observable<unknown[]> {
  const params = new HttpParams().set('contentId', contentId);
  return this.http
    .get<unknown[]>(`${this.baseUrl}/api/v1/comments`, { params })
    .pipe(
      timeout(this.defaultTimeoutMs),
      catchError(error => this.handleError('getComments', error)),
    );
}
```

### Step 4: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "storage-api-comments"
```

### Step 5: Commit

```bash
git add app/elohim-app/src/app/elohim/services/storage-api.service.ts \
       app/elohim-app/src/app/elohim/services/storage-api-comments.service.spec.ts
git commit -m "feat(elohim): add createComment and getComments to StorageApiService

Gated POST to /api/v1/comments with gate response extraction.
Ungated GET with contentId query parameter."
```

---

## Task 8: GateInteractionService.submitWithApi()

### Files
- Modify: `app/elohim-app/src/app/elohim/services/gate-interaction.service.ts`
- Modify: `app/elohim-app/src/app/elohim/services/gate-interaction.service.spec.ts`

### Step 1: Write the failing tests

Add to the existing spec file:

```typescript
// --- submitWithApi Flow ---

it('should transition to evaluating on submitWithApi', () => {
  const apiCall = vi.fn().mockReturnValue(of({
    data: { id: 'c-1' },
    gate: makeEvaluation(),
  }));

  service.submitWithApi('Hello', 'comment', { contentId: 'c-1' }, apiCall);

  expect(apiCall).toHaveBeenCalledWith('Hello', { contentId: 'c-1' });
});

it('should transition to affirm on successful API response', () => {
  const apiCall = vi.fn().mockReturnValue(of({
    data: { id: 'c-1' },
    gate: makeEvaluation({ trustContext: makeTrustContext({ compositeTrust: 0.5 }) }),
  }));

  service.submitWithApi('Hello', 'comment', { contentId: 'c-1' }, apiCall);

  expect(service.state()).toBe('affirm');
  expect(service.gateResult()).toBeTruthy();
});

it('should transition to dialogue on 409 error with gate', () => {
  const error = {
    status: 409,
    error: {
      gate: makeEvaluation({
        pausePrompt: 'Consider rephrasing.',
        confirmToken: 'tok-1',
      }),
    },
  };
  const apiCall = vi.fn().mockReturnValue(throwError(() => error));

  service.submitWithApi('Harsh', 'comment', { contentId: 'c-1' }, apiCall);

  expect(service.state()).toBe('dialogue');
  expect(service.gateResult()?.pausePrompt).toBe('Consider rephrasing.');
});

it('should transition to settled on 403 error with gate', () => {
  const error = {
    status: 403,
    error: {
      gate: makeEvaluation({
        settlementBoundary: 'constitutional-limit',
        appealPath: '/appeal/123',
      }),
    },
  };
  const apiCall = vi.fn().mockReturnValue(throwError(() => error));

  service.submitWithApi('Bad', 'comment', { contentId: 'c-1' }, apiCall);

  expect(service.state()).toBe('settled');
});

it('should revert to draft on network error', () => {
  const error = { status: 0, message: 'Network error' };
  const apiCall = vi.fn().mockReturnValue(throwError(() => error));

  service.submitWithApi('Hello', 'comment', { contentId: 'c-1' }, apiCall);

  expect(service.state()).toBe('draft');
});

it('should not double-submit when already evaluating via submitWithApi', () => {
  let resolveApi: (value: unknown) => void;
  const pending = new Observable(sub => {
    resolveApi = (val) => { sub.next(val); sub.complete(); };
  });
  const apiCall = vi.fn().mockReturnValue(pending);

  service.submitWithApi('First', 'comment', { contentId: 'c-1' }, apiCall);
  service.submitWithApi('Second', 'comment', { contentId: 'c-2' }, apiCall);

  expect(apiCall).toHaveBeenCalledTimes(1);
  expect(service.draftText()).toBe('First');
});
```

Add `import { Observable } from 'rxjs';` if not already imported.

### Step 2: Run tests to verify new tests fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-interaction.service"
```

### Step 3: Add submitWithApi to the service

Add to `gate-interaction.service.ts`:

```typescript
import { Observable } from 'rxjs';
import { extractGateFromResponse } from '../models/gated-response.model';

// ... inside the class:

submitWithApi(
  text: string,
  mutationType: string,
  context: MutationContext,
  apiCall: (text: string, context: MutationContext) => Observable<unknown>,
): void {
  if (this._state() === 'evaluating') return;
  this._draftText.set(text);
  this._mutationType = mutationType;
  this._context = context;
  this._state.set('evaluating');

  apiCall(text, context).subscribe({
    next: (response) => {
      const gate = extractGateFromResponse(response);
      if (gate) {
        this.handleGateEvaluation(gate);
      } else {
        // Ungated response — go straight to posted
        this._state.set('posted');
      }
    },
    error: (err) => {
      const gate = err?.error?.gate;
      if ((err?.status === 409 || err?.status === 403) && gate) {
        this.handleGateEvaluation(gate);
      } else {
        // Network error — revert to draft so user can retry
        this._state.set('draft');
      }
    },
  });
}
```

### Step 4: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-interaction.service"
```

### Step 5: Export the method type if needed

The barrel export already covers `GateInteractionService`, so no change needed.

### Step 6: Commit

```bash
git add app/elohim-app/src/app/elohim/services/gate-interaction.service.ts \
       app/elohim-app/src/app/elohim/services/gate-interaction.service.spec.ts
git commit -m "feat(elohim): add submitWithApi to GateInteractionService

HTTP-aware submit that calls the provided API function, extracts gate
from response (200) or error (409/403), and drives state transitions.
Reverts to draft on network errors."
```

---

## Task 9: GateArtifactCardComponent — Add gateApiCall Input

### Files
- Modify: `app/elohim-app/src/app/elohim/components/gate-artifact-card/gate-artifact-card.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/gate-artifact-card/gate-artifact-card.component.spec.ts`

### Step 1: Write the failing test

Add to the existing spec file:

```typescript
it('should call gateApiCall on submit when provided', () => {
  const apiCall = vi.fn().mockReturnValue(of({
    data: { id: 'c-1' },
    gate: makeEvaluation(),
  }));
  component.gateApiCall = apiCall;
  fixture.detectChanges();

  // Type text and submit
  const textarea = fixture.nativeElement.querySelector('[data-testid="artifact-textarea"]');
  textarea.value = 'API comment';
  textarea.dispatchEvent(new Event('input'));
  fixture.detectChanges();

  const submitBtn = fixture.nativeElement.querySelector('[data-testid="artifact-submit"]');
  submitBtn.click();
  fixture.detectChanges();

  expect(apiCall).toHaveBeenCalledWith('API comment', { contentId: 'c-1' });
  expect(component.interaction.state()).toBe('affirm');
});

it('should use manual submit when no gateApiCall provided', () => {
  // No gateApiCall set — should fall back to manual submit
  const textarea = fixture.nativeElement.querySelector('[data-testid="artifact-textarea"]');
  textarea.value = 'Manual text';
  textarea.dispatchEvent(new Event('input'));
  fixture.detectChanges();

  const submitBtn = fixture.nativeElement.querySelector('[data-testid="artifact-submit"]');
  submitBtn.click();
  fixture.detectChanges();

  // Manual submit just transitions to evaluating
  expect(component.interaction.state()).toBe('evaluating');
});
```

### Step 2: Run tests to verify new tests fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-artifact-card"
```

### Step 3: Add gateApiCall input and update onSubmit

In `gate-artifact-card.component.ts`, add the input:

```typescript
import { Observable } from 'rxjs';

// In the component class:
@Input() gateApiCall?: (text: string, context: MutationContext) => Observable<unknown>;
```

Update `onSubmit()`:

```typescript
onSubmit(): void {
  const text = this.localText().trim();
  if (!text) return;
  if (this.gateApiCall) {
    this.interaction.submitWithApi(text, this.mutationType, this.contextMetadata, this.gateApiCall);
  } else {
    this.interaction.submit(text, this.mutationType, this.contextMetadata);
  }
}
```

Update `onResubmit()` similarly:

```typescript
protected onResubmit(): void {
  const text = this.localText().trim();
  if (!text) return;
  if (this.gateApiCall) {
    this.interaction.submitWithApi(text, this.mutationType, this.contextMetadata, this.gateApiCall);
  } else {
    this.interaction.submit(text, this.mutationType, this.contextMetadata);
  }
}
```

### Step 4: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-artifact-card"
```

### Step 5: Commit

```bash
git add app/elohim-app/src/app/elohim/components/gate-artifact-card/
git commit -m "feat(elohim): add gateApiCall input to GateArtifactCardComponent

When gateApiCall is provided, submit delegates to submitWithApi for
real HTTP flow. Falls back to manual submit for testing/demo."
```

---

## Task 10: GateCommentComponent — Wire API Callback

### Files
- Modify: `app/elohim-app/src/app/elohim/components/gate-comment/gate-comment.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/gate-comment/gate-comment.component.spec.ts`

### Step 1: Write the failing test

Add to existing spec:

```typescript
it('should provide gateApiCall to the card', () => {
  // The card should receive an apiCall function
  const card = fixture.nativeElement.querySelector('app-gate-artifact-card');
  expect(card).toBeTruthy();
  // Verify the component has a storageApi-backed apiCall
  expect(component.apiCall).toBeDefined();
  expect(typeof component.apiCall).toBe('function');
});
```

### Step 2: Run tests to verify it fails

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-comment"
```

### Step 3: Update GateCommentComponent

```typescript
import {
  Component,
  ChangeDetectionStrategy,
  Input,
  Output,
  EventEmitter,
  inject,
} from '@angular/core';
import { Observable } from 'rxjs';

import { StorageApiService } from '../../services/storage-api.service';
import { GateArtifactCardComponent } from '../gate-artifact-card/gate-artifact-card.component';
import type { ReachTier, MutationContext } from '../../services/gate-interaction.service';

@Component({
  selector: 'app-gate-comment',
  standalone: true,
  imports: [GateArtifactCardComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="gate-comment">
      <app-gate-artifact-card
        [placeholder]="'Add a comment...'"
        [mutationType]="'comment'"
        [contextMetadata]="{ contentId: contentId }"
        [gateApiCall]="apiCall"
        (posted)="onPosted($event)"
        (settled)="onSettled($event)"
      ></app-gate-artifact-card>
    </div>
  `,
  styles: [`
    .gate-comment {
      margin: 1rem 0;
    }
  `],
})
export class GateCommentComponent {
  @Input({ required: true }) contentId!: string;

  @Output() commentPosted = new EventEmitter<{ reachTier: ReachTier }>();
  @Output() commentSettled = new EventEmitter<{ boundary: string; appealPath: string | null }>();

  private readonly storageApi = inject(StorageApiService);

  readonly apiCall = (_text: string, context: MutationContext): Observable<unknown> => {
    return this.storageApi.createComment(context.contentId as string, _text);
  };

  protected onPosted(event: { reachTier: ReachTier }): void {
    this.commentPosted.emit(event);
  }

  protected onSettled(event: { boundary: string; appealPath: string | null }): void {
    this.commentSettled.emit(event);
  }
}
```

### Step 4: Update spec to mock StorageApiService

Update the `beforeEach` to also provide `StorageApiService`:

```typescript
beforeEach(async () => {
  httpMock = { post: vi.fn().mockReturnValue(of({})) };

  await TestBed.configureTestingModule({
    imports: [GateCommentComponent],
    providers: [
      { provide: HttpClient, useValue: httpMock },
    ],
  }).compileComponents();

  fixture = TestBed.createComponent(GateCommentComponent);
  component = fixture.componentInstance;
  component.contentId = 'content-123';
  fixture.detectChanges();
});
```

(StorageApiService is `providedIn: 'root'` so it picks up the HttpClient mock automatically.)

### Step 5: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-comment"
```

### Step 6: Commit

```bash
git add app/elohim-app/src/app/elohim/components/gate-comment/
git commit -m "feat(elohim): wire GateCommentComponent to real API

Injects StorageApiService, provides apiCall that delegates to
createComment(). Card now makes real gated HTTP calls."
```

---

## Task 11: Integration Verification

### Step 1: Run all gate tests

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate"
```

Expected: All gate tests pass (~60+ tests).

### Step 2: Run full Angular test suite

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts
```

Expected: No regressions.

### Step 3: Run Rust tests

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -10
```

Expected: All Rust tests pass including any new comment-related tests.

### Step 4: Run Angular lint

```bash
cd app/elohim-app && pnpm run lint 2>&1 | grep -E "gate-interaction|gate-artifact|gate-comment|storage-api-comments"
```

Expected: No lint errors in our files.

### Step 5: Add proxy route for comments (if needed)

Check `app/elohim-app/proxy.conf.mjs` — the existing `/api` proxy should already cover `/api/v1/comments`. Verify:

```bash
grep -n "api" app/elohim-app/proxy.conf.mjs
```

If `/api` is in the context array, no change needed.
