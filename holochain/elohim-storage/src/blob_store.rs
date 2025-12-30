//! Content-addressed blob storage
//!
//! Stores blobs in a local directory structure using SHA256 hashes as filenames.
//! Supports chunked storage for large files.

use crate::error::StorageError;
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};

/// Chunk size for large blobs (1MB)
pub const CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum blob size before chunking (16MB - matches Holochain entry limit)
pub const MAX_INLINE_SIZE: usize = 16 * 1024 * 1024;

/// Result of storing a blob
#[derive(Debug, Clone)]
pub struct StoreResult {
    /// SHA256 hash of the blob
    pub hash: String,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Number of chunks (1 for small blobs)
    pub chunk_count: u32,
    /// Whether blob was chunked
    pub chunked: bool,
    /// Whether blob already existed
    pub already_existed: bool,
}

/// Blob storage manager
pub struct BlobStore {
    /// Root directory for blob storage
    root_dir: PathBuf,
}

impl BlobStore {
    /// Create a new blob store at the given directory
    pub async fn new<P: AsRef<Path>>(root_dir: P) -> Result<Self, StorageError> {
        let root_dir = root_dir.as_ref().to_path_buf();

        // Ensure directory exists
        fs::create_dir_all(&root_dir).await?;

        info!(path = %root_dir.display(), "Initialized blob store");

        Ok(Self { root_dir })
    }

    /// Create an in-memory blob store (for tests)
    ///
    /// Uses a temporary directory that will be cleaned up when dropped.
    /// Note: This creates a real temp directory, not truly in-memory,
    /// but is suitable for unit tests.
    pub fn new_memory() -> Self {
        let temp_dir = std::env::temp_dir().join(format!("elohim-blobs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).ok();
        Self { root_dir: temp_dir }
    }

    /// Compute SHA256 hash of data
    pub fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        format!("sha256-{}", hex::encode(result))
    }

    /// Get path for a blob by hash
    fn blob_path(&self, hash: &str) -> PathBuf {
        // Use first 4 chars of hash (after "sha256-") as subdirectory for better filesystem distribution
        let hash_part = hash.strip_prefix("sha256-").unwrap_or(hash);
        let subdir = &hash_part[..4.min(hash_part.len())];
        self.root_dir.join("blobs").join(subdir).join(hash)
    }

    /// Get path for chunk index file
    fn chunk_index_path(&self, hash: &str) -> PathBuf {
        self.blob_path(hash).with_extension("chunks")
    }

    /// Store a blob, returning its hash
    ///
    /// For large blobs (>16MB), automatically chunks and stores pieces.
    pub async fn store(&self, data: &[u8]) -> Result<StoreResult, StorageError> {
        let hash = Self::compute_hash(data);
        let blob_path = self.blob_path(&hash);

        // Check if already exists
        if fs::metadata(&blob_path).await.is_ok() {
            debug!(hash = %hash, "Blob already exists");
            return Ok(StoreResult {
                hash,
                size_bytes: data.len() as u64,
                chunk_count: 1, // TODO: read from chunk index if exists
                chunked: data.len() > MAX_INLINE_SIZE,
                already_existed: true,
            });
        }

        // Ensure parent directory exists
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        if data.len() <= MAX_INLINE_SIZE {
            // Store as single file
            fs::write(&blob_path, data).await?;

            info!(hash = %hash, size = data.len(), "Stored blob");

            Ok(StoreResult {
                hash,
                size_bytes: data.len() as u64,
                chunk_count: 1,
                chunked: false,
                already_existed: false,
            })
        } else {
            // Store as chunks
            self.store_chunked(&hash, data).await
        }
    }

    /// Store a large blob as chunks
    async fn store_chunked(&self, hash: &str, data: &[u8]) -> Result<StoreResult, StorageError> {
        let blob_path = self.blob_path(hash);
        let chunk_dir = blob_path.with_extension("d");

        fs::create_dir_all(&chunk_dir).await?;

        let chunk_count = (data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;
        let mut chunk_hashes = Vec::with_capacity(chunk_count);

        for (i, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
            let chunk_hash = Self::compute_hash(chunk);
            let chunk_path = chunk_dir.join(format!("{:08}", i));

            fs::write(&chunk_path, chunk).await?;
            chunk_hashes.push(chunk_hash);
        }

        // Write chunk index file
        let index_path = self.chunk_index_path(hash);
        let index_json = serde_json::json!({
            "hash": hash,
            "size_bytes": data.len(),
            "chunk_count": chunk_count,
            "chunk_size": CHUNK_SIZE,
            "chunk_hashes": chunk_hashes,
        });
        fs::write(&index_path, index_json.to_string()).await?;

        // Write a marker file at the blob path
        fs::write(&blob_path, b"CHUNKED").await?;

        info!(
            hash = %hash,
            size = data.len(),
            chunks = chunk_count,
            "Stored chunked blob"
        );

        Ok(StoreResult {
            hash: hash.to_string(),
            size_bytes: data.len() as u64,
            chunk_count: chunk_count as u32,
            chunked: true,
            already_existed: false,
        })
    }

    /// Check if a blob exists
    pub async fn exists(&self, hash: &str) -> bool {
        fs::metadata(self.blob_path(hash)).await.is_ok()
    }

    /// Get blob size (without loading data)
    pub async fn size(&self, hash: &str) -> Result<u64, StorageError> {
        let blob_path = self.blob_path(hash);
        let metadata = fs::metadata(&blob_path).await?;

        // Check if chunked
        if metadata.len() == 7 {
            let content = fs::read_to_string(&blob_path).await?;
            if content == "CHUNKED" {
                // Read from chunk index
                let index_path = self.chunk_index_path(hash);
                let index_str = fs::read_to_string(&index_path).await?;
                let index: serde_json::Value = serde_json::from_str(&index_str)?;
                return Ok(index["size_bytes"].as_u64().unwrap_or(0));
            }
        }

        Ok(metadata.len())
    }

    /// Retrieve a blob by hash
    pub async fn get(&self, hash: &str) -> Result<Vec<u8>, StorageError> {
        let blob_path = self.blob_path(hash);

        // Check if file exists
        if !fs::metadata(&blob_path).await.is_ok() {
            return Err(StorageError::NotFound(hash.to_string()));
        }

        // Check if chunked
        let content = fs::read(&blob_path).await?;
        if content == b"CHUNKED" {
            return self.get_chunked(hash).await;
        }

        Ok(content)
    }

    /// Retrieve a chunked blob
    async fn get_chunked(&self, hash: &str) -> Result<Vec<u8>, StorageError> {
        let index_path = self.chunk_index_path(hash);
        let index_str = fs::read_to_string(&index_path).await?;
        let index: serde_json::Value = serde_json::from_str(&index_str)?;

        let chunk_count = index["chunk_count"].as_u64().unwrap_or(0) as usize;
        let size_bytes = index["size_bytes"].as_u64().unwrap_or(0) as usize;

        let chunk_dir = self.blob_path(hash).with_extension("d");
        let mut data = Vec::with_capacity(size_bytes);

        for i in 0..chunk_count {
            let chunk_path = chunk_dir.join(format!("{:08}", i));
            let chunk = fs::read(&chunk_path).await?;
            data.extend_from_slice(&chunk);
        }

        // Verify reassembled hash
        let computed_hash = Self::compute_hash(&data);
        if computed_hash != hash {
            return Err(StorageError::HashMismatch {
                expected: hash.to_string(),
                actual: computed_hash,
            });
        }

        Ok(data)
    }

    /// Get a range of bytes from a blob (for HTTP Range requests)
    pub async fn get_range(&self, hash: &str, start: u64, end: u64) -> Result<Vec<u8>, StorageError> {
        let blob_path = self.blob_path(hash);

        // Check if chunked - for now, load full blob and slice
        // TODO: optimize for chunked blobs by only loading needed chunks
        let content = fs::read(&blob_path).await?;
        if content == b"CHUNKED" {
            let full = self.get_chunked(hash).await?;
            return Ok(full[start as usize..end.min(full.len() as u64) as usize].to_vec());
        }

        Ok(content[start as usize..end.min(content.len() as u64) as usize].to_vec())
    }

    /// Delete a blob
    pub async fn delete(&self, hash: &str) -> Result<(), StorageError> {
        let blob_path = self.blob_path(hash);

        // Check if chunked
        if let Ok(content) = fs::read(&blob_path).await {
            if content == b"CHUNKED" {
                // Delete chunk directory
                let chunk_dir = blob_path.with_extension("d");
                fs::remove_dir_all(&chunk_dir).await.ok();

                // Delete chunk index
                let index_path = self.chunk_index_path(hash);
                fs::remove_file(&index_path).await.ok();
            }
        }

        // Delete main file
        fs::remove_file(&blob_path).await.ok();

        info!(hash = %hash, "Deleted blob");
        Ok(())
    }

    /// Get storage statistics
    pub async fn stats(&self) -> Result<StorageStats, StorageError> {
        let blobs_dir = self.root_dir.join("blobs");
        let mut total_blobs = 0u64;
        let mut total_bytes = 0u64;
        let mut chunked_blobs = 0u64;

        if let Ok(mut entries) = fs::read_dir(&blobs_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.path().is_dir() {
                    if let Ok(mut subentries) = fs::read_dir(entry.path()).await {
                        while let Ok(Some(subentry)) = subentries.next_entry().await {
                            let path = subentry.path();
                            if !path.to_string_lossy().contains(".chunks") && !path.to_string_lossy().ends_with(".d") {
                                total_blobs += 1;
                                if let Ok(metadata) = fs::metadata(&path).await {
                                    if metadata.len() == 7 {
                                        // Might be a chunked marker
                                        if let Ok(content) = fs::read(&path).await {
                                            if content == b"CHUNKED" {
                                                chunked_blobs += 1;
                                                // Get actual size from chunk index
                                                if let Some(hash) = path.file_name().map(|f| f.to_string_lossy().to_string()) {
                                                    if let Ok(size) = self.size(&hash).await {
                                                        total_bytes += size;
                                                        continue;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    total_bytes += metadata.len();
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(StorageStats {
            total_blobs,
            total_bytes,
            chunked_blobs,
        })
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_blobs: u64,
    pub total_bytes: u64,
    pub chunked_blobs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let store = BlobStore::new(temp_dir.path()).await.unwrap();

        let data = b"Hello, Elohim!";
        let result = store.store(data).await.unwrap();

        assert!(result.hash.starts_with("sha256-"));
        assert_eq!(result.size_bytes, data.len() as u64);
        assert!(!result.chunked);
        assert!(!result.already_existed);

        let retrieved = store.get(&result.hash).await.unwrap();
        assert_eq!(retrieved, data);
    }

    #[tokio::test]
    async fn test_idempotent_store() {
        let temp_dir = TempDir::new().unwrap();
        let store = BlobStore::new(temp_dir.path()).await.unwrap();

        let data = b"Duplicate test";
        let result1 = store.store(data).await.unwrap();
        let result2 = store.store(data).await.unwrap();

        assert_eq!(result1.hash, result2.hash);
        assert!(!result1.already_existed);
        assert!(result2.already_existed);
    }

    #[tokio::test]
    async fn test_compute_hash() {
        let hash = BlobStore::compute_hash(b"test");
        assert!(hash.starts_with("sha256-"));
        assert_eq!(hash.len(), 7 + 64); // "sha256-" + 64 hex chars
    }
}
