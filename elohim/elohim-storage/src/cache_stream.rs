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
//! event: cache.done
//! data: {"content":342,"humans":5,"relationships":89}
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
use crate::db::context::AppContext;
use crate::db::DbPool;
use crate::sse::SseBody;
use crate::views::{ContentView, HumanView, RelationshipView};

const BATCH_SIZE: i64 = 500;

/// Counts of items streamed, sent in the cache.done event
#[derive(Debug, Default, Serialize)]
pub struct StreamCounts {
    pub content: usize,
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
#[allow(clippy::field_reassign_with_default)]
pub fn create_cache_stream(pool: DbPool, h_app_id: &str) -> hyper::Response<SseBody> {
    let (tx, rx) = mpsc::channel::<Bytes>(64);
    let h_app_id = h_app_id.to_string();

    // Spawn the DB reader task
    tokio::spawn(async move {
        let ctx = AppContext::new(&h_app_id);
        let mut counts = StreamCounts::default();

        // Stream content (reach = 'commons') — paths are now content rows with contentType 'path'
        counts.content = stream_table(&tx, &pool, &ctx, "cache.content", content_page_events).await;

        // Stream humans (profile_reach = 'public')
        counts.humans = stream_table(
            &tx,
            &pool,
            &ctx,
            "cache.human",
            |conn, ctx, limit, offset| {
                cache_queries::list_cacheable_humans(conn, ctx, limit, offset).map(|items| {
                    items
                        .into_iter()
                        .map(|h| {
                            let view = HumanView::from(h);
                            let id = view.id.clone();
                            (id, serde_json::to_value(&view).unwrap_or_default())
                        })
                        .collect()
                })
            },
        )
        .await;

        // Stream relationships (reach = 'commons')
        counts.relationships = stream_table(
            &tx,
            &pool,
            &ctx,
            "cache.relationship",
            |conn, ctx, limit, offset| {
                cache_queries::list_cacheable_relationships(conn, ctx, limit, offset).map(|items| {
                    items
                        .into_iter()
                        .map(|r| {
                            let view = RelationshipView::from(r);
                            let id = view.id.clone();
                            (id, serde_json::to_value(&view).unwrap_or_default())
                        })
                        .collect()
                })
            },
        )
        .await;

        // Send done event
        let done = format_done_event(&counts);
        let _ = tx.send(Bytes::from(done)).await;

        info!(
            content = counts.content,
            humans = counts.humans,
            relationships = counts.relationships,
            "Cache stream complete"
        );
    });

    // Build SSE response from mpsc receiver + heartbeat
    let rx_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    let events =
        tokio_stream::StreamExt::map(rx_stream, |bytes| Ok::<_, Infallible>(Frame::data(bytes)));

    let heartbeat = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(30),
    ));
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

/// One page of `cache.content` events. Each row carries its tags (one batched
/// tag read per page — `cache_queries::list_cacheable_content_with_tags`), so
/// the doorway's warmed and swept cache documents match the `/db/content/{id}`
/// read instead of arriving with `tags: []`.
fn content_page_events(
    conn: &mut diesel::SqliteConnection,
    ctx: &AppContext,
    limit: i64,
    offset: i64,
) -> Result<Vec<(String, serde_json::Value)>, crate::error::StorageError> {
    cache_queries::list_cacheable_content_with_tags(conn, ctx, limit, offset).map(|items| {
        items
            .into_iter()
            .map(|c| {
                let view = ContentView::from(c);
                let id = view.id.clone();
                (id, serde_json::to_value(&view).unwrap_or_default())
            })
            .collect()
    })
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
    F: Fn(
            &mut diesel::SqliteConnection,
            &AppContext,
            i64,
            i64,
        ) -> Result<Vec<(String, serde_json::Value)>, crate::error::StorageError>
        + Send
        + 'static
        + Clone,
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
            let mut conn = pool
                .get()
                .map_err(|e| crate::error::StorageError::Internal(format!("Pool error: {e}")))?;
            query_fn(&mut conn, &ctx_clone, BATCH_SIZE, current_offset)
        })
        .await
        .unwrap_or_else(|e| {
            Err(crate::error::StorageError::Internal(format!(
                "Join error: {e}"
            )))
        });

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
    fn test_format_cache_event_preserves_json() {
        let data = serde_json::json!({"id": "abc", "title": "Hello World"});
        let event = format_cache_event("cache.content", "abc", &data);
        assert!(event.contains(r#""id":"abc""#));
        assert!(event.contains(r#""title":"Hello World""#));
    }

    #[test]
    fn test_format_done_event() {
        let counts = StreamCounts {
            content: 10,
            humans: 2,
            relationships: 5,
        };
        let event = format_done_event(&counts);
        assert!(event.starts_with("event: cache.done\n"));
        assert!(event.contains("\"content\":10"));
        assert!(event.contains("\"humans\":2"));
        assert!(event.contains("\"relationships\":5"));
        assert!(event.ends_with("\n\n"));
    }

    #[test]
    fn test_format_done_event_zeros() {
        let counts = StreamCounts::default();
        let event = format_done_event(&counts);
        assert!(event.contains("\"content\":0"));
    }

    /// The streamed `cache.content` data is the `/db/content/{id}` view: the
    /// row's tags ride along (F: doorway cache docs lost tags on boot-warm and
    /// sweep when the stream built views from bare `Content` rows).
    #[test]
    fn streamed_content_rows_carry_their_tags() {
        use crate::db::content_diesel::{create_content, CreateContentInput};

        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::new("lamad");
        for (id, tags) in [("tagged", vec!["b", "a"]), ("untagged", vec![])] {
            create_content(
                &mut conn,
                &ctx,
                CreateContentInput {
                    id: id.into(),
                    title: id.into(),
                    description: None,
                    content_type: "concept".into(),
                    content_format: "markdown".into(),
                    blob_hash: None,
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "commons".into(),
                    created_by: None,
                    tags: tags.into_iter().map(String::from).collect(),
                    content_body: None,
                    dht_anchor_hash: None,
                },
            )
            .unwrap();
        }

        let events = content_page_events(&mut conn, &ctx, BATCH_SIZE, 0).unwrap();
        let tags_of = |id: &str| {
            events
                .iter()
                .find(|(event_id, _)| event_id == id)
                .map(|(_, data)| data["tags"].clone())
                .unwrap()
        };
        assert_eq!(tags_of("tagged"), serde_json::json!(["a", "b"]));
        assert_eq!(tags_of("untagged"), serde_json::json!([]));

        let read = crate::db::content_diesel::get_content_with_tags(
            &mut conn,
            &ctx,
            "tagged",
            crate::db::content_diesel::MinTrust::Invisible,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            tags_of("tagged"),
            serde_json::to_value(ContentView::from(read)).unwrap()["tags"],
            "a streamed row's tags equal the by-id read's"
        );
    }

    #[test]
    fn test_format_heartbeat() {
        let hb = format_heartbeat();
        assert_eq!(hb, ": heartbeat\n\n");
    }
}
