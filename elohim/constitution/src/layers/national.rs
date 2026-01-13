//! National/cultural constitutional layer.
//!
//! This layer allows for cultural expressions and interpretations of
//! universal principles while remaining bound by global and bioregional limits.

use crate::layers::LayerProvider;
use crate::types::{
    Boundary, BoundaryType, ConstitutionalContent, ConstitutionalLayer, EnforcementLevel,
    ImmutabilityLevel, Principle,
};

/// Provider for national constitutional layer.
pub struct NationalLayer;

impl LayerProvider for NationalLayer {
    fn layer(&self) -> ConstitutionalLayer {
        ConstitutionalLayer::NationState
    }

    fn default_content(&self) -> ConstitutionalContent {
        ConstitutionalContent {
            principles: vec![
                Principle {
                    id: "national-cultural-expression".to_string(),
                    name: "Cultural Expression".to_string(),
                    statement: "Communities have the right to express universal principles through their own cultural traditions and practices.".to_string(),
                    weight: 0.8,
                    immutability: ImmutabilityLevel::Entrenched,
                },
                Principle {
                    id: "national-self-determination".to_string(),
                    name: "Self-Determination".to_string(),
                    statement: "Peoples have the right to determine their own governance structures within universal bounds.".to_string(),
                    weight: 0.8,
                    immutability: ImmutabilityLevel::Entrenched,
                },
                Principle {
                    id: "national-diversity-unity".to_string(),
                    name: "Diversity Within Unity".to_string(),
                    statement: "Cultural diversity strengthens humanity when united by shared commitments to dignity and flourishing.".to_string(),
                    weight: 0.75,
                    immutability: ImmutabilityLevel::Statutory,
                },
                Principle {
                    id: "national-mutual-aid".to_string(),
                    name: "Mutual Aid Across Cultures".to_string(),
                    statement: "Different cultures have wisdom to share. Cross-pollination of successful patterns benefits all.".to_string(),
                    weight: 0.7,
                    immutability: ImmutabilityLevel::Statutory,
                },
            ],
            boundaries: vec![
                Boundary {
                    id: "boundary-cultural-oppression".to_string(),
                    name: "Cultural Oppression Prevention".to_string(),
                    description: "No culture may be forcibly suppressed or assimilated.".to_string(),
                    boundary_type: BoundaryType::Dignity,
                    enforcement: EnforcementLevel::RequireGovernance,
                },
                Boundary {
                    id: "boundary-exit-rights".to_string(),
                    name: "Exit Rights".to_string(),
                    description: "Individuals must be able to exit their cultural context without undue penalty.".to_string(),
                    boundary_type: BoundaryType::Consent,
                    enforcement: EnforcementLevel::SoftLimit,
                },
            ],
            interpretive_guidance: vec![
                "Cultural practices that violate global principles are not protected.".to_string(),
                "Diversity is valuable but not when it enables harm.".to_string(),
                "Translation between cultural frameworks requires good faith.".to_string(),
            ],
            parent_refs: vec!["global".to_string(), "bioregional".to_string()],
        }
    }

    fn prompt_fragment(&self) -> String {
        r#"## NATIONAL/CULTURAL CONSTITUTIONAL LAYER

Cultural expression within universal bounds:

PRINCIPLES:
- Communities may express universal principles through their traditions
- Peoples have self-determination rights within universal limits
- Cultural diversity strengthens humanity
- Cross-cultural wisdom sharing benefits all

BOUNDARIES:
- No forced cultural suppression or assimilation
- Individuals must be able to exit cultural contexts

Cultural practices violating global principles are not protected."#
            .to_string()
    }
}
