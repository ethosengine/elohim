//! EprService — business logic for EPR ingest, fetch, list, verify.
//!
//! See Integrator Compatibility Contract §2.2 for REST surface; this
//! service is the model/controller layer's business logic.

use diesel::prelude::*;
use elohim_epr::{
    cid::compute_cid,
    proof::verify as verify_ed25519,
    validate_coupling, Epr, EprError,
};
use serde::{Deserialize, Serialize};

use crate::db::epr_atoms::{
    self, EprAtom, EprClaimRow, EprCouplingRow, EprListQuery,
};
use crate::error::StorageError;

// ---------------------------------------------------------------------------
// Public result shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EprIngestResult {
    pub cid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EprVerifyReport {
    pub cid: String,
    pub verified: bool,
    pub stages_run: Vec<String>,
    pub stages_skipped: Vec<String>,
    pub error: Option<EprVerifyError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EprVerifyError {
    pub stage: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Ingest
// ---------------------------------------------------------------------------

/// Validate an Epr (stages 1–3) and persist to storage.
/// Stage 4 (payload schema validation) is deferred to Phase 3.
pub fn ingest(conn: &mut SqliteConnection, epr: Epr) -> Result<EprIngestResult, StorageError> {
    // Stage 1: canonicalization — recompute canonical bytes and confirm CID matches
    let canonical = epr
        .envelope
        .canonical_bytes(&epr.payload)
        .map_err(|e| StorageError::InvalidInput(format!("canonicalization: {e}")))?;
    let derived_cid = compute_cid(&canonical);
    if derived_cid.to_string() != epr.envelope.cid.to_string() {
        return Err(StorageError::InvalidInput(format!(
            "cid mismatch: derived {} vs declared {}",
            derived_cid, epr.envelope.cid
        )));
    }

    // Stage 2: signature — validate algorithm + length structurally.
    // PHASE 2a ACCEPTANCE: trust the proof bytes structurally (64 bytes ed25519).
    // Full resolver-based verify lands in Phase 3 when the manifest graph exists.
    if epr.envelope.proof.algorithm != "ed25519" {
        return Err(StorageError::InvalidInput(format!(
            "unsupported proof algorithm: {}",
            epr.envelope.proof.algorithm
        )));
    }
    if epr.envelope.proof.signature.len() != 64 {
        return Err(StorageError::InvalidInput(
            "ed25519 signature must be 64 bytes".into(),
        ));
    }

    // Stage 3: coupling — verify all kind-required legs are present
    validate_coupling(&epr.envelope)
        .map_err(|e: EprError| StorageError::InvalidInput(format!("coupling: {e}")))?;

    // Stage 4: payload schema — DEFERRED to Phase 3 (needs manifest resolver)

    // Persist
    let atom = to_atom(&epr, &canonical);
    let coupling = coupling_rows(&epr);
    let claims = claim_rows(&epr);

    conn.transaction::<_, diesel::result::Error, _>(|txn| {
        epr_atoms::insert_atom(txn, &atom)?;
        if !coupling.is_empty() {
            epr_atoms::insert_coupling_rows(txn, &coupling)?;
        }
        if !claims.is_empty() {
            epr_atoms::insert_claim_rows(txn, &claims)?;
        }
        Ok(())
    })
    .map_err(|e| StorageError::Database(e.to_string()))?;

    Ok(EprIngestResult {
        cid: epr.envelope.cid.to_string(),
    })
}

fn to_atom(epr: &Epr, canonical: &[u8]) -> EprAtom {
    EprAtom {
        cid: epr.envelope.cid.to_string(),
        kind: format!("{:?}", epr.envelope.kind),
        schema_ref: epr.envelope.schema_ref.to_string(),
        schema_key: epr.envelope.schema_key.clone(),
        reach: reach_canonical(&epr.envelope.reach),
        issued_at: epr
            .envelope
            .issued_at
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        signer_cid: epr.envelope.proof.signer.to_string(),
        supersedes: epr.envelope.supersedes.map(|c| c.to_string()),
        canonical_bytes: canonical.to_vec(),
        payload_bytes: epr.payload.clone(),
        proof_bytes: epr.envelope.proof.signature.clone(),
        proof_algorithm: epr.envelope.proof.algorithm.clone(),
    }
}

fn coupling_rows(epr: &Epr) -> Vec<EprCouplingRow> {
    let mut rows = vec![];
    let cid = epr.envelope.cid.to_string();
    if let Some(k) = epr.envelope.coupling.knowledge {
        rows.push(EprCouplingRow {
            epr_cid: cid.clone(),
            leg: "knowledge".into(),
            target_cid: k.to_string(),
        });
    }
    if let Some(v) = epr.envelope.coupling.value {
        rows.push(EprCouplingRow {
            epr_cid: cid.clone(),
            leg: "value".into(),
            target_cid: v.to_string(),
        });
    }
    if let Some(g) = epr.envelope.coupling.governance {
        rows.push(EprCouplingRow {
            epr_cid: cid,
            leg: "governance".into(),
            target_cid: g.to_string(),
        });
    }
    rows
}

fn claim_rows(epr: &Epr) -> Vec<EprClaimRow> {
    epr.envelope
        .claims
        .iter()
        .map(|c| EprClaimRow {
            epr_cid: epr.envelope.cid.to_string(),
            claim_cid: c.to_string(),
        })
        .collect()
}

fn reach_canonical(r: &elohim_epr::Reach) -> String {
    use elohim_epr::Reach::*;
    match r {
        Private => "private",
        SelfScope => "self",
        Intimate => "intimate",
        Trusted => "trusted",
        Familiar => "familiar",
        Community => "community",
        Public => "public",
        Commons => "commons",
    }
    .into()
}

// ---------------------------------------------------------------------------
// Fetch
// ---------------------------------------------------------------------------

/// All the rows necessary to reconstruct an Envelope for a REST response.
#[derive(Debug, Clone)]
pub struct FetchedEpr {
    pub atom: EprAtom,
    pub coupling: Vec<EprCouplingRow>,
    pub claims: Vec<EprClaimRow>,
    pub superseded_by: Option<String>,
}

/// Reconstruct an Epr from storage by joining all four EPR tables.
/// Returns None if the cid is not in the store.
pub fn fetch_by_cid(
    conn: &mut SqliteConnection,
    cid: &str,
) -> Result<Option<FetchedEpr>, StorageError> {
    let Some(atom) = epr_atoms::fetch_atom_by_cid(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?
    else {
        return Ok(None);
    };

    let coupling = epr_atoms::fetch_coupling_for_atom(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let claims = epr_atoms::fetch_claims_for_atom(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let superseded_by = epr_atoms::fetch_superseded_by(conn, cid)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    Ok(Some(FetchedEpr {
        atom,
        coupling,
        claims,
        superseded_by,
    }))
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

/// Paged list of atoms, filtered by kind / reach / schemaRef / cursor.
/// Returns (items, next_cursor) where next_cursor is None when the page is exhausted.
/// Fetches limit+1 rows to detect the next page without a count query.
pub fn list(
    conn: &mut SqliteConnection,
    q: &EprListQuery,
) -> Result<(Vec<EprAtom>, Option<String>), StorageError> {
    // Fetch one extra row to determine whether there's a next page
    let fetch_limit = q.limit + 1;
    let query = EprListQuery {
        limit: fetch_limit,
        kind: q.kind.clone(),
        reach: q.reach.clone(),
        schema_ref: q.schema_ref.clone(),
        after_cid: q.after_cid.clone(),
    };

    let mut rows = epr_atoms::list_atoms(conn, &query)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let next_cursor = if rows.len() as i64 > q.limit {
        rows.pop(); // discard the sentinel row
        rows.last().map(|r| r.cid.clone())
    } else {
        None
    };

    Ok((rows, next_cursor))
}

// ---------------------------------------------------------------------------
// Verify
// ---------------------------------------------------------------------------

/// Verify a stored EPR under a given public key. Runs stages 1-3 fully; stage 4
/// is reported as skipped until Phase 3 resolver exists.
pub fn verify(
    conn: &mut SqliteConnection,
    cid: &str,
    public_key: &[u8; 32],
) -> Result<EprVerifyReport, StorageError> {
    let Some(fetched) = fetch_by_cid(conn, cid)? else {
        return Err(StorageError::NotFound(format!("epr not found: {cid}")));
    };

    let mut stages_run = vec!["canonicalization".to_string()];
    let stages_skipped = vec!["payloadSchema".to_string()];

    // Stage 1: CID matches canonical bytes
    let derived = compute_cid(&fetched.atom.canonical_bytes);
    if derived.to_string() != fetched.atom.cid {
        return Ok(EprVerifyReport {
            cid: cid.into(),
            verified: false,
            stages_run,
            stages_skipped,
            error: Some(EprVerifyError {
                stage: "canonicalization".into(),
                message: format!(
                    "cid does not match canonical bytes (derived {}, stored {})",
                    derived, fetched.atom.cid
                ),
            }),
        });
    }

    // Stage 2: signature verifies under provided public key
    stages_run.push("signature".into());
    if !verify_ed25519(
        public_key,
        &fetched.atom.canonical_bytes,
        &fetched.atom.proof_bytes,
    ) {
        return Ok(EprVerifyReport {
            cid: cid.into(),
            verified: false,
            stages_run,
            stages_skipped,
            error: Some(EprVerifyError {
                stage: "signature".into(),
                message: "ed25519 signature verification failed".into(),
            }),
        });
    }

    // Stage 3: coupling requirements met
    stages_run.push("coupling".into());
    let kind = kind_from_str(&fetched.atom.kind)?;
    let has_knowledge = fetched.coupling.iter().any(|c| c.leg == "knowledge");
    let has_value = fetched.coupling.iter().any(|c| c.leg == "value");
    let has_governance = fetched.coupling.iter().any(|c| c.leg == "governance");

    use elohim_epr::CouplingLeg;
    for required in kind.required_coupling() {
        let have = match required {
            CouplingLeg::Knowledge => has_knowledge,
            CouplingLeg::Value => has_value,
            CouplingLeg::Governance => has_governance,
        };
        if !have {
            return Ok(EprVerifyReport {
                cid: cid.into(),
                verified: false,
                stages_run,
                stages_skipped,
                error: Some(EprVerifyError {
                    stage: "coupling".into(),
                    message: format!(
                        "kind {} requires coupling leg {:?}",
                        fetched.atom.kind, required
                    ),
                }),
            });
        }
    }

    // Stage 4: DEFERRED — payload schema validation needs manifest resolver (Phase 3)

    Ok(EprVerifyReport {
        cid: cid.into(),
        verified: true,
        stages_run,
        stages_skipped,
        error: None,
    })
}

fn kind_from_str(s: &str) -> Result<elohim_epr::EprKind, StorageError> {
    use elohim_epr::EprKind::*;
    match s {
        "Content" => Ok(Content),
        "Agent" => Ok(Agent),
        "Manifest" => Ok(Manifest),
        "Claim" => Ok(Claim),
        "Observation" => Ok(Observation),
        "EconomicEvent" => Ok(EconomicEvent),
        "Commitment" => Ok(Commitment),
        "Attestation" => Ok(Attestation),
        "Delegation" => Ok(Delegation),
        other => Err(StorageError::InvalidInput(format!(
            "unknown EprKind: {other}"
        ))),
    }
}
