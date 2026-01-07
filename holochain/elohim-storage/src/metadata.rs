//! Metadata database for blob tracking
//!
//! Tracks:
//! - Which blobs we have locally
//! - Replication status (which peers have copies)
//! - Access timestamps (for LRU eviction)
//! - Sync status with Holochain DNA

use crate::error::StorageError;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::Path;
use tracing::info;

/// Blob metadata stored in local database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobMetadata {
    /// SHA256 hash
    pub hash: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// MIME type
    pub mime_type: String,
    /// Reach level (private, family, community, commons)
    pub reach: String,
    /// When first stored locally
    pub stored_at: u64,
    /// Last access time
    pub last_accessed: u64,
    /// Whether registered in Holochain DNA
    pub dna_registered: bool,
    /// Agents known to have this blob
    pub known_replicas: Vec<String>,
}

/// Metadata database
pub struct MetadataDb {
    db: Db,
}

impl MetadataDb {
    /// Open or create metadata database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let db = sled::open(path.as_ref())?;
        info!(path = %path.as_ref().display(), "Opened metadata database");
        Ok(Self { db })
    }

    /// Store blob metadata
    pub fn put(&self, metadata: &BlobMetadata) -> Result<(), StorageError> {
        let key = metadata.hash.as_bytes();
        let value = rmp_serde::to_vec(metadata)
            .map_err(|e| StorageError::Database(format!("Serialization error: {}", e)))?;
        self.db.insert(key, value)?;
        Ok(())
    }

    /// Get blob metadata
    pub fn get(&self, hash: &str) -> Result<Option<BlobMetadata>, StorageError> {
        if let Some(value) = self.db.get(hash.as_bytes())? {
            let metadata: BlobMetadata = rmp_serde::from_slice(&value)
                .map_err(|e| StorageError::Database(format!("Deserialization error: {}", e)))?;
            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }

    /// Delete blob metadata
    pub fn delete(&self, hash: &str) -> Result<(), StorageError> {
        self.db.remove(hash.as_bytes())?;
        Ok(())
    }

    /// Update last accessed timestamp
    pub fn touch(&self, hash: &str) -> Result<(), StorageError> {
        if let Some(mut metadata) = self.get(hash)? {
            metadata.last_accessed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.put(&metadata)?;
        }
        Ok(())
    }

    /// Mark blob as registered in DNA
    pub fn mark_dna_registered(&self, hash: &str) -> Result<(), StorageError> {
        if let Some(mut metadata) = self.get(hash)? {
            metadata.dna_registered = true;
            self.put(&metadata)?;
        }
        Ok(())
    }

    /// Add known replica
    pub fn add_replica(&self, hash: &str, agent_id: &str) -> Result<(), StorageError> {
        if let Some(mut metadata) = self.get(hash)? {
            if !metadata.known_replicas.contains(&agent_id.to_string()) {
                metadata.known_replicas.push(agent_id.to_string());
                self.put(&metadata)?;
            }
        }
        Ok(())
    }

    /// List all blob hashes
    pub fn list_all(&self) -> Result<Vec<String>, StorageError> {
        let mut hashes = Vec::new();
        for item in self.db.iter() {
            let (key, _) = item?;
            if let Ok(hash) = String::from_utf8(key.to_vec()) {
                hashes.push(hash);
            }
        }
        Ok(hashes)
    }

    /// List blobs not yet registered in DNA
    pub fn list_unregistered(&self) -> Result<Vec<BlobMetadata>, StorageError> {
        let mut result = Vec::new();
        for item in self.db.iter() {
            let (_, value) = item?;
            let metadata: BlobMetadata = rmp_serde::from_slice(&value)
                .map_err(|e| StorageError::Database(format!("Deserialization error: {}", e)))?;
            if !metadata.dna_registered {
                result.push(metadata);
            }
        }
        Ok(result)
    }

    /// Get LRU candidates for eviction
    pub fn get_lru_candidates(&self, count: usize) -> Result<Vec<BlobMetadata>, StorageError> {
        let mut all: Vec<BlobMetadata> = Vec::new();
        for item in self.db.iter() {
            let (_, value) = item?;
            let metadata: BlobMetadata = rmp_serde::from_slice(&value)
                .map_err(|e| StorageError::Database(format!("Deserialization error: {}", e)))?;
            all.push(metadata);
        }

        // Sort by last accessed (oldest first)
        all.sort_by_key(|m| m.last_accessed);

        Ok(all.into_iter().take(count).collect())
    }

    /// Get storage statistics
    pub fn stats(&self) -> Result<MetadataStats, StorageError> {
        let mut total_blobs = 0u64;
        let mut total_bytes = 0u64;
        let mut registered = 0u64;
        let mut unregistered = 0u64;
        let mut by_reach: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

        for item in self.db.iter() {
            let (_, value) = item?;
            let metadata: BlobMetadata = rmp_serde::from_slice(&value)
                .map_err(|e| StorageError::Database(format!("Deserialization error: {}", e)))?;

            total_blobs += 1;
            total_bytes += metadata.size_bytes;

            if metadata.dna_registered {
                registered += 1;
            } else {
                unregistered += 1;
            }

            *by_reach.entry(metadata.reach.clone()).or_insert(0) += 1;
        }

        Ok(MetadataStats {
            total_blobs,
            total_bytes,
            registered,
            unregistered,
            by_reach,
        })
    }
}

/// Metadata statistics
#[derive(Debug, Clone)]
pub struct MetadataStats {
    pub total_blobs: u64,
    pub total_bytes: u64,
    pub registered: u64,
    pub unregistered: u64,
    pub by_reach: std::collections::HashMap<String, u64>,
}
