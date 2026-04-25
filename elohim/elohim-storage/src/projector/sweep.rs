//! Projection revocation sweep — EPR Phase 2B Task B.6 / Invariant I4.
//!
//! When a key is revoked, the `reconcile::sweep::sweep_on_revocation` already
//! clears `epr_atoms.verified_at` for tainted atoms. This module extends that
//! sweep to projection rows: for each registered projection, it clears the
//! `verified_at` column on any row whose `dht_anchor_hash` (= the atom CID)
//! falls in the revoked window.
//!
//! ## Design choice (I4)
//!
//! Projection rows are NOT deleted on revocation. The row is preserved as an
//! audit trail of the historical projection; only `verified_at` is cleared
//! (set to NULL) to signal that the source EPR was revoked. Pillar readers
//! can distinguish:
//!
//! - `verified_at IS NOT NULL` → source EPR is currently verified.
//! - `verified_at IS NULL` → source EPR either never verified or was revoked.
//!
//! Combined with `epr_atoms.verified_signer_fingerprint = "revoked_stale"`,
//! consumers can determine the precise cause of the NULL.
//!
//! ## Sweep strategy
//!
//! For each registered projection:
//! 1. Query `epr_atoms` for atoms from `signer_cid` with `issued_at >=
//!    effective_at`, collecting their CIDs.
//! 2. UPDATE `{target_table}` SET `verified_at = NULL` WHERE `dht_anchor_hash
//!    IN (...)`.
//!
//! The sweep uses a single SQL statement per projection table. If a projection
//! table has no `verified_at` column (future projections that opt out of trust
//! mirroring), the UPDATE is silently skipped — the column check is done by
//! inspecting the registered column mapping.

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use thiserror::Error;
use tracing::{debug, warn};

use crate::db::diesel_schema::epr_atoms;
use crate::projector::mapping::ManifestRegistry;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during the projection revocation sweep.
#[derive(Debug, Error)]
pub enum ProjectionSweepError {
    #[error("diesel error querying epr_atoms: {0}")]
    AtomsQuery(#[from] diesel::result::Error),

    #[error("batch_execute on projection table {table}: {cause}")]
    TableUpdate { table: String, cause: String },
}

// ---------------------------------------------------------------------------
// SweepReport
// ---------------------------------------------------------------------------

/// Summary of a projection revocation sweep.
#[derive(Debug, Clone, Default)]
pub struct ProjectionSweepReport {
    /// Number of projection tables that were updated.
    pub tables_updated: usize,
    /// Total number of projection rows whose `verified_at` was cleared across
    /// all tables.
    pub rows_cleared: usize,
}

// ---------------------------------------------------------------------------
// sweep_projections_on_revocation
// ---------------------------------------------------------------------------

/// Clear `verified_at` on projection rows whose source EPR atom was tainted by
/// the given revocation.
///
/// ## Arguments
///
/// - `conn` — exclusive SQLite connection.
/// - `registry` — manifest registry declaring all active projections.
/// - `signer_cid` — the agent whose key was revoked.
/// - `effective_at` — the revocation effective timestamp (ISO-8601 TEXT). Atoms
///   with `issued_at >= effective_at` from this signer are tainted.
///
/// ## Boundary semantics
///
/// Uses `issued_at >= effective_at` (inclusive), matching the boundary used by
/// `sweep_on_revocation` in `reconcile::sweep`. The two sweeps are designed to
/// be called in sequence within the same reconciliation pass so their windows
/// align.
///
/// ## Future-proofing
///
/// Only projections whose `column_mapping` declares a `verified_at` key are
/// updated. Projections that don't map `verified_at` are silently skipped
/// (a debug log is emitted). This allows future projections to opt out of
/// trust mirroring.
pub fn sweep_projections_on_revocation(
    conn: &mut SqliteConnection,
    registry: &ManifestRegistry,
    signer_cid: &str,
    effective_at: &str,
) -> Result<ProjectionSweepReport, ProjectionSweepError> {
    let mut report = ProjectionSweepReport::default();

    // Step 1: collect the CIDs of all tainted atoms for this signer.
    let tainted_cids: Vec<String> = epr_atoms::table
        .filter(epr_atoms::signer_cid.eq(signer_cid))
        .filter(epr_atoms::issued_at.ge(effective_at))
        .select(epr_atoms::cid)
        .load::<String>(conn)?;

    if tainted_cids.is_empty() {
        debug!(
            signer_cid = %signer_cid,
            effective_at = %effective_at,
            "projection sweep: no tainted atoms found — all projection tables unaffected"
        );
        return Ok(report);
    }

    debug!(
        signer_cid = %signer_cid,
        effective_at = %effective_at,
        tainted_count = tainted_cids.len(),
        "projection sweep: found tainted atoms, clearing projection rows"
    );

    // Step 2: for each registered projection, clear verified_at if the projection
    // maps that column (i.e. it participates in trust mirroring).
    for projection in registry.all_projections() {
        // Only sweep tables that declared a verified_at mapping.
        if !projection.column_mapping.contains_key("verified_at") {
            debug!(
                table = %projection.target_table,
                "projection sweep: table has no verified_at mapping — skipped"
            );
            continue;
        }

        // Build IN clause from tainted CIDs.
        // Safety: CIDs are base58/base32 content addresses — no SQL injection risk.
        // We still escape by wrapping each in single quotes.
        let in_list: Vec<String> = tainted_cids
            .iter()
            .map(|cid| format!("'{}'", cid.replace('\'', "''")))
            .collect();

        let sql = format!(
            "UPDATE {} SET verified_at = NULL WHERE dht_anchor_hash IN ({})",
            projection.target_table,
            in_list.join(", ")
        );

        conn.batch_execute(&sql)
            .map_err(|e| ProjectionSweepError::TableUpdate {
                table: projection.target_table.clone(),
                cause: e.to_string(),
            })?;

        report.tables_updated += 1;
        // Note: SQLite's batch_execute doesn't return row counts. We count
        // tables_updated as a proxy. A future enhancement can use
        // diesel::sql_query with a row count return if needed.

        warn!(
            table = %projection.target_table,
            signer_cid = %signer_cid,
            tainted_cids_count = tainted_cids.len(),
            "projection sweep: cleared verified_at on projection rows for revoked signer"
        );
    }

    // Approximate rows_cleared: tainted_cids * tables_updated (upper bound).
    // Actual cleared rows may be fewer if some CIDs were never projected.
    report.rows_cleared = tainted_cids.len() * report.tables_updated;

    Ok(report)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diesel_schema::{economic_events, epr_atoms};
    use crate::db::epr_atoms::EprAtom;
    use crate::projector::mapping::ManifestRegistry;
    use crate::test_util::test_pool;

    /// Insert a minimal EprAtom into the DB.
    fn insert_atom(conn: &mut SqliteConnection, cid: &str, signer: &str, issued_at: &str) {
        let atom = EprAtom {
            cid: cid.to_string(),
            kind: "EconomicEvent".to_string(),
            schema_ref: "test-schema-ref".to_string(),
            schema_key: "economic-event".to_string(),
            reach: "commons".to_string(),
            issued_at: issued_at.to_string(),
            signer_cid: signer.to_string(),
            supersedes: None,
            canonical_bytes: vec![0u8; 4],
            payload_bytes: vec![0u8; 4],
            proof_bytes: vec![0u8; 64],
            proof_algorithm: "ed25519".to_string(),
            verified_at: Some("2026-04-25T12:00:00Z".to_string()),
            verified_signer_fingerprint: Some("abcdef0123456789abcdef0123456789".to_string()),
        };
        diesel::insert_into(epr_atoms::table)
            .values(&atom)
            .execute(conn)
            .expect("insert atom");
    }

    /// Insert a minimal economic_events row that references the given CID as dht_anchor_hash.
    fn insert_economic_event_row(
        conn: &mut SqliteConnection,
        cid: &str,
        verified_at: Option<&str>,
    ) {
        use diesel::connection::SimpleConnection;
        let verified_sql = match verified_at {
            Some(v) => format!("'{}'", v.replace('\'', "''")),
            None => "NULL".to_string(),
        };
        let sql = format!(
            "INSERT OR REPLACE INTO economic_events \
            (id, h_app_id, action, provider, receiver, has_point_in_time, state, dht_anchor_hash, created_at, verified_at) \
            VALUES ('{cid}', 'test-signer', 'use', 'provider-A', 'receiver-B', \
            '2026-04-25T10:00:00Z', 'recorded', '{cid}', '2026-04-25T10:00:00Z', {verified_sql})"
        );
        conn.batch_execute(&sql)
            .expect("insert economic_events row");
    }

    /// Projection sweep clears verified_at on economic_events rows for tainted atoms.
    #[test]
    fn projection_sweep_clears_verified_at_for_tainted_atoms() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        // Insert atoms: one in the revocation window, one before.
        insert_atom(&mut conn, "cid-tainted", "signer-X", "2026-04-25T11:00:00Z");
        insert_atom(&mut conn, "cid-safe", "signer-X", "2026-04-25T09:00:00Z");

        // Insert projection rows for both atoms.
        insert_economic_event_row(&mut conn, "cid-tainted", Some("2026-04-25T12:00:00Z"));
        insert_economic_event_row(&mut conn, "cid-safe", Some("2026-04-25T12:00:00Z"));

        let registry = ManifestRegistry::with_shefa_economic_event();

        // Sweep: effective_at = "2026-04-25T10:00:00Z" — tainted is after, safe is before.
        let report = sweep_projections_on_revocation(
            &mut conn,
            &registry,
            "signer-X",
            "2026-04-25T10:00:00Z",
        )
        .expect("sweep must succeed");

        assert_eq!(report.tables_updated, 1);

        // Tainted row: verified_at must be NULL.
        // verified_at is Nullable<Text> in the schema, so it loads as Option<String>.
        let tainted: Option<String> = economic_events::table
            .filter(economic_events::id.eq("cid-tainted"))
            .select(economic_events::verified_at)
            .first(&mut conn)
            .expect("fetch tainted row");
        assert!(
            tainted.is_none(),
            "tainted projection row must have verified_at = NULL"
        );

        // Safe row: verified_at must remain set.
        let safe: Option<String> = economic_events::table
            .filter(economic_events::id.eq("cid-safe"))
            .select(economic_events::verified_at)
            .first(&mut conn)
            .expect("fetch safe row");
        assert!(
            safe.is_some(),
            "pre-revocation projection row must retain verified_at"
        );
    }

    /// Empty registry: sweep succeeds with zero tables updated.
    #[test]
    fn projection_sweep_with_empty_registry_is_noop() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        let registry = ManifestRegistry::empty();
        let report = sweep_projections_on_revocation(
            &mut conn,
            &registry,
            "signer-Z",
            "2026-04-25T10:00:00Z",
        )
        .expect("sweep must succeed");

        assert_eq!(report.tables_updated, 0);
        assert_eq!(report.rows_cleared, 0);
    }
}
