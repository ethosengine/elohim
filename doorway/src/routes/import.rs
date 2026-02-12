//! Import Route Handler - Forwards import requests to elohim-storage
//!
//! ## Architecture
//!
//! ```text
//! Seeder → Doorway → elohim-storage → Conductor
//!               │           │
//!          (relay)    (batching, single connection)
//! ```
//!
//! Doorway acts as a simple relay:
//! - If STORAGE_URL is set: forwards to elohim-storage
//! - If not set: returns 503 (storage not configured)
//!
//! elohim-storage owns the conductor connection and handles:
//! - Batch queuing and chunking
//! - Write buffering to avoid overwhelming conductor
//! - Progress tracking
//!
//! ## Endpoints (forwarded to storage)
//!
//! - POST /import/queue → elohim-storage /import/queue
//! - GET /import/status/{batch_id} → elohim-storage /import/status/{batch_id}

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::services::ImportConfigStore;

// =============================================================================
// Request/Response Types
// =============================================================================

/// Import queue request from client
///
/// Clients MUST upload their items blob to elohim-storage first,
/// then pass the blob_hash here. The zome stores only the manifest,
/// and elohim-storage orchestrates chunk processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportQueueRequest {
    /// Batch identifier (optional, will be generated if not provided)
    #[serde(default)]
    pub batch_id: Option<String>,
    /// Hash of the blob in elohim-storage containing the items JSON
    /// This is REQUIRED - upload to storage first, then queue import
    pub blob_hash: String,
    /// Total number of items in the blob
    pub total_items: u32,
    /// Schema version for the import data
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// Items per chunk (optional, uses server default if not provided)
    /// Smaller chunks = less conductor pressure, slower overall
    #[serde(default)]
    pub chunk_size: Option<usize>,
    /// Delay between chunks in ms (optional, uses server default if not provided)
    /// Higher delay = more conductor breathing room, slower overall
    #[serde(default)]
    pub chunk_delay_ms: Option<u64>,
}

fn default_schema_version() -> u32 {
    1
}

/// Import queue response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportQueueResponse {
    /// Assigned batch ID
    pub batch_id: String,
    /// Number of items queued
    pub queued_count: u32,
    /// Whether processing started immediately
    pub processing: bool,
    /// Optional message
    pub message: Option<String>,
}

/// Import status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatusResponse {
    /// Batch ID
    pub batch_id: String,
    /// Current status
    pub status: String,
    /// Total items in batch
    pub total_items: u32,
    /// Items processed so far
    pub processed_count: u32,
    /// Error count
    pub error_count: u32,
    /// Errors if any
    pub errors: Vec<String>,
    /// Completion timestamp if done
    pub completed_at: Option<String>,
}

// =============================================================================
// Route Matching
// =============================================================================

/// Check if a path matches any discovered import route
///
/// Returns (dna_hash, batch_type, optional batch_id) if matched
pub fn match_import_route(
    path: &str,
    import_store: &ImportConfigStore,
) -> Option<(String, String, Option<String>)> {
    // Iterate all discovered configs
    for dna_hash in import_store.get_import_enabled_dnas() {
        if let Some(config) = import_store.get_config(&dna_hash) {
            let base_route = &config.config.base_route;

            // Check if path starts with this DNA's base_route
            if path.starts_with(base_route) {
                let remainder = path.strip_prefix(base_route).unwrap_or("");
                let remainder = remainder.strip_prefix('/').unwrap_or(remainder);

                if remainder.is_empty() {
                    continue; // Just the base route, no batch type
                }

                // Parse: {batch_type} or {batch_type}/{batch_id}
                let parts: Vec<&str> = remainder.splitn(2, '/').collect();
                let batch_type = parts[0];

                // Verify this batch_type is supported
                if config.supports_batch_type(batch_type) {
                    let batch_id = parts.get(1).map(|s| s.to_string());
                    return Some((dna_hash, batch_type.to_string(), batch_id));
                }
            }
        }
    }

    None
}

// =============================================================================
// Route Handler
// =============================================================================

/// Handle import route request
///
/// Doorway forwards all import requests to elohim-storage.
/// elohim-storage owns the conductor connection and handles batching/buffering.
///
/// ## Routes
/// - POST /import/queue → forward to storage
/// - GET /import/status/{batch_id} → forward to storage
pub async fn handle_import_request(
    req: Request<Incoming>,
    storage_url: Option<String>,
    batch_type: String,
    batch_id: Option<String>,
) -> Response<Full<Bytes>> {
    let storage_url = match storage_url {
        Some(url) => url,
        None => {
            warn!("Import request received but STORAGE_URL not configured");
            return import_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Import service unavailable: STORAGE_URL not configured",
            );
        }
    };

    let method = req.method().clone();

    match method {
        Method::POST if batch_id.is_none() => {
            // POST /import/{batch_type} → forward to storage /import/queue
            forward_queue_import(req, &storage_url, &batch_type).await
        }
        Method::GET if batch_id.is_some() => {
            // GET /import/{batch_type}/{batch_id} → forward to storage /import/status/{batch_id}
            forward_get_status(&storage_url, batch_id.as_ref().unwrap()).await
        }
        _ => import_error_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "Use POST to queue imports, GET with batch_id to check status",
        ),
    }
}

/// Forward POST queue request to elohim-storage
async fn forward_queue_import(
    req: Request<Incoming>,
    storage_url: &str,
    batch_type: &str,
) -> Response<Full<Bytes>> {
    // Read request body
    let body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!("Import request body error: {}", e);
            return import_error_response(StatusCode::BAD_REQUEST, "Failed to read request body");
        }
    };

    // Parse to validate
    let import_req: ImportQueueRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            warn!("Import request JSON parse error: {}", e);
            return import_error_response(StatusCode::BAD_REQUEST, &format!("Invalid JSON: {}", e));
        }
    };

    info!(
        batch_type = batch_type,
        blob_hash = %import_req.blob_hash,
        total_items = import_req.total_items,
        chunk_size = ?import_req.chunk_size,
        chunk_delay_ms = ?import_req.chunk_delay_ms,
        "Forwarding import queue request to elohim-storage"
    );

    // Build storage request (add batch_type if not in original)
    let storage_req = serde_json::json!({
        "batch_id": import_req.batch_id,
        "batch_type": batch_type,
        "blob_hash": import_req.blob_hash,
        "total_items": import_req.total_items,
        "schema_version": import_req.schema_version,
        "chunk_size": import_req.chunk_size,
        "chunk_delay_ms": import_req.chunk_delay_ms,
    });

    // IMPORT_DEBUG: Log full request body
    if std::env::var("IMPORT_DEBUG").is_ok() {
        debug!(
            incoming_body = %String::from_utf8_lossy(&body).chars().take(2000).collect::<String>(),
            outgoing_body = %serde_json::to_string_pretty(&storage_req).unwrap_or_default(),
            "[IMPORT_DEBUG] doorway -> elohim-storage request"
        );
    }

    // Forward to elohim-storage
    let storage_endpoint = format!("{}/import/queue", storage_url.trim_end_matches('/'));

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return import_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to create HTTP client: {}", e),
            );
        }
    };

    match client
        .post(&storage_endpoint)
        .json(&storage_req)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(body) => {
                    info!(
                        status = %status,
                        "elohim-storage queue response"
                    );

                    // IMPORT_DEBUG: Log response body
                    if std::env::var("IMPORT_DEBUG").is_ok() {
                        debug!(
                            response_status = %status,
                            response_body = %body.chars().take(2000).collect::<String>(),
                            "[IMPORT_DEBUG] elohim-storage -> doorway response"
                        );
                    }

                    Response::builder()
                        .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                        .header("Content-Type", "application/json")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(Full::new(Bytes::from(body)))
                        .unwrap()
                }
                Err(e) => import_error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("Failed to read storage response: {}", e),
                ),
            }
        }
        Err(e) => {
            warn!(error = %e, "Failed to reach elohim-storage");
            import_error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to reach elohim-storage: {}", e),
            )
        }
    }
}

/// Forward GET status request to elohim-storage
async fn forward_get_status(storage_url: &str, batch_id: &str) -> Response<Full<Bytes>> {
    debug!(
        batch_id = batch_id,
        "Forwarding status request to elohim-storage"
    );

    let storage_endpoint = format!(
        "{}/import/status/{}",
        storage_url.trim_end_matches('/'),
        batch_id
    );

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return import_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to create HTTP client: {}", e),
            );
        }
    };

    match client.get(&storage_endpoint).send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(body) => Response::builder()
                    .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                    .header("Content-Type", "application/json")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(Full::new(Bytes::from(body)))
                    .unwrap(),
                Err(e) => import_error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("Failed to read storage response: {}", e),
                ),
            }
        }
        Err(e) => import_error_response(
            StatusCode::BAD_GATEWAY,
            &format!("Failed to reach elohim-storage: {}", e),
        ),
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Create error response
fn import_error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": message,
    });

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use doorway_client::ImportBatchType;
    use std::sync::Arc;

    fn make_batch_type(name: &str, queue_fn: &str, status_fn: &str) -> ImportBatchType {
        ImportBatchType {
            batch_type: name.to_string(),
            queue_fn: queue_fn.to_string(),
            process_fn: "process_import_chunk".to_string(),
            status_fn: status_fn.to_string(),
            max_items: 5000,
            chunk_size: 50,
            chunk_interval_ms: 100,
            schema_version: 1,
        }
    }

    fn setup_test_store() -> Arc<ImportConfigStore> {
        let store = ImportConfigStore::new();

        store.set_config(
            "test_dna",
            doorway_client::ImportConfig {
                enabled: true,
                base_route: "/import".to_string(),
                batch_types: vec![
                    make_batch_type("content", "queue_import", "get_import_status"),
                    make_batch_type("paths", "queue_path_import", "get_path_import_status"),
                ],
                require_auth: false,
                allowed_agents: None,
            },
        );

        Arc::new(store)
    }

    #[test]
    fn test_match_import_route_queue() {
        let store = setup_test_store();

        // POST /import/content -> queue
        let result = match_import_route("/import/content", &store);
        assert!(result.is_some());
        let (dna, batch_type, batch_id) = result.unwrap();
        assert_eq!(dna, "test_dna");
        assert_eq!(batch_type, "content");
        assert!(batch_id.is_none());
    }

    #[test]
    fn test_match_import_route_status() {
        let store = setup_test_store();

        // GET /import/content/batch-001 -> status
        let result = match_import_route("/import/content/batch-001", &store);
        assert!(result.is_some());
        let (dna, batch_type, batch_id) = result.unwrap();
        assert_eq!(dna, "test_dna");
        assert_eq!(batch_type, "content");
        assert_eq!(batch_id, Some("batch-001".to_string()));
    }

    #[test]
    fn test_match_import_route_unknown_type() {
        let store = setup_test_store();

        // Unknown batch type
        let result = match_import_route("/import/unknown", &store);
        assert!(result.is_none());
    }

    #[test]
    fn test_match_import_route_no_match() {
        let store = setup_test_store();

        // Completely different path
        let result = match_import_route("/api/v1/cache/content/test", &store);
        assert!(result.is_none());
    }
}
