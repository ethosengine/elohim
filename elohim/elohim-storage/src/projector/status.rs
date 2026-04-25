//! Projector status builder — powers the `/api/v1/status/projector` endpoint.
//!
//! ## Lag definition
//!
//! For each `(pillar, kind)` projection declared in the manifest registry,
//! the status computes:
//!
//! - **cursor**: the projector's `projector_cursor` row (where it last got to)
//! - **lag**: `max(epr_atoms.issued_at) - cursor.last_issued_at` for that
//!   `(kind, schema_key)` pair
//!
//! ## Edge cases
//!
//! | Condition | `lagSeconds` |
//! |-----------|-------------|
//! | No atoms in `epr_atoms` for this projection | `Some(0)` |
//! | Cursor is None (never projected) but atoms exist | `None` |
//! | Cursor exists and atoms exist | computed diff |
//! | Cursor is ahead of max atom (should never happen) | `Some(0)` |

use chrono::DateTime;
use diesel::prelude::*;
use serde::Serialize;
use ts_rs::TS;

use crate::db::diesel_schema::{epr_atoms, projector_cursor};
use crate::projector::mapping::ManifestRegistry;

// ---------------------------------------------------------------------------
// View types (API boundary — camelCase)
// ---------------------------------------------------------------------------

/// Per-(pillar, kind) projector cursor entry for the status view.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProjectorCursorView {
    pub pillar: String,
    pub kind: String,
    /// CID of the last projected EPR atom.  Null when the projector has never
    /// run for this (pillar, kind) pair.
    pub last_epr_cid: Option<String>,
    /// ISO 8601 UTC timestamp of the last projected atom's `issued_at`.
    pub last_issued_at: Option<String>,
    /// ISO 8601 UTC timestamp of when this cursor row was last updated.
    pub updated_at: Option<String>,
}

/// Per-(pillar, kind) reconciliation lag entry for the status view.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProjectorLagView {
    pub pillar: String,
    pub kind: String,
    /// `issued_at` of the most recently ingested atom for this (kind, schema_key).
    /// Null when there are no atoms at all.
    pub latest_observed_issued_at: Option<String>,
    /// `issued_at` of the last projected atom (mirrors cursor.last_issued_at).
    pub last_projected_issued_at: Option<String>,
    /// Seconds of reconciliation lag.
    ///
    /// - `Some(n)` when both sides are known (n ≥ 0, clamped at 0).
    /// - `None` when the cursor is absent but atoms exist (never projected).
    /// - `Some(0)` when there are no atoms.
    pub lag_seconds: Option<i64>,
}

/// Top-level view for `GET /api/v1/status/projector`.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ProjectorStatusView {
    pub cursors: Vec<ProjectorCursorView>,
    pub lag: Vec<ProjectorLagView>,
}

// ---------------------------------------------------------------------------
// Builder function (tested directly; HTTP handler is a thin wrapper)
// ---------------------------------------------------------------------------

/// Compute the projector status by querying `projector_cursor` and the max
/// `issued_at` from `epr_atoms` for each registered projection.
///
/// This function is `pub` so the test suite can call it directly without
/// standing up an HTTP server. The HTTP handler at
/// `GET /api/v1/status/projector` is a thin wrapper around this function.
pub fn compute_projector_status(
    conn: &mut SqliteConnection,
    registry: &ManifestRegistry,
) -> Result<ProjectorStatusView, diesel::result::Error> {
    let mut cursors = Vec::new();
    let mut lag = Vec::new();

    for proj in registry.all_projections() {
        // Load cursor row (may not exist yet).
        let cursor_row: Option<crate::projector::cursor::ProjectorCursorRow> =
            projector_cursor::table
                .find((&proj.pillar, &proj.kind))
                .first(conn)
                .optional()?;

        // Load the maximum `issued_at` in `epr_atoms` for this (kind, schema_key).
        let max_issued_at: Option<String> = epr_atoms::table
            .filter(epr_atoms::kind.eq(&proj.kind))
            .filter(epr_atoms::schema_key.eq(&proj.schema_key))
            .select(diesel::dsl::max(epr_atoms::issued_at))
            .first(conn)?;

        // Build cursor view.
        let cursor_view = ProjectorCursorView {
            pillar: proj.pillar.clone(),
            kind: proj.kind.clone(),
            last_epr_cid: cursor_row.as_ref().and_then(|c| c.last_epr_cid.clone()),
            last_issued_at: cursor_row.as_ref().and_then(|c| c.last_issued_at.clone()),
            updated_at: cursor_row.as_ref().map(|c| c.updated_at.clone()),
        };

        // Compute lag.
        let last_projected = cursor_row
            .as_ref()
            .and_then(|c| c.last_issued_at.as_deref());
        let lag_seconds = compute_lag_seconds(max_issued_at.as_deref(), last_projected);

        let lag_view = ProjectorLagView {
            pillar: proj.pillar.clone(),
            kind: proj.kind.clone(),
            latest_observed_issued_at: max_issued_at.clone(),
            last_projected_issued_at: last_projected.map(|s| s.to_string()),
            lag_seconds,
        };

        cursors.push(cursor_view);
        lag.push(lag_view);
    }

    Ok(ProjectorStatusView { cursors, lag })
}

/// Compute lag in seconds given the max observed `issued_at` and the last
/// projected `issued_at`.
///
/// | `max_observed` | `last_projected` | result |
/// |----------------|------------------|--------|
/// | None           | _                | `Some(0)` — no atoms at all |
/// | Some(_)        | None             | `None`   — atoms exist but never projected |
/// | Some(obs)      | Some(proj)       | `Some(max(0, obs - proj))` |
fn compute_lag_seconds(max_observed: Option<&str>, last_projected: Option<&str>) -> Option<i64> {
    match (max_observed, last_projected) {
        (None, _) => Some(0),
        (Some(_), None) => None,
        (Some(obs), Some(proj)) => {
            // Parse ISO-8601 timestamps; if either fails, report 0 to avoid panic.
            let obs_dt = DateTime::parse_from_rfc3339(obs).ok()?;
            let proj_dt = DateTime::parse_from_rfc3339(proj).ok()?;
            let diff = (obs_dt - proj_dt).num_seconds().max(0);
            Some(diff)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diesel_schema::epr_atoms;
    use crate::db::epr_atoms::EprAtom;
    use crate::projector::cursor::advance_cursor;
    use crate::projector::mapping::mock_manifest_registry;
    use crate::test_util::test_pool;

    fn insert_atom(conn: &mut SqliteConnection, cid: &str, issued_at: &str) {
        let payload = serde_json::json!({
            "action": "use",
            "provider": "agent:provider",
            "receiver": "agent:receiver",
            "hasPointInTime": issued_at
        });
        let payload_bytes = serde_json::to_vec(&payload).unwrap();

        let atom = EprAtom {
            cid: cid.to_string(),
            kind: "EconomicEvent".to_string(),
            schema_ref: "test-ref".to_string(),
            schema_key: "economic-event".to_string(),
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
            .expect("insert atom");
    }

    /// No atoms, no cursor: lag = 0.
    #[test]
    fn status_with_no_atoms_reports_zero_lag() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");
        let registry = mock_manifest_registry();

        let status = compute_projector_status(&mut conn, &registry).expect("compute");

        assert_eq!(status.cursors.len(), 1);
        assert_eq!(status.lag.len(), 1);

        let lag = &status.lag[0];
        assert_eq!(lag.pillar, "shefa");
        assert_eq!(lag.kind, "EconomicEvent");
        assert_eq!(lag.latest_observed_issued_at, None);
        assert_eq!(lag.lag_seconds, Some(0));
    }

    /// Atom ingested but not yet projected: cursor is None, lag is None.
    #[test]
    fn status_with_atom_but_no_cursor_reports_null_lag() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");
        let registry = mock_manifest_registry();

        insert_atom(&mut conn, "cid-unproj", "2026-04-25T12:36:00Z");

        let status = compute_projector_status(&mut conn, &registry).expect("compute");

        let lag = &status.lag[0];
        // Atom exists, cursor absent → None
        assert_eq!(
            lag.latest_observed_issued_at.as_deref(),
            Some("2026-04-25T12:36:00Z")
        );
        assert_eq!(lag.last_projected_issued_at, None);
        assert_eq!(lag.lag_seconds, None, "no cursor → lag is null");
    }

    /// Projector status endpoint reports cursor and lag (B.8 acceptance test).
    ///
    /// Ingest two atoms; project only the first. Assert the lag reflects the
    /// unprojected second atom.
    #[test]
    fn projector_status_endpoint_reports_cursor_and_lag() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");
        let registry = mock_manifest_registry();

        // Ingest two atoms, second is 64 seconds later.
        insert_atom(&mut conn, "cid-first", "2026-04-25T12:34:56Z");
        insert_atom(&mut conn, "cid-second", "2026-04-25T12:36:00Z");

        // Simulate projector having processed only the first atom.
        advance_cursor(
            &mut conn,
            "shefa",
            "EconomicEvent",
            "cid-first",
            "2026-04-25T12:34:56Z",
        )
        .expect("advance_cursor");

        let status = compute_projector_status(&mut conn, &registry).expect("compute");

        let cursor = &status.cursors[0];
        assert_eq!(cursor.last_epr_cid.as_deref(), Some("cid-first"));
        assert_eq!(
            cursor.last_issued_at.as_deref(),
            Some("2026-04-25T12:34:56Z")
        );

        let lag = &status.lag[0];
        assert_eq!(
            lag.latest_observed_issued_at.as_deref(),
            Some("2026-04-25T12:36:00Z")
        );
        assert_eq!(
            lag.last_projected_issued_at.as_deref(),
            Some("2026-04-25T12:34:56Z")
        );
        // 12:36:00 - 12:34:56 = 64 seconds
        assert_eq!(lag.lag_seconds, Some(64));
    }
}
