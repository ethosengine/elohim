//! EprRawView projection service — the EPR-content facing's `/raw` inspector.
//!
//! Read-only projection over `epr_coupling`. Category C per p2p-design-gate —
//! zero new DHT entry types, zero new tables, zero migrations. The view is
//! assembled entirely from existing rows on every request, by mapping the
//! diesel `EprCouplingRow`s onto the DB-free `CouplingRow` mirror and calling
//! the pure folds in `elohim_facings::folds::epr_content`.
//!
//! Charter: `genesis/docs/superpowers/specs/2026-06-19-epr-content-perspective-facing-lens-design.md` §6 Slice 2.

use diesel::prelude::*;
use elohim_facings::folds::epr_content::{
    fold_coupling_legs, neighborhood_degree, reverse_part_of, CouplingRow,
};

use crate::db::epr_atoms::{self, EprCouplingRow};
use crate::error::StorageError;
use crate::views::EprRawView;

/// Map a diesel `EprCouplingRow` onto the DB-free fold mirror. The pure folds
/// cannot take a diesel row (the `elohim-facings` crate has no diesel dep — the
/// absence IS the boundary), so the adapter maps here.
fn to_fold_row(r: &EprCouplingRow) -> CouplingRow {
    CouplingRow {
        epr_cid: r.epr_cid.clone(),
        leg: r.leg.clone(),
        target_cid: r.target_cid.clone(),
    }
}

/// Project the raw inspector view for the given CID.
///
/// Returns `Ok(None)` when the focal atom is not locally held (404 path).
/// Returns `Ok(Some(view))` with the focal atom's folded leg-neighborhood.
///
/// Accepts either a CIDv1 string (primary-key lookup) or a human-readable slug
/// (`schema_key` fallback) — mirroring the nav-context projection so test
/// clients and the Angular app can address well-known atoms by `schema_key`.
pub fn project(conn: &mut SqliteConnection, cid: &str) -> Result<Option<EprRawView>, StorageError> {
    // 1. Resolve the focal atom — CID primary-key first, then a `schema_key`
    //    slug fallback (mirrors `epr_nav_context_view::project` so well-known
    //    atoms can be addressed by their schema_key). Absent → Ok(None) (404).
    let focal = match epr_atoms::fetch_atom_by_cid(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?
    {
        Some(a) => a,
        None => {
            // CIDv1 multibase strings start with one of b/B/z/Z/u/U; anything
            // else is treated as a human-readable slug and tried as schema_key.
            let looks_like_slug = !cid.starts_with('b')
                && !cid.starts_with('B')
                && !cid.starts_with('z')
                && !cid.starts_with('Z')
                && !cid.starts_with('u')
                && !cid.starts_with('U');
            if looks_like_slug {
                match epr_atoms::fetch_atom_by_schema_key(conn, cid)
                    .map_err(|e| StorageError::Database(e.to_string()))?
                {
                    Some(a) => a,
                    None => return Ok(None),
                }
            } else {
                return Ok(None);
            }
        }
    };

    // Canonical key for all subsequent queries is the stored cid field.
    let cid = &focal.cid;

    // 2. Forward couplings (focal's outbound legs) → leg pivot + neighborhood degree.
    let forward: Vec<CouplingRow> = epr_atoms::fetch_coupling_for_atom(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?
        .iter()
        .map(to_fold_row)
        .collect();

    // 3. Reverse couplings (atoms that couple TO focal) → inbound part-of set.
    let reverse: Vec<CouplingRow> = epr_atoms::fetch_reverse_coupling(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?
        .iter()
        .map(to_fold_row)
        .collect();

    Ok(Some(EprRawView {
        cid: cid.to_string(),
        coupling: fold_coupling_legs(&forward),
        neighborhood_degree: neighborhood_degree(&forward),
        reverse_part_of: reverse_part_of(&reverse),
    }))
}

// ---------------------------------------------------------------------------
// Unit tests (DB-backed, in-crate — `project` stays private)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::epr_atoms::{insert_atom, insert_coupling_rows, EprAtom};
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    /// A minimal commons-reach content atom; `cid == schema_key` so either
    /// lookup path resolves it.
    fn atom(cid: &str) -> EprAtom {
        EprAtom {
            cid: cid.to_string(),
            kind: "content-epr".into(),
            schema_ref: "epr/content/v1".into(),
            schema_key: cid.to_string(),
            reach: "commons".into(),
            issued_at: "2026-06-25T00:00:00Z".into(),
            signer_cid: "uhCAk-test-signer".into(),
            supersedes: None,
            canonical_bytes: vec![0u8; 4],
            payload_bytes: vec![0u8; 4],
            proof_bytes: vec![0u8; 64],
            proof_algorithm: "ed25519".into(),
            verified_at: None,
            verified_signer_fingerprint: None,
        }
    }

    fn coupling(epr_cid: &str, leg: &str, target_cid: &str) -> EprCouplingRow {
        EprCouplingRow {
            epr_cid: epr_cid.to_string(),
            leg: leg.to_string(),
            target_cid: target_cid.to_string(),
        }
    }

    #[test]
    fn project_folds_legs_degree_and_reverse() {
        let mut conn = setup();
        insert_atom(&mut conn, &atom("focal")).unwrap();
        insert_atom(&mut conn, &atom("citing")).unwrap();
        insert_coupling_rows(
            &mut conn,
            &[
                // forward: focal's outbound legs (two distinct targets)
                coupling("focal", "knowledge", "k-cid"),
                coupling("focal", "value", "v-cid"),
                // reverse: `citing` couples TO focal → focal's inbound part-of
                coupling("citing", "governance", "focal"),
            ],
        )
        .unwrap();

        let view = project(&mut conn, "focal")
            .expect("project ok")
            .expect("focal atom found");

        assert_eq!(view.cid, "focal");
        assert_eq!(view.coupling.knowledge.as_deref(), Some("k-cid"));
        assert_eq!(view.coupling.value.as_deref(), Some("v-cid"));
        assert_eq!(view.coupling.governance, None, "governance leg uncoupled");
        assert_eq!(
            view.neighborhood_degree, 2,
            "two distinct forward targets (k-cid, v-cid)"
        );
        assert_eq!(
            view.reverse_part_of,
            vec!["citing".to_string()],
            "the inbound source atom that couples to focal"
        );
    }

    #[test]
    fn project_absent_atom_is_none() {
        let mut conn = setup();
        assert!(
            project(&mut conn, "no-such-atom")
                .expect("project ok")
                .is_none(),
            "absent atom must project None (404 path)"
        );
    }
}
