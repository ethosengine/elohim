//! Mishpat::Commitment projection CRUD (Category A DHT projection).
//!
//! Read-optimised cache of the compute-bounds commitment (policy envelope) that
//! the bounds_validator checks. Source of truth is the Holochain DHT (mishpat
//! DNA Commitment entry); these rows are the P1 reconciliation projection,
//! populated from the create_commitment post-commit signal.
//!
//! A NULL `dht_anchor_hash` means un-notarized / storage-only.  The
//! `ProjectionCommitmentFetcher` (Slice 2a T6) refuses a bounds-pass on such
//! rows — notarised provenance is required before the validator accepts a
//! commitment as authoritative.
//!
//! Spec: 2026-06-07-epr-acquisition-pull-queue-design.md §6.5.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::mishpat_commitments::dsl as mc;
use super::models::{current_timestamp, MishpatCommitment, NewMishpatCommitment};

/// Idempotent upsert on `cid` (content-derived identifier).
///
/// On conflict the row is refreshed — all projected fields are updated and
/// `updated_at` is bumped via `current_timestamp()`.
///
/// **Anchor-preservation rule:** `dht_anchor_hash` is only overwritten when
/// the incoming `new.dht_anchor_hash` is `Some(_)`.  A later un-anchored
/// re-projection (signal replay with `None`) must never clobber a previously
/// written anchor — doing so would silently strip the notarised provenance that
/// the `ProjectionCommitmentFetcher` requires before granting a bounds-pass.
///
/// Timestamps are always computed in Rust via `current_timestamp()` so every
/// row uses ISO 8601 `YYYY-MM-DDTHH:MM:SSZ` format instead of the SQL
/// `datetime('now')` default (`YYYY-MM-DD HH:MM:SS`).
pub fn upsert_with_anchor(
    conn: &mut SqliteConnection,
    new: NewMishpatCommitment,
) -> QueryResult<MishpatCommitment> {
    let now = current_timestamp();

    // Base insert — sets created_at and updated_at explicitly.
    diesel::insert_into(mc::mishpat_commitments)
        .values((
            mc::cid.eq(&new.cid),
            mc::action.eq(&new.action),
            mc::scope.eq(&new.scope),
            mc::provider.eq(&new.provider),
            mc::recipient.eq(&new.recipient),
            mc::bounds_json.eq(&new.bounds_json),
            mc::valid_from.eq(&new.valid_from),
            mc::valid_until.eq(&new.valid_until),
            mc::revoked_at.eq(&new.revoked_at),
            mc::state.eq(&new.state),
            mc::dht_anchor_hash.eq(&new.dht_anchor_hash),
            mc::created_at.eq(&now),
            mc::updated_at.eq(&now),
        ))
        .on_conflict(mc::cid)
        .do_update()
        .set((
            mc::action.eq(new.action.clone()),
            mc::scope.eq(new.scope.clone()),
            mc::provider.eq(new.provider.clone()),
            mc::recipient.eq(new.recipient.clone()),
            mc::bounds_json.eq(new.bounds_json.clone()),
            mc::valid_from.eq(new.valid_from.clone()),
            mc::valid_until.eq(new.valid_until.clone()),
            mc::revoked_at.eq(new.revoked_at.clone()),
            mc::state.eq(new.state.clone()),
            // Anchor-preservation: only overwrite when incoming is Some(_).
            // When None, the existing column value is kept via the EXCLUDED
            // trick: we update only the non-null path by branching in Rust
            // before issuing the statement.  This is the idiomatic Diesel
            // approach for conditional-on-new-value updates — a single
            // do_update() set() cannot reference the existing column value
            // without raw SQL, so we branch here.
            mc::updated_at.eq(&now),
        ))
        .execute(conn)?;

    // If the incoming anchor is Some, overwrite whatever is in the row now
    // (including a previously-NULL anchor getting its first value).
    if let Some(ref anchor) = new.dht_anchor_hash {
        diesel::update(mc::mishpat_commitments.filter(mc::cid.eq(&new.cid)))
            .set(mc::dht_anchor_hash.eq(anchor))
            .execute(conn)?;
    }
    // If incoming anchor is None, leave dht_anchor_hash untouched — a non-null
    // anchor written by an earlier upsert is preserved.

    mc::mishpat_commitments
        .filter(mc::cid.eq(&new.cid))
        .first(conn)
}

/// Fetch a single commitment by its CID.
///
/// Returns `Ok(None)` when no row exists for the given CID.
pub fn get_by_cid(
    conn: &mut SqliteConnection,
    cid: &str,
) -> QueryResult<Option<MishpatCommitment>> {
    mc::mishpat_commitments
        .filter(mc::cid.eq(cid))
        .first(conn)
        .optional()
}

/// List ALL mishpat commitment rows, newest-first (`created_at` descending).
///
/// The read-side projection the REA economic facing surfaces over
/// (`GET /api/v1/commitments/facing/rea`). Node-scoped and viewer-less, mirroring
/// the operational-weave loaders (`load_all_placement_gaps`): there is no single
/// viewer or app for the compute-commitment ledger, so no scope filter is applied.
///
/// Reads EMPTY until the `CommitmentCommitted` post-commit signal projects rows
/// (the table is write-only/unread-surfaced today — the honest correct-but-dormant
/// projection; it lights up the moment a notarized commitment lands).
pub fn list_all(conn: &mut SqliteConnection) -> QueryResult<Vec<MishpatCommitment>> {
    mc::mishpat_commitments
        .order(mc::created_at.desc())
        .load(conn)
}

/// Update the lifecycle state of a commitment (e.g. `'proposed'` → `'active'`).
///
/// Returns the number of rows affected (0 if the CID does not exist).
pub fn set_state(conn: &mut SqliteConnection, cid: &str, state: &str) -> QueryResult<usize> {
    let now = current_timestamp();
    diesel::update(mc::mishpat_commitments.filter(mc::cid.eq(cid)))
        .set((mc::state.eq(state), mc::updated_at.eq(&now)))
        .execute(conn)
}

/// Record a revocation timestamp on a commitment.
///
/// Callers pass an ISO-8601 string; the bounds_validator will reject any
/// event whose `bounded_by` points to a commitment with a non-null `revoked_at`.
///
/// Returns the number of rows affected (0 if the CID does not exist).
pub fn set_revoked_at(conn: &mut SqliteConnection, cid: &str, ts: &str) -> QueryResult<usize> {
    let now = current_timestamp();
    diesel::update(mc::mishpat_commitments.filter(mc::cid.eq(cid)))
        .set((mc::revoked_at.eq(ts), mc::updated_at.eq(&now)))
        .execute(conn)
}

/// Graduate a Mishpat commitment proposed → active (the projection reflecting
/// "this commitment now has a notarized provide event" — the event is truth,
/// this column is projection; spec §6.5). Idempotent + guarded: only flips a
/// row currently in 'proposed' (never downgrades an active/cancelled/breached
/// commitment). Returns rows affected (0 if not proposed or cid absent).
pub fn graduate_to_active(conn: &mut SqliteConnection, cid: &str) -> QueryResult<usize> {
    let now = current_timestamp();
    diesel::update(
        mc::mishpat_commitments
            .filter(mc::cid.eq(cid))
            .filter(mc::state.eq("proposed")),
    )
    .set((mc::state.eq("active"), mc::updated_at.eq(&now)))
    .execute(conn)
}

/// Load this provider's live (non-revoked) `replicates-commons` commitments.
///
/// "Live" = `action == "replicates-commons"` AND `provider == self` AND
/// `revoked_at IS NULL`. State is intentionally NOT filtered — both 'proposed'
/// (authored, not yet graduated) and 'active' (graduated by a provide event)
/// count as a live provide for logical-key dedup; only revocation removes a row
/// from the actual set (spec §provide-loop). The provide reconciler diffs the
/// `(provider, recipient)` logical keys of these rows against its desired set.
pub fn live_commons_provides_for_provider(
    conn: &mut SqliteConnection,
    provider: &str,
) -> QueryResult<Vec<MishpatCommitment>> {
    mc::mishpat_commitments
        .filter(mc::action.eq("replicates-commons"))
        .filter(mc::provider.eq(provider))
        .filter(mc::revoked_at.is_null())
        .load(conn)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn test_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn sample_commitment(cid: &str, anchor: Option<&str>) -> NewMishpatCommitment {
        NewMishpatCommitment {
            cid: cid.to_string(),
            action: "delegates-compute".to_string(),
            scope: "republish-epr".to_string(),
            provider: "agent:matthew-steward".to_string(),
            recipient: "agent:deploy-svc".to_string(),
            bounds_json: r#"{"rate_per_hour":30,"reach_ceiling":"commons"}"#.to_string(),
            valid_from: "2026-06-01T00:00:00Z".to_string(),
            valid_until: "2026-09-01T00:00:00Z".to_string(),
            revoked_at: None,
            state: "proposed".to_string(),
            dht_anchor_hash: anchor.map(str::to_string),
        }
    }

    #[test]
    fn upsert_idempotent_on_cid() {
        let mut conn = test_conn();
        let cid = "commitment:abc123";

        // First upsert.
        let row1 = upsert_with_anchor(&mut conn, sample_commitment(cid, Some("h1")))
            .expect("first upsert");
        assert_eq!(row1.cid, cid);
        assert_eq!(row1.action, "delegates-compute");
        assert_eq!(row1.dht_anchor_hash.as_deref(), Some("h1"));

        // Second upsert — same cid, different scope to verify fields update.
        let mut updated = sample_commitment(cid, Some("h2"));
        updated.scope = "content-delivery".to_string();
        let row2 = upsert_with_anchor(&mut conn, updated).expect("second upsert");

        // Fields updated.
        assert_eq!(row2.scope, "content-delivery");
        // Anchor updated to h2.
        assert_eq!(row2.dht_anchor_hash.as_deref(), Some("h2"));

        // Exactly one row in the table.
        let all: Vec<MishpatCommitment> =
            mc::mishpat_commitments.load(&mut conn).expect("load all");
        assert_eq!(
            all.len(),
            1,
            "exactly one row after two upserts on same cid"
        );
    }

    #[test]
    fn upsert_preserves_anchor_when_new_is_null() {
        let mut conn = test_conn();
        let cid = "commitment:anchor-preserve";

        // First upsert — establishes anchor "h1".
        upsert_with_anchor(&mut conn, sample_commitment(cid, Some("h1")))
            .expect("first upsert with anchor");

        // Second upsert — incoming anchor is None (un-anchored re-projection).
        upsert_with_anchor(&mut conn, sample_commitment(cid, None))
            .expect("second upsert without anchor");

        // Row must still carry "h1" — notarised provenance must not be lost.
        let row = get_by_cid(&mut conn, cid)
            .expect("get_by_cid")
            .expect("row must exist");
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("h1"),
            "dht_anchor_hash must be preserved when the incoming upsert carries None"
        );
    }

    #[test]
    fn graduate_only_flips_proposed() {
        let mut conn = test_conn();

        // ── Case 1: 'proposed' row → flips to 'active' ──────────────────────
        let cid_proposed = "commitment:graduate-proposed";
        upsert_with_anchor(&mut conn, sample_commitment(cid_proposed, Some("h-p1")))
            .expect("upsert proposed");
        // Precondition: state is 'proposed'.
        let pre = get_by_cid(&mut conn, cid_proposed)
            .expect("get pre")
            .expect("row must exist");
        assert_eq!(pre.state, "proposed");

        let affected = graduate_to_active(&mut conn, cid_proposed).expect("graduate proposed");
        assert_eq!(affected, 1, "graduate_to_active must flip exactly one row");

        let row = get_by_cid(&mut conn, cid_proposed)
            .expect("get post")
            .expect("row must exist after graduation");
        assert_eq!(
            row.state, "active",
            "state must be 'active' after graduation"
        );

        // ── Case 2: idempotent — graduate again returns 0 rows ───────────────
        let second = graduate_to_active(&mut conn, cid_proposed).expect("second graduate");
        assert_eq!(
            second, 0,
            "second graduate_to_active must be a no-op (0 rows)"
        );

        let row2 = get_by_cid(&mut conn, cid_proposed)
            .expect("get after idempotent")
            .expect("row must still exist");
        assert_eq!(
            row2.state, "active",
            "state must remain 'active' after idempotent call"
        );

        // ── Case 3: 'active' row from birth → not downgraded ────────────────
        let cid_active = "commitment:already-active";
        let mut already_active = sample_commitment(cid_active, Some("h-a1"));
        already_active.state = "active".to_string();
        upsert_with_anchor(&mut conn, already_active).expect("upsert active");

        let affected_active =
            graduate_to_active(&mut conn, cid_active).expect("graduate already-active");
        assert_eq!(
            affected_active, 0,
            "graduate_to_active must not touch an already-active row"
        );

        let row_active = get_by_cid(&mut conn, cid_active)
            .expect("get active after graduate")
            .expect("row must exist");
        assert_eq!(row_active.state, "active", "state must remain 'active'");

        // ── Case 4: 'cancelled' row → unchanged ─────────────────────────────
        let cid_cancelled = "commitment:cancelled";
        let mut cancelled = sample_commitment(cid_cancelled, Some("h-c1"));
        cancelled.state = "cancelled".to_string();
        upsert_with_anchor(&mut conn, cancelled).expect("upsert cancelled");

        let affected_cancelled =
            graduate_to_active(&mut conn, cid_cancelled).expect("graduate cancelled");
        assert_eq!(
            affected_cancelled, 0,
            "graduate_to_active must not touch a cancelled row"
        );

        let row_cancelled = get_by_cid(&mut conn, cid_cancelled)
            .expect("get cancelled after graduate")
            .expect("row must exist");
        assert_eq!(
            row_cancelled.state, "cancelled",
            "state must remain 'cancelled'"
        );
    }

    #[test]
    fn set_state_and_revoked() {
        let mut conn = test_conn();
        let cid = "commitment:lifecycle";

        upsert_with_anchor(&mut conn, sample_commitment(cid, Some("h-anchor"))).expect("upsert");

        // Graduate to active.
        let affected = set_state(&mut conn, cid, "active").expect("set_state");
        assert_eq!(affected, 1, "set_state must affect exactly one row");

        let row = get_by_cid(&mut conn, cid)
            .expect("get_by_cid")
            .expect("row must exist after set_state");
        assert_eq!(row.state, "active");
        assert!(row.revoked_at.is_none(), "revoked_at must still be None");

        // Revoke.
        let ts = "2026-07-01T12:00:00Z";
        let affected = set_revoked_at(&mut conn, cid, ts).expect("set_revoked_at");
        assert_eq!(affected, 1, "set_revoked_at must affect exactly one row");

        let row = get_by_cid(&mut conn, cid)
            .expect("get_by_cid after revocation")
            .expect("row must exist after revocation");
        assert_eq!(
            row.state, "active",
            "state must be unchanged after revocation"
        );
        assert_eq!(
            row.revoked_at.as_deref(),
            Some(ts),
            "revoked_at must reflect the supplied timestamp"
        );
    }

    #[test]
    fn live_commons_provides_lists_non_revoked_by_provider_recipient() {
        let mut conn = test_conn();

        let mut active = sample_commitment("cid:commons-live", Some("anchor-live"));
        active.action = "replicates-commons".to_string();
        active.scope = "replicates-commons".to_string();
        active.provider = "agent:self".to_string();
        active.recipient = "epr:album-1".to_string();
        active.state = "proposed".to_string();
        upsert_with_anchor(&mut conn, active).expect("upsert live commons");

        // A revoked row for the SAME logical key must NOT count as live.
        let mut revoked = sample_commitment("cid:commons-revoked", Some("anchor-rev"));
        revoked.action = "replicates-commons".to_string();
        revoked.provider = "agent:self".to_string();
        revoked.recipient = "epr:album-1".to_string();
        revoked.revoked_at = Some("2026-06-10T00:00:00Z".to_string());
        upsert_with_anchor(&mut conn, revoked).expect("upsert revoked commons");

        // A dwelling commitment must NOT appear (action filter).
        let mut dwelling = sample_commitment("cid:dwelling", Some("anchor-dw"));
        dwelling.provider = "agent:self".to_string();
        dwelling.recipient = "epr:album-1".to_string();
        upsert_with_anchor(&mut conn, dwelling).expect("upsert dwelling");

        let live = live_commons_provides_for_provider(&mut conn, "agent:self").expect("query");
        assert_eq!(
            live.len(),
            1,
            "exactly one non-revoked replicates-commons row"
        );
        assert_eq!(live[0].recipient, "epr:album-1");
        assert_eq!(live[0].cid, "cid:commons-live");

        // A different provider sees nothing.
        let other = live_commons_provides_for_provider(&mut conn, "agent:other").expect("query");
        assert!(
            other.is_empty(),
            "provider filter must exclude other agents"
        );
    }
}
