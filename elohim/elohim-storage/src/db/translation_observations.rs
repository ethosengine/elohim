//! Diesel model + insert helper for the `translation_observations` table.
//!
//! Schema lives in `src/db/diesel_schema.rs`.
//! See migration `2026-05-20-000000_translation_observations`.
//!
//! Source of truth: local write (Category C operational — produced by the
//! valueflows-bridge at translation call sites; aggregated at M5).

use chrono::Utc;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use valueflows_types::{
    ClientCapability, Direction, OntologicalCommitment, SemanticCost, TranslationKind,
    TranslationPoint,
};

use crate::db::diesel_schema::translation_observations;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = translation_observations)]
pub struct TranslationObservationRow {
    pub id: i32,
    pub observed_at: String,
    pub block_height: Option<i64>,
    pub direction: String,
    pub vf_type: String,
    pub elohim_source: String,
    pub translation_kind: String,
    pub semantic_cost: String,
    pub ontological_commitment: Option<String>,
    pub client_capability: String,
    pub code_location: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = translation_observations)]
pub struct NewTranslationObservation {
    pub observed_at: String,
    pub block_height: Option<i64>,
    pub direction: String,
    pub vf_type: String,
    pub elohim_source: String,
    pub translation_kind: String,
    pub semantic_cost: String,
    pub ontological_commitment: Option<String>,
    pub client_capability: String,
    pub code_location: String,
    pub notes: Option<String>,
}

impl From<TranslationPoint> for NewTranslationObservation {
    fn from(p: TranslationPoint) -> Self {
        Self {
            observed_at: p.at_iso,
            // u64 → i64. Block heights below 2^63 are safe; above that is far beyond
            // any realistic chain. Saturate on overflow rather than panic.
            block_height: p.at_block_height.map(|h| i64::try_from(h).unwrap_or(i64::MAX)),
            direction: direction_string(p.direction).to_string(),
            vf_type: p.vf_type,
            elohim_source: p.elohim_source,
            translation_kind: translation_kind_string(p.translation_kind).to_string(),
            semantic_cost: semantic_cost_string(p.semantic_cost).to_string(),
            ontological_commitment: p
                .ontological_commitment
                .map(|o| ontological_commitment_string(o).to_string()),
            client_capability: client_capability_string(p.client_capability).to_string(),
            code_location: p.code_location,
            notes: p.notes,
        }
    }
}

/// Insert a TranslationPoint into the ledger.
pub fn insert_observation(
    conn: &mut SqliteConnection,
    point: TranslationPoint,
) -> Result<(), DieselError> {
    let row: NewTranslationObservation = point.into();
    diesel::insert_into(translation_observations::table)
        .values(&row)
        .execute(conn)?;
    Ok(())
}

/// Convenience: build a now-stamped TranslationPoint with the given fields.
///
/// `at_block_height` is always `None` — block height is populated from M3+
/// when live conductor context is available. M3 callers needing to attach
/// a real block height should construct `TranslationPoint` directly rather
/// than going through `observe_now`.
#[allow(clippy::too_many_arguments)]
pub fn observe_now(
    direction: Direction,
    vf_type: &str,
    elohim_source: &str,
    translation_kind: TranslationKind,
    semantic_cost: SemanticCost,
    ontological_commitment: Option<OntologicalCommitment>,
    client_capability: ClientCapability,
    code_location: &str,
) -> TranslationPoint {
    TranslationPoint {
        at_iso: Utc::now().to_rfc3339(),
        at_block_height: None,
        direction,
        vf_type: vf_type.to_string(),
        elohim_source: elohim_source.to_string(),
        translation_kind,
        semantic_cost,
        ontological_commitment,
        client_capability,
        code_location: code_location.to_string(),
        notes: None,
    }
}

fn direction_string(d: Direction) -> &'static str {
    match d {
        Direction::Read => "Read",
        Direction::Write => "Write",
    }
}

fn translation_kind_string(k: TranslationKind) -> &'static str {
    match k {
        TranslationKind::IdentityShape => "IdentityShape",
        TranslationKind::FieldRename => "FieldRename",
        TranslationKind::SemanticBridge => "SemanticBridge",
        TranslationKind::Reconciliation => "Reconciliation",
        TranslationKind::Sidecar => "Sidecar",
    }
}

fn semantic_cost_string(c: SemanticCost) -> &'static str {
    match c {
        SemanticCost::Mechanical => "Mechanical",
        SemanticCost::JustifiedDistinct => "JustifiedDistinct",
        SemanticCost::UnclearYet => "UnclearYet",
    }
}

fn ontological_commitment_string(o: OntologicalCommitment) -> &'static str {
    match o {
        OntologicalCommitment::SovereigntyToStewardship => "SovereigntyToStewardship",
        OntologicalCommitment::KeyAuthorityToSocialAuthority => "KeyAuthorityToSocialAuthority",
        OntologicalCommitment::FixedAudienceToReachClass => "FixedAudienceToReachClass",
        OntologicalCommitment::BilateralToRelational => "BilateralToRelational",
        OntologicalCommitment::IndividualWillToContribution => "IndividualWillToContribution",
        OntologicalCommitment::EntryToEprAtom => "EntryToEprAtom",
    }
}

fn client_capability_string(c: ClientCapability) -> &'static str {
    match c {
        ClientCapability::StockVf => "StockVf",
        ClientCapability::ElohimAware => "ElohimAware",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_point_converts_to_insert_row() {
        let p = observe_now(
            Direction::Read,
            "EconomicEvent",
            "fixture",
            TranslationKind::IdentityShape,
            SemanticCost::Mechanical,
            None,
            ClientCapability::StockVf,
            "test:1",
        );
        let row: NewTranslationObservation = p.into();
        assert_eq!(row.direction, "Read");
        assert_eq!(row.vf_type, "EconomicEvent");
        assert_eq!(row.translation_kind, "IdentityShape");
        assert_eq!(row.semantic_cost, "Mechanical");
        assert!(row.ontological_commitment.is_none());
        assert!(row.block_height.is_none());
    }

    #[test]
    fn block_height_u64_to_i64_conversion() {
        let p = TranslationPoint {
            at_iso: "2026-05-20T00:00:00Z".to_string(),
            at_block_height: Some(42_u64),
            direction: Direction::Write,
            vf_type: "Proposal".to_string(),
            elohim_source: "hREA::Proposal".to_string(),
            translation_kind: TranslationKind::SemanticBridge,
            semantic_cost: SemanticCost::JustifiedDistinct,
            ontological_commitment: Some(OntologicalCommitment::SovereigntyToStewardship),
            client_capability: ClientCapability::ElohimAware,
            code_location: "src/schema/proposal.rs:99".to_string(),
            notes: Some("test note".to_string()),
        };
        let row: NewTranslationObservation = p.into();
        assert_eq!(row.block_height, Some(42_i64));
        assert_eq!(
            row.ontological_commitment.as_deref(),
            Some("SovereigntyToStewardship")
        );
        assert_eq!(row.client_capability, "ElohimAware");
        assert_eq!(row.notes.as_deref(), Some("test note"));
    }

    #[test]
    fn block_height_overflow_saturates() {
        let p = TranslationPoint {
            at_iso: "2026-05-20T00:00:00Z".to_string(),
            at_block_height: Some(u64::MAX),
            direction: Direction::Read,
            vf_type: "EconomicEvent".to_string(),
            elohim_source: "fixture".to_string(),
            translation_kind: TranslationKind::IdentityShape,
            semantic_cost: SemanticCost::Mechanical,
            ontological_commitment: None,
            client_capability: ClientCapability::StockVf,
            code_location: "test:1".to_string(),
            notes: None,
        };
        let row: NewTranslationObservation = p.into();
        assert_eq!(row.block_height, Some(i64::MAX));
    }
}
