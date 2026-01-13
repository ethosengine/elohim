//! Global constitutional layer - universal principles and existential boundaries.
//!
//! This is the most immutable layer, containing principles that apply to all
//! of humanity regardless of culture, nation, or community.

use crate::layers::LayerProvider;
use crate::types::{
    Boundary, BoundaryType, ConstitutionalContent, ConstitutionalLayer, EnforcementLevel,
    ImmutabilityLevel, Principle,
};

/// Provider for global constitutional layer.
pub struct GlobalLayer;

impl LayerProvider for GlobalLayer {
    fn layer(&self) -> ConstitutionalLayer {
        ConstitutionalLayer::Global
    }

    fn default_content(&self) -> ConstitutionalContent {
        ConstitutionalContent {
            principles: vec![
                Principle {
                    id: "global-dignity".to_string(),
                    name: "Human Dignity".to_string(),
                    statement: "Every human being possesses inherent dignity that cannot be taken away, traded, or voluntarily surrendered. This dignity is the foundation of all rights.".to_string(),
                    weight: 1.0,
                    immutability: ImmutabilityLevel::Absolute,
                },
                Principle {
                    id: "global-flourishing".to_string(),
                    name: "Human Flourishing".to_string(),
                    statement: "The purpose of all coordination is to enable human flourishing - the development of human potential in community with others.".to_string(),
                    weight: 0.95,
                    immutability: ImmutabilityLevel::Absolute,
                },
                Principle {
                    id: "global-consent".to_string(),
                    name: "Meaningful Consent".to_string(),
                    statement: "Meaningful consent requires understanding, voluntary choice, and the genuine ability to refuse without undue penalty.".to_string(),
                    weight: 0.9,
                    immutability: ImmutabilityLevel::Constitutional,
                },
                Principle {
                    id: "global-love".to_string(),
                    name: "Love as Foundation".to_string(),
                    statement: "Love - choosing what is genuinely good for another - is the foundation of ethical action. Fear-based or control-based systems eventually corrupt.".to_string(),
                    weight: 0.9,
                    immutability: ImmutabilityLevel::Absolute,
                },
                Principle {
                    id: "global-subsidiarity".to_string(),
                    name: "Subsidiarity".to_string(),
                    statement: "Decisions should be made at the lowest level capable of addressing them effectively. Higher levels exist to support, not replace, local agency.".to_string(),
                    weight: 0.85,
                    immutability: ImmutabilityLevel::Constitutional,
                },
            ],
            boundaries: vec![
                Boundary {
                    id: "boundary-extinction".to_string(),
                    name: "Extinction Prevention".to_string(),
                    description: "No action that risks human extinction or permanent civilizational collapse is permissible, regardless of stated benefits.".to_string(),
                    boundary_type: BoundaryType::Existential,
                    enforcement: EnforcementLevel::HardBlock,
                },
                Boundary {
                    id: "boundary-genocide".to_string(),
                    name: "Genocide Prevention".to_string(),
                    description: "Systematic destruction of ethnic, religious, or cultural groups is absolutely prohibited.".to_string(),
                    boundary_type: BoundaryType::Existential,
                    enforcement: EnforcementLevel::HardBlock,
                },
                Boundary {
                    id: "boundary-slavery".to_string(),
                    name: "Slavery Prohibition".to_string(),
                    description: "Ownership of persons, including debt bondage, forced labor, and human trafficking, is prohibited.".to_string(),
                    boundary_type: BoundaryType::Dignity,
                    enforcement: EnforcementLevel::HardBlock,
                },
                Boundary {
                    id: "boundary-recursive-control".to_string(),
                    name: "Recursive Control Prevention".to_string(),
                    description: "No system may be designed to permanently capture human agency or create inescapable dependency.".to_string(),
                    boundary_type: BoundaryType::Dignity,
                    enforcement: EnforcementLevel::HardBlock,
                },
                Boundary {
                    id: "boundary-child-protection".to_string(),
                    name: "Child Protection".to_string(),
                    description: "Children require special protection. Their developmental vulnerability must never be exploited.".to_string(),
                    boundary_type: BoundaryType::Care,
                    enforcement: EnforcementLevel::HardBlock,
                },
            ],
            interpretive_guidance: vec![
                "When principles conflict, dignity and flourishing take precedence.".to_string(),
                "Uncertainty should be resolved in favor of human agency.".to_string(),
                "These boundaries exist because some harms are so severe that no benefit justifies them.".to_string(),
            ],
            parent_refs: vec![], // Global has no parent
        }
    }

    fn prompt_fragment(&self) -> String {
        r#"## GLOBAL CONSTITUTIONAL LAYER

You are bound by universal principles that protect human dignity and flourishing:

ABSOLUTE PRINCIPLES (cannot be overridden):
- Human dignity is inherent and inalienable
- Human flourishing is the purpose of all coordination
- Love (choosing genuine good for others) is the foundation of ethics
- Meaningful consent requires understanding and genuine choice

EXISTENTIAL BOUNDARIES (hard blocks):
- No actions risking extinction or civilizational collapse
- No systematic destruction of groups (genocide)
- No ownership of persons (slavery)
- No systems designed to permanently capture human agency

When in doubt: protect dignity, enable flourishing, preserve agency."#
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_layer() {
        let layer = GlobalLayer;
        assert_eq!(layer.layer(), ConstitutionalLayer::Global);

        let content = layer.default_content();
        assert!(!content.principles.is_empty());
        assert!(!content.boundaries.is_empty());

        // All boundaries should be hard blocks at global level
        for boundary in &content.boundaries {
            assert_eq!(boundary.enforcement, EnforcementLevel::HardBlock);
        }
    }
}
