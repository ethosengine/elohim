//! REA economic facing — the per-commitment read surface over the mishpat
//! compute/recovery-class commitment ledger (Wave 4.2).
//!
//! Mirrors `operational_weave_facing.rs`: an impure loader (`load_mishpat_commitments`,
//! takes `&mut conn`) + a `build_mishpat_commitment_view` that projects each
//! `MishpatCommitment` DB row into a typed `MishpatCommitmentView` for the wire.
//!
//! ## Why the row, not the fold's `CommitmentRow`
//! The 4.1 folds (`elohim_facings::folds::rea`) operate on `CommitmentRow` — a
//! DB-free mirror carrying `resource_classified_as` + `household_id` (fold INPUTS
//! for `commitment_backed` / `by_action` / `mutual_compute`). Those folds drive the
//! AGGREGATE lens (`ReaFacingView`, a sibling deliverable). This per-commitment view
//! instead needs `dht_anchor_hash` + `created_at` — fields `CommitmentRow` does NOT
//! carry but the `mishpat_commitments` row does. So the view maps from the DB row.
//!
//! ## Cardinality
//! `build_weave_view` returns ONE aggregate; this returns a LIST (one
//! `MishpatCommitmentView` per row). That difference is correct — the weave is a
//! cluster fold, this is a row projection.
//!
//! Charter: genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md
//! (§"Typed VIEW + HTTP surface").

use diesel::sqlite::SqliteConnection;
use elohim_views::shared::parse_json_opt;
use elohim_views::MishpatCommitmentView;

use crate::db::models::MishpatCommitment;

/// Project one `mishpat_commitments` DB row into its typed wire view.
///
/// `bounds` is PARSED from the `bounds_json` column. It is `Some` ONLY when the
/// column parses to a JSON OBJECT — the schema declares `bounds` as `type:object`,
/// so a non-object value (a scalar, an array, or an unparseable string) yields
/// `None`, OMITTING the field per the not-selected contract rather than emitting a
/// schema-violating non-object. (Real envelopes are always object-shaped; this
/// guard keeps a malformed row from breaking the wire contract.)
/// `dht_anchor_hash` carries through as `Option<String>` — a NULL anchor (the
/// un-notarized / local-stack-gap row) omits the field rather than sending `null`.
pub(crate) fn to_view(row: MishpatCommitment) -> MishpatCommitmentView {
    let bounds = parse_json_opt(&Some(row.bounds_json)).filter(|v| v.0.is_object());
    MishpatCommitmentView {
        cid: row.cid,
        action: row.action,
        scope: row.scope,
        provider: row.provider,
        recipient: row.recipient,
        bounds,
        valid_from: row.valid_from,
        valid_until: row.valid_until,
        state: row.state,
        dht_anchor_hash: row.dht_anchor_hash,
        created_at: row.created_at,
    }
}

/// Load ALL mishpat commitment rows (node-scoped, viewer-less) from the DB.
///
/// A query error degrades to an empty `Vec` (warn-and-continue), exactly like the
/// operational-weave loaders. The underlying table reads EMPTY until the
/// `CommitmentCommitted` post-commit signal projects rows — the honest
/// correct-but-dormant projection.
pub fn load_mishpat_commitments(conn: &mut SqliteConnection) -> Vec<MishpatCommitment> {
    match crate::db::mishpat_commitments::list_all(conn) {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("load_mishpat_commitments: query failed: {e}");
            Vec::new()
        }
    }
}

/// Build the full list of [`MishpatCommitmentView`]s from the current DB state.
///
/// The REA economic facing's per-commitment surface — read-only over the existing
/// notarized commitment ledger (Operational Category C; no new DHT entry type).
pub fn build_mishpat_commitment_view(conn: &mut SqliteConnection) -> Vec<MishpatCommitmentView> {
    load_mishpat_commitments(conn)
        .into_iter()
        .map(to_view)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewMishpatCommitment;
    use diesel::connection::SimpleConnection;
    use diesel::Connection;

    /// In-memory SQLite with just the `mishpat_commitments` table created from the
    /// diesel-schema column shape (avoids pulling the full migration set into a unit
    /// test — the focused-fixture pattern).
    fn conn_with_table() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("open in-memory sqlite");
        conn.batch_execute(
            "CREATE TABLE mishpat_commitments (
                cid TEXT PRIMARY KEY NOT NULL,
                action TEXT NOT NULL,
                scope TEXT NOT NULL,
                provider TEXT NOT NULL,
                recipient TEXT NOT NULL,
                bounds_json TEXT NOT NULL,
                valid_from TEXT NOT NULL,
                valid_until TEXT NOT NULL,
                revoked_at TEXT,
                state TEXT NOT NULL DEFAULT 'proposed',
                dht_anchor_hash TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .expect("create table");
        conn
    }

    fn seed(conn: &mut SqliteConnection, cid: &str, bounds_json: &str, anchor: Option<&str>) {
        crate::db::mishpat_commitments::upsert_with_anchor(
            conn,
            NewMishpatCommitment {
                cid: cid.to_string(),
                action: "delegates-compute".to_string(),
                scope: "republish-epr".to_string(),
                provider: "uhCAk-p".to_string(),
                recipient: "uhCAk-r".to_string(),
                bounds_json: bounds_json.to_string(),
                valid_from: "2026-05-28T00:00:00Z".to_string(),
                valid_until: "2026-08-26T00:00:00Z".to_string(),
                revoked_at: None,
                state: "active".to_string(),
                dht_anchor_hash: anchor.map(|s| s.to_string()),
            },
        )
        .expect("seed commitment");
    }

    #[test]
    fn empty_table_builds_empty_list() {
        let mut conn = conn_with_table();
        assert!(build_mishpat_commitment_view(&mut conn).is_empty());
    }

    #[test]
    fn projects_rows_parsing_bounds_and_preserving_anchor() {
        let mut conn = conn_with_table();
        // notarized row with a real bounds envelope
        seed(
            &mut conn,
            "cid-anchored",
            r#"{"maxCallsPerHour":100}"#,
            Some("uhCkk-anchor"),
        );
        // un-notarized row (NULL anchor) + non-object bounds → bounds parses to a
        // JSON value but to_view still maps it; anchor omitted on the wire
        seed(&mut conn, "cid-bare", r#"{}"#, None);

        let views = build_mishpat_commitment_view(&mut conn);
        assert_eq!(views.len(), 2);

        let anchored = views.iter().find(|v| v.cid == "cid-anchored").unwrap();
        assert_eq!(anchored.dht_anchor_hash.as_deref(), Some("uhCkk-anchor"));
        assert!(anchored.bounds.is_some(), "bounds_json parsed to Some");
        assert_eq!(anchored.action, "delegates-compute");
        assert_eq!(anchored.state, "active");

        let bare = views.iter().find(|v| v.cid == "cid-bare").unwrap();
        assert!(
            bare.dht_anchor_hash.is_none(),
            "NULL anchor → None (field omitted on the wire)"
        );
    }

    #[test]
    fn unparseable_or_non_object_bounds_yields_none() {
        let mut conn = conn_with_table();
        // unparseable → None
        seed(&mut conn, "cid-bad", "not json", None);
        // parseable BUT a JSON array (non-object) → None (schema says type:object)
        seed(&mut conn, "cid-arr", r#"[1,2,3]"#, None);
        // parseable BUT a scalar (non-object) → None
        seed(&mut conn, "cid-scalar", r#"42"#, None);
        let views = build_mishpat_commitment_view(&mut conn);
        assert_eq!(views.len(), 3);
        for v in &views {
            assert!(
                v.bounds.is_none(),
                "non-object bounds_json ({}) → None (omitted), never a schema-violating non-object",
                v.cid
            );
        }
    }
}
