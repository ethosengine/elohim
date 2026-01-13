//! Family/household constitutional layer.
//!
//! This layer governs intimate relationships and household governance
//! with significant flexibility while protecting vulnerable members.

use crate::layers::LayerProvider;
use crate::types::{
    Boundary, BoundaryType, ConstitutionalContent, ConstitutionalLayer, EnforcementLevel,
    ImmutabilityLevel, Principle,
};

/// Provider for family constitutional layer.
pub struct FamilyLayer;

impl LayerProvider for FamilyLayer {
    fn layer(&self) -> ConstitutionalLayer {
        ConstitutionalLayer::Family
    }

    fn default_content(&self) -> ConstitutionalContent {
        ConstitutionalContent {
            principles: vec![
                Principle {
                    id: "family-autonomy".to_string(),
                    name: "Family Autonomy".to_string(),
                    statement: "Families have broad autonomy to organize their internal affairs according to their values.".to_string(),
                    weight: 0.85,
                    immutability: ImmutabilityLevel::Statutory,
                },
                Principle {
                    id: "family-care".to_string(),
                    name: "Care Responsibilities".to_string(),
                    statement: "Family members have mutual care responsibilities that society should support, not replace.".to_string(),
                    weight: 0.8,
                    immutability: ImmutabilityLevel::Statutory,
                },
                Principle {
                    id: "family-privacy".to_string(),
                    name: "Family Privacy".to_string(),
                    statement: "Family life deserves privacy protection. External intervention requires strong justification.".to_string(),
                    weight: 0.8,
                    immutability: ImmutabilityLevel::Statutory,
                },
                Principle {
                    id: "family-formation".to_string(),
                    name: "Formation Freedom".to_string(),
                    statement: "Adults may form families according to their own understanding, with mutual consent.".to_string(),
                    weight: 0.75,
                    immutability: ImmutabilityLevel::Regulatory,
                },
            ],
            boundaries: vec![
                Boundary {
                    id: "boundary-domestic-abuse".to_string(),
                    name: "Domestic Abuse Prevention".to_string(),
                    description: "Family autonomy does not protect abuse. Harm to family members triggers external protection.".to_string(),
                    boundary_type: BoundaryType::Care,
                    enforcement: EnforcementLevel::RequireGovernance,
                },
                Boundary {
                    id: "boundary-child-welfare".to_string(),
                    name: "Child Welfare".to_string(),
                    description: "Children's welfare takes precedence over parental preferences when in conflict.".to_string(),
                    boundary_type: BoundaryType::Care,
                    enforcement: EnforcementLevel::RequireGovernance,
                },
                Boundary {
                    id: "boundary-exit-family".to_string(),
                    name: "Family Exit Rights".to_string(),
                    description: "Adult family members must be able to exit family arrangements.".to_string(),
                    boundary_type: BoundaryType::Consent,
                    enforcement: EnforcementLevel::SoftLimit,
                },
            ],
            interpretive_guidance: vec![
                "Family privacy is important but not absolute.".to_string(),
                "Protecting the vulnerable takes precedence over family autonomy.".to_string(),
                "External intervention should be supportive first, coercive only when necessary.".to_string(),
            ],
            parent_refs: vec![
                "global".to_string(),
                "bioregional".to_string(),
                "national".to_string(),
                "community".to_string(),
            ],
        }
    }

    fn prompt_fragment(&self) -> String {
        r#"## FAMILY CONSTITUTIONAL LAYER

Intimate governance with protection for vulnerable members:

PRINCIPLES:
- Families have broad autonomy for internal affairs
- Family members have mutual care responsibilities
- Family life deserves privacy protection
- Adults may form families with mutual consent

BOUNDARIES:
- Family autonomy does not protect abuse
- Children's welfare takes precedence over parental preferences
- Adults must be able to exit family arrangements

Protecting the vulnerable takes precedence over family autonomy."#
            .to_string()
    }
}
