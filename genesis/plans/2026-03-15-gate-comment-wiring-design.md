# Gate Comment Wiring — Design Document

**Date:** 2026-03-15
**Status:** Approved
**Sprint:** ElohimGate Sprint 7
**Depends on:** Sprint 6 (GateArtifactCardComponent, GateInteractionService, GateCommentComponent)

## Goal

Wire the gate artifact card to real HTTP calls end-to-end. Build comment persistence in the backend, add gated `POST /api/v1/comments` endpoint, and connect the Angular card to it via an injectable API callback pattern.

## Architecture

The gate artifact card receives an `@Input() gateApiCall` function from its shell. On submit, the card calls the function, receives the gated response (200/409/403), and drives the state machine from real gate data. Each shell provides the appropriate API function for its mutation type.

```
GateCommentComponent (shell)
  │ provides apiCall → storageApi.createComment()
  ▼
GateArtifactCardComponent
  │ calls gateApiCall(text, context) on submit
  ▼
GateInteractionService.submitWithApi()
  │ transitions to evaluating, makes HTTP call
  ▼
StorageApiService.createComment()
  │ POST /api/v1/comments { contentId, body }
  ▼
Rust evaluate_gate() → GateResult
  │
  ├─ 200 → { data: CommentView, gate: GateEvaluationView } → AFFIRM
  ├─ 409 → { gate: GateEvaluationView } → DIALOGUE
  └─ 403 → { gate: GateEvaluationView } → SETTLED
```

---

## Backend: Comment Data Model

### Table

```sql
CREATE TABLE comments (
    id TEXT PRIMARY KEY,
    content_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    body TEXT NOT NULL,
    reach TEXT NOT NULL DEFAULT 'close',
    governance_state TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### Views

`CommentView` — camelCase view for TypeScript generation:
- `id`, `contentId`, `humanId`, `body`, `reach`, `governanceState`, `createdAt`

`CreateCommentInput` — `contentId` and `body` only. `humanId` from auth context.

### Routes

| Method | Route | Gate | Response |
|--------|-------|------|----------|
| POST | `/api/v1/comments` | Yes | `{ data: CommentView, gate: GateEvaluationView }` |
| GET | `/api/v1/comments?contentId={id}` | No | `Vec<CommentView>` |
| GET | `/api/v1/comments/{id}` | No | `CommentView` |

POST follows the stewardship allocation pattern: calls `evaluate_gate()`, stores on pass, returns 409 on pause, 403 on settlement.

---

## Frontend: Injectable API Callback

### GateArtifactCardComponent

New input:
```typescript
@Input() gateApiCall: (text: string, context: MutationContext) => Observable<unknown>;
```

On submit, delegates to `GateInteractionService.submitWithApi()`.

### GateInteractionService

New method:
```typescript
submitWithApi(text, mutationType, context, apiCall): void
```

- Guards against double-submit (already evaluating)
- Sets draft text, transitions to evaluating
- Calls `apiCall(text, context).subscribe()`
- On 200: extracts gate from response → `handleGateEvaluation()` → affirm
- On 409/403: extracts gate from error → `handleGateEvaluation()` → dialogue/settled
- On network error: reverts to draft (user can retry)

Old `submit()` remains for testing and manual state transitions.

### StorageApiService

New method:
```typescript
createComment(contentId: string, body: string): Observable<unknown>
```

Same gate response pattern as stewardship methods: `tap(extractGateFromResponse)` + `catchError(handleGateError)`.

### GateCommentComponent

Provides the API callback:
```typescript
protected apiCall = (text: string, context: MutationContext) =>
  this.storageApi.createComment(context.contentId!, text);
```

---

## Not In Scope

- **Feedback modal surface** — next sprint, same `gateApiCall` pattern
- **Journal surface** — separate design, needs own route + sidebar layout + rich editing
- **Comment editing/deletion** — future, requires gate re-evaluation
- **Comment threading/replies** — future
- **SSE streaming** — future sprint, for async Deep/Constitutional evaluation
- **Inference sidecar deployment** — future, sidecar engine code exists but isn't deployed

---

## Build Order

1. Rust: Comment model + Diesel migration
2. Rust: Comment service (CRUD with gate)
3. Rust: Comment routes (gated POST, ungated GET)
4. Rust: Type generation (`cargo test export_bindings`)
5. Angular: `StorageApiService.createComment()`
6. Angular: `GateInteractionService.submitWithApi()`
7. Angular: `GateArtifactCardComponent` — add `gateApiCall` input
8. Angular: `GateCommentComponent` — wire API callback
9. Integration verification
