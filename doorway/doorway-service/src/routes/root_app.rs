//! Bootstrap page for root app cold-start
//!
//! Served when `ROOT_APP_SLUG` is configured but the SPA hasn't been extracted yet —
//! cold start, storage pod restart, or first-ever deploy. Instead of a generic error
//! page, we show the protocol being honest about its own state.
//!
//! The page:
//! - Polls `/health/startup` every 2 seconds
//! - Shows live status for identity, storage, projection cache, and application bundle
//! - Falls back to a simple 5-second reload if the status endpoint is unreachable
//! - Auto-navigates to `/` when `rootApp.ready` becomes `true` (with 500ms delay)
//! - Has ZERO external dependencies — pure inline HTML/CSS/JS in a Rust string constant

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use std::sync::Arc;
use tracing::debug;

use crate::server::AppState;

/// Embedded HTML bootstrap page — served when the root SPA isn't ready yet.
///
/// This is the first thing a visitor sees during a cold start. Every status line
/// teaches them how the system works: identity on the DHT, content in storage,
/// projection cache for fast reads, and the SPA bundle itself.
const BOOTSTRAP_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Elohim Protocol — Connecting</title>
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

    :root {
      --bg:        #0d1117;
      --surface:   #161b22;
      --border:    #30363d;
      --text:      #c9d1d9;
      --muted:     #8b949e;
      --accent:    #58a6ff;
      --success:   #3fb950;
      --active:    #d29922;
      --radius:    8px;
      --font:      -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
    }

    html, body {
      height: 100%;
      background: var(--bg);
      color: var(--text);
      font-family: var(--font);
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100vh;
    }

    .card {
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: var(--radius);
      padding: 2.5rem 3rem;
      max-width: 480px;
      width: 100%;
      margin: 1rem;
    }

    .logo {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      margin-bottom: 2rem;
    }

    .logo-mark {
      width: 36px;
      height: 36px;
      border-radius: 50%;
      background: linear-gradient(135deg, #58a6ff 0%, #3fb950 100%);
      flex-shrink: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 18px;
    }

    h1 {
      font-size: 1.1rem;
      font-weight: 600;
      color: var(--text);
      line-height: 1.3;
    }

    h1 small {
      display: block;
      font-size: 0.8rem;
      font-weight: 400;
      color: var(--muted);
      margin-top: 2px;
    }

    .status-list {
      list-style: none;
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
      margin-bottom: 2rem;
    }

    .status-item {
      display: flex;
      align-items: flex-start;
      gap: 0.75rem;
      padding: 0.75rem 1rem;
      background: var(--bg);
      border: 1px solid var(--border);
      border-radius: calc(var(--radius) - 2px);
      transition: border-color 0.3s;
    }

    .status-item.ready  { border-color: var(--success); }
    .status-item.active { border-color: var(--active); }

    .status-icon {
      font-size: 1rem;
      line-height: 1.4;
      flex-shrink: 0;
      width: 1.25rem;
      text-align: center;
      transition: transform 0.3s;
    }

    .status-item.active .status-icon {
      animation: spin 2s linear infinite;
    }

    @keyframes spin {
      from { transform: rotate(0deg); }
      to   { transform: rotate(360deg); }
    }

    .status-body {
      flex: 1;
      min-width: 0;
    }

    .status-label {
      font-size: 0.85rem;
      font-weight: 500;
      color: var(--text);
      line-height: 1.4;
    }

    .status-detail {
      font-size: 0.75rem;
      color: var(--muted);
      margin-top: 2px;
      word-break: break-all;
      font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
      line-height: 1.4;
      min-height: 1rem;
    }

    .footer {
      font-size: 0.75rem;
      color: var(--muted);
      text-align: center;
      line-height: 1.5;
    }

    .footer a {
      color: var(--accent);
      text-decoration: none;
    }

    .footer a:hover { text-decoration: underline; }

    .pulse {
      display: inline-block;
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--active);
      animation: pulse 1.5s ease-in-out infinite;
      margin-right: 0.4rem;
      vertical-align: middle;
    }

    @keyframes pulse {
      0%, 100% { opacity: 1;   transform: scale(1); }
      50%       { opacity: 0.4; transform: scale(0.7); }
    }
  </style>
</head>
<body>
  <div class="card" role="main">
    <div class="logo">
      <div class="logo-mark" aria-hidden="true">&#9652;</div>
      <h1>
        Connecting to the Elohim Protocol&hellip;
        <small>Establishing your gateway to the network</small>
      </h1>
    </div>

    <ul class="status-list" aria-live="polite" aria-label="System status">
      <li class="status-item" id="item-identity">
        <span class="status-icon" id="icon-identity" aria-hidden="true">&#9711;</span>
        <div class="status-body">
          <div class="status-label">Identity</div>
          <div class="status-detail" id="detail-identity">Waiting for agent key&hellip;</div>
        </div>
      </li>
      <li class="status-item" id="item-storage">
        <span class="status-icon" id="icon-storage" aria-hidden="true">&#9711;</span>
        <div class="status-body">
          <div class="status-label">Storage</div>
          <div class="status-detail" id="detail-storage">Waiting for storage layer&hellip;</div>
        </div>
      </li>
      <li class="status-item" id="item-projection">
        <span class="status-icon" id="icon-projection" aria-hidden="true">&#9711;</span>
        <div class="status-body">
          <div class="status-label">Projection cache</div>
          <div class="status-detail" id="detail-projection">Waiting for projection cache&hellip;</div>
        </div>
      </li>
      <li class="status-item" id="item-app">
        <span class="status-icon" id="icon-app" aria-hidden="true">&#9711;</span>
        <div class="status-body">
          <div class="status-label">Application bundle</div>
          <div class="status-detail" id="detail-app">Waiting for SPA extraction&hellip;</div>
        </div>
      </li>
    </ul>

    <div class="footer">
      <span class="pulse" aria-hidden="true"></span>
      Checking status every 2 seconds&nbsp;&mdash;&nbsp;
      <a href="/health" target="_blank" rel="noopener">health</a>
    </div>
  </div>

  <script>
    (function () {
      'use strict';

      var POLL_INTERVAL_MS = 2000;
      var FALLBACK_RELOAD_MS = 5000;
      var NAVIGATE_DELAY_MS = 500;
      var consecutiveErrors = 0;
      var MAX_CONSECUTIVE_ERRORS = 3;
      var fallbackTimer = null;

      // Icon characters
      var ICON_READY  = '\u2713'; // ✓
      var ICON_ACTIVE = '\u25CB'; // ○ (CSS spins this when item has .active class)
      var ICON_WAIT   = '\u25CB'; // ○

      function setItem(id, state, icon, detail) {
        var item   = document.getElementById('item-' + id);
        var iconEl = document.getElementById('icon-' + id);
        var detEl  = document.getElementById('detail-' + id);
        if (!item || !iconEl || !detEl) return;

        item.classList.remove('ready', 'active');
        if (state === 'ready')  item.classList.add('ready');
        if (state === 'active') item.classList.add('active');

        iconEl.textContent = icon;
        if (detail) detEl.textContent = detail;
      }

      function applyStatus(data) {
        // Identity
        if (data.identity && data.identity.ready) {
          var did = data.identity.did ? data.identity.did : 'agent key resolved';
          setItem('identity', 'ready', ICON_READY, did);
        } else {
          setItem('identity', 'active', ICON_ACTIVE, 'Resolving agent identity on DHT\u2026');
        }

        // Storage
        if (data.storage && data.storage.ready) {
          setItem('storage', 'ready', ICON_READY, 'Storage layer connected');
        } else {
          setItem('storage', 'active', ICON_ACTIVE, 'Connecting to elohim-storage\u2026');
        }

        // Projection cache
        if (data.projection && data.projection.ready) {
          var count = (data.projection.count !== undefined && data.projection.count !== null)
            ? data.projection.count + ' entries cached'
            : 'Projection cache ready';
          setItem('projection', 'ready', ICON_READY, count);
        } else {
          setItem('projection', 'active', ICON_ACTIVE, 'Warming projection cache\u2026');
        }

        // Root app / SPA bundle
        if (data.rootApp && data.rootApp.ready) {
          setItem('app', 'ready', ICON_READY, 'Application bundle ready');
          scheduleNavigate();
        } else {
          var appDetail = (data.rootApp && data.rootApp.slug)
            ? 'Extracting \u201c' + data.rootApp.slug + '\u201d from storage\u2026'
            : 'Extracting application bundle\u2026';
          setItem('app', 'active', ICON_ACTIVE, appDetail);
        }
      }

      var navigateScheduled = false;
      function scheduleNavigate() {
        if (navigateScheduled) return;
        navigateScheduled = true;
        setTimeout(function () {
          window.location.replace('/');
        }, NAVIGATE_DELAY_MS);
      }

      function armFallback() {
        if (fallbackTimer !== null) return;
        fallbackTimer = setTimeout(function () {
          // Endpoint unreachable — simple reload
          window.location.reload();
        }, FALLBACK_RELOAD_MS);
      }

      function clearFallback() {
        if (fallbackTimer !== null) {
          clearTimeout(fallbackTimer);
          fallbackTimer = null;
        }
      }

      function poll() {
        var xhr = new XMLHttpRequest();
        xhr.open('GET', '/health/startup', true);
        xhr.timeout = 4000;

        xhr.onload = function () {
          if (xhr.status >= 200 && xhr.status < 300) {
            consecutiveErrors = 0;
            clearFallback();
            try {
              var data = JSON.parse(xhr.responseText);
              applyStatus(data);
            } catch (e) {
              // Malformed JSON — keep polling
            }
          } else {
            handleError();
          }
        };

        xhr.onerror   = handleError;
        xhr.ontimeout = handleError;

        xhr.send();
      }

      function handleError() {
        consecutiveErrors++;
        if (consecutiveErrors >= MAX_CONSECUTIVE_ERRORS) {
          armFallback();
        }
      }

      // Start polling immediately, then repeat
      poll();
      setInterval(poll, POLL_INTERVAL_MS);
    }());
  </script>
</body>
</html>
"#;

/// Return the bootstrap page as an HTTP response.
///
/// Called from `http.rs` when `ROOT_APP_SLUG` is set but the SPA file cache
/// doesn't yet have the extracted bundle. The response disables caching so
/// the browser picks up the redirect as soon as the SPA becomes ready.
pub fn bootstrap_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
        .body(Full::new(Bytes::from(BOOTSTRAP_HTML)))
        .unwrap()
}

/// Handle a request that didn't match any API route — resolve via root app.
///
/// Fallback chain:
/// 1. No ROOT_APP_SLUG → redirect to /threshold
/// 2. No app_file_cache (MongoDB not configured) → redirect to /threshold
/// 3. Slug not resolved (cache cold, blob hash unknown) → bootstrap page
/// 4. File found in MongoDB cache → serve static asset
/// 5. File not found in cache → delegate to /apps/ handler to trigger extraction
/// 6. If all else fails → bootstrap page
pub async fn handle_root_app_request(state: Arc<AppState>, path: &str) -> Response<Full<Bytes>> {
    // Step 1: require ROOT_APP_SLUG
    let slug = match &state.args.root_app_slug {
        Some(s) => s.clone(),
        None => {
            return Response::builder()
                .status(StatusCode::TEMPORARY_REDIRECT)
                .header("Location", "/threshold")
                .body(Full::new(Bytes::from(
                    r#"<html><body>Redirecting to <a href="/threshold">/threshold</a></body></html>"#,
                )))
                .unwrap();
        }
    };

    // Step 2: require cache service
    let cache = match &state.app_file_cache {
        Some(c) => Arc::clone(c),
        None => {
            debug!(slug = %slug, "Root app: no app_file_cache configured, redirecting to /threshold");
            return Response::builder()
                .status(StatusCode::TEMPORARY_REDIRECT)
                .header("Location", "/threshold")
                .body(Full::new(Bytes::from(
                    r#"<html><body>Redirecting to <a href="/threshold">/threshold</a></body></html>"#,
                )))
                .unwrap();
        }
    };

    // Step 3: resolve blob_hash — if unknown the zip isn't projected yet
    let blob_hash = match cache.resolve_blob_hash(&slug).await {
        Some(h) => h,
        None => {
            debug!(slug = %slug, "Root app: blob hash not resolved yet, serving bootstrap page");
            return bootstrap_response();
        }
    };

    // Determine file path from request: strip leading '/', default to index.html
    let file_path = {
        let stripped = path.trim_start_matches('/');
        if stripped.is_empty() {
            "index.html"
        } else {
            stripped
        }
    };

    // Path traversal guard
    if file_path.contains("..") || file_path.contains('\0') {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(r#"{"error":"Invalid file path"}"#)))
            .unwrap();
    }

    // Step 4: try MongoDB cache
    if let Some(cached) = cache.get(&slug, file_path, &blob_hash).await {
        debug!(slug = %slug, file_path = %file_path, "Root app cache HIT");
        return Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", cached.content_type.as_str())
            .header("Cache-Control", cache_control_for(file_path))
            .header("X-Root-App", slug.as_str())
            .header("X-Cache", "HIT")
            .body(Full::new(Bytes::from(cached.data)))
            .unwrap();
    }

    // Step 5: delegate to /apps/ handler — triggers extraction and populates MongoDB cache.
    debug!(slug = %slug, file_path = %file_path, "Root app cache MISS — delegating to /apps/ handler");
    let apps_path = format!("/apps/{slug}/{file_path}");
    let apps_response = super::apps::handle_app_request(Arc::clone(&state), &apps_path).await;

    let status = apps_response.status();

    // If storage returned 404 for this specific file (not index.html), serve index.html
    // instead — SPA fallback for Angular client-side routing.
    if status == StatusCode::NOT_FOUND && file_path != "index.html" {
        debug!(
            slug = %slug,
            file_path = %file_path,
            "Root app: file not found, serving index.html for SPA routing"
        );

        // Trigger index.html fetch+cache via apps handler, then read from MongoDB cache.
        let index_apps_path = format!("/apps/{slug}/index.html");
        let index_response =
            super::apps::handle_app_request(Arc::clone(&state), &index_apps_path).await;

        if index_response.status().is_success() {
            return Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/html; charset=utf-8")
                .header("Cache-Control", cache_control_for("index.html"))
                .header("X-Root-App", slug.as_str())
                .header("X-SPA-Fallback", "true")
                .body(index_response.into_body())
                .unwrap();
        }

        // index.html also not found — cache is fully cold; serve bootstrap
        debug!(
            slug = %slug,
            "Root app: index.html not found either, serving bootstrap page"
        );
        return bootstrap_response();
    }

    // For successful responses from the apps handler, pass through as-is.
    // The /apps/ handler sets appropriate Content-Type, Cache-Control, and X-Cache headers.
    apps_response
}

/// Map a file extension to a MIME type for root-app static assets.
pub fn get_mime_type(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript",
        "css" => "text/css",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "wasm" => "application/wasm",
        "txt" => "text/plain",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

/// Return an appropriate Cache-Control header value for a root-app file.
///
/// - `index.html` and files without hashed names: `no-cache` — browser must
///   revalidate so new SPA deployments are picked up immediately.
/// - Everything else: immutable long-lived cache (hashed filenames guarantee
///   content correctness; Angular/Vite output files always include a hash).
pub fn cache_control_for(path: &str) -> &'static str {
    // Files that must never be served stale
    let basename = path.rsplit('/').next().unwrap_or(path);
    match basename {
        "index.html"
        | "index.htm"
        | "manifest.json"
        | "manifest.webmanifest"
        | "service-worker.js"
        | "sw.js"
        | "ngsw.json"
        | "ngsw-worker.js" => "no-cache, no-store, must-revalidate",
        _ => "public, max-age=31536000, immutable",
    }
}
