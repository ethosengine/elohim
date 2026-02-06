//! Elohim capability definitions.
//!
//! Matches TypeScript `ElohimCapability` in elohim-agent.model.ts.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Elohim agent capabilities.
///
/// Each capability represents a specific function an Elohim agent can perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum ElohimCapability {
    // ========== Content Operations ==========
    /// Review content for safety issues
    ContentSafetyReview,
    /// Verify factual accuracy
    AccuracyVerification,
    /// Recommend whether to issue attestations
    AttestationRecommendation,
    /// Verify constitutional compliance
    ConstitutionalVerification,

    // ========== Knowledge Map Operations ==========
    /// Synthesize knowledge maps from content
    KnowledgeMapSynthesis,
    /// Analyze learning affinity patterns
    AffinityAnalysis,
    /// Recommend learning paths
    PathRecommendation,

    // ========== Care Operations ==========
    /// Detect spiral patterns (individual or community)
    SpiralDetection,
    /// Connect individuals to care resources
    CareConnection,
    /// Apply graduated intervention protocols
    GraduatedIntervention,

    // ========== Governance Operations ==========
    /// Validate actions across constitutional layers
    CrossLayerValidation,
    /// Enforce existential boundaries
    ExistentialBoundaryEnforcement,
    /// Process governance ratification
    GovernanceRatification,

    // ========== Path Operations ==========
    /// Analyze learning path structure
    PathAnalysis,
    /// Validate learning objectives
    LearningObjectiveValidation,
    /// Verify prerequisites
    PrerequisiteVerification,
    /// Design mastery assessments
    MasteryAssessmentDesign,

    // ========== Family/Individual Operations ==========
    /// Align with family-specific values
    FamilyValueAlignment,
    /// Provide personal agent support
    PersonalAgentSupport,

    // ========== Feedback Profile Operations ==========
    /// Negotiate feedback profiles
    FeedbackProfileNegotiation,
    /// Enforce feedback profile constraints
    FeedbackProfileEnforcement,
    /// Upgrade feedback profile trust level
    FeedbackProfileUpgrade,
    /// Downgrade feedback profile trust level
    FeedbackProfileDowngrade,

    // ========== Place Operations ==========
    /// Attest to place relationships
    PlaceAttestation,
    /// Govern place naming
    PlaceNamingGovernance,
    /// Assign geographic reach
    GeographicReachAssignment,
    /// Enforce bioregional boundaries
    BioregionalEnforcement,
}

impl ElohimCapability {
    /// Get estimated processing time in milliseconds.
    pub fn estimated_time_ms(&self) -> u64 {
        match self {
            // Fast operations
            Self::SpiralDetection => 800,
            Self::AttestationRecommendation => 1200,
            Self::FeedbackProfileEnforcement => 500,
            Self::FeedbackProfileUpgrade | Self::FeedbackProfileDowngrade => 600,

            // Medium operations
            Self::ContentSafetyReview => 1500,
            Self::AccuracyVerification => 2000,
            Self::PathAnalysis => 2500,
            Self::PrerequisiteVerification => 1000,
            Self::LearningObjectiveValidation => 1500,
            Self::FamilyValueAlignment => 1800,
            Self::PersonalAgentSupport => 1500,
            Self::AffinityAnalysis => 2000,

            // Slow operations
            Self::KnowledgeMapSynthesis => 3000,
            Self::CrossLayerValidation => 2500,
            Self::GovernanceRatification => 3500,
            Self::ConstitutionalVerification => 2500,
            Self::MasteryAssessmentDesign => 4000,

            // Default
            _ => 1000,
        }
    }

    /// Get the minimum required constitutional layer for this capability.
    pub fn required_layer(&self) -> Option<constitution::ConstitutionalLayer> {
        use constitution::ConstitutionalLayer;

        match self {
            // Global layer capabilities
            Self::ExistentialBoundaryEnforcement => Some(ConstitutionalLayer::Global),
            Self::CrossLayerValidation => Some(ConstitutionalLayer::Global),

            // Bioregional capabilities
            Self::BioregionalEnforcement => Some(ConstitutionalLayer::Bioregional),
            Self::GeographicReachAssignment => Some(ConstitutionalLayer::Bioregional),

            // National/Provincial capabilities
            Self::GovernanceRatification => Some(ConstitutionalLayer::NationState),
            Self::PlaceNamingGovernance => Some(ConstitutionalLayer::Provincial),

            // Community capabilities
            Self::CareConnection => Some(ConstitutionalLayer::Community),
            Self::GraduatedIntervention => Some(ConstitutionalLayer::Community),

            // Family capabilities
            Self::FamilyValueAlignment => Some(ConstitutionalLayer::Family),

            // Individual capabilities (or no layer restriction)
            Self::PersonalAgentSupport => Some(ConstitutionalLayer::Individual),

            // No specific layer required
            _ => None,
        }
    }

    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::ContentSafetyReview => "Review content for harmful patterns and safety issues",
            Self::AccuracyVerification => "Verify factual accuracy of claims",
            Self::AttestationRecommendation => "Recommend attestation decisions",
            Self::ConstitutionalVerification => "Verify constitutional compliance of actions",
            Self::KnowledgeMapSynthesis => "Build or update knowledge maps from content",
            Self::AffinityAnalysis => "Analyze learning affinity patterns",
            Self::PathRecommendation => "Recommend learning paths based on goals and history",
            Self::SpiralDetection => "Detect individual or community spiraling patterns",
            Self::CareConnection => "Connect individuals to appropriate care resources",
            Self::GraduatedIntervention => "Apply graduated intervention protocols",
            Self::CrossLayerValidation => "Validate actions across constitutional layers",
            Self::ExistentialBoundaryEnforcement => "Enforce existential safety boundaries",
            Self::GovernanceRatification => "Process governance decisions requiring ratification",
            Self::PathAnalysis => "Analyze learning path structure and effectiveness",
            Self::LearningObjectiveValidation => "Validate learning objectives for clarity and achievability",
            Self::PrerequisiteVerification => "Verify prerequisites are met for learning progression",
            Self::MasteryAssessmentDesign => "Design assessments for mastery verification",
            Self::FamilyValueAlignment => "Align recommendations with family-specific values",
            Self::PersonalAgentSupport => "Provide personalized agent support",
            Self::FeedbackProfileNegotiation => "Negotiate feedback profile terms",
            Self::FeedbackProfileEnforcement => "Enforce feedback profile constraints",
            Self::FeedbackProfileUpgrade => "Upgrade feedback profile trust level",
            Self::FeedbackProfileDowngrade => "Downgrade feedback profile trust level",
            Self::PlaceAttestation => "Attest to place relationships and history",
            Self::PlaceNamingGovernance => "Govern place naming and identity",
            Self::GeographicReachAssignment => "Assign geographic reach for content or agents",
            Self::BioregionalEnforcement => "Enforce bioregional boundaries and limits",
        }
    }

    /// All capabilities as a list.
    pub fn all() -> Vec<Self> {
        vec![
            Self::ContentSafetyReview,
            Self::AccuracyVerification,
            Self::AttestationRecommendation,
            Self::ConstitutionalVerification,
            Self::KnowledgeMapSynthesis,
            Self::AffinityAnalysis,
            Self::PathRecommendation,
            Self::SpiralDetection,
            Self::CareConnection,
            Self::GraduatedIntervention,
            Self::CrossLayerValidation,
            Self::ExistentialBoundaryEnforcement,
            Self::GovernanceRatification,
            Self::PathAnalysis,
            Self::LearningObjectiveValidation,
            Self::PrerequisiteVerification,
            Self::MasteryAssessmentDesign,
            Self::FamilyValueAlignment,
            Self::PersonalAgentSupport,
            Self::FeedbackProfileNegotiation,
            Self::FeedbackProfileEnforcement,
            Self::FeedbackProfileUpgrade,
            Self::FeedbackProfileDowngrade,
            Self::PlaceAttestation,
            Self::PlaceNamingGovernance,
            Self::GeographicReachAssignment,
            Self::BioregionalEnforcement,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_serialization() {
        let cap = ElohimCapability::ContentSafetyReview;
        let json = serde_json::to_string(&cap).unwrap();
        assert_eq!(json, "\"content-safety-review\"");

        let parsed: ElohimCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cap);
    }

    #[test]
    fn test_all_capabilities() {
        let all = ElohimCapability::all();
        assert!(all.len() > 20);
        assert!(all.contains(&ElohimCapability::SpiralDetection));
    }
}
