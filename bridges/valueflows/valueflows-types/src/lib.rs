//! Stable type definitions consumed by `valueflows-bridge` and any future
//! consumer of the bridge's learning-ledger schema. Kept in a separate crate
//! so the schema can be referenced by analysis tooling without pulling in
//! the full bridge (which depends on async-graphql + hyper).
//!
//! The ledger records each translation event — direction, VF type, semantic
//! cost — so we can produce an upstream-contribution inventory + R&O
//! compatibility report at M5.
//!
//! Reference spec:
//! `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
//! §4.2 (Learning Ledger Schema).

use serde::{Deserialize, Serialize};

/// One observation of the bridge translating between VF-GraphQL and elohim's
/// EPR-REA substrate. Written to the `translation_observations` Diesel table
/// in elohim-storage; aggregated at end-of-Wave-3 (M5) into the
/// upstream-contribution inventory + R&O compatibility report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranslationPoint {
    pub at_iso: String, // ISO-8601 UTC; chrono::Utc::now().to_rfc3339()
    pub direction: Direction,
    pub vf_type: String, // "EconomicEvent", "Proposal", ...
    pub elohim_source: String, // "hREA::EconomicEvent" | "elohim::EprAtom" | ...
    pub translation_kind: TranslationKind,
    pub semantic_cost: SemanticCost,
    pub ontological_commitment: Option<OntologicalCommitment>,
    pub client_capability: ClientCapability,
    pub code_location: String, // file:line, captured via macro
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Direction {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TranslationKind {
    /// Identical shape — pure routing.
    IdentityShape,
    /// Shape identical, names differ.
    FieldRename,
    /// Genuine domain difference (Reach, ElohimAgent, ...).
    SemanticBridge,
    /// Same fact in two DHTs; merge for read.
    Reconciliation,
    /// Elohim-only data linked to canonical entry.
    Sidecar,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SemanticCost {
    /// Shape-equivalent translation; pure routing.
    Mechanical,
    /// Real semantic difference — keep distinct.
    JustifiedDistinct,
    /// Need more usage to judge.
    UnclearYet,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OntologicalCommitment {
    SovereigntyToStewardship,
    KeyAuthorityToSocialAuthority,
    FixedAudienceToReachClass,
    BilateralToRelational,
    IndividualWillToContribution,
    EntryToEprAtom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientCapability {
    /// Stock VF/hREA client; ignores elohim extension fields.
    StockVf,
    /// Client advertised support for `extensions.elohim.*` (via SDL
    /// `@elohim` directive or `X-Elohim-Extensions: 1` request header).
    ElohimAware,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_point_roundtrips_through_serde() {
        let p = TranslationPoint {
            at_iso: "2026-05-20T00:00:00Z".to_string(),
            direction: Direction::Read,
            vf_type: "EconomicEvent".to_string(),
            elohim_source: "fixture".to_string(),
            translation_kind: TranslationKind::IdentityShape,
            semantic_cost: SemanticCost::Mechanical,
            ontological_commitment: None,
            client_capability: ClientCapability::StockVf,
            code_location: "src/schema/economic_event.rs:42".to_string(),
            notes: None,
        };
        let json = serde_json::to_string(&p).expect("serialize");
        let back: TranslationPoint = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p, back);
    }

    #[test]
    fn enums_serialize_as_strings() {
        let json = serde_json::to_value(Direction::Read).unwrap();
        assert_eq!(json, serde_json::json!("Read"));
        let json = serde_json::to_value(TranslationKind::SemanticBridge).unwrap();
        assert_eq!(json, serde_json::json!("SemanticBridge"));
    }
}
