//! Lens projection CRUD (Category A DHT projection) — lens-market S2.
//!
//! Read-optimised cache of an `author-lens` Mishpat::Commitment (the plural-Mishpat
//! governance primitive). Source of truth is the Holochain DHT (mishpat DNA
//! Commitment entry, action='author-lens'); these rows are the P1 reconciliation
//! projection, populated from the create_commitment post-commit signal (plan S3).
//!
//! A NULL `dht_anchor_hash` means un-notarized / storage-only. The forward index
//! (`find_lenses_governing_epr`, plan S4) fail-closes on such rows. `cid` is the
//! Commitment `entry_hash` (NEVER `action_hash`); `governs_epr` is the EPR slug-id
//! scope key (plan A3).
//!
//! Spec: 2026-06-27-plural-mishpat-lenses-over-epr-design.md §8.
//! Plan: 2026-06-27-plural-mishpat-lenses-service-layer-plan.md (S2, I5).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::lenses::dsl as ln;
use super::models::{current_timestamp, Lens, NewLens};

/// Idempotent upsert on `cid` (= Commitment entry_hash).
///
/// On conflict the row is refreshed — all projected fields are updated and
/// `updated_at` is bumped via `current_timestamp()`.
///
/// **Anchor-preservation rule** (mirrors `mishpat_commitments::upsert_with_anchor`):
/// `dht_anchor_hash` is only overwritten when the incoming `new.dht_anchor_hash` is
/// `Some(_)`. A later un-anchored re-projection (signal replay with `None`) must
/// never clobber a previously written anchor — doing so would silently strip the
/// notarised provenance the forward index requires before surfacing a lens.
pub fn upsert_with_anchor(conn: &mut SqliteConnection, new: NewLens) -> QueryResult<Lens> {
    let now = current_timestamp();

    // Base insert — sets created_at and updated_at explicitly (ISO-8601 via
    // current_timestamp(), not the SQL datetime('now') default).
    diesel::insert_into(ln::lenses)
        .values((
            ln::cid.eq(&new.cid),
            ln::governs_epr.eq(&new.governs_epr),
            ln::school.eq(&new.school),
            ln::role.eq(&new.role),
            ln::rule_json.eq(&new.rule_json),
            ln::telos_json.eq(&new.telos_json),
            ln::version_parent.eq(&new.version_parent),
            ln::revoked_at.eq(&new.revoked_at),
            ln::dht_anchor_hash.eq(&new.dht_anchor_hash),
            ln::created_at.eq(&now),
            ln::updated_at.eq(&now),
        ))
        .on_conflict(ln::cid)
        .do_update()
        .set((
            ln::governs_epr.eq(new.governs_epr.clone()),
            ln::school.eq(new.school.clone()),
            ln::role.eq(new.role.clone()),
            ln::rule_json.eq(new.rule_json.clone()),
            ln::telos_json.eq(new.telos_json.clone()),
            ln::version_parent.eq(new.version_parent.clone()),
            ln::revoked_at.eq(new.revoked_at.clone()),
            // Anchor-preservation: dht_anchor_hash is updated conditionally below,
            // never in this set() — so an incoming None cannot clobber an existing
            // anchor (mirror mishpat_commitments::upsert_with_anchor).
            ln::updated_at.eq(&now),
        ))
        .execute(conn)?;

    // If the incoming anchor is Some, overwrite whatever is in the row now
    // (including a previously-NULL anchor getting its first value). If None,
    // leave dht_anchor_hash untouched — notarised provenance is preserved.
    if let Some(ref anchor) = new.dht_anchor_hash {
        diesel::update(ln::lenses.filter(ln::cid.eq(&new.cid)))
            .set(ln::dht_anchor_hash.eq(anchor))
            .execute(conn)?;
    }

    ln::lenses.filter(ln::cid.eq(&new.cid)).first(conn)
}

/// Fetch a single lens by its CID. Returns `Ok(None)` when no row exists.
pub fn get_by_cid(conn: &mut SqliteConnection, cid: &str) -> QueryResult<Option<Lens>> {
    ln::lenses.filter(ln::cid.eq(cid)).first(conn).optional()
}

/// The forward index: all live lenses governing an EPR (keyed by slug-id, plan A3).
///
/// Mirrors `mishpat_commitments::find_active_delegates_compute` — a pure SQL scope
/// projection (zero new DHT entry/link type). **Fail-closed**: only surfaces lenses
/// that are non-revoked AND notarized (`dht_anchor_hash IS NOT NULL`) — an
/// un-notarized or revoked lens must never enter the market. Newest-first so the
/// selector sees the current version chain head first.
pub fn find_lenses_governing_epr(
    conn: &mut SqliteConnection,
    epr_slug_id: &str,
) -> QueryResult<Vec<Lens>> {
    ln::lenses
        .filter(ln::governs_epr.eq(epr_slug_id))
        .filter(ln::revoked_at.is_null())
        .filter(ln::dht_anchor_hash.is_not_null())
        .order(ln::created_at.desc())
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

    fn sample_lens(cid: &str, anchor: Option<&str>) -> NewLens {
        NewLens {
            cid: cid.to_string(),
            governs_epr: "epr:lamad-spa".to_string(),
            school: "georgist".to_string(),
            role: "lens".to_string(),
            rule_json: r#"{"predicate":"land_value_uplift > rent_capture"}"#.to_string(),
            telos_json: r#"{"summary":"tax unimproved land value, not labor"}"#.to_string(),
            version_parent: None,
            revoked_at: None,
            dht_anchor_hash: anchor.map(str::to_string),
        }
    }

    #[test]
    fn upsert_inserts_and_round_trips() {
        let mut conn = test_conn();
        let cid = "lens:georgist-1";

        let row = upsert_with_anchor(&mut conn, sample_lens(cid, Some("action-hash-1")))
            .expect("first upsert");
        assert_eq!(row.cid, cid, "cid is the entry_hash read key");
        assert_eq!(row.governs_epr, "epr:lamad-spa");
        assert_eq!(row.school, "georgist");
        assert_eq!(row.role, "lens");
        assert_eq!(
            row.dht_anchor_hash.as_deref(),
            Some("action-hash-1"),
            "A-class: dht_anchor_hash carries the notarized provenance"
        );

        // Round-trips via get_by_cid.
        let fetched = get_by_cid(&mut conn, cid)
            .expect("get_by_cid")
            .expect("row must exist");
        assert_eq!(fetched.cid, cid);
        assert_eq!(fetched.telos_json, row.telos_json);
    }

    #[test]
    fn upsert_idempotent_on_cid() {
        let mut conn = test_conn();
        let cid = "lens:idem";

        upsert_with_anchor(&mut conn, sample_lens(cid, Some("h1"))).expect("first upsert");

        // Second upsert — same cid, different school + later anchor.
        let mut updated = sample_lens(cid, Some("h2"));
        updated.school = "beerian".to_string();
        let row2 = upsert_with_anchor(&mut conn, updated).expect("second upsert");
        assert_eq!(row2.school, "beerian", "fields refresh on conflict");
        assert_eq!(
            row2.dht_anchor_hash.as_deref(),
            Some("h2"),
            "anchor updates"
        );

        // Exactly one row.
        let all: Vec<Lens> = ln::lenses.load(&mut conn).expect("load all");
        assert_eq!(
            all.len(),
            1,
            "exactly one row after two upserts on same cid"
        );
    }

    #[test]
    fn upsert_preserves_anchor_when_new_is_null() {
        let mut conn = test_conn();
        let cid = "lens:anchor-preserve";

        // First upsert establishes anchor "h1".
        upsert_with_anchor(&mut conn, sample_lens(cid, Some("h1"))).expect("first upsert");

        // Second upsert — incoming anchor is None (un-anchored re-projection).
        upsert_with_anchor(&mut conn, sample_lens(cid, None)).expect("second upsert");

        // Notarised provenance must survive.
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
    fn find_lenses_governing_epr_is_scoped_and_fail_closed() {
        let mut conn = test_conn();

        // Two valid lenses governing epr:lamad-spa.
        let mut g1 = sample_lens("lens:g1", Some("a1"));
        g1.school = "georgist".to_string();
        upsert_with_anchor(&mut conn, g1).expect("g1");
        let mut g2 = sample_lens("lens:g2", Some("a2"));
        g2.school = "beerian".to_string();
        upsert_with_anchor(&mut conn, g2).expect("g2");

        // A lens governing a DIFFERENT scope — must be excluded.
        let mut other = sample_lens("lens:other", Some("a3"));
        other.governs_epr = "epr:somewhere-else".to_string();
        upsert_with_anchor(&mut conn, other).expect("other");

        // A REVOKED lens in scope — fail-closed exclusion.
        let mut revoked = sample_lens("lens:revoked", Some("a4"));
        revoked.revoked_at = Some("2026-06-27T01:00:00Z".to_string());
        upsert_with_anchor(&mut conn, revoked).expect("revoked");

        // An UN-NOTARIZED lens in scope (anchor NULL) — fail-closed exclusion.
        upsert_with_anchor(&mut conn, sample_lens("lens:unanchored", None)).expect("unanchored");

        let found = find_lenses_governing_epr(&mut conn, "epr:lamad-spa").expect("query");
        let cids: Vec<&str> = found.iter().map(|l| l.cid.as_str()).collect();
        assert_eq!(
            found.len(),
            2,
            "only the 2 valid governing lenses: {cids:?}"
        );
        assert!(cids.contains(&"lens:g1"));
        assert!(cids.contains(&"lens:g2"));
        assert!(!cids.contains(&"lens:other"), "different scope excluded");
        assert!(
            !cids.contains(&"lens:revoked"),
            "revoked excluded (fail-closed)"
        );
        assert!(
            !cids.contains(&"lens:unanchored"),
            "un-notarized excluded (fail-closed)"
        );

        // Unknown scope → empty (not an error).
        assert!(find_lenses_governing_epr(&mut conn, "epr:nope")
            .expect("query")
            .is_empty());
    }
}
