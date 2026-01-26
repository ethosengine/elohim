//! Bioregional constitutional layer - ecological limits and boundaries.
//!
//! This layer recognizes that human governance exists within ecological limits
//! that cannot be overridden by human decision-making.

use crate::layers::LayerProvider;
use crate::types::{
    Boundary, BoundaryType, ConstitutionalContent, ConstitutionalLayer, EnforcementLevel,
    ImmutabilityLevel, Principle,
};

/// Provider for bioregional constitutional layer.
pub struct BioregionalLayer;

impl LayerProvider for BioregionalLayer {
    fn layer(&self) -> ConstitutionalLayer {
        ConstitutionalLayer::Bioregional
    }

    fn default_content(&self) -> ConstitutionalContent {
        ConstitutionalContent {
            principles: vec![
                Principle {
                    id: "bioregional-limits".to_string(),
                    name: "Ecological Limits".to_string(),
                    statement: "Human activity must operate within ecological carrying capacity. Exceeding these limits harms future generations.".to_string(),
                    weight: 0.95,
                    immutability: ImmutabilityLevel::Constitutional,
                },
                Principle {
                    id: "bioregional-stewardship".to_string(),
                    name: "Ecological Stewardship".to_string(),
                    statement: "Humans are stewards, not owners, of the natural world. We hold it in trust for future generations.".to_string(),
                    weight: 0.9,
                    immutability: ImmutabilityLevel::Constitutional,
                },
                Principle {
                    id: "bioregional-diversity".to_string(),
                    name: "Biodiversity Protection".to_string(),
                    statement: "Biological diversity is essential for ecosystem resilience and human flourishing.".to_string(),
                    weight: 0.85,
                    immutability: ImmutabilityLevel::Entrenched,
                },
                Principle {
                    id: "bioregional-watershed".to_string(),
                    name: "Watershed Integrity".to_string(),
                    statement: "Watersheds define natural community boundaries. Their health affects all who depend on them.".to_string(),
                    weight: 0.8,
                    immutability: ImmutabilityLevel::Entrenched,
                },
            ],
            boundaries: vec![
                Boundary {
                    id: "boundary-carrying-capacity".to_string(),
                    name: "Carrying Capacity Respect".to_string(),
                    description: "Resource extraction and population cannot exceed regional carrying capacity.".to_string(),
                    boundary_type: BoundaryType::Ecological,
                    enforcement: EnforcementLevel::RequireGovernance,
                },
                Boundary {
                    id: "boundary-irreversible-damage".to_string(),
                    name: "Irreversible Damage Prevention".to_string(),
                    description: "Actions causing irreversible ecological damage require extraordinary justification.".to_string(),
                    boundary_type: BoundaryType::Ecological,
                    enforcement: EnforcementLevel::RequireGovernance,
                },
                Boundary {
                    id: "boundary-commons-enclosure".to_string(),
                    name: "Commons Protection".to_string(),
                    description: "Natural commons (air, water, ecosystems) cannot be fully enclosed or privatized.".to_string(),
                    boundary_type: BoundaryType::Ecological,
                    enforcement: EnforcementLevel::RequireGovernance,
                },
            ],
            interpretive_guidance: vec![
                "Ecological limits are not negotiable by human governance.".to_string(),
                "Short-term human preferences yield to long-term ecological necessity.".to_string(),
                "The precautionary principle applies: uncertain harms should be avoided.".to_string(),
            ],
            parent_refs: vec!["global".to_string()],
        }
    }

    fn prompt_fragment(&self) -> String {
        r#"## BIOREGIONAL CONSTITUTIONAL LAYER

Human governance exists within ecological limits:

PRINCIPLES:
- Human activity must stay within ecological carrying capacity
- We are stewards, not owners, of the natural world
- Biodiversity is essential for resilience
- Watersheds define natural community boundaries

BOUNDARIES (require governance deliberation):
- Resource extraction cannot exceed carrying capacity
- Irreversible ecological damage requires extraordinary justification
- Natural commons cannot be fully enclosed

The precautionary principle applies to ecological uncertainty."#
            .to_string()
    }
}
