//! Document Store - Automerge document persistence
//!
//! Stores Automerge documents in sled for durability and fast access.

use automerge::Automerge;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::error::StorageError;

/// Configuration for document store
#[derive(Debug, Clone)]
pub struct DocStoreConfig {
    /// Path to sled database
    pub db_path: std::path::PathBuf,
    /// Cache size in bytes
    pub cache_size: u64,
}

impl Default for DocStoreConfig {
    fn default() -> Self {
        Self {
            db_path: dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("elohim-storage")
                .join("sync.sled"),
            cache_size: 64 * 1024 * 1024, // 64MB
        }
    }
}

/// Stored document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDocument {
    /// Application namespace (e.g., "lamad", "calendar")
    pub app_id: String,
    /// Document ID
    pub doc_id: String,
    /// Document type (e.g., "graph", "path", "personal")
    pub doc_type: String,
    /// Serialized Automerge document
    pub data: Vec<u8>,
    /// Number of changes
    pub change_count: u64,
    /// Last modified timestamp (Unix millis)
    pub last_modified: u64,
    /// Current heads (hex-encoded)
    pub heads: Vec<String>,
}

/// Document store backed by sled
pub struct DocStore {
    /// Sled database
    db: sled::Db,
    /// Documents tree
    docs: sled::Tree,
    /// Metadata tree (for indexing)
    meta: sled::Tree,
}

impl DocStore {
    /// Create a new document store
    pub async fn new(config: DocStoreConfig) -> Result<Self, StorageError> {
        // Ensure parent directory exists
        if let Some(parent) = config.db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Open sled database
        let db = sled::Config::new()
            .path(&config.db_path)
            .cache_capacity(config.cache_size)
            .mode(sled::Mode::HighThroughput)
            .open()
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let docs = db
            .open_tree("documents")
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let meta = db
            .open_tree("metadata")
            .map_err(|e| StorageError::Database(e.to_string()))?;

        info!(path = %config.db_path.display(), "DocStore initialized");

        Ok(Self { db, docs, meta })
    }

    /// Create document store at a specific path
    pub async fn at_path(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Self::new(DocStoreConfig {
            db_path: path.as_ref().to_path_buf(),
            ..Default::default()
        })
        .await
    }

    /// Save an Automerge document
    ///
    /// Documents are stored with composite keys: `{app_id}:{doc_id}`
    pub async fn save(&self, app_id: &str, doc_id: &str, doc: &Automerge) -> Result<(), StorageError> {
        let data = doc.save();
        let heads: Vec<String> = doc.get_heads().iter().map(|h| hex::encode(h.0)).collect();
        let change_count = doc.get_changes(&[]).len() as u64;

        let stored = StoredDocument {
            app_id: app_id.to_string(),
            doc_id: doc_id.to_string(),
            doc_type: self.infer_doc_type(doc_id),
            data,
            change_count,
            last_modified: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            heads,
        };

        let bytes = rmp_serde::to_vec(&stored)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        // Composite key: app_id:doc_id
        let storage_key = format!("{}:{}", app_id, doc_id);
        self.docs
            .insert(storage_key.as_bytes(), bytes)
            .map_err(|e| StorageError::Database(e.to_string()))?;

        // Update metadata index: app_id:doc_type:doc_id -> storage_key
        let meta_key = format!("{}:{}:{}", app_id, stored.doc_type, doc_id);
        self.meta
            .insert(meta_key.as_bytes(), storage_key.as_bytes())
            .map_err(|e| StorageError::Database(e.to_string()))?;

        debug!(app_id = %app_id, doc_id = %doc_id, change_count = change_count, "Document saved");
        Ok(())
    }

    /// Get a stored document
    pub async fn get(&self, app_id: &str, doc_id: &str) -> Result<Option<StoredDocument>, StorageError> {
        let storage_key = format!("{}:{}", app_id, doc_id);
        match self.docs.get(storage_key.as_bytes()) {
            Ok(Some(bytes)) => {
                let stored: StoredDocument = rmp_serde::from_slice(&bytes)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(stored))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Database(e.to_string())),
        }
    }

    /// Get a stored document by storage key (internal use)
    async fn get_by_key(&self, storage_key: &str) -> Result<Option<StoredDocument>, StorageError> {
        match self.docs.get(storage_key.as_bytes()) {
            Ok(Some(bytes)) => {
                let stored: StoredDocument = rmp_serde::from_slice(&bytes)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(stored))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(StorageError::Database(e.to_string())),
        }
    }

    /// Delete a document
    pub async fn delete(&self, app_id: &str, doc_id: &str) -> Result<bool, StorageError> {
        let storage_key = format!("{}:{}", app_id, doc_id);

        // Get doc to find its type for metadata cleanup
        if let Some(stored) = self.get(app_id, doc_id).await? {
            let meta_key = format!("{}:{}:{}", app_id, stored.doc_type, doc_id);
            self.meta
                .remove(meta_key.as_bytes())
                .map_err(|e| StorageError::Database(e.to_string()))?;
        }

        let existed = self
            .docs
            .remove(storage_key.as_bytes())
            .map_err(|e| StorageError::Database(e.to_string()))?
            .is_some();

        if existed {
            debug!(app_id = %app_id, doc_id = %doc_id, "Document deleted");
        }

        Ok(existed)
    }

    /// Check if a document exists
    pub async fn exists(&self, app_id: &str, doc_id: &str) -> Result<bool, StorageError> {
        let storage_key = format!("{}:{}", app_id, doc_id);
        self.docs
            .contains_key(storage_key.as_bytes())
            .map_err(|e| StorageError::Database(e.to_string()))
    }

    /// List documents with optional filtering and pagination
    ///
    /// - `app_id`: Required application namespace
    /// - `prefix`: Optional filter by document type (e.g., "graph", "path")
    pub async fn list(
        &self,
        app_id: &str,
        prefix: Option<&str>,
        offset: u32,
        limit: u32,
    ) -> Result<(Vec<StoredDocument>, u64), StorageError> {
        let mut docs = Vec::new();
        let mut total = 0u64;
        let mut skipped = 0u32;

        // Build scan prefix: app_id:type: or app_id:
        let scan_prefix = if let Some(doc_type) = prefix {
            format!("{}:{}:", app_id, doc_type)
        } else {
            format!("{}:", app_id)
        };

        // Scan documents with prefix
        let iter = self.docs.scan_prefix(scan_prefix.as_bytes());

        for item in iter {
            let (key, _) = item.map_err(|e| StorageError::Database(e.to_string()))?;
            let storage_key = String::from_utf8_lossy(&key).to_string();

            total += 1;

            // Skip for pagination
            if skipped < offset {
                skipped += 1;
                continue;
            }

            // Limit check
            if docs.len() >= limit as usize {
                continue; // Keep counting total
            }

            // Fetch actual document by storage key
            if let Some(stored) = self.get_by_key(&storage_key).await? {
                docs.push(stored);
            }
        }

        Ok((docs, total))
    }

    /// Get document count for an app
    pub async fn count(&self, app_id: &str) -> Result<u64, StorageError> {
        let prefix = format!("{}:", app_id);
        Ok(self.docs.scan_prefix(prefix.as_bytes()).count() as u64)
    }

    /// Get total document count across all apps
    pub async fn count_all(&self) -> Result<u64, StorageError> {
        Ok(self.docs.len() as u64)
    }

    /// Flush changes to disk
    pub async fn flush(&self) -> Result<(), StorageError> {
        self.db
            .flush_async()
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        Ok(())
    }

    /// Infer document type from ID
    fn infer_doc_type(&self, doc_id: &str) -> String {
        if doc_id.starts_with("graph:") {
            "graph".to_string()
        } else if doc_id.starts_with("path:") {
            "path".to_string()
        } else if doc_id.starts_with("personal:") {
            "personal".to_string()
        } else if doc_id.starts_with("community:") {
            "community".to_string()
        } else {
            "unknown".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use automerge::transaction::Transactable;
    use automerge::ReadDoc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_doc_store_crud() {
        let temp_dir = TempDir::new().unwrap();
        let store = DocStore::at_path(temp_dir.path().join("test.sled"))
            .await
            .unwrap();

        // Create a new document
        let mut doc = Automerge::new();
        doc.transact::<_, _, automerge::AutomergeError>(|tx| {
            tx.put(automerge::ROOT, "key", "value")?;
            Ok(())
        })
        .unwrap();

        // Save with app_id
        store.save("lamad", "test-doc", &doc).await.unwrap();

        // Get
        let stored = store.get("lamad", "test-doc").await.unwrap().unwrap();
        assert_eq!(stored.app_id, "lamad");
        assert_eq!(stored.doc_id, "test-doc");
        assert!(stored.change_count > 0);

        // Load and verify
        let loaded = Automerge::load(&stored.data).unwrap();
        let value: Option<String> = loaded.get(automerge::ROOT, "key").unwrap().map(|(v, _)| {
            if let automerge::Value::Scalar(s) = v {
                if let automerge::ScalarValue::Str(smol) = s.as_ref() {
                    return smol.to_string();
                }
            }
            String::new()
        });
        assert_eq!(value, Some("value".to_string()));

        // Delete
        assert!(store.delete("lamad", "test-doc").await.unwrap());
        assert!(store.get("lamad", "test-doc").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_app_isolation() {
        let temp_dir = TempDir::new().unwrap();
        let store = DocStore::at_path(temp_dir.path().join("test.sled"))
            .await
            .unwrap();

        // Create a document
        let mut doc = Automerge::new();
        doc.transact::<_, _, automerge::AutomergeError>(|tx| {
            tx.put(automerge::ROOT, "value", "app1-data")?;
            Ok(())
        })
        .unwrap();

        // Save same doc_id under different apps
        store.save("app1", "shared-doc", &doc).await.unwrap();

        let mut doc2 = Automerge::new();
        doc2.transact::<_, _, automerge::AutomergeError>(|tx| {
            tx.put(automerge::ROOT, "value", "app2-data")?;
            Ok(())
        })
        .unwrap();
        store.save("app2", "shared-doc", &doc2).await.unwrap();

        // Verify isolation - each app sees its own doc
        let stored1 = store.get("app1", "shared-doc").await.unwrap().unwrap();
        assert_eq!(stored1.app_id, "app1");

        let stored2 = store.get("app2", "shared-doc").await.unwrap().unwrap();
        assert_eq!(stored2.app_id, "app2");

        // Verify list isolation
        let (docs1, count1) = store.list("app1", None, 0, 10).await.unwrap();
        assert_eq!(count1, 1);
        assert_eq!(docs1[0].app_id, "app1");

        let (docs2, count2) = store.list("app2", None, 0, 10).await.unwrap();
        assert_eq!(count2, 1);
        assert_eq!(docs2[0].app_id, "app2");

        // Verify count isolation
        assert_eq!(store.count("app1").await.unwrap(), 1);
        assert_eq!(store.count("app2").await.unwrap(), 1);
        assert_eq!(store.count_all().await.unwrap(), 2);
    }
}
