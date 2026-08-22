//! The ONE mapping from a DHT `Membership` entry to the local
//! `collective_participations` projection.
//!
//! Twin of [`crate::db::collectives::project_collective`], and it exists for the
//! same reason: two arms land the same DHT truth and must not drift.
//!
//! - the **post-commit signal arm** — `ReconcileController::on_membership_projected`,
//!   fed by `MembershipCommitted` from the AUTHORING conductor's `post_commit`;
//! - the **cross-peer reconcile arm** — `p2p::projection_reconcile`'s
//!   participations leg, which learns WHICH Membership anchors exist from peers
//!   and reads each entry BODY from its OWN conductor
//!   (`imagodei::get_membership_by_action`).
//!
//! `post_commit` fires only on the authoring conductor, so before the reconcile
//! arm existed a Membership authored on jessica's node was invisible on
//! matthew's forever — the empty `{"items": [], "count": 0}` the
//! household-formation E2E measured against alpha on 2026-08-20. Both arms now
//! funnel here, so the identity resolution, the anchor idempotency key and the
//! household backfill cannot diverge between them.
//!
//! ## Three resolutions this mapping owns
//!
//! 1. **`collective_id`** — the local `collectives` row carrying
//!    `collective_cid`, else the cid verbatim (a Collective/Membership commit
//!    race must not lose the row).
//! 2. **`human_id`** — the local `humans` row whose `agent_pub_key` matches the
//!    member key, else the `member_cid` verbatim. This is what lets a healed
//!    row land under the canonical slug (`human-jessica-spouse`) rather than
//!    `agent:uhCAk…`, and what merges a DHT Membership onto the participation
//!    an account-import already created for the same person.
//! 3. **the anchor** — `dht_anchor_hash = Membership ActionHash`, the
//!    idempotency key for BOTH arms and the reconcile identity on the wire.
//!
//! ## `h_app_id` partitions (named, because this is the drift class)
//!
//! Participations: [`crate::db::PARTICIPATIONS_HAPP_ID`] (`qahal`), owned by the
//! functions in [`crate::db::collectives`] which take no ctx at all.
//! Collectives lookups: `lamad` — the scope `on_collective_projected` and the
//! collectives reconcile arm both write. Humans stamps: scope-agnostic (matched
//! by `id` / `agent_pub_key`, never by partition).

use diesel::prelude::*;
use tracing::{debug, warn};

use super::context::AppContext;
use super::diesel_schema::{collective_participations, collectives, humans};
use crate::error::StorageError;

/// Everything the shared mapping needs to project ONE `Membership` DHT entry.
#[derive(Debug, Clone)]
pub struct MembershipProjection<'a> {
    /// ActionHash of the `Membership` entry. THE idempotency key.
    pub action_hash: &'a str,
    /// Canonical `collective:{action_hash}` identity of the parent Collective.
    pub collective_cid: &'a str,
    /// Canonical member CID (`agent:{pubkey}` or `collective:{hash}`).
    pub member_cid: &'a str,
    /// Storage `member_kind`: `"person"` | `"collective"` | `"elohim_agent"`.
    pub member_kind: &'a str,
    /// Storage `role_context`: `"steward"` | `"contributor"` | `"observer"`.
    pub role_context: &'a str,
    /// ISO 8601 departure timestamp; `None` while the membership is active.
    pub departed_at: Option<&'a str>,
}

/// Outcome of [`project_membership`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipProjectionOutcome {
    /// No local row carried this anchor: a row was created, or an existing
    /// `(collective_id, human_id)` row was adopted and stamped with it.
    /// Convergence.
    Projected,
    /// A row already carried this exact anchor; mutable fields were refreshed
    /// but nothing converged. Deliberately distinguished — a no-op counted as a
    /// cure hides a spinning heal plane (the `Refreshed` discipline the content
    /// and collectives arms already keep).
    Refreshed,
}

/// Project ONE `Membership` into `collective_participations`, then backfill the
/// member's identity columns from it.
///
/// The participation row is the PRIMARY product: the humans backfill below is
/// gated on it and every failure there WARNs without aborting.
pub fn project_membership(
    conn: &mut SqliteConnection,
    p: &MembershipProjection<'_>,
) -> Result<MembershipProjectionOutcome, StorageError> {
    // The collectives projection's scope — NOT the participations scope. Named
    // once, here, so the two partitions cannot be confused at a call site.
    let collectives_ctx = AppContext::default_lamad();

    let collective_id = resolve_collective_id(conn, &collectives_ctx, p.collective_cid);
    let human_id = resolve_human_id(conn, p.member_cid);

    let existing_id: Option<String> = collective_participations::table
        .filter(collective_participations::h_app_id.eq(crate::db::PARTICIPATIONS_HAPP_ID))
        .filter(collective_participations::dht_anchor_hash.eq(Some(p.action_hash)))
        .select(collective_participations::id)
        .first::<String>(conn)
        .optional()
        .unwrap_or(None);

    let outcome = if let Some(existing_id) = existing_id {
        refresh_participation(conn, &existing_id, p)?;
        debug!(
            member_cid = %p.member_cid,
            dht_anchor_hash = %p.action_hash,
            "collective_participations row refreshed on re-delivery"
        );
        MembershipProjectionOutcome::Refreshed
    } else {
        insert_or_adopt_participation(conn, &collective_id, &human_id, p)?;
        debug!(
            member_cid = %p.member_cid,
            collective_id = %collective_id,
            human_id = %human_id,
            dht_anchor_hash = %p.action_hash,
            "collective_participations row projected from DHT truth"
        );
        MembershipProjectionOutcome::Projected
    };

    backfill_member_identity(conn, &collectives_ctx, p);
    Ok(outcome)
}

/// The local `collectives.id` carrying this cid, else the cid verbatim.
///
/// A Collective/Membership commit race (Membership projected before its
/// Collective) must not LOSE the participation row — falling back to the cid as
/// the id keeps it, and the collectives arm converges the parent later.
fn resolve_collective_id(
    conn: &mut SqliteConnection,
    collectives_ctx: &AppContext,
    collective_cid: &str,
) -> String {
    collectives::table
        .filter(collectives::h_app_id.eq(&collectives_ctx.h_app_id))
        .filter(collectives::collective_cid.eq(collective_cid))
        .select(collectives::id)
        .first::<String>(conn)
        .unwrap_or_else(|_| collective_cid.to_string())
}

/// The local `humans.id` for this member key, else the `member_cid` verbatim.
///
/// `member_cid` is the imagodei `agent:{pubkey}` form; production `humans` rows
/// are keyed by slug (`human-jessica-spouse`) with the RAW `uhCAk…` key in
/// `agent_pub_key`. Resolving through that column is what makes a HEALED row
/// name the same person the account-import path named — and what lets
/// [`insert_or_adopt_participation`] adopt that existing row instead of minting
/// a second participation for one human.
///
/// Degrade-don't-guess: no matching row (this peer has never met the member)
/// keeps the `member_cid` as `human_id`, exactly the pre-cure behaviour, and a
/// later sweep re-resolves once the `humans` row lands.
fn resolve_human_id(conn: &mut SqliteConnection, member_cid: &str) -> String {
    let bare = member_cid
        .strip_prefix("agent:")
        .unwrap_or(member_cid)
        .trim();
    if bare.is_empty() {
        return member_cid.to_string();
    }
    match crate::db::humans::get_human_by_agent_key(conn, bare) {
        Ok(Some(row)) => row.id,
        Ok(None) => member_cid.to_string(),
        Err(e) => {
            warn!(
                member_cid = %member_cid,
                error = %e,
                "project_membership: humans lookup failed; keeping member_cid as human_id"
            );
            member_cid.to_string()
        }
    }
}

/// Re-delivery / re-discovery of an anchor we already hold: refresh the mutable
/// fields only.
///
/// `human_id` is deliberately NOT moved here. Re-pointing it could collide with
/// `UNIQUE(h_app_id, collective_id, human_id)` when a canonical row for the same
/// person already exists, and identity churn on every re-delivery is worse than
/// a stale alias: the reader matches on either shape.
fn refresh_participation(
    conn: &mut SqliteConnection,
    existing_id: &str,
    p: &MembershipProjection<'_>,
) -> Result<(), StorageError> {
    diesel::update(
        collective_participations::table.filter(collective_participations::id.eq(existing_id)),
    )
    .set((
        collective_participations::member_cid.eq(Some(p.member_cid)),
        collective_participations::member_kind.eq(p.member_kind),
        collective_participations::role_context.eq(Some(p.role_context)),
        collective_participations::departed_at.eq(p.departed_at),
        collective_participations::updated_at.eq(now_iso()),
    ))
    .execute(conn)
    .map_err(|e| {
        StorageError::Internal(format!(
            "participation refresh failed for {existing_id}: {e}"
        ))
    })?;
    Ok(())
}

/// First sighting of this anchor: upsert the `(collective_id, human_id)` row and
/// stamp the DHT-derived columns onto it.
///
/// `create_participation` is an upsert on `(collective_id, human_id)`, so when a
/// row for this person already exists — the account-import path's row, or a
/// seeded one — this ADOPTS it and stamps the anchor rather than minting a
/// second participation. One person, one participation, one row.
fn insert_or_adopt_participation(
    conn: &mut SqliteConnection,
    collective_id: &str,
    human_id: &str,
    p: &MembershipProjection<'_>,
) -> Result<(), StorageError> {
    use crate::db::collectives::{create_participation, CreateParticipationInput};
    use crate::db::models::{consent_states, intimacy_levels};

    let input = CreateParticipationInput {
        id: None, // auto-generated
        collective_id: collective_id.to_string(),
        human_id: human_id.to_string(),
        intimacy_level: intimacy_levels::RECOGNITION.to_string(),
        role_context: Some(p.role_context.to_string()),
        governance_weight: 1.0,
        consent_state: consent_states::CONSENTED.to_string(),
        metadata_json: None,
    };

    let row = create_participation(conn, &input)?;

    // Stamp the DHT-derived columns `create_participation` does not accept.
    diesel::update(
        collective_participations::table.filter(collective_participations::id.eq(&row.id)),
    )
    .set((
        collective_participations::member_cid.eq(Some(p.member_cid)),
        collective_participations::member_kind.eq(p.member_kind),
        collective_participations::dht_anchor_hash.eq(Some(p.action_hash)),
        collective_participations::departed_at.eq(p.departed_at),
    ))
    .execute(conn)
    .map_err(|e| {
        StorageError::Internal(format!(
            "participation anchor stamp failed for {}: {e}",
            row.id
        ))
    })?;
    Ok(())
}

/// Backfill `humans.agent_pub_key` and `humans.household_id` from this
/// membership.
///
/// Both resilience `snapshot()` joins read `humans.household_id`, and a
/// membership signal is its canonical source: the member now belongs to this
/// collective. Four guards make the stamp production-correct rather than a
/// no-op or an over-broad write:
///
///  (a) KEYING — `member_cid` is the imagodei `agent:{pubkey}` form; production
///      `humans` rows carry the RAW `uhCAk…` in `agent_pub_key` (seeder
///      `bootstrap.ts`), which is what the resilience join reads. Both
///      vocabularies are matched: `id == member_cid` OR
///      `agent_pub_key == <member_cid sans agent:>`.
///  (b) COLLECTIVE KIND — only HOUSEHOLD collectives populate
///      `humans.household_id`; a community membership arriving first must not
///      permanently stamp the wrong household (the NULL-only guard would then
///      block correction).
///  (c) DEPARTED — a departure never stamps.
///  (d) NULL-ONLY — never overwrite a set value. Overwriting a set
///      `agent_pub_key` is exclusively `rekey_human_agent_key`'s job, and this
///      stamp must never block a later key-supersede.
fn backfill_member_identity(
    conn: &mut SqliteConnection,
    collectives_ctx: &AppContext,
    p: &MembershipProjection<'_>,
) {
    let member_agent_key = p.member_cid.strip_prefix("agent:").unwrap_or(p.member_cid);

    // (b): resolve the collective's governance_layer; stamp only households.
    let collective_is_household: bool = collectives::table
        .filter(collectives::h_app_id.eq(&collectives_ctx.h_app_id))
        .filter(collectives::collective_cid.eq(p.collective_cid))
        .select(collectives::governance_layer)
        .first::<String>(conn)
        .ok()
        .map(|layer| layer == crate::db::models::governance_layers::FAMILY)
        .unwrap_or(false);

    let skip_reason = if p.departed_at.is_some() {
        Some("member departed (departed_at set)")
    } else if !collective_is_household {
        Some("collective is not a household (governance_layer != family)")
    } else {
        None
    };

    if let Some(reason) = skip_reason {
        debug!(
            member_cid = %p.member_cid,
            collective_cid = %p.collective_cid,
            reason,
            "project_membership: household_id stamp skipped"
        );
        return;
    }

    // Stamp `humans.agent_pub_key` BEFORE `household_id` — the documented cure
    // for the all-zeros resilience card (NULL `humans.agent_pub_key`; see
    // elohim-storage/CLAUDE.md "Identity & Transport-Identity Coherence").
    //
    // Matched by `humans::id` ONLY (never the `agent_pub_key` OR-arm the
    // household stamp uses — we cannot match on the column we are writing), and
    // only when the value is agent_cid-shaped: a transport id (libp2p
    // `12D3Koo…`, iroh hex) landing in this join key is exactly the
    // cross-namespace join bug the CLAUDE.md doc warns about.
    if crate::identity_namespace::is_agent_cid(member_agent_key) {
        match diesel::update(
            humans::table
                .filter(humans::id.eq(p.member_cid))
                .filter(humans::agent_pub_key.is_null()),
        )
        .set(humans::agent_pub_key.eq(Some(member_agent_key)))
        .execute(conn)
        {
            Ok(n) if n > 0 => {
                debug!(
                    member_cid = %p.member_cid,
                    collective_cid = %p.collective_cid,
                    "project_membership: stamped humans.agent_pub_key"
                );
            }
            // No-op: human row absent, or the key is already set. Honest.
            Ok(_) => {}
            Err(e) => {
                warn!(
                    member_cid = %p.member_cid,
                    error = %e,
                    "project_membership: agent_pub_key stamp failed (non-fatal)"
                );
            }
        }
    } else {
        crate::identity_namespace::observe_agent_cid_write(
            "humans.agent_pub_key",
            Some(member_agent_key),
        );
    }

    match diesel::update(
        humans::table
            .filter(
                humans::id
                    .eq(p.member_cid)
                    .or(humans::agent_pub_key.eq(member_agent_key)),
            )
            .filter(humans::household_id.is_null()),
    )
    .set(humans::household_id.eq(Some(p.collective_cid)))
    .execute(conn)
    {
        Ok(n) if n > 0 => {
            debug!(
                member_cid = %p.member_cid,
                collective_cid = %p.collective_cid,
                "project_membership: stamped humans.household_id"
            );
        }
        // No-op: human row absent or household_id already set. Honest.
        Ok(_) => {}
        Err(e) => {
            warn!(
                member_cid = %p.member_cid,
                error = %e,
                "project_membership: household_id stamp failed (non-fatal)"
            );
        }
    }
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::collectives::{
        create_collective, create_participation, CreateCollectiveInput, CreateParticipationInput,
    };
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    const CID: &str = "collective:uhCkkDowellHouseholdActionHash000000000";
    const ANCHOR: &str = "uhCkkMembershipJessicaActionHash00000000000";
    const JESSICA_KEY: &str = "uhCAkJessicaAgentPubKey00000000000000000000";

    fn test_pool() -> DbPool {
        let url = format!(
            "file:memberships_db_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    /// Seed the household collective the mapping resolves against, anchored to
    /// `CID` and marked `family` (the household marker the backfill gates on).
    fn seed_household(conn: &mut SqliteConnection) {
        let ctx = AppContext::default_lamad();
        create_collective(
            conn,
            &ctx,
            &CreateCollectiveInput {
                id: "household-dowell".to_string(),
                name: "Dowell Household".to_string(),
                description: None,
                governance_layer: "family".to_string(),
                constitutional_parent_id: None,
                reach: "private".to_string(),
                region: None,
                metadata_json: None,
                created_by: None,
            },
        )
        .expect("seed collective");
        diesel::update(
            collectives::table
                .filter(collectives::h_app_id.eq(&ctx.h_app_id))
                .filter(collectives::id.eq("household-dowell")),
        )
        .set(collectives::collective_cid.eq(Some(CID)))
        .execute(conn)
        .expect("stamp cid");
    }

    fn seed_human(conn: &mut SqliteConnection, id: &str, agent_pub_key: Option<&str>) {
        use crate::db::diesel_schema::humans;
        diesel::insert_into(humans::table)
            .values((
                humans::id.eq(id),
                humans::h_app_id.eq(crate::db::HUMANS_HAPP_ID),
                humans::display_name.eq(id),
                humans::agent_pub_key.eq(agent_pub_key),
            ))
            .execute(conn)
            .expect("seed human");
    }

    fn projection<'a>(departed_at: Option<&'a str>) -> MembershipProjection<'a> {
        MembershipProjection {
            action_hash: ANCHOR,
            collective_cid: CID,
            member_cid: "agent:uhCAkJessicaAgentPubKey00000000000000000000",
            member_kind: "person",
            role_context: "contributor",
            departed_at,
        }
    }

    /// The cure the household-formation E2E needs: a Membership healed from the
    /// conductor lands under the CANONICAL human slug, not `agent:uhCAk…`,
    /// because the member resolves through `humans.agent_pub_key`.
    #[test]
    fn a_healed_membership_lands_under_the_canonical_human_slug() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        seed_household(&mut conn);
        seed_human(&mut conn, "human-jessica-spouse", Some(JESSICA_KEY));

        let outcome = project_membership(&mut conn, &projection(None)).expect("project");
        assert_eq!(outcome, MembershipProjectionOutcome::Projected);

        let rows =
            crate::db::collectives::get_participants_of_collective(&mut conn, "household-dowell")
                .expect("participants");
        assert_eq!(rows.len(), 1, "one membership, one participation row");
        assert_eq!(
            rows[0].human_id, "human-jessica-spouse",
            "the member resolved to the canonical humans slug, not the agent cid"
        );
        assert_eq!(rows[0].dht_anchor_hash.as_deref(), Some(ANCHOR));
    }

    /// One person, one participation: a DHT Membership ADOPTS the row an
    /// account-import already created for the same human rather than minting a
    /// second one. This is what keeps the participants list from double-counting
    /// the triad once both surfaces have run.
    #[test]
    fn a_membership_adopts_the_account_import_row_for_the_same_human() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        seed_household(&mut conn);
        seed_human(&mut conn, "human-jessica-spouse", Some(JESSICA_KEY));

        create_participation(
            &mut conn,
            &CreateParticipationInput {
                id: Some("import-row".to_string()),
                collective_id: "household-dowell".to_string(),
                human_id: "human-jessica-spouse".to_string(),
                intimacy_level: "connection".to_string(),
                role_context: None,
                governance_weight: 1.0,
                consent_state: "consented".to_string(),
                metadata_json: None,
            },
        )
        .expect("account-import row");

        project_membership(&mut conn, &projection(None)).expect("project");

        let rows =
            crate::db::collectives::get_participants_of_collective(&mut conn, "household-dowell")
                .expect("participants");
        assert_eq!(rows.len(), 1, "the DHT membership adopted the existing row");
        assert_eq!(rows[0].id, "import-row");
        assert_eq!(
            rows[0].dht_anchor_hash.as_deref(),
            Some(ANCHOR),
            "the adopted row now carries the DHT anchor, so it enters the reconcile inventory"
        );
    }

    /// Re-delivery is idempotent on the anchor: no second row, and the outcome
    /// reports `Refreshed` so a heal cannot count a no-op as a cure.
    #[test]
    fn re_delivery_refreshes_rather_than_duplicating() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        seed_household(&mut conn);
        seed_human(&mut conn, "human-jessica-spouse", Some(JESSICA_KEY));

        assert_eq!(
            project_membership(&mut conn, &projection(None)).expect("first"),
            MembershipProjectionOutcome::Projected
        );
        assert_eq!(
            project_membership(&mut conn, &projection(None)).expect("second"),
            MembershipProjectionOutcome::Refreshed
        );
        let rows =
            crate::db::collectives::get_participants_of_collective(&mut conn, "household-dowell")
                .expect("participants");
        assert_eq!(rows.len(), 1);
    }

    /// The humans backfill rides the SAME mapping, so the reconcile arm heals
    /// identity coherence too: a member row with no `household_id` gets one
    /// from the household Membership it just projected.
    #[test]
    fn the_household_backfill_stamps_the_member_row() {
        use crate::db::diesel_schema::humans;
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        seed_household(&mut conn);
        seed_human(&mut conn, "human-jessica-spouse", Some(JESSICA_KEY));

        project_membership(&mut conn, &projection(None)).expect("project");

        let household: Option<String> = humans::table
            .filter(humans::id.eq("human-jessica-spouse"))
            .select(humans::household_id)
            .first::<Option<String>>(&mut conn)
            .expect("humans row");
        assert_eq!(household.as_deref(), Some(CID));
    }

    /// A departure never stamps a household onto the member (guard (c)) — and
    /// the participation still lands, carrying the departure.
    #[test]
    fn a_departed_membership_projects_but_never_stamps_a_household() {
        use crate::db::diesel_schema::humans;
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        seed_household(&mut conn);
        seed_human(&mut conn, "human-jessica-spouse", Some(JESSICA_KEY));

        project_membership(&mut conn, &projection(Some("2026-08-22T00:00:00Z"))).expect("project");

        let household: Option<String> = humans::table
            .filter(humans::id.eq("human-jessica-spouse"))
            .select(humans::household_id)
            .first::<Option<String>>(&mut conn)
            .expect("humans row");
        assert!(
            household.is_none(),
            "a departure must not stamp a household"
        );

        // The row exists but is departed, so the active-participants read skips it.
        let active =
            crate::db::collectives::get_participants_of_collective(&mut conn, "household-dowell")
                .expect("participants");
        assert!(active.is_empty(), "a departed participation is not active");
    }

    /// Degrade-don't-guess: a member this peer has never met keeps the
    /// `member_cid` as `human_id` (the pre-cure behaviour), so the row is never
    /// lost to an unresolvable identity.
    #[test]
    fn an_unknown_member_keeps_the_member_cid_as_human_id() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        seed_household(&mut conn);

        project_membership(&mut conn, &projection(None)).expect("project");

        let rows =
            crate::db::collectives::get_participants_of_collective(&mut conn, "household-dowell")
                .expect("participants");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].human_id,
            "agent:uhCAkJessicaAgentPubKey00000000000000000000"
        );
    }

    /// The projected row is readable through the canonical participations
    /// scope — the (b) half of this cure. A row written under one partition and
    /// read under another is the silent-empty class this constant exists to
    /// close.
    #[test]
    fn the_projected_row_lands_in_the_canonical_participations_scope() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        seed_household(&mut conn);

        project_membership(&mut conn, &projection(None)).expect("project");

        let scope: String = collective_participations::table
            .filter(collective_participations::dht_anchor_hash.eq(Some(ANCHOR)))
            .select(collective_participations::h_app_id)
            .first(&mut conn)
            .expect("row");
        assert_eq!(scope, crate::db::PARTICIPATIONS_HAPP_ID);
    }

    /// The reconcile inventory advertises the anchor (and nothing un-anchored),
    /// and the presence probe answers on it — the two queries the participations
    /// arm rides.
    #[test]
    fn the_anchor_inventory_serves_only_dht_anchored_rows() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        seed_household(&mut conn);

        // A purely local participation (no DHT identity) must NOT be advertised.
        create_participation(
            &mut conn,
            &CreateParticipationInput {
                id: Some("local-only".to_string()),
                collective_id: "household-dowell".to_string(),
                human_id: "human-local".to_string(),
                intimacy_level: "connection".to_string(),
                role_context: None,
                governance_weight: 1.0,
                consent_state: "consented".to_string(),
                metadata_json: None,
            },
        )
        .expect("local row");

        project_membership(&mut conn, &projection(None)).expect("project");

        let (anchors, total) =
            crate::db::collectives::list_participation_anchor_inventory(&mut conn, 0, 100)
                .expect("inventory");
        assert_eq!(anchors, vec![ANCHOR.to_string()]);
        assert_eq!(total, 1, "the un-anchored local row is not advertised");

        let present = crate::db::collectives::participation_anchors_present(
            &mut conn,
            &[ANCHOR.to_string(), "uhCkkNeverSeen".to_string()],
        )
        .expect("presence");
        assert!(present.contains(ANCHOR));
        assert!(!present.contains("uhCkkNeverSeen"));
    }
}
