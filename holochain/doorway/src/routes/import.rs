//! Import Route Handler - Dynamic routing for zome-declared import endpoints
//!
//! Routes import requests to zome functions based on discovered ImportConfig.
//!
//! ## Discovery Pattern
//!
//! 1. Doorway discovers import config from zome's `__doorway_import_config`
//! 2. Zome declares base_route (e.g., "/import") and batch_types
//! 3. Doorway dynamically routes:
//!    - POST /{base_route}/{batch_type} → queue_fn
//!    - GET /{base_route}/{batch_type}/{batch_id} → status_fn
//!
//! ## Example Flow
//!
//! ```text
//! Seeder: POST /import/content {"items": [...]}
//!    → Doorway matches route to discovered ImportConfig
//!    → Doorway calls zome's queue_import function
//!    → Returns batch_id to seeder
//!
//! Seeder: GET /import/content/batch-001
//!    → Doorway calls zome's get_import_status function
//!    → Returns batch status
//! ```

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::services::ImportConfigStore;
use crate::worker::{WorkerPool, ZomeCallConfig};
use crate::types::Result;

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
}

fn default_schema_version() -> u32 { 1 }

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
/// Doorway acts as a relay:
/// - POST /{base_route}/{batch_type} → queue_import on zome
/// - GET /{base_route}/{batch_type}/{batch_id} → get_import_status on zome
///
/// Chunk processing happens in elohim-storage, which listens for
/// ImportBatchQueued signals and calls process_import_chunk.
pub async fn handle_import_request(
    req: Request<Incoming>,
    import_store: Arc<ImportConfigStore>,
    worker_pool: Arc<WorkerPool>,
    zome_config: ZomeCallConfig,
    dna_hash: String,
    batch_type: String,
    batch_id: Option<String>,
) -> Response<Full<Bytes>> {
    let method = req.method().clone();

    match method {
        Method::POST if batch_id.is_none() => {
            // POST /{base_route}/{batch_type} - Queue new import
            handle_queue_import(req, import_store, worker_pool, zome_config, &dna_hash, &batch_type).await
        }
        Method::GET if batch_id.is_some() => {
            // GET /{base_route}/{batch_type}/{batch_id} - Get status
            handle_get_status(import_store, worker_pool, zome_config, &dna_hash, &batch_type, batch_id.as_ref().unwrap()).await
        }
        _ => {
            import_error_response(
                StatusCode::METHOD_NOT_ALLOWED,
                "Use POST to queue imports, GET with batch_id to check status",
            )
        }
    }
}

/// Handle POST - queue new import (relay to zome)
///
/// Doorway just relays the queue_import call to the zome.
/// elohim-storage listens for ImportBatchQueued signal and processes chunks.
async fn handle_queue_import(
    req: Request<Incoming>,
    import_store: Arc<ImportConfigStore>,
    worker_pool: Arc<WorkerPool>,
    zome_config: ZomeCallConfig,
    dna_hash: &str,
    batch_type: &str,
) -> Response<Full<Bytes>> {
    // Get batch type config
    let batch_config = match import_store.get_batch_type(dna_hash, batch_type) {
        Some(c) => c,
        None => {
            return import_error_response(
                StatusCode::BAD_REQUEST,
                &format!("Batch type '{}' not supported for this DNA", batch_type),
            );
        }
    };

    // Read request body
    let body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!("Import request body error: {}", e);
            return import_error_response(
                StatusCode::BAD_REQUEST,
                "Failed to read request body",
            );
        }
    };

    // Parse request
    let import_req: ImportQueueRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            warn!("Import request JSON parse error: {}", e);
            return import_error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid JSON: {}", e),
            );
        }
    };

    info!(
        batch_type = batch_type,
        blob_hash = %import_req.blob_hash,
        total_items = import_req.total_items,
        queue_fn = batch_config.queue_fn,
        "Relaying import queue request to zome"
    );

    // Generate batch ID if not provided
    let batch_id = import_req.batch_id.unwrap_or_else(|| {
        format!("import-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f"))
    });

    // Build zome call for queue_fn
    // The zome expects "id" (not "batch_id") to match QueueImportInput struct
    let payload = serde_json::json!({
        "id": batch_id,
        "batch_type": batch_type,
        "blob_hash": import_req.blob_hash,
        "total_items": import_req.total_items,
        "schema_version": import_req.schema_version,
    });

    // Build the zome call
    let call_payload = match build_zome_call(&zome_config, &batch_config.queue_fn, payload) {
        Ok(p) => p,
        Err(e) => {
            return import_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to build zome call: {}", e),
            );
        }
    };

    // Relay to conductor
    match worker_pool.request(call_payload).await {
        Ok(response) => {
            // Parse response
            match parse_zome_response::<ImportQueueResponse>(&response) {
                Ok(result) => {
                    info!(
                        batch_id = %result.batch_id,
                        queued_count = result.queued_count,
                        "Import queued - elohim-storage will process chunks"
                    );
                    import_json_response(StatusCode::ACCEPTED, &result)
                }
                Err(e) => {
                    warn!(error = ?e, "Failed to parse queue response");
                    import_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        &format!("Failed to parse conductor response: {}", e),
                    )
                }
            }
        }
        Err(e) => {
            warn!(error = ?e, "Conductor call failed");
            import_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("Conductor unavailable: {}", e),
            )
        }
    }
}

/// Handle GET - get import status
async fn handle_get_status(
    import_store: Arc<ImportConfigStore>,
    worker_pool: Arc<WorkerPool>,
    zome_config: ZomeCallConfig,
    dna_hash: &str,
    batch_type: &str,
    batch_id: &str,
) -> Response<Full<Bytes>> {
    // Get batch type config
    let batch_config = match import_store.get_batch_type(dna_hash, batch_type) {
        Some(c) => c,
        None => {
            return import_error_response(
                StatusCode::BAD_REQUEST,
                &format!("Batch type '{}' not supported", batch_type),
            );
        }
    };

    debug!(
        batch_id = batch_id,
        status_fn = batch_config.status_fn,
        "Getting import status"
    );

    // Build zome call for status_fn
    let payload = serde_json::json!({
        "batch_id": batch_id,
    });

    let call_payload = match build_zome_call(&zome_config, &batch_config.status_fn, payload) {
        Ok(p) => p,
        Err(e) => {
            return import_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to build zome call: {}", e),
            );
        }
    };

    // Send to conductor
    match worker_pool.request(call_payload).await {
        Ok(response) => {
            match parse_zome_response::<ImportStatusResponse>(&response) {
                Ok(result) => import_json_response(StatusCode::OK, &result),
                Err(e) => {
                    // Could be not found
                    import_error_response(
                        StatusCode::NOT_FOUND,
                        &format!("Batch not found or invalid response: {}", e),
                    )
                }
            }
        }
        Err(e) => {
            import_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("Conductor unavailable: {}", e),
            )
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Build a zome call payload
fn build_zome_call(
    config: &ZomeCallConfig,
    fn_name: &str,
    payload: serde_json::Value,
) -> Result<Vec<u8>> {
    use crate::worker::ZomeCallBuilder;

    let builder = ZomeCallBuilder::new(config.clone());
    builder.build_zome_call(fn_name, &payload)
}

/// Parse zome response
fn parse_zome_response<T: serde::de::DeserializeOwned>(response: &[u8]) -> Result<T> {
    // Response is typically msgpack-encoded
    // For simplicity, try JSON first then msgpack
    if let Ok(result) = serde_json::from_slice::<T>(response) {
        return Ok(result);
    }

    // Try msgpack
    rmp_serde::from_slice::<T>(response)
        .map_err(|e| crate::types::DoorwayError::Internal(format!("Parse error: {}", e)))
}

/// Create JSON response
fn import_json_response<T: Serialize>(status: StatusCode, data: &T) -> Response<Full<Bytes>> {
    let body = serde_json::to_string(data).unwrap_or_else(|_| r#"{"error":"Serialization failed"}"#.to_string());

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

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
    use doorway_client::ImportBatchTypeBuilder;

    fn setup_test_store() -> Arc<ImportConfigStore> {
        let store = ImportConfigStore::new();

        store.set_config("test_dna", doorway_client::ImportConfig {
            enabled: true,
            base_route: "/import".to_string(),
            batch_types: vec![
                ImportBatchTypeBuilder::new("content")
                    .queue_fn("queue_import")
                    .status_fn("get_import_status")
                    .build(),
                ImportBatchTypeBuilder::new("paths")
                    .queue_fn("queue_path_import")
                    .status_fn("get_path_import_status")
                    .build(),
            ],
            require_auth: false,
            allowed_agents: None,
        });

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
