//! Projection Engine - type-agnostic signal processing
//!
//! The engine receives generic projection signals from DNA and stores
//! opaque data in MongoDB projections. Doorway does NOT interpret signal
//! content - it just stores what DNA tells it to store.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Projection Engine                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌──────────────────┐    ┌──────────────────────────┐      │
//! │  │ Signal Subscriber│───▶│  Generic Handler         │      │
//! │  │ (WS to Conductor)│    │  (type-agnostic)         │      │
//! │  └──────────────────┘    └──────────┬───────────────┘      │
//! │                                     │                       │
//! │                          ┌──────────▼───────────────┐      │
//! │                          │   ProjectionStore        │      │
//! │                          │   (Hot Cache + MongoDB)  │      │
//! │                          └──────────────────────────┘      │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Type-Agnostic Design
//!
//! Doorway does NOT define content types - the Holochain DNA does.
//! Signals include explicit metadata (search_tokens, invalidates, ttl)
//! so doorway can process any type without parsing the data field.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::types::DoorwayError;

use super::document::ProjectedDocument;
use super::store::ProjectionStore;

/// Generic projection signal from DNA post_commit.
///
/// Doorway is type-agnostic - it stores whatever DNA sends without
/// parsing the `data` field. DNA provides explicit metadata for:
/// - Search indexing (search_tokens)
/// - Cache invalidation (invalidates)
/// - TTL/expiry (ttl_secs)
///
/// ## Special Actions
///
/// - `"commit"` / `"update"`: Store/update a document
/// - `"delete"`: Soft-delete a document
/// - `"update_endpoints"`: Update blob_endpoints for documents with matching blob_hash
///   (used by ContentServerCommitted signals from infrastructure DNA)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionSignal {
    /// Document type (e.g., "Content", "LearningPath", "MyCustomType")
    /// For "update_endpoints" action, this is ignored.
    pub doc_type: String,
    /// Action ("commit", "delete", "update", "update_endpoints")
    pub action: String,
    /// Document ID, or blob_hash for "update_endpoints" action
    pub id: String,
    /// Opaque data - doorway never parses this
    /// For "update_endpoints", this should be a JSON array of endpoint URLs
    pub data: JsonValue,
    /// Holochain action hash
    pub action_hash: String,
    /// Entry hash (optional for deletes)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_hash: Option<String>,
    /// Author agent pub key
    pub author: String,

    // Explicit metadata from DNA (doorway applies but doesn't compute)
    /// Search tokens (DNA computes these from title, description, tags, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub search_tokens: Vec<String>,
    /// Cache keys to invalidate (e.g., ["LearningPath:governance-intro"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invalidates: Vec<String>,
    /// TTL in seconds (None = no expiry)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u64>,
}

/// Projection Engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Buffer size for signal processing
    pub buffer_size: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self { buffer_size: 1000 }
    }
}

/// Projection Engine - processes DNA signals and updates projections
pub struct ProjectionEngine {
    /// Projection store for persisting projections
    store: Arc<ProjectionStore>,
    /// Configuration
    config: EngineConfig,
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
}

impl ProjectionEngine {
    /// Create a new projection engine
    pub fn new(store: Arc<ProjectionStore>, config: EngineConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            store,
            config,
            shutdown_tx,
        }
    }

    /// Get a shutdown receiver for graceful termination
    pub fn shutdown_receiver(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Signal shutdown to the engine
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Process a single projection signal.
    ///
    /// This is type-agnostic - doorway stores whatever DNA sends without
    /// parsing the data field. DNA provides explicit metadata for search
    /// tokens and cache invalidation.
    pub async fn process_signal(&self, signal: ProjectionSignal) -> Result<(), DoorwayError> {
        info!(
            doc_type = signal.doc_type,
            action = signal.action,
            id = signal.id,
            "Processing projection signal"
        );

        match signal.action.as_str() {
            "commit" | "update" => {
                // Store opaque data with DNA-provided metadata
                let mut doc = ProjectedDocument::new(
                    &signal.doc_type,
                    &signal.id,
                    &signal.action_hash,
                    &signal.author,
                    signal.data,
                );

                if let Some(ref entry_hash) = signal.entry_hash {
                    doc = doc.with_entry_hash(entry_hash);
                }

                if !signal.search_tokens.is_empty() {
                    doc = doc.with_search_tokens(signal.search_tokens.clone());
                }

                self.store.set(doc).await?;
                debug!(
                    doc_type = signal.doc_type,
                    id = signal.id,
                    "Document projected"
                );
            }
            "delete" => {
                // Delete by invalidating the cache entry
                let pattern = format!("{}:{}", signal.doc_type, signal.id);
                self.store.invalidate(&pattern).await?;
                debug!(
                    doc_type = signal.doc_type,
                    id = signal.id,
                    "Document deleted (via invalidation)"
                );
            }
            "update_endpoints" => {
                // Update blob_endpoints for documents with matching blob_hash
                // signal.id = blob_hash, signal.data = JSON array of endpoint URLs
                let blob_hash = &signal.id;
                let endpoints: Vec<String> = signal
                    .data
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                if !endpoints.is_empty() {
                    let count = self.store.update_blob_endpoints(blob_hash, endpoints.clone()).await?;
                    debug!(
                        blob_hash = blob_hash,
                        endpoints = ?endpoints,
                        updated_count = count,
                        "Updated blob endpoints in projection store"
                    );
                }
            }
            other => {
                debug!(action = other, "Unknown signal action, ignoring");
            }
        }

        // Apply cache invalidations from DNA
        for pattern in &signal.invalidates {
            if let Err(e) = self.store.invalidate(pattern).await {
                warn!(pattern = pattern, error = ?e, "Cache invalidation failed");
            }
        }

        Ok(())
    }
}

/// Start the projection engine with a signal receiver
///
/// This spawns a task that listens for signals and processes them.
pub fn spawn_engine_task(
    engine: Arc<ProjectionEngine>,
    mut signal_rx: broadcast::Receiver<ProjectionSignal>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut shutdown_rx = engine.shutdown_receiver();

        info!("Projection engine started");

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("Projection engine shutting down");
                    break;
                }
                // Process incoming signals
                signal = signal_rx.recv() => {
                    match signal {
                        Ok(sig) => {
                            if let Err(e) = engine.process_signal(sig).await {
                                error!("Error processing projection signal: {}", e);
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("Projection engine lagged {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("Signal channel closed, engine stopping");
                            break;
                        }
                    }
                }
            }
        }

        info!("Projection engine stopped");
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_serialization() {
        let signal = ProjectionSignal {
            doc_type: "Content".to_string(),
            action: "commit".to_string(),
            id: "manifesto".to_string(),
            data: serde_json::json!({
                "title": "The Elohim Protocol",
                "description": "A manifesto for decentralized governance"
            }),
            action_hash: "uhCkk...".to_string(),
            entry_hash: Some("uhCEk...".to_string()),
            author: "uhCAk...".to_string(),
            search_tokens: vec!["elohim".to_string(), "protocol".to_string(), "governance".to_string()],
            invalidates: vec![],
            ttl_secs: None,
        };

        let json = serde_json::to_string(&signal).unwrap();
        assert!(json.contains("Content"));
        assert!(json.contains("manifesto"));
        assert!(json.contains("governance"));
    }

    #[test]
    fn test_signal_deserialization() {
        let json = r#"{
            "doc_type": "LearningPath",
            "action": "commit",
            "id": "governance-intro",
            "data": {"title": "Introduction to Governance"},
            "action_hash": "uhCkk...",
            "author": "uhCAk...",
            "search_tokens": ["governance", "intro"],
            "invalidates": ["Content:manifesto"]
        }"#;

        let signal: ProjectionSignal = serde_json::from_str(json).unwrap();
        assert_eq!(signal.doc_type, "LearningPath");
        assert_eq!(signal.action, "commit");
        assert_eq!(signal.id, "governance-intro");
        assert_eq!(signal.search_tokens.len(), 2);
        assert_eq!(signal.invalidates.len(), 1);
        assert!(signal.entry_hash.is_none()); // Optional field
        assert!(signal.ttl_secs.is_none()); // Optional field
    }

    #[test]
    fn test_signal_minimal() {
        // Test minimal signal with only required fields
        let json = r#"{
            "doc_type": "CustomType",
            "action": "delete",
            "id": "some-id",
            "data": null,
            "action_hash": "uhCkk...",
            "author": "uhCAk..."
        }"#;

        let signal: ProjectionSignal = serde_json::from_str(json).unwrap();
        assert_eq!(signal.doc_type, "CustomType");
        assert_eq!(signal.action, "delete");
        assert!(signal.search_tokens.is_empty());
        assert!(signal.invalidates.is_empty());
    }

    #[test]
    fn test_config_default() {
        let config = EngineConfig::default();
        assert_eq!(config.buffer_size, 1000);
    }
}
