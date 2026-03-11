# Learner Backend API Boundary — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract learner-backend's 20 protocol methods into a thin API behind `LEARNER_BACKEND` token, create Rust HTTP endpoints, and delete 13 dead deprecated methods from holochain-content.service.

**Architecture:** Two workstreams. WS1: delete dead deprecated methods from holochain-content.service (zero consumers, pure cleanup). WS2: create Rust `/api/v1/mastery/*` endpoints → Angular `ILearnerBackend` interface + `LEARNER_BACKEND` token + `LearnerBackendApiService` thin HTTP client → rewire 3 consumers → delete fat service.

**Tech Stack:** Rust (hyper, diesel, serde, ts-rs), Angular 19 (HttpClient, InjectionToken, inject()), Vitest

---

## Workstream 1: holochain-content Dead Code Cleanup

### Task 1: Delete deprecated methods from holochain-content.service

**Files:**
- Modify: `elohim-app/src/app/elohim/services/holochain-content.service.ts`

**Context:** 13 methods are marked `@deprecated` with zero consumers calling them. They span lines ~929-1650. Each has associated private helpers. The `contentCache` Map at the top of the service only serves the deprecated `getContent`/`batchGetContent` methods.

**Step 1: Verify zero consumers**

Run the following searches to confirm no code calls the deprecated methods:

```bash
cd /projects/elohim/elohim-app
```

Search for each deprecated method name in the codebase (excluding the service file itself and its spec):

```
getContent(        → only in holochain-content.service.ts itself
batchGetContent(   → only in holochain-content.service.ts itself
getContentByType(  → only in holochain-content.service.ts itself
getStats(          → only in holochain-content.service.ts itself
getPathIndex(      → only in holochain-content.service.ts itself
getPathWithSteps(  → only in holochain-content.service.ts itself
getPathOverviewRest( → only in holochain-content.service.ts itself
getRelationships(  → check carefully — may exist in StorageApiService (different method)
getContentGraph(   → only in holochain-content.service.ts itself
getKnowledgeMapById( → only in holochain-content.service.ts itself
queryKnowledgeMaps(  → only in holochain-content.service.ts itself
getPathExtensionById( → only in holochain-content.service.ts itself
queryPathExtensions(  → only in holochain-content.service.ts itself
```

Use `grep -rn "methodName" src/ --include="*.ts" | grep -v holochain-content.service` for each.

**Step 2: Delete the deprecated methods**

Remove these 13 deprecated public methods and their associated private helpers:
- `getContent()` (~line 929) + private helpers `separateUncachedIds`, `resolveCachedItems`, `fetchUncachedAndMerge`, `fetchContentById`
- `batchGetContent()` (~line 958)
- `getContentByType()` (~line 1088)
- `getStats()` (~line 1209)
- `getPathIndex()` (~line 1268)
- `getPathWithSteps()` (~line 1288)
- `getPathOverviewRest()` (~line 1314)
- `getRelationships()` (~line 1511)
- `getContentGraph()` (~line 1531)
- `getKnowledgeMapById()` (~line 1563)
- `queryKnowledgeMaps()` (~line 1583)
- `getPathExtensionById()` (~line 1607)
- `queryPathExtensions()` (~line 1627)

Also remove:
- The `contentCache` Map property (only used by deprecated getContent/batchGetContent)
- Any imports that become unused after deletion (e.g., ContentNode, HolochainPathIndex, etc.)
- Any private helper methods only called by the deprecated methods

Keep ALL non-deprecated methods intact (agents, attestations, content-attestations, governance).

**Step 3: Run tests**

```bash
cd /projects/elohim/elohim-app
pnpm exec vitest run --config vite.config.ts "holochain-content.service"
```

Expected: existing non-deprecated method tests still pass.

**Step 4: Run lint**

```bash
pnpm run lint
```

Fix any unused import warnings.

**Step 5: Verify build**

```bash
pnpm run build
```

Expected: clean build with no errors.

**Step 6: Commit**

```bash
git add elohim-app/src/app/elohim/services/holochain-content.service.ts
git commit -m "refactor(elohim): delete 13 deprecated methods from holochain-content.service

All deprecated content CRUD methods had zero consumers — the migration
to StorageApiService/ContentService is complete. Removes ~700 lines of
dead code including content cache infrastructure.

Active methods (agents, attestations, governance) remain intact."
```

---

## Workstream 2: Learner Backend Migration

### Task 2: Create Rust mastery API controller — mastery CRUD endpoints

**Files:**
- Create: `holochain/elohim-storage/src/api/mastery.rs`
- Modify: `holochain/elohim-storage/src/api/mod.rs`

**Context:** Follow the `presence.rs` controller pattern exactly. The controller dispatches based on HTTP method + path. DB functions in `db/content_mastery.rs` provide the underlying operations. View type `ContentMasteryView` in `views.rs` handles the camelCase boundary.

**Step 1: Create the mastery controller file**

Create `holochain/elohim-storage/src/api/mastery.rs`:

```rust
//! Mastery API controller
//!
//! Routes: `/api/v1/mastery[/{contentId}][/engagement|/assessment|/batch|/stats|/check-privilege|/path/{pathId}|/pool|/challenge|/points]`
//!
//! Provides the learner protocol surface: mastery tracking, practice pools,
//! challenges, and learning points. This is lamad's API boundary.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::content_mastery::{self, CreateMasteryInput, MasteryQuery};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::ContentMasteryView;

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Request types (camelCase API boundary)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializeMasteryRequest {
    pub content_id: String,
    #[serde(default)]
    pub human_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordEngagementRequest {
    pub content_id: String,
    pub engagement_type: String,
    #[serde(default)]
    pub human_id: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordAssessmentRequest {
    pub content_id: String,
    pub assessment_type: String,
    pub score: f32,
    pub passing_threshold: f32,
    #[serde(default)]
    pub human_id: Option<String>,
    #[serde(default)]
    pub time_spent_seconds: Option<i32>,
    #[serde(default)]
    pub question_count: Option<i32>,
    #[serde(default)]
    pub correct_count: Option<i32>,
    #[serde(default)]
    pub evidence: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchMasteryRequest {
    pub content_ids: Vec<String>,
    #[serde(default)]
    pub human_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckPrivilegeRequest {
    pub content_id: String,
    pub privilege: String,
    #[serde(default)]
    pub human_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn extract_id(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let id = trimmed.split('/').next()?;
    if id.is_empty() { None } else { Some(id) }
}

fn extract_action(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    let mut parts = trimmed.splitn(2, '/');
    let _ = parts.next();
    parts.next().filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// Mastery CRUD handlers
// ---------------------------------------------------------------------------

async fn initialize_mastery(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: InitializeMasteryRequest = parse_body(req).await?;
    let human_id = body.human_id.unwrap_or_else(|| "anonymous".to_string());

    let input = CreateMasteryInput {
        id: None,
        human_id,
        content_id: body.content_id,
        mastery_level: "not_started".to_string(),
        content_version_at_mastery: None,
    };

    let mut conn = get_conn(pool)?;
    let mastery = content_mastery::upsert_mastery(&mut conn, ctx, input)?;
    Ok(response::created(&ContentMasteryView::from(mastery)))
}

async fn record_engagement(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: RecordEngagementRequest = parse_body(req).await?;
    let human_id = body.human_id.unwrap_or_else(|| "anonymous".to_string());

    let mut conn = get_conn(pool)?;
    let mastery = content_mastery::record_engagement(
        &mut conn, ctx, &human_id, &body.content_id, &body.engagement_type,
    )?;
    Ok(response::ok(&ContentMasteryView::from(mastery)))
}

async fn record_assessment(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: RecordAssessmentRequest = parse_body(req).await?;
    let human_id = body.human_id.unwrap_or_else(|| "anonymous".to_string());

    // Determine new level based on score vs threshold
    let new_level = if body.score >= body.passing_threshold {
        "understand" // Advance on pass
    } else {
        "remember" // Stay at remember on fail
    };

    let evidence = body.evidence
        .map(|v| serde_json::to_string(&v))
        .transpose()
        .map_err(|e| StorageError::InvalidInput(format!("Invalid evidence: {}", e)))?;

    let mut conn = get_conn(pool)?;
    let advancement = content_mastery::advance_mastery(
        &mut conn, ctx, &human_id, &body.content_id, new_level, evidence.as_deref(),
    )?;
    Ok(response::ok(&ContentMasteryView::from(advancement.mastery)))
}

async fn get_my_mastery(
    content_id: &str,
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Extract human_id from query params
    let query_str = req.uri().query().unwrap_or("");
    let params: std::collections::HashMap<String, String> =
        serde_urlencoded::from_str(query_str).unwrap_or_default();
    let human_id = params.get("humanId").cloned().unwrap_or_else(|| "anonymous".to_string());

    let mut conn = get_conn(pool)?;
    let result = content_mastery::get_mastery_for_content(&mut conn, ctx, &human_id, content_id)?;
    Ok(response::from_option(
        Ok(result.map(ContentMasteryView::from)),
        &format!("Mastery for content {} not found", content_id),
    ))
}

async fn get_all_mastery(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query_str = req.uri().query().unwrap_or("");
    let query: MasteryQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = get_conn(pool)?;
    let results = content_mastery::list_mastery(&mut conn, ctx, &query)?;
    let views: Vec<ContentMasteryView> = results.into_iter().map(ContentMasteryView::from).collect();
    Ok(response::ok(&views))
}

async fn get_mastery_batch(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: BatchMasteryRequest = parse_body(req).await?;
    let human_id = body.human_id.unwrap_or_else(|| "anonymous".to_string());

    let mut conn = get_conn(pool)?;
    let results = content_mastery::get_mastery_for_contents(
        &mut conn, ctx, &human_id, &body.content_ids,
    )?;
    let views: Vec<ContentMasteryView> = results.into_iter().map(ContentMasteryView::from).collect();
    Ok(response::ok(&views))
}

async fn get_path_mastery_overview(
    path_id: &str,
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query_str = req.uri().query().unwrap_or("");
    let params: std::collections::HashMap<String, String> =
        serde_urlencoded::from_str(query_str).unwrap_or_default();
    let human_id = params.get("humanId").cloned().unwrap_or_else(|| "anonymous".to_string());

    let mut conn = get_conn(pool)?;
    // Use empty content_ids — the DB function resolves from path
    let summary = content_mastery::calculate_path_mastery(
        &mut conn, ctx, &human_id, path_id, &[],
    )?;
    Ok(response::ok(&summary))
}

async fn get_mastery_stats(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query_str = req.uri().query().unwrap_or("");
    let params: std::collections::HashMap<String, String> =
        serde_urlencoded::from_str(query_str).unwrap_or_default();
    let _human_id = params.get("humanId").cloned().unwrap_or_else(|| "anonymous".to_string());

    let mut conn = get_conn(pool)?;
    let by_level = content_mastery::stats_by_level(&mut conn, ctx)?;
    let total = content_mastery::mastery_count(&mut conn, ctx)?;
    let refresh_needed = content_mastery::refresh_needed_count(&mut conn, ctx)?;
    let avg_freshness = content_mastery::average_freshness(&mut conn, ctx)?;

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct MasteryStats {
        total_tracked: i64,
        level_distribution: Vec<(String, i64)>,
        needs_refresh_count: i64,
        average_freshness: f64,
    }

    Ok(response::ok(&MasteryStats {
        total_tracked: total,
        level_distribution: by_level,
        needs_refresh_count: refresh_needed,
        average_freshness: avg_freshness,
    }))
}

async fn check_privilege(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: CheckPrivilegeRequest = parse_body(req).await?;
    let human_id = body.human_id.unwrap_or_else(|| "anonymous".to_string());

    let mut conn = get_conn(pool)?;
    let mastery = content_mastery::get_mastery_for_content(
        &mut conn, ctx, &human_id, &body.content_id,
    )?;

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PrivilegeResult {
        has_privilege: bool,
        current_level: String,
        current_level_index: i32,
        privilege: String,
    }

    let (has_privilege, level, index) = match mastery {
        Some(m) => {
            // Simple privilege check: mastery level >= required
            let has = m.mastery_level_index >= 2; // "understand" or higher
            (has, m.mastery_level, m.mastery_level_index)
        }
        None => (false, "not_started".to_string(), 0),
    };

    Ok(response::ok(&PrivilegeResult {
        has_privilege,
        current_level: level,
        current_level_index: index,
        privilege: body.privilege,
    }))
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/mastery*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let trimmed = resource_path.trim_end_matches('/');

    match (&method, trimmed) {
        // GET /api/v1/mastery — list all mastery
        (&Method::GET, "") | (&Method::GET, "/") => get_all_mastery(req, pool, ctx).await,

        // POST /api/v1/mastery — initialize mastery
        (&Method::POST, "") | (&Method::POST, "/") => initialize_mastery(req, pool, ctx).await,

        // POST /api/v1/mastery/engagement — record engagement
        (&Method::POST, "/engagement") => record_engagement(req, pool, ctx).await,

        // POST /api/v1/mastery/assessment — record assessment
        (&Method::POST, "/assessment") => record_assessment(req, pool, ctx).await,

        // POST /api/v1/mastery/batch — batch query
        (&Method::POST, "/batch") => get_mastery_batch(req, pool, ctx).await,

        // GET /api/v1/mastery/stats — stats dashboard
        (&Method::GET, "/stats") => get_mastery_stats(req, pool, ctx).await,

        // POST /api/v1/mastery/check-privilege — privilege gate
        (&Method::POST, "/check-privilege") => check_privilege(req, pool, ctx).await,

        // Practice pool routes
        (&Method::POST, "/pool") | (&Method::POST, "/pool/refresh") | (&Method::POST, "/pool/add-path") => {
            // TODO: Practice pool endpoints — requires new DB tables/functions
            Ok(response::not_found("Practice pool endpoints not yet implemented"))
        }
        (&Method::GET, "/pool/recommendations") => {
            Ok(response::not_found("Practice pool endpoints not yet implemented"))
        }

        // Challenge routes
        (&Method::GET, "/challenge/cooldown") | (&Method::POST, "/challenge/start") |
        (&Method::POST, "/challenge/submit") | (&Method::GET, "/challenge/history") => {
            // TODO: Challenge endpoints — requires new DB tables/functions
            Ok(response::not_found("Challenge endpoints not yet implemented"))
        }

        // Points routes
        (&Method::POST, "/points/earn") | (&Method::GET, "/points/balance") |
        (&Method::GET, "/points/history") => {
            // TODO: Points endpoints — future Shefa integration
            Ok(response::not_found("Points endpoints not yet implemented"))
        }

        // Routes with content ID
        _ => {
            // Check for /path/{pathId} pattern
            if trimmed.starts_with("/path/") {
                let path_id = trimmed.strip_prefix("/path/").unwrap_or("");
                if !path_id.is_empty() && method == Method::GET {
                    return get_path_mastery_overview(path_id, req, pool, ctx).await;
                }
            }

            // Default: treat as /{contentId}
            let content_id = match extract_id(trimmed) {
                Some(id) => id,
                None => return Ok(response::not_found(&format!(
                    "Unknown mastery route: {} {}", method, resource_path
                ))),
            };

            match &method {
                &Method::GET => get_my_mastery(content_id, req, pool, ctx).await,
                _ => Ok(response::not_found(&format!(
                    "Unknown mastery route: {} {}", method, resource_path
                ))),
            }
        }
    }
}
```

**Step 2: Register in api/mod.rs**

Add `pub mod mastery;` to the module declarations and add the route to `handle_api_request()`:

In `holochain/elohim-storage/src/api/mod.rs`, add to module list:

```rust
pub mod mastery;
```

Add dispatch branch before the `else` fallback:

```rust
    } else if sub_path.starts_with("mastery") {
        let resource_path = sub_path.strip_prefix("mastery").unwrap_or("");
        mastery::handle(req, method, resource_path, &pool, &app_ctx).await
```

**Step 3: Verify Rust build**

```bash
cd /projects/elohim/holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -20
```

Expected: clean build. Fix any compile errors.

**Step 4: Run Rust tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -20
```

**Step 5: Commit**

```bash
git add holochain/elohim-storage/src/api/mastery.rs holochain/elohim-storage/src/api/mod.rs
git commit -m "feat(elohim-storage): add /api/v1/mastery/* endpoints for lamad protocol boundary

9 mastery CRUD endpoints implemented: initialize, engagement, assessment,
get, list, batch, path overview, stats, check-privilege.

Practice pool, challenge, and points endpoints stubbed as TODO —
require new DB tables. Points will integrate with Shefa economics."
```

---

### Task 3: Create ILearnerBackend interface and LEARNER_BACKEND token

**Files:**
- Create: `elohim-app/src/app/elohim/interfaces/learner-backend.interface.ts`
- Modify: `elohim-app/src/app/elohim/interfaces/index.ts`

**Context:** Follow the exact pattern from `shefa/interfaces/economic-event-factory.interface.ts`. Interface defines the contract, token factory defaults to the thin service.

**Step 1: Create the interface file**

Create `elohim-app/src/app/elohim/interfaces/learner-backend.interface.ts`:

```typescript
/**
 * ILearnerBackend — Abstract interface for lamad learning protocol operations.
 *
 * Covers three protocol domains:
 * - Mastery: content engagement, assessment, privilege gates
 * - Practice: pools, recommendations, challenges
 * - Points: learning point economics (future Shefa integration)
 *
 * Consumers inject the LEARNER_BACKEND token; the default factory
 * resolves to LearnerBackendApiService (thin HTTP client).
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class ContentMasteryService {
 *   private readonly backend = inject(LEARNER_BACKEND);
 *
 *   async recordView(contentId: string) {
 *     return this.backend.recordEngagement({
 *       content_id: contentId,
 *       engagement_type: 'view',
 *     });
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { LearnerBackendApiService } from '../services/learner-backend-api.service';

import type {
  ContentMasteryOutput,
  RecordEngagementInput,
  RecordAssessmentInput,
  MasterySnapshot,
  PathMasteryOverview,
  MasteryStatsWire,
  CheckPrivilegeInput,
  PrivilegeCheckResult,
} from '@app/lamad/models/content-mastery.model';
import type {
  LearnerPointBalanceOutput,
  LamadPointEventOutput,
  EarnLamadPointsInput,
  EarnLamadPointsResult,
} from '@app/lamad/models/learning-points.model';
import type {
  PracticePoolOutput,
  CreatePoolInput,
  PoolRecommendations,
  CooldownCheckResult,
  MasteryChallengeOutput,
  StartChallengeInput,
  SubmitChallengeInput,
  ChallengeResult,
} from '@app/lamad/models/practice.model';

export interface ILearnerBackend {
  // =========================================================================
  // Connection
  // =========================================================================
  isAvailable(): boolean;

  // =========================================================================
  // Content Mastery
  // =========================================================================
  initializeMastery(contentId: string): Promise<ContentMasteryOutput | null>;
  recordEngagement(input: RecordEngagementInput): Promise<ContentMasteryOutput | null>;
  recordAssessment(input: RecordAssessmentInput): Promise<ContentMasteryOutput | null>;
  getMyMastery(contentId: string): Promise<ContentMasteryOutput | null>;
  getMyAllMastery(): Promise<ContentMasteryOutput[]>;
  getMasteryBatch(contentIds: string[]): Promise<MasterySnapshot[]>;
  getPathMasteryOverview(pathId: string): Promise<PathMasteryOverview | null>;
  getMyMasteryStats(): Promise<MasteryStatsWire | null>;
  checkPrivilege(input: CheckPrivilegeInput): Promise<PrivilegeCheckResult | null>;

  // =========================================================================
  // Practice Pool
  // =========================================================================
  getOrCreatePracticePool(input: CreatePoolInput): Promise<PracticePoolOutput | null>;
  refreshPracticePool(): Promise<PracticePoolOutput | null>;
  addPathToPool(pathId: string): Promise<PracticePoolOutput | null>;
  getPoolRecommendations(): Promise<PoolRecommendations | null>;
  checkChallengeCooldown(): Promise<CooldownCheckResult | null>;

  // =========================================================================
  // Mastery Challenges
  // =========================================================================
  startMasteryChallenge(input: StartChallengeInput): Promise<MasteryChallengeOutput | null>;
  submitMasteryChallenge(input: SubmitChallengeInput): Promise<ChallengeResult | null>;
  getChallengeHistory(): Promise<MasteryChallengeOutput[]>;

  // =========================================================================
  // Learning Points (Future Shefa)
  // =========================================================================
  earnLamadPoints(input: EarnLamadPointsInput): Promise<EarnLamadPointsResult | null>;
  getMyLamadPointBalance(): Promise<LearnerPointBalanceOutput | null>;
  getMyLamadPointHistory(limit?: number): Promise<LamadPointEventOutput[]>;
}

/**
 * Injection token for learner backend operations.
 *
 * Default factory resolves to LearnerBackendApiService which calls
 * `/api/v1/mastery/*` HTTP endpoints in elohim-storage.
 *
 * Override in tests:
 * ```typescript
 * { provide: LEARNER_BACKEND, useValue: mockBackend }
 * ```
 */
export const LEARNER_BACKEND = new InjectionToken<ILearnerBackend>('LearnerBackend', {
  providedIn: 'root',
  factory: () => inject(LearnerBackendApiService),
});
```

**Step 2: Export from barrel**

Add to `elohim-app/src/app/elohim/interfaces/index.ts`:

```typescript
export type { ILearnerBackend } from './learner-backend.interface';
export { LEARNER_BACKEND } from './learner-backend.interface';
```

**Step 3: Verify no circular imports**

The interface imports `LearnerBackendApiService` (which doesn't exist yet — will be created in Task 4). This file won't compile until Task 4 is complete. That's OK — we'll verify the full chain after Task 4.

**Step 4: Commit (combined with Task 4)**

This task's commit is deferred to Task 4 to avoid a broken intermediate state.

---

### Task 4: Create LearnerBackendApiService (thin HTTP client)

**Files:**
- Create: `elohim-app/src/app/elohim/services/learner-backend-api.service.ts`

**Context:** Follow the `EconomicEventsApiService` pattern: `HttpClient` + `firstValueFrom()` + `catchError()`. Each method maps to an `/api/v1/mastery/*` endpoint. Returns fail-open defaults (null or []) on error.

**Step 1: Create the thin service**

Create `elohim-app/src/app/elohim/services/learner-backend-api.service.ts`:

```typescript
/**
 * LearnerBackendApiService — Thin HTTP client for lamad protocol operations.
 *
 * Calls elohim-storage `/api/v1/mastery/*` endpoints, implementing
 * ILearnerBackend. Replaces the fat LearnerBackendService that made
 * direct zome calls.
 *
 * Fail-open: all methods return null/[] on error (graceful degradation).
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError, of, firstValueFrom } from 'rxjs';

import type { ILearnerBackend } from '../interfaces/learner-backend.interface';
import type {
  ContentMasteryOutput,
  RecordEngagementInput,
  RecordAssessmentInput,
  MasterySnapshot,
  PathMasteryOverview,
  MasteryStatsWire,
  CheckPrivilegeInput,
  PrivilegeCheckResult,
} from '@app/lamad/models/content-mastery.model';
import type {
  LearnerPointBalanceOutput,
  LamadPointEventOutput,
  EarnLamadPointsInput,
  EarnLamadPointsResult,
} from '@app/lamad/models/learning-points.model';
import type {
  PracticePoolOutput,
  CreatePoolInput,
  PoolRecommendations,
  CooldownCheckResult,
  MasteryChallengeOutput,
  StartChallengeInput,
  SubmitChallengeInput,
  ChallengeResult,
} from '@app/lamad/models/practice.model';

const BASE = '/api/v1/mastery';

@Injectable({ providedIn: 'root' })
export class LearnerBackendApiService implements ILearnerBackend {
  private readonly http = inject(HttpClient);

  // =========================================================================
  // Connection
  // =========================================================================

  isAvailable(): boolean {
    return true; // HTTP is always "available" — errors handled per-request
  }

  // =========================================================================
  // Content Mastery
  // =========================================================================

  async initializeMastery(contentId: string): Promise<ContentMasteryOutput | null> {
    return firstValueFrom(
      this.http.post<ContentMasteryOutput>(BASE, { contentId }).pipe(catchError(() => of(null)))
    );
  }

  async recordEngagement(input: RecordEngagementInput): Promise<ContentMasteryOutput | null> {
    return firstValueFrom(
      this.http.post<ContentMasteryOutput>(`${BASE}/engagement`, input).pipe(catchError(() => of(null)))
    );
  }

  async recordAssessment(input: RecordAssessmentInput): Promise<ContentMasteryOutput | null> {
    return firstValueFrom(
      this.http.post<ContentMasteryOutput>(`${BASE}/assessment`, input).pipe(catchError(() => of(null)))
    );
  }

  async getMyMastery(contentId: string): Promise<ContentMasteryOutput | null> {
    return firstValueFrom(
      this.http.get<ContentMasteryOutput>(`${BASE}/${encodeURIComponent(contentId)}`).pipe(
        catchError(() => of(null))
      )
    );
  }

  async getMyAllMastery(): Promise<ContentMasteryOutput[]> {
    return firstValueFrom(
      this.http.get<ContentMasteryOutput[]>(BASE).pipe(catchError(() => of([])))
    );
  }

  async getMasteryBatch(contentIds: string[]): Promise<MasterySnapshot[]> {
    return firstValueFrom(
      this.http.post<MasterySnapshot[]>(`${BASE}/batch`, { contentIds }).pipe(catchError(() => of([])))
    );
  }

  async getPathMasteryOverview(pathId: string): Promise<PathMasteryOverview | null> {
    return firstValueFrom(
      this.http.get<PathMasteryOverview>(`${BASE}/path/${encodeURIComponent(pathId)}`).pipe(
        catchError(() => of(null))
      )
    );
  }

  async getMyMasteryStats(): Promise<MasteryStatsWire | null> {
    return firstValueFrom(
      this.http.get<MasteryStatsWire>(`${BASE}/stats`).pipe(catchError(() => of(null)))
    );
  }

  async checkPrivilege(input: CheckPrivilegeInput): Promise<PrivilegeCheckResult | null> {
    return firstValueFrom(
      this.http.post<PrivilegeCheckResult>(`${BASE}/check-privilege`, input).pipe(
        catchError(() => of(null))
      )
    );
  }

  // =========================================================================
  // Practice Pool
  // =========================================================================

  async getOrCreatePracticePool(input: CreatePoolInput): Promise<PracticePoolOutput | null> {
    return firstValueFrom(
      this.http.post<PracticePoolOutput>(`${BASE}/pool`, input).pipe(catchError(() => of(null)))
    );
  }

  async refreshPracticePool(): Promise<PracticePoolOutput | null> {
    return firstValueFrom(
      this.http.post<PracticePoolOutput>(`${BASE}/pool/refresh`, {}).pipe(catchError(() => of(null)))
    );
  }

  async addPathToPool(pathId: string): Promise<PracticePoolOutput | null> {
    return firstValueFrom(
      this.http.post<PracticePoolOutput>(`${BASE}/pool/add-path`, { pathId }).pipe(
        catchError(() => of(null))
      )
    );
  }

  async getPoolRecommendations(): Promise<PoolRecommendations | null> {
    return firstValueFrom(
      this.http.get<PoolRecommendations>(`${BASE}/pool/recommendations`).pipe(
        catchError(() => of(null))
      )
    );
  }

  async checkChallengeCooldown(): Promise<CooldownCheckResult | null> {
    return firstValueFrom(
      this.http.get<CooldownCheckResult>(`${BASE}/challenge/cooldown`).pipe(
        catchError(() => of(null))
      )
    );
  }

  // =========================================================================
  // Mastery Challenges
  // =========================================================================

  async startMasteryChallenge(input: StartChallengeInput): Promise<MasteryChallengeOutput | null> {
    return firstValueFrom(
      this.http.post<MasteryChallengeOutput>(`${BASE}/challenge/start`, input).pipe(
        catchError(() => of(null))
      )
    );
  }

  async submitMasteryChallenge(input: SubmitChallengeInput): Promise<ChallengeResult | null> {
    return firstValueFrom(
      this.http.post<ChallengeResult>(`${BASE}/challenge/submit`, input).pipe(
        catchError(() => of(null))
      )
    );
  }

  async getChallengeHistory(): Promise<MasteryChallengeOutput[]> {
    return firstValueFrom(
      this.http.get<MasteryChallengeOutput[]>(`${BASE}/challenge/history`).pipe(
        catchError(() => of([]))
      )
    );
  }

  // =========================================================================
  // Learning Points (Future Shefa)
  // =========================================================================

  async earnLamadPoints(input: EarnLamadPointsInput): Promise<EarnLamadPointsResult | null> {
    return firstValueFrom(
      this.http.post<EarnLamadPointsResult>(`${BASE}/points/earn`, input).pipe(
        catchError(() => of(null))
      )
    );
  }

  async getMyLamadPointBalance(): Promise<LearnerPointBalanceOutput | null> {
    return firstValueFrom(
      this.http.get<LearnerPointBalanceOutput>(`${BASE}/points/balance`).pipe(
        catchError(() => of(null))
      )
    );
  }

  async getMyLamadPointHistory(limit?: number): Promise<LamadPointEventOutput[]> {
    const params = limit ? `?limit=${limit}` : '';
    return firstValueFrom(
      this.http.get<LamadPointEventOutput[]>(`${BASE}/points/history${params}`).pipe(
        catchError(() => of([]))
      )
    );
  }
}
```

**Step 2: Verify build compiles (interface + service together)**

```bash
cd /projects/elohim/elohim-app
pnpm run build 2>&1 | tail -20
```

Expected: clean build. The interface and service reference each other via the token factory.

**Step 3: Commit interface + service together**

```bash
git add elohim-app/src/app/elohim/interfaces/learner-backend.interface.ts \
       elohim-app/src/app/elohim/interfaces/index.ts \
       elohim-app/src/app/elohim/services/learner-backend-api.service.ts
git commit -m "feat(elohim): add ILearnerBackend interface + LEARNER_BACKEND token + thin HTTP service

ILearnerBackend defines 20 protocol methods (mastery, practice, points).
LEARNER_BACKEND injection token defaults to LearnerBackendApiService
which calls /api/v1/mastery/* endpoints in elohim-storage.

This reveals lamad's API boundary as an explicit protocol surface."
```

---

### Task 5: Rewire consumers to LEARNER_BACKEND token

**Files:**
- Modify: `elohim-app/src/app/lamad/services/content-mastery.service.ts`
- Modify: `elohim-app/src/app/lamad/services/practice.service.ts`
- Modify: `elohim-app/src/app/lamad/services/points.service.ts`
- Modify: `elohim-app/src/app/lamad/services/content-mastery.service.spec.ts`
- Modify: `elohim-app/src/app/lamad/services/practice.service.spec.ts`
- Modify: `elohim-app/src/app/lamad/services/points.service.spec.ts`

**Context:** Each consumer currently does `inject(LearnerBackendService)`. Change to `inject(LEARNER_BACKEND)`. Update specs to provide mock via token.

**Step 1: Rewire content-mastery.service.ts**

In `elohim-app/src/app/lamad/services/content-mastery.service.ts`:

Change import:
```typescript
// Before:
import { LearnerBackendService } from '@app/elohim';
// After:
import { LEARNER_BACKEND } from '@app/elohim/interfaces';
```

Change injection (~line 77):
```typescript
// Before:
private readonly backend = inject(LearnerBackendService);
// After:
private readonly backend = inject(LEARNER_BACKEND);
```

**Step 2: Rewire practice.service.ts**

In `elohim-app/src/app/lamad/services/practice.service.ts`:

Change import:
```typescript
// Before:
import { LearnerBackendService } from '@app/elohim';
// After:
import { LEARNER_BACKEND } from '@app/elohim/interfaces';
```

Change injection (~line 75):
```typescript
// Before:
private readonly backend = inject(LearnerBackendService);
// After:
private readonly backend = inject(LEARNER_BACKEND);
```

**Step 3: Rewire points.service.ts**

In `elohim-app/src/app/lamad/services/points.service.ts`:

Change import:
```typescript
// Before:
import { LearnerBackendService } from '@app/elohim';
// After:
import { LEARNER_BACKEND } from '@app/elohim/interfaces';
```

Change injection (~line 67):
```typescript
// Before:
private readonly backend = inject(LearnerBackendService);
// After:
private readonly backend = inject(LEARNER_BACKEND);
```

**Step 4: Update test specs**

For each spec file, update the provider from the concrete class to the token:

```typescript
// Before:
{ provide: LearnerBackendService, useValue: mockBackend }
// After:
{ provide: LEARNER_BACKEND, useValue: mockBackend }
```

And update imports accordingly:
```typescript
// Before:
import { LearnerBackendService } from '@app/elohim';
// After:
import { LEARNER_BACKEND } from '@app/elohim/interfaces';
```

**Step 5: Run tests**

```bash
cd /projects/elohim/elohim-app
pnpm exec vitest run --config vite.config.ts "content-mastery.service"
pnpm exec vitest run --config vite.config.ts "practice.service"
pnpm exec vitest run --config vite.config.ts "points.service"
```

Expected: all tests pass with the token-based injection.

**Step 6: Run full test suite**

```bash
pnpm exec vitest run --config vite.config.ts
```

Expected: all ~7,200 tests pass.

**Step 7: Commit**

```bash
git add elohim-app/src/app/lamad/services/content-mastery.service.ts \
       elohim-app/src/app/lamad/services/content-mastery.service.spec.ts \
       elohim-app/src/app/lamad/services/practice.service.ts \
       elohim-app/src/app/lamad/services/practice.service.spec.ts \
       elohim-app/src/app/lamad/services/points.service.ts \
       elohim-app/src/app/lamad/services/points.service.spec.ts
git commit -m "refactor(lamad): rewire 3 consumers from LearnerBackendService to LEARNER_BACKEND token

ContentMasteryService, PracticeService, and PointsService now inject
the LEARNER_BACKEND token instead of the concrete class. Tests updated
to provide mocks via the same token."
```

---

### Task 6: Delete fat LearnerBackendService

**Files:**
- Delete: `elohim-app/src/app/elohim/services/learner-backend.service.ts`
- Delete: `elohim-app/src/app/elohim/services/learner-backend.service.spec.ts`
- Modify: `elohim-app/src/app/elohim/services/index.ts`

**Step 1: Verify zero remaining consumers of the concrete class**

Search for any remaining imports of `LearnerBackendService`:

```bash
cd /projects/elohim/elohim-app
grep -rn "LearnerBackendService" src/ --include="*.ts" | grep -v learner-backend.service
```

Expected: zero results (all consumers rewired to token in Task 5).

**Step 2: Delete the fat service and spec**

Delete:
- `elohim-app/src/app/elohim/services/learner-backend.service.ts` (411 lines)
- `elohim-app/src/app/elohim/services/learner-backend.service.spec.ts`

**Step 3: Update barrel export**

In `elohim-app/src/app/elohim/services/index.ts`, remove:

```typescript
export { LearnerBackendService } from './learner-backend.service';
```

Add (if not already present):
```typescript
export { LearnerBackendApiService } from './learner-backend-api.service';
```

**Step 4: Run full test suite**

```bash
pnpm exec vitest run --config vite.config.ts
```

Expected: all tests pass.

**Step 5: Run build**

```bash
pnpm run build
```

Expected: clean build.

**Step 6: Commit**

```bash
git add -u elohim-app/src/app/elohim/services/
git commit -m "refactor(elohim): delete fat LearnerBackendService (411 lines)

Replaced by LearnerBackendApiService behind LEARNER_BACKEND token.
All 3 consumers (ContentMasteryService, PracticeService, PointsService)
already rewired to the token. Zero remaining consumers of concrete class."
```

---

### Task 7: Final verification and proxy config

**Files:**
- Modify: `elohim-app/proxy.conf.mjs` (if `/api/v1/mastery` not already proxied)

**Step 1: Verify proxy config covers mastery routes**

Read `elohim-app/proxy.conf.mjs` and check if `/api` is already proxied. The existing config proxies `/api` which should cover `/api/v1/mastery/*`. If not, add it.

**Step 2: Run full test suite one final time**

```bash
cd /projects/elohim/elohim-app
pnpm exec vitest run --config vite.config.ts
```

**Step 3: Run lint**

```bash
pnpm run lint
```

**Step 4: Verify build**

```bash
pnpm run build
```

**Step 5: Verify Rust build**

```bash
cd /projects/elohim/holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5
```

**Step 6: Summary commit (if any remaining changes)**

If proxy config was updated:
```bash
git add elohim-app/proxy.conf.mjs
git commit -m "chore(elohim-app): ensure proxy covers /api/v1/mastery routes"
```
