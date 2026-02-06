//! Precedent storage and retrieval.
//!
//! Manages constitutional precedents - past decisions that inform future ones.
//! Uses an in-memory store with optional persistence via Diesel.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::*;

/// Error types for precedent operations.
#[derive(Debug, thiserror::Error)]
pub enum PrecedentError {
    /// Precedent not found
    #[error("Precedent not found: {0}")]
    NotFound(String),

    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Invalid precedent data
    #[error("Invalid precedent: {0}")]
    InvalidPrecedent(String),
}

/// In-memory precedent store with optional persistence.
///
/// This provides a clean abstraction over precedent storage,
/// allowing for both in-memory operation and database persistence.
pub struct PrecedentStore {
    /// In-memory cache of precedents
    cache: Arc<RwLock<HashMap<String, Precedent>>>,
    /// Index by layer for efficient retrieval
    by_layer: Arc<RwLock<HashMap<ConstitutionalLayer, Vec<String>>>>,
    /// Index by principle for efficient retrieval
    by_principle: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl PrecedentStore {
    /// Create a new empty precedent store.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            by_layer: Arc::new(RwLock::new(HashMap::new())),
            by_principle: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a new precedent.
    pub async fn store(&self, precedent: Precedent) -> Result<(), PrecedentError> {
        let id = precedent.id.clone();
        let layer = precedent.layer;
        let principles = precedent.principles_applied.clone();

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(id.clone(), precedent);
        }

        // Update layer index
        {
            let mut by_layer = self.by_layer.write().await;
            by_layer.entry(layer).or_default().push(id.clone());
        }

        // Update principle index
        {
            let mut by_principle = self.by_principle.write().await;
            for principle_id in principles {
                by_principle
                    .entry(principle_id)
                    .or_default()
                    .push(id.clone());
            }
        }

        tracing::debug!(precedent_id = %id, "Stored precedent");
        Ok(())
    }

    /// Get a precedent by ID.
    pub async fn get(&self, id: &str) -> Result<Precedent, PrecedentError> {
        let cache = self.cache.read().await;
        cache
            .get(id)
            .cloned()
            .ok_or_else(|| PrecedentError::NotFound(id.to_string()))
    }

    /// Get all precedents for a layer.
    pub async fn get_by_layer(&self, layer: ConstitutionalLayer) -> Vec<Precedent> {
        let by_layer = self.by_layer.read().await;
        let cache = self.cache.read().await;

        by_layer
            .get(&layer)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| cache.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all precedents that applied a specific principle.
    pub async fn get_by_principle(&self, principle_id: &str) -> Vec<Precedent> {
        let by_principle = self.by_principle.read().await;
        let cache = self.cache.read().await;

        by_principle
            .get(principle_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| cache.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search for relevant precedents given a query.
    ///
    /// Currently uses simple keyword matching. Could be enhanced with
    /// semantic search using embeddings.
    pub async fn search(&self, query: &str, limit: usize) -> Vec<Precedent> {
        let cache = self.cache.read().await;
        let query_lower = query.to_lowercase();

        let mut matches: Vec<(Precedent, f32)> = cache
            .values()
            .filter_map(|p| {
                let score = self.relevance_score(p, &query_lower);
                if score > 0.0 {
                    Some((p.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by relevance score (descending)
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        matches.into_iter().take(limit).map(|(p, _)| p).collect()
    }

    /// Get precedents ordered by weight (most influential first).
    pub async fn get_top_precedents(&self, limit: usize) -> Vec<Precedent> {
        let cache = self.cache.read().await;

        let mut precedents: Vec<Precedent> = cache.values().cloned().collect();
        precedents.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        precedents.into_iter().take(limit).collect()
    }

    /// Increment citation count for a precedent.
    pub async fn cite(&self, id: &str) -> Result<(), PrecedentError> {
        let mut cache = self.cache.write().await;

        if let Some(precedent) = cache.get_mut(id) {
            precedent.citation_count += 1;
            // Increase weight slightly with citations (diminishing returns)
            precedent.weight = (precedent.weight + 0.01).min(1.0);
            Ok(())
        } else {
            Err(PrecedentError::NotFound(id.to_string()))
        }
    }

    /// Get total count of stored precedents.
    pub async fn count(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Calculate relevance score for a precedent given a query.
    fn relevance_score(&self, precedent: &Precedent, query: &str) -> f32 {
        let mut score = 0.0;

        // Check case summary
        if precedent.case_summary.to_lowercase().contains(query) {
            score += 0.5;
        }

        // Check reasoning
        if precedent.reasoning.to_lowercase().contains(query) {
            score += 0.3;
        }

        // Boost by weight
        score *= precedent.weight;

        // Boost by citations
        score *= 1.0 + (precedent.citation_count as f32 * 0.01);

        score
    }
}

impl Default for PrecedentStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating precedents.
pub struct PrecedentBuilder {
    layer: ConstitutionalLayer,
    case_summary: String,
    principles_applied: Vec<String>,
    reasoning: String,
    outcome: Option<PrecedentOutcome>,
    weight: f32,
}

impl PrecedentBuilder {
    /// Create a new precedent builder.
    pub fn new(layer: ConstitutionalLayer) -> Self {
        Self {
            layer,
            case_summary: String::new(),
            principles_applied: Vec::new(),
            reasoning: String::new(),
            outcome: None,
            weight: 0.5,
        }
    }

    /// Set the case summary.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.case_summary = summary.into();
        self
    }

    /// Add a principle that was applied.
    pub fn applied_principle(mut self, principle_id: impl Into<String>) -> Self {
        self.principles_applied.push(principle_id.into());
        self
    }

    /// Set the reasoning.
    pub fn reasoning(mut self, reasoning: impl Into<String>) -> Self {
        self.reasoning = reasoning.into();
        self
    }

    /// Set the outcome.
    pub fn outcome(mut self, outcome: PrecedentOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// Set the weight.
    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Build the precedent.
    pub fn build(self) -> Result<Precedent, PrecedentError> {
        let outcome = self
            .outcome
            .ok_or_else(|| PrecedentError::InvalidPrecedent("Outcome is required".to_string()))?;

        if self.case_summary.is_empty() {
            return Err(PrecedentError::InvalidPrecedent(
                "Case summary is required".to_string(),
            ));
        }

        Ok(Precedent {
            id: uuid::Uuid::new_v4().to_string(),
            layer: self.layer,
            case_summary: self.case_summary,
            principles_applied: self.principles_applied,
            reasoning: self.reasoning,
            outcome,
            created_at: chrono::Utc::now(),
            weight: self.weight,
            citation_count: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_precedent(store: &PrecedentStore, summary: &str) -> String {
        let precedent = PrecedentBuilder::new(ConstitutionalLayer::Community)
            .summary(summary)
            .applied_principle("global-dignity")
            .reasoning("Test reasoning")
            .outcome(PrecedentOutcome::Approved {
                conditions: vec![],
            })
            .weight(0.7)
            .build()
            .unwrap();

        let id = precedent.id.clone();
        store.store(precedent).await.unwrap();
        id
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = PrecedentStore::new();
        let id = create_test_precedent(&store, "Test case").await;

        let retrieved = store.get(&id).await.unwrap();
        assert_eq!(retrieved.case_summary, "Test case");
    }

    #[tokio::test]
    async fn test_get_by_layer() {
        let store = PrecedentStore::new();
        create_test_precedent(&store, "Community case 1").await;
        create_test_precedent(&store, "Community case 2").await;

        let community_precedents = store.get_by_layer(ConstitutionalLayer::Community).await;
        assert_eq!(community_precedents.len(), 2);

        let global_precedents = store.get_by_layer(ConstitutionalLayer::Global).await;
        assert_eq!(global_precedents.len(), 0);
    }

    #[tokio::test]
    async fn test_search() {
        let store = PrecedentStore::new();
        create_test_precedent(&store, "Privacy violation case").await;
        create_test_precedent(&store, "Content moderation case").await;

        let results = store.search("privacy", 10).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].case_summary.contains("Privacy"));
    }

    #[tokio::test]
    async fn test_citation() {
        let store = PrecedentStore::new();
        let id = create_test_precedent(&store, "Important case").await;

        let before = store.get(&id).await.unwrap();
        assert_eq!(before.citation_count, 0);

        store.cite(&id).await.unwrap();
        store.cite(&id).await.unwrap();

        let after = store.get(&id).await.unwrap();
        assert_eq!(after.citation_count, 2);
    }
}
