//! Community constitutional layer.
//!
//! This layer governs local community norms, membership rules, and
//! specific practices within broader constitutional bounds.

use crate::layers::LayerProvider;
use crate::types::{
    Boundary, BoundaryType, ConstitutionalContent, ConstitutionalLayer, EnforcementLevel,
    ImmutabilityLevel, Principle,
};

/// Provider for community constitutional layer.
pub struct CommunityLayer;

impl LayerProvider for CommunityLayer {
    fn layer(&self) -> ConstitutionalLayer {
        ConstitutionalLayer::Community
    }

    fn default_content(&self) -> ConstitutionalContent {
        ConstitutionalContent {
            principles: vec![
                Principle {
                    id: "community-dunbar".to_string(),
                    name: "Dunbar-Scale Governance".to_string(),
                    statement: "Effective governance requires human-scale groups. Communities should subdivide when they exceed coordination capacity (~150 active members).".to_string(),
                    weight: 0.85,
                    immutability: ImmutabilityLevel::Statutory,
                },
                Principle {
                    id: "community-membership".to_string(),
                    name: "Membership Rights".to_string(),
                    statement: "Communities define their own membership criteria, subject to non-discrimination in protected categories.".to_string(),
                    weight: 0.8,
                    immutability: ImmutabilityLevel::Statutory,
                },
                Principle {
                    id: "community-local-wisdom".to_string(),
                    name: "Local Wisdom".to_string(),
                    statement: "Communities have knowledge about their specific context that outside authorities lack.".to_string(),
                    weight: 0.75,
                    immutability: ImmutabilityLevel::Regulatory,
                },
                Principle {
                    id: "community-mutual-accountability".to_string(),
                    name: "Mutual Accountability".to_string(),
                    statement: "Community members hold each other accountable through relationship, not just rules.".to_string(),
                    weight: 0.75,
                    immutability: ImmutabilityLevel::Regulatory,
                },
                Principle {
                    id: "community-redemption".to_string(),
                    name: "Redemptive Justice".to_string(),
                    statement: "Justice aims to restore relationships and reintegrate offenders, not merely punish.".to_string(),
                    weight: 0.7,
                    immutability: ImmutabilityLevel::Statutory,
                },
            ],
            boundaries: vec![
                Boundary {
                    id: "boundary-exclusion-abuse".to_string(),
                    name: "Exclusion Abuse Prevention".to_string(),
                    description: "Communities cannot use exclusion to trap vulnerable members.".to_string(),
                    boundary_type: BoundaryType::Consent,
                    enforcement: EnforcementLevel::SoftLimit,
                },
                Boundary {
                    id: "boundary-voice".to_string(),
                    name: "Voice Rights".to_string(),
                    description: "Members must have meaningful voice in decisions affecting them.".to_string(),
                    boundary_type: BoundaryType::Dignity,
                    enforcement: EnforcementLevel::SoftLimit,
                },
                Boundary {
                    id: "boundary-transparency".to_string(),
                    name: "Governance Transparency".to_string(),
                    description: "Community governance decisions must be transparent and explainable.".to_string(),
                    boundary_type: BoundaryType::Consent,
                    enforcement: EnforcementLevel::Warning,
                },
            ],
            interpretive_guidance: vec![
                "Community norms should serve members, not the other way around.".to_string(),
                "Healthy communities can handle disagreement and conflict.".to_string(),
                "When in doubt, enable member agency over community control.".to_string(),
            ],
            parent_refs: vec![
                "global".to_string(),
                "bioregional".to_string(),
                "national".to_string(),
            ],
        }
    }

    fn prompt_fragment(&self) -> String {
        r#"## COMMUNITY CONSTITUTIONAL LAYER

Local governance within broader bounds:

PRINCIPLES:
- Effective governance requires human-scale groups (~150 members)
- Communities define membership subject to non-discrimination
- Local wisdom deserves respect
- Mutual accountability through relationship
- Justice aims to restore and reintegrate

BOUNDARIES:
- Cannot use exclusion to trap vulnerable members
- Members must have voice in decisions affecting them
- Governance must be transparent

Community norms serve members, not vice versa."#
            .to_string()
    }
}
