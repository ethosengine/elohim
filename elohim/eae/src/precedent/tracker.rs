//! Precedent tracking for learning from past decisions.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use constitution::ConstitutionalLayer;
use crate::types::{AnalysisEventType, Decision, DecisionType, Result};

/// A stored precedent from a past decision.
#[derive(Debug, Clone)]
pub struct Precedent {
    /// Unique precedent ID
    pub id: String,
    /// Original decision ID
    pub decision_id: String,
    /// Event type that triggered this
    pub event_type: AnalysisEventType,
    /// Decision that was made
    pub decision_type: DecisionType,
    /// Constitutional layer
    pub layer: ConstitutionalLayer,
    /// Key factors in the decision
    pub factors: Vec<String>,
    /// Outcome assessment (if available)
    pub outcome: Option<PrecedentOutcome>,
    /// When the decision was made
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Relevance score for future matching
    pub relevance_score: f32,
}

impl Precedent {
    /// Create a new precedent from a decision.
    pub fn from_decision(decision: &Decision, event_type: AnalysisEventType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            decision_id: decision.id.clone(),
            event_type,
            decision_type: decision.decision_type,
            layer: decision.reasoning.determining_layer,
            factors: decision.reasoning.matched_rules.clone(),
            outcome: None,
            created_at: decision.timestamp,
            relevance_score: decision.confidence,
        }
    }
}

/// Outcome assessment for a precedent.
#[derive(Debug, Clone)]
pub struct PrecedentOutcome {
    /// Was the decision successful?
    pub successful: bool,
    /// Feedback received
    pub feedback: Option<String>,
    /// When outcome was assessed
    pub assessed_at: chrono::DateTime<chrono::Utc>,
}

/// A match against stored precedents.
#[derive(Debug, Clone)]
pub struct PrecedentMatch {
    /// Matched precedent
    pub precedent: Precedent,
    /// Similarity score (0.0 - 1.0)
    pub similarity: f32,
    /// Matching factors
    pub matching_factors: Vec<String>,
}

/// Tracks and retrieves precedents for decision-making.
pub struct PrecedentTracker {
    /// Stored precedents
    precedents: Arc<RwLock<Vec<Precedent>>>,
    /// Index by event type for faster lookup
    by_event_type: Arc<RwLock<HashMap<AnalysisEventType, Vec<String>>>>,
    /// Index by layer
    by_layer: Arc<RwLock<HashMap<ConstitutionalLayer, Vec<String>>>>,
    /// Maximum precedents to store
    max_precedents: usize,
}

impl PrecedentTracker {
    /// Create a new tracker.
    pub fn new() -> Self {
        Self {
            precedents: Arc::new(RwLock::new(Vec::new())),
            by_event_type: Arc::new(RwLock::new(HashMap::new())),
            by_layer: Arc::new(RwLock::new(HashMap::new())),
            max_precedents: 10_000,
        }
    }

    /// Create with custom limit.
    pub fn with_max_precedents(max: usize) -> Self {
        Self {
            precedents: Arc::new(RwLock::new(Vec::new())),
            by_event_type: Arc::new(RwLock::new(HashMap::new())),
            by_layer: Arc::new(RwLock::new(HashMap::new())),
            max_precedents: max,
        }
    }

    /// Store a new precedent.
    pub async fn store(&self, precedent: Precedent) {
        let precedent_id = precedent.id.clone();
        let event_type = precedent.event_type;
        let layer = precedent.layer;

        info!(
            precedent_id = %precedent_id,
            event_type = ?event_type,
            layer = %layer.as_str(),
            "Storing precedent"
        );

        // Store precedent
        {
            let mut precedents = self.precedents.write().await;
            precedents.push(precedent);

            // Prune old precedents if over limit
            while precedents.len() > self.max_precedents {
                let removed = precedents.remove(0);
                // Clean up indices
                self.remove_from_indices(&removed.id, removed.event_type, removed.layer)
                    .await;
            }
        }

        // Update indices
        {
            let mut by_event = self.by_event_type.write().await;
            by_event
                .entry(event_type)
                .or_insert_with(Vec::new)
                .push(precedent_id.clone());
        }

        {
            let mut by_layer = self.by_layer.write().await;
            by_layer
                .entry(layer)
                .or_insert_with(Vec::new)
                .push(precedent_id);
        }
    }

    /// Remove a precedent from indices.
    async fn remove_from_indices(
        &self,
        id: &str,
        event_type: AnalysisEventType,
        layer: ConstitutionalLayer,
    ) {
        {
            let mut by_event = self.by_event_type.write().await;
            if let Some(ids) = by_event.get_mut(&event_type) {
                ids.retain(|i: &String| i != id);
            }
        }

        {
            let mut by_layer = self.by_layer.write().await;
            if let Some(ids) = by_layer.get_mut(&layer) {
                ids.retain(|i: &String| i != id);
            }
        }
    }

    /// Store from a decision.
    pub async fn store_from_decision(&self, decision: &Decision, event_type: AnalysisEventType) {
        let precedent = Precedent::from_decision(decision, event_type);
        self.store(precedent).await;
    }

    /// Find relevant precedents for an event type.
    pub async fn find_relevant(
        &self,
        event_type: AnalysisEventType,
        layer: ConstitutionalLayer,
        limit: usize,
    ) -> Vec<PrecedentMatch> {
        let precedents = self.precedents.read().await;
        let by_event = self.by_event_type.read().await;

        // Get precedent IDs for this event type
        let event_ids = by_event.get(&event_type).cloned().unwrap_or_default();

        // Score and rank precedents
        let mut matches: Vec<PrecedentMatch> = event_ids
            .iter()
            .filter_map(|id| {
                precedents.iter().find(|p| &p.id == id).map(|p| {
                    let similarity = self.calculate_similarity(p, layer);
                    PrecedentMatch {
                        precedent: p.clone(),
                        similarity,
                        matching_factors: p.factors.clone(),
                    }
                })
            })
            .collect();

        // Sort by similarity (descending)
        matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

        // Return top matches
        matches.into_iter().take(limit).collect()
    }

    /// Calculate similarity between precedent and current context.
    fn calculate_similarity(&self, precedent: &Precedent, layer: ConstitutionalLayer) -> f32 {
        let mut score = precedent.relevance_score;

        // Layer match bonus
        if precedent.layer == layer {
            score += 0.2;
        } else if (precedent.layer as i32 - layer as i32).abs() == 1 {
            // Adjacent layer
            score += 0.1;
        }

        // Successful outcome bonus
        if let Some(outcome) = &precedent.outcome {
            if outcome.successful {
                score += 0.1;
            }
        }

        // Recency bonus (newer is better)
        let age_days = (chrono::Utc::now() - precedent.created_at).num_days();
        if age_days < 7 {
            score += 0.1;
        } else if age_days < 30 {
            score += 0.05;
        }

        score.min(1.0)
    }

    /// Update precedent outcome.
    pub async fn update_outcome(
        &self,
        precedent_id: &str,
        successful: bool,
        feedback: Option<String>,
    ) -> Result<()> {
        let mut precedents = self.precedents.write().await;

        if let Some(precedent) = precedents.iter_mut().find(|p| p.id == precedent_id) {
            precedent.outcome = Some(PrecedentOutcome {
                successful,
                feedback,
                assessed_at: chrono::Utc::now(),
            });

            // Adjust relevance based on outcome
            if successful {
                precedent.relevance_score = (precedent.relevance_score + 0.1).min(1.0);
            } else {
                precedent.relevance_score = (precedent.relevance_score - 0.1).max(0.0);
            }

            debug!(
                precedent_id = %precedent_id,
                successful = successful,
                new_relevance = precedent.relevance_score,
                "Precedent outcome updated"
            );

            Ok(())
        } else {
            Err(crate::types::EaeError::ConfigError(format!(
                "Precedent {} not found",
                precedent_id
            )))
        }
    }

    /// Get statistics.
    pub async fn stats(&self) -> PrecedentStats {
        let precedents = self.precedents.read().await;

        let total = precedents.len();
        let with_outcome = precedents.iter().filter(|p| p.outcome.is_some()).count();
        let successful = precedents
            .iter()
            .filter(|p| p.outcome.as_ref().map(|o| o.successful).unwrap_or(false))
            .count();

        PrecedentStats {
            total_precedents: total,
            with_outcome,
            successful,
            success_rate: if with_outcome > 0 {
                successful as f32 / with_outcome as f32
            } else {
                0.0
            },
        }
    }

    /// Clear all precedents.
    pub async fn clear(&self) {
        let mut precedents = self.precedents.write().await;
        precedents.clear();

        let mut by_event = self.by_event_type.write().await;
        by_event.clear();

        let mut by_layer = self.by_layer.write().await;
        by_layer.clear();
    }
}

impl Default for PrecedentTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about stored precedents.
#[derive(Debug, Clone)]
pub struct PrecedentStats {
    /// Total precedents stored
    pub total_precedents: usize,
    /// Precedents with outcome assessment
    pub with_outcome: usize,
    /// Successful outcomes
    pub successful: usize,
    /// Success rate
    pub success_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, ActionType, DecisionReasoning};

    fn make_decision(layer: ConstitutionalLayer) -> Decision {
        Decision {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            event_id: "event".to_string(),
            decision_type: DecisionType::Allow,
            actions: vec![],
            reasoning: DecisionReasoning {
                primary_principle: "Test".to_string(),
                interpretation: "Test".to_string(),
                matched_rules: vec!["rule-1".to_string()],
                llm_assisted: false,
                precedents_considered: vec![],
                determining_layer: layer,
                stack_hash: "".to_string(),
            },
            confidence: 0.8,
            requires_consensus: false,
            consensus_status: None,
        }
    }

    #[tokio::test]
    async fn test_store_and_find() {
        let tracker = PrecedentTracker::new();

        let decision = make_decision(ConstitutionalLayer::Individual);
        tracker
            .store_from_decision(&decision, AnalysisEventType::RoutineCheck)
            .await;

        let matches = tracker
            .find_relevant(
                AnalysisEventType::RoutineCheck,
                ConstitutionalLayer::Individual,
                10,
            )
            .await;

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].precedent.decision_id, decision.id);
    }

    #[tokio::test]
    async fn test_outcome_update() {
        let tracker = PrecedentTracker::new();

        let decision = make_decision(ConstitutionalLayer::Individual);
        tracker
            .store_from_decision(&decision, AnalysisEventType::RoutineCheck)
            .await;

        let matches = tracker
            .find_relevant(
                AnalysisEventType::RoutineCheck,
                ConstitutionalLayer::Individual,
                1,
            )
            .await;

        tracker
            .update_outcome(&matches[0].precedent.id, true, Some("Good decision".to_string()))
            .await
            .unwrap();

        let stats = tracker.stats().await;
        assert_eq!(stats.successful, 1);
        assert_eq!(stats.success_rate, 1.0);
    }
}
