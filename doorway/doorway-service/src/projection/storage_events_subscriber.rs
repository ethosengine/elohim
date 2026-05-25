//! Storage Events Subscriber — live projection refresh on substrate change.
//!
//! Connects to elohim-storage's `GET /api/v1/events` SSE endpoint
//! (sse.rs in elohim-storage emits the StorageEventBus as text/event-stream).
//! When a content event arrives, evicts the matching entry from doorway's
//! app file cache so the next /apps/{slug} request re-resolves against
//! storage's now-fresh slug_index.
//!
//! Pairs with `warm_stream.rs`:
//! - `warm_stream` is the *one-shot cold-start* path. At boot, doorway pulls
//!   the current bulk projection from `/api/v1/cache/stream` into MongoDB.
//! - `storage_events_subscriber` is the *live tail* path. Forever after,
//!   doorway listens for `content.updated`/`content.created`/`content.deleted`
//!   on `/api/v1/events` and invalidates its caches.
//!
//! Per Pattern Z (`genesis/docs/superpowers/specs/`
//! `2026-05-23-doorway-access-tier-patterns.md`): doorway is a projection
//! of substrate truth, not an authority. It accepts what storage emits.
//! The substrate-correct long-term path is `PUT /api/v1/epr/{cid}` which
//! emits DHT signals; this subscriber is the bridge until stageSpaBlob
//! and other deploy-time mutation callers migrate to EPR Head republish
//! (Pattern Z.D / Z.E).
//!
//! ## Failure modes (intentional)
//!
//! - Storage unreachable at startup: task logs and retries with exponential
//!   backoff (1s → 60s cap). Doorway still serves requests; the projection
//!   just stays at whatever state warm_stream left it in.
//! - Mid-stream disconnect (storage restart, network blip): same backoff
//!   loop; the inner `run_subscriber` returns with an error, the outer
//!   `spawn_subscriber_task` reconnects.
//! - Malformed event payload: logged at debug level, event dropped, stream
//!   continues. A bad event never breaks the loop.
//! - `app_file_cache` not configured: the task still runs but the
//!   invalidation is a no-op — fine, the gap is documented.

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::cache::AppFileCacheService;

/// Spawn the long-running storage-events subscriber.
///
/// Returns immediately; the SSE consumer runs as a tokio task that survives
/// transient disconnects. Empty `storage_url` is treated as "no storage to
/// subscribe to" — the task exits cleanly without retrying.
pub fn spawn_subscriber_task(
    storage_url: String,
    app_file_cache: Option<Arc<AppFileCacheService>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if storage_url.is_empty() {
            info!("storage_events_subscriber: storage_url is empty; subscriber will not start");
            return;
        }

        let url = format!("{}/api/v1/events", storage_url.trim_end_matches('/'));
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(60);

        loop {
            match run_subscriber(&url, app_file_cache.as_ref()).await {
                Ok(()) => {
                    info!(
                        url = %url,
                        "storage_events_subscriber: stream ended cleanly; reconnecting"
                    );
                    backoff = Duration::from_secs(1);
                }
                Err(e) => {
                    warn!(
                        url = %url,
                        error = %e,
                        backoff_secs = %backoff.as_secs(),
                        "storage_events_subscriber: stream error; backing off before retry"
                    );
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(max_backoff);
                }
            }
        }
    })
}

/// Inner subscriber loop. Connects, tails events, returns on disconnect.
async fn run_subscriber(
    url: &str,
    app_file_cache: Option<&Arc<AppFileCacheService>>,
) -> Result<(), String> {
    // No top-level timeout — this is a long-lived stream. The reqwest body
    // stream itself yields whenever the connection drops, which is the
    // signal we use to trigger a reconnect.
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("client build: {e}"))?;

    let response = client
        .get(url)
        .header("Accept", "text/event-stream")
        .send()
        .await
        .map_err(|e| format!("connect: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} from {}", response.status(), url));
    }

    info!(url = %url, "storage_events_subscriber: connected; tailing events");

    let mut byte_stream = response.bytes_stream();
    let mut line_buffer = String::new();
    let mut current_event_type: Option<String> = None;
    let mut current_event_data: Option<String> = None;

    while let Some(chunk_result) = byte_stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("stream chunk: {e}"))?;
        let text = std::str::from_utf8(&chunk).map_err(|e| format!("non-utf8 chunk: {e}"))?;
        line_buffer.push_str(text);

        while let Some(newline_pos) = line_buffer.find('\n') {
            let line = line_buffer[..newline_pos]
                .trim_end_matches('\r')
                .to_string();
            line_buffer = line_buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                // Blank line terminates one SSE event. Dispatch if we
                // accumulated a type+data pair.
                if let (Some(etype), Some(edata)) =
                    (current_event_type.take(), current_event_data.take())
                {
                    handle_event(&etype, &edata, app_file_cache).await;
                }
            } else if let Some(rest) = line.strip_prefix("event:") {
                current_event_type = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("data:") {
                current_event_data = Some(rest.trim().to_string());
            } else if line.starts_with(':') {
                // SSE comment (heartbeat) — ignore
            }
            // Unknown lines silently ignored (per SSE spec)
        }
    }

    // Stream closed without error — let the outer loop reconnect.
    Ok(())
}

/// Dispatch one parsed SSE event to the right cache-invalidation primitive.
async fn handle_event(
    event_type: &str,
    event_data: &str,
    app_file_cache: Option<&Arc<AppFileCacheService>>,
) {
    // Storage emits content.created / content.updated / content.deleted /
    // bulk + relationship/knowledge-map events. The cache surface that
    // matters for /apps/{slug} is keyed by content slug (which is the
    // content id for html5-app/spa-bundle rows). Other event kinds are
    // routed by future bridges (relationships, etc.) — out of scope here.
    if !matches!(
        event_type,
        "content.created" | "content.updated" | "content.deleted"
    ) {
        debug!(
            event_type = %event_type,
            "storage_events_subscriber: skipping non-content event"
        );
        return;
    }

    let id = match parse_id_from_data(event_data) {
        Some(id) => id,
        None => {
            debug!(
                event_type = %event_type,
                data = %event_data,
                "storage_events_subscriber: event without parseable id; skipping"
            );
            return;
        }
    };

    info!(
        event_type = %event_type,
        id = %id,
        "storage_events_subscriber: invalidating app file cache"
    );

    if let Some(cache) = app_file_cache {
        // clear_slug evicts both the per-file MongoDB cache entries AND the
        // in-memory slug→blob_hash index for this content. The next request
        // to /apps/{slug}/{file} will re-resolve through resolve_blob_hash's
        // slow path (MongoDB query), then cache miss → fetch from storage,
        // where Z.B.1 has refreshed storage's slug_index to point at the
        // new blob. End-to-end fresh content.
        //
        // Known follow-up gap (Pattern Z.D scope): doorway's MongoDB
        // projection store (projected_entries) is not refreshed by this
        // event — only the app_file_cache. resolve_blob_hash's slow path
        // queries projected_entries and may return a stale blob_hash. The
        // resulting cache key would be wrong, but storage's downstream
        // /apps handler still does its own slug resolution and serves the
        // fresh bytes, so user-visible behavior is correct. Tightening this
        // belongs to the projection-refresh-on-event extension, which lands
        // alongside stageSpaBlob's substrate-correct migration to PUT
        // /api/v1/epr/{cid} (then conductor signals refresh projected_entries
        // through the proper channel and this subscriber's role narrows).
        let _ = cache.clear_slug(&id).await;
    }
}

/// Pull the `id` field out of an SSE event's JSON data payload.
///
/// Storage's sse.rs encodes content events as `{"id":"<content-id>"}`.
fn parse_id_from_data(data: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    v.get("id").and_then(|x| x.as_str()).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed_id_payload() {
        assert_eq!(
            parse_id_from_data(r#"{"id":"elohim-host-landing"}"#),
            Some("elohim-host-landing".to_string())
        );
    }

    #[test]
    fn ignores_payload_without_id() {
        assert_eq!(parse_id_from_data(r#"{"foo":"bar"}"#), None);
    }

    #[test]
    fn ignores_malformed_json() {
        assert_eq!(parse_id_from_data("not json at all"), None);
    }

    #[test]
    fn handles_extra_fields_in_payload() {
        // Content events may carry additional fields (title, contentType)
        // for ContentCreated; we should still extract id.
        assert_eq!(
            parse_id_from_data(r#"{"id":"abc","title":"x","contentType":"concept"}"#),
            Some("abc".to_string())
        );
    }

    #[tokio::test]
    async fn empty_storage_url_exits_cleanly() {
        // The spawned task should return without panicking when storage_url
        // is empty — covers the "no peer configured" startup case.
        let handle = spawn_subscriber_task(String::new(), None);
        // Task should complete (return) — give it a generous timeout in
        // case the runtime is slow.
        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(
            result.is_ok(),
            "spawn_subscriber_task with empty URL should return promptly"
        );
    }
}
