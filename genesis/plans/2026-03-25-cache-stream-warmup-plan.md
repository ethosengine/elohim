# Cache Stream Warm-Up Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the HTTP pull warm-up with an SSE stream from elohim-storage, triggered on startup and subscriber reconnect.

**Architecture:** Storage exposes `GET /api/v1/cache/stream` that queries SQLite for cacheable content (filtered by reach) and streams each row as an SSE event. Doorway consumes with reqwest streaming, projects each item into MongoDB. Re-triggered on subscriber reconnect to cover signals missed during disconnection.

**Tech Stack:** Hyper SSE (storage), reqwest stream (doorway), Diesel/SQLite queries, tokio::mpsc for backpressure

**Design doc:** `genesis/plans/2026-03-25-cache-stream-warmup-design.md`

---

### Task 1: Storage — Cache Stream DB Queries

Add functions to query cacheable content from SQLite, filtered by reach level.

**Files:**
- Create: `elohim/elohim-storage/src/db/cache_queries.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` — add `pub mod cache_queries;`

**Step 1: Write the failing test**

```rust
// In cache_queries.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cacheable_content_filters_by_reach() {
        // Uses test DB helper pattern from existing diesel tests
        let pool = crate::db::test_helpers::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::new("lamad");

        // Insert commons content and private content
        // ... (use create_content from content_diesel.rs)

        let results = list_cacheable_content(&mut conn, &ctx, 100, 0).unwrap();

        // Only commons content should be returned
        assert!(results.iter().all(|c| c.reach == "commons"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test cache_queries -v`
Expected: FAIL — module doesn't exist

**Step 3: Write implementation**

```rust
//! Cache-eligible content queries for the cache stream endpoint.
//!
//! These queries return content filtered by reach level — only commons/public
//! content is eligible for projection cache warm-up.

use diesel::prelude::*;

use super::context::AppContext;
use super::diesel_schema::{content, humans, paths, relationships};
use super::models::{Content, Human, Path, Relationship};
use crate::error::StorageError;

/// List content with reach = 'commons' (cacheable for projection)
pub fn list_cacheable_content(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Content>, StorageError> {
    content::table
        .filter(content::app_id.eq(&ctx.app_id))
        .filter(content::reach.eq("commons"))
        .order(content::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable content query failed: {e}")))
}

/// List all paths (all are public per cache rules)
pub fn list_cacheable_paths(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Path>, StorageError> {
    paths::table
        .filter(paths::app_id.eq(&ctx.app_id))
        .order(paths::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable paths query failed: {e}")))
}

/// List humans with profile_reach = 'public'
pub fn list_cacheable_humans(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Human>, StorageError> {
    humans::table
        .filter(humans::app_id.eq(&ctx.app_id))
        .filter(humans::profile_reach.eq("public"))
        .order(humans::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable humans query failed: {e}")))
}

/// List relationships with reach = 'commons'
pub fn list_cacheable_relationships(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<Relationship>, StorageError> {
    relationships::table
        .filter(relationships::app_id.eq(&ctx.app_id))
        .filter(relationships::reach.eq("commons"))
        .order(relationships::updated_at.asc())
        .limit(limit)
        .offset(offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Cacheable relationships query failed: {e}")))
}
```

**Step 4: Run tests to verify they pass**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test cache_queries -v`
Expected: PASS

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/db/cache_queries.rs elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): add cache-eligible content queries filtered by reach"
```

---

### Task 2: Storage — SSE Cache Stream Handler

Create the SSE streaming handler that reads from the DB queries and emits events.

**Files:**
- Create: `elohim/elohim-storage/src/cache_stream.rs`
- Modify: `elohim/elohim-storage/src/lib.rs` — add `pub mod cache_stream;`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_cache_event() {
        let data = serde_json::json!({"id": "test-1", "title": "Test"});
        let event = format_cache_event("cache.content", "test-1", &data);
        assert!(event.starts_with("event: cache.content\n"));
        assert!(event.contains("id: test-1\n"));
        assert!(event.contains("data: {"));
        assert!(event.ends_with("\n\n"));
    }

    #[test]
    fn test_format_done_event() {
        let counts = StreamCounts { content: 10, paths: 3, humans: 2, relationships: 5 };
        let event = format_done_event(&counts);
        assert!(event.starts_with("event: cache.done\n"));
        assert!(event.contains("\"content\":10"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test cache_stream -v`
Expected: FAIL — module doesn't exist

**Step 3: Write implementation**

```rust
//! Cache Stream — SSE endpoint for projection cache warm-up.
//!
//! Streams all cacheable content (filtered by reach) as Server-Sent Events.
//! Doorway consumes this on startup and subscriber reconnect to populate
//! its MongoDB projection cache.
//!
//! ## Event Format
//!
//! ```text
//! event: cache.content
//! id: manifesto
//! data: {"id":"manifesto","title":"The Elohim Protocol",...}
//!
//! event: cache.path
//! id: governance-intro
//! data: {"id":"governance-intro","title":"Introduction to Governance",...}
//!
//! event: cache.done
//! data: {"content":342,"paths":12,"humans":5,"relationships":89}
//! ```

use std::convert::Infallible;
use std::pin::Pin;

use bytes::Bytes;
use http_body_util::StreamBody;
use hyper::body::Frame;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::db::cache_queries;
use crate::db::content_diesel::DbPool;
use crate::db::context::AppContext;
use crate::sse::SseBody;
use crate::views::{ContentView, HumanView, PathView, RelationshipView};

const BATCH_SIZE: i64 = 500;

/// Counts of items streamed, sent in the cache.done event
#[derive(Debug, Default, Serialize)]
pub struct StreamCounts {
    pub content: usize,
    pub paths: usize,
    pub humans: usize,
    pub relationships: usize,
}

/// Format a single cache SSE event
pub fn format_cache_event(event_type: &str, id: &str, data: &serde_json::Value) -> String {
    format!("event: {event_type}\nid: {id}\ndata: {data}\n\n")
}

/// Format the done event with counts
pub fn format_done_event(counts: &StreamCounts) -> String {
    let data = serde_json::to_string(counts).unwrap_or_default();
    format!("event: cache.done\ndata: {data}\n\n")
}

/// Format a heartbeat comment
fn format_heartbeat() -> String {
    ": heartbeat\n\n".to_string()
}

/// Create an SSE cache stream response.
///
/// Spawns a background task that queries SQLite in batches and sends
/// each cacheable item as an SSE event. The stream ends with a
/// `cache.done` event containing item counts.
pub fn create_cache_stream(pool: DbPool, app_id: &str) -> hyper::Response<SseBody> {
    let (tx, rx) = mpsc::channel::<Bytes>(64);
    let app_id = app_id.to_string();

    // Spawn the DB reader task
    tokio::spawn(async move {
        let ctx = AppContext::new(&app_id);
        let mut counts = StreamCounts::default();

        // Stream content (reach = 'commons')
        counts.content = stream_table(&tx, &pool, &ctx, "cache.content", |conn, ctx, limit, offset| {
            cache_queries::list_cacheable_content(conn, ctx, limit, offset)
                .map(|items| items.into_iter().map(|c| {
                    let view = ContentView::from(c);
                    let id = view.id.clone();
                    (id, serde_json::to_value(&view).unwrap_or_default())
                }).collect())
        }).await;

        // Stream paths (all public)
        counts.paths = stream_table(&tx, &pool, &ctx, "cache.path", |conn, ctx, limit, offset| {
            cache_queries::list_cacheable_paths(conn, ctx, limit, offset)
                .map(|items| items.into_iter().map(|p| {
                    let view = PathView::from(p);
                    let id = view.id.clone();
                    (id, serde_json::to_value(&view).unwrap_or_default())
                }).collect())
        }).await;

        // Stream humans (profile_reach = 'public')
        counts.humans = stream_table(&tx, &pool, &ctx, "cache.human", |conn, ctx, limit, offset| {
            cache_queries::list_cacheable_humans(conn, ctx, limit, offset)
                .map(|items| items.into_iter().map(|h| {
                    let view = HumanView::from(h);
                    let id = view.id.clone();
                    (id, serde_json::to_value(&view).unwrap_or_default())
                }).collect())
        }).await;

        // Stream relationships (reach = 'commons')
        counts.relationships = stream_table(&tx, &pool, &ctx, "cache.relationship", |conn, ctx, limit, offset| {
            cache_queries::list_cacheable_relationships(conn, ctx, limit, offset)
                .map(|items| items.into_iter().map(|r| {
                    let view = RelationshipView::from(r);
                    let id = view.id.clone();
                    (id, serde_json::to_value(&view).unwrap_or_default())
                }).collect())
        }).await;

        // Send done event
        let done = format_done_event(&counts);
        let _ = tx.send(Bytes::from(done)).await;

        info!(
            content = counts.content,
            paths = counts.paths,
            humans = counts.humans,
            relationships = counts.relationships,
            "Cache stream complete"
        );
    });

    // Build SSE response from mpsc receiver + heartbeat
    let rx_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    let events = tokio_stream::StreamExt::map(rx_stream, |bytes| {
        Ok::<_, Infallible>(Frame::data(bytes))
    });

    let heartbeat = tokio_stream::wrappers::IntervalStream::new(
        tokio::time::interval(std::time::Duration::from_secs(30)),
    );
    let heartbeats = tokio_stream::StreamExt::map(heartbeat, |_| {
        Ok(Frame::data(Bytes::from(format_heartbeat())))
    });

    let merged = tokio_stream::StreamExt::merge(events, heartbeats);
    let pinned: Pin<Box<dyn futures_util::Stream<Item = Result<Frame<Bytes>, Infallible>> + Send>> =
        Box::pin(merged);

    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .body(StreamBody::new(pinned))
        .expect("SSE response builder should not fail")
}

/// Stream a single table in batches, sending SSE events via the channel.
///
/// Returns the total number of items streamed.
async fn stream_table<F>(
    tx: &mpsc::Sender<Bytes>,
    pool: &DbPool,
    ctx: &AppContext,
    event_type: &str,
    query_fn: F,
) -> usize
where
    F: Fn(&mut diesel::SqliteConnection, &AppContext, i64, i64)
        -> Result<Vec<(String, serde_json::Value)>, crate::error::StorageError>
        + Send + 'static,
    F: Clone,
{
    let mut total = 0usize;
    let mut offset = 0i64;

    loop {
        let pool = pool.clone();
        let ctx_clone = ctx.clone();
        let query_fn = query_fn.clone();
        let current_offset = offset;

        // Run DB query on blocking thread
        let batch = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                crate::error::StorageError::Internal(format!("Pool error: {e}"))
            })?;
            query_fn(&mut conn, &ctx_clone, BATCH_SIZE, current_offset)
        })
        .await
        .unwrap_or_else(|e| Err(crate::error::StorageError::Internal(format!("Join error: {e}"))));

        match batch {
            Ok(items) => {
                let count = items.len();
                if count == 0 {
                    break;
                }

                for (id, data) in &items {
                    let event = format_cache_event(event_type, id, data);
                    if tx.send(Bytes::from(event)).await.is_err() {
                        debug!("Cache stream client disconnected");
                        return total;
                    }
                    total += 1;
                }

                if count < BATCH_SIZE as usize {
                    break; // Last batch
                }
                offset += BATCH_SIZE;
            }
            Err(e) => {
                warn!(event_type, error = %e, "Cache stream query failed");
                break;
            }
        }
    }

    total
}
```

**Step 4: Run tests to verify they pass**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test cache_stream -v`
Expected: PASS

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/cache_stream.rs elohim/elohim-storage/src/lib.rs
git commit -m "feat(storage): add SSE cache stream handler for projection warm-up"
```

---

### Task 3: Storage — Wire Route and Register in Manifest

Add the HTTP route and register it in the doorway manifest.

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` — add route match + manifest entry

**Step 1: Add route match in http.rs**

Find the existing SSE route match (around line 405):
```rust
// SSE event stream — must be matched before the /api/v1/ catch-all
(Method::GET, "/api/v1/events") => {
```

Add the cache stream route AFTER that block but BEFORE the `/api/v1/` catch-all:

```rust
// Cache stream for projection warm-up (SSE)
(Method::GET, "/api/v1/cache/stream") => {
    if let Some(ref pool) = self.db_pool {
        let response = crate::cache_stream::create_cache_stream(
            pool.clone(),
            "lamad", // Default app context for cache stream
        );
        return Ok(response.map(Either::Right));
    } else {
        Ok(response::service_unavailable("Database not available"))
    }
}
```

**Step 2: Add to build_manifest()**

Find `build_manifest()` (around line 5577) and add:

```rust
.route(Route::get("/api/v1/cache/stream")
    .handler("cache_stream")
    .build())
```

**Step 3: Build and verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`
Expected: BUILD SUCCESS

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): wire cache stream SSE route and register in manifest"
```

---

### Task 4: Doorway — SSE Stream Consumer

Create the doorway-side SSE consumer that replaces the HTTP pull warm-up.

**Files:**
- Create: `doorway/doorway-service/src/projection/warm_stream.rs`
- Modify: `doorway/doorway-service/src/projection/mod.rs` — add `pub mod warm_stream;`
- Modify: `doorway/doorway-service/Cargo.toml` — add `stream` feature to reqwest

**Step 1: Add reqwest `stream` feature**

In `doorway/doorway-service/Cargo.toml`, change:
```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
```
to:
```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "stream"] }
```

**Step 2: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_event_content() {
        let lines = vec![
            "event: cache.content".to_string(),
            "id: manifesto".to_string(),
            r#"data: {"id":"manifesto","title":"Test"}"#.to_string(),
            "".to_string(),
        ];

        let event = parse_sse_event(&lines).unwrap();
        assert_eq!(event.event_type, "cache.content");
        assert_eq!(event.id, "manifesto");
        assert!(event.data.is_object());
    }

    #[test]
    fn test_parse_sse_event_done() {
        let lines = vec![
            "event: cache.done".to_string(),
            r#"data: {"content":10,"paths":3,"humans":2,"relationships":5}"#.to_string(),
            "".to_string(),
        ];

        let event = parse_sse_event(&lines).unwrap();
        assert_eq!(event.event_type, "cache.done");
    }

    #[test]
    fn test_parse_sse_heartbeat_returns_none() {
        let lines = vec![": heartbeat".to_string(), "".to_string()];
        assert!(parse_sse_event(&lines).is_none());
    }

    #[test]
    fn test_event_type_to_doc_type() {
        assert_eq!(event_type_to_doc_type("cache.content"), Some("Content"));
        assert_eq!(event_type_to_doc_type("cache.path"), Some("LearningPath"));
        assert_eq!(event_type_to_doc_type("cache.human"), Some("Human"));
        assert_eq!(event_type_to_doc_type("cache.relationship"), Some("Relationship"));
        assert_eq!(event_type_to_doc_type("cache.done"), None);
    }
}
```

**Step 3: Run test to verify it fails**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test warm_stream -v`
Expected: FAIL — module doesn't exist

**Step 4: Write implementation**

```rust
//! SSE Cache Stream Consumer — warm-up via streaming from peer storage.
//!
//! Replaces the old HTTP pull warm-up (`warm.rs`). Connects to each peer's
//! `GET /api/v1/cache/stream` endpoint and processes SSE events into the
//! projection store (MongoDB).
//!
//! ## Trigger Points
//!
//! 1. **Startup** — for each peer storage URL
//! 2. **Subscriber reconnect** — after AppWebsocket reconnects to conductor

use std::sync::Arc;

use futures_util::StreamExt;
use serde_json::Value as JsonValue;
use tracing::{debug, info, warn};

use super::document::ProjectedDocument;
use super::store::ProjectionStore;

/// A parsed SSE event
#[derive(Debug)]
pub struct SseEvent {
    pub event_type: String,
    pub id: String,
    pub data: JsonValue,
}

/// Parse accumulated SSE lines into an event.
/// Returns None for heartbeats/comments.
pub fn parse_sse_event(lines: &[String]) -> Option<SseEvent> {
    let mut event_type = String::new();
    let mut id = String::new();
    let mut data = String::new();

    for line in lines {
        if line.starts_with(": ") || line == ":" {
            return None; // Comment/heartbeat
        } else if let Some(val) = line.strip_prefix("event: ") {
            event_type = val.to_string();
        } else if let Some(val) = line.strip_prefix("id: ") {
            id = val.to_string();
        } else if let Some(val) = line.strip_prefix("data: ") {
            data = val.to_string();
        }
    }

    if event_type.is_empty() && data.is_empty() {
        return None;
    }

    let data = serde_json::from_str(&data).unwrap_or(JsonValue::Null);

    Some(SseEvent {
        event_type,
        id,
        data,
    })
}

/// Map SSE event type to ProjectedDocument doc_type
pub fn event_type_to_doc_type(event_type: &str) -> Option<&'static str> {
    match event_type {
        "cache.content" => Some("Content"),
        "cache.path" => Some("LearningPath"),
        "cache.human" => Some("Human"),
        "cache.relationship" => Some("Relationship"),
        _ => None,
    }
}

/// Result of a cache stream warm-up
#[derive(Debug, Default)]
pub struct StreamResult {
    pub content_count: usize,
    pub path_count: usize,
    pub human_count: usize,
    pub relationship_count: usize,
    pub errors: Vec<String>,
}

/// Stream cacheable content from a peer's storage into the projection store.
///
/// Connects to `GET {storage_url}/api/v1/cache/stream` and processes each
/// SSE event into a `ProjectedDocument` stored in MongoDB.
pub async fn stream_from_peer(
    store: Arc<ProjectionStore>,
    storage_url: &str,
) -> StreamResult {
    let mut result = StreamResult::default();
    let base = storage_url.trim_end_matches('/');
    let url = format!("{base}/api/v1/cache/stream");

    info!(url = %url, "Starting cache stream from peer");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 min for full stream
        .build()
        .unwrap();

    let response = match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => resp,
        Ok(resp) => {
            let status = resp.status();
            result.errors.push(format!("HTTP {status} from {url}"));
            warn!(url = %url, status = %status, "Cache stream request failed");
            return result;
        }
        Err(e) => {
            result.errors.push(format!("Connection failed: {e}"));
            warn!(url = %url, error = %e, "Cache stream connection failed");
            return result;
        }
    };

    // Read the streaming response line by line
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut current_event_lines: Vec<String> = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Cache stream read error");
                result.errors.push(format!("Stream read error: {e}"));
                break;
            }
        };

        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete lines
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                // Empty line = end of event
                if !current_event_lines.is_empty() {
                    if let Some(event) = parse_sse_event(&current_event_lines) {
                        process_event(&store, &event, &mut result).await;

                        // Stop on done event
                        if event.event_type == "cache.done" {
                            info!(
                                content = result.content_count,
                                paths = result.path_count,
                                humans = result.human_count,
                                relationships = result.relationship_count,
                                "Cache stream warm-up complete from {}", base
                            );
                            return result;
                        }
                    }
                    current_event_lines.clear();
                }
            } else {
                current_event_lines.push(line);
            }
        }
    }

    info!(
        content = result.content_count,
        paths = result.path_count,
        humans = result.human_count,
        relationships = result.relationship_count,
        "Cache stream ended (no done event) from {}", base
    );

    result
}

/// Process a single SSE event into the projection store
async fn process_event(
    store: &ProjectionStore,
    event: &SseEvent,
    result: &mut StreamResult,
) {
    let doc_type = match event_type_to_doc_type(&event.event_type) {
        Some(dt) => dt,
        None => return, // cache.done or unknown — skip
    };

    let doc = ProjectedDocument::new(
        doc_type,
        &event.id,
        "cache-stream",
        "cache-stream",
        event.data.clone(),
    );

    match store.set(doc).await {
        Ok(_) => {
            match doc_type {
                "Content" => result.content_count += 1,
                "LearningPath" => result.path_count += 1,
                "Human" => result.human_count += 1,
                "Relationship" => result.relationship_count += 1,
                _ => {}
            }
        }
        Err(e) => {
            warn!(
                doc_type,
                id = %event.id,
                error = %e,
                "Failed to project cache stream entry"
            );
            result.errors.push(format!("{doc_type}:{}: {e}", event.id));
        }
    }
}

/// Spawn cache stream warm-up for multiple peers as a background task.
///
/// This is the startup entry point — called from main.rs to replace
/// the old `spawn_warm_task`.
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
            let result = stream_from_peer(Arc::clone(&store), storage_url).await;

            if result.errors.is_empty() {
                info!(
                    storage_url,
                    content = result.content_count,
                    paths = result.path_count,
                    "Cache stream completed successfully"
                );
            } else {
                warn!(
                    storage_url,
                    errors = ?result.errors,
                    "Cache stream completed with errors"
                );
            }
        }
    })
}
```

**Step 5: Run tests to verify they pass**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test warm_stream -v`
Expected: PASS

**Step 6: Commit**

```bash
git add doorway/doorway-service/src/projection/warm_stream.rs doorway/doorway-service/src/projection/mod.rs doorway/doorway-service/Cargo.toml
git commit -m "feat(doorway): add SSE cache stream consumer for projection warm-up"
```

---

### Task 5: Doorway — Wire Startup and Subscriber Reconnect

Replace the old warm-up call in main.rs and wire the reconnect trigger.

**Files:**
- Modify: `doorway/doorway-service/src/main.rs` — replace `spawn_warm_task` with `spawn_stream_task`
- Modify: `doorway/doorway-service/src/projection/subscriber.rs` — trigger stream after reconnect
- Modify: `doorway/doorway-service/src/projection/subscriber.rs` — add storage_url + store to config

**Step 1: Update SubscriberConfig**

In `subscriber.rs`, add to `SubscriberConfig`:

```rust
/// Storage URL for cache stream warm-up on reconnect
pub storage_url: Option<String>,
/// Projection store for cache stream warm-up on reconnect
#[allow(dead_code)]
pub projection_store: Option<Arc<ProjectionStore>>,
```

Update `Default` and `from_env()` to set both to `None`.

**Step 2: Add reconnect trigger in subscriber.rs**

In the `run()` method, after `AppWebsocket::connect()` succeeds (after line ~370 where `reconnect_attempts = 0`), add:

```rust
// Trigger cache stream warm-up on (re)connect
if let (Some(ref storage_url), Some(ref store)) =
    (&self.config.storage_url, &self.config.projection_store)
{
    let store = Arc::clone(store);
    let url = storage_url.clone();
    tokio::spawn(async move {
        super::warm_stream::stream_from_peer(store, &url).await;
    });
    info!("Cache stream warm-up triggered after connect");
}
```

**Step 3: Update main.rs startup**

Replace the warm-up block (around line 598-612):

```rust
// Old:
// if args.projection_writer && !peer_urls.is_empty() {
//     if let Some(ref projection_store) = state.projection {
//         let _warm_handle = doorway::projection::warm::spawn_warm_task(
//             Arc::clone(projection_store),
//             peer_urls.clone(),
//             10,
//         );

// New:
if args.projection_writer && !peer_urls.is_empty() {
    if let Some(ref projection_store) = state.projection {
        let _warm_handle = doorway::projection::warm_stream::spawn_stream_task(
            Arc::clone(projection_store),
            peer_urls.clone(),
            10, // 10s delay — let MongoDB + storage settle
        );
        info!(
            peers = peer_urls.len(),
            "Cache stream warm-up scheduled (10s delay)"
        );
    }
}
```

Also update the subscriber config creation (around line 511) to pass storage URL and projection store:

```rust
let subscriber_config = SubscriberConfig {
    admin_url: admin_url.clone(),
    app_url: conductor_app_url.clone(),
    installed_app_id: args.installed_app_id.clone(),
    storage_url: peer_urls.get(i).cloned(),
    projection_store: state.projection.as_ref().map(Arc::clone),
    ..SubscriberConfig::default()
};
```

**Step 4: Build and verify**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo build`
Expected: BUILD SUCCESS

**Step 5: Commit**

```bash
git add doorway/doorway-service/src/main.rs doorway/doorway-service/src/projection/subscriber.rs
git commit -m "feat(doorway): wire cache stream to startup and subscriber reconnect"
```

---

### Task 6: Deprecate Old Warm-Up

Mark the old warm.rs as deprecated and update module exports.

**Files:**
- Modify: `doorway/doorway-service/src/projection/warm.rs` — add deprecation notices
- Modify: `doorway/doorway-service/src/projection/mod.rs` — update re-exports

**Step 1: Add deprecation to warm.rs**

Add at the top of `warm.rs` after the module doc:

```rust
#![allow(deprecated)]
```

Add `#[deprecated(since = "0.1.0", note = "Use warm_stream::spawn_stream_task instead")]` to `spawn_warm_task` and `warm_projection_cache`.

**Step 2: Update mod.rs re-exports**

Add to `mod.rs`:
```rust
pub mod warm_stream;
```

Ensure `warm_stream::spawn_stream_task` is re-exported if desired.

**Step 3: Build and verify no warnings leak**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo build 2>&1 | grep -i warn`
Expected: No unexpected warnings (deprecation warnings only from warm.rs itself, suppressed by `#![allow(deprecated)]`)

**Step 4: Commit**

```bash
git add doorway/doorway-service/src/projection/warm.rs doorway/doorway-service/src/projection/mod.rs
git commit -m "chore(doorway): deprecate HTTP pull warm-up in favor of cache stream"
```

---

### Task 7: Integration Test

Verify the full pipeline end-to-end with a manual smoke test.

**Files:** None (testing only)

**Step 1: Build both services**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
cd doorway/doorway-service && RUSTFLAGS="" cargo build --release
```

**Step 2: Verify storage endpoint with curl**

Start storage locally, then:

```bash
curl -N http://localhost:8090/api/v1/cache/stream
```

Expected output (SSE events streaming):
```
event: cache.content
id: manifesto
data: {"id":"manifesto","title":"The Elohim Protocol",...}

event: cache.path
id: governance-intro
data: {"id":"governance-intro","title":"Introduction to Governance",...}

event: cache.done
data: {"content":N,"paths":N,"humans":N,"relationships":N}
```

**Step 3: Run all unit tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins
```

Expected: ALL PASS

**Step 4: Run clippy**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
cd doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings
```

Expected: No warnings

**Step 5: Final commit (if any fixes needed)**

```bash
git commit -m "test: verify cache stream warm-up pipeline"
```
