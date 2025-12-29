//! Signal-based P2P blob transfer
//!
//! Couples blob storage to Holochain DHT by using Holochain signals for
//! coordination. The DNA network handles discovery, this handles data transfer.
//!
//! ## Protocol
//!
//! 1. **Request**: DNA emits `BlobRequest { hash, requester_agent }`
//! 2. **Announce**: Nodes that have the blob emit `BlobAnnounce { hash, chunks, size }`
//! 3. **Transfer**: Direct P2P transfer negotiated between nodes
//!
//! ## Signal Types
//!
//! Sent via Holochain DNA signal mechanism:
//! - `blob_request` - "I need this blob"
//! - `blob_announce` - "I have this blob"
//! - `blob_chunk` - "Here's a chunk" (for small chunks only)
//! - `blob_transfer_init` - "Let's do a direct transfer"

use crate::blob_store::BlobStore;
use crate::error::StorageError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Signal types for blob coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum BlobSignal {
    /// Request a blob by hash
    Request {
        hash: String,
        requester: String,
    },

    /// Announce having a blob
    Announce {
        hash: String,
        size_bytes: u64,
        chunk_count: u32,
        provider: String,
    },

    /// Small chunk delivery (for blobs < 256KB, fits in signal)
    ChunkDelivery {
        hash: String,
        chunk_index: u32,
        total_chunks: u32,
        data: Vec<u8>,
    },

    /// Initiate direct transfer for large blobs
    TransferInit {
        hash: String,
        size_bytes: u64,
        transfer_id: String,
        /// Direct connection info (e.g., WebRTC offer, or TCP address if on same network)
        connection_info: String,
    },
}

/// Maximum size for inline chunk delivery via signal (256KB)
const MAX_SIGNAL_CHUNK_SIZE: usize = 256 * 1024;

/// Pending blob requests
struct PendingRequest {
    hash: String,
    requester: String,
    received_chunks: HashMap<u32, Vec<u8>>,
    total_chunks: Option<u32>,
    created_at: std::time::Instant,
}

/// Signal handler for blob P2P coordination
pub struct SignalHandler {
    store: Arc<BlobStore>,
    pending_requests: RwLock<HashMap<String, PendingRequest>>,
    my_agent_id: String,
}

impl SignalHandler {
    pub fn new(store: Arc<BlobStore>, my_agent_id: String) -> Self {
        Self {
            store,
            pending_requests: RwLock::new(HashMap::new()),
            my_agent_id,
        }
    }

    /// Handle incoming signal from Holochain
    pub async fn handle_signal(&self, signal: BlobSignal) -> Result<Option<BlobSignal>, StorageError> {
        match signal {
            BlobSignal::Request { hash, requester } => {
                self.handle_request(&hash, &requester).await
            }
            BlobSignal::Announce { hash, size_bytes, chunk_count, provider } => {
                self.handle_announce(&hash, size_bytes, chunk_count, &provider).await
            }
            BlobSignal::ChunkDelivery { hash, chunk_index, total_chunks, data } => {
                self.handle_chunk_delivery(&hash, chunk_index, total_chunks, data).await
            }
            BlobSignal::TransferInit { hash, size_bytes, transfer_id, connection_info } => {
                self.handle_transfer_init(&hash, size_bytes, &transfer_id, &connection_info).await
            }
        }
    }

    /// Handle blob request - check if we have it and can provide
    async fn handle_request(&self, hash: &str, requester: &str) -> Result<Option<BlobSignal>, StorageError> {
        if !self.store.exists(hash).await {
            debug!(hash = %hash, "Blob request received but we don't have it");
            return Ok(None);
        }

        let size_bytes = self.store.size(hash).await?;

        info!(hash = %hash, requester = %requester, size = size_bytes, "Announcing blob availability");

        // If small enough, could send directly via chunks
        // For now, just announce availability
        Ok(Some(BlobSignal::Announce {
            hash: hash.to_string(),
            size_bytes,
            chunk_count: ((size_bytes + MAX_SIGNAL_CHUNK_SIZE as u64 - 1) / MAX_SIGNAL_CHUNK_SIZE as u64) as u32,
            provider: self.my_agent_id.clone(),
        }))
    }

    /// Handle blob announcement - decide if we want to request it
    async fn handle_announce(
        &self,
        hash: &str,
        size_bytes: u64,
        chunk_count: u32,
        provider: &str,
    ) -> Result<Option<BlobSignal>, StorageError> {
        // Check if we have a pending request for this blob
        let mut pending = self.pending_requests.write().await;

        if let Some(request) = pending.get_mut(hash) {
            request.total_chunks = Some(chunk_count);

            info!(
                hash = %hash,
                provider = %provider,
                size = size_bytes,
                chunks = chunk_count,
                "Provider announced for pending request"
            );

            // For small blobs, request chunk delivery via signals
            if size_bytes <= MAX_SIGNAL_CHUNK_SIZE as u64 * 4 {
                // Request will be handled by provider sending ChunkDelivery signals
                return Ok(None);
            }

            // For large blobs, we'd negotiate a direct transfer
            // This would involve WebRTC or direct TCP connection
            warn!(hash = %hash, "Large blob transfer not yet implemented");
        }

        Ok(None)
    }

    /// Handle incoming chunk
    async fn handle_chunk_delivery(
        &self,
        hash: &str,
        chunk_index: u32,
        total_chunks: u32,
        data: Vec<u8>,
    ) -> Result<Option<BlobSignal>, StorageError> {
        let mut pending = self.pending_requests.write().await;

        if let Some(request) = pending.get_mut(hash) {
            request.received_chunks.insert(chunk_index, data);
            request.total_chunks = Some(total_chunks);

            // Check if we have all chunks
            if request.received_chunks.len() as u32 == total_chunks {
                info!(hash = %hash, chunks = total_chunks, "All chunks received, reassembling");

                // Reassemble and store
                let mut full_data = Vec::new();
                for i in 0..total_chunks {
                    if let Some(chunk) = request.received_chunks.get(&i) {
                        full_data.extend_from_slice(chunk);
                    } else {
                        return Err(StorageError::ChunkMissing {
                            hash: hash.to_string(),
                            index: i,
                        });
                    }
                }

                // Verify hash
                let computed_hash = BlobStore::compute_hash(&full_data);
                if computed_hash != hash {
                    return Err(StorageError::HashMismatch {
                        expected: hash.to_string(),
                        actual: computed_hash,
                    });
                }

                // Store
                self.store.store(&full_data).await?;
                pending.remove(hash);

                info!(hash = %hash, size = full_data.len(), "Blob received and stored");
            }
        } else {
            debug!(hash = %hash, "Received chunk for unknown request");
        }

        Ok(None)
    }

    /// Handle direct transfer initiation
    async fn handle_transfer_init(
        &self,
        hash: &str,
        size_bytes: u64,
        transfer_id: &str,
        connection_info: &str,
    ) -> Result<Option<BlobSignal>, StorageError> {
        // TODO: Implement WebRTC or direct TCP transfer for large blobs
        warn!(
            hash = %hash,
            size = size_bytes,
            transfer_id = %transfer_id,
            "Direct transfer requested but not yet implemented"
        );

        Ok(None)
    }

    /// Request a blob from the network
    pub async fn request_blob(&self, hash: &str) -> Result<(), StorageError> {
        // Check if we already have it
        if self.store.exists(hash).await {
            debug!(hash = %hash, "Already have blob, skipping request");
            return Ok(());
        }

        // Add to pending requests
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(hash.to_string(), PendingRequest {
                hash: hash.to_string(),
                requester: self.my_agent_id.clone(),
                received_chunks: HashMap::new(),
                total_chunks: None,
                created_at: std::time::Instant::now(),
            });
        }

        info!(hash = %hash, "Requesting blob from network");

        // The actual signal emission would happen through Holochain client
        // Caller should emit BlobSignal::Request through the conductor

        Ok(())
    }

    /// Provide a blob to a requester
    pub async fn provide_blob(&self, hash: &str, requester: &str) -> Result<Vec<BlobSignal>, StorageError> {
        let data = self.store.get(hash).await?;
        let size = data.len();

        if size <= MAX_SIGNAL_CHUNK_SIZE {
            // Send as single chunk
            Ok(vec![BlobSignal::ChunkDelivery {
                hash: hash.to_string(),
                chunk_index: 0,
                total_chunks: 1,
                data,
            }])
        } else if size <= MAX_SIGNAL_CHUNK_SIZE * 10 {
            // Send as multiple chunks via signals
            let chunk_count = (size + MAX_SIGNAL_CHUNK_SIZE - 1) / MAX_SIGNAL_CHUNK_SIZE;
            let mut signals = Vec::with_capacity(chunk_count);

            for (i, chunk) in data.chunks(MAX_SIGNAL_CHUNK_SIZE).enumerate() {
                signals.push(BlobSignal::ChunkDelivery {
                    hash: hash.to_string(),
                    chunk_index: i as u32,
                    total_chunks: chunk_count as u32,
                    data: chunk.to_vec(),
                });
            }

            Ok(signals)
        } else {
            // Too large for signal-based transfer, need direct connection
            Ok(vec![BlobSignal::TransferInit {
                hash: hash.to_string(),
                size_bytes: size as u64,
                transfer_id: uuid::Uuid::new_v4().to_string(),
                connection_info: "TODO: WebRTC or TCP".to_string(),
            }])
        }
    }

    /// Clean up old pending requests
    pub async fn cleanup_stale_requests(&self, max_age_secs: u64) {
        let mut pending = self.pending_requests.write().await;
        let now = std::time::Instant::now();

        pending.retain(|hash, request| {
            let age = now.duration_since(request.created_at).as_secs();
            if age > max_age_secs {
                warn!(hash = %hash, age_secs = age, "Cleaning up stale blob request");
                false
            } else {
                true
            }
        });
    }
}
