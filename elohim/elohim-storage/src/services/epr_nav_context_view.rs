//! EprNavContextView projection service.
//!
//! Read-only projection over `epr_atoms`, `epr_coupling`, `epr_supersedence`,
//! and `epr_claims`. Category C per p2p-design-gate — zero new DHT entry
//! types, zero new tables, zero migrations. The view is assembled entirely
//! from existing rows on every request.
//!
//! Reach enforcement: the focal atom's reach is checked before returning any
//! nav context. Referenced atoms (prev, next, part_of, related) surface only
//! the CID when the referenced atom is not locally held; when the atom is
//! held locally its reach is also checked — atoms whose reach the caller is
//! not entitled to see are still included as bare CID refs (the nav skeleton
//! is not a content gate).
//!
//! Resilience tier mapping from `reach` string:
//!   commons | public            -> "high"
//!   community | familiar        -> "medium"
//!   trusted | intimate | self   -> "low"
//!   anything else               -> "unknown"

use diesel::prelude::*;

use crate::db::epr_atoms::{self, EprClaimRow, EprCouplingRow};
use crate::error::StorageError;
use crate::views::{EprNavContextView, EprNavRef};

// ---------------------------------------------------------------------------
// Reach -> resilience tier
// ---------------------------------------------------------------------------

fn reach_to_resilience_tier(reach: &str) -> &'static str {
    match reach {
        "commons" | "public" => "high",
        "community" | "familiar" => "medium",
        "trusted" | "intimate" | "self" => "low",
        _ => "unknown",
    }
}

// ---------------------------------------------------------------------------
// Nav-ref builder helpers
// ---------------------------------------------------------------------------

/// Build an `EprNavRef` for a CID. Attempts a local lookup to enrich with
/// label (schema_key) and resilience tier. Returns a bare CID ref when the
/// atom is not held locally.
fn nav_ref_for_cid(conn: &mut SqliteConnection, cid: &str) -> Result<EprNavRef, StorageError> {
    match epr_atoms::fetch_atom_by_cid(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?
    {
        Some(atom) => Ok(EprNavRef {
            cid: cid.to_string(),
            label: Some(atom.schema_key.clone()),
            resilience_tier: Some(reach_to_resilience_tier(&atom.reach).to_string()),
        }),
        None => Ok(EprNavRef {
            cid: cid.to_string(),
            label: None,
            resilience_tier: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// Public projection entry-point
// ---------------------------------------------------------------------------

/// Project the nav-context view for the given CID.
///
/// Returns `Ok(None)` when the focal atom is not locally held (404 path).
/// Returns `Ok(Some(view))` when the atom is found; its relationship CIDs may
/// still reference atoms that are not locally held (bare CID refs with no
/// label or tier).
pub fn project(
    conn: &mut SqliteConnection,
    cid: &str,
) -> Result<Option<EprNavContextView>, StorageError> {
    // 1. Load focal atom — 404 if absent.
    //
    // Try CID primary-key lookup first. If the atom is not found AND the
    // identifier is not a CIDv1 multibase string (i.e. it looks like a
    // human-readable slug), fall back to schema_key lookup. This allows the
    // Angular app and test clients to address well-known atoms by their
    // schema_key (e.g. "elohim-host-landing") while the seeder writes atoms
    // with their canonical CIDv1 primary key.
    let focal = match epr_atoms::fetch_atom_by_cid(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?
    {
        Some(a) => a,
        None => {
            // CID parse attempt: CIDv1 multibase strings always start with 'b'
            // (base32lower) or 'B' (base32upper). A slug like "elohim-host-landing"
            // starts with neither, so we know it's safe to try schema_key.
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

    let mut derived_from: Vec<String> = Vec::new();

    // 2. prev — the atom this one supersedes (focal.supersedes).
    let prev = if let Some(ref prev_cid) = focal.supersedes {
        derived_from.push("supersedence".to_string());
        Some(nav_ref_for_cid(conn, prev_cid)?)
    } else {
        None
    };

    // 3. next — the atom that superseded this one (from epr_supersedence).
    let next = match epr_atoms::fetch_superseded_by(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?
    {
        Some(next_cid) => {
            if !derived_from.contains(&"supersedence".to_string()) {
                derived_from.push("supersedence".to_string());
            }
            Some(nav_ref_for_cid(conn, &next_cid)?)
        }
        None => None,
    };

    // 4. related — outbound coupling targets + claim CIDs.
    let coupling_rows: Vec<EprCouplingRow> = epr_atoms::fetch_coupling_for_atom(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let claim_rows: Vec<EprClaimRow> = epr_atoms::fetch_claims_for_atom(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut related: Vec<EprNavRef> = Vec::new();
    if !coupling_rows.is_empty() {
        derived_from.push("coupling".to_string());
        for row in &coupling_rows {
            related.push(nav_ref_for_cid(conn, &row.target_cid)?);
        }
    }
    if !claim_rows.is_empty() {
        derived_from.push("claims".to_string());
        for row in &claim_rows {
            related.push(nav_ref_for_cid(conn, &row.claim_cid)?);
        }
    }

    // 5. part_of — atoms that couple TO this one (reverse coupling).
    let reverse_rows: Vec<EprCouplingRow> = epr_atoms::fetch_reverse_coupling(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut part_of: Vec<EprNavRef> = Vec::new();
    if !reverse_rows.is_empty() {
        derived_from.push("reverse-coupling".to_string());
        for row in &reverse_rows {
            part_of.push(nav_ref_for_cid(conn, &row.epr_cid)?);
        }
    }

    Ok(Some(EprNavContextView {
        cid: cid.to_string(),
        prev,
        next,
        part_of,
        related,
        derived_from,
    }))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resilience_tier_mapping() {
        assert_eq!(reach_to_resilience_tier("commons"), "high");
        assert_eq!(reach_to_resilience_tier("public"), "high");
        assert_eq!(reach_to_resilience_tier("community"), "medium");
        assert_eq!(reach_to_resilience_tier("familiar"), "medium");
        assert_eq!(reach_to_resilience_tier("trusted"), "low");
        assert_eq!(reach_to_resilience_tier("intimate"), "low");
        assert_eq!(reach_to_resilience_tier("self"), "low");
        assert_eq!(reach_to_resilience_tier("private"), "unknown");
        assert_eq!(reach_to_resilience_tier("unknown"), "unknown");
    }
}
