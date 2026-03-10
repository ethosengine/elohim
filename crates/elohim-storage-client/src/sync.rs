//! Automerge sync helpers for elohim-storage

use crate::client::StorageClient;
use crate::error::{Result, StorageError};
use automerge::{Automerge, ChangeHash};
use base64::Engine;
use std::collections::HashMap;

/// Sync result
pub struct SyncResult {
    /// Updated document
    pub doc: Automerge,
    /// Whether any changes were applied
    pub changed: bool,
    /// New heads after sync
    pub heads: Vec<String>,
}

/// Helper for Automerge document sync
///
/// # Example
///
/// ```rust,no_run
/// use elohim_storage_client::{StorageClient, StorageConfig, AutomergeSync};
/// use automerge::transaction::Transactable;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = StorageClient::new(StorageConfig {
///     base_url: "http://localhost:8080".into(),
///     app_id: "lamad".into(),
///     ..Default::default()
/// });
/// let mut sync = AutomergeSync::new(client);
///
/// // Load or create a document
/// let mut doc = sync.load("graph:my-doc").await?;
///
/// // Make local changes
/// doc.transact::<_, _, automerge::AutomergeError>(|tx| {
///     tx.put(automerge::ROOT, "title", "Updated")?;
///     Ok(())
/// })?;
///
/// // Save changes to server
/// sync.save("graph:my-doc", &doc).await?;
/// # Ok(())
/// # }
/// ```
pub struct AutomergeSync {
    client: StorageClient,
    /// Local cache of known heads per document
    known_heads: HashMap<String, Vec<String>>,
}

impl AutomergeSync {
    /// Create a new sync helper
    pub fn new(client: StorageClient) -> Self {
        Self {
            client,
            known_heads: HashMap::new(),
        }
    }

    /// Load a document from the server
    ///
    /// Creates a new empty document if it doesn't exist.
    pub async fn load(&mut self, doc_id: &str) -> Result<Automerge> {
        let response = self.client.get_changes_since(doc_id, &[]).await;

        match response {
            Ok(resp) => {
                if resp.changes.is_empty() {
                    // Document doesn't exist, return empty doc
                    let doc = Automerge::new();
                    self.known_heads.insert(doc_id.to_string(), vec![]);
                    return Ok(doc);
                }

                // Decode and apply all changes
                let mut doc = Automerge::new();
                for change_b64 in &resp.changes {
                    let change_bytes = base64::engine::general_purpose::STANDARD
                        .decode(change_b64)
                        .map_err(StorageError::from)?;
                    doc.load_incremental(&change_bytes)?;
                }

                // Track known heads
                self.known_heads
                    .insert(doc_id.to_string(), resp.new_heads.clone());

                Ok(doc)
            }
            Err(StorageError::NotFound(_)) => {
                // Document doesn't exist
                let doc = Automerge::new();
                self.known_heads.insert(doc_id.to_string(), vec![]);
                Ok(doc)
            }
            Err(e) => Err(e),
        }
    }

    /// Save local changes to the server
    ///
    /// Sends only changes since last sync.
    pub async fn save(&mut self, doc_id: &str, doc: &Automerge) -> Result<Vec<String>> {
        let known_heads = self.known_heads.get(doc_id).cloned().unwrap_or_default();

        // Get changes since known heads
        let change_bytes = self.get_changes_since(doc, &known_heads);

        if change_bytes.is_empty() {
            // No new changes
            return Ok(self.get_heads(doc));
        }

        // Send changes to server
        let response = self.client.apply_changes(doc_id, &[change_bytes]).await?;

        // Update known heads
        self.known_heads
            .insert(doc_id.to_string(), response.new_heads.clone());

        Ok(response.new_heads)
    }

    /// Bidirectional sync with server
    ///
    /// 1. Gets server changes since our known heads
    /// 2. Sends our changes since server's heads
    /// 3. Merges everything locally
    pub async fn sync(&mut self, doc_id: &str, doc: Automerge) -> Result<SyncResult> {
        let known_heads = self.known_heads.get(doc_id).cloned().unwrap_or_default();

        // 1. Get server changes since our known heads
        let server_response = self.client.get_changes_since(doc_id, &known_heads).await?;

        // 2. Apply server changes locally
        let mut updated_doc = doc;
        let mut changed = false;

        for change_b64 in &server_response.changes {
            let change_bytes = base64::engine::general_purpose::STANDARD
                .decode(change_b64)
                .map_err(StorageError::from)?;
            updated_doc.load_incremental(&change_bytes)?;
            changed = true;
        }

        // 3. Check if we have local changes the server doesn't have
        let local_changes = self.get_changes_since(&updated_doc, &known_heads);
        if !local_changes.is_empty() {
            // Send our changes to server
            self.client.apply_changes(doc_id, &[local_changes]).await?;
        }

        // 4. Update known heads
        let new_heads = self.get_heads(&updated_doc);
        self.known_heads.insert(doc_id.to_string(), new_heads.clone());

        Ok(SyncResult {
            doc: updated_doc,
            changed,
            heads: new_heads,
        })
    }

    /// Check if document exists on server
    pub async fn exists(&self, doc_id: &str) -> Result<bool> {
        match self.client.get_heads(doc_id).await {
            Ok(response) => Ok(!response.heads.is_empty()),
            Err(StorageError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Delete local tracking for a document
    pub fn forget(&mut self, doc_id: &str) {
        self.known_heads.remove(doc_id);
    }

    /// Get current heads from an Automerge document
    fn get_heads(&self, doc: &Automerge) -> Vec<String> {
        doc.get_heads().iter().map(|h| hex::encode(h.0)).collect()
    }

    /// Get changes since given heads from an Automerge document
    fn get_changes_since(&self, doc: &Automerge, heads: &[String]) -> Vec<u8> {
        let change_hashes: Vec<ChangeHash> = heads
            .iter()
            .filter_map(|h| {
                let bytes = hex::decode(h).ok()?;
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Some(ChangeHash(arr))
                } else {
                    None
                }
            })
            .collect();

        doc.save_after(&change_hashes)
    }
}
