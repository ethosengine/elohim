//! Admin cache control endpoints
//!
//! Exposes diagnostic controls for the projection cache and app file cache.
//! Used by delivery diagnostics tests to toggle cache layers and prove
//! fallback behavior.
//!
//! ## Endpoints
//!
//! - `GET  /admin/cache/stats`        — Returns enabled flag, projection entry counts, file cache count
//! - `POST /admin/cache/disable`      — Disables the app file cache (forces storage proxy)
//! - `POST /admin/cache/enable`       — Re-enables the app file cache
//! - `POST /admin/cache/clear/{slug}` — Evicts all cached files for a given app slug
//! - `POST /admin/cache/warm`         — Spawns a background projection warm-up from storage

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tracing::info;

use crate::server::AppState;

// ============================================================================
// GET /admin/cache/stats
// ============================================================================

/// Return a snapshot of the current cache state.
///
/// Response shape:
/// ```json
/// {
///   "enabled": true,
///   "projection": { "entries": 1234, "contentEntries": 800 },
///   "appFileCache": { "cachedFiles": 42 }
/// }
/// ```
pub async fn cache_stats(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let enabled = state.cache_enabled.load(Ordering::Relaxed);

    // Projection store counts — content + total hot-cache entries
    let (content_entries, total_entries) = if let Some(ref proj) = state.projection {
        let (content, humans, relationships) = proj.count_by_type();
        let total = content + humans + relationships;
        (content, total)
    } else {
        (0, 0)
    };

    // App file cache document count (MongoDB)
    let cached_files = if let Some(ref afc) = state.app_file_cache {
        afc.cached_file_count().await
    } else {
        0
    };

    let body = serde_json::json!({
        "enabled": enabled,
        "projection": {
            "entries": total_entries,
            "contentEntries": content_entries,
        },
        "appFileCache": {
            "cachedFiles": cached_files,
        },
    });

    json_response(StatusCode::OK, body)
}

// ============================================================================
// POST /admin/cache/disable
// ============================================================================

/// Disable the app file cache.
///
/// All subsequent `/apps/*` requests will bypass MongoDB and proxy directly
/// to elohim-storage. Useful for proving fallback behavior in delivery tests.
pub async fn cache_disable(state: Arc<AppState>) -> Response<Full<Bytes>> {
    state.cache_enabled.store(false, Ordering::Relaxed);
    info!("Admin cache control: cache DISABLED");
    json_response(StatusCode::OK, serde_json::json!({ "status": "disabled" }))
}

// ============================================================================
// POST /admin/cache/enable
// ============================================================================

/// Re-enable the app file cache.
pub async fn cache_enable(state: Arc<AppState>) -> Response<Full<Bytes>> {
    state.cache_enabled.store(true, Ordering::Relaxed);
    info!("Admin cache control: cache ENABLED");
    json_response(StatusCode::OK, serde_json::json!({ "status": "enabled" }))
}

// ============================================================================
// POST /admin/cache/clear/{slug}
// ============================================================================

/// Evict all cached files for a specific app slug from MongoDB.
///
/// Also removes the slug from the in-memory blob-hash index so the next
/// request re-resolves with a fresh hash. Returns the number of documents
/// removed.
pub async fn cache_clear_slug(state: Arc<AppState>, slug: &str) -> Response<Full<Bytes>> {
    let removed = if let Some(ref afc) = state.app_file_cache {
        let count = afc.clear_slug(slug).await;
        info!(slug = %slug, removed = count, "Admin cache control: slug evicted");
        count
    } else {
        0
    };

    json_response(
        StatusCode::OK,
        serde_json::json!({ "slug": slug, "removed": removed }),
    )
}

// ============================================================================
// POST /admin/cache/warm
// ============================================================================

/// Trigger a background projection warm-up from elohim-storage.
///
/// Spawns a tokio task that calls `stream_from_peer` for each configured
/// storage URL — the same warm-up logic as the startup sequence. Returns
/// immediately with `{ "status": "warming", "async": true }`.
pub async fn cache_warm(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let Some(ref projection_store) = state.projection else {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({ "error": "Projection store not available" }),
        );
    };

    // Build the peer URL list (same logic as startup and the existing inline handler)
    let mut peer_urls: Vec<String> = Vec::new();
    if let Some(ref url) = state.args.storage_url {
        peer_urls.push(url.clone());
    }
    for url in &state.args.storage_urls {
        let trimmed = url.trim().to_string();
        if !trimmed.is_empty() && !peer_urls.contains(&trimmed) {
            peer_urls.push(trimmed);
        }
    }

    if peer_urls.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            serde_json::json!({ "error": "No storage URLs configured" }),
        );
    }

    let peer_count = peer_urls.len();
    let store = Arc::clone(projection_store);

    tokio::spawn(async move {
        info!(peers = peer_count, "Admin cache warm-up started");
        for url in &peer_urls {
            crate::projection::warm_stream::stream_from_peer(Arc::clone(&store), url).await;
        }
        info!(peers = peer_count, "Admin cache warm-up complete");
    });

    json_response(
        StatusCode::ACCEPTED,
        serde_json::json!({ "status": "warming", "async": true, "peers": peer_count }),
    )
}

// ============================================================================
// Helper
// ============================================================================

fn json_response(status: StatusCode, body: serde_json::Value) -> Response<Full<Bytes>> {
    match serde_json::to_string_pretty(&body) {
        Ok(json) => Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(json)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Failed to build response")))
                    .unwrap()
            }),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from("Failed to serialize response")))
            .unwrap(),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_response_ok() {
        let resp = json_response(StatusCode::OK, serde_json::json!({ "status": "ok" }));
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_json_response_accepted() {
        let resp = json_response(
            StatusCode::ACCEPTED,
            serde_json::json!({ "status": "warming" }),
        );
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn test_json_response_service_unavailable() {
        let resp = json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({ "error": "not available" }),
        );
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
