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
pub async fn stream_from_peer(store: Arc<ProjectionStore>, storage_url: &str) -> StreamResult {
    let mut result = StreamResult::default();
    let base = storage_url.trim_end_matches('/');
    let url = format!("{base}/api/v1/cache/stream");

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout for full stream
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            result
                .errors
                .push(format!("Failed to build HTTP client: {e}"));
            return result;
        }
    };

    let response = match client.get(&url).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                result
                    .errors
                    .push(format!("HTTP {} from {url}", resp.status()));
                return result;
            }
            resp
        }
        Err(e) => {
            result
                .errors
                .push(format!("Failed to connect to {url}: {e}"));
            return result;
        }
    };

    info!(url = %url, "Connected to cache stream");

    // Stream response bytes and parse SSE lines
    let mut byte_stream = response.bytes_stream();
    let mut line_buffer = String::new();
    let mut event_lines: Vec<String> = Vec::new();

    while let Some(chunk_result) = byte_stream.next().await {
        let chunk = match chunk_result {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!(error = %e, "Error reading stream chunk");
                result.errors.push(format!("Stream read error: {e}"));
                break;
            }
        };

        let text = match std::str::from_utf8(&chunk) {
            Ok(t) => t,
            Err(e) => {
                warn!(error = %e, "Non-UTF8 chunk in SSE stream");
                continue;
            }
        };

        line_buffer.push_str(text);

        // Process complete lines from the buffer
        while let Some(newline_pos) = line_buffer.find('\n') {
            let line = line_buffer[..newline_pos]
                .trim_end_matches('\r')
                .to_string();
            line_buffer = line_buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                // Empty line = end of SSE event
                if !event_lines.is_empty() {
                    if let Some(event) = parse_sse_event(&event_lines) {
                        // Check for done signal
                        if event.event_type == "cache.done" {
                            info!(
                                content = result.content_count,
                                paths = result.path_count,
                                humans = result.human_count,
                                relationships = result.relationship_count,
                                "Cache stream completed (done event)"
                            );
                            event_lines.clear();
                            return result;
                        }

                        // Map event type to doc_type and project
                        if let Some(doc_type) = event_type_to_doc_type(&event.event_type) {
                            let doc_id = if !event.id.is_empty() {
                                event.id.clone()
                            } else {
                                event
                                    .data
                                    .get("id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string()
                            };

                            let doc = ProjectedDocument::new(
                                doc_type,
                                &doc_id,
                                "cache-stream",
                                "cache-stream",
                                event.data,
                            );

                            if let Err(e) = store.set(doc).await {
                                warn!(
                                    doc_type,
                                    doc_id = %doc_id,
                                    error = %e,
                                    "Failed to project streamed entry"
                                );
                                result.errors.push(format!("{doc_type}:{doc_id}: {e}"));
                            } else {
                                match doc_type {
                                    "Content" => result.content_count += 1,
                                    "LearningPath" => result.path_count += 1,
                                    "Human" => result.human_count += 1,
                                    "Relationship" => result.relationship_count += 1,
                                    _ => {}
                                }
                                debug!(doc_type, doc_id = %doc_id, "Projected streamed entry");
                            }
                        }
                    }
                    event_lines.clear();
                }
            } else {
                event_lines.push(line);
            }
        }
    }

    info!(
        content = result.content_count,
        paths = result.path_count,
        humans = result.human_count,
        relationships = result.relationship_count,
        "Cache stream finished (stream ended)"
    );

    result
}

/// Spawn cache stream warm-up for multiple peers as a background task.
/// This is the startup entry point — called from main.rs.
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
                    storage_url = %storage_url,
                    content = result.content_count,
                    paths = result.path_count,
                    humans = result.human_count,
                    relationships = result.relationship_count,
                    "Cache stream warm-up completed successfully"
                );
            } else {
                warn!(
                    storage_url = %storage_url,
                    content = result.content_count,
                    paths = result.path_count,
                    humans = result.human_count,
                    relationships = result.relationship_count,
                    errors = ?result.errors,
                    "Cache stream warm-up completed with errors"
                );
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_event_content() {
        let lines = vec![
            "event: cache.content".to_string(),
            "id: manifesto".to_string(),
            r#"data: {"id":"manifesto","title":"Test"}"#.to_string(),
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
        ];
        let event = parse_sse_event(&lines).unwrap();
        assert_eq!(event.event_type, "cache.done");
    }

    #[test]
    fn test_parse_sse_heartbeat_returns_none() {
        let lines = vec![": heartbeat".to_string()];
        assert!(parse_sse_event(&lines).is_none());
    }

    #[test]
    fn test_parse_sse_bare_comment_returns_none() {
        let lines = vec![":".to_string()];
        assert!(parse_sse_event(&lines).is_none());
    }

    #[test]
    fn test_parse_sse_empty_lines_returns_none() {
        let lines: Vec<String> = vec![];
        assert!(parse_sse_event(&lines).is_none());
    }

    #[test]
    fn test_parse_sse_event_with_no_id() {
        let lines = vec![
            "event: cache.human".to_string(),
            r#"data: {"id":"agent-123","name":"Alice"}"#.to_string(),
        ];
        let event = parse_sse_event(&lines).unwrap();
        assert_eq!(event.event_type, "cache.human");
        assert_eq!(event.id, "");
        assert!(event.data.is_object());
    }

    #[test]
    fn test_parse_sse_event_invalid_json_data() {
        let lines = vec![
            "event: cache.content".to_string(),
            "id: test".to_string(),
            "data: not-valid-json".to_string(),
        ];
        let event = parse_sse_event(&lines).unwrap();
        assert_eq!(event.event_type, "cache.content");
        assert_eq!(event.data, JsonValue::Null);
    }

    #[test]
    fn test_event_type_to_doc_type() {
        assert_eq!(event_type_to_doc_type("cache.content"), Some("Content"));
        assert_eq!(event_type_to_doc_type("cache.path"), Some("LearningPath"));
        assert_eq!(event_type_to_doc_type("cache.human"), Some("Human"));
        assert_eq!(
            event_type_to_doc_type("cache.relationship"),
            Some("Relationship")
        );
        assert_eq!(event_type_to_doc_type("cache.done"), None);
    }

    #[test]
    fn test_event_type_to_doc_type_unknown() {
        assert_eq!(event_type_to_doc_type("cache.unknown"), None);
        assert_eq!(event_type_to_doc_type(""), None);
        assert_eq!(event_type_to_doc_type("something.else"), None);
    }

    #[test]
    fn test_stream_result_default() {
        let result = StreamResult::default();
        assert_eq!(result.content_count, 0);
        assert_eq!(result.path_count, 0);
        assert_eq!(result.human_count, 0);
        assert_eq!(result.relationship_count, 0);
        assert!(result.errors.is_empty());
    }
}
