//! CRUD for the `custodian_shares` table.
//!
//! ## Classification
//!
//! Category C operational — source of truth is the stewardship-rotation
//! ceremony (T22 extern `create_shamir_custody_setup`). This table is NOT a
//! DHT projection; it is the write target of the T22 coordinator function.
//! Rows survive node restarts and are re-used across recovery events for the
//! same governance action.
//!
//! ## Primary read path (responder)
//!
//! The swarm responder arm calls
//! `get_share_for_governance_action(conn, local_agent_cid, recovery_governance_action_cid)`
//! after the authorization gate passes. If `Some(share)` is returned, the
//! responder builds a signed `ShamirShareResponse`; if `None`, it returns an
//! error envelope with reason "custodian has no share for this governance action".
//!
//! ## Integrity check
//!
//! Every read helper verifies `sha256(share_data) == share_blob_hash`.
//! A mismatch indicates SQLite-level corruption and is surfaced as
//! `StorageError::Internal`.
//!
//! ## At-rest encryption
//!
//! Share bytes are stored unencrypted in this release.  See the migration
//! `2026-05-15-T21-000001_custodian_shares/up.sql` comment for the deferred
//! post-M4 threat model and the remediation path.

use diesel::prelude::*;
use sha2::{Digest, Sha256};

use crate::db::diesel_schema::custodian_shares;
use crate::db::models::{CustodianShare, NewCustodianShare};
use crate::error::StorageError;

// ─────────────────────────────────────────────────────────────────────────────
// Hash helper
// ─────────────────────────────────────────────────────────────────────────────

/// Compute the hex-encoded SHA-256 of `bytes`.
///
/// Called both when inserting (`upsert_share` computes + stores the hash) and
/// when reading (`verify_share_integrity` recomputes + compares).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// Verify that `share.share_data` matches `share.share_blob_hash`.
///
/// Returns `Ok(())` if the integrity check passes; `Err(StorageError::Internal)`
/// if the hash does not match (indicates corruption).
fn verify_share_integrity(share: &CustodianShare) -> Result<(), StorageError> {
    let computed = sha256_hex(&share.share_data);
    if computed != share.share_blob_hash {
        return Err(StorageError::Internal(format!(
            "custodian_shares integrity failure: row id={} governance_action_cid={} \
             expected hash {} got {}",
            share.id, share.governance_action_cid, share.share_blob_hash, computed
        )));
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Write path
// ─────────────────────────────────────────────────────────────────────────────

/// Install (or replace) a custodian share for a governance action.
///
/// ## Supersession semantics
///
/// If an active share already exists for `(custodian_cid, governance_action_cid)`,
/// it is marked superseded (`superseded_at = now_iso`) before the new row is
/// inserted.  This preserves the audit trail for stewardship-rotation events
/// while ensuring `get_share_for_governance_action` always returns the latest
/// active share.
///
/// ## Called by
///
/// T22 extern `create_shamir_custody_setup` — not by the responder or assembler.
///
/// ## Parameters
///
/// - `conn` — mutable Diesel connection
/// - `custodian_cid` — CID of this node's custodian identity
/// - `governance_action_cid` — governance action this share authorises
/// - `share_index` — 1-based index in the (m, n) scheme
/// - `share_data` — raw Shamir share bytes
/// - `now_iso` — RFC-3339 / ISO-8601 timestamp for `created_at` and supersession
///
/// Returns the AUTOINCREMENT `id` of the newly inserted row.
pub fn upsert_share(
    conn: &mut SqliteConnection,
    custodian_cid: &str,
    governance_action_cid: &str,
    share_index: i32,
    share_data: Vec<u8>,
    now_iso: &str,
) -> Result<i32, StorageError> {
    // Mark any existing active share as superseded.
    diesel::update(
        custodian_shares::table
            .filter(custodian_shares::custodian_cid.eq(custodian_cid))
            .filter(custodian_shares::governance_action_cid.eq(governance_action_cid))
            .filter(custodian_shares::superseded_at.is_null()),
    )
    .set(custodian_shares::superseded_at.eq(Some(now_iso.to_string())))
    .execute(conn)
    .map_err(|e| {
        StorageError::Internal(format!(
            "custodian_shares: failed to supersede prior share: {e}"
        ))
    })?;

    let blob_hash = sha256_hex(&share_data);

    let row = NewCustodianShare {
        governance_action_cid: governance_action_cid.to_string(),
        custodian_cid: custodian_cid.to_string(),
        share_index,
        share_data,
        share_blob_hash: blob_hash,
        created_at: now_iso.to_string(),
        superseded_at: None,
    };

    diesel::insert_into(custodian_shares::table)
        .values(&row)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("custodian_shares insert: {e}")))?;

    // Retrieve the AUTOINCREMENT id of the row we just inserted.
    custodian_shares::table
        .filter(custodian_shares::custodian_cid.eq(&row.custodian_cid))
        .filter(custodian_shares::governance_action_cid.eq(&row.governance_action_cid))
        .filter(custodian_shares::superseded_at.is_null())
        .select(custodian_shares::id)
        .first::<i32>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "custodian_shares: failed to retrieve inserted row id: {e}"
            ))
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// Read path (responder)
// ─────────────────────────────────────────────────────────────────────────────

/// Look up the active share this custodian holds for a governance action.
///
/// "Active" means `superseded_at IS NULL`.  Returns `None` if no share has
/// been installed for this `(custodian_cid, governance_action_cid)` pair.
///
/// The integrity check (`sha256(share_data) == share_blob_hash`) is
/// performed on every successful fetch.  A mismatch returns
/// `Err(StorageError::Internal)` rather than silently returning corrupted bytes.
///
/// ## Called by
///
/// The swarm responder arm in `p2p/mod.rs` after the authorization gate passes.
pub fn get_share_for_governance_action(
    conn: &mut SqliteConnection,
    custodian_cid: &str,
    governance_action_cid: &str,
) -> Result<Option<CustodianShare>, StorageError> {
    let maybe = custodian_shares::table
        .filter(custodian_shares::custodian_cid.eq(custodian_cid))
        .filter(custodian_shares::governance_action_cid.eq(governance_action_cid))
        .filter(custodian_shares::superseded_at.is_null())
        .order(custodian_shares::id.desc())
        .first::<CustodianShare>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Internal(format!(
                "custodian_shares get_share_for_governance_action: {e}"
            ))
        })?;

    if let Some(ref share) = maybe {
        verify_share_integrity(share)?;
    }
    Ok(maybe)
}

/// List all active shares held by `custodian_cid` (most recently created first).
///
/// Used by the stewardship-rotation cleanup sweep (T22+) and diagnostic queries.
/// Integrity check is performed on each returned row; the first failing row
/// short-circuits with `Err`.
pub fn list_shares_for_custodian(
    conn: &mut SqliteConnection,
    custodian_cid: &str,
) -> Result<Vec<CustodianShare>, StorageError> {
    let rows = custodian_shares::table
        .filter(custodian_shares::custodian_cid.eq(custodian_cid))
        .filter(custodian_shares::superseded_at.is_null())
        .order(custodian_shares::created_at.desc())
        .load::<CustodianShare>(conn)
        .map_err(|e| {
            StorageError::Internal(format!("custodian_shares list_shares_for_custodian: {e}"))
        })?;

    for share in &rows {
        verify_share_integrity(share)?;
    }
    Ok(rows)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    // ── CRUD: upsert + get ────────────────────────────────────────────────────

    #[test]
    fn upsert_and_get_roundtrip() {
        let mut conn = setup();
        let share_bytes = b"test-share-bytes-001".to_vec();

        let row_id = upsert_share(
            &mut conn,
            "custodian-cid-A",
            "governance-action-001",
            1,
            share_bytes.clone(),
            "2026-05-15T00:00:00Z",
        )
        .expect("upsert");

        assert!(row_id > 0, "AUTOINCREMENT id should be positive");

        let share =
            get_share_for_governance_action(&mut conn, "custodian-cid-A", "governance-action-001")
                .expect("get")
                .expect("should be Some");

        assert_eq!(share.share_data, share_bytes);
        assert_eq!(share.share_index, 1);
        assert_eq!(share.custodian_cid, "custodian-cid-A");
        assert_eq!(share.governance_action_cid, "governance-action-001");
        assert!(share.superseded_at.is_none());
    }

    // ── CRUD: upsert is supersede-then-insert ─────────────────────────────────

    #[test]
    fn second_upsert_supersedes_first() {
        let mut conn = setup();

        upsert_share(
            &mut conn,
            "custodian-cid-B",
            "governance-action-002",
            1,
            b"share-v1".to_vec(),
            "2026-05-15T00:00:00Z",
        )
        .expect("first upsert");

        upsert_share(
            &mut conn,
            "custodian-cid-B",
            "governance-action-002",
            1,
            b"share-v2".to_vec(),
            "2026-05-15T01:00:00Z",
        )
        .expect("second upsert");

        // Only the most recent active share is returned.
        let active =
            get_share_for_governance_action(&mut conn, "custodian-cid-B", "governance-action-002")
                .expect("get")
                .expect("should be Some");

        assert_eq!(active.share_data, b"share-v2");
        assert!(
            active.superseded_at.is_none(),
            "active share must not be superseded"
        );

        // The superseded share still exists in the table (audit trail).
        let all: Vec<CustodianShare> = custodian_shares::table
            .filter(custodian_shares::custodian_cid.eq("custodian-cid-B"))
            .load(&mut conn)
            .expect("load all");
        assert_eq!(all.len(), 2, "both rows must exist for audit trail");
        let superseded: Vec<_> = all.iter().filter(|r| r.superseded_at.is_some()).collect();
        assert_eq!(superseded.len(), 1, "exactly one row must be superseded");
    }

    // ── CRUD: get returns None when no share installed ────────────────────────

    #[test]
    fn get_returns_none_when_no_share() {
        let mut conn = setup();
        let result = get_share_for_governance_action(
            &mut conn,
            "no-such-custodian",
            "no-such-governance-action",
        )
        .expect("query ok");
        assert!(result.is_none());
    }

    // ── CRUD: list_shares_for_custodian ───────────────────────────────────────

    #[test]
    fn list_shares_for_custodian_returns_active_only() {
        let mut conn = setup();

        // Install shares for two governance actions.
        upsert_share(
            &mut conn,
            "custodian-cid-C",
            "governance-action-A",
            1,
            b"share-A".to_vec(),
            "2026-05-15T00:00:00Z",
        )
        .expect("upsert A");

        upsert_share(
            &mut conn,
            "custodian-cid-C",
            "governance-action-B",
            2,
            b"share-B".to_vec(),
            "2026-05-15T00:01:00Z",
        )
        .expect("upsert B");

        // Supersede share-A with a replacement.
        upsert_share(
            &mut conn,
            "custodian-cid-C",
            "governance-action-A",
            1,
            b"share-A-v2".to_vec(),
            "2026-05-15T01:00:00Z",
        )
        .expect("upsert A v2");

        let active = list_shares_for_custodian(&mut conn, "custodian-cid-C").expect("list");

        // Only 2 active shares: share-A-v2 and share-B.
        assert_eq!(active.len(), 2, "list must exclude superseded rows");
        let cids: Vec<_> = active
            .iter()
            .map(|s| s.governance_action_cid.as_str())
            .collect();
        assert!(cids.contains(&"governance-action-A"), "A must be present");
        assert!(cids.contains(&"governance-action-B"), "B must be present");
        let a_bytes: Vec<_> = active
            .iter()
            .filter(|s| s.governance_action_cid == "governance-action-A")
            .collect();
        assert_eq!(a_bytes[0].share_data, b"share-A-v2");
    }

    // ── Integrity check catches corruption ────────────────────────────────────

    #[test]
    fn integrity_check_detects_tampered_share_data() {
        let mut conn = setup();

        upsert_share(
            &mut conn,
            "custodian-cid-D",
            "governance-action-003",
            1,
            b"clean-share-bytes".to_vec(),
            "2026-05-15T00:00:00Z",
        )
        .expect("upsert");

        // Tamper with the share_data directly in SQLite to simulate corruption.
        diesel::update(
            custodian_shares::table.filter(custodian_shares::custodian_cid.eq("custodian-cid-D")),
        )
        .set(custodian_shares::share_data.eq(b"corrupted-bytes".to_vec()))
        .execute(&mut conn)
        .expect("tamper");

        let result =
            get_share_for_governance_action(&mut conn, "custodian-cid-D", "governance-action-003");

        assert!(
            result.is_err(),
            "integrity check must fail on tampered share_data"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("integrity failure"),
            "error must mention integrity failure: {err}"
        );
    }

    // ── sha256_hex helper ─────────────────────────────────────────────────────

    #[test]
    fn sha256_hex_known_vector() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let hash = sha256_hex(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
