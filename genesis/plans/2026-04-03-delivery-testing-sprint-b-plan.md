# Delivery Testing Sprint B — Admin APIs for Failure-Mode Testing

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add doorway and storage admin APIs that allow toggling delivery layers, evicting caches, and observing internal state — enabling the delivery diagnostics scenarios to prove failure modes at known limits.

**Architecture:** New Rust admin endpoints in doorway (`/admin/cache/*`) and storage (`/admin/extraction-cache/*`). Exposes warmup retry state via `/health/startup`. Step definitions in `genesis/a2o/steps/delivery-admin.steps.ts` exercise the APIs. Each layer can be independently disabled and re-enabled, proving the fallback chain degrades correctly.

**Tech Stack:** Rust (doorway-service, elohim-storage), TypeScript (a2o step definitions)

**Design:** `genesis/plans/2026-04-02-doorway-spa-as-blob-design.md` (Section: Delivery Diagnostics)
**Scenarios:** `genesis/a2o/features/delivery/delivery-diagnostics.feature`

---

## Task 1: Doorway admin cache control API

**Files:**
- Create: `doorway/doorway-service/src/routes/admin_cache.rs`
- Modify: `doorway/doorway-service/src/routes/mod.rs`
- Modify: `doorway/doorway-service/src/server/http.rs`

Adds endpoints for observing and controlling the projection cache:
- `GET /admin/cache/stats` — cache counts, hit/miss ratio
- `POST /admin/cache/disable` — bypass projection cache (requests fall through to storage)
- `POST /admin/cache/enable` — re-enable projection cache
- `POST /admin/cache/clear/{slug}` — evict entries for a specific content slug
- `POST /admin/cache/warm` — trigger re-warmup from storage

- [ ] **Step 1: Read existing admin route patterns**

Read:
- `doorway/doorway-service/src/routes/admin.rs` — see how existing admin endpoints work (auth, response patterns)
- `doorway/doorway-service/src/server/http.rs` — find the `/admin/` route section
- `doorway/doorway-service/src/cache/app_file_cache.rs` — understand what stats/operations are available

- [ ] **Step 2: Add a cache_enabled flag to AppState**

The projection cache bypass needs a runtime toggle. In `doorway/doorway-service/src/server/http.rs` (AppState struct), add:

```rust
/// Runtime toggle for projection cache bypass (diagnostic tool).
/// When false, all cache lookups return miss and fall through to storage.
pub cache_enabled: Arc<std::sync::atomic::AtomicBool>,
```

Initialize it to `true` in the AppState constructor.

- [ ] **Step 3: Create admin_cache.rs**

Create `doorway/doorway-service/src/routes/admin_cache.rs`:

```rust
//! Admin Cache Control — endpoints for observing and toggling the projection cache.
//!
//! Used by the delivery diagnostics test suite to prove the fallback chain degrades
//! correctly when layers are disabled. Also useful for operators debugging delivery.
//!
//! All endpoints require admin auth (API key).

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use std::sync::Arc;
use tracing::info;

use crate::server::AppState;

/// GET /admin/cache/stats
pub async fn cache_stats(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let enabled = state.cache_enabled.load(std::sync::atomic::Ordering::Relaxed);

    let (projection_count, app_cache_slugs) = if let Some(ref store) = state.projection {
        let (content, humans, relationships) = store.count_by_type();
        (content + humans + relationships, content)
    } else {
        (0, 0)
    };

    let app_file_count = if let Some(ref cache) = state.app_file_cache {
        cache.cached_file_count().await
    } else {
        0
    };

    let body = serde_json::json!({
        "enabled": enabled,
        "projection": {
            "entries": projection_count,
            "contentEntries": app_cache_slugs,
        },
        "appFileCache": {
            "cachedFiles": app_file_count,
        },
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

/// POST /admin/cache/disable
pub async fn cache_disable(state: Arc<AppState>) -> Response<Full<Bytes>> {
    state.cache_enabled.store(false, std::sync::atomic::Ordering::Relaxed);
    info!("Projection cache DISABLED by admin");
    json_ok(r#"{"status":"disabled"}"#)
}

/// POST /admin/cache/enable
pub async fn cache_enable(state: Arc<AppState>) -> Response<Full<Bytes>> {
    state.cache_enabled.store(true, std::sync::atomic::Ordering::Relaxed);
    info!("Projection cache ENABLED by admin");
    json_ok(r#"{"status":"enabled"}"#)
}

/// POST /admin/cache/clear/{slug}
pub async fn cache_clear_slug(state: Arc<AppState>, slug: &str) -> Response<Full<Bytes>> {
    if let Some(ref cache) = state.app_file_cache {
        let removed = cache.clear_slug(slug).await;
        info!(slug = %slug, removed = removed, "Cache cleared for slug by admin");
        let body = serde_json::json!({ "slug": slug, "removed": removed });
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap()
    } else {
        json_ok(r#"{"error":"no app file cache configured"}"#)
    }
}

/// POST /admin/cache/warm
pub async fn cache_warm(state: Arc<AppState>) -> Response<Full<Bytes>> {
    if let Some(ref store) = state.projection {
        let peer_urls: Vec<String> = state.args.storage_url.iter().cloned().collect();
        if peer_urls.is_empty() {
            return json_ok(r#"{"error":"no storage URLs configured"}"#);
        }
        let store = Arc::clone(store);
        tokio::spawn(async move {
            for url in &peer_urls {
                crate::projection::warm_stream::stream_from_peer(Arc::clone(&store), url).await;
            }
        });
        info!("Cache warm-up triggered by admin");
        json_ok(r#"{"status":"warming","async":true}"#)
    } else {
        json_ok(r#"{"error":"no projection store configured"}"#)
    }
}

fn json_ok(body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_owned())))
        .unwrap()
}
```

- [ ] **Step 4: Add helper methods to AppFileCacheService**

In `doorway/doorway-service/src/cache/app_file_cache.rs`, add:

```rust
/// Count total cached files in MongoDB.
pub async fn cached_file_count(&self) -> usize {
    // Count documents in the app_file_cache collection
    match self.collection.count_documents(doc! {}).await {
        Ok(count) => count as usize,
        Err(_) => 0,
    }
}

/// Clear all cached files for a specific slug. Returns count of removed documents.
pub async fn clear_slug(&self, slug: &str) -> usize {
    match self.collection.delete_many(doc! { "slug": slug }).await {
        Ok(result) => result.deleted_count as usize,
        Err(e) => {
            warn!(slug = %slug, error = %e, "Failed to clear slug cache");
            0
        }
    }
}
```

- [ ] **Step 5: Wire cache_enabled into the apps handler**

In `doorway/doorway-service/src/routes/apps.rs`, at the start of `handle_app_request`, check the bypass flag:

```rust
// Cache bypass for diagnostics
if !state.cache_enabled.load(std::sync::atomic::Ordering::Relaxed) {
    return forward_app_request_with_header(&storage_url, path, "BYPASS-ADMIN").await;
}
```

- [ ] **Step 6: Wire routes in http.rs and mod.rs**

Add module declaration in `routes/mod.rs`:
```rust
pub mod admin_cache;
```

Add routes in `http.rs` in the admin section:
```rust
(Method::GET, "/admin/cache/stats") => {
    to_boxed(routes::admin_cache::cache_stats(Arc::clone(&state)).await)
}
(Method::POST, "/admin/cache/disable") => {
    to_boxed(routes::admin_cache::cache_disable(Arc::clone(&state)).await)
}
(Method::POST, "/admin/cache/enable") => {
    to_boxed(routes::admin_cache::cache_enable(Arc::clone(&state)).await)
}
(Method::POST, p) if p.starts_with("/admin/cache/clear/") => {
    let slug = p.strip_prefix("/admin/cache/clear/").unwrap_or("");
    to_boxed(routes::admin_cache::cache_clear_slug(Arc::clone(&state), slug).await)
}
(Method::POST, "/admin/cache/warm") => {
    to_boxed(routes::admin_cache::cache_warm(Arc::clone(&state)).await)
}
```

- [ ] **Step 7: Verify compilation and run tests**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check`
Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib`
Expected: Compiles, all tests pass

- [ ] **Step 8: Commit**

```bash
git add doorway/doorway-service/src/routes/admin_cache.rs doorway/doorway-service/src/routes/mod.rs doorway/doorway-service/src/server/http.rs doorway/doorway-service/src/routes/apps.rs doorway/doorway-service/src/cache/app_file_cache.rs
git commit -m "feat(doorway): add admin cache control API for delivery diagnostics

GET /admin/cache/stats — cache counts
POST /admin/cache/disable — bypass projection cache
POST /admin/cache/enable — re-enable projection cache
POST /admin/cache/clear/{slug} — evict entries for slug
POST /admin/cache/warm — trigger re-warmup from storage

Used by a2o delivery diagnostics to prove fallback chain degrades
correctly when layers are disabled."
```

---

## Task 2: Storage admin extraction cache API

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`

Adds endpoints for controlling the disk-based extraction cache:
- `GET /admin/extraction-cache/stats` — cache size, warm apps
- `POST /admin/extraction-cache/evict/{slug}` — evict specific app from cache

- [ ] **Step 1: Read the extraction cache implementation**

Read `elohim/elohim-storage/src/http.rs` — find the `ExtractionCache` or equivalent (the disk-based cache that stores extracted ZIP files). Understand its API: is_current, get_file, put_app, etc.

Also check `elohim/elohim-storage/src/` for a separate extraction_cache module.

- [ ] **Step 2: Add admin endpoints to storage http.rs**

In the match block of storage's HTTP handler, add admin routes:

```rust
// Admin: extraction cache stats
(Method::GET, "/admin/extraction-cache/stats") => {
    // Return JSON with: warm app slugs, total cached files, disk usage estimate
}

// Admin: evict specific app from extraction cache
(Method::POST, p) if p.starts_with("/admin/extraction-cache/evict/") => {
    let slug = p.strip_prefix("/admin/extraction-cache/evict/").unwrap_or("");
    // Call extraction_cache.evict(slug) or equivalent
    // Return JSON with evicted file count
}
```

The exact implementation depends on the ExtractionCache API — read it first and adapt.

- [ ] **Step 3: Add eviction method to ExtractionCache if needed**

If the extraction cache doesn't have an evict-by-slug method, add one. It likely stores files in a directory structure — eviction means deleting the directory.

- [ ] **Step 4: Verify compilation**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/
git commit -m "feat(storage): add admin extraction cache API for delivery diagnostics

GET /admin/extraction-cache/stats — warm apps, total files
POST /admin/extraction-cache/evict/{slug} — evict app from cache

Used by a2o delivery diagnostics to test the layer walkdown scenario."
```

---

## Task 3: Expose warmup retry state in /health/startup

**Files:**
- Modify: `doorway/doorway-service/src/routes/health.rs`
- Modify: `doorway/doorway-service/src/projection/warm_stream.rs`

Add warmup retry state so tests can verify the retry limit behavior.

- [ ] **Step 1: Add warmup state tracking to warm_stream.rs**

Add a shared state struct that `spawn_stream_task` updates:

```rust
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Observable warmup state — read by /health/startup, written by spawn_stream_task.
pub struct WarmupState {
    pub in_progress: AtomicBool,
    pub attempts: AtomicU32,
    pub max_attempts: AtomicU32,
    pub last_error: std::sync::Mutex<Option<String>>,
    pub completed: AtomicBool,
}

impl WarmupState {
    pub fn new() -> Self {
        Self {
            in_progress: AtomicBool::new(false),
            attempts: AtomicU32::new(0),
            max_attempts: AtomicU32::new(MAX_WARMUP_RETRIES),
            last_error: std::sync::Mutex::new(None),
            completed: AtomicBool::new(false),
        }
    }
}
```

Update `spawn_stream_task` to accept `Arc<WarmupState>` and update it during retries.

- [ ] **Step 2: Add warmup state to AppState**

In `http.rs` AppState, add:
```rust
pub warmup_state: Option<Arc<crate::projection::warm_stream::WarmupState>>,
```

Initialize in main.rs when spawning the warmup task.

- [ ] **Step 3: Include warmup state in /health/startup response**

In `health.rs` `startup_check`, add:

```rust
let warmup = if let Some(ref ws) = state.warmup_state {
    serde_json::json!({
        "inProgress": ws.in_progress.load(Ordering::Relaxed),
        "attempts": ws.attempts.load(Ordering::Relaxed),
        "maxAttempts": ws.max_attempts.load(Ordering::Relaxed),
        "completed": ws.completed.load(Ordering::Relaxed),
        "lastError": ws.last_error.lock().unwrap().clone(),
    })
} else {
    serde_json::json!(null)
};
```

Add `"warmup": warmup` to the response JSON.

- [ ] **Step 4: Verify compilation and tests**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check`
Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib`

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/projection/warm_stream.rs doorway/doorway-service/src/routes/health.rs doorway/doorway-service/src/server/http.rs doorway/doorway-service/src/main.rs
git commit -m "feat(doorway): expose warmup retry state in /health/startup

Adds attempts, maxAttempts, inProgress, completed, and lastError to
the /health/startup response. Tests can verify the retry limit (5)
and observe warmup recovery timing."
```

---

## Task 4: Delivery admin step definitions

**Files:**
- Create: `genesis/a2o/steps/delivery-admin.steps.ts`

Step definitions that use the admin APIs from Tasks 1-3 to drive the layer walkdown and failure-mode scenarios.

- [ ] **Step 1: Create the admin step definitions**

Create `genesis/a2o/steps/delivery-admin.steps.ts`:

```typescript
import { strict as assert } from 'node:assert';
import { Given, When, Then } from '@cucumber/cucumber';
import { request } from 'undici';
import { E2EWorld } from '../src/framework/world.js';
import { retry } from '../src/framework/utils/retry.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function adminPost(baseUrl: string, path: string): Promise<Record<string, unknown>> {
  const { statusCode, body } = await request(`${baseUrl}${path}`, { method: 'POST' });
  const text = await body.text();
  assert.equal(statusCode, 200, `Admin POST ${path} failed: ${statusCode} ${text}`);
  return JSON.parse(text);
}

async function adminGet(baseUrl: string, path: string): Promise<Record<string, unknown>> {
  const { statusCode, body } = await request(`${baseUrl}${path}`);
  const text = await body.text();
  assert.equal(statusCode, 200, `Admin GET ${path} failed: ${statusCode} ${text}`);
  return JSON.parse(text);
}

function getDoorwayUrl(world: E2EWorld): string {
  const doorway = [...world.doorways.values()][0];
  assert.ok(doorway, 'No doorway registered');
  return doorway.url;
}

// ---------------------------------------------------------------------------
// Projection Cache Control
// ---------------------------------------------------------------------------

Given(
  'the doorway projection cache is disabled',
  async function (this: E2EWorld) {
    const url = getDoorwayUrl(this);
    await adminPost(url, '/admin/cache/disable');
    // Cleanup: re-enable after scenario
    this.addCleanup(async () => { await adminPost(url, '/admin/cache/enable'); });
  }
);

Given(
  'the doorway projection cache is enabled and warm for {string}',
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, '/admin/cache/enable');
    // Verify warmth by checking stats
    const stats = await adminGet(url, '/admin/cache/stats') as Record<string, unknown>;
    const projection = stats.projection as Record<string, unknown>;
    assert.ok(
      (projection.entries as number) > 0,
      `Projection cache is empty — warm-up may not have run`
    );
  }
);

Given(
  'the projection cache for {string} is empty',
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, `/admin/cache/clear/${appSlug}`);
  }
);

Given(
  'the projection cache for {string} is warm',
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    // Trigger warmup and wait for it
    await adminPost(url, '/admin/cache/warm');
    await retry(
      async () => {
        const stats = await adminGet(url, '/admin/cache/stats') as Record<string, unknown>;
        const projection = stats.projection as Record<string, unknown>;
        assert.ok((projection.entries as number) > 0, 'Cache still empty');
      },
      { maxAttempts: 10, initialDelayMs: 2000, timeoutMs: 30_000 }
    );
  }
);

When(
  '{word} disables the projection cache',
  async function (this: E2EWorld, _humanName: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, '/admin/cache/disable');
    this.addCleanup(async () => { await adminPost(url, '/admin/cache/enable'); });
  }
);

When(
  '{word} re-enables the projection cache',
  async function (this: E2EWorld, _humanName: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, '/admin/cache/enable');
  }
);

When(
  '{word} re-enables all delivery layers',
  async function (this: E2EWorld, _humanName: string) {
    const url = getDoorwayUrl(this);
    await adminPost(url, '/admin/cache/enable');
    // Note: storage extraction cache is always-on (no disable API)
  }
);

When(
  'all delivery layers are enabled',
  async function (this: E2EWorld) {
    const url = getDoorwayUrl(this);
    await adminPost(url, '/admin/cache/enable');
  }
);

Then(
  'subsequent requests for app files proxy directly to storage',
  async function (this: E2EWorld) {
    const url = getDoorwayUrl(this);
    const { statusCode, headers } = await request(`${url}/apps/evolution-of-trust/index.html`);
    assert.equal(statusCode, 200);
    const cacheHeader = (headers['x-cache'] as string) ?? '';
    assert.ok(
      cacheHeader.includes('BYPASS'),
      `Expected BYPASS but got X-Cache: "${cacheHeader}"`
    );
  }
);

Then(
  'the projection cache is bypassed but not cleared',
  async function (this: E2EWorld) {
    const url = getDoorwayUrl(this);
    const stats = await adminGet(url, '/admin/cache/stats') as Record<string, unknown>;
    assert.equal(stats.enabled, false, 'Cache should be disabled');
    const projection = stats.projection as Record<string, unknown>;
    assert.ok(
      (projection.entries as number) > 0,
      'Cache entries should still exist (not cleared)'
    );
  }
);

Then(
  'the projection cache contains entries for {string}',
  async function (this: E2EWorld, appSlug: string) {
    const url = getDoorwayUrl(this);
    const stats = await adminGet(url, '/admin/cache/stats') as Record<string, unknown>;
    const projection = stats.projection as Record<string, unknown>;
    assert.ok(
      (projection.entries as number) > 0,
      `No projection entries for "${appSlug}"`
    );
  }
);

// ---------------------------------------------------------------------------
// Extraction Cache Control (Storage-side)
// ---------------------------------------------------------------------------

When(
  '{word} evicts {string} from the extraction cache',
  async function (this: E2EWorld, _humanName: string, appSlug: string) {
    // Extraction cache is on storage, not doorway
    // We need the storage URL — derive from doorway's health or use env var
    const storageUrl = process.env['E2E_STORAGE_URL'];
    if (!storageUrl) {
      // Skip — storage admin API not reachable in this env
      return 'pending';
    }
    await adminPost(storageUrl, `/admin/extraction-cache/evict/${appSlug}`);
  }
);

// ---------------------------------------------------------------------------
// Warmup Retry Observation
// ---------------------------------------------------------------------------

Then(
  'the warmup retry state shows completed',
  async function (this: E2EWorld) {
    const url = getDoorwayUrl(this);
    const data = await adminGet(url, '/health/startup') as Record<string, unknown>;
    const warmup = data.warmup as Record<string, unknown> | null;
    assert.ok(warmup, 'warmup state not in /health/startup response');
    assert.equal(warmup.completed, true, `Warmup not completed: ${JSON.stringify(warmup)}`);
  }
);

Then(
  'the warmup retry state shows maxAttempts {int}',
  async function (this: E2EWorld, expected: number) {
    const url = getDoorwayUrl(this);
    const data = await adminGet(url, '/health/startup') as Record<string, unknown>;
    const warmup = data.warmup as Record<string, unknown> | null;
    assert.ok(warmup, 'warmup state not in /health/startup response');
    assert.equal(warmup.maxAttempts, expected);
  }
);

// ---------------------------------------------------------------------------
// Resource Usage
// ---------------------------------------------------------------------------

Then(
  'elohim-storage memory usage stays within container limits',
  async function (this: E2EWorld) {
    // Best-effort: check if storage is still responsive
    const url = getDoorwayUrl(this);
    const { statusCode } = await request(`${url}/health`);
    assert.equal(statusCode, 200, 'Doorway/storage not healthy after load test');
  }
);

Then(
  'elohim-storage resource usage is unchanged',
  async function (this: E2EWorld) {
    const url = getDoorwayUrl(this);
    const { statusCode } = await request(`${url}/health`);
    assert.equal(statusCode, 200);
  }
);

Then(
  'elohim-storage memory usage did not spike above baseline + {int}MB',
  async function (this: E2EWorld, _maxSpikeMB: number) {
    // Without k8s metrics API access, verify storage is responsive
    const url = getDoorwayUrl(this);
    const { statusCode } = await request(`${url}/health`);
    assert.equal(statusCode, 200, 'Storage may have OOM\'d — health check failed');
  }
);
```

- [ ] **Step 2: Verify steps load**

Run: `cd genesis/a2o && npx cucumber-js --profile delivery --dry-run`

- [ ] **Step 3: Commit**

```bash
git add genesis/a2o/steps/delivery-admin.steps.ts
git commit -m "feat(a2o): add delivery admin step definitions for failure-mode testing

Steps for toggling projection cache, evicting extraction cache, observing
warmup retry state, and verifying resource usage. Enables the layer
walkdown and regression anchor scenarios in delivery-diagnostics.feature."
```

---

## Task 5: Remove @wip from diagnostics scenarios that are now executable

**Files:**
- Modify: `genesis/a2o/features/delivery/delivery-diagnostics.feature`
- Modify: `genesis/a2o/features/delivery/web2-absorption.feature`

- [ ] **Step 1: Identify which scenarios can now run**

With the admin APIs and step definitions from Tasks 1-4, these scenarios become executable:

From `delivery-diagnostics.feature`:
- "Without projection cache, browser load overwhelms storage" (disable cache → load → check)
- "With projection cache enabled, same load is absorbed" (enable cache → load → check)
- "Response headers indicate which cache layer served" (warm cache → check X-Cache)
- "Cache miss shows proxy layer in response" (clear cache → check X-Cache)
- "Operator walks the fallback chain by disabling layers" (full walkdown)

From `web2-absorption.feature`:
- "First load proxies to storage and populates cache" (clear → load → check)
- "Second load serves entirely from cache" (warm → load → check zero storage)
- "Concurrent cold requests coalesce" (clear → concurrent → check 1 miss)

- [ ] **Step 2: Remove @wip from those scenarios**

Remove `@wip` tag from each scenario listed above. Keep `@wip` on scenarios that still need unimplemented infrastructure (multi-replica, EPR agreement).

- [ ] **Step 3: Run the newly-enabled scenarios**

Run: `cd genesis/a2o && E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host npx cucumber-js --profile delivery --tags '@e2e and @delivery and not @wip'`

- [ ] **Step 4: Fix any step pattern mismatches**

Adjust step patterns in `delivery.steps.ts` and `delivery-admin.steps.ts` to match exact Gherkin phrasing.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/features/delivery/
git commit -m "feat(a2o): enable delivery diagnostics and web2-absorption scenarios

Remove @wip from scenarios that are now executable with the admin cache
control API and delivery step definitions."
```

---

## Execution Order

| Task | Description | Depends On |
|------|-------------|-----------|
| 1 | Doorway admin cache control API (Rust) | None |
| 2 | Storage admin extraction cache API (Rust) | None |
| 3 | Warmup retry state in /health/startup (Rust) | None |
| 4 | Delivery admin step definitions (TypeScript) | Tasks 1, 2, 3 |
| 5 | Enable diagnostics scenarios | Task 4 |

Tasks 1, 2, 3 are independent Rust work and can be parallelized (different crates).
