# Kairos — Temporal Dimension Design

**Design philosophy:** Kairos (the right moment) is the temporal dimension of the protocol. Every artifact that enters the protocol's record can carry scheduling, duration, and recurrence. The name "Kairos" is internal design language — the API uses plain English ("schedules").

---

## Core Concept

A `schedule` is a polymorphic temporal attachment linked to any entity by `(entity_type, entity_id)`. One schedule per entity. The `entity_id` is the entity's **CID** (content address), not a database row ID — ensuring schedules travel with content across P2P peers.

Three temporal dimensions:
- **Schedule** — when does this become active? (`scheduled_at`)
- **Duration** — when does this expire? (`expires_at`)
- **Recurrence** — does this repeat? (RFC 5545 RRULE)

Conditional triggers (event-based, not time-based) are out of scope.

---

## Data Model

```sql
CREATE TABLE schedules (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    entity_type TEXT NOT NULL,        -- 'content', 'commitment', 'work-item', 'offer'
    entity_id TEXT NOT NULL,          -- CID of the entity

    -- Schedule: when does this become active?
    scheduled_at TEXT,                -- ISO 8601 (null = immediate)

    -- Duration: when does this expire?
    expires_at TEXT,                  -- ISO 8601 (null = no expiry)

    -- Recurrence: RFC 5545 RRULE (null = one-time)
    rrule TEXT,                       -- e.g. 'FREQ=MONTHLY;INTERVAL=6'

    -- Tracking
    last_occurred_at TEXT,            -- last recurrence fire
    next_occurrence_at TEXT,          -- precomputed, indexed for "what's due?" queries
    occurrence_count INTEGER NOT NULL DEFAULT 0,

    -- Metadata
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    UNIQUE(entity_type, entity_id)
);

CREATE INDEX idx_schedules_entity ON schedules (entity_type, entity_id);
CREATE INDEX idx_schedules_next ON schedules (next_occurrence_at);
CREATE INDEX idx_schedules_scheduled ON schedules (scheduled_at);
```

Key decisions:
- **One schedule per entity** (unique constraint)
- **`next_occurrence_at`** precomputed and indexed — enables "what's coming due?" without RRULE parsing at query time
- **`entity_id` is CID** — P2P-friendly, survives sync across peers
- **RRULE for recurrence** — RFC 5545, parsed by `rrule` crate in Rust. No RRULE parsing in TypeScript.

---

## API Routes (Hybrid)

### Standalone collection

```
POST   /api/v1/schedules                              — create
GET    /api/v1/schedules?entityType=X&entityId=Y       — get by entity
PATCH  /api/v1/schedules/{id}                          — update
GET    /api/v1/schedules/due?before=2026-03-20T00:00Z  — list what's due
```

### Entity-nested (convenience)

```
GET    /api/v1/content/{cid}/schedule                  — schedule for a content node
POST   /api/v1/content/{cid}/schedule                  — create schedule for content
```

Both hit the same underlying table. Entity-nested routes filter by `entity_type` + CID.

No gate on schedule CRUD — it's metadata about the entity, not the entity itself.

---

## Rust Backend

### Views

- `ScheduleView` — output (id, entityType, entityId, scheduledAt, expiresAt, rrule, nextOccurrenceAt, occurrenceCount, createdAt)
- `CreateScheduleInputView` — input (entityType, entityId, scheduledAt?, expiresAt?, rrule?)
- `UpdateScheduleInputView` — patch input (scheduledAt?, expiresAt?, rrule?)

All with `#[derive(TS)]` for TypeScript generation, `#[serde(rename_all = "camelCase")]`.

### CRUD Module (`db/schedules.rs`)

- `create_schedule(entity_type, entity_id, ...)` — creates record, computes `next_occurrence_at` from RRULE if present
- `get_schedule(entity_type, entity_id)` — fetch by entity
- `update_schedule(id, patch)` — partial update, recomputes `next_occurrence_at` if RRULE changes
- `list_due(before)` — `WHERE next_occurrence_at < before` — "what's coming due?"
- `advance_occurrence(id)` — bumps `last_occurred_at`, increments count, recomputes `next_occurrence_at`

### RRULE Dependency

`rrule = "0.13"` crate for RFC 5545 parsing and next-occurrence computation. Used in `create_schedule`, `update_schedule`, and `advance_occurrence`.

---

## Angular Frontend

### ScheduleService (elohim pillar, thin HTTP client)

```typescript
@Injectable({ providedIn: 'root' })
export class ScheduleService {
  getSchedule(entityType: string, entityId: string): Observable<ScheduleView | null>
  createSchedule(input: CreateScheduleInputView): Observable<ScheduleView>
  updateSchedule(id: string, patch: UpdateScheduleInputView): Observable<ScheduleView>
  getDueSchedules(before?: string): Observable<ScheduleView[]>
}
```

No RRULE parsing in TypeScript. Backend computes `nextOccurrenceAt` and returns it as a plain datetime.

UI components (schedule picker, recurrence selector) are future sprint — service + types only for now.

---

## Cross-Cutting Concerns

### P2P Sync
- `entity_id` is CID — when content syncs to a peer, the schedule can be synced alongside and re-linked by CID
- Schedule records themselves could be content-addressed for DHT gossip (future)

### Journal Routing (future)
- When the "finish" event fires on a journal entry, if the elohim suggests publishing, the schedule picker lets the human choose "now", "Thursday at 9am", or "review in 6 months"
- The schedule attaches to the content node CID at routing time

### Avodah (future)
- Work items get schedules for due dates, recurring tasks, sprint boundaries
- `GET /api/v1/schedules/due` powers the "what's overdue?" dashboard

---

## What This Sprint Does NOT Include

- Background job runner for auto-triggering on schedule (future — needs a tick loop)
- Conditional triggers (event-based, not time-based)
- Schedule picker UI components
- P2P schedule sync protocol
- RRULE timezone handling (assume UTC for now)

---

## Build Order

1. Diesel migration — `schedules` table
2. Models — `Schedule` + `NewSchedule` + `UpdateScheduleInput`
3. Views — `ScheduleView` + input views with ts-rs
4. RRULE dependency + CRUD module — `db/schedules.rs`
5. API routes — standalone + entity-nested hybrid
6. TypeScript type generation
7. Angular ScheduleService with tests
8. Integration verification
