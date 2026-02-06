//! Syncable trait for CRDT-based synchronization

/// Trait for content that can be synchronized via Automerge CRDT.
///
/// Content types that implement this trait can participate in P2P sync
/// with automatic conflict resolution.
///
/// # Example
///
/// ```rust,ignore
/// use elohim_sdk::Syncable;
/// use automerge::AutoCommit;
///
/// impl Syncable for UserProgress {
///     fn sync_doc_type() -> &'static str { "progress" }
///
///     fn to_automerge(&self) -> Result<AutoCommit> {
///         // Convert to Automerge document
///         let mut doc = AutoCommit::new();
///         doc.put(automerge::ROOT, "mastery_level", self.mastery_level)?;
///         Ok(doc)
///     }
///
///     fn from_automerge(doc: &AutoCommit) -> Result<Self> {
///         // Parse from Automerge document
///         let level = doc.get(automerge::ROOT, "mastery_level")?;
///         Ok(Self { mastery_level: level })
///     }
/// }
/// ```
pub trait Syncable: Sized {
    /// Document type for sync API (e.g., "progress", "settings")
    fn sync_doc_type() -> &'static str;

    /// Document ID for this instance
    fn sync_doc_id(&self) -> String;

    /// Convert to Automerge document bytes
    #[cfg(feature = "sync")]
    fn to_automerge(&self) -> Result<automerge::AutoCommit>;

    /// Parse from Automerge document
    #[cfg(feature = "sync")]
    fn from_automerge(doc: &automerge::AutoCommit) -> Result<Self>;

    /// Merge with changes from another document
    #[cfg(feature = "sync")]
    fn merge(&mut self, changes: &[u8]) -> Result<()> {
        // Default implementation: ignore changes
        // Override for custom merge behavior
        let _ = changes;
        Ok(())
    }
}

/// Sync state for tracking document versions
#[derive(Debug, Clone, Default)]
pub struct SyncState {
    /// Current document heads (commit hashes)
    pub heads: Vec<String>,
    /// Last sync timestamp (ms since epoch)
    pub last_sync: u64,
    /// Whether there are local changes to push
    pub has_local_changes: bool,
}

impl SyncState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_heads(mut self, heads: Vec<String>) -> Self {
        self.heads = heads;
        self
    }

    pub fn mark_changed(&mut self) {
        self.has_local_changes = true;
    }

    pub fn mark_synced(&mut self, new_heads: Vec<String>) {
        self.heads = new_heads;
        self.last_sync = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.has_local_changes = false;
    }
}
