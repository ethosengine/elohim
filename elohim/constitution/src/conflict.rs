//! Conflict resolution between constitutional layers.
//!
//! When principles from different layers conflict, this module provides
//! algorithms for resolving them according to the constitutional hierarchy.

use crate::types::*;

/// Resolves conflicts between principles from different layers.
pub struct ConflictResolver {
    /// Weight multiplier for layer precedence
    layer_weight_factor: f32,
    /// Whether to allow lower layers to override higher ones in specific cases
    allow_specialization: bool,
}

impl ConflictResolver {
    /// Create a new conflict resolver with default settings.
    pub fn new() -> Self {
        Self {
            layer_weight_factor: 0.15,
            allow_specialization: true,
        }
    }

    /// Create a resolver with custom settings.
    pub fn with_settings(layer_weight_factor: f32, allow_specialization: bool) -> Self {
        Self {
            layer_weight_factor,
            allow_specialization,
        }
    }

    /// Resolve a set of principles from multiple layers.
    ///
    /// Returns principles ordered by effective weight, with conflicts resolved.
    pub fn resolve_principles(
        &self,
        principles: &[(Principle, ConstitutionalLayer)],
    ) -> Vec<ResolvedPrinciple> {
        let mut resolved: Vec<ResolvedPrinciple> = Vec::new();

        // Group principles by name to detect conflicts
        let mut by_name: std::collections::HashMap<String, Vec<(Principle, ConstitutionalLayer)>> =
            std::collections::HashMap::new();

        for (principle, layer) in principles {
            by_name
                .entry(principle.name.clone())
                .or_default()
                .push((principle.clone(), *layer));
        }

        // Resolve each group
        for (name, variants) in by_name {
            if variants.len() == 1 {
                // No conflict, just compute effective weight
                let (principle, layer) = &variants[0];
                let effective_weight = self.compute_effective_weight(principle, *layer);

                resolved.push(ResolvedPrinciple {
                    principle: principle.clone(),
                    source_layer: *layer,
                    effective_weight,
                    overridden_by: None,
                });
            } else {
                // Conflict - resolve based on layer precedence
                let winner = self.resolve_conflict(&name, &variants);
                resolved.push(winner);
            }
        }

        // Sort by effective weight (descending), then by layer precedence (descending)
        resolved.sort_by(|a, b| {
            match b.effective_weight.partial_cmp(&a.effective_weight) {
                Some(std::cmp::Ordering::Equal) | None => {
                    // Secondary sort by layer precedence (higher layer = more authoritative)
                    b.source_layer.precedence().cmp(&a.source_layer.precedence())
                }
                Some(ord) => ord,
            }
        });

        resolved
    }

    /// Compute effective weight of a principle.
    ///
    /// Combines the principle's inherent weight with its layer precedence.
    fn compute_effective_weight(&self, principle: &Principle, layer: ConstitutionalLayer) -> f32 {
        let layer_bonus = layer.precedence() as f32 * self.layer_weight_factor;
        let immutability_bonus = principle.immutability.difficulty() as f32 * 0.05;

        (principle.weight + layer_bonus + immutability_bonus).min(1.0)
    }

    /// Resolve a conflict between multiple variants of a principle.
    fn resolve_conflict(
        &self,
        _name: &str,
        variants: &[(Principle, ConstitutionalLayer)],
    ) -> ResolvedPrinciple {
        // Sort by layer precedence (highest first)
        let mut sorted: Vec<_> = variants.to_vec();
        sorted.sort_by(|a, b| b.1.precedence().cmp(&a.1.precedence()));

        let (winner_principle, winner_layer) = &sorted[0];
        let effective_weight = self.compute_effective_weight(winner_principle, *winner_layer);

        // Check if lower layers provide valid specializations
        let overridden_by = if self.allow_specialization && sorted.len() > 1 {
            // A lower layer can specialize if it has higher immutability
            let lower = &sorted[1];
            if lower.0.immutability.difficulty() > winner_principle.immutability.difficulty() {
                Some(lower.0.id.clone())
            } else {
                None
            }
        } else {
            None
        };

        ResolvedPrinciple {
            principle: winner_principle.clone(),
            source_layer: *winner_layer,
            effective_weight,
            overridden_by,
        }
    }

    /// Check if two principles are in conflict.
    ///
    /// Principles conflict if they have the same name but different statements
    /// or if they have explicitly contradictory requirements.
    pub fn are_in_conflict(&self, a: &Principle, b: &Principle) -> bool {
        if a.name == b.name && a.statement != b.statement {
            return true;
        }

        // TODO: Semantic conflict detection would go here
        // Could use embeddings or keyword analysis

        false
    }

    /// Get resolution explanation for audit purposes.
    pub fn explain_resolution(
        &self,
        principle_name: &str,
        variants: &[(Principle, ConstitutionalLayer)],
    ) -> String {
        if variants.len() <= 1 {
            return "No conflict to resolve.".to_string();
        }

        let mut explanation = format!(
            "Principle '{}' exists in {} layers:\n",
            principle_name,
            variants.len()
        );

        for (principle, layer) in variants {
            explanation.push_str(&format!(
                "  - {} (weight: {:.2}, immutability: {:?})\n",
                layer.as_str(),
                principle.weight,
                principle.immutability
            ));
        }

        // Determine winner
        let mut sorted: Vec<_> = variants.to_vec();
        sorted.sort_by(|a, b| b.1.precedence().cmp(&a.1.precedence()));

        explanation.push_str(&format!(
            "\nResolution: {} layer takes precedence.\n",
            sorted[0].1.as_str()
        ));

        explanation
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_principle(name: &str, weight: f32, immutability: ImmutabilityLevel) -> Principle {
        Principle {
            id: format!("test-{}", name.to_lowercase().replace(' ', "-")),
            name: name.to_string(),
            statement: format!("Test statement for {}", name),
            weight,
            immutability,
        }
    }

    #[test]
    fn test_no_conflict() {
        let resolver = ConflictResolver::new();

        let principles = vec![
            (
                make_principle("Dignity", 1.0, ImmutabilityLevel::Absolute),
                ConstitutionalLayer::Global,
            ),
            (
                make_principle("Privacy", 0.8, ImmutabilityLevel::Statutory),
                ConstitutionalLayer::Individual,
            ),
        ];

        let resolved = resolver.resolve_principles(&principles);
        assert_eq!(resolved.len(), 2);

        // Global dignity should have higher effective weight
        assert!(resolved[0].source_layer == ConstitutionalLayer::Global);
    }

    #[test]
    fn test_conflict_resolution() {
        let resolver = ConflictResolver::new();

        // Same principle name, different layers
        let principles = vec![
            (
                make_principle("Expression", 0.8, ImmutabilityLevel::Statutory),
                ConstitutionalLayer::Global,
            ),
            (
                make_principle("Expression", 0.9, ImmutabilityLevel::Regulatory),
                ConstitutionalLayer::Community,
            ),
        ];

        let resolved = resolver.resolve_principles(&principles);

        // Should produce single resolved principle
        // Global should win due to layer precedence
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].source_layer, ConstitutionalLayer::Global);
    }

    #[test]
    fn test_effective_weight_calculation() {
        let resolver = ConflictResolver::new();

        let global_principle = make_principle("Test", 0.5, ImmutabilityLevel::Constitutional);
        let individual_principle = make_principle("Test", 0.5, ImmutabilityLevel::Regulatory);

        let global_weight =
            resolver.compute_effective_weight(&global_principle, ConstitutionalLayer::Global);
        let individual_weight = resolver
            .compute_effective_weight(&individual_principle, ConstitutionalLayer::Individual);

        // Global should have higher effective weight due to layer precedence
        assert!(global_weight > individual_weight);
    }
}
