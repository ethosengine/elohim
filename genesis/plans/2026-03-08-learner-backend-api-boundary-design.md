# Learner Backend API Boundary — Design

**Date:** 2026-03-08
**Pillar:** elohim (protocol core) → lamad (learning)
**Goal:** Extract the learner-backend protocol surface into a thin API, revealing lamad's API boundary. Clean dead code from holochain-content.service.

## Context

`LearnerBackendService` (411 lines) is a pure data-boundary adapter — 20 methods wrapping zome calls with zero UX logic. It serves 3 domain services (ContentMasteryService, PracticeService, PointsService). All 20 methods are protocol-level concerns:

- **Mastery/attestation** (9 methods) — content affinity, engagement, assessment evidence
- **Practice pools & challenges** (8 methods) — content recommendation and assessment primitives
- **Learning points** (3 methods) — economic flows from learning activity (future Shefa integration)

The Rust DB layer (`content_mastery.rs`) is fully implemented. Basic `/db/mastery*` CRUD endpoints exist. New `/api/v1/mastery/*` endpoints are needed for the enriched operations (engagement, assessment, practice, challenges, points).

Separately, `holochain-content.service.ts` (1,949 lines) has 13 deprecated methods with zero consumers — dead code ready to delete.

## Workstream 1: holochain-content Dead Code Cleanup (~700 lines)

Delete 13 deprecated methods from `holochain-content.service.ts`. No consumers to rewire — confirmed zero calls to any deprecated method. Also remove associated private helpers and cache infrastructure that only served the deprecated methods.

## Workstream 2: Learner Backend Migration (~411 lines restructured)

### Rust Side — New `/api/v1/mastery/*` Endpoints

Create `src/api/mastery.rs` following the `presence.rs` controller pattern:

| Endpoint | Method | Maps to DB/Service |
|----------|--------|--------------------|
| `POST /api/v1/mastery` | initializeMastery | `upsert_mastery` |
| `POST /api/v1/mastery/engagement` | recordEngagement | `record_engagement` |
| `POST /api/v1/mastery/assessment` | recordAssessment | `advance_mastery` |
| `GET /api/v1/mastery/{contentId}` | getMyMastery | `get_mastery_for_content` |
| `GET /api/v1/mastery` | getMyAllMastery | `get_mastery_for_human` |
| `POST /api/v1/mastery/batch` | getMasteryBatch | `get_mastery_for_contents` |
| `GET /api/v1/mastery/path/{pathId}` | getPathMasteryOverview | `calculate_path_mastery` |
| `GET /api/v1/mastery/stats` | getMyMasteryStats | `stats_by_level` + aggregates |
| `POST /api/v1/mastery/check-privilege` | checkPrivilege | privilege gate logic |
| `POST /api/v1/mastery/pool` | getOrCreatePracticePool | pool CRUD |
| `POST /api/v1/mastery/pool/refresh` | refreshPracticePool | pool refresh |
| `POST /api/v1/mastery/pool/add-path` | addPathToPool | pool path add |
| `GET /api/v1/mastery/pool/recommendations` | getPoolRecommendations | pool query |
| `GET /api/v1/mastery/challenge/cooldown` | checkChallengeCooldown | cooldown check |
| `POST /api/v1/mastery/challenge/start` | startMasteryChallenge | challenge CRUD |
| `POST /api/v1/mastery/challenge/submit` | submitMasteryChallenge | challenge + level changes |
| `GET /api/v1/mastery/challenge/history` | getChallengeHistory | challenge query |
| `POST /api/v1/mastery/points/earn` | earnLamadPoints | points CRUD |
| `GET /api/v1/mastery/points/balance` | getMyLamadPointBalance | points query |
| `GET /api/v1/mastery/points/history` | getMyLamadPointHistory | points query |

View types: `ContentMasteryView` already exists. Need new View types for practice pools, challenges, points, and aggregate responses.

### Angular Side — Interface + Token + Thin Service

1. **`ILearnerBackend`** interface in `elohim/interfaces/learner-backend.interface.ts` — all 20 methods
2. **`LEARNER_BACKEND`** injection token (factory → `LearnerBackendApiService`)
3. **`learner-backend-api.service.ts`** — thin HTTP service using `HttpClient` + `firstValueFrom()` + `catchError()` with fail-open defaults
4. Rewire 3 consumers to use `LEARNER_BACKEND` token
5. Delete fat `learner-backend.service.ts`

### Wire Type Simplification

The current zome wire types use `*_json` string fields parsed client-side via `transformZomeResponse()`. The HTTP API returns pre-parsed JSON via View types (camelCase, proper booleans). The thin API service will receive ready-to-use objects — no parsing needed.

## Scorecard After Completion

| Metric | Before | After |
|--------|--------|-------|
| Thin API services | 10 | 11 |
| Fat services | 21 | 20 |
| Lines removed (dead code) | — | ~700 |
| Lines restructured | — | ~411 |
| New Rust endpoints | — | 20 |
| Lamad protocol boundary | implicit | explicit |

## P2P-First Note

The HTTP endpoints live in elohim-storage. The DHT remains the truth layer — storage queries the conductor's DHT via zome calls for operations not yet in the projection (practice pools, challenges, points). The existing `/db/mastery*` endpoints already project mastery state to SQLite for fast reads. New endpoints extend this pattern.

Points operations are economic flows that should eventually integrate with Shefa's REA economic event system. For now they route through the mastery API; future migration to `/api/v1/economic-events/*` is a separate workstream.
