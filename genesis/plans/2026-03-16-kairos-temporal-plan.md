# Kairos Temporal Schedules — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a polymorphic `schedules` table with RRULE support so any CID-addressed entity can carry scheduling, expiry, and recurrence settings.

**Architecture:** Diesel migration + model + views + CRUD module with `rrule` crate for RFC 5545 parsing. Hybrid API: standalone `/api/v1/schedules` + entity-nested `/api/v1/content/{cid}/schedule`. Angular `ScheduleService` is a thin HTTP client — all RRULE computation stays in Rust.

**Tech Stack:** Rust (Diesel ORM, rrule 0.14, hyper, serde, ts-rs), Angular 19 (signals, OnPush, Vitest).

---

## Task 1: Diesel Migration — Schedules Table

### Files
- Create: `elohim/elohim-storage/migrations/2026-03-16-100000_schedules/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-16-100000_schedules/down.sql`

### Step 1: Create migration directory

```bash
mkdir -p elohim/elohim-storage/migrations/2026-03-16-100000_schedules
```

### Step 2: Write up.sql

```sql
CREATE TABLE schedules (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    scheduled_at TEXT,
    expires_at TEXT,
    rrule TEXT,
    last_occurred_at TEXT,
    next_occurrence_at TEXT,
    occurrence_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(entity_type, entity_id)
);

CREATE INDEX idx_schedules_entity ON schedules (entity_type, entity_id);
CREATE INDEX idx_schedules_next ON schedules (next_occurrence_at);
CREATE INDEX idx_schedules_scheduled ON schedules (scheduled_at);
```

### Step 3: Write down.sql

```sql
DROP TABLE IF EXISTS schedules;
```

### Step 4: Add schedules table to diesel_schema.rs

Manually add the table definition to `src/db/diesel_schema.rs` (same approach as the comments migration — `diesel migration run` can't run from scratch due to the humans table issue):

```rust
diesel::table! {
    schedules (id) {
        id -> Text,
        app_id -> Text,
        entity_type -> Text,
        entity_id -> Text,
        scheduled_at -> Nullable<Text>,
        expires_at -> Nullable<Text>,
        rrule -> Nullable<Text>,
        last_occurred_at -> Nullable<Text>,
        next_occurrence_at -> Nullable<Text>,
        occurrence_count -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

Also add `schedules` to the `allow_tables_to_appear_in_same_query!` macro.

### Step 5: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5
```

### Step 6: Commit

```bash
git add elohim/elohim-storage/migrations/2026-03-16-100000_schedules/ \
       elohim/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(storage): add schedules table migration"
```

---

## Task 2: Diesel Models — Schedule + NewSchedule

### Files
- Modify: `elohim/elohim-storage/src/db/models.rs`

### Step 1: Add `schedules` to the diesel_schema import

In `models.rs`, add `schedules` to the `use super::diesel_schema::{...}` import.

### Step 2: Add model structs

```rust
// ============================================================================
// Schedule Models (Kairos temporal dimension)
// ============================================================================

/// Schedule from the schedules table (Queryable)
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = schedules)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Schedule {
    pub id: String,
    pub app_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
    pub last_occurred_at: Option<String>,
    pub next_occurrence_at: Option<String>,
    pub occurrence_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// New schedule for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = schedules)]
pub struct NewSchedule {
    pub id: String,
    pub app_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
    pub last_occurred_at: Option<String>,
    pub next_occurrence_at: Option<String>,
    pub occurrence_count: i32,
    pub created_at: String,
    pub updated_at: String,
}
```

### Step 3: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5
```

### Step 4: Commit

```bash
git add elohim/elohim-storage/src/db/models.rs
git commit -m "feat(storage): add Schedule and NewSchedule diesel models"
```

---

## Task 3: View Types — ScheduleView + Input Views

### Files
- Modify: `elohim/elohim-storage/src/views.rs`

### Step 1: Add ScheduleView and input views

Follow the existing pattern with `#[derive(TS)]` and `#[serde(rename_all = "camelCase")]`:

```rust
// ============================================================================
// Schedule Views (Kairos temporal dimension)
// ============================================================================

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ScheduleView {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
    pub next_occurrence_at: Option<String>,
    pub occurrence_count: i32,
    pub created_at: String,
}

impl From<crate::db::models::Schedule> for ScheduleView {
    fn from(s: crate::db::models::Schedule) -> Self {
        Self {
            id: s.id,
            entity_type: s.entity_type,
            entity_id: s.entity_id,
            scheduled_at: s.scheduled_at,
            expires_at: s.expires_at,
            rrule: s.rrule,
            next_occurrence_at: s.next_occurrence_at,
            occurrence_count: s.occurrence_count,
            created_at: s.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateScheduleInputView {
    pub entity_type: String,
    pub entity_id: String,
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct UpdateScheduleInputView {
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
}
```

### Step 2: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5
```

### Step 3: Commit

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "feat(storage): add ScheduleView and input views with ts-rs"
```

---

## Task 4: RRULE Dependency + CRUD Module

### Files
- Modify: `elohim/elohim-storage/Cargo.toml` (add `rrule` dependency)
- Create: `elohim/elohim-storage/src/db/schedules.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` (add `pub mod schedules;`)

### Step 1: Add rrule dependency

Add to `[dependencies]` in `Cargo.toml`:

```toml
rrule = "0.14"
```

### Step 2: Read reference files first

- `elohim/elohim-storage/src/db/stewardship_allocations.rs` — CRUD pattern
- `elohim/elohim-storage/src/db/comments.rs` — simpler CRUD pattern (created in Sprint 7)

### Step 3: Create `src/db/schedules.rs`

```rust
//! Schedule CRUD operations (Kairos temporal dimension)
//!
//! Polymorphic temporal attachment for any CID-addressed entity.
//! Supports scheduling (future activation), expiry (duration),
//! and recurrence (RFC 5545 RRULE).

use diesel::prelude::*;
use rrule::RRuleSet;
use tracing::debug;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::schedules;
use super::models::{current_timestamp, NewSchedule, Schedule};
use crate::error::StorageError;

/// Input for creating a schedule
pub struct CreateScheduleInput {
    pub entity_type: String,
    pub entity_id: String,
    pub scheduled_at: Option<String>,
    pub expires_at: Option<String>,
    pub rrule: Option<String>,
}

/// Input for updating a schedule (PATCH semantics)
#[derive(Debug, Default)]
pub struct UpdateScheduleInput {
    pub scheduled_at: Option<Option<String>>,
    pub expires_at: Option<Option<String>>,
    pub rrule: Option<Option<String>>,
}

/// Query params for listing schedules
#[derive(Debug, Clone, Default)]
pub struct ScheduleQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub due_before: Option<String>,
    pub limit: Option<i64>,
}

/// Compute next occurrence from an RRULE string and an optional last occurrence.
/// Returns None if the RRULE has no more occurrences.
fn compute_next_occurrence(
    rrule_str: &str,
    after: Option<&str>,
) -> Result<Option<String>, StorageError> {
    let rrule_set: RRuleSet = format!("DTSTART:20260101T000000Z\nRRULE:{}", rrule_str)
        .parse()
        .map_err(|e| StorageError::Internal(format!("Invalid RRULE: {}", e)))?;

    let now_str = after
        .map(|s| s.to_string())
        .unwrap_or_else(current_timestamp);

    // Parse the "after" time, compute occurrences after it
    let after_dt = chrono::DateTime::parse_from_rfc3339(&now_str)
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(&now_str, "%Y-%m-%dT%H:%M:%S")
            .map(|ndt| ndt.and_utc().fixed_offset()))
        .map_err(|e| StorageError::Internal(format!("Invalid datetime for RRULE after: {}", e)))?;

    // Get occurrences — take first one after the given datetime
    let occurrences = rrule_set.after(after_dt.with_timezone(&rrule::Tz::UTC))
        .all(100)  // limit to prevent infinite iteration
        .dates;

    Ok(occurrences.first().map(|dt| dt.to_rfc3339()))
}

/// Create a new schedule
pub fn create_schedule(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateScheduleInput,
) -> Result<Schedule, StorageError> {
    let now = current_timestamp();

    // Compute next_occurrence_at from RRULE if provided
    let next_occurrence = if let Some(ref rrule_str) = input.rrule {
        compute_next_occurrence(rrule_str, None)?
    } else {
        // No recurrence — next occurrence is the scheduled_at time (if any)
        input.scheduled_at.clone()
    };

    let new = NewSchedule {
        id: Uuid::new_v4().to_string(),
        app_id: ctx.app_id().to_string(),
        entity_type: input.entity_type,
        entity_id: input.entity_id,
        scheduled_at: input.scheduled_at,
        expires_at: input.expires_at,
        rrule: input.rrule,
        last_occurred_at: None,
        next_occurrence_at: next_occurrence,
        occurrence_count: 0,
        created_at: now.clone(),
        updated_at: now,
    };

    debug!(id = %new.id, entity_type = %new.entity_type, entity_id = %new.entity_id, "Creating schedule");

    diesel::insert_into(schedules::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to create schedule: {}", e)))?;

    schedules::table
        .filter(schedules::id.eq(&new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to fetch created schedule: {}", e)))
}

/// Get schedule by entity type and entity ID
pub fn get_schedule(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    entity_type: &str,
    entity_id: &str,
) -> Result<Schedule, StorageError> {
    schedules::table
        .filter(schedules::app_id.eq(ctx.app_id()))
        .filter(schedules::entity_type.eq(entity_type))
        .filter(schedules::entity_id.eq(entity_id))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                StorageError::NotFound(format!("Schedule not found for {}:{}", entity_type, entity_id))
            }
            _ => StorageError::Internal(format!("Failed to fetch schedule: {}", e)),
        })
}

/// Get schedule by ID
pub fn get_schedule_by_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Schedule, StorageError> {
    schedules::table
        .filter(schedules::id.eq(id))
        .filter(schedules::app_id.eq(ctx.app_id()))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                StorageError::NotFound(format!("Schedule not found: {}", id))
            }
            _ => StorageError::Internal(format!("Failed to fetch schedule: {}", e)),
        })
}

/// List schedules with optional filters
pub fn list_schedules(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: ScheduleQuery,
) -> Result<Vec<Schedule>, StorageError> {
    let mut q = schedules::table
        .filter(schedules::app_id.eq(ctx.app_id()))
        .into_boxed();

    if let Some(ref entity_type) = query.entity_type {
        q = q.filter(schedules::entity_type.eq(entity_type));
    }
    if let Some(ref entity_id) = query.entity_id {
        q = q.filter(schedules::entity_id.eq(entity_id));
    }
    if let Some(ref before) = query.due_before {
        q = q.filter(schedules::next_occurrence_at.le(before));
    }
    if let Some(limit) = query.limit {
        q = q.limit(limit);
    }

    q.order(schedules::next_occurrence_at.asc())
        .load::<Schedule>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list schedules: {}", e)))
}

/// Update a schedule (PATCH semantics)
pub fn update_schedule(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    input: UpdateScheduleInput,
) -> Result<Schedule, StorageError> {
    let existing = get_schedule_by_id(conn, ctx, id)?;
    let now = current_timestamp();

    // Apply updates
    let new_scheduled_at = input.scheduled_at.unwrap_or(existing.scheduled_at.clone());
    let new_expires_at = input.expires_at.unwrap_or(existing.expires_at.clone());
    let new_rrule = input.rrule.unwrap_or(existing.rrule.clone());

    // Recompute next_occurrence if rrule changed
    let next_occurrence = if new_rrule != existing.rrule {
        if let Some(ref rrule_str) = new_rrule {
            compute_next_occurrence(rrule_str, existing.last_occurred_at.as_deref())?
        } else {
            new_scheduled_at.clone()
        }
    } else {
        existing.next_occurrence_at
    };

    diesel::update(schedules::table.filter(schedules::id.eq(id)))
        .set((
            schedules::scheduled_at.eq(&new_scheduled_at),
            schedules::expires_at.eq(&new_expires_at),
            schedules::rrule.eq(&new_rrule),
            schedules::next_occurrence_at.eq(&next_occurrence),
            schedules::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to update schedule: {}", e)))?;

    get_schedule_by_id(conn, ctx, id)
}

/// Advance a recurring schedule — bump last_occurred_at, recompute next_occurrence_at
pub fn advance_occurrence(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Schedule, StorageError> {
    let existing = get_schedule_by_id(conn, ctx, id)?;
    let now = current_timestamp();

    let next_occurrence = if let Some(ref rrule_str) = existing.rrule {
        compute_next_occurrence(rrule_str, Some(&now))?
    } else {
        None // One-time schedule — no more occurrences
    };

    diesel::update(schedules::table.filter(schedules::id.eq(id)))
        .set((
            schedules::last_occurred_at.eq(&now),
            schedules::next_occurrence_at.eq(&next_occurrence),
            schedules::occurrence_count.eq(existing.occurrence_count + 1),
            schedules::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to advance schedule: {}", e)))?;

    get_schedule_by_id(conn, ctx, id)
}
```

**IMPORTANT:** The `rrule` crate API may differ from this pseudocode. The implementing agent MUST read the `rrule` crate docs and adapt `compute_next_occurrence` to the actual API. The key contract: given an RRULE string and an "after" datetime, return the next occurrence.

### Step 4: Register module in `db/mod.rs`

Add `pub mod schedules;` alphabetically.

### Step 5: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -10
```

### Step 6: Run tests

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -10
```

### Step 7: Commit

```bash
git add elohim/elohim-storage/Cargo.toml \
       elohim/elohim-storage/src/db/schedules.rs \
       elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): add schedules CRUD module with RRULE support

create, get, list, update, advance_occurrence. RRULE parsed via rrule
crate for next_occurrence_at precomputation. list_due query for
'what's coming due?' across all entity types."
```

---

## Task 5: API Routes — Standalone + Entity-Nested

### Files
- Create: `elohim/elohim-storage/src/api/schedules.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs` (add module + routes)
- Modify: `elohim/elohim-storage/src/http.rs` (add entity-nested routes)

### Step 1: Read reference files

- `elohim/elohim-storage/src/api/comments.rs` — the REST pattern (Sprint 7)
- `elohim/elohim-storage/src/api/mod.rs` — route dispatch
- `elohim/elohim-storage/src/http.rs` — entity-nested routes at `/db/content/...`

### Step 2: Create `src/api/schedules.rs`

Follow the comments.rs pattern. Handle:
- `POST ""` — create schedule
- `GET ""` — list with query params (?entityType, ?entityId, ?dueBefore, ?limit)
- `GET "due"` — alias for list with dueBefore=now
- `GET "{id}"` — get by schedule ID
- `PATCH "{id}"` — update
- `POST "{id}/advance"` — advance occurrence

No gate evaluation — schedules are metadata.

### Step 3: Add module + route dispatch to `api/mod.rs`

Add `pub mod schedules;` and route dispatch for `"schedules"` in `handle_api_request`.

### Step 4: Add entity-nested routes in `http.rs`

In `handle_db_request`, add routes for content-nested schedule access:
- `GET /db/content/{cid}/schedule` → delegates to `schedules::get_schedule` with entity_type="content"
- `POST /db/content/{cid}/schedule` → delegates to `schedules::create_schedule` with entity_type="content"

### Step 5: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -10
```

### Step 6: Run tests

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -10
```

### Step 7: Commit

```bash
git add elohim/elohim-storage/src/api/schedules.rs \
       elohim/elohim-storage/src/api/mod.rs \
       elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add schedule API routes — standalone + entity-nested

POST/GET/PATCH /api/v1/schedules — standalone collection
GET/POST /db/content/{cid}/schedule — entity-nested convenience
POST /api/v1/schedules/{id}/advance — bump recurrence"
```

---

## Task 6: Generate TypeScript Types

### Step 1: Run type generation

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -10
```

### Step 2: Verify generated files

```bash
ls elohim/sdk/storage-client-ts/src/generated/ScheduleView.ts
ls elohim/sdk/storage-client-ts/src/generated/CreateScheduleInputView.ts
ls elohim/sdk/storage-client-ts/src/generated/UpdateScheduleInputView.ts
```

### Step 3: Add exports to barrel file

Check `elohim/sdk/storage-client-ts/src/generated/index.ts` and add:

```typescript
export type { ScheduleView } from './ScheduleView';
export type { CreateScheduleInputView } from './CreateScheduleInputView';
export type { UpdateScheduleInputView } from './UpdateScheduleInputView';
```

### Step 4: Commit

```bash
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(sdk): generate Schedule TypeScript types"
```

---

## Task 7: Angular ScheduleService

### Files
- Create: `app/elohim-app/src/app/elohim/services/schedule.service.ts`
- Create: `app/elohim-app/src/app/elohim/services/schedule.service.spec.ts`

### Step 1: Write the failing tests

```typescript
// schedule.service.spec.ts
import { TestBed } from '@angular/core/testing';
import { HttpClient, HttpParams } from '@angular/common/http';
import { of } from 'rxjs';
import { vi, describe, it, expect, beforeEach } from 'vitest';

import { ScheduleService } from './schedule.service';

describe('ScheduleService', () => {
  let service: ScheduleService;
  let httpMock: Record<string, ReturnType<typeof vi.fn>>;

  beforeEach(() => {
    httpMock = {
      get: vi.fn().mockReturnValue(of({})),
      post: vi.fn().mockReturnValue(of({})),
      patch: vi.fn().mockReturnValue(of({})),
    };

    TestBed.configureTestingModule({
      providers: [
        ScheduleService,
        { provide: HttpClient, useValue: httpMock },
      ],
    });
    service = TestBed.inject(ScheduleService);
  });

  it('should GET schedule by entity', () => {
    service.getSchedule('content', 'cid-123').subscribe();
    expect(httpMock.get).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/schedules'),
      expect.objectContaining({ params: expect.anything() }),
    );
  });

  it('should POST to create schedule', () => {
    const input = { entityType: 'content', entityId: 'cid-123', scheduledAt: '2026-03-20T09:00:00Z' };
    service.createSchedule(input).subscribe();
    expect(httpMock.post).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/schedules'),
      input,
    );
  });

  it('should PATCH to update schedule', () => {
    service.updateSchedule('sched-1', { expiresAt: '2026-04-20T00:00:00Z' }).subscribe();
    expect(httpMock.patch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/schedules/sched-1'),
      { expiresAt: '2026-04-20T00:00:00Z' },
    );
  });

  it('should GET due schedules', () => {
    service.getDueSchedules('2026-03-20T00:00:00Z').subscribe();
    expect(httpMock.get).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/schedules'),
      expect.objectContaining({ params: expect.anything() }),
    );
  });

  it('should POST to advance occurrence', () => {
    service.advanceOccurrence('sched-1').subscribe();
    expect(httpMock.post).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/schedules/sched-1/advance'),
      {},
    );
  });
});
```

### Step 2: Run tests to verify they fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "schedule.service" 2>&1 | tail -15
```

### Step 3: Write the implementation

```typescript
// schedule.service.ts
import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpParams } from '@angular/common/http';
import { Observable, catchError } from 'rxjs';

import type { ScheduleView, CreateScheduleInputView, UpdateScheduleInputView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class ScheduleService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  getSchedule(entityType: string, entityId: string): Observable<ScheduleView> {
    const params = new HttpParams()
      .set('entityType', entityType)
      .set('entityId', entityId);
    return this.http.get<ScheduleView>(`${this.baseUrl}/api/v1/schedules`, { params });
  }

  createSchedule(input: CreateScheduleInputView): Observable<ScheduleView> {
    return this.http.post<ScheduleView>(`${this.baseUrl}/api/v1/schedules`, input);
  }

  updateSchedule(id: string, patch: Partial<UpdateScheduleInputView>): Observable<ScheduleView> {
    return this.http.patch<ScheduleView>(`${this.baseUrl}/api/v1/schedules/${id}`, patch);
  }

  getDueSchedules(before?: string): Observable<ScheduleView[]> {
    let params = new HttpParams();
    if (before) {
      params = params.set('dueBefore', before);
    }
    return this.http.get<ScheduleView[]>(`${this.baseUrl}/api/v1/schedules`, { params });
  }

  advanceOccurrence(id: string): Observable<ScheduleView> {
    return this.http.post<ScheduleView>(`${this.baseUrl}/api/v1/schedules/${id}/advance`, {});
  }
}
```

### Step 4: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "schedule.service" 2>&1 | tail -15
```

### Step 5: Add barrel export

In `app/elohim-app/src/app/elohim/services/index.ts`, add:

```typescript
export { ScheduleService } from './schedule.service';
```

### Step 6: Commit

```bash
git add app/elohim-app/src/app/elohim/services/schedule.service.ts \
       app/elohim-app/src/app/elohim/services/schedule.service.spec.ts \
       app/elohim-app/src/app/elohim/services/index.ts
git commit -m "feat(elohim): add ScheduleService — thin HTTP client for temporal settings

CRUD for schedules, getDueSchedules for 'what's coming due?',
advanceOccurrence for recurrence. All RRULE logic stays in Rust."
```

---

## Task 8: Integration Verification

### Step 1: Run all Rust tests

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -15
```

### Step 2: Run Angular schedule tests

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "schedule" 2>&1 | tail -15
```

### Step 3: Run full Angular test suite

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -20
```

### Step 4: Run lint

```bash
cd app/elohim-app && pnpm run lint 2>&1 | tail -15
```

### Step 5: Verify proxy

```bash
grep -n "api" app/elohim-app/proxy.conf.mjs
```

### Step 6: Show git log

```bash
git log --oneline feature/kairos-schedules --not dev
```
