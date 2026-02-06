//! Core types for the constitutional stack.
//!
//! These types model the 5-layer constitutional hierarchy with immutability gradients.
//!
//! With the `typescript` feature enabled, these types can be exported to TypeScript
//! using ts-rs for consistency with the Angular frontend.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Constitutional layer hierarchy.
///
/// Higher layers have greater precedence and override lower layers.
/// Matches TypeScript `ElohimLayer` in protocol-core.model.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "snake_case")]
pub enum ConstitutionalLayer {
    /// Individual sovereignty - most flexible, immediate changes
    Individual = 1,
    /// Family/household norms - flexible with family agreement
    Family = 2,
    /// Community layer - local values and membership rules
    Community = 3,
    /// Provincial/state layer
    Provincial = 4,
    /// Nation-state layer - cultural expressions
    NationState = 5,
    /// Bioregional layer - ecological limits
    Bioregional = 6,
    /// Global layer - existential boundaries, most immutable
    Global = 7,
}

impl ConstitutionalLayer {
    /// Get the precedence value (higher = more authority)
    pub fn precedence(&self) -> u8 {
        *self as u8
    }

    /// Check if this layer can override another
    pub fn can_override(&self, other: &Self) -> bool {
        self.precedence() > other.precedence()
    }

    /// Get string representation for prompts
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Global => "GLOBAL",
            Self::Bioregional => "BIOREGIONAL",
            Self::NationState => "NATION",
            Self::Provincial => "PROVINCIAL",
            Self::Community => "COMMUNITY",
            Self::Family => "FAMILY",
            Self::Individual => "INDIVIDUAL",
        }
    }

    /// All layers in precedence order (highest first)
    pub fn all_descending() -> Vec<Self> {
        vec![
            Self::Global,
            Self::Bioregional,
            Self::NationState,
            Self::Provincial,
            Self::Community,
            Self::Family,
            Self::Individual,
        ]
    }
}

impl Default for ConstitutionalLayer {
    fn default() -> Self {
        Self::Individual
    }
}

/// A constitutional document at a specific layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ConstitutionalDocument {
    /// Unique identifier
    pub id: String,
    /// Which layer this document belongs to
    pub layer: ConstitutionalLayer,
    /// Semantic version
    pub version: String,
    /// SHA256 hash for DHT verification
    pub hash: String,
    /// The actual content
    pub content: ConstitutionalContent,
    /// When the document was created
    pub created_at: DateTime<Utc>,
    /// When it was last verified against DHT
    pub verified_at: Option<DateTime<Utc>>,
    /// Optional context IDs (community_id, family_id, etc.)
    pub context_id: Option<String>,
}

/// The content of a constitutional document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ConstitutionalContent {
    /// Core principles at this layer
    pub principles: Vec<Principle>,
    /// Boundaries that cannot be crossed
    pub boundaries: Vec<Boundary>,
    /// Guidance for interpretation
    pub interpretive_guidance: Vec<String>,
    /// References to parent layer documents
    pub parent_refs: Vec<String>,
}

impl Default for ConstitutionalContent {
    fn default() -> Self {
        Self {
            principles: Vec::new(),
            boundaries: Vec::new(),
            interpretive_guidance: Vec::new(),
            parent_refs: Vec::new(),
        }
    }
}

/// A constitutional principle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct Principle {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// The principle statement
    pub statement: String,
    /// Relative importance (0.0-1.0)
    pub weight: f32,
    /// How hard it is to change
    pub immutability: ImmutabilityLevel,
}

/// Immutability gradient for principles.
///
/// Determines how difficult it is to amend a principle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "snake_case")]
pub enum ImmutabilityLevel {
    /// Cannot be changed by any means
    Absolute,
    /// Requires multi-generational consensus
    Constitutional,
    /// Requires supermajority + time delay
    Entrenched,
    /// Requires normal governance process
    Statutory,
    /// Can be adjusted by local authority
    Regulatory,
}

impl ImmutabilityLevel {
    /// Get amendment difficulty (1-5, higher = harder)
    pub fn difficulty(&self) -> u8 {
        match self {
            Self::Absolute => 5,
            Self::Constitutional => 4,
            Self::Entrenched => 3,
            Self::Statutory => 2,
            Self::Regulatory => 1,
        }
    }
}

/// A boundary that constrains behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct Boundary {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what the boundary protects
    pub description: String,
    /// Category of boundary
    pub boundary_type: BoundaryType,
    /// How it's enforced
    pub enforcement: EnforcementLevel,
}

/// Categories of boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "snake_case")]
pub enum BoundaryType {
    /// Existential risk prevention (extinction, genocide, etc.)
    Existential,
    /// Ecological limits
    Ecological,
    /// Human dignity protection
    Dignity,
    /// Privacy boundaries
    Privacy,
    /// Consent requirements
    Consent,
    /// Care and wellbeing
    Care,
}

/// How a boundary is enforced.
///
/// Matches TypeScript `EnforcementLevel` in protocol-core.model.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "snake_case")]
pub enum EnforcementLevel {
    /// Hard block, cryptographic prohibition, no override possible
    HardBlock,
    /// Must go through governance deliberation
    RequireGovernance,
    /// Soft limit, can be overridden with justification
    SoftLimit,
    /// Warning only, logged but not blocked
    Warning,
}

impl EnforcementLevel {
    /// Check if this level blocks actions
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::HardBlock | Self::RequireGovernance)
    }
}

/// Result of constitutional verification against DHT.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct VerificationResult {
    /// Document being verified
    pub document_id: String,
    /// Expected hash from local document
    pub expected_hash: String,
    /// Actual hash from DHT (if found)
    pub actual_hash: Option<String>,
    /// Whether verification passed
    pub verified: bool,
    /// When verification occurred
    pub verified_at: DateTime<Utc>,
    /// DHT source identifier
    pub dht_source: Option<String>,
}

/// A precedent from previous constitutional reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct Precedent {
    /// Unique identifier
    pub id: String,
    /// Layer where the precedent was established
    pub layer: ConstitutionalLayer,
    /// Summary of the case
    pub case_summary: String,
    /// IDs of principles that were applied
    pub principles_applied: Vec<String>,
    /// The reasoning used
    pub reasoning: String,
    /// What was decided
    pub outcome: PrecedentOutcome,
    /// When established
    pub created_at: DateTime<Utc>,
    /// How much weight this precedent carries (0.0-1.0)
    pub weight: f32,
    /// How many times this precedent has been cited
    pub citation_count: u32,
}

/// Outcome of a precedent-setting case.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
#[serde(rename_all = "snake_case")]
pub enum PrecedentOutcome {
    /// Action was approved with optional conditions
    Approved { conditions: Vec<String> },
    /// Action was denied with reason
    Denied { reason: String },
    /// Case was escalated to higher layer
    Escalated { to_layer: ConstitutionalLayer },
    /// Decision was deferred
    Deferred { until: String, reason: String },
}

/// A principle after conflict resolution between layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ResolvedPrinciple {
    /// The underlying principle
    pub principle: Principle,
    /// Which layer it came from
    pub source_layer: ConstitutionalLayer,
    /// Weight after conflict resolution
    pub effective_weight: f32,
    /// If overridden by a higher layer principle
    pub overridden_by: Option<String>,
}

/// Violation of a constitutional boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct BoundaryViolation {
    /// The boundary that was violated
    pub boundary: Boundary,
    /// How severe the enforcement should be
    pub severity: EnforcementLevel,
    /// Explanation of the violation
    pub explanation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_precedence() {
        assert!(ConstitutionalLayer::Global.can_override(&ConstitutionalLayer::Community));
        assert!(ConstitutionalLayer::Community.can_override(&ConstitutionalLayer::Individual));
        assert!(!ConstitutionalLayer::Individual.can_override(&ConstitutionalLayer::Global));
    }

    #[test]
    fn test_layer_ordering() {
        let layers = ConstitutionalLayer::all_descending();
        assert_eq!(layers[0], ConstitutionalLayer::Global);
        assert_eq!(layers[6], ConstitutionalLayer::Individual);
    }

    #[test]
    fn test_enforcement_blocking() {
        assert!(EnforcementLevel::HardBlock.is_blocking());
        assert!(EnforcementLevel::RequireGovernance.is_blocking());
        assert!(!EnforcementLevel::SoftLimit.is_blocking());
        assert!(!EnforcementLevel::Warning.is_blocking());
    }
}
