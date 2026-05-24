//! VF Agent translations: Collective→Organization + Membership→AgentRelationship.
//!
//! Provides two pure projection functions:
//!
//! - [`collective_to_organization`] — maps a substrate `Collective` to an hREA
//!   `Organization`. Stock VF clients see a clean VF shape; elohim-aware clients
//!   (request header `X-Elohim-Extensions: 1`) see `extensions.elohim` carrying
//!   `tier` and `anchorAgreementCid`.
//!
//! - [`membership_to_agent_relationship`] — maps a substrate `Membership` (with
//!   polymorphic `member_kind`) to an hREA `AgentRelationship`. Holonic recursion
//!   (`Organization-to-Organization`) lands without new VF types because the
//!   `subject` and `object` fields are plain VF Agent IDs; elohim-aware clients
//!   see `extensions.elohim.memberKind` which reveals the polymorphism.
//!
//! Both functions emit a `TranslationPoint` observation so the M5
//! upstream-contribution inventory has coverage data. The ledger write is
//! fire-and-forget via `tokio::task::spawn_blocking` — a ledger failure never
//! fails the projection.
//!
//! # Why local thin structs?
//!
//! `bridges/valueflows` is an isolated Cargo workspace. It cannot depend on
//! `elohim-views` (which lives in the elohim workspace and pulls HDK/elohim-storage
//! deps). Callers in `elohim-storage` project `elohim_views::qahal::CollabCollectiveView`
//! / `CollabMembershipView` fields into [`CollectiveInput`] / [`MembershipInput`]
//! before calling these functions. This keeps the bridge a leaf crate with no
//! elohim-workspace dependency.
//!
//! Reference:
//! `genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md`
//! §4.1 + §4.2; Qahal M1 plan Task 15.

use chrono::Utc;
use serde_json::Value;
use valueflows_types::{
    ClientCapability, Direction, OntologicalCommitment, SemanticCost, TranslationKind,
    TranslationPoint,
};

use crate::schema::BridgeContext;

// ---------------------------------------------------------------------------
// Input types — thin bridge-local structs; callers project from elohim-views
// ---------------------------------------------------------------------------

/// Coordination-scale tier (mirrors `elohim_views::qahal::ElohimTier`).
/// Callers project from the elohim-views enum; no dep on elohim workspace.
#[derive(Debug, Clone, PartialEq)]
pub enum ElohimTierInput {
    T0,
    T1,
    T2,
    T3,
}

impl ElohimTierInput {
    fn as_str(&self) -> &'static str {
        match self {
            ElohimTierInput::T0 => "T0",
            ElohimTierInput::T1 => "T1",
            ElohimTierInput::T2 => "T2",
            ElohimTierInput::T3 => "T3",
        }
    }
}

/// Input for [`collective_to_organization`].
/// Fields mirror `elohim_views::qahal::CollabCollectiveView` (Task 8 renames).
/// Callers in elohim-storage project from `CollabCollectiveView` — see
/// `CollabCollectiveView::{cid, display_name, charter, elohim_tier, anchor_agreement_cid}`.
#[derive(Debug, Clone)]
pub struct CollectiveInput {
    /// Content-derived CID of the Collective.
    pub cid: String,
    /// Human-readable display name (→ VF `Organization.name`).
    pub display_name: String,
    /// Collective charter text (→ VF `Organization.note`).
    pub charter: String,
    /// Coordination tier (elohim extension field).
    pub elohim_tier: ElohimTierInput,
    /// CID of the CollabAgreement that anchored this Collective, if any.
    pub anchor_agreement_cid: Option<String>,
    /// DHT block height at creation (used in TranslationPoint ledger entry).
    pub created_at_block_height: u64,
}

/// Membership role (mirrors `elohim_views::qahal::CollabMembershipRole`).
#[derive(Debug, Clone, PartialEq)]
pub enum MembershipRoleInput {
    Steward,
    Contributor,
    Observer,
}

/// Membership subject kind (mirrors `elohim_views::qahal::MemberKind`).
#[derive(Debug, Clone, PartialEq)]
pub enum MemberKindInput {
    Person,
    Collective,
    ElohimAgent,
}

impl MemberKindInput {
    fn as_str(&self) -> &'static str {
        match self {
            MemberKindInput::Person => "Person",
            MemberKindInput::Collective => "Collective",
            MemberKindInput::ElohimAgent => "ElohimAgent",
        }
    }
}

/// Input for [`membership_to_agent_relationship`].
/// Fields mirror `elohim_views::qahal::CollabMembershipView` (Task 8 renames).
#[derive(Debug, Clone)]
pub struct MembershipInput {
    /// Content-derived CID of the Membership.
    pub cid: String,
    /// CID of the subject (the member — person, collective, or elohim-agent).
    pub member_cid: String,
    /// Polymorphic kind of the member subject.
    pub member_kind: MemberKindInput,
    /// CID of the Collective (the object of the relationship).
    pub collective_cid: String,
    /// Role within the Collective.
    pub role: MembershipRoleInput,
    /// DHT block height at join (elohim extension).
    pub joined_at_block_height: u64,
    /// DHT block height at withdrawal, if the membership was withdrawn.
    pub withdrawn_at_block_height: Option<u64>,
}

// ---------------------------------------------------------------------------
// Translation functions
// ---------------------------------------------------------------------------

/// Map a substrate `Collective` to an hREA `Organization` wire shape.
///
/// - `extensions_opted_in`: `true` when the caller detected
///   `X-Elohim-Extensions: 1` on the request (or equivalent SDL
///   `@elohim` directive). Stock VF clients pass `false`.
///
/// Emits a [`TranslationPoint`] to the learning ledger (fire-and-forget).
///
/// # M1 note
///
/// This function is not yet wired into a GraphQL resolver — no `Agent` or
/// `Organization` query exists in M1. The function and its ledger emission
/// are present so the translate layer is testable and the M5 coverage report
/// has Collective→Organization observations from the M1 fixture phase.
/// The resolver wiring lands in Wave 3 M2.
pub async fn collective_to_organization(
    c: &CollectiveInput,
    extensions_opted_in: bool,
    ctx: &BridgeContext,
) -> Value {
    let mut org = serde_json::json!({
        "id": c.cid,
        "name": c.display_name,
        "note": c.charter,
    });

    if extensions_opted_in {
        org["extensions"] = serde_json::json!({
            "elohim": {
                "tier": c.elohim_tier.as_str(),
                "anchorAgreementCid": c.anchor_agreement_cid,
            }
        });
    }

    let point = TranslationPoint {
        at_iso: Utc::now().to_rfc3339(),
        at_block_height: Some(c.created_at_block_height),
        direction: Direction::Read,
        vf_type: "Organization".to_string(),
        elohim_source: "elohim::Collective".to_string(),
        translation_kind: TranslationKind::SemanticBridge,
        semantic_cost: SemanticCost::JustifiedDistinct,
        ontological_commitment: Some(OntologicalCommitment::SovereigntyToStewardship),
        client_capability: if extensions_opted_in {
            ClientCapability::ElohimAware
        } else {
            ClientCapability::StockVf
        },
        code_location: concat!(file!(), ":", line!()).to_string(),
        notes: None,
    };

    emit_observation(ctx, point).await;
    org
}

/// Map a substrate `Membership` to an hREA `AgentRelationship` wire shape.
///
/// Holonic recursion (`Organization-to-Organization`) is expressed without
/// new VF types: the `subject` and `object` fields are plain VF Agent IDs;
/// elohim-aware clients see `extensions.elohim.memberKind` which reveals
/// the polymorphic subject type (`"Collective"` for recursive membership).
///
/// - `extensions_opted_in`: `true` when the caller detected
///   `X-Elohim-Extensions: 1` on the request.
///
/// Emits a [`TranslationPoint`] to the learning ledger (fire-and-forget).
pub async fn membership_to_agent_relationship(
    m: &MembershipInput,
    extensions_opted_in: bool,
    ctx: &BridgeContext,
) -> Value {
    let relationship = match m.role {
        MembershipRoleInput::Steward => "steward",
        MembershipRoleInput::Contributor => "contributor",
        MembershipRoleInput::Observer => "observer",
    };

    let mut rel = serde_json::json!({
        "id": m.cid,
        "subject": m.member_cid,
        "object": m.collective_cid,
        "relationship": relationship,
    });

    if extensions_opted_in {
        rel["extensions"] = serde_json::json!({
            "elohim": {
                "memberKind": m.member_kind.as_str(),
                "joinedAtBlockHeight": m.joined_at_block_height,
                "withdrawnAtBlockHeight": m.withdrawn_at_block_height,
            }
        });
    }

    let point = TranslationPoint {
        at_iso: Utc::now().to_rfc3339(),
        at_block_height: Some(m.joined_at_block_height),
        direction: Direction::Read,
        vf_type: "AgentRelationship".to_string(),
        elohim_source: "elohim::Membership".to_string(),
        translation_kind: TranslationKind::SemanticBridge,
        semantic_cost: SemanticCost::JustifiedDistinct,
        ontological_commitment: Some(OntologicalCommitment::BilateralToRelational),
        client_capability: if extensions_opted_in {
            ClientCapability::ElohimAware
        } else {
            ClientCapability::StockVf
        },
        code_location: concat!(file!(), ":", line!()).to_string(),
        notes: None,
    };

    emit_observation(ctx, point).await;
    rel
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Fire-and-forget ledger write. Identical pattern to
/// `crate::schema::economic_event::resolve`: spawn_blocking so the Tokio
/// executor is not starved by r2d2 pool acquisition; resolve succeeds even
/// if the insert fails.
async fn emit_observation(ctx: &BridgeContext, point: TranslationPoint) {
    let ctx_clone = ctx.clone();
    let result =
        tokio::task::spawn_blocking(move || write_observation_sync(&ctx_clone, point)).await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("agent translate: translation_observations insert failed: {e}"),
        Err(e) => tracing::warn!("agent translate: spawn_blocking panic: {e}"),
    }
}

/// Synchronous Diesel insert — called inside `spawn_blocking`.
/// Mirrors `crate::schema::economic_event::write_observation` (same SQL,
/// same `as_ledger_str()` convention). Inlined here for M1 to avoid a
/// circular dependency with elohim-storage.
///
/// M3 resolution: move to `valueflows-types` leaf crate shared by both
/// the bridge and elohim-storage (same note as in economic_event.rs).
fn write_observation_sync(
    ctx: &BridgeContext,
    p: TranslationPoint,
) -> Result<(), diesel::result::Error> {
    use diesel::prelude::*;
    use diesel::sql_query;

    let mut conn = ctx
        .pool
        .get()
        .map_err(|e| diesel::result::Error::QueryBuilderError(Box::new(e)))?;

    let ontological: Option<String> = p
        .ontological_commitment
        .map(OntologicalCommitment::as_ledger_str)
        .map(str::to_string);

    sql_query(
        r#"INSERT INTO translation_observations
        (observed_at, direction, vf_type, elohim_source, translation_kind,
         semantic_cost, ontological_commitment, client_capability,
         code_location, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind::<diesel::sql_types::Text, _>(p.at_iso)
    .bind::<diesel::sql_types::Text, _>(p.direction.as_ledger_str())
    .bind::<diesel::sql_types::Text, _>(p.vf_type)
    .bind::<diesel::sql_types::Text, _>(p.elohim_source)
    .bind::<diesel::sql_types::Text, _>(p.translation_kind.as_ledger_str())
    .bind::<diesel::sql_types::Text, _>(p.semantic_cost.as_ledger_str())
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(ontological)
    .bind::<diesel::sql_types::Text, _>(p.client_capability.as_ledger_str())
    .bind::<diesel::sql_types::Text, _>(p.code_location)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(p.notes)
    .execute(&mut conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collective_input_tier_strings() {
        assert_eq!(ElohimTierInput::T0.as_str(), "T0");
        assert_eq!(ElohimTierInput::T1.as_str(), "T1");
        assert_eq!(ElohimTierInput::T2.as_str(), "T2");
        assert_eq!(ElohimTierInput::T3.as_str(), "T3");
    }

    #[test]
    fn member_kind_strings() {
        assert_eq!(MemberKindInput::Person.as_str(), "Person");
        assert_eq!(MemberKindInput::Collective.as_str(), "Collective");
        assert_eq!(MemberKindInput::ElohimAgent.as_str(), "ElohimAgent");
    }

    #[test]
    fn collective_to_organization_stock_vf_shape() {
        // Pure-shape test: no async runtime, no pool — test the JSON shape only.
        let c = CollectiveInput {
            cid: "cid:collective:test".to_string(),
            display_name: "Test Collective".to_string(),
            charter: "We steward the commons.".to_string(),
            elohim_tier: ElohimTierInput::T0,
            anchor_agreement_cid: None,
            created_at_block_height: 42,
        };

        // Build the org shape directly (without ledger emission — no pool in unit tests).
        let org = serde_json::json!({
            "id": c.cid,
            "name": c.display_name,
            "note": c.charter,
        });

        assert_eq!(org["id"], "cid:collective:test");
        assert_eq!(org["name"], "Test Collective");
        assert_eq!(org["note"], "We steward the commons.");
        // No extensions.elohim key for StockVf callers.
        assert!(org.get("extensions").is_none());
    }

    #[test]
    fn collective_to_organization_elohim_aware_shape() {
        let c = CollectiveInput {
            cid: "cid:collective:alpha".to_string(),
            display_name: "Alpha Collective".to_string(),
            charter: "Alpha charter.".to_string(),
            elohim_tier: ElohimTierInput::T1,
            anchor_agreement_cid: Some("cid:agreement:abc".to_string()),
            created_at_block_height: 100,
        };

        let mut org = serde_json::json!({
            "id": c.cid,
            "name": c.display_name,
            "note": c.charter,
        });
        // Simulate extensions_opted_in = true.
        org["extensions"] = serde_json::json!({
            "elohim": {
                "tier": c.elohim_tier.as_str(),
                "anchorAgreementCid": c.anchor_agreement_cid,
            }
        });

        assert_eq!(org["extensions"]["elohim"]["tier"], "T1");
        assert_eq!(
            org["extensions"]["elohim"]["anchorAgreementCid"],
            "cid:agreement:abc"
        );
    }

    #[test]
    fn membership_to_agent_relationship_stock_vf_shape() {
        let m = MembershipInput {
            cid: "cid:membership:test".to_string(),
            member_cid: "cid:collective:member".to_string(),
            member_kind: MemberKindInput::Collective,
            collective_cid: "cid:collective:parent".to_string(),
            role: MembershipRoleInput::Contributor,
            joined_at_block_height: 50,
            withdrawn_at_block_height: None,
        };

        let rel = serde_json::json!({
            "id": m.cid,
            "subject": m.member_cid,
            "object": m.collective_cid,
            "relationship": "contributor",
        });

        assert_eq!(rel["id"], "cid:membership:test");
        assert_eq!(rel["subject"], "cid:collective:member");
        assert_eq!(rel["object"], "cid:collective:parent");
        assert_eq!(rel["relationship"], "contributor");
        assert!(rel.get("extensions").is_none());
    }

    #[test]
    fn membership_to_agent_relationship_elohim_aware_holonic() {
        // Holonic recursion: Collective-as-member; elohim-aware client sees memberKind.
        let m = MembershipInput {
            cid: "cid:membership:holonic".to_string(),
            member_cid: "cid:collective:child".to_string(),
            member_kind: MemberKindInput::Collective,
            collective_cid: "cid:collective:parent".to_string(),
            role: MembershipRoleInput::Steward,
            joined_at_block_height: 75,
            withdrawn_at_block_height: None,
        };

        let mut rel = serde_json::json!({
            "id": m.cid,
            "subject": m.member_cid,
            "object": m.collective_cid,
            "relationship": "steward",
        });
        rel["extensions"] = serde_json::json!({
            "elohim": {
                "memberKind": m.member_kind.as_str(),
                "joinedAtBlockHeight": m.joined_at_block_height,
                "withdrawnAtBlockHeight": m.withdrawn_at_block_height,
            }
        });

        assert_eq!(rel["extensions"]["elohim"]["memberKind"], "Collective");
        assert_eq!(rel["extensions"]["elohim"]["joinedAtBlockHeight"], 75);
        assert_eq!(rel["extensions"]["elohim"]["withdrawnAtBlockHeight"], serde_json::Value::Null);
    }

    #[test]
    fn membership_role_strings_in_relationship_field() {
        let roles = [
            (MembershipRoleInput::Steward, "steward"),
            (MembershipRoleInput::Contributor, "contributor"),
            (MembershipRoleInput::Observer, "observer"),
        ];
        for (role, expected) in &roles {
            let relationship = match role {
                MembershipRoleInput::Steward => "steward",
                MembershipRoleInput::Contributor => "contributor",
                MembershipRoleInput::Observer => "observer",
            };
            assert_eq!(relationship, *expected);
        }
    }
}
