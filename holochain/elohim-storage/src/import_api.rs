//! Import API - HTTP endpoints for batch import operations
//!
//! Provides REST endpoints for doorway to forward import requests:
//!
//! - `POST /import/queue` - Queue a new import batch
//! - `GET /import/status/{batch_id}` - Get import progress
//! - `GET /import/stream/{batch_id}` - SSE stream of progress updates
//!
//! ## Architecture
//!
//! ```text
//! Doorway → POST /import/queue → elohim-storage
//!                                     │
//!                                     ├── Store blob locally
//!                                     ├── Parse items
//!                                     ├── Queue chunks for processing
//!                                     │
//!                                     └── ConductorClient (single connection)
//!                                              │
//!                                              └── Batched process_import_chunk calls
//! ```
//!
//! ## Why Here Instead of Doorway?
//!
//! 1. **Single conductor connection** - Avoids connection pressure
//! 2. **Local blob storage** - Fast writes, no network hops
//! 3. **WriteBuffer batching** - Protects conductor from overwhelming
//! 4. **Scalable** - Multiple doorways can share one elohim-storage

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, Request, Response, StatusCode, header};
use hyper::body::Incoming;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

use crate::blob_store::BlobStore;
use crate::cell_discovery::discover_cell_id;
use crate::conductor::{ConductorClient, ConductorClientConfig};
use crate::debug_stream::DebugBroadcaster;
use crate::error::StorageError;
use crate::progress_hub::ProgressHub;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Queue import request from doorway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueImportRequest {
    /// Batch identifier (optional, generated if not provided)
    #[serde(default)]
    pub batch_id: Option<String>,
    /// Type of items: "content", "paths", "assessments"
    pub batch_type: String,
    /// Hash of blob containing items (if pre-uploaded)
    #[serde(default)]
    pub blob_hash: Option<String>,
    /// Total items count (required if blob_hash provided)
    #[serde(default)]
    pub total_items: Option<u32>,
    /// Schema version
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// Inline items JSON (alternative to blob_hash)
    #[serde(default)]
    pub items: Option<Vec<serde_json::Value>>,
}

fn default_schema_version() -> u32 { 1 }

/// Queue import response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueImportResponse {
    pub batch_id: String,
    pub blob_hash: String,
    pub total_items: u32,
    pub status: String,
    pub message: String,
}

/// Import status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatusResponse {
    pub batch_id: String,
    pub status: ImportStatus,
    pub total_items: u32,
    pub processed_count: u32,
    pub error_count: u32,
    pub errors: Vec<String>,
    pub elapsed_ms: u64,
    pub items_per_second: f64,
}

/// Import status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportStatus {
    Queued,
    Processing,
    Completed,
    CompletedWithErrors,
    Failed,
}

// ============================================================================
// Import Batch State
// ============================================================================

/// State for an active import batch
struct ImportBatch {
    batch_id: String,
    batch_type: String,
    blob_hash: String,
    total_items: u32,
    status: ImportStatus,
    processed_count: u32,
    error_count: u32,
    errors: Vec<String>,
    started_at: Instant,
    /// Progress broadcast channel
    progress_tx: broadcast::Sender<ImportStatusResponse>,
}

// ============================================================================
// Import API Service
// ============================================================================

/// Configuration for import API
#[derive(Debug, Clone)]
pub struct ImportApiConfig {
    /// Conductor admin interface URL (for auth)
    pub admin_url: String,
    /// Conductor app interface URL
    pub conductor_url: String,
    /// App ID for authentication
    pub app_id: String,
    /// Role filter for cell discovery (e.g., "lamad")
    pub role_filter: Option<String>,
    /// Chunk size for processing
    pub chunk_size: usize,
    /// Base delay between chunks (backpressure)
    pub chunk_delay: Duration,
    /// Maximum delay between chunks (backpressure ceiling)
    pub max_chunk_delay: Duration,
    /// Maximum concurrent batches
    pub max_concurrent_batches: usize,
    /// Consecutive errors before circuit breaker trips
    pub circuit_breaker_threshold: usize,
    /// Pause duration when circuit breaker trips
    pub circuit_breaker_pause: Duration,
    /// Cell ID for zome calls (discovered or configured)
    pub cell_id: Option<Vec<u8>>,
    /// Zome name
    pub zome_name: String,
}

impl Default for ImportApiConfig {
    fn default() -> Self {
        Self {
            admin_url: "ws://localhost:4444".to_string(),
            conductor_url: "ws://localhost:4445".to_string(),
            app_id: "elohim".to_string(),
            role_filter: Some("lamad".to_string()),
            chunk_size: 50,
            chunk_delay: Duration::from_millis(100),
            max_chunk_delay: Duration::from_secs(5),
            max_concurrent_batches: 3,
            circuit_breaker_threshold: 5,
            circuit_breaker_pause: Duration::from_secs(10),
            cell_id: None,
            zome_name: "content_store".to_string(),
        }
    }
}

/// Import API service
pub struct ImportApi {
    config: ImportApiConfig,
    blob_store: Arc<BlobStore>,
    conductor: Option<Arc<ConductorClient>>,
    /// Active batches by ID
    batches: Arc<RwLock<HashMap<String, ImportBatch>>>,
    /// Progress hub for WebSocket streaming
    progress_hub: Option<Arc<ProgressHub>>,
    /// Debug broadcaster for real-time debugging
    debug_broadcaster: Option<Arc<DebugBroadcaster>>,
    /// Lazily discovered cell_id cache - shared across all processing tasks
    cell_id_cache: Arc<RwLock<Option<Vec<u8>>>>,
}

impl ImportApi {
    /// Create a new import API service
    pub fn new(config: ImportApiConfig, blob_store: Arc<BlobStore>) -> Self {
        // Pre-populate cell_id cache if configured
        let cell_id_cache = Arc::new(RwLock::new(config.cell_id.clone()));

        Self {
            config,
            blob_store,
            conductor: None,
            batches: Arc::new(RwLock::new(HashMap::new())),
            progress_hub: None,
            debug_broadcaster: None,
            cell_id_cache,
        }
    }

    /// Set the progress hub for WebSocket streaming
    pub fn with_progress_hub(mut self, hub: Arc<ProgressHub>) -> Self {
        self.progress_hub = Some(hub);
        self
    }

    /// Set the debug broadcaster for real-time debugging
    pub fn with_debug_broadcaster(mut self, broadcaster: Arc<DebugBroadcaster>) -> Self {
        self.debug_broadcaster = Some(broadcaster);
        self
    }

    /// Initialize conductor connection with retry logic.
    ///
    /// Retries connection with exponential backoff because:
    /// - Conductor may still be starting up
    /// - App may not be installed yet (hApp installer runs async)
    /// - Network may have transient issues
    pub async fn connect_conductor(&mut self) -> Result<(), StorageError> {
        let conductor_config = ConductorClientConfig {
            admin_url: self.config.admin_url.clone(),
            app_url: self.config.conductor_url.clone(),
            app_id: self.config.app_id.clone(),
            request_timeout: Duration::from_secs(60),
            ..Default::default()
        };

        // Retry with backoff - conductor/app may not be ready at startup
        let max_attempts = 5;
        let mut attempt = 0;
        let mut delay = Duration::from_secs(2);

        loop {
            attempt += 1;
            info!(
                attempt = attempt,
                max_attempts = max_attempts,
                admin_url = %conductor_config.admin_url,
                app_url = %conductor_config.app_url,
                app_id = %conductor_config.app_id,
                "Attempting conductor connection"
            );

            match ConductorClient::connect(conductor_config.clone()).await {
                Ok(client) => {
                    self.conductor = Some(Arc::new(client));
                    info!("ImportApi conductor client connected");
                    return Ok(());
                }
                Err(e) => {
                    if attempt >= max_attempts {
                        error!(
                            attempt = attempt,
                            error = %e,
                            "Conductor connection failed after max attempts"
                        );
                        return Err(e);
                    }
                    warn!(
                        attempt = attempt,
                        error = %e,
                        delay_secs = delay.as_secs(),
                        "Conductor connection failed, retrying..."
                    );
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, Duration::from_secs(30));
                }
            }
        }
    }

    /// Handle import API requests
    pub async fn handle_request(
        &self,
        req: Request<Incoming>,
        path: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let method = req.method().clone();

        match (method, path) {
            // POST /import/queue - Queue new import
            (Method::POST, "/import/queue") => {
                self.handle_queue_import(req).await
            }

            // GET /import/status/{batch_id} - Get status
            (Method::GET, p) if p.starts_with("/import/status/") => {
                let batch_id = p.strip_prefix("/import/status/").unwrap_or("");
                self.handle_get_status(batch_id).await
            }

            // GET /import/batches - List all batches
            (Method::GET, "/import/batches") => {
                self.handle_list_batches().await
            }

            // Not found
            _ => Ok(error_response(
                StatusCode::NOT_FOUND,
                "Import endpoint not found",
            )),
        }
    }

    /// Handle POST /import/queue
    async fn handle_queue_import(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Parse request body
        let body = req.collect().await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let body_bytes = body.to_bytes();

        let request: QueueImportRequest = serde_json::from_slice(&body_bytes)?;

        // Generate batch ID if not provided
        let batch_id = request.batch_id.unwrap_or_else(|| {
            format!("import-{}", chrono::Utc::now().timestamp_millis())
        });

        // Check concurrent batch limit
        {
            let batches = self.batches.read().await;
            let active_count = batches.values()
                .filter(|b| matches!(b.status, ImportStatus::Queued | ImportStatus::Processing))
                .count();

            if active_count >= self.config.max_concurrent_batches {
                return Ok(error_response(
                    StatusCode::TOO_MANY_REQUESTS,
                    "Too many concurrent imports, try again later",
                ));
            }
        }

        // Get or store blob
        let (blob_hash, items_json, total_items) = if let Some(items) = request.items {
            // Inline items - store as blob
            let items_json = serde_json::to_string(&items)?;
            let total_items = items.len() as u32;
            let blob_result = self.blob_store.store(items_json.as_bytes()).await?;
            (blob_result.hash, items_json, total_items)
        } else if let Some(hash) = request.blob_hash {
            // Pre-uploaded blob - retrieve it
            let blob_data = self.blob_store.get(&hash).await?;
            let items_json = String::from_utf8(blob_data)
                .map_err(|e| StorageError::Parse(format!("Invalid UTF-8 in blob: {}", e)))?;
            let total_items = request.total_items.unwrap_or_else(|| {
                // Count items in JSON array
                serde_json::from_str::<Vec<serde_json::Value>>(&items_json)
                    .map(|v| v.len() as u32)
                    .unwrap_or(0)
            });
            (hash, items_json, total_items)
        } else {
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "Must provide either 'items' or 'blob_hash'",
            ));
        };

        // Create batch state
        let (progress_tx, _) = broadcast::channel(100);
        let batch = ImportBatch {
            batch_id: batch_id.clone(),
            batch_type: request.batch_type.clone(),
            blob_hash: blob_hash.clone(),
            total_items,
            status: ImportStatus::Queued,
            processed_count: 0,
            error_count: 0,
            errors: Vec::new(),
            started_at: Instant::now(),
            progress_tx,
        };

        // Store batch
        {
            let mut batches = self.batches.write().await;
            batches.insert(batch_id.clone(), batch);
        }

        // Register with progress hub for WebSocket streaming
        if let Some(ref hub) = self.progress_hub {
            hub.register_batch(&batch_id, &request.batch_type, total_items).await;
        }

        info!(
            batch_id = %batch_id,
            batch_type = %request.batch_type,
            total_items = total_items,
            "Import batch queued"
        );

        // Spawn processing task
        let api_self = self.clone_for_processing();
        let batch_id_clone = batch_id.clone();
        let batch_type = request.batch_type.clone();
        tokio::spawn(async move {
            if let Err(e) = api_self.process_batch(&batch_id_clone, &batch_type, &items_json).await {
                error!(batch_id = %batch_id_clone, error = %e, "Batch processing failed");
            }
        });

        // Return response
        let response = QueueImportResponse {
            batch_id,
            blob_hash,
            total_items,
            status: "queued".to_string(),
            message: "Import batch queued for processing".to_string(),
        };

        Ok(Response::builder()
            .status(StatusCode::ACCEPTED)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(serde_json::to_string(&response)?)))
            .unwrap())
    }

    /// Handle GET /import/status/{batch_id}
    async fn handle_get_status(
        &self,
        batch_id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let batches = self.batches.read().await;

        let batch = match batches.get(batch_id) {
            Some(b) => b,
            None => {
                return Ok(error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Batch '{}' not found", batch_id),
                ));
            }
        };

        let elapsed = batch.started_at.elapsed();
        let items_per_second = if elapsed.as_secs_f64() > 0.0 {
            batch.processed_count as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let response = ImportStatusResponse {
            batch_id: batch.batch_id.clone(),
            status: batch.status,
            total_items: batch.total_items,
            processed_count: batch.processed_count,
            error_count: batch.error_count,
            errors: batch.errors.clone(),
            elapsed_ms: elapsed.as_millis() as u64,
            items_per_second,
        };

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(serde_json::to_string(&response)?)))
            .unwrap())
    }

    /// Handle GET /import/batches - List all batches
    async fn handle_list_batches(
        &self,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let batches = self.batches.read().await;

        let batch_list: Vec<ImportStatusResponse> = batches.values()
            .map(|batch| {
                let elapsed = batch.started_at.elapsed();
                let items_per_second = if elapsed.as_secs_f64() > 0.0 {
                    batch.processed_count as f64 / elapsed.as_secs_f64()
                } else {
                    0.0
                };

                ImportStatusResponse {
                    batch_id: batch.batch_id.clone(),
                    status: batch.status,
                    total_items: batch.total_items,
                    processed_count: batch.processed_count,
                    error_count: batch.error_count,
                    errors: batch.errors.clone(),
                    elapsed_ms: elapsed.as_millis() as u64,
                    items_per_second,
                }
            })
            .collect();

        let response = serde_json::json!({
            "total": batch_list.len(),
            "batches": batch_list,
        });

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(serde_json::to_string(&response)?)))
            .unwrap())
    }

    /// Clone self for spawned processing task
    fn clone_for_processing(&self) -> ImportApiProcessor {
        ImportApiProcessor {
            config: self.config.clone(),
            blob_store: Arc::clone(&self.blob_store),
            conductor: self.conductor.clone(),
            batches: Arc::clone(&self.batches),
            progress_hub: self.progress_hub.clone(),
            debug_broadcaster: self.debug_broadcaster.clone(),
            cell_id_cache: Arc::clone(&self.cell_id_cache),
        }
    }
}

/// Separate struct for processing to avoid lifetime issues
struct ImportApiProcessor {
    config: ImportApiConfig,
    blob_store: Arc<BlobStore>,
    conductor: Option<Arc<ConductorClient>>,
    batches: Arc<RwLock<HashMap<String, ImportBatch>>>,
    progress_hub: Option<Arc<ProgressHub>>,
    debug_broadcaster: Option<Arc<DebugBroadcaster>>,
    cell_id_cache: Arc<RwLock<Option<Vec<u8>>>>,
}

impl ImportApiProcessor {
    /// Ensure cell_id is discovered - lazy discovery on first use
    ///
    /// This is the KEY fix for the silent failure bug. Previously, if cell_id
    /// wasn't discovered at startup (race with hApp installer), imports would
    /// silently skip. Now we discover lazily and fail loudly if we can't.
    async fn ensure_cell_id(&self, batch_id: &str) -> Result<Vec<u8>, StorageError> {
        // Check cache first (fast path)
        {
            let cache = self.cell_id_cache.read().await;
            if let Some(ref cell_id) = *cache {
                return Ok(cell_id.clone());
            }
        }

        // Not cached - need to discover
        info!(
            batch_id = %batch_id,
            admin_url = %self.config.admin_url,
            app_id = %self.config.app_id,
            role = ?self.config.role_filter,
            "🔍 CELL_DISCOVERY: cell_id not cached, discovering lazily..."
        );

        // Emit debug event
        if let Some(ref broadcaster) = self.debug_broadcaster {
            broadcaster.cell_discovery_start(&self.config.admin_url, &self.config.app_id);
        }

        // Attempt discovery
        match discover_cell_id(
            &self.config.admin_url,
            &self.config.app_id,
            self.config.role_filter.as_deref(),
        ).await {
            Ok(cell_id) => {
                // Cache it for future use
                {
                    let mut cache = self.cell_id_cache.write().await;
                    *cache = Some(cell_id.clone());
                }

                info!(
                    batch_id = %batch_id,
                    cell_id_len = cell_id.len(),
                    "✅ CELL_DISCOVERY: cell_id discovered and cached"
                );

                if let Some(ref broadcaster) = self.debug_broadcaster {
                    broadcaster.cell_discovery_success(
                        self.config.role_filter.as_deref().unwrap_or("default")
                    );
                }

                Ok(cell_id)
            }
            Err(e) => {
                error!(
                    batch_id = %batch_id,
                    error = %e,
                    "❌ CELL_DISCOVERY: Failed to discover cell_id"
                );

                if let Some(ref broadcaster) = self.debug_broadcaster {
                    broadcaster.cell_discovery_failed(&e.to_string());
                }

                Err(e)
            }
        }
    }

    /// Process a batch of items
    async fn process_batch(
        &self,
        batch_id: &str,
        batch_type: &str,
        items_json: &str,
    ) -> Result<(), StorageError> {
        let batch_start = Instant::now();

        // Update status to processing
        self.update_status(batch_id, ImportStatus::Processing).await;

        // Emit debug event for batch start
        if let Some(ref broadcaster) = self.debug_broadcaster {
            let items_count: usize = serde_json::from_str::<Vec<serde_json::Value>>(items_json)
                .map(|v| v.len())
                .unwrap_or(0);
            broadcaster.import_batch_start(batch_id, batch_type, items_count);
        }

        // CRITICAL: Ensure we have a cell_id BEFORE processing
        // This is the lazy discovery - fail loudly if we can't discover
        let cell_id = match self.ensure_cell_id(batch_id).await {
            Ok(id) => id,
            Err(e) => {
                let error_msg = format!("Cell discovery failed: {}", e);
                error!(batch_id = %batch_id, error = %error_msg, "❌ BATCH_FAILED: Cannot proceed without cell_id");
                self.add_error(batch_id, error_msg.clone()).await;
                self.update_status(batch_id, ImportStatus::Failed).await;

                if let Some(ref broadcaster) = self.debug_broadcaster {
                    broadcaster.import_batch_complete(batch_id, 0, 1, batch_start.elapsed().as_millis() as u64);
                }

                return Err(StorageError::NotFound(error_msg));
            }
        };

        // Check conductor is connected
        let conductor = match &self.conductor {
            Some(c) => c,
            None => {
                let error_msg = "No conductor connection available";
                error!(batch_id = %batch_id, "❌ BATCH_FAILED: {}", error_msg);
                self.add_error(batch_id, error_msg.to_string()).await;
                self.update_status(batch_id, ImportStatus::Failed).await;

                if let Some(ref broadcaster) = self.debug_broadcaster {
                    broadcaster.import_batch_complete(batch_id, 0, 1, batch_start.elapsed().as_millis() as u64);
                }

                return Err(StorageError::Connection(error_msg.to_string()));
            }
        };

        // Parse items
        let items: Vec<serde_json::Value> = serde_json::from_str(items_json)?;
        let total = items.len();
        let total_chunks = (total + self.config.chunk_size - 1) / self.config.chunk_size;

        info!(
            batch_id = %batch_id,
            batch_type = %batch_type,
            total_items = total,
            total_chunks = total_chunks,
            chunk_size = self.config.chunk_size,
            conductor_url = %self.config.conductor_url,
            "📥 BATCH_START: Starting batch processing via ImportApi (cell_id ready)"
        );

        // CRITICAL: Call queue_import FIRST to create the batch entry in the zome
        // The zome's process_import_chunk looks up the batch by ID, so it MUST exist first
        let queue_payload = serde_json::json!({
            "id": batch_id,
            "batch_type": batch_type,
            "blob_hash": format!("inline-{}", batch_id), // No blob for inline imports
            "total_items": total as u32,
        });
        let queue_payload_bytes = rmp_serde::to_vec(&queue_payload)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        info!(
            batch_id = %batch_id,
            batch_type = %batch_type,
            total_items = total,
            "📝 QUEUE_IMPORT: Creating batch entry in zome"
        );

        match conductor.call_zome(
            &cell_id,
            &self.config.zome_name,
            "queue_import",
            &queue_payload_bytes,
        ).await {
            Ok(response) => {
                info!(
                    batch_id = %batch_id,
                    response_len = response.len(),
                    "✅ QUEUE_IMPORT_OK: Batch entry created in zome"
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to create batch entry in zome: {}", e);
                error!(batch_id = %batch_id, error = %error_msg, "❌ QUEUE_IMPORT_FAILED");
                self.add_error(batch_id, error_msg.clone()).await;
                self.update_status(batch_id, ImportStatus::Failed).await;

                if let Some(ref broadcaster) = self.debug_broadcaster {
                    broadcaster.import_batch_complete(batch_id, 0, 1, batch_start.elapsed().as_millis() as u64);
                }

                return Err(StorageError::Conductor(error_msg));
            }
        }

        // Process in chunks with adaptive backpressure
        let mut processed = 0;
        let mut errors = 0;
        let mut consecutive_errors = 0;
        let mut current_delay = self.config.chunk_delay;
        let mut avg_response_time_ms: f64 = 0.0;

        for (chunk_idx, chunk) in items.chunks(self.config.chunk_size).enumerate() {
            let chunk_start = Instant::now();
            let is_final = processed + chunk.len() >= total;

            // Circuit breaker check - pause if too many consecutive errors
            if consecutive_errors >= self.config.circuit_breaker_threshold {
                warn!(
                    batch_id = %batch_id,
                    consecutive_errors = consecutive_errors,
                    pause_seconds = self.config.circuit_breaker_pause.as_secs(),
                    "⚡ CIRCUIT_BREAKER: Pausing due to consecutive errors"
                );
                tokio::time::sleep(self.config.circuit_breaker_pause).await;
                consecutive_errors = 0; // Reset after pause
                current_delay = self.config.chunk_delay; // Reset delay
            }

            info!(
                batch_id = %batch_id,
                chunk_index = chunk_idx,
                chunk_items = chunk.len(),
                total_chunks = total_chunks,
                is_final = is_final,
                current_delay_ms = current_delay.as_millis(),
                "🔄 CHUNK_START: Sending chunk to conductor"
            );

            if let Some(ref broadcaster) = self.debug_broadcaster {
                broadcaster.import_chunk_start(batch_id, chunk_idx, chunk.len());
            }

            // Build payload
            // Serialize items to JSON string - zome expects items_json: String
            let items_json = serde_json::to_string(&chunk)
                .map_err(|e| StorageError::Internal(format!("Failed to serialize items: {}", e)))?;

            let payload = serde_json::json!({
                "batch_id": batch_id,
                "chunk_index": chunk_idx,
                "is_final": is_final,
                "items_json": items_json,
            });
            let payload_bytes = rmp_serde::to_vec(&payload)
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            match conductor.call_zome(
                &cell_id,
                &self.config.zome_name,
                "process_import_chunk",
                &payload_bytes,
            ).await {
                Ok(_) => {
                    let chunk_duration = chunk_start.elapsed();
                    let response_ms = chunk_duration.as_millis() as f64;
                    processed += chunk.len();
                    consecutive_errors = 0; // Reset on success

                    // Update running average of response times
                    if avg_response_time_ms == 0.0 {
                        avg_response_time_ms = response_ms;
                    } else {
                        avg_response_time_ms = (avg_response_time_ms * 0.8) + (response_ms * 0.2);
                    }

                    // Adaptive backpressure: if responses are fast, reduce delay
                    // Target: delay should be ~50% of average response time
                    let target_delay_ms = (avg_response_time_ms * 0.5).max(self.config.chunk_delay.as_millis() as f64);
                    current_delay = Duration::from_millis(target_delay_ms as u64).min(self.config.max_chunk_delay);

                    info!(
                        batch_id = %batch_id,
                        chunk_index = chunk_idx,
                        chunk_items = chunk.len(),
                        duration_ms = chunk_duration.as_millis(),
                        total_processed = processed,
                        avg_response_ms = format!("{:.1}", avg_response_time_ms),
                        next_delay_ms = current_delay.as_millis(),
                        "✅ CHUNK_OK: Chunk sent to conductor successfully"
                    );

                    if let Some(ref broadcaster) = self.debug_broadcaster {
                        broadcaster.import_chunk_success(batch_id, chunk_idx, chunk_duration.as_millis() as u64);
                    }
                }
                Err(e) => {
                    let chunk_duration = chunk_start.elapsed();
                    consecutive_errors += 1;

                    // Exponential backoff on errors (double delay, up to max)
                    current_delay = (current_delay * 2).min(self.config.max_chunk_delay);

                    error!(
                        batch_id = %batch_id,
                        chunk_index = chunk_idx,
                        chunk_items = chunk.len(),
                        duration_ms = chunk_duration.as_millis(),
                        error = %e,
                        consecutive_errors = consecutive_errors,
                        next_delay_ms = current_delay.as_millis(),
                        "❌ CHUNK_ERROR: Conductor call failed, applying backoff"
                    );
                    errors += chunk.len();
                    self.add_error(batch_id, format!("Chunk {}: {}", chunk_idx, e)).await;

                    if let Some(ref broadcaster) = self.debug_broadcaster {
                        broadcaster.import_chunk_error(batch_id, chunk_idx, &e.to_string());
                    }
                }
            }

            // Update progress
            self.update_progress(batch_id, processed as u32, errors as u32).await;

            // Adaptive backpressure delay
            if !is_final {
                tokio::time::sleep(current_delay).await;
            }
        }

        // Final status
        let final_status = if errors == 0 {
            ImportStatus::Completed
        } else if processed > 0 {
            ImportStatus::CompletedWithErrors
        } else {
            ImportStatus::Failed
        };

        self.update_status(batch_id, final_status).await;

        let batch_duration = batch_start.elapsed();
        let items_per_sec = if batch_duration.as_secs_f64() > 0.0 {
            processed as f64 / batch_duration.as_secs_f64()
        } else {
            0.0
        };

        info!(
            batch_id = %batch_id,
            batch_type = %batch_type,
            processed = processed,
            errors = errors,
            total_items = total,
            duration_ms = batch_duration.as_millis(),
            items_per_sec = format!("{:.1}", items_per_sec),
            status = ?final_status,
            "📦 BATCH_COMPLETE: Batch processing finished"
        );

        if let Some(ref broadcaster) = self.debug_broadcaster {
            broadcaster.import_batch_complete(
                batch_id,
                processed,
                errors,
                batch_duration.as_millis() as u64,
            );
        }

        Ok(())
    }

    async fn update_status(&self, batch_id: &str, status: ImportStatus) {
        let mut batches = self.batches.write().await;
        if let Some(batch) = batches.get_mut(batch_id) {
            batch.status = status;
        }
    }

    async fn update_progress(&self, batch_id: &str, processed: u32, errors: u32) {
        let response = {
            let mut batches = self.batches.write().await;
            if let Some(batch) = batches.get_mut(batch_id) {
                batch.processed_count = processed;
                batch.error_count = errors;

                // Broadcast progress to per-batch channel
                let elapsed = batch.started_at.elapsed();
                let response = ImportStatusResponse {
                    batch_id: batch.batch_id.clone(),
                    status: batch.status,
                    total_items: batch.total_items,
                    processed_count: processed,
                    error_count: errors,
                    errors: batch.errors.clone(),
                    elapsed_ms: elapsed.as_millis() as u64,
                    items_per_second: processed as f64 / elapsed.as_secs_f64().max(0.001),
                };
                let _ = batch.progress_tx.send(response.clone());
                Some(response)
            } else {
                None
            }
        };

        // Also broadcast to global progress hub
        if let (Some(response), Some(ref hub)) = (response, &self.progress_hub) {
            hub.update_progress(&response).await;
        }
    }

    async fn add_error(&self, batch_id: &str, error: String) {
        let mut batches = self.batches.write().await;
        if let Some(batch) = batches.get_mut(batch_id) {
            if batch.errors.len() < 100 {
                batch.errors.push(error);
            }
        }
    }
}

/// Create an error response
fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": message,
    });

    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}
