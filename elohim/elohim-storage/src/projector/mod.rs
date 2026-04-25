//! EPR Projector — consumes EPR atoms from `epr_atoms` and projects them into
//! pillar SQL read-model tables according to manifest-declared mappings.
//!
//! ## Principle P1 — elohim-storage as reconciliation controller
//!
//! The projector is the second integration controller from EPR Phase 2B
//! Decision #3. It complements the [`ReconcileController`] (which handles DHT
//! signals) by consuming EPR atoms and projecting them into pillar read-models.
//!
//! ## Architecture
//!
//! ```text
//! epr_atoms table (ingest writes here)
//!     │
//!     │  Projector::run_one_pass
//!     ▼
//! projector_cursor (per-projection watermark)
//!     │
//!     │  fetch_epr_atoms_since
//!     ▼
//! pillar read-model tables (economic_events, ...)
//!     (populated by Task B.4 — UPSERT stub for B.3)
//! ```
//!
//! ## Current state (B.3 skeleton)
//!
//! `run_one_pass` walks each registered projection, fetches new atoms since the
//! cursor, calls the stub `project()` method (returns `Ok(())`), and advances
//! the cursor. Task B.4 fills in the actual UPSERT logic.

pub mod cursor;
pub mod mapping;

pub use cursor::{advance_cursor, load_cursor, CursorError, ProjectorCursorRow};
pub use mapping::{ManifestRegistry, RegisteredProjection, RegistryError};

use std::sync::Arc;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{debug, trace};

use crate::db::diesel_schema::epr_atoms;
use crate::db::epr_atoms::EprAtom;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur in the projector.
#[derive(Debug, Error)]
pub enum ProjectorError {
    #[error("diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),

    #[error("cursor error: {0}")]
    Cursor(#[from] CursorError),
}

// ---------------------------------------------------------------------------
// PassReport
// ---------------------------------------------------------------------------

/// Summary of a single projector pass.
#[derive(Debug, Clone, Default)]
pub struct PassReport {
    /// Total number of atoms processed across all projections.
    pub atoms_processed: usize,
    /// Number of projections that were advanced.
    pub projections_advanced: usize,
}

// ---------------------------------------------------------------------------
// Projector
// ---------------------------------------------------------------------------

/// EPR atom projector — advances per-(pillar, kind) cursors and projects atoms
/// into pillar read-model tables per manifest-declared mappings.
pub struct Projector {
    manifest_registry: Arc<ManifestRegistry>,
}

impl Projector {
    /// Construct a projector with the given manifest registry.
    pub fn new(manifest_registry: ManifestRegistry) -> Self {
        Self {
            manifest_registry: Arc::new(manifest_registry),
        }
    }

    /// Construct a projector with a shared registry reference.
    pub fn with_registry(manifest_registry: Arc<ManifestRegistry>) -> Self {
        Self { manifest_registry }
    }

    /// Run one projection pass across all registered projections.
    ///
    /// For each projection:
    /// 1. Load the current cursor (or start from the beginning if absent).
    /// 2. Fetch all new atoms since the cursor's `last_issued_at`.
    /// 3. For each atom, invoke [`Self::project`] (stub in B.3 — no-op log).
    /// 4. Advance the cursor to the atom's CID and issued_at.
    ///
    /// Returns a [`PassReport`] summarising the pass.
    pub fn run_one_pass(&self, conn: &mut SqliteConnection) -> Result<PassReport, ProjectorError> {
        let mut report = PassReport::default();

        for projection in self.manifest_registry.all_projections() {
            let cursor = load_cursor(conn, &projection.pillar, &projection.kind)?;

            let since_issued_at = cursor
                .as_ref()
                .and_then(|c| c.last_issued_at.as_deref())
                .unwrap_or("1970-01-01T00:00:00Z");

            let atoms = fetch_epr_atoms_since(
                conn,
                &projection.kind,
                &projection.schema_key,
                since_issued_at,
            )?;

            if atoms.is_empty() {
                trace!(
                    pillar = %projection.pillar,
                    kind = %projection.kind,
                    "no new atoms since cursor"
                );
                continue;
            }

            for atom in &atoms {
                self.project(conn, projection, atom)?;
                advance_cursor(
                    conn,
                    &projection.pillar,
                    &projection.kind,
                    &atom.cid,
                    &atom.issued_at,
                )?;
                report.atoms_processed += 1;
            }

            report.projections_advanced += 1;
            debug!(
                pillar = %projection.pillar,
                kind = %projection.kind,
                atoms = atoms.len(),
                "projection advanced"
            );
        }

        Ok(report)
    }

    // -----------------------------------------------------------------------
    // Stub handler — Task B.4 replaces this with UPSERT logic
    // -----------------------------------------------------------------------

    /// Project a single EPR atom into the target pillar table.
    ///
    /// **B.3 stub**: logs the projection intent and returns `Ok(())`.
    /// Task B.4 fills in the actual UPSERT into `projection.target_table`
    /// using the `column_mapping` JSONPath expressions over `atom.payload_bytes`.
    fn project(
        &self,
        _conn: &mut SqliteConnection,
        projection: &RegisteredProjection,
        atom: &EprAtom,
    ) -> Result<(), ProjectorError> {
        // B.3 stub — intentional no-op. Cursor advances; no table writes yet.
        // Task B.4 replaces this with a generic JSONPath → UPSERT implementation.
        trace!(
            target_table = %projection.target_table,
            atom_cid = %atom.cid,
            "project stub: would upsert atom into target table (B.4)"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

/// Fetch all EPR atoms for a given (kind, schema_key) pair whose `issued_at`
/// is strictly after `since_issued_at`, ordered by `issued_at` ascending.
///
/// Used by [`Projector::run_one_pass`] to page forward from the last cursor.
pub fn fetch_epr_atoms_since(
    conn: &mut SqliteConnection,
    kind: &str,
    schema_key: &str,
    since_issued_at: &str,
) -> Result<Vec<EprAtom>, diesel::result::Error> {
    epr_atoms::table
        .filter(epr_atoms::kind.eq(kind))
        .filter(epr_atoms::schema_key.eq(schema_key))
        .filter(epr_atoms::issued_at.gt(since_issued_at))
        .order(epr_atoms::issued_at.asc())
        .load::<EprAtom>(conn)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::epr_atoms::EprAtom;
    use crate::projector::cursor::load_cursor;
    use crate::projector::mapping::mock_manifest_registry;
    use crate::test_util::test_pool;
    use diesel::prelude::*;

    /// Insert a minimal EprAtom with the given CID, kind, and issued_at into the test DB.
    fn insert_epr_atom(
        conn: &mut SqliteConnection,
        cid: &str,
        kind: &str,
        schema_key: &str,
        issued_at: &str,
    ) {
        let atom = EprAtom {
            cid: cid.to_string(),
            kind: kind.to_string(),
            schema_ref: "test-schema-ref".to_string(),
            schema_key: schema_key.to_string(),
            reach: "commons".to_string(),
            issued_at: issued_at.to_string(),
            signer_cid: "test-signer".to_string(),
            supersedes: None,
            canonical_bytes: vec![0u8; 4],
            payload_bytes: vec![0u8; 4],
            proof_bytes: vec![0u8; 64],
            proof_algorithm: "ed25519".to_string(),
            verified_at: None,
            verified_signer_fingerprint: None,
        };
        diesel::insert_into(epr_atoms::table)
            .values(&atom)
            .execute(conn)
            .expect("insert epr_atom");
    }

    /// B.3 acceptance test: projector advances cursor after a single pass.
    ///
    /// Setup: one EprAtom with kind="EconomicEvent", schema_key="economic-event".
    /// Action: `run_one_pass`.
    /// Assert: cursor row has `last_epr_cid = "test-cid-x"`.
    #[test]
    fn projector_advances_cursor_after_pass() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        // Insert one EconomicEvent atom.
        insert_epr_atom(
            &mut conn,
            "test-cid-x",
            "EconomicEvent",
            "economic-event",
            "2026-01-01T00:00:00Z",
        );

        // Build projector with the mock registry (shefa / EconomicEvent).
        let registry = mock_manifest_registry();
        let projector = Projector::new(registry);

        // Run one pass.
        let report = projector.run_one_pass(&mut conn).expect("run_one_pass");

        // Cursor must have advanced to the atom's CID.
        let cursor = load_cursor(&mut conn, "shefa", "EconomicEvent")
            .expect("load_cursor")
            .expect("cursor row must exist after pass");

        assert_eq!(
            cursor.last_epr_cid.as_deref(),
            Some("test-cid-x"),
            "cursor must point to the projected atom's CID"
        );

        // Report sanity.
        assert_eq!(report.atoms_processed, 1);
        assert_eq!(report.projections_advanced, 1);
    }

    /// A second pass with no new atoms is a no-op — cursor stays put.
    #[test]
    fn projector_second_pass_with_no_new_atoms_is_noop() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        insert_epr_atom(
            &mut conn,
            "cid-only",
            "EconomicEvent",
            "economic-event",
            "2026-01-01T00:00:00Z",
        );

        let projector = Projector::new(mock_manifest_registry());
        projector.run_one_pass(&mut conn).unwrap();

        // Second pass — no new atoms.
        let report2 = projector.run_one_pass(&mut conn).unwrap();
        assert_eq!(report2.atoms_processed, 0);
        assert_eq!(report2.projections_advanced, 0);

        // Cursor still points to the first atom.
        let cursor = load_cursor(&mut conn, "shefa", "EconomicEvent")
            .unwrap()
            .unwrap();
        assert_eq!(cursor.last_epr_cid.as_deref(), Some("cid-only"));
    }

    /// Empty atom table — pass is a no-op with zero atoms processed.
    #[test]
    fn projector_empty_atom_table_is_noop() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        let projector = Projector::new(mock_manifest_registry());
        let report = projector.run_one_pass(&mut conn).unwrap();

        assert_eq!(report.atoms_processed, 0);
        assert_eq!(report.projections_advanced, 0);

        // No cursor row should be created.
        let cursor = load_cursor(&mut conn, "shefa", "EconomicEvent").unwrap();
        assert!(cursor.is_none());
    }
}
