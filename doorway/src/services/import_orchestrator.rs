//! Import Orchestrator - Batch Import Processing for Elohim-Store
//!
//! Provides web 2.0 performance for Holochain batch imports.
//!
//! ## Architecture
//!
//! ```text
//! Client → Doorway → Elohim-Store (this module) → Zome
//!                         │                          │
//!                    (blob write)              (queue_import)
//!                         │                          │
//!                         │    ┌─────────────────────┘
//!                         │    │ ImportBatchQueued signal
//!                         │    ▼
//!                         │ process_import_chunk() calls
//!                         │    │
//!                         │    │ ImportBatchProgress signals
//!                         │<───┘
//!                         │
//!                    relay to client via SSE
//! ```
//!
//! ## Key Insight
//!
//! Elohim-Store gives Holochain web 2.0 performance.
//! Doorway just extends that to HTTP clients.
//!
//! The zome/DHT handles validation, provenance, sensemaking, meaning,
//! reach, privacy, and final storage decisions (local/sharded/replicated).

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info};

// ============================================================================
// Types
// ============================================================================

/// Configuration for the import orchestrator
#[derive(Debug, Clone)]
pub struct ImportOrchestratorConfig {
    /// Chunk size for processing (items per chunk)
    pub chunk_size: usize,
    /// Maximum concurrent batches
    pub max_concurrent_batches: usize,
    /// Timeout for zome calls
    pub zome_call_timeout: Duration,
    /// Interval between chunks (to avoid overwhelming conductor)
    pub chunk_interval: Duration,
}

impl Default for ImportOrchestratorConfig {
    fn default() -> Self {
        Self {
            chunk_size: 50,
            max_concurrent_batches: 3,
            zome_call_timeout: Duration::from_secs(60),
            chunk_interval: Duration::from_millis(100),
        }
    }
}

/// Import batch status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ImportStatus {
    /// Blob received, queuing with zome
    Queuing,
    /// Queued with zome, processing chunks
    Processing,
    /// All chunks processed successfully
    Completed,
    /// Processing completed with some errors
    CompletedWithErrors,
    /// Critical failure, processing halted
    Failed,
}

/// Progress update for an import batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub batch_id: String,
    pub status: ImportStatus,
    pub total_items: u32,
    pub processed_count: u32,
    pub error_count: u32,
    pub errors: Vec<String>,
    pub blob_hash: String,
}

/// Input for starting an import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartImportInput {
    /// Unique batch identifier
    pub batch_id: String,
    /// Type of items: "content", "paths", "steps"
    pub batch_type: String,
    /// JSON array of items to import
    pub items_json: String,
}

/// Output from starting an import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartImportOutput {
    pub batch_id: String,
    pub blob_hash: String,
    pub total_items: u32,
    pub status: ImportStatus,
}

/// Error types for import operations
#[derive(Debug, Clone)]
pub enum ImportError {
    /// Failed to parse items JSON
    ParseError(String),
    /// Failed to write blob
    BlobWriteError(String),
    /// Failed to call zome
    ZomeCallError(String),
    /// Batch not found
    BatchNotFound(String),
    /// Maximum concurrent batches exceeded
    TooManyBatches,
    /// Generic internal error
    InternalError(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ImportError::BlobWriteError(msg) => write!(f, "Blob write error: {}", msg),
            ImportError::ZomeCallError(msg) => write!(f, "Zome call error: {}", msg),
            ImportError::BatchNotFound(id) => write!(f, "Batch not found: {}", id),
            ImportError::TooManyBatches => write!(f, "Too many concurrent batches"),
            ImportError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ImportError {}

// ============================================================================
// Zome Client Trait (for dependency injection)
// ============================================================================

/// Trait for calling zome functions (allows mocking in tests)
#[async_trait::async_trait]
pub trait ZomeClient: Send + Sync {
    /// Call queue_import on the zome
    async fn queue_import(
        &self,
        batch_id: &str,
        batch_type: &str,
        blob_hash: &str,
        total_items: u32,
        schema_version: u32,
    ) -> Result<(), ImportError>;

    /// Call process_import_chunk on the zome
    async fn process_import_chunk(
        &self,
        batch_id: &str,
        chunk_index: u32,
        is_final: bool,
        items_json: &str,
    ) -> Result<ChunkResult, ImportError>;
}

/// Result from processing a chunk
#[derive(Debug, Clone)]
pub struct ChunkResult {
    pub chunk_processed: u32,
    pub chunk_errors: u32,
    pub total_processed: u32,
    pub total_errors: u32,
    pub status: String,
}

// ============================================================================
// Blob Store Trait (for dependency injection)
// ============================================================================

/// Trait for blob storage (allows different backends)
#[async_trait::async_trait]
pub trait BlobStore: Send + Sync {
    /// Write a blob and return its hash
    async fn write_blob(&self, data: &[u8]) -> Result<String, ImportError>;

    /// Read a blob by hash
    async fn read_blob(&self, hash: &str) -> Result<Bytes, ImportError>;

    /// Check if blob exists
    async fn blob_exists(&self, hash: &str) -> bool;
}

// ============================================================================
// Import Orchestrator
// ============================================================================

/// State for an active import batch
struct ActiveBatch {
    batch_id: String,
    _batch_type: String,
    blob_hash: String,
    total_items: u32,
    _items_json: String,
    status: ImportStatus,
    processed_count: u32,
    error_count: u32,
    errors: Vec<String>,
}

/// Import Orchestrator Service
///
/// Coordinates batch imports through the elohim-store → zome pipeline.
/// Provides web 2.0 performance for Holochain by:
/// 1. Fast blob writes (no DHT overhead)
/// 2. Chunked processing at conductor's pace
/// 3. Progress signals for real-time updates
pub struct ImportOrchestrator<Z: ZomeClient, B: BlobStore> {
    config: ImportOrchestratorConfig,
    zome_client: Arc<Z>,
    blob_store: Arc<B>,
    /// Active batches being processed
    active_batches: Arc<RwLock<HashMap<String, ActiveBatch>>>,
    /// Progress broadcast channel
    progress_tx: broadcast::Sender<ImportProgress>,
}

impl<Z: ZomeClient + 'static, B: BlobStore + 'static> ImportOrchestrator<Z, B> {
    /// Create a new import orchestrator
    pub fn new(config: ImportOrchestratorConfig, zome_client: Arc<Z>, blob_store: Arc<B>) -> Self {
        let (progress_tx, _) = broadcast::channel(100);
        Self {
            config,
            zome_client,
            blob_store,
            active_batches: Arc::new(RwLock::new(HashMap::new())),
            progress_tx,
        }
    }

    /// Subscribe to progress updates
    pub fn subscribe_progress(&self) -> broadcast::Receiver<ImportProgress> {
        self.progress_tx.subscribe()
    }

    /// Start a batch import
    ///
    /// 1. Writes items to blob store (fast)
    /// 2. Calls queue_import on zome (stores manifest)
    /// 3. Spawns background task to process chunks
    pub async fn start_import(
        &self,
        input: StartImportInput,
    ) -> Result<StartImportOutput, ImportError> {
        // Check concurrent batch limit
        let active_count = self.active_batches.read().await.len();
        if active_count >= self.config.max_concurrent_batches {
            return Err(ImportError::TooManyBatches);
        }

        // Parse items to get count
        let items: Vec<serde_json::Value> = serde_json::from_str(&input.items_json)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse items: {}", e)))?;

        let total_items = items.len() as u32;
        if total_items == 0 {
            return Err(ImportError::ParseError("Empty items array".to_string()));
        }

        info!(
            batch_id = %input.batch_id,
            batch_type = %input.batch_type,
            total_items = total_items,
            "Starting import batch"
        );

        // 1. Write blob to store (blazing fast)
        let blob_hash = self
            .blob_store
            .write_blob(input.items_json.as_bytes())
            .await?;
        debug!(batch_id = %input.batch_id, blob_hash = %blob_hash, "Blob written");

        // 2. Create active batch record
        let batch = ActiveBatch {
            batch_id: input.batch_id.clone(),
            _batch_type: input.batch_type.clone(),
            blob_hash: blob_hash.clone(),
            total_items,
            _items_json: input.items_json.clone(),
            status: ImportStatus::Queuing,
            processed_count: 0,
            error_count: 0,
            errors: Vec::new(),
        };

        self.active_batches
            .write()
            .await
            .insert(input.batch_id.clone(), batch);

        // 3. Queue with zome (stores manifest, no payload)
        self.zome_client
            .queue_import(
                &input.batch_id,
                &input.batch_type,
                &blob_hash,
                total_items,
                1, // schema_version
            )
            .await?;

        // Update status
        if let Some(batch) = self.active_batches.write().await.get_mut(&input.batch_id) {
            batch.status = ImportStatus::Processing;
        }

        // Emit initial progress
        let _ = self.progress_tx.send(ImportProgress {
            batch_id: input.batch_id.clone(),
            status: ImportStatus::Processing,
            total_items,
            processed_count: 0,
            error_count: 0,
            errors: Vec::new(),
            blob_hash: blob_hash.clone(),
        });

        // 4. Spawn background task to process chunks
        let orchestrator = ImportOrchestratorHandle {
            config: self.config.clone(),
            zome_client: Arc::clone(&self.zome_client),
            active_batches: Arc::clone(&self.active_batches),
            progress_tx: self.progress_tx.clone(),
        };

        let batch_id = input.batch_id.clone();
        let items_json = input.items_json.clone();

        tokio::spawn(async move {
            if let Err(e) = orchestrator.process_batch(&batch_id, &items_json).await {
                error!(batch_id = %batch_id, error = %e, "Batch processing failed");
            }
        });

        Ok(StartImportOutput {
            batch_id: input.batch_id,
            blob_hash,
            total_items,
            status: ImportStatus::Processing,
        })
    }

    /// Get current status of a batch
    pub async fn get_status(&self, batch_id: &str) -> Option<ImportProgress> {
        let batches = self.active_batches.read().await;
        batches.get(batch_id).map(|b| ImportProgress {
            batch_id: b.batch_id.clone(),
            status: b.status.clone(),
            total_items: b.total_items,
            processed_count: b.processed_count,
            error_count: b.error_count,
            errors: b.errors.clone(),
            blob_hash: b.blob_hash.clone(),
        })
    }

    /// List all active batches
    pub async fn list_active(&self) -> Vec<ImportProgress> {
        let batches = self.active_batches.read().await;
        batches
            .values()
            .map(|b| ImportProgress {
                batch_id: b.batch_id.clone(),
                status: b.status.clone(),
                total_items: b.total_items,
                processed_count: b.processed_count,
                error_count: b.error_count,
                errors: b.errors.clone(),
                blob_hash: b.blob_hash.clone(),
            })
            .collect()
    }
}

/// Handle for background processing (avoids lifetime issues)
struct ImportOrchestratorHandle<Z: ZomeClient> {
    config: ImportOrchestratorConfig,
    zome_client: Arc<Z>,
    active_batches: Arc<RwLock<HashMap<String, ActiveBatch>>>,
    progress_tx: broadcast::Sender<ImportProgress>,
}

impl<Z: ZomeClient + 'static> ImportOrchestratorHandle<Z> {
    /// Process a batch in chunks
    async fn process_batch(&self, batch_id: &str, items_json: &str) -> Result<(), ImportError> {
        // Parse items
        let items: Vec<serde_json::Value> =
            serde_json::from_str(items_json).map_err(|e| ImportError::ParseError(e.to_string()))?;

        let total_items = items.len();
        let chunk_size = self.config.chunk_size;
        let total_chunks = total_items.div_ceil(chunk_size);

        info!(
            batch_id = %batch_id,
            total_items = total_items,
            chunk_size = chunk_size,
            total_chunks = total_chunks,
            "Processing batch in chunks"
        );

        for (chunk_index, chunk) in items.chunks(chunk_size).enumerate() {
            let is_final = chunk_index == total_chunks - 1;
            let chunk_json = serde_json::to_string(chunk)
                .map_err(|e| ImportError::InternalError(e.to_string()))?;

            debug!(
                batch_id = %batch_id,
                chunk_index = chunk_index,
                chunk_size = chunk.len(),
                is_final = is_final,
                "Processing chunk"
            );

            // Call zome to process chunk
            let result = self
                .zome_client
                .process_import_chunk(batch_id, chunk_index as u32, is_final, &chunk_json)
                .await?;

            // Update batch state
            {
                let mut batches = self.active_batches.write().await;
                if let Some(batch) = batches.get_mut(batch_id) {
                    batch.processed_count = result.total_processed;
                    batch.error_count = result.total_errors;

                    if is_final {
                        batch.status = if result.total_errors == 0 {
                            ImportStatus::Completed
                        } else if result.total_processed == 0 {
                            ImportStatus::Failed
                        } else {
                            ImportStatus::CompletedWithErrors
                        };
                    }
                }
            }

            // Emit progress
            let progress = {
                let batches = self.active_batches.read().await;
                batches.get(batch_id).map(|b| ImportProgress {
                    batch_id: b.batch_id.clone(),
                    status: b.status.clone(),
                    total_items: b.total_items,
                    processed_count: b.processed_count,
                    error_count: b.error_count,
                    errors: b.errors.clone(),
                    blob_hash: b.blob_hash.clone(),
                })
            };

            if let Some(progress) = progress {
                let _ = self.progress_tx.send(progress);
            }

            // Interval between chunks to avoid overwhelming conductor
            if !is_final {
                tokio::time::sleep(self.config.chunk_interval).await;
            }
        }

        info!(batch_id = %batch_id, "Batch processing completed");

        // Clean up completed batch after a delay (allow clients to fetch final status)
        let batch_id = batch_id.to_string();
        let active_batches = Arc::clone(&self.active_batches);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(300)).await; // 5 min retention
            active_batches.write().await.remove(&batch_id);
            debug!(batch_id = %batch_id, "Removed completed batch from active list");
        });

        Ok(())
    }
}

// ============================================================================
// In-Memory Blob Store (for testing/local development)
// ============================================================================

/// Simple in-memory blob store
pub struct InMemoryBlobStore {
    blobs: Arc<RwLock<HashMap<String, Bytes>>>,
}

impl InMemoryBlobStore {
    pub fn new() -> Self {
        Self {
            blobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryBlobStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl BlobStore for InMemoryBlobStore {
    async fn write_blob(&self, data: &[u8]) -> Result<String, ImportError> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("sha256-{}", hex::encode(hasher.finalize()));

        self.blobs
            .write()
            .await
            .insert(hash.clone(), Bytes::copy_from_slice(data));

        Ok(hash)
    }

    async fn read_blob(&self, hash: &str) -> Result<Bytes, ImportError> {
        self.blobs
            .read()
            .await
            .get(hash)
            .cloned()
            .ok_or_else(|| ImportError::BlobWriteError(format!("Blob not found: {}", hash)))
    }

    async fn blob_exists(&self, hash: &str) -> bool {
        self.blobs.read().await.contains_key(hash)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct MockZomeClient;

    #[async_trait::async_trait]
    impl ZomeClient for MockZomeClient {
        async fn queue_import(
            &self,
            _batch_id: &str,
            _batch_type: &str,
            _blob_hash: &str,
            _total_items: u32,
            _schema_version: u32,
        ) -> Result<(), ImportError> {
            Ok(())
        }

        async fn process_import_chunk(
            &self,
            _batch_id: &str,
            chunk_index: u32,
            is_final: bool,
            items_json: &str,
        ) -> Result<ChunkResult, ImportError> {
            let items: Vec<serde_json::Value> = serde_json::from_str(items_json).unwrap();
            Ok(ChunkResult {
                chunk_processed: items.len() as u32,
                chunk_errors: 0,
                total_processed: (chunk_index + 1) * items.len() as u32,
                total_errors: 0,
                status: if is_final { "completed" } else { "processing" }.to_string(),
            })
        }
    }

    #[tokio::test]
    async fn test_start_import() {
        let config = ImportOrchestratorConfig {
            chunk_size: 2,
            ..Default::default()
        };
        let zome_client = Arc::new(MockZomeClient);
        let blob_store = Arc::new(InMemoryBlobStore::new());

        let orchestrator = ImportOrchestrator::new(config, zome_client, blob_store);

        let input = StartImportInput {
            batch_id: "test-batch-1".to_string(),
            batch_type: "content".to_string(),
            items_json: r#"[{"id": "a"}, {"id": "b"}, {"id": "c"}]"#.to_string(),
        };

        let result = orchestrator.start_import(input).await.unwrap();

        assert_eq!(result.batch_id, "test-batch-1");
        assert_eq!(result.total_items, 3);
        assert!(result.blob_hash.starts_with("sha256-"));

        // Wait for processing to complete
        tokio::time::sleep(Duration::from_millis(500)).await;

        let status = orchestrator.get_status("test-batch-1").await;
        assert!(status.is_some());
    }

    #[tokio::test]
    async fn test_blob_store() {
        let store = InMemoryBlobStore::new();

        let data = b"hello world";
        let hash = store.write_blob(data).await.unwrap();

        assert!(hash.starts_with("sha256-"));
        assert!(store.blob_exists(&hash).await);

        let retrieved = store.read_blob(&hash).await.unwrap();
        assert_eq!(&retrieved[..], data);
    }
}
