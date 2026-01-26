//! Constitutional stack assembly and management.
//!
//! The constitutional stack represents the full hierarchy of constitutional
//! documents applicable to a given context (agent, community, family, etc.).

use std::collections::HashMap;
use std::sync::Arc;

use crate::conflict::ConflictResolver;
use crate::layers::*;
use crate::types::*;
use crate::verification::{hash_document_content, DhtVerifier};

/// Error types for constitutional stack operations.
#[derive(Debug, thiserror::Error)]
pub enum ConstitutionError {
    /// Failed to verify a document
    #[error("Failed to verify document: {0}")]
    VerificationFailed(String),

    /// Document not found
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    /// Conflict resolution failed
    #[error("Conflict resolution failed: {0}")]
    ConflictResolutionFailed(String),

    /// Invalid layer configuration
    #[error("Invalid layer configuration: {0}")]
    InvalidConfiguration(String),
}

/// Context for building a constitutional stack.
///
/// Specifies which documents should be loaded for each layer.
#[derive(Debug, Clone, Default)]
pub struct StackContext {
    /// Agent ID (always required)
    pub agent_id: String,
    /// Optional family context
    pub family_id: Option<String>,
    /// Optional community context
    pub community_id: Option<String>,
    /// Optional provincial/regional context
    pub provincial_id: Option<String>,
    /// Optional nation context
    pub nation_id: Option<String>,
    /// Optional bioregion context
    pub bioregion_id: Option<String>,
}

impl StackContext {
    /// Create a minimal context with just an agent ID.
    pub fn agent_only(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            ..Default::default()
        }
    }

    /// Builder: set family ID.
    pub fn with_family(mut self, family_id: impl Into<String>) -> Self {
        self.family_id = Some(family_id.into());
        self
    }

    /// Builder: set community ID.
    pub fn with_community(mut self, community_id: impl Into<String>) -> Self {
        self.community_id = Some(community_id.into());
        self
    }
}

/// The assembled constitutional stack for a specific context.
///
/// Contains all applicable constitutional documents and resolved principles.
pub struct ConstitutionalStack {
    /// Documents by layer (multiple per layer possible)
    documents: HashMap<ConstitutionalLayer, Vec<ConstitutionalDocument>>,
    /// Resolved principles (after conflict resolution)
    resolved_principles: Vec<ResolvedPrinciple>,
    /// Active boundaries from all layers
    active_boundaries: Vec<Boundary>,
    /// Context this stack was built for
    context: StackContext,
    /// Hash of the entire stack for audit purposes
    stack_hash: String,
}

impl ConstitutionalStack {
    /// Build a constitutional stack for the given context.
    ///
    /// This loads default content for each layer, verifies against DHT
    /// where applicable, and resolves conflicts between layers.
    pub async fn build(
        context: StackContext,
        verifier: &dyn DhtVerifier,
        resolver: &ConflictResolver,
    ) -> Result<Self, ConstitutionError> {
        let mut documents: HashMap<ConstitutionalLayer, Vec<ConstitutionalDocument>> =
            HashMap::new();

        // Load default content for each layer
        let layers: Vec<Box<dyn LayerProvider>> = vec![
            Box::new(GlobalLayer),
            Box::new(BioregionalLayer),
            Box::new(NationalLayer),
            Box::new(CommunityLayer),
            Box::new(FamilyLayer),
            Box::new(IndividualLayer),
        ];

        for provider in layers {
            let layer = provider.layer();
            let content = provider.default_content();
            let hash = hash_document_content(&content);

            let doc = ConstitutionalDocument {
                id: format!("default-{}", layer.as_str().to_lowercase()),
                layer,
                version: "1.0.0".to_string(),
                hash: hash.clone(),
                content,
                created_at: chrono::Utc::now(),
                verified_at: None,
                context_id: None,
            };

            // Verify against DHT if available
            if verifier.is_available().await {
                if let Ok(result) = verifier.verify(&doc.id, &hash).await {
                    if !result.verified {
                        tracing::warn!(
                            doc_id = %doc.id,
                            "Document hash mismatch with DHT"
                        );
                    }
                }
            }

            documents.entry(layer).or_default().push(doc);
        }

        // Collect all principles and boundaries
        let mut all_principles: Vec<(Principle, ConstitutionalLayer)> = Vec::new();
        let mut all_boundaries: Vec<Boundary> = Vec::new();

        for (layer, docs) in &documents {
            for doc in docs {
                for principle in &doc.content.principles {
                    all_principles.push((principle.clone(), *layer));
                }
                all_boundaries.extend(doc.content.boundaries.clone());
            }
        }

        // Resolve conflicts
        let resolved_principles = resolver.resolve_principles(&all_principles);

        // Compute stack hash for audit
        let stack_hash = Self::compute_stack_hash(&documents);

        Ok(Self {
            documents,
            resolved_principles,
            active_boundaries: all_boundaries,
            context,
            stack_hash,
        })
    }

    /// Build a minimal stack with just default layers (no DHT verification).
    pub fn build_defaults(context: StackContext) -> Self {
        let mut documents: HashMap<ConstitutionalLayer, Vec<ConstitutionalDocument>> =
            HashMap::new();

        let layers: Vec<Box<dyn LayerProvider>> = vec![
            Box::new(GlobalLayer),
            Box::new(BioregionalLayer),
            Box::new(NationalLayer),
            Box::new(CommunityLayer),
            Box::new(FamilyLayer),
            Box::new(IndividualLayer),
        ];

        let mut all_principles: Vec<(Principle, ConstitutionalLayer)> = Vec::new();
        let mut all_boundaries: Vec<Boundary> = Vec::new();

        for provider in layers {
            let layer = provider.layer();
            let content = provider.default_content();
            let hash = hash_document_content(&content);

            for principle in &content.principles {
                all_principles.push((principle.clone(), layer));
            }
            all_boundaries.extend(content.boundaries.clone());

            let doc = ConstitutionalDocument {
                id: format!("default-{}", layer.as_str().to_lowercase()),
                layer,
                version: "1.0.0".to_string(),
                hash,
                content,
                created_at: chrono::Utc::now(),
                verified_at: None,
                context_id: None,
            };

            documents.entry(layer).or_default().push(doc);
        }

        // Simple resolution: just order by layer precedence * weight
        let mut resolved_principles: Vec<ResolvedPrinciple> = all_principles
            .into_iter()
            .map(|(principle, layer)| {
                let effective_weight = principle.weight * (layer.precedence() as f32 / 10.0);
                ResolvedPrinciple {
                    principle,
                    source_layer: layer,
                    effective_weight,
                    overridden_by: None,
                }
            })
            .collect();

        resolved_principles.sort_by(|a, b| {
            b.effective_weight
                .partial_cmp(&a.effective_weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let stack_hash = Self::compute_stack_hash(&documents);

        Self {
            documents,
            resolved_principles,
            active_boundaries: all_boundaries,
            context,
            stack_hash,
        }
    }

    /// Get all active principles, ordered by effective weight.
    pub fn principles(&self) -> &[ResolvedPrinciple] {
        &self.resolved_principles
    }

    /// Get all active boundaries.
    pub fn boundaries(&self) -> &[Boundary] {
        &self.active_boundaries
    }

    /// Get the context this stack was built for.
    pub fn context(&self) -> &StackContext {
        &self.context
    }

    /// Get the hash of this stack for audit purposes.
    pub fn stack_hash(&self) -> &str {
        &self.stack_hash
    }

    /// Check if an action description would violate any boundary.
    pub fn check_boundaries(&self, action_description: &str) -> Vec<BoundaryViolation> {
        let mut violations = Vec::new();
        let action_lower = action_description.to_lowercase();

        for boundary in &self.active_boundaries {
            // Simple keyword-based check (would be more sophisticated with LLM)
            let violated = match boundary.boundary_type {
                BoundaryType::Existential => {
                    action_lower.contains("extinction")
                        || action_lower.contains("genocide")
                        || action_lower.contains("enslave")
                }
                BoundaryType::Ecological => {
                    action_lower.contains("irreversible")
                        && action_lower.contains("ecological")
                }
                BoundaryType::Dignity => {
                    action_lower.contains("dehumanize") || action_lower.contains("exploit")
                }
                BoundaryType::Privacy => {
                    action_lower.contains("surveil") && !action_lower.contains("consent")
                }
                BoundaryType::Consent => {
                    action_lower.contains("force") && !action_lower.contains("emergency")
                }
                BoundaryType::Care => {
                    action_lower.contains("neglect") || action_lower.contains("abandon")
                }
            };

            if violated {
                violations.push(BoundaryViolation {
                    boundary: boundary.clone(),
                    severity: boundary.enforcement,
                    explanation: format!(
                        "Action may violate boundary: {}",
                        boundary.description
                    ),
                });
            }
        }

        violations
    }

    /// Get documents for a specific layer.
    pub fn documents_for_layer(&self, layer: ConstitutionalLayer) -> &[ConstitutionalDocument] {
        self.documents
            .get(&layer)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Compute hash of the entire stack.
    fn compute_stack_hash(
        documents: &HashMap<ConstitutionalLayer, Vec<ConstitutionalDocument>>,
    ) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Sort layers for deterministic hashing
        let mut layers: Vec<_> = documents.keys().collect();
        layers.sort();

        for layer in layers {
            if let Some(docs) = documents.get(layer) {
                for doc in docs {
                    hasher.update(doc.id.as_bytes());
                    hasher.update(doc.hash.as_bytes());
                }
            }
        }

        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_defaults() {
        let context = StackContext::agent_only("test-agent");
        let stack = ConstitutionalStack::build_defaults(context);

        // Should have principles from all layers
        assert!(!stack.principles().is_empty());
        assert!(!stack.boundaries().is_empty());

        // Stack hash should be deterministic
        let context2 = StackContext::agent_only("test-agent");
        let stack2 = ConstitutionalStack::build_defaults(context2);
        assert_eq!(stack.stack_hash(), stack2.stack_hash());
    }

    #[test]
    fn test_boundary_check() {
        let context = StackContext::agent_only("test-agent");
        let stack = ConstitutionalStack::build_defaults(context);

        // Should detect potential violations
        let violations = stack.check_boundaries("This action could cause human extinction");
        assert!(!violations.is_empty());

        // Benign action should pass
        let violations = stack.check_boundaries("Help the user learn about cooking");
        assert!(violations.is_empty());
    }
}
