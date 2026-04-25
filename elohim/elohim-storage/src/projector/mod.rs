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
//! ```
//!
//! ## Column mapping source paths (Task B.4)
//!
//! Each entry in a manifest's `columnMapping` maps a snake_case DB column name
//! to a **source path**, which is one of:
//!
//! - `payload.<dotted.path>` — JSON path into the atom's `payload_bytes`
//!   (deserialized as a JSON object). Dots separate object keys. Only string
//!   and number leaf values are projected; objects/arrays are silently skipped.
//!
//! - `$cid` — the atom's `cid` field (content address). Idempotent UPSERT key.
//!
//! - `$signer` — the atom's `signer_cid` field (economic agent identity).
//!
//! - `$issuedAt` — the atom's `issued_at` field (ISO-8601 TEXT).
//!
//! - `$state:<default>` — a literal constant value (e.g. `$state:recorded`
//!   maps to the string `"recorded"`).
//!
//! All other `$`-prefixed paths that don't match the above are treated as
//! unknown and silently skipped (column left out of the INSERT).
//!
//! ## UPSERT semantics
//!
//! The projector issues `INSERT OR REPLACE INTO {target_table}` (SQLite alias
//! for `INSERT OR REPLACE`, equivalent to `INSERT ... ON CONFLICT DO REPLACE`).
//! The `id` column — sourced from `$cid` — makes every UPSERT idempotent: if
//! the projector re-processes an atom that was already projected (e.g. after a
//! crash-restart replay), the row is replaced in place rather than duplicated.
//!
//! ## NOT NULL columns not covered by the manifest
//!
//! The manifest only maps columns it knows about. NOT NULL columns that the
//! manifest does not map (`has_point_in_time` being the sentinel) must be
//! covered by `$`-prefixed rules or default values. If a required column is
//! absent from the built column set AND has no fallback, the INSERT will fail
//! with a Diesel `NotNullViolation`. The canonical resolution is to add the
//! mapping to the manifest (see shefa `manifest.json`'s `projections[0]`).

pub mod cursor;
pub mod mapping;

pub use cursor::{advance_cursor, load_cursor, CursorError, ProjectorCursorRow};
pub use mapping::{ManifestRegistry, RegisteredProjection, RegistryError};

use std::collections::BTreeMap;
use std::sync::Arc;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{debug, trace, warn};

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

    #[error("projection error for table {table}: {cause}")]
    Projection { table: String, cause: String },
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
    // Column mapping evaluator (Task B.4)
    // -----------------------------------------------------------------------

    /// Project a single EPR atom into the target pillar table.
    ///
    /// Evaluates each `(column, source_path)` entry in the manifest's
    /// `column_mapping`, resolves a scalar string value for each, and issues
    /// an `INSERT OR REPLACE INTO {target_table}` via `diesel::sql_query`.
    ///
    /// ## Source path resolution
    ///
    /// - `payload.<dotted.path>` → walk the atom's JSON payload. Only string
    ///   and number leaves are emitted; objects/arrays are skipped.
    /// - `$cid` → `atom.cid`
    /// - `$signer` → `atom.signer_cid`
    /// - `$issuedAt` → `atom.issued_at`
    /// - `$state:<literal>` → the literal string after the colon (e.g. `"recorded"`)
    /// - Any other `$`-prefixed path → warn + skip (column omitted from INSERT)
    ///
    /// Columns with no resolvable value are silently omitted from the INSERT.
    /// If a NOT NULL column is omitted, Diesel will return a constraint error.
    fn project(
        &self,
        conn: &mut SqliteConnection,
        projection: &RegisteredProjection,
        atom: &EprAtom,
    ) -> Result<(), ProjectorError> {
        trace!(
            target_table = %projection.target_table,
            atom_cid = %atom.cid,
            "projecting atom into target table"
        );

        // Deserialize payload — payload_bytes is JSON-encoded in the EPR atom.
        let payload: serde_json::Value = if atom.payload_bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&atom.payload_bytes).unwrap_or(serde_json::Value::Null)
        };

        // Resolve each column → scalar string value.
        let resolved = evaluate_column_mapping(&projection.column_mapping, atom, &payload);

        if resolved.is_empty() {
            warn!(
                target_table = %projection.target_table,
                atom_cid = %atom.cid,
                "column mapping produced no columns — skipping UPSERT"
            );
            return Ok(());
        }

        // Build a dynamic INSERT OR REPLACE via SimpleConnection::batch_execute.
        //
        // Diesel's typed query builder cannot handle dynamic column sets, and
        // diesel::sql_query::bind() returns a new owned type each call, making
        // loop-based accumulation impossible. Instead we embed the values inline
        // in the SQL string, escaped for SQLite's TEXT literal syntax.
        //
        // Safety: all values come from controlled sources — EPR CID strings
        // (base58 content addresses), ISO-8601 timestamps, and JSON string/number
        // scalars extracted by the evaluator. None are user-supplied HTML. We
        // escape each value by replacing single-quotes with '' (SQLite convention).
        let columns: Vec<&str> = resolved.keys().map(String::as_str).collect();
        let values_sql: Vec<String> = columns
            .iter()
            .map(|col| {
                let val = resolved.get(*col).expect("key from own map");
                format!("'{}'", val.replace('\'', "''"))
            })
            .collect();

        let sql = format!(
            "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
            projection.target_table,
            columns.join(", "),
            values_sql.join(", ")
        );

        use diesel::connection::SimpleConnection;
        conn.batch_execute(&sql)
            .map_err(|e| ProjectorError::Projection {
                table: projection.target_table.clone(),
                cause: format!("SQL={sql} error={e}"),
            })?;

        debug!(
            target_table = %projection.target_table,
            atom_cid = %atom.cid,
            columns = columns.len(),
            "atom projected successfully"
        );

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Column mapping evaluator
// ---------------------------------------------------------------------------

/// Evaluate a manifest `column_mapping` against an EPR atom and its decoded
/// payload, returning a map of `column_name → resolved_string_value`.
///
/// Columns that cannot be resolved (unknown `$`-prefix, missing payload path,
/// non-scalar payload value) are silently omitted. The caller decides whether
/// to treat an empty result as an error.
///
/// This function is `pub(crate)` so the integration test can call it directly.
pub(crate) fn evaluate_column_mapping(
    column_mapping: &BTreeMap<String, String>,
    atom: &EprAtom,
    payload: &serde_json::Value,
) -> BTreeMap<String, String> {
    let mut resolved: BTreeMap<String, String> = BTreeMap::new();

    for (column, source_path) in column_mapping {
        // Skip comment entries (manifest allows a "_comment" key for documentation).
        if column.starts_with('_') {
            continue;
        }

        let value_opt = if source_path.starts_with('$') {
            resolve_projector_source(source_path, atom)
        } else if let Some(dotted) = source_path.strip_prefix("payload.") {
            resolve_payload_path(dotted, payload)
        } else {
            // Unrecognised source prefix — log and skip.
            warn!(
                column = %column,
                source = %source_path,
                "unknown source path prefix in column mapping — skipping"
            );
            None
        };

        if let Some(value) = value_opt {
            resolved.insert(column.clone(), value);
        } else {
            trace!(
                column = %column,
                source = %source_path,
                "column mapping produced no value — column omitted from INSERT"
            );
        }
    }

    resolved
}

/// Resolve a `$`-prefixed projector-derived source path against the atom.
///
/// Supported rules:
/// - `$cid` → atom CID string
/// - `$signer` → atom signer CID string
/// - `$issuedAt` → atom issued_at ISO-8601 string
/// - `$state:<literal>` → the literal string after the colon
///
/// Returns `None` for unknown `$`-prefixed paths.
fn resolve_projector_source(source_path: &str, atom: &EprAtom) -> Option<String> {
    match source_path {
        "$cid" => Some(atom.cid.clone()),
        "$signer" => Some(atom.signer_cid.clone()),
        "$issuedAt" => Some(atom.issued_at.clone()),
        other if other.starts_with("$state:") => {
            let literal = other.trim_start_matches("$state:");
            Some(literal.to_string())
        }
        unknown => {
            warn!(
                source = %unknown,
                "unknown $-prefixed projector source — column will be omitted"
            );
            None
        }
    }
}

/// Walk a dotted payload path (e.g. `"resourceQuantity.numericValue"`) through
/// a JSON value and return the leaf as a string.
///
/// Only string and number leaves are returned. Objects, arrays, booleans, and
/// null produce `None` (the column is omitted from the INSERT).
fn resolve_payload_path(dotted_path: &str, payload: &serde_json::Value) -> Option<String> {
    let mut current = payload;
    for key in dotted_path.split('.') {
        match current.get(key) {
            Some(next) => current = next,
            None => return None,
        }
    }
    scalar_to_string(current)
}

/// Convert a JSON scalar (string or number) to a `String`. Returns `None` for
/// objects, arrays, booleans, and null.
fn scalar_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
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
    ///
    /// `payload_bytes` carries a minimal JSON payload that satisfies the NOT NULL
    /// columns of `economic_events` (`action`, `provider`, `receiver`,
    /// `hasPointInTime`) so the real column mapping evaluator can UPSERT
    /// successfully in the projector_advances_cursor tests.
    fn insert_epr_atom(
        conn: &mut SqliteConnection,
        cid: &str,
        kind: &str,
        schema_key: &str,
        issued_at: &str,
    ) {
        let payload = serde_json::json!({
            "action": "use",
            "provider": "agent:provider-skeleton",
            "receiver": "agent:receiver-skeleton",
            "hasPointInTime": issued_at
        });
        let payload_bytes = serde_json::to_vec(&payload).expect("serialize skeleton payload");

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
            payload_bytes,
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
