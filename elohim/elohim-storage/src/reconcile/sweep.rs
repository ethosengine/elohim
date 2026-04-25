//! Eager revocation sweep — Task A.8.
//!
//! When a [`KeyRevocationSignal`] arrives, any EPR atom whose signer is the
//! revoked agent AND whose `issued_at` falls on or after `compromise_at` is
//! considered tainted: the signing key was already compromised at that point.
//! Pre-compromise attestations (issued *before* `compromise_at`) remain valid
//! because the signature was legitimate when it was produced.
//!
//! ## Sweep semantics
//!
//! | Atom `issued_at`          | `verified_at` after sweep | `verified_signer_fingerprint` |
//! |---------------------------|---------------------------|-------------------------------|
//! | `< compromise_at`         | unchanged (still valid)   | unchanged                     |
//! | `>= compromise_at`        | `NULL`                    | `"revoked_stale"`             |
//!
//! The `(signer_cid, issued_at)` index from A.7 makes the sweep O(k) where k
//! is the number of tainted rows, not the full table size.
//!
//! ## Audit / re-verify surface
//!
//! Rows tagged `verified_signer_fingerprint = "revoked_stale"` can be
//! targeted precisely for re-verification once a replacement key is established
//! or for audit removal.
//!
//! ## Deferral pointer
//!
//! The affected-peer list (who holds atoms that were tainted) is consumed by
//! Batch D's direct-notify task. See `TODO(phase-2b-D5)` below.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::db::diesel_schema::epr_atoms::dsl::{
    epr_atoms, issued_at, signer_cid, verified_at, verified_signer_fingerprint,
};
use crate::reconcile::signal_stream::KeyRevocationSignal;

/// Sentinel value written to `verified_signer_fingerprint` for tainted rows.
///
/// Chosen to be distinguishable from any real blake3-128-prefix fingerprint
/// (which is always 32 lowercase hex chars). This value is 13 chars, so it
/// cannot be confused with a real fingerprint.
pub const REVOKED_STALE_FINGERPRINT: &str = "revoked_stale";

// ---------------------------------------------------------------------------
// SweepReport
// ---------------------------------------------------------------------------

/// Result of a single revocation sweep pass.
#[derive(Debug)]
pub struct SweepReport {
    /// Number of `epr_atoms` rows whose `verified_at` was cleared.
    pub affected_rows: usize,
    /// Wall-clock time at which the sweep completed.
    pub at: DateTime<Utc>,
    /// Agent CID that was revoked, triggering this sweep.
    pub agent_cid: String,
    /// The compromise boundary used as the sweep filter.
    ///
    /// Atoms with `issued_at >= compromise_at` were cleared.
    /// Atoms with `issued_at < compromise_at` were untouched.
    pub compromise_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// SweepError
// ---------------------------------------------------------------------------

/// Errors that can occur during a revocation sweep.
#[derive(Debug, thiserror::Error)]
pub enum SweepError {
    #[error("revocation sweep DB write failed: {0}")]
    DbWrite(#[from] diesel::result::Error),
}

// ---------------------------------------------------------------------------
// sweep_on_revocation
// ---------------------------------------------------------------------------

/// Clear `verified_at` for EPR atoms signed by the revoked agent after the
/// compromise point.
///
/// ## Arguments
///
/// - `conn` — exclusive SQLite connection (sync; caller holds the r2d2 lease).
/// - `signal` — the revocation signal carrying `agent_cid` and `compromise_at`.
///
/// ## Boundary semantics
///
/// The filter is `issued_at >= compromise_at` (inclusive). An atom signed
/// *exactly* at the compromise timestamp is considered tainted because the key
/// may already have been under adversary control at that instant.
///
/// ISO-8601 strings sort lexicographically in the same order as instants, so
/// string comparison is correct for the SQLite `TEXT` column.
///
/// ## Index use
///
/// The `(signer_cid, issued_at)` partial index created in A.7's migration is
/// used automatically by SQLite's query planner for this filter.
pub fn sweep_on_revocation(
    conn: &mut SqliteConnection,
    signal: &KeyRevocationSignal,
) -> Result<SweepReport, SweepError> {
    // Serialize compromise_at to the same ISO-8601 format used in the column.
    let compromise_str = signal
        .compromise_at
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let affected = diesel::update(
        epr_atoms
            .filter(signer_cid.eq(&signal.agent_cid))
            .filter(issued_at.ge(&compromise_str)),
    )
    .set((
        verified_at.eq::<Option<String>>(None),
        verified_signer_fingerprint.eq(REVOKED_STALE_FINGERPRINT),
    ))
    .execute(conn)?;

    // TODO(phase-2b-D5): emit affected-peer list for Batch D direct-notify
    // (D.5). The list of peers that hold tainted atoms can be derived by
    // querying peer_identity_bindings for agents whose epr_atoms were swept.
    // Not implemented here — flagged for Batch D.

    Ok(SweepReport {
        affected_rows: affected,
        at: Utc::now(),
        agent_cid: signal.agent_cid.clone(),
        compromise_at: signal.compromise_at,
    })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diesel_schema::epr_atoms::dsl as atoms_dsl;
    use crate::db::epr_atoms::EprAtom;
    use crate::reconcile::signal_stream::KeyRevocationSignal;
    use crate::test_util::test_pool;
    use chrono::{TimeZone, Utc};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn t(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0)
            .single()
            .expect("test timestamp must be valid")
    }

    fn ts(dt: DateTime<Utc>) -> String {
        dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    /// Insert a minimal EprAtom with the given signer and issued_at.
    /// `verified_at_dt` is `Some(t)` if the atom has been verified, or `None`.
    fn insert_test_atom(
        conn: &mut SqliteConnection,
        agent_cid: &str,
        issued: DateTime<Utc>,
        verified_dt: Option<DateTime<Utc>>,
    ) {
        let cid = format!("bafy-{agent_cid}-{}", issued.timestamp());
        let atom = EprAtom {
            cid,
            kind: "test-kind".into(),
            schema_ref: "test-schema".into(),
            schema_key: "test-key".into(),
            reach: "commons".into(),
            issued_at: ts(issued),
            signer_cid: agent_cid.to_string(),
            supersedes: None,
            canonical_bytes: vec![0u8; 4],
            payload_bytes: vec![0u8; 4],
            proof_bytes: vec![0u8; 64],
            proof_algorithm: "ed25519".into(),
            verified_at: verified_dt.map(ts),
            verified_signer_fingerprint: verified_dt
                .map(|_| "abcdef0123456789abcdef0123456789".to_string()),
        };
        diesel::insert_into(atoms_dsl::epr_atoms)
            .values(&atom)
            .execute(conn)
            .expect("insert test atom");
    }

    /// Fetch back an atom by (signer_cid, issued_at) for assertion.
    fn fetch_atom_at(
        conn: &mut SqliteConnection,
        agent: &str,
        issued: DateTime<Utc>,
    ) -> Option<EprAtom> {
        atoms_dsl::epr_atoms
            .filter(atoms_dsl::signer_cid.eq(agent))
            .filter(atoms_dsl::issued_at.eq(ts(issued)))
            .first::<EprAtom>(conn)
            .optional()
            .expect("fetch atom query")
    }

    fn sample_revocation(agent: &str, compromise: DateTime<Utc>) -> KeyRevocationSignal {
        KeyRevocationSignal {
            action_hash: "uhCkk-rev-action-hash".into(),
            agent_cid: agent.to_string(),
            revoked_pubkey: "cmV2b2tlZGtleWJhc2U2NAAAAAAAAAAAAAAAAAAAAAA".into(),
            compromise_at: compromise,
            effective_at: compromise,
            triggering_revocation_id: Some("rev-1".into()),
            emitted_at: Utc::now(),
        }
    }

    // -----------------------------------------------------------------------
    // Test: main sweep correctness
    // -----------------------------------------------------------------------

    /// Pre-compromise atoms stay verified; post-compromise atoms are cleared.
    /// Decoy atoms from another agent are untouched.
    #[test]
    fn sweep_clears_verified_within_compromise_window() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        // Agent A: t(50) pre-compromise, t(80) and t(100) post-compromise.
        // compromise_at = t(75).
        insert_test_atom(&mut conn, "agent-A", t(50), Some(t(51)));
        insert_test_atom(&mut conn, "agent-A", t(80), Some(t(81)));
        insert_test_atom(&mut conn, "agent-A", t(100), Some(t(101)));

        // Decoy: agent-B at the same post-compromise time — must not be touched.
        insert_test_atom(&mut conn, "agent-B", t(80), Some(t(81)));

        let signal = sample_revocation("agent-A", t(75));
        let report = sweep_on_revocation(&mut conn, &signal).expect("sweep");

        assert_eq!(report.affected_rows, 2, "two post-compromise EPRs cleared");
        assert_eq!(report.agent_cid, "agent-A");
        assert_eq!(report.compromise_at, t(75));

        // Pre-compromise: still verified, fingerprint unchanged.
        let pre = fetch_atom_at(&mut conn, "agent-A", t(50)).expect("pre-compromise atom");
        assert_eq!(pre.verified_at, Some(ts(t(51))));
        assert_eq!(
            pre.verified_signer_fingerprint,
            Some("abcdef0123456789abcdef0123456789".to_string())
        );

        // Post-compromise t(80): verified_at cleared, fingerprint = "revoked_stale".
        let post1 = fetch_atom_at(&mut conn, "agent-A", t(80)).expect("post-compromise atom 1");
        assert!(
            post1.verified_at.is_none(),
            "verified_at must be NULL after sweep"
        );
        assert_eq!(
            post1.verified_signer_fingerprint,
            Some(REVOKED_STALE_FINGERPRINT.to_string())
        );

        // Post-compromise t(100): cleared as well.
        let post2 = fetch_atom_at(&mut conn, "agent-A", t(100)).expect("post-compromise atom 2");
        assert!(post2.verified_at.is_none());
        assert_eq!(
            post2.verified_signer_fingerprint,
            Some(REVOKED_STALE_FINGERPRINT.to_string())
        );

        // Decoy agent-B: completely untouched.
        let decoy = fetch_atom_at(&mut conn, "agent-B", t(80)).expect("decoy atom");
        assert_eq!(decoy.verified_at, Some(ts(t(81))));
        assert_ne!(
            decoy.verified_signer_fingerprint,
            Some(REVOKED_STALE_FINGERPRINT.to_string())
        );
    }

    // -----------------------------------------------------------------------
    // Test: boundary inclusivity
    // -----------------------------------------------------------------------

    /// An atom signed exactly at compromise_at is swept (>= is inclusive).
    #[test]
    fn sweep_at_compromise_boundary_is_inclusive() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        // Atom at exactly t(100) — same as compromise_at.
        insert_test_atom(&mut conn, "agent-C", t(100), Some(t(101)));
        // Atom at t(99) — one second before, must NOT be swept.
        insert_test_atom(&mut conn, "agent-C", t(99), Some(t(100)));

        let signal = sample_revocation("agent-C", t(100));
        let report = sweep_on_revocation(&mut conn, &signal).expect("sweep");

        assert_eq!(report.affected_rows, 1, "only the boundary atom is swept");

        let at_boundary = fetch_atom_at(&mut conn, "agent-C", t(100)).expect("boundary atom");
        assert!(at_boundary.verified_at.is_none());
        assert_eq!(
            at_boundary.verified_signer_fingerprint,
            Some(REVOKED_STALE_FINGERPRINT.to_string())
        );

        let before = fetch_atom_at(&mut conn, "agent-C", t(99)).expect("pre-boundary atom");
        assert!(
            before.verified_at.is_some(),
            "pre-boundary must stay verified"
        );
    }

    // -----------------------------------------------------------------------
    // Test: no atoms for agent — zero rows, no error
    // -----------------------------------------------------------------------

    #[test]
    fn sweep_with_no_matching_atoms_returns_zero() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        let signal = sample_revocation("nonexistent-agent", t(100));
        let report = sweep_on_revocation(&mut conn, &signal).expect("sweep");

        assert_eq!(report.affected_rows, 0);
    }

    // -----------------------------------------------------------------------
    // Test: unverified atoms (verified_at already NULL) don't error
    // -----------------------------------------------------------------------

    #[test]
    fn sweep_handles_already_null_verified_at() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        // Insert an atom that was never verified (A.7 path with no timeline).
        insert_test_atom(&mut conn, "agent-D", t(200), None);

        let signal = sample_revocation("agent-D", t(100));
        let report = sweep_on_revocation(&mut conn, &signal).expect("sweep");

        // The row is post-compromise so it matches the filter; it gets tagged
        // revoked_stale regardless. The verified_at stays NULL (already was).
        assert_eq!(report.affected_rows, 1);

        let atom = fetch_atom_at(&mut conn, "agent-D", t(200)).expect("atom");
        assert!(atom.verified_at.is_none());
        assert_eq!(
            atom.verified_signer_fingerprint,
            Some(REVOKED_STALE_FINGERPRINT.to_string())
        );
    }
}
