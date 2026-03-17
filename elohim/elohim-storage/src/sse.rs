//! Server-Sent Events (SSE) streaming handler
//!
//! Provides real-time event streaming to browser clients via the standard
//! SSE protocol (text/event-stream). Events come from the existing EventBus
//! (tokio::broadcast) which carries StorageEvent variants from all service
//! layer mutations.
//!
//! ## Usage
//!
//! ```text
//! GET /api/v1/events
//! Accept: text/event-stream
//! ```
//!
//! ## Event Format
//!
//! ```text
//! event: content.created
//! id: 42
//! data: {"id":"abc","title":"My Content"}
//!
//! ```

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::StreamBody;
use hyper::body::Frame;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tracing::{debug, trace};

use crate::services::events::{EventBus, StorageEvent};

/// Global event ID counter (monotonically increasing across all SSE connections)
static EVENT_ID: AtomicU64 = AtomicU64::new(1);

/// The SSE body type — a stream of `Frame<Bytes>` that never errors
pub type SseBody =
    StreamBody<Pin<Box<dyn futures_util::Stream<Item = Result<Frame<Bytes>, Infallible>> + Send>>>;

/// Map a StorageEvent to its SSE event type string
fn event_type(event: &StorageEvent) -> &'static str {
    match event {
        StorageEvent::ContentCreated { .. } => "content.created",
        StorageEvent::ContentUpdated { .. } => "content.updated",
        StorageEvent::ContentDeleted { .. } => "content.deleted",
        StorageEvent::ContentBulkCreated { .. } => "content.bulk-created",
        StorageEvent::PathCreated { .. } => "path.created",
        StorageEvent::PathUpdated { .. } => "path.updated",
        StorageEvent::PathDeleted { .. } => "path.deleted",
        StorageEvent::PathBulkCreated { .. } => "path.bulk-created",
        StorageEvent::RelationshipCreated { .. } => "relationship.created",
        StorageEvent::RelationshipDeleted { .. } => "relationship.deleted",
        StorageEvent::RelationshipBulkCreated { .. } => "relationship.bulk-created",
        StorageEvent::KnowledgeMapCreated { .. } => "knowledge-map.created",
        StorageEvent::KnowledgeMapUpdated { .. } => "knowledge-map.updated",
        StorageEvent::KnowledgeMapDeleted { .. } => "knowledge-map.deleted",
        StorageEvent::PathExtensionCreated { .. } => "path-extension.created",
        StorageEvent::PathExtensionUpdated { .. } => "path-extension.updated",
        StorageEvent::PathExtensionDeleted { .. } => "path-extension.deleted",
    }
}

/// Serialize a StorageEvent to its JSON data payload
fn event_data(event: &StorageEvent) -> String {
    match event {
        StorageEvent::ContentCreated {
            id,
            title,
            content_type,
        } => {
            serde_json::json!({ "id": id, "title": title, "contentType": content_type }).to_string()
        }
        StorageEvent::ContentUpdated { id } => serde_json::json!({ "id": id }).to_string(),
        StorageEvent::ContentDeleted { id } => serde_json::json!({ "id": id }).to_string(),
        StorageEvent::ContentBulkCreated { count, ids } => {
            serde_json::json!({ "count": count, "ids": ids }).to_string()
        }
        StorageEvent::PathCreated { id, title } => {
            serde_json::json!({ "id": id, "title": title }).to_string()
        }
        StorageEvent::PathUpdated { id } => serde_json::json!({ "id": id }).to_string(),
        StorageEvent::PathDeleted { id } => serde_json::json!({ "id": id }).to_string(),
        StorageEvent::PathBulkCreated { count, ids } => {
            serde_json::json!({ "count": count, "ids": ids }).to_string()
        }
        StorageEvent::RelationshipCreated {
            id,
            source_id,
            target_id,
            relationship_type,
        } => serde_json::json!({
            "id": id,
            "sourceId": source_id,
            "targetId": target_id,
            "relationshipType": relationship_type,
        })
        .to_string(),
        StorageEvent::RelationshipDeleted { id } => serde_json::json!({ "id": id }).to_string(),
        StorageEvent::RelationshipBulkCreated { count } => {
            serde_json::json!({ "count": count }).to_string()
        }
        StorageEvent::KnowledgeMapCreated {
            id,
            map_type,
            owner_id,
        } => serde_json::json!({ "id": id, "mapType": map_type, "ownerId": owner_id }).to_string(),
        StorageEvent::KnowledgeMapUpdated { id } => serde_json::json!({ "id": id }).to_string(),
        StorageEvent::KnowledgeMapDeleted { id } => serde_json::json!({ "id": id }).to_string(),
        StorageEvent::PathExtensionCreated {
            id,
            base_path_id,
            extended_by,
        } => serde_json::json!({ "id": id, "basePathId": base_path_id, "extendedBy": extended_by })
            .to_string(),
        StorageEvent::PathExtensionUpdated { id } => serde_json::json!({ "id": id }).to_string(),
        StorageEvent::PathExtensionDeleted { id } => serde_json::json!({ "id": id }).to_string(),
    }
}

/// Format a single SSE event as a string
///
/// Output format:
/// ```text
/// event: content.created
/// id: 42
/// data: {"id":"abc","title":"My Content"}
///
/// ```
pub fn format_sse_event(event: &StorageEvent) -> String {
    let id = EVENT_ID.fetch_add(1, Ordering::Relaxed);
    let etype = event_type(event);
    let data = event_data(event);
    format!("event: {}\nid: {}\ndata: {}\n\n", etype, id, data)
}

/// Format a heartbeat comment (keeps connection alive through proxies)
pub fn format_heartbeat() -> String {
    ": heartbeat\n\n".to_string()
}

/// Create an SSE streaming response from the EventBus
///
/// Returns a `Response` with `Content-Type: text/event-stream` and a streaming
/// body that emits events as they arrive from the broadcast channel, plus a
/// heartbeat comment every 30 seconds.
pub fn create_sse_stream(event_bus: &Arc<EventBus>) -> hyper::Response<SseBody> {
    let rx = event_bus.subscribe();
    let broadcast_stream = BroadcastStream::new(rx);

    debug!(
        subscribers = event_bus.subscriber_count(),
        "New SSE client connected"
    );

    // Merge event stream with heartbeat interval
    let heartbeat = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(30),
    ));

    // Event frames from broadcast — map all results to SSE frames
    let events = broadcast_stream.map(|result| {
        match result {
            Ok(event) => {
                trace!(event_type = event_type(&event), "SSE: emitting event");
                let text = format_sse_event(&event);
                Ok(Frame::data(Bytes::from(text)))
            }
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                debug!(skipped = n, "SSE client lagged, skipped events");
                // Send a comment indicating lag
                let text = format!(": lagged, skipped {} events\n\n", n);
                Ok(Frame::data(Bytes::from(text)))
            }
        }
    });

    // Heartbeat frames
    let heartbeats = heartbeat.map(|_| Ok(Frame::data(Bytes::from(format_heartbeat()))));

    // Merge both streams — events + heartbeats
    let merged = events.merge(heartbeats);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_sse_event_content_created() {
        let event = StorageEvent::ContentCreated {
            id: "test-1".into(),
            title: "My Content".into(),
            content_type: Some("journal".into()),
        };
        let formatted = format_sse_event(&event);
        assert!(formatted.starts_with("event: content.created\n"));
        assert!(formatted.contains("id: "));
        assert!(formatted.contains(r#""id":"test-1""#));
        assert!(formatted.contains(r#""title":"My Content""#));
        assert!(formatted.ends_with("\n\n"));
    }

    #[test]
    fn test_format_sse_event_relationship_created() {
        let event = StorageEvent::RelationshipCreated {
            id: "rel-1".into(),
            source_id: "a".into(),
            target_id: "b".into(),
            relationship_type: "requires".into(),
        };
        let formatted = format_sse_event(&event);
        assert!(formatted.starts_with("event: relationship.created\n"));
        assert!(formatted.contains(r#""sourceId":"a""#));
        assert!(formatted.contains(r#""targetId":"b""#));
    }

    #[test]
    fn test_format_heartbeat() {
        let hb = format_heartbeat();
        assert_eq!(hb, ": heartbeat\n\n");
    }

    #[test]
    fn test_event_type_mapping() {
        assert_eq!(
            event_type(&StorageEvent::ContentCreated {
                id: "".into(),
                title: "".into(),
                content_type: None
            }),
            "content.created"
        );
        assert_eq!(
            event_type(&StorageEvent::PathDeleted { id: "".into() }),
            "path.deleted"
        );
        assert_eq!(
            event_type(&StorageEvent::KnowledgeMapCreated {
                id: "".into(),
                map_type: "".into(),
                owner_id: "".into()
            }),
            "knowledge-map.created"
        );
    }

    #[test]
    fn test_event_ids_are_monotonic() {
        let e1 = StorageEvent::ContentUpdated { id: "a".into() };
        let e2 = StorageEvent::ContentUpdated { id: "b".into() };
        let f1 = format_sse_event(&e1);
        let f2 = format_sse_event(&e2);
        // Extract IDs
        let id1: u64 = f1
            .lines()
            .find(|l| l.starts_with("id: "))
            .unwrap()
            .strip_prefix("id: ")
            .unwrap()
            .parse()
            .unwrap();
        let id2: u64 = f2
            .lines()
            .find(|l| l.starts_with("id: "))
            .unwrap()
            .strip_prefix("id: ")
            .unwrap()
            .parse()
            .unwrap();
        assert!(id2 > id1);
    }
}
