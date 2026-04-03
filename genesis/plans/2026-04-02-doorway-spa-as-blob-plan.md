# Doorway SPA-as-Blob Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make doorway serve the Angular SPA from blob storage — one origin, no nginx container, service worker ZIP delivery on cold cache.

**Architecture:** Add `spa-bundle` content format to protocol schema. Doorway resolves a root app slug from its extraction cache, serves extracted files for static assets and `index.html` for SPA routes. Bootstrap page with live status serves during cold start. Warmup retry ensures projection cache recovers from pod restart timing.

**Tech Stack:** Rust (doorway-service), Angular 19 (iframe-renderer), JSON Schema, K8s Ingress

**Design:** `genesis/plans/2026-04-02-doorway-spa-as-blob-design.md`

---

## Task 1: Add `spa-bundle` Content Format to Protocol Schema

**Files:**
- Modify: `elohim/sdk/schemas/v1/enums/content-format.schema.json:7-14`
- Modify: `elohim/sdk/domains/lamad/manifest.json:672-682`

- [ ] **Step 1: Add `spa-bundle` to protocol schema enum**

In `elohim/sdk/schemas/v1/enums/content-format.schema.json`, add `"spa-bundle"` to the enum array:

```json
"enum": [
  "markdown", "html", "video", "audio", "interactive", "external", "epr-composite",
  "plaintext", "text", "plain", "gherkin",
  "perseus", "perseus-json", "perseus-quiz-json",
  "video-embed", "audio-file", "html5-app",
  "human-json", "organization-json", "json",
  "sophia", "sophia-quiz-json",
  "spa-bundle"
],
```

- [ ] **Step 2: Add `spa-bundle` to lamad manifest**

In `elohim/sdk/domains/lamad/manifest.json`, add the `spa-bundle` entry in the `contentFormats` section, after the `html5-app` entry (after line 677):

```json
"spa-bundle": {
  "description": "Single-page application served as a doorway's root web surface. ZIP archive extracted and cached by doorway with SPA routing (unmatched paths serve index.html).",
  "renderer": null,
  "mimeTypes": ["application/zip", "application/x-zip-compressed"],
  "extensions": ["zip"]
},
```

- [ ] **Step 3: Run schema validation**

Run: `pnpm run schema:validate`
Expected: PASS (new enum value doesn't break existing content)

Run: `pnpm run schema:check-dna`
Expected: PASS (DNA constants don't need `spa-bundle` — it's an app-layer format)

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/schemas/v1/enums/content-format.schema.json elohim/sdk/domains/lamad/manifest.json
git commit -m "feat(schema): add spa-bundle content format for doorway root app serving"
```

---

## Task 2: Add `ROOT_APP_SLUG` Config to Doorway

**Files:**
- Modify: `doorway/doorway-service/src/config.rs:128-131`

- [ ] **Step 1: Add the env var field to Args struct**

In `doorway/doorway-service/src/config.rs`, add after the `storage_url` field (after line 131):

```rust
    /// Slug of the content node to serve as the root SPA.
    /// When set, doorway serves the SPA at `/` instead of redirecting to `/threshold`.
    /// The content node must have contentFormat "spa-bundle" and a valid blobHash.
    #[arg(long, env = "ROOT_APP_SLUG")]
    pub root_app_slug: Option<String>,
```

- [ ] **Step 2: Verify doorway compiles**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check`
Expected: Compiles with no errors (clap derives the new arg automatically)

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/src/config.rs
git commit -m "feat(doorway): add ROOT_APP_SLUG config for root SPA serving"
```

---

## Task 3: Warmup Retry with Exponential Backoff

**Files:**
- Modify: `doorway/doorway-service/src/projection/warm_stream.rs:226-265`

- [ ] **Step 1: Write test for retry logic**

In `doorway/doorway-service/src/projection/warm_stream.rs`, add to the existing `#[cfg(test)]` module (after line 299):

```rust
    #[test]
    fn test_retry_delay_calculation() {
        let base_delay_secs: u64 = 10;
        let max_delay_secs: u64 = 120;

        // attempt 0: 10s
        assert_eq!(base_delay_secs.saturating_mul(2u64.pow(0)).min(max_delay_secs), 10);
        // attempt 1: 20s
        assert_eq!(base_delay_secs.saturating_mul(2u64.pow(1)).min(max_delay_secs), 20);
        // attempt 2: 40s
        assert_eq!(base_delay_secs.saturating_mul(2u64.pow(2)).min(max_delay_secs), 40);
        // attempt 3: 80s
        assert_eq!(base_delay_secs.saturating_mul(2u64.pow(3)).min(max_delay_secs), 80);
        // attempt 4: capped at 120s
        assert_eq!(base_delay_secs.saturating_mul(2u64.pow(4)).min(max_delay_secs), 120);
    }
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib warm_stream::tests::test_retry_delay -- --nocapture`
Expected: PASS

- [ ] **Step 3: Rewrite `spawn_stream_task` with retry logic**

Replace the function at lines 226-265 with:

```rust
/// Maximum retry attempts per peer before giving up.
const MAX_WARMUP_RETRIES: u32 = 5;

/// Base delay between retries (doubles each attempt).
const WARMUP_RETRY_BASE_SECS: u64 = 10;

/// Maximum delay between retries.
const WARMUP_RETRY_MAX_SECS: u64 = 120;

/// Spawn cache stream warm-up for multiple peers as a background task.
/// This is the startup entry point — called from main.rs.
///
/// Retries with exponential backoff if storage is unreachable (e.g., pod
/// restart timing where doorway boots before storage).
pub fn spawn_stream_task(
    store: Arc<ProjectionStore>,
    storage_urls: Vec<String>,
    delay_secs: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Let services settle before streaming
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;

        info!(
            peer_count = storage_urls.len(),
            "Starting cache stream warm-up"
        );

        for storage_url in &storage_urls {
            let mut attempt: u32 = 0;

            loop {
                let result = stream_from_peer(Arc::clone(&store), storage_url).await;

                let has_content = result.content_count > 0
                    || result.human_count > 0
                    || result.relationship_count > 0;

                if result.errors.is_empty() || has_content {
                    info!(
                        storage_url = %storage_url,
                        content = result.content_count,
                        humans = result.human_count,
                        relationships = result.relationship_count,
                        attempt = attempt + 1,
                        "Cache stream warm-up completed successfully"
                    );
                    break;
                }

                attempt += 1;
                if attempt >= MAX_WARMUP_RETRIES {
                    warn!(
                        storage_url = %storage_url,
                        attempts = attempt,
                        errors = ?result.errors,
                        "Cache stream warm-up failed after max retries"
                    );
                    break;
                }

                let delay = WARMUP_RETRY_BASE_SECS
                    .saturating_mul(2u64.pow(attempt - 1))
                    .min(WARMUP_RETRY_MAX_SECS);

                warn!(
                    storage_url = %storage_url,
                    attempt = attempt,
                    retry_in_secs = delay,
                    errors = ?result.errors,
                    "Cache stream warm-up failed, retrying"
                );

                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
        }
    })
}
```

- [ ] **Step 4: Run existing tests to verify no regressions**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib warm_stream -- --nocapture`
Expected: All existing tests PASS

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/projection/warm_stream.rs
git commit -m "fix(doorway): add retry with exponential backoff to cache warmup

Doorway's cache stream warm-up fires once at startup. If storage pods
restart after doorway (common in k8s), the projection cache stays empty
permanently. Add retry with exponential backoff (10s-120s, 5 attempts)
so warmup recovers when storage becomes reachable."
```

---

## Task 4: `/health/startup` Endpoint

**Files:**
- Modify: `doorway/doorway-service/src/routes/health.rs`
- Modify: `doorway/doorway-service/src/server/http.rs`

- [ ] **Step 1: Read the existing health module**

Read: `doorway/doorway-service/src/routes/health.rs` to understand the existing health_check and readiness_check patterns. The new endpoint follows the same style.

- [ ] **Step 2: Add `startup_check` function to health.rs**

Add to the end of `doorway/doorway-service/src/routes/health.rs` (before any `#[cfg(test)]` module):

```rust
/// Startup readiness check — used by bootstrap page to show live progress.
///
/// Route: GET /health/startup
///
/// Returns JSON with the status of each subsystem needed before the root
/// SPA can be served. The bootstrap page polls this every 2 seconds.
pub async fn startup_check(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let identity_ready = true; // Doorway always has identity once running
    let identity_did = state
        .args
        .doorway_id
        .as_deref()
        .map(|id| {
            if id.contains('.') {
                format!("did:web:{id}")
            } else {
                format!("did:web:{}", id.replace('-', "."))
            }
        })
        .unwrap_or_default();

    let storage_ready = state.args.storage_url.is_some();
    let storage_url = state
        .args
        .storage_url
        .as_deref()
        .unwrap_or("")
        .to_string();

    // Projection cache stats
    let (projection_ready, content_count, human_count, relationship_count) =
        if let Some(ref store) = state.projection {
            let stats = store.stats().await;
            (stats.total > 0, stats.content, stats.humans, stats.relationships)
        } else {
            (false, 0, 0, 0)
        };

    // Root app status
    let root_slug = state.args.root_app_slug.as_deref().unwrap_or("");
    let (root_ready, root_blob_hash, root_extracted) = if root_slug.is_empty() {
        (false, String::new(), false)
    } else if let Some(ref cache) = state.app_file_cache {
        let blob_hash = cache.resolve_blob_hash(root_slug).await;
        let extracted = if let Some(ref hash) = blob_hash {
            cache.has_cached_files(root_slug, hash).await
        } else {
            false
        };
        (extracted, blob_hash.unwrap_or_default(), extracted)
    } else {
        (false, String::new(), false)
    };

    let body = serde_json::json!({
        "identity": {
            "ready": identity_ready,
            "did": identity_did,
        },
        "storage": {
            "ready": storage_ready,
            "url": storage_url,
        },
        "projection": {
            "ready": projection_ready,
            "content": content_count,
            "humans": human_count,
            "relationships": relationship_count,
        },
        "rootApp": {
            "ready": root_ready,
            "slug": root_slug,
            "blobHash": root_blob_hash,
            "extracted": root_extracted,
        },
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "no-cache")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}
```

- [ ] **Step 3: Check if ProjectionStore has a `stats()` method**

Read: `doorway/doorway-service/src/projection/store.rs` to check for an existing stats method. If it doesn't exist, add one that returns counts from the hot cache or MongoDB. If the method signature differs from what step 2 assumes, adjust the `startup_check` code to match.

Similarly check if `AppFileCacheService` has `resolve_blob_hash` and `has_cached_files` methods. If `has_cached_files` doesn't exist, add a simple check that queries whether any cached files exist for a given slug+hash.

- [ ] **Step 4: Wire the route in http.rs**

In `doorway/doorway-service/src/server/http.rs`, add after the existing `/health` routes (after the `/healthz` line, around line 712):

```rust
// Startup status — bootstrap page polls this for live progress
(Method::GET, "/health/startup") => {
    to_boxed(routes::startup_check(Arc::clone(&state)).await)
}
```

**Important:** This must appear BEFORE the general `/health` match arm so it matches first.

- [ ] **Step 5: Add re-export in routes/mod.rs**

In `doorway/doorway-service/src/routes/mod.rs`, update the health re-export line:

```rust
pub use health::{health_check, readiness_check, startup_check, version_info};
```

- [ ] **Step 6: Verify compilation**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check`
Expected: Compiles. If `ProjectionStore::stats()` or `AppFileCacheService::has_cached_files()` don't exist, you'll get compile errors — implement them (see step 3).

- [ ] **Step 7: Commit**

```bash
git add doorway/doorway-service/src/routes/health.rs doorway/doorway-service/src/routes/mod.rs doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): add /health/startup endpoint for bootstrap page

Returns JSON with identity, storage, projection cache, and root app
status. The bootstrap page polls this every 2s to show live progress
as doorway connects to the network."
```

---

## Task 5: Bootstrap Page (Embedded HTML)

**Files:**
- Create: `doorway/doorway-service/src/routes/root_app.rs`
- Modify: `doorway/doorway-service/src/routes/mod.rs`

- [ ] **Step 1: Create the root_app module with bootstrap page**

Create `doorway/doorway-service/src/routes/root_app.rs`:

```rust
//! Root App Handler — serves a spa-bundle from blob storage at `/`.
//!
//! When ROOT_APP_SLUG is configured, doorway resolves the slug to a blob hash,
//! extracts the ZIP, and serves the SPA. Unmatched paths fall back to index.html
//! (SPA routing). During cold start, serves an embedded bootstrap page that
//! polls /health/startup for live progress.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::server::AppState;

/// Embedded bootstrap page — served when the root SPA is not yet loaded.
/// Polls /health/startup for live progress, falls back to simple reload.
const BOOTSTRAP_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Connecting...</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
         background: #0a0a0a; color: #e0e0e0; display: flex; align-items: center;
         justify-content: center; min-height: 100vh; }
  .container { max-width: 480px; padding: 2rem; }
  h1 { font-size: 1.25rem; font-weight: 400; margin-bottom: 1.5rem; color: #fff; }
  .status-line { display: flex; align-items: center; gap: 0.75rem; padding: 0.5rem 0;
                 font-size: 0.9rem; opacity: 0.6; transition: opacity 0.3s; }
  .status-line.ready { opacity: 1; }
  .status-line.active { opacity: 0.85; }
  .icon { width: 1.25rem; text-align: center; font-size: 0.8rem; }
  .ready .icon { color: #4ade80; }
  .active .icon { color: #facc15; }
  .detail { color: #888; font-size: 0.8rem; margin-left: 2rem; }
</style>
</head>
<body>
<div class="container">
  <h1>Connecting to the Elohim Protocol&hellip;</h1>
  <div id="identity" class="status-line">
    <span class="icon">&bull;</span><span class="label">Doorway identity</span>
  </div>
  <div id="storage" class="status-line">
    <span class="icon">&bull;</span><span class="label">Storage sidecar</span>
  </div>
  <div id="projection" class="status-line">
    <span class="icon">&bull;</span><span class="label">Projection cache</span>
  </div>
  <div id="projection-detail" class="detail"></div>
  <div id="rootApp" class="status-line">
    <span class="icon">&bull;</span><span class="label">Application bundle</span>
  </div>
</div>
<script>
(function() {
  var fallbackTimer = setTimeout(function() { location.reload(); }, 5000);
  var poll = setInterval(function() {
    fetch('/health/startup').then(function(r) { return r.json(); }).then(function(s) {
      clearTimeout(fallbackTimer);
      update('identity', s.identity.ready, s.identity.did ? s.identity.did : '');
      update('storage', s.storage.ready, s.storage.ready ? 'connected' : 'waiting');
      update('projection', s.projection.ready, '');
      var pd = document.getElementById('projection-detail');
      if (s.projection.content > 0) {
        pd.textContent = s.projection.content + ' content, ' + s.projection.humans + ' humans';
      }
      update('rootApp', s.rootApp.ready, s.rootApp.slug ? 'Loading ' + s.rootApp.slug : '');
      if (s.rootApp.ready) {
        clearInterval(poll);
        setTimeout(function() { location.replace('/'); }, 500);
      }
    }).catch(function() { /* /health/startup unreachable — fallback timer handles reload */ });
  }, 2000);
  function update(id, ready, detail) {
    var el = document.getElementById(id);
    el.className = 'status-line ' + (ready ? 'ready' : 'active');
    el.querySelector('.icon').textContent = ready ? '\u2713' : '\u25CB';
    if (detail) {
      var label = el.querySelector('.label');
      label.textContent = label.textContent.split(':')[0] + ': ' + detail;
    }
  }
})();
</script>
</body>
</html>"#;

/// Serve the bootstrap page during cold start.
pub fn bootstrap_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-cache, no-store")
        .body(Full::new(Bytes::from(BOOTSTRAP_HTML)))
        .unwrap()
}
```

- [ ] **Step 2: Add module declaration and re-export in routes/mod.rs**

In `doorway/doorway-service/src/routes/mod.rs`, add the module declaration (alphabetical, after `pub mod journal;`):

```rust
pub mod root_app;
```

And add to the re-exports section:

```rust
pub use root_app::bootstrap_response;
```

- [ ] **Step 3: Verify compilation**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check`
Expected: Compiles (bootstrap_response is not wired to routing yet — that's Task 6)

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/src/routes/root_app.rs doorway/doorway-service/src/routes/mod.rs
git commit -m "feat(doorway): add bootstrap page for cold-start root app loading

Embedded HTML page that polls /health/startup every 2s, showing live
status as doorway connects to its network. Falls back to simple reload
if the status endpoint is unreachable. Auto-navigates when the root
SPA is extracted and ready."
```

---

## Task 6: Root App Resolution in http.rs

**Files:**
- Modify: `doorway/doorway-service/src/routes/root_app.rs`
- Modify: `doorway/doorway-service/src/server/http.rs:825-852,1422`
- Modify: `doorway/doorway-service/src/routes/mod.rs`

- [ ] **Step 1: Add the root app request handler to root_app.rs**

Add to `doorway/doorway-service/src/routes/root_app.rs` after `bootstrap_response()`:

```rust
/// Handle a request that didn't match any API route — resolve via root app.
///
/// Fallback chain:
/// 1. No ROOT_APP_SLUG → redirect to /threshold
/// 2. Slug not resolved (cache cold) → bootstrap page
/// 3. File found in extraction cache → serve static asset
/// 4. File not found → serve index.html (SPA fallback)
pub async fn handle_root_app_request(state: Arc<AppState>, path: &str) -> Response<Full<Bytes>> {
    let slug = match &state.args.root_app_slug {
        Some(s) => s.as_str(),
        None => {
            // No root app configured — redirect to operator dashboard
            return Response::builder()
                .status(StatusCode::TEMPORARY_REDIRECT)
                .header("Location", "/threshold")
                .body(Full::new(Bytes::from(
                    r#"<html><body>Redirecting to <a href="/threshold">/threshold</a></body></html>"#,
                )))
                .unwrap();
        }
    };

    // Resolve blob hash from slug index (populated by projection cache warmup)
    let blob_hash = if let Some(ref cache) = state.app_file_cache {
        cache.resolve_blob_hash(slug).await
    } else {
        None
    };

    let blob_hash = match blob_hash {
        Some(h) => h,
        None => {
            // Slug not resolved — cache still warming, show bootstrap
            debug!(slug = %slug, "Root app slug not resolved, serving bootstrap page");
            return bootstrap_response();
        }
    };

    // Determine the file path from the request
    let file_path = path.trim_start_matches('/');
    let file_path = if file_path.is_empty() { "index.html" } else { file_path };

    // Try extraction cache for the file
    if let Some(ref cache) = state.app_file_cache {
        // Ensure the ZIP is extracted
        if !cache.is_current(slug, &blob_hash).await {
            // ZIP not extracted yet — trigger extraction via the existing /apps/ mechanism
            // by delegating to the storage fallback
            debug!(slug = %slug, blob_hash = %blob_hash, "Root app ZIP not extracted, triggering extraction");
            let app_path = format!("/apps/{}/{}", slug, file_path);
            return super::handle_app_request(Arc::clone(&state), &app_path).await;
        }

        // Check if the requested file exists in the extraction cache
        if let Some(data) = cache.get_file(slug, file_path).await {
            let content_type = get_mime_type(file_path);
            debug!(slug = %slug, file_path = %file_path, "Root app file served from cache");
            return Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", content_type)
                .header("Content-Length", data.len())
                .header("Cache-Control", cache_control_for(file_path))
                .header("X-Root-App", slug)
                .body(Full::new(Bytes::from(data)))
                .unwrap();
        }

        // File not in cache — SPA fallback: serve index.html
        if let Some(data) = cache.get_file(slug, "index.html").await {
            debug!(slug = %slug, file_path = %file_path, "Root app SPA fallback to index.html");
            return Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/html; charset=utf-8")
                .header("Cache-Control", "no-cache")
                .header("X-Root-App", slug)
                .body(Full::new(Bytes::from(data)))
                .unwrap();
        }
    }

    // Extraction cache unavailable — show bootstrap
    warn!(slug = %slug, "Root app extraction cache unavailable, serving bootstrap");
    bootstrap_response()
}

/// MIME type lookup for common SPA assets.
fn get_mime_type(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "wasm" => "application/wasm",
        "webp" => "image/webp",
        "txt" => "text/plain; charset=utf-8",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

/// Cache-control strategy: hash-named assets get long cache, index.html gets none.
fn cache_control_for(path: &str) -> &'static str {
    if path == "index.html" || path == "version.json" || path.ends_with("config.json") {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    }
}
```

- [ ] **Step 2: Add re-export in routes/mod.rs**

Update the root_app re-export line:

```rust
pub use root_app::{bootstrap_response, handle_root_app_request};
```

- [ ] **Step 3: Replace root `/` handler and add catch-all in http.rs**

In `doorway/doorway-service/src/server/http.rs`, replace the existing root handler (lines 825-852) with:

```rust
        // Root path: serve root SPA or redirect to threshold
        (Method::GET, "/") => {
            if hyper_tungstenite::is_upgrade_request(&req) {
                if state.args.dev_mode {
                    debug!("Legacy WebSocket path used - consider migrating to /hc/admin");
                    to_boxed(websocket::handle_admin_upgrade(state, req).await)
                } else {
                    to_boxed(
                        Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .header("Content-Type", "application/json")
                            .body(Full::new(Bytes::from(
                                r#"{"error":"Admin WebSocket disabled in production. Use POST /hc/connect."}"#,
                            )))
                            .unwrap(),
                    )
                }
            } else {
                to_boxed(
                    routes::handle_root_app_request(Arc::clone(&state), "/").await,
                )
            }
        }

        // Admin redirect (preserved)
        (Method::GET, "/admin") => {
            to_boxed(
                Response::builder()
                    .status(StatusCode::TEMPORARY_REDIRECT)
                    .header("Location", "/threshold")
                    .body(Full::new(Bytes::from(
                        r#"<html><body>Redirecting to <a href="/threshold">/threshold</a></body></html>"#,
                    )))
                    .unwrap(),
            )
        }
```

Then replace the catch-all `_ => 404` (line 1422) with:

```rust
        // Root app catch-all: any unmatched GET serves the SPA (if configured)
        (Method::GET, p) if state.args.root_app_slug.is_some() => {
            to_boxed(
                routes::handle_root_app_request(Arc::clone(&state), p).await,
            )
        }

        // Not found
        _ => to_boxed(not_found_response(&path)),
```

- [ ] **Step 4: Check that the `is_current` and `get_file` methods exist on AppFileCacheService**

Read `doorway/doorway-service/src/cache/app_file_cache.rs` to verify:
- `resolve_blob_hash(slug: &str) -> Option<String>` exists
- `is_current(slug: &str, blob_hash: &str) -> bool` exists
- `get_file(slug: &str, file_path: &str) -> Option<Vec<u8>>` exists

If any method is missing or has a different signature, adapt the code in step 1 accordingly. The extraction cache on storage has these methods — doorway's cache may differ.

- [ ] **Step 5: Verify compilation**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check`
Expected: Compiles. Fix any method signature mismatches found in step 4.

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/src/routes/root_app.rs doorway/doorway-service/src/routes/mod.rs doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): serve root SPA from blob extraction cache

When ROOT_APP_SLUG is set, doorway resolves the slug to a blob hash,
extracts the ZIP, and serves static files at /. Unmatched paths fall
back to index.html (SPA routing). Bootstrap page during cold start.
No ROOT_APP_SLUG redirects to /threshold as before."
```

---

## Task 7: Seed Content Node for Lamad SPA

**Files:**
- Create: `genesis/data/lamad/content/lamad-spa.json`

- [ ] **Step 1: Create the seed content node**

Create `genesis/data/lamad/content/lamad-spa.json`:

```json
{
  "id": "lamad-spa",
  "contentType": "application",
  "title": "Lamad Learning Platform",
  "name": "Lamad",
  "description": "The Elohim Protocol's learning platform — served as doorway's root web surface.",
  "content": {
    "slug": "lamad",
    "entryPoint": "index.html"
  },
  "contentFormat": "spa-bundle",
  "tags": [
    "application",
    "spa",
    "learning-platform",
    "lamad"
  ],
  "blobHash": "",
  "reach": "commons",
  "metadata": {
    "category": "application",
    "embedStrategy": "root"
  },
  "createdAt": "2026-04-02T00:00:00.000000",
  "updatedAt": "2026-04-02T00:00:00.000000"
}
```

Note: `blobHash` is empty — it gets set by CI when the Angular build is uploaded. The content node exists so the slug `lamad` is registered in the system.

- [ ] **Step 2: Commit**

```bash
git add genesis/data/lamad/content/lamad-spa.json
git commit -m "feat(genesis): add lamad-spa content node for root SPA serving

Seed data for the Lamad learning platform as a spa-bundle content node.
blobHash is populated by CI when the Angular dist ZIP is uploaded."
```

---

## Task 8: Iframe Renderer — Same-Origin URLs

**Files:**
- Modify: `app/elohim-app/src/app/lamad/renderers/iframe-renderer/iframe-renderer.component.ts:177-193`

- [ ] **Step 1: Simplify `resolveDoorwayUrl()` to always return empty string**

In `app/elohim-app/src/app/lamad/renderers/iframe-renderer/iframe-renderer.component.ts`, replace the `resolveDoorwayUrl()` method (lines 177-193) with:

```typescript
  /**
   * Resolve the doorway base URL.
   *
   * Returns empty string (relative URL) for all environments. Doorway serves
   * everything from the same origin — either directly or via ingress proxy.
   * This ensures the service worker can intercept /apps/ requests for ZIP
   * delivery on cold cache.
   */
  private resolveDoorwayUrl(): string {
    return '';
  }
```

- [ ] **Step 2: Remove unused private helper methods**

Remove `isCheEnvironment()`, `isLocalDevelopment()`, and `getCheDevProxyUrl()` methods (lines 195-234) since they're no longer called. Verify no other code in the file references them first.

- [ ] **Step 3: Remove unused environment import if applicable**

Check the top of the file for `import { environment } from '...'`. If `resolveDoorwayUrl()` was the only consumer of `environment`, remove the import.

- [ ] **Step 4: Run lint**

Run: `cd app/elohim-app && pnpm run lint`
Expected: No new errors (removing dead code shouldn't introduce issues)

- [ ] **Step 5: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "iframe-renderer"`
Expected: PASS (or skip if iframe-renderer tests use `templateUrl` — known Vitest issue)

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/lamad/renderers/iframe-renderer/iframe-renderer.component.ts
git commit -m "fix(lamad): use same-origin URLs for iframe renderer in all environments

All /apps/ requests now go through the same origin, allowing the
service worker to intercept and use ZIP delivery on cold cache.
Removes Che/localhost environment detection — no longer needed."
```

---

## Task 9: Ingress Phase 1 — Route `/apps/` and `/blob/` Through Main Origin

**Files:**
- Modify: `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml`
- Modify: `genesis/orchestrator/manifests/elohim-app/staging/ingress.yaml`

- [ ] **Step 1: Add `/apps` and `/blob` paths to alpha ingress**

In `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml`, add before the catch-all `/` path:

```yaml
spec:
  ingressClassName: public
  rules:
  - host: alpha.elohim.host
    http:
      paths:
      - backend:
          service:
            name: elohim-ui-playground-alpha-service
            port:
              number: 80
        path: /ui-playground
        pathType: Prefix
      - backend:
          service:
            name: elohim-doorway-alpha-service
            port:
              number: 8888
        path: /apps
        pathType: Prefix
      - backend:
          service:
            name: elohim-doorway-alpha-service
            port:
              number: 8888
        path: /blob
        pathType: Prefix
      - backend:
          service:
            name: elohim-site-alpha-service
            port:
              number: 80
        path: /
        pathType: Prefix
```

- [ ] **Step 2: Apply the same pattern to staging ingress**

Read `genesis/orchestrator/manifests/elohim-app/staging/ingress.yaml` and apply the same `/apps` and `/blob` paths pointing to the staging doorway service name.

- [ ] **Step 3: Verify doorway service name**

Check that `elohim-doorway-alpha-service` is the correct service name and `8888` is the correct port by reading the doorway k8s manifests. Adjust if different.

- [ ] **Step 4: Commit**

```bash
git add genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml genesis/orchestrator/manifests/elohim-app/staging/ingress.yaml
git commit -m "feat(infra): route /apps and /blob through main origin for SW support

Phase 1 of ingress transition. Routes /apps/* and /blob/* through
alpha.elohim.host to doorway, enabling the apps service worker to
intercept same-origin requests for ZIP delivery on cold cache."
```

---

## Task 10: CI Pipeline — ZIP and Upload SPA Blob

**Files:**
- Modify: `Jenkinsfile` (root)

- [ ] **Step 1: Read the current build stage**

Read `Jenkinsfile` lines 625-830 to understand the Angular build flow and where to insert the ZIP + upload step.

- [ ] **Step 2: Add SPA blob upload stage**

In `Jenkinsfile`, add a new stage after the Angular build completes but before (or parallel to) the Docker image build. Add as a helper method in the `// STAGE HELPER METHODS` section (line 78+) to stay under the JVM method size limit:

```groovy
def stageSpaBlob(String storageUrl, String distDir) {
    sh """
        cd ${distDir}
        zip -r lamad-spa.zip .
        SPA_HASH=\$(sha256sum lamad-spa.zip | awk '{print \$1}')
        echo "SPA blob hash: \${SPA_HASH}"

        # Upload ZIP as blob
        curl -f -X PUT \\
            -H "Content-Type: application/zip" \\
            --data-binary @lamad-spa.zip \\
            "${storageUrl}/blob/\${SPA_HASH}"

        # Update content node with new blobHash
        curl -f -X PUT \\
            -H "Content-Type: application/json" \\
            -d "{\\"id\\":\\"lamad-spa\\",\\"blobHash\\":\\"\${SPA_HASH}\\"}" \\
            "${storageUrl}/db/content/lamad-spa"
    """
}
```

Call it from a new stage:

```groovy
stage('Upload SPA Blob') {
    when { expression { params.MODE != 'ci-only' } }
    steps {
        container('ci-builder') {
            stageSpaBlob(
                env.STORAGE_URL ?: 'http://elohim-matthew-alpha:8090',
                'app/elohim-app/dist/elohim-app/browser'
            )
        }
    }
}
```

- [ ] **Step 3: Verify the storage API accepts content node updates**

Check that `PUT /db/content/{id}` exists in elohim-storage and accepts partial updates (at minimum `blobHash`). If the endpoint expects a full content node, adjust the curl payload to include all required fields from the seed data.

- [ ] **Step 4: Commit**

```bash
git add Jenkinsfile
git commit -m "feat(ci): upload Angular SPA as blob to storage after build

Zips the Angular dist, uploads to storage as a content-addressed blob,
and updates the lamad-spa content node with the new blobHash. Doorway
serves the SPA from this blob via its extraction cache."
```

---

## Execution Order Summary

| Task | Description | Depends On |
|------|-------------|-----------|
| 1 | Schema: add `spa-bundle` format | None |
| 2 | Config: `ROOT_APP_SLUG` env var | None |
| 3 | Warmup retry (RCA fix) | None |
| 4 | `/health/startup` endpoint | Task 2 |
| 5 | Bootstrap page HTML | None |
| 6 | Root app resolution in http.rs | Tasks 2, 4, 5 |
| 7 | Seed content node | Task 1 |
| 8 | Iframe renderer same-origin | None |
| 9 | Ingress Phase 1 | None |
| 10 | CI blob upload | Tasks 1, 7 |

Tasks 1, 2, 3, 5, 8, 9 are independent and can be parallelized.
