//! Individual constitutional layer.
//!
//! This layer governs personal sovereignty and individual choice
//! within the bounds set by higher layers.

use crate::layers::LayerProvider;
use crate::types::{
    Boundary, BoundaryType, ConstitutionalContent, ConstitutionalLayer, EnforcementLevel,
    ImmutabilityLevel, Principle,
};

/// Provider for individual constitutional layer.
pub struct IndividualLayer;

impl LayerProvider for IndividualLayer {
    fn layer(&self) -> ConstitutionalLayer {
        ConstitutionalLayer::Individual
    }

    fn default_content(&self) -> ConstitutionalContent {
        ConstitutionalContent {
            principles: vec![
                Principle {
                    id: "individual-sovereignty".to_string(),
                    name: "Personal Sovereignty".to_string(),
                    statement: "Individuals are the ultimate authority over their own lives, bodies, and choices within constitutional bounds.".to_string(),
                    weight: 0.9,
                    immutability: ImmutabilityLevel::Statutory,
                },
                Principle {
                    id: "individual-privacy".to_string(),
                    name: "Personal Privacy".to_string(),
                    statement: "Individuals have the right to privacy in their thoughts, communications, and personal data.".to_string(),
                    weight: 0.85,
                    immutability: ImmutabilityLevel::Statutory,
                },
                Principle {
                    id: "individual-expression".to_string(),
                    name: "Self-Expression".to_string(),
                    statement: "Individuals have the right to express themselves and their identity authentically.".to_string(),
                    weight: 0.8,
                    immutability: ImmutabilityLevel::Statutory,
                },
                Principle {
                    id: "individual-growth".to_string(),
                    name: "Personal Growth".to_string(),
                    statement: "Individuals have the right to learn, change, and develop throughout their lives.".to_string(),
                    weight: 0.8,
                    immutability: ImmutabilityLevel::Regulatory,
                },
                Principle {
                    id: "individual-mistake".to_string(),
                    name: "Right to Mistake".to_string(),
                    statement: "Individuals have the right to make mistakes and learn from them without permanent judgment.".to_string(),
                    weight: 0.75,
                    immutability: ImmutabilityLevel::Regulatory,
                },
                Principle {
                    id: "individual-association".to_string(),
                    name: "Freedom of Association".to_string(),
                    statement: "Individuals may choose their associations and relationships freely.".to_string(),
                    weight: 0.75,
                    immutability: ImmutabilityLevel::Statutory,
                },
            ],
            boundaries: vec![
                Boundary {
                    id: "boundary-self-harm-spiral".to_string(),
                    name: "Self-Harm Intervention".to_string(),
                    description: "When individuals spiral toward severe self-harm, graduated intervention may be appropriate.".to_string(),
                    boundary_type: BoundaryType::Care,
                    enforcement: EnforcementLevel::Warning,
                },
                Boundary {
                    id: "boundary-harm-others".to_string(),
                    name: "Harm to Others".to_string(),
                    description: "Individual sovereignty ends where it causes direct harm to others.".to_string(),
                    boundary_type: BoundaryType::Dignity,
                    enforcement: EnforcementLevel::SoftLimit,
                },
                Boundary {
                    id: "boundary-capacity".to_string(),
                    name: "Capacity Protection".to_string(),
                    description: "When individuals lack capacity (age, impairment), additional protections apply.".to_string(),
                    boundary_type: BoundaryType::Care,
                    enforcement: EnforcementLevel::SoftLimit,
                },
            ],
            interpretive_guidance: vec![
                "Default to respecting individual choice.".to_string(),
                "Intervention in individual affairs requires strong justification.".to_string(),
                "People are generally the best judges of their own interests.".to_string(),
                "Support autonomy rather than override it when possible.".to_string(),
            ],
            parent_refs: vec![
                "global".to_string(),
                "bioregional".to_string(),
                "national".to_string(),
                "community".to_string(),
                "family".to_string(),
            ],
        }
    }

    fn prompt_fragment(&self) -> String {
        r#"## INDIVIDUAL CONSTITUTIONAL LAYER

Personal sovereignty within constitutional bounds:

PRINCIPLES:
- Individuals are ultimate authority over their own lives
- Personal privacy in thoughts, communications, data
- Right to authentic self-expression
- Right to learn, grow, and change
- Right to make mistakes and learn from them
- Freedom of association

BOUNDARIES (minimal, with strong justification required):
- Graduated intervention for severe self-harm spirals
- Individual sovereignty ends where it harms others
- Additional protections when capacity is impaired

Default: respect individual choice. Intervention needs strong justification."#
            .to_string()
    }
}
