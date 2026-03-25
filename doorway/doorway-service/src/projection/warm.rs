//! Cache Warm-Up — bootstrap projection from peer storage.
//!
//! **DEPRECATED**: Use `warm_stream` instead, which streams cacheable content
//! via SSE from elohim-storage, filtered by reach, with reconnect support.
//!
//! This module fetches all content from each peer's storage URL
//! via HTTP pull with arbitrary limits. It doesn't respect reach levels
//! and pulls blindly. Kept temporarily as fallback for older storage versions.

#![allow(deprecated)]

use std::sync::Arc;
use tracing::{info, warn};

use super::document::ProjectedDocument;
use super::store::ProjectionStore;

/// Warm the projection cache from peer storage endpoints.
///
/// For each storage URL, fetches `/db/content?limit=10000`,
/// then upserts each item into the projection store. Errors are logged but don't
/// block startup — partial warmup is better than no warmup.
#[deprecated(since = "0.1.0", note = "Use warm_stream::stream_from_peer instead")]
pub async fn warm_projection_cache(
    store: Arc<ProjectionStore>,
    storage_urls: Vec<String>,
) -> WarmResult {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    let mut total_content = 0usize;
    let mut errors = Vec::new();

    for storage_url in &storage_urls {
        let base = storage_url.trim_end_matches('/');

        // Fetch content (paths are now ContentNodes, so this covers everything)
        match fetch_and_project(&client, &store, base, "content", "Content").await {
            Ok(count) => {
                total_content += count;
                info!(
                    storage_url = %base,
                    count,
                    "Warmed content from storage"
                );
            }
            Err(e) => {
                warn!(storage_url = %base, error = %e, "Failed to warm content");
                errors.push(format!("{base}/db/content: {e}"));
            }
        }
    }

    info!(
        total_content,
        peer_count = storage_urls.len(),
        "Projection cache warm-up complete"
    );

    WarmResult {
        content_count: total_content,
        errors,
    }
}

/// Fetch items from a storage endpoint and project them.
async fn fetch_and_project(
    client: &reqwest::Client,
    store: &ProjectionStore,
    base_url: &str,
    endpoint: &str,
    doc_type: &str,
) -> Result<usize, String> {
    let url = format!("{base_url}/db/{endpoint}?limit=10000");

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    // Storage returns {"items":[...]} wrapper, not a bare array
    #[derive(serde::Deserialize)]
    struct DbResponse {
        items: Vec<serde_json::Value>,
    }

    let response: DbResponse = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse failed: {e}"))?;
    let items = response.items;

    let count = items.len();

    for item in items {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let doc = ProjectedDocument::new(
            doc_type,
            &id,
            "cache-warm", // No action_hash for warmed entries
            "cache-warm", // No author for warmed entries
            item,
        );

        if let Err(e) = store.set(doc).await {
            warn!(
                doc_type,
                id = %id,
                error = %e,
                "Failed to project warmed entry"
            );
        }
    }

    Ok(count)
}

/// Result of cache warm-up
pub struct WarmResult {
    pub content_count: usize,
    pub errors: Vec<String>,
}

/// Spawn cache warm-up as a background task with a delay.
///
/// Waits `delay` seconds after startup to let MongoDB and storage settle,
/// then warms the projection cache from all configured storage peers.
#[deprecated(since = "0.1.0", note = "Use warm_stream::spawn_stream_task instead")]
pub fn spawn_warm_task(
    store: Arc<ProjectionStore>,
    storage_urls: Vec<String>,
    delay_secs: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Let services settle before warming
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;

        info!(
            peer_count = storage_urls.len(),
            "Starting projection cache warm-up"
        );

        let result = warm_projection_cache(store, storage_urls).await;

        if result.errors.is_empty() {
            info!(
                content = result.content_count,
                "Cache warm-up completed successfully"
            );
        } else {
            warn!(
                content = result.content_count,
                errors = ?result.errors,
                "Cache warm-up completed with errors"
            );
        }
    })
}
