//! Projection Engine - transforms DHT signals into cached projections
//!
//! The engine subscribes to Holochain conductor signals and transforms
//! committed entries into MongoDB projections for fast reads.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Projection Engine                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌──────────────────┐    ┌──────────────────────────┐      │
//! │  │ Signal Subscriber│───▶│  Transformers            │      │
//! │  │ (WS to Conductor)│    │  - ContentTransformer    │      │
//! │  │                  │    │  - PathTransformer       │      │
//! │  └──────────────────┘    │  - RelationshipTransform │      │
//! │                          └──────────┬───────────────┘      │
//! │                                     │                       │
//! │                          ┌──────────▼───────────────┐      │
//! │                          │   ProjectionStore        │      │
//! │                          │   (Hot Cache + MongoDB)  │      │
//! │                          └──────────────────────────┘      │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::types::DoorwayError;

use super::document::ProjectedDocument;
use super::store::ProjectionStore;
use super::collections::{ContentProjection, PathProjection};

/// Signal types from the DNA post_commit handler.
///
/// These mirror the `ProjectionSignal` enum in the content_store zome.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ProjectionSignal {
    /// Content entry was created or updated
    ContentCommitted {
        action_hash: String,
        entry_hash: String,
        content: ContentSignalData,
        author: String,
    },
    /// LearningPath was created or updated
    PathCommitted {
        action_hash: String,
        entry_hash: String,
        path: PathSignalData,
        author: String,
    },
    /// PathStep was created or updated
    StepCommitted {
        action_hash: String,
        entry_hash: String,
        step: StepSignalData,
        author: String,
    },
    /// PathChapter was created or updated
    ChapterCommitted {
        action_hash: String,
        entry_hash: String,
        chapter: ChapterSignalData,
        author: String,
    },
    /// Relationship was created
    RelationshipCommitted {
        action_hash: String,
        entry_hash: String,
        relationship: RelationshipSignalData,
        author: String,
    },
    /// Human (agent profile) was created or updated
    HumanCommitted {
        action_hash: String,
        entry_hash: String,
        human: JsonValue,
        author: String,
    },
    /// Agent was created or updated
    AgentCommitted {
        action_hash: String,
        entry_hash: String,
        agent: JsonValue,
        author: String,
    },
    /// ContributorPresence was created or updated
    PresenceCommitted {
        action_hash: String,
        entry_hash: String,
        presence: JsonValue,
        author: String,
    },
    /// Generic entry committed
    EntryCommitted {
        action_hash: String,
        entry_hash: String,
        entry_type: String,
        author: String,
    },
}

/// Content data from DNA signal (matches Content entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSignalData {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    pub summary: Option<String>,
    pub content: String,
    pub content_format: String,
    pub tags: Vec<String>,
    pub source_path: Option<String>,
    pub related_node_ids: Vec<String>,
    pub author_id: Option<String>,
    pub reach: String,
    pub trust_score: f64,
    pub estimated_minutes: Option<u32>,
    pub thumbnail_url: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// LearningPath data from DNA signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSignalData {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub purpose: Option<String>,
    pub created_by: String,
    pub difficulty: String,
    pub estimated_duration: Option<String>,
    pub visibility: String,
    pub path_type: String,
    pub tags: Vec<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

/// PathStep data from DNA signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepSignalData {
    pub id: String,
    pub path_id: String,
    pub chapter_id: Option<String>,
    pub order_index: u32,
    pub step_type: String,
    pub resource_id: String,
    pub step_title: Option<String>,
    pub step_narrative: Option<String>,
    pub is_optional: bool,
    pub estimated_minutes: Option<u32>,
    pub created_at: String,
    pub updated_at: String,
}

/// PathChapter data from DNA signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSignalData {
    pub id: String,
    pub path_id: String,
    pub order_index: u32,
    pub title: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Relationship data from DNA signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSignalData {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub confidence: f64,
    pub inference_source: String,
    pub metadata_json: Option<String>,
    pub created_at: String,
}

/// Projection Engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Whether to process content signals
    pub process_content: bool,
    /// Whether to process path signals
    pub process_paths: bool,
    /// Whether to process relationship signals
    pub process_relationships: bool,
    /// Buffer size for signal processing
    pub buffer_size: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            process_content: true,
            process_paths: true,
            process_relationships: true,
            buffer_size: 1000,
        }
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

    /// Process a single projection signal
    pub async fn process_signal(&self, signal: ProjectionSignal) -> Result<(), DoorwayError> {
        match signal {
            ProjectionSignal::ContentCommitted {
                action_hash,
                entry_hash,
                content,
                author,
            } => {
                if self.config.process_content {
                    self.process_content(action_hash, entry_hash, content, author).await?;
                }
            }
            ProjectionSignal::PathCommitted {
                action_hash,
                entry_hash,
                path,
                author,
            } => {
                if self.config.process_paths {
                    self.process_path(action_hash, entry_hash, path, author).await?;
                }
            }
            ProjectionSignal::StepCommitted {
                action_hash,
                entry_hash,
                step,
                author,
            } => {
                if self.config.process_paths {
                    self.process_step(action_hash, entry_hash, step, author).await?;
                }
            }
            ProjectionSignal::ChapterCommitted {
                action_hash,
                entry_hash,
                chapter,
                author,
            } => {
                if self.config.process_paths {
                    self.process_chapter(action_hash, entry_hash, chapter, author).await?;
                }
            }
            ProjectionSignal::RelationshipCommitted {
                action_hash,
                entry_hash,
                relationship,
                author,
            } => {
                if self.config.process_relationships {
                    self.process_relationship(action_hash, entry_hash, relationship, author).await?;
                }
            }
            ProjectionSignal::HumanCommitted { action_hash, .. } => {
                debug!("Human committed (action: {}), skipping projection", action_hash);
            }
            ProjectionSignal::AgentCommitted { action_hash, .. } => {
                debug!("Agent committed (action: {}), skipping projection", action_hash);
            }
            ProjectionSignal::PresenceCommitted { action_hash, .. } => {
                debug!("Presence committed (action: {}), skipping projection", action_hash);
            }
            ProjectionSignal::EntryCommitted { action_hash, entry_type, .. } => {
                debug!("Generic entry committed: {} (action: {})", entry_type, action_hash);
            }
        }

        Ok(())
    }

    /// Process a content signal into a projection
    async fn process_content(
        &self,
        action_hash: String,
        entry_hash: String,
        content: ContentSignalData,
        author: String,
    ) -> Result<(), DoorwayError> {
        info!("Projecting content: {} ({})", content.title, content.id);

        // Convert to JSON for the projection
        let data = serde_json::to_value(&content)
            .map_err(|e| DoorwayError::Projection(format!("Serialize content: {}", e)))?;

        // Create the projected document
        let doc = ProjectedDocument::new(
            "Content",
            &content.id,
            &action_hash,
            &author,
            data,
        )
        .with_entry_hash(&entry_hash)
        .with_search_tokens(Self::extract_content_tokens(&content));

        // Store in projection store
        self.store.set(doc).await?;

        debug!("Content projected: {}", content.id);
        Ok(())
    }

    /// Process a path signal into a projection
    async fn process_path(
        &self,
        action_hash: String,
        entry_hash: String,
        path: PathSignalData,
        author: String,
    ) -> Result<(), DoorwayError> {
        info!("Projecting path: {} ({})", path.title, path.id);

        // Convert to JSON for the projection
        let data = serde_json::to_value(&path)
            .map_err(|e| DoorwayError::Projection(format!("Serialize path: {}", e)))?;

        // Create the projected document
        // Note: step_count would need to be calculated separately
        let doc = ProjectedDocument::new(
            "LearningPath",
            &path.id,
            &action_hash,
            &author,
            data,
        )
        .with_entry_hash(&entry_hash)
        .with_search_tokens(Self::extract_path_tokens(&path));

        // Store in projection store
        self.store.set(doc).await?;

        debug!("Path projected: {}", path.id);
        Ok(())
    }

    /// Process a step signal into a projection
    async fn process_step(
        &self,
        action_hash: String,
        entry_hash: String,
        step: StepSignalData,
        author: String,
    ) -> Result<(), DoorwayError> {
        debug!("Projecting step: {} (path: {})", step.id, step.path_id);

        // Convert to JSON for the projection
        let data = serde_json::to_value(&step)
            .map_err(|e| DoorwayError::Projection(format!("Serialize step: {}", e)))?;

        // Create the projected document
        let doc = ProjectedDocument::new(
            "PathStep",
            &step.id,
            &action_hash,
            &author,
            data,
        )
        .with_entry_hash(&entry_hash);

        // Store in projection store
        self.store.set(doc).await?;

        // Also invalidate the parent path's cache so step count updates
        let pattern = format!("LearningPath:{}", step.path_id);
        let _ = self.store.invalidate(&pattern).await;

        debug!("Step projected: {}", step.id);
        Ok(())
    }

    /// Process a chapter signal into a projection
    async fn process_chapter(
        &self,
        action_hash: String,
        entry_hash: String,
        chapter: ChapterSignalData,
        author: String,
    ) -> Result<(), DoorwayError> {
        debug!("Projecting chapter: {} (path: {})", chapter.id, chapter.path_id);

        // Convert to JSON for the projection
        let data = serde_json::to_value(&chapter)
            .map_err(|e| DoorwayError::Projection(format!("Serialize chapter: {}", e)))?;

        // Create the projected document
        let doc = ProjectedDocument::new(
            "PathChapter",
            &chapter.id,
            &action_hash,
            &author,
            data,
        )
        .with_entry_hash(&entry_hash);

        // Store in projection store
        self.store.set(doc).await?;

        // Also invalidate the parent path's cache so chapter count updates
        let pattern = format!("LearningPath:{}", chapter.path_id);
        let _ = self.store.invalidate(&pattern).await;

        debug!("Chapter projected: {}", chapter.id);
        Ok(())
    }

    /// Process a relationship signal into a projection
    async fn process_relationship(
        &self,
        action_hash: String,
        entry_hash: String,
        relationship: RelationshipSignalData,
        author: String,
    ) -> Result<(), DoorwayError> {
        debug!(
            "Projecting relationship: {} ({} -> {})",
            relationship.id, relationship.source_id, relationship.target_id
        );

        // Convert to JSON for the projection
        let data = serde_json::to_value(&relationship)
            .map_err(|e| DoorwayError::Projection(format!("Serialize relationship: {}", e)))?;

        // Create the projected document
        let doc = ProjectedDocument::new(
            "Relationship",
            &relationship.id,
            &action_hash,
            &author,
            data,
        )
        .with_entry_hash(&entry_hash);

        // Store in projection store
        self.store.set(doc).await?;

        debug!("Relationship projected: {}", relationship.id);
        Ok(())
    }

    /// Extract search tokens from content
    fn extract_content_tokens(content: &ContentSignalData) -> Vec<String> {
        let mut tokens = Vec::new();

        // Add tokens from title and description
        tokens.extend(Self::tokenize(&content.title));
        tokens.extend(Self::tokenize(&content.description));

        // Add tags directly
        for tag in &content.tags {
            tokens.push(tag.to_lowercase());
        }

        // Add content type
        tokens.push(content.content_type.to_lowercase());

        tokens.sort();
        tokens.dedup();
        tokens
    }

    /// Extract search tokens from path
    fn extract_path_tokens(path: &PathSignalData) -> Vec<String> {
        let mut tokens = Vec::new();

        tokens.extend(Self::tokenize(&path.title));
        tokens.extend(Self::tokenize(&path.description));

        for tag in &path.tags {
            tokens.push(tag.to_lowercase());
        }

        tokens.push(path.difficulty.to_lowercase());

        tokens.sort();
        tokens.dedup();
        tokens
    }

    /// Tokenize text for search
    fn tokenize(text: &str) -> Vec<String> {
        text.split_whitespace()
            .filter(|word| word.len() >= 3)
            .map(|word| {
                word.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect()
            })
            .filter(|word: &String| !word.is_empty())
            .collect()
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
    fn test_tokenize() {
        let tokens = ProjectionEngine::tokenize("The quick brown fox");
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(!tokens.contains(&"the".to_string())); // Too short
    }

    #[test]
    fn test_content_tokens() {
        let content = ContentSignalData {
            id: "test".to_string(),
            content_type: "concept".to_string(),
            title: "Economic Flows".to_string(),
            description: "Understanding value in networks".to_string(),
            summary: None,
            content: "".to_string(),
            content_format: "markdown".to_string(),
            tags: vec!["economics".to_string(), "governance".to_string()],
            source_path: None,
            related_node_ids: Vec::new(),
            author_id: None,
            reach: "public".to_string(),
            trust_score: 1.0,
            estimated_minutes: None,
            thumbnail_url: None,
            metadata_json: "{}".to_string(),
            created_at: "".to_string(),
            updated_at: "".to_string(),
        };

        let tokens = ProjectionEngine::extract_content_tokens(&content);
        assert!(tokens.contains(&"economic".to_string()));
        assert!(tokens.contains(&"economics".to_string()));
        assert!(tokens.contains(&"concept".to_string()));
    }
}
