# Observation Session Diagnostic System

**Date:** 2026-04-04
**Status:** Design approved
**Scope:** elohim-storage (primary), doorway (contributor), genesis/a2o (consumer)

## Problem

A2O acceptance tests run against alpha and produce 177 failures, but the test framework has no way to correlate *why* things failed at the infrastructure level. The failures cascade (missing imagodei DNA causes auth failures, which blocks every authenticated scenario), but the test output is a flat list of assertion errors with no causal structure.

The Angular app already has feedback infrastructure (governance signals, economic events, issue reports, diagnostic bundles), but none of it is accessible to the a2o test harness. The test framework composes its own diagnostic reports from browser artifacts, duplicating logic that should live in the infrastructure.

## Design Principle

**Observation is a native infrastructure concern.** The protocol itself observes, correlates, and reports. Genesis (a2o) only activates observation and requests the report. No diagnostic composition logic in the test harness.

Feedback flows through the protocol end-to-end: the same observation infrastructure that serves a2o tests will eventually serve elohim self-healing loops and operator dashboards.

## P2P Design Gate

### Entity: ObservationSession
- **Classification**: Operational (C)
- **Justification**: Ephemeral coordination state. Disposable after report composition. No peer needs to witness or verify that a session existed.
- **Content Address Strategy**: UUID (no content to hash, not agent-scoped, ephemeral correlation key)
- **Source of Truth**: SQLite (operational)
- **Coordinator Zome**: None
- **Storage Projection**: `observation_sessions` (dht_anchor_hash: no)
- **Reconstruction Strategy**: Not needed — sessions are disposable. The report survives independently.
- **Anti-Pattern Check**: UUID justified for operational entity. No DHT entry type consumed.

### Entity: ObservationEntry
- **Classification**: Operational (C)
- **Justification**: Working data that feeds report composition. Purgeable after report is composed. Reconstructable by re-running the scenario.
- **Content Address Strategy**: Auto-increment integer (internal, never referenced externally)
- **Source of Truth**: SQLite (operational)
- **Coordinator Zome**: None
- **Storage Projection**: `observation_entries` (dht_anchor_hash: no)
- **Reconstruction Strategy**: Re-run the scenario with observation active.
- **Anti-Pattern Check**: Not putting granular request data on DHT (correct). No DHT entry type consumed.

### Entity: ObservationReport (the EPR artifact)
- **Classification**: Notarized (A) — uses existing Content entry type
- **Justification**: The report is a content artifact that operators review, elohim read for self-healing, and can be linked to avodah work-stories. Uses existing `Content` DHT entry type with `contentType: "observation-report"`. No new entry type needed.
- **Content Address Strategy**: Content-Derived (CID) — report is immutable once composed
- **Source of Truth**: Holochain DHT (via existing Content entry type)
- **Coordinator Zome**: `lamad::create_content` (existing)
- **Storage Projection**: `content` table (dht_anchor_hash: yes)
- **Anti-Pattern Check**: Reuses existing Content entry type. CID is canonical. Report composition calls coordinator to notarize, then projects.

### Design Constraints
- **No new DHT entry types consumed.** Sessions and entries are operational. Reports reuse Content.
- **Report persistence must go through the coordinator** (not direct SQLite insert) to get a valid `dht_anchor_hash`.
- **Entry purge timing**: Entries purged after report composition, not before.

## Architecture

### Observation as Aspect

An observation session is a cross-cutting concern that weaves through the request path via an `X-Observation-Id` header. Callers (a2o, operators, elohim) start a session, make requests as normal, and ask for the report when done.

```
a2o Before hook                    a2o After hook
     |                                  |
     v                                  v
POST /observations/begin  ──>  GET /observations/{id}/report
     |                                  |
     | returns sessionId                | returns composed report
     |                                  | persists as ContentNode
     v                                  v
  X-Observation-Id header        ObservationReport EPR
  travels on every request       (the addressable artifact)
```

### Data Flow

1. **a2o Before hook** calls `POST /api/v1/observations/begin` on storage, receives session ID
2. **DoorwayClient** adds `X-Observation-Id: {sessionId}` to every subsequent request
3. **Doorway** passes the header through to storage. If doorway itself generates an error before reaching storage (route miss, conductor unavailable, auth rejection), it POSTs an entry to storage with `origin: "doorway"`
4. **Storage** middleware detects the header on incoming requests and auto-appends an observation entry for each request's outcome (status code, timing, content IDs)
5. **a2o After hook** calls `GET /api/v1/observations/{sessionId}/report`
6. **Storage** reads accumulated entries, correlates by content ID and causal pattern, composes the report, persists it as a ContentNode (`contentType: "observation-report"`), and returns it
7. **a2o** writes the report JSON to `reports/observations/{scenario}.json` if errors were observed

### EPR Scope

Individual observation entries are **not** EPR content. They are working data in an `observation_entries` table, purgeable after the report is composed.

The **report** is the EPR artifact. Like a GitHub issue: the stack traces and API calls aren't each their own artifact, but the composed issue is. The report is a ContentNode with `contentType: "observation-report"`, `contentFormat: "json"`, addressable and queryable like any other content.

## Data Model

### `observation_sessions` table (elohim-storage, SQLite/Diesel)

| Column | Type | Notes |
|--------|------|-------|
| `id` | TEXT PK | UUID |
| `started_at` | TIMESTAMP | Session start |
| `ended_at` | TIMESTAMP NULL | Null while active, set when report is composed |
| `ttl_seconds` | INTEGER DEFAULT 300 | Auto-expire for forgotten sessions |
| `source` | TEXT | Who started it: "a2o", "operator", "elohim" |
| `metadata` | TEXT (JSON) | Caller context: scenario name, tags, etc. |

### `observation_entries` table

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER PK AUTOINCREMENT | |
| `session_id` | TEXT FK | References observation_sessions |
| `timestamp` | TIMESTAMP | When the observation occurred |
| `origin` | TEXT | "storage" or "doorway" |
| `category` | TEXT | "http", "auth", "content", "conductor", "route" |
| `severity` | TEXT | "info", "warning", "error" |
| `method` | TEXT NULL | HTTP method |
| `path` | TEXT NULL | Request path |
| `status_code` | INTEGER NULL | HTTP status code |
| `message` | TEXT | Human-readable description |
| `context` | TEXT (JSON) | Structured details: content IDs, error bodies, timing |

No indexes beyond `session_id`. Entries are cheap writes, purged after report composition or TTL expiry.

## API Surface (elohim-storage)

### `POST /api/v1/observations/begin`

Starts an observation session.

**Request:**
```json
{
  "source": "a2o",
  "ttlSeconds": 300,
  "metadata": {
    "scenario": "Matthew logs in as admin",
    "tags": ["@e2e", "@auth"],
    "feature": "auth-lifecycle.feature"
  }
}
```

**Response (201):**
```json
{
  "sessionId": "uuid-...",
  "expiresAt": "2026-04-04T14:05:00Z"
}
```

### `POST /api/v1/observations/{sessionId}/entries`

Internal write path for doorway contributions. Accepts single entry or array.

**Request:**
```json
{
  "origin": "doorway",
  "category": "route",
  "severity": "error",
  "method": "POST",
  "path": "/api/v1/economic-events",
  "statusCode": 405,
  "message": "Route not in manifest for POST method",
  "context": { "registryRouteCount": 47 }
}
```

**Response:** 201 (no body, fire and forget)

### `GET /api/v1/observations/{sessionId}/report`

Composes the report from accumulated entries, persists as ContentNode, returns it. Idempotent — if the session already has a report, returns the existing one without recomposing.

**Response (200):**
```json
{
  "contentId": "observation-report-uuid-...",
  "sessionId": "uuid-...",
  "source": "a2o",
  "metadata": { "scenario": "...", "tags": [...] },
  "duration": {
    "startedAt": "2026-04-04T14:00:00Z",
    "endedAt": "2026-04-04T14:00:03Z",
    "durationMs": 3000
  },
  "summary": {
    "totalEntries": 12,
    "byOrigin": { "storage": 9, "doorway": 3 },
    "bySeverity": { "info": 5, "warning": 2, "error": 5 },
    "byCategory": { "http": 4, "auth": 3, "content": 3, "route": 2 }
  },
  "issues": [
    {
      "id": "auth-401-cascade",
      "category": "auth",
      "severity": "error",
      "title": "Auth failed for all requests — credentials invalid or humans not registered",
      "entries": [ "..." ],
      "relatedContentIds": [],
      "suggestedCause": "Fixture humans not registered on alpha. Registration endpoint returns 503 (imagodei DNA missing from conductor)."
    },
    {
      "id": "content-404-love-map-adam-eve",
      "category": "content",
      "severity": "error",
      "title": "Content 'love-map-adam-eve' not found — cascading to stewardship lookup",
      "entries": [ "..." ],
      "relatedContentIds": ["love-map-adam-eve"],
      "suggestedCause": "Content not seeded on alpha."
    }
  ],
  "systemState": {
    "storageHealthy": true,
    "conductorConnected": true,
    "p2pPeerCount": 2
  }
}
```

## Issue Correlation Logic

Report composition groups entries by content ID and causal pattern, then collapses them into issues:

| Pattern | Collapse Rule | suggestedCause |
|---------|--------------|----------------|
| N entries with same status_code + path prefix | Single issue | Path-specific message |
| All auth entries are 401 | Single "auth cascade" issue | "Credentials invalid or humans not registered" |
| 404 on `/db/content/{id}` + 404 on `/db/allocations/content/{id}` | Single issue per content ID | "Content '{id}' not found — cascading to dependent lookups" |
| 405 on any path | One issue per path | "Method not allowed — check manifest route registration" |
| 503 with "imagodei" in body | Infrastructure issue | "imagodei DNA missing from conductor configuration" |

This starts as simple pattern matching on status codes and path prefixes. Grows smarter over time as elohim get involved in report interpretation.

## Doorway Contribution

Doorway's role is pass-through with contribution. It does not own observations.

**When `X-Observation-Id` header is present:**

1. Forward the header to storage (already happens via `forward_to_storage`)
2. If doorway generates an error *before* reaching storage, POST an entry to storage with `origin: "doorway"`

**What doorway observes that storage cannot:**
- Route registry misses (path not in manifest)
- Conductor pool exhaustion (no worker available)
- Auth failures at the gateway level (JWT validation, session expired)
- Proxy timeouts (storage unreachable or slow)
- Cache decisions (served from projection cache vs forwarded)

**Implementation:** A function `maybe_contribute_observation(req, response, state)` in doorway's request handler. Checks for the header, checks if the response is 4xx/5xx or notable, POSTs to storage if so. Fire-and-forget (does not block the response to the caller).

## A2O Integration

Genesis is intentionally thin. Three changes total.

### DoorwayClient (genesis/a2o/src/framework/api/doorway-client.ts)

- New property: `observationId: string | null = null`
- New method: `beginObservation(metadata)` — calls POST begin, stores session ID
- New method: `getObservationReport()` — calls GET report, returns composed report
- `headers()` method: if `observationId` is set, includes `X-Observation-Id`

### common.steps.ts Before hook

```typescript
// After existing setup
const doorway = this.getDoorway('alpha');
if (doorway) {
  const meta = { scenario: pickle.name, tags: pickle.tags.map(t => t.name) };
  await doorway.client.beginObservation(meta);
}
```

### common.steps.ts After hook

```typescript
// After existing capture logic, before cleanup
const doorway = this.getDoorway('alpha');
if (doorway?.client.observationId) {
  const report = await doorway.client.getObservationReport();
  if (report.summary.bySeverity.error > 0) {
    writeFileSync(
      `reports/observations/${safeName}.json`,
      JSON.stringify(report, null, 2)
    );
  }
}
```

Works in both device modes (HTTP-only and Playwright). The observation ID lives on DoorwayClient, not on any device.

## Storage Middleware (the Aspect)

In elohim-storage's request handler, before returning a response:

```
if request has X-Observation-Id header:
  if session exists and is active:
    append entry with:
      origin: "storage"
      category: infer from path ("/db/content" → "content", "/auth" → "auth", etc.)
      severity: infer from status code (2xx → "info", 4xx → "warning", 5xx → "error")
      method, path, status_code from the request/response
      message: response body summary for non-2xx
      context: { contentId if extractable from path, timing }
```

This is the core of the aspect pattern. No caller changes needed. Every request through storage is observed if the header is present.

## Content Type

The observation report is persisted as:

```json
{
  "contentType": "observation-report",
  "contentFormat": "json",
  "title": "Observation: Matthew logs in as admin",
  "tags": ["observation-report", "a2o", "auth"],
  "contentBody": "{ ... serialized report ... }"
}
```

This means observation reports are queryable via the existing content APIs, can appear in search results, and can be referenced by EPR. Elohim can read them for self-healing. Operators can browse them in the dashboard.

## Future Hooks (Not In Scope)

- **Avodah work-story template:** Observation reports could serve as templates for avodah work-stories, with the current steward (operator/developer) as the assignee. This would make outstanding issues browsable as a backlog — the report becomes the ticket.
- **Elohim self-healing:** Elohim reads observation reports, identifies recurring patterns, proposes infrastructure changes
- **Operator dashboard:** Browse observation reports, filter by category/severity, track resolution
- **Protocol-artifact issue reports:** Observation reports feed into the existing IssueReportService pattern
- **Continuous observation:** Always-on mode for production monitoring (current design is session-scoped/opt-in)

## Implementation Order

1. Diesel migration: `observation_sessions` + `observation_entries` tables
2. Storage API: begin, entries, report endpoints
3. Storage middleware: `X-Observation-Id` aspect in request handler
4. Report composition: correlation logic, ContentNode persistence
5. Doorway contribution: `maybe_contribute_observation` function
6. A2O integration: DoorwayClient + Before/After hooks
7. Manifest registration: Add observation routes to `build_manifest()`
