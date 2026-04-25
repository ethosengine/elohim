//! EprService — business logic for EPR ingest, fetch, list, verify.
//!
//! See Integrator Compatibility Contract §2.2 for REST surface; this
//! service is the model/controller layer's business logic.

use chrono::Utc;
use diesel::prelude::*;
use ed25519_dalek::{Signature as EdSig, VerifyingKey};
use elohim_epr::{
    cid::compute_cid, proof::verify as verify_ed25519, validate_coupling, Epr, EprError,
};
use serde::{Deserialize, Serialize};

use crate::db::epr_atoms::{self, EprAtom, EprClaimRow, EprCouplingRow, EprListQuery};
use crate::error::StorageError;
use crate::reconcile::pubkey_timeline::PubkeyTimelineCache;

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
///
/// This variant performs structural-only signature verification (algorithm +
/// length check). For resolver-backed Ed25519 verify that stamps `verified_at`
/// and `verified_signer_fingerprint`, use [`ingest_with_cache`] instead.
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

    // Persist — no resolver-backed verify, so stamp fields are None
    let atom = to_atom(&epr, &canonical, None, None);
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

/// Validate an Epr with **resolver-backed Ed25519 verification** and persist.
///
/// Extends the structural stages of [`ingest`] with a timeline-based pubkey
/// lookup (Task A.7 — Decision #2):
///
/// 1. Structural decode + CID + algorithm + length (same as `ingest`)
/// 2. Look up the signer's pubkey in `pubkey_cache` at `envelope.issued_at`
/// 3. Verify the Ed25519 signature against that pubkey with `verify_strict`
/// 4. On success: stamp `verified_at = Utc::now()` and
///    `verified_signer_fingerprint = blake3-128-prefix(pubkey)` before persistence
/// 5. On failure: return [`StorageError::VerifyFailed`] — the atom is NOT persisted
///
/// ## Cache miss behaviour
///
/// If `pubkey_cache` has no entry for `envelope.proof.signer`, the EPR is
/// rejected with `StorageError::VerifyFailed`. Callers responsible for cache
/// warm-up (the reconcile controller) must pre-populate entries before routing
/// EPRs through this path.
///
/// ## Fingerprint format
///
/// `verified_signer_fingerprint` = first 16 bytes of `blake3::hash(pubkey_bytes)`
/// expressed as 32 lowercase hex characters. This 128-bit prefix is cheap to
/// index and sufficient for A.8's sweep targeting.
pub fn ingest_with_cache(
    conn: &mut SqliteConnection,
    pubkey_cache: &mut PubkeyTimelineCache,
    epr: Epr,
) -> Result<EprIngestResult, StorageError> {
    // Stage 1: canonicalization
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

    // Stage 2: structural check (algorithm + length)
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

    // Stage 3: coupling
    validate_coupling(&epr.envelope)
        .map_err(|e: EprError| StorageError::InvalidInput(format!("coupling: {e}")))?;

    // Stage 2B (A.7): resolver-backed Ed25519 verify
    let signer_cid_str = epr.envelope.proof.signer.to_string();
    let issued_at = epr.envelope.issued_at;

    let validity = pubkey_cache
        .get(&signer_cid_str)
        .and_then(|tl| tl.pubkey_at(issued_at))
        .ok_or_else(|| StorageError::VerifyFailed {
            reason: format!(
                "no pubkey valid at {} for signer {}",
                issued_at.to_rfc3339(),
                signer_cid_str
            ),
        })?;

    let pubkey_bytes = validity.pubkey;

    // Verify using ed25519-dalek verify_strict (rejects weak-form signatures)
    let verifying_key =
        VerifyingKey::from_bytes(&pubkey_bytes).map_err(|e| StorageError::VerifyFailed {
            reason: format!("invalid pubkey bytes for signer {signer_cid_str}: {e}"),
        })?;
    let sig = EdSig::from_slice(&epr.envelope.proof.signature).map_err(|e| {
        StorageError::VerifyFailed {
            reason: format!("invalid signature encoding: {e}"),
        }
    })?;
    verifying_key
        .verify_strict(&canonical, &sig)
        .map_err(|_| StorageError::VerifyFailed {
            reason: format!(
                "Ed25519 signature mismatch for signer {} at {}",
                signer_cid_str,
                issued_at.to_rfc3339()
            ),
        })?;

    // Compute blake3-128-prefix fingerprint (first 16 bytes = 32 hex chars)
    let hash = blake3::hash(&pubkey_bytes);
    let fingerprint: String = hash.as_bytes()[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    let verified_at_str = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // Stage 4: payload schema — DEFERRED to Phase 3 (needs manifest resolver)

    // Persist with verify stamp
    let atom = to_atom(&epr, &canonical, Some(verified_at_str), Some(fingerprint));
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

fn to_atom(
    epr: &Epr,
    canonical: &[u8],
    verified_at: Option<String>,
    verified_signer_fingerprint: Option<String>,
) -> EprAtom {
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
        verified_at,
        verified_signer_fingerprint,
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

    let mut rows =
        epr_atoms::list_atoms(conn, &query).map_err(|e| StorageError::Database(e.to_string()))?;

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

fn reach_from_str(s: &str) -> Result<elohim_epr::Reach, StorageError> {
    use elohim_epr::Reach::*;
    match s {
        "private" => Ok(Private),
        "self" => Ok(SelfScope),
        "intimate" => Ok(Intimate),
        "trusted" => Ok(Trusted),
        "familiar" => Ok(Familiar),
        "community" => Ok(Community),
        "public" => Ok(Public),
        "commons" => Ok(Commons),
        other => Err(StorageError::InvalidInput(format!(
            "unknown reach: {other}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Wire-bytes helpers (Phase 2c — libp2p federation)
// ---------------------------------------------------------------------------

/// Ciborium-serialized Epr bytes + the metadata a libp2p handler needs to apply
/// the reach gate. Wire format is `ciborium::ser::into_writer(&elohim_epr::Epr, &mut buf)`.
#[derive(Debug, Clone)]
pub struct FetchedWireBytes {
    pub wire_bytes: Vec<u8>,
    pub reach: String,
    pub signer_cid: String,
}

/// Decode ciborium-encoded `Epr` wire bytes and persist via the existing
/// `ingest()` validator chain. Idempotent on CID (Phase 2a INSERT OR IGNORE).
pub fn ingest_from_wire_bytes(
    conn: &mut SqliteConnection,
    bytes: &[u8],
) -> Result<EprIngestResult, StorageError> {
    let epr: elohim_epr::Epr = ciborium::de::from_reader(bytes)
        .map_err(|e| StorageError::InvalidInput(format!("cbor decode: {e}")))?;
    ingest(conn, epr)
}

/// Reconstruct an `elohim_epr::Epr` from stored rows, serialize to ciborium
/// CBOR, and return together with reach + signer info.
pub fn fetch_wire_bytes_by_cid(
    conn: &mut SqliteConnection,
    cid: &str,
) -> Result<Option<FetchedWireBytes>, StorageError> {
    let Some(fetched) = fetch_by_cid(conn, cid)? else {
        return Ok(None);
    };

    let epr = reconstruct_epr(&fetched)?;
    let mut wire_bytes = Vec::new();
    ciborium::ser::into_writer(&epr, &mut wire_bytes)
        .map_err(|e| StorageError::Database(format!("cbor encode: {e}")))?;

    let reach = fetched.atom.reach.clone();
    let signer_cid = fetched.atom.signer_cid.clone();

    Ok(Some(FetchedWireBytes {
        wire_bytes,
        reach,
        signer_cid,
    }))
}

fn reconstruct_epr(fetched: &FetchedEpr) -> Result<elohim_epr::Epr, StorageError> {
    use cid::Cid;
    use elohim_epr::{Coupling, Envelope, Epr, Signature};
    use std::str::FromStr;

    let parse_cid = |s: &str| -> Result<Cid, StorageError> {
        Cid::from_str(s).map_err(|e| StorageError::InvalidInput(format!("bad cid {s}: {e}")))
    };

    let cid = parse_cid(&fetched.atom.cid)?;
    let kind = kind_from_str(&fetched.atom.kind)?;
    let schema_ref = parse_cid(&fetched.atom.schema_ref)?;
    let reach = reach_from_str(&fetched.atom.reach)?;
    let signer = parse_cid(&fetched.atom.signer_cid)?;

    let issued_at = chrono::DateTime::parse_from_rfc3339(&fetched.atom.issued_at)
        .map_err(|e| StorageError::InvalidInput(format!("bad issued_at: {e}")))?
        .with_timezone(&chrono::Utc);

    let supersedes = fetched
        .atom
        .supersedes
        .as_deref()
        .map(parse_cid)
        .transpose()?;

    let claims: Result<Vec<Cid>, StorageError> = fetched
        .claims
        .iter()
        .map(|c| parse_cid(&c.claim_cid))
        .collect();
    let claims = claims?;

    let mut coupling = Coupling {
        knowledge: None,
        value: None,
        governance: None,
    };
    for row in &fetched.coupling {
        let target = parse_cid(&row.target_cid)?;
        match row.leg.as_str() {
            "knowledge" => coupling.knowledge = Some(target),
            "value" => coupling.value = Some(target),
            "governance" => coupling.governance = Some(target),
            other => {
                return Err(StorageError::Database(format!(
                    "unknown coupling leg in row: {other}"
                )))
            }
        }
    }

    let envelope = Envelope {
        cid,
        kind,
        schema_ref,
        schema_key: fetched.atom.schema_key.clone(),
        reach,
        coupling,
        claims,
        supersedes,
        superseded_by: None,
        issued_at,
        proof: Signature {
            signer,
            algorithm: fetched.atom.proof_algorithm.clone(),
            signature: fetched.atom.proof_bytes.clone(),
        },
    };

    Ok(Epr {
        envelope,
        payload: fetched.atom.payload_bytes.clone(),
    })
}

#[cfg(test)]
mod wire_bytes_tests {
    use super::*;
    use diesel::connection::SimpleConnection;
    use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};

    fn setup_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("open memory db");
        let migrations_sql =
            std::fs::read_to_string("migrations/2026-04-22-050000_add_epr_tables/up.sql")
                .expect("load epr up.sql");
        conn.batch_execute(&migrations_sql)
            .expect("apply epr migrations");
        // A.7 migration: verified_at + verified_signer_fingerprint columns
        let a7_sql =
            std::fs::read_to_string("migrations/2026-04-25-000000_verified_at_on_epr_atoms/up.sql")
                .expect("load a7 verified_at migration");
        conn.batch_execute(&a7_sql)
            .expect("apply a7 verified_at migration");
        conn
    }

    fn sample_epr() -> Epr {
        let kp = AgentKeypair::from_secret(&[77u8; 32]).unwrap();
        let signer_cid = compute_cid(b"signer-agent-placeholder");
        let schema_ref = compute_cid(b"schema-ref-placeholder");
        let gov = compute_cid(b"governance-target");
        let coupling = Coupling {
            knowledge: None,
            value: None,
            governance: Some(gov),
        };
        Epr::builder()
            .kind(EprKind::Manifest)
            .schema_ref(schema_ref)
            .schema_key("test/schema")
            .reach(Reach::Commons)
            .coupling(coupling)
            .issued_at(chrono::Utc::now())
            .payload(b"payload".to_vec())
            .sign(&kp, signer_cid)
            .expect("sign")
    }

    #[test]
    fn ingest_and_fetch_wire_bytes_round_trip() {
        let mut conn = setup_conn();
        let epr = sample_epr();

        let mut wire = Vec::new();
        ciborium::ser::into_writer(&epr, &mut wire).expect("encode");

        let ingested = ingest_from_wire_bytes(&mut conn, &wire).expect("ingest");
        assert_eq!(ingested.cid, epr.envelope.cid.to_string());

        let fetched = fetch_wire_bytes_by_cid(&mut conn, &ingested.cid)
            .expect("fetch")
            .expect("row present");
        assert_eq!(fetched.reach, "commons");
        assert_eq!(fetched.signer_cid, epr.envelope.proof.signer.to_string());

        // Decode the round-tripped wire bytes back into an Epr and confirm CID + payload match.
        let decoded: Epr = ciborium::de::from_reader(&fetched.wire_bytes[..]).expect("decode");
        assert_eq!(decoded.envelope.cid, epr.envelope.cid);
        assert_eq!(decoded.payload, epr.payload);
    }

    #[test]
    fn ingest_from_wire_bytes_rejects_malformed() {
        let mut conn = setup_conn();
        let err = ingest_from_wire_bytes(&mut conn, &[0xFF, 0xFE]).unwrap_err();
        assert!(matches!(err, StorageError::InvalidInput(_)));
    }
}

// ---------------------------------------------------------------------------
// Task A.7 — ingest_with_cache tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ingest_with_cache_tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use diesel::connection::SimpleConnection;
    use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};

    use crate::reconcile::pubkey_timeline::{PubkeyTimeline, PubkeyTimelineCache};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn t(secs: i64) -> chrono::DateTime<Utc> {
        Utc.timestamp_opt(secs, 0)
            .single()
            .expect("test timestamp must be valid")
    }

    fn setup_conn() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("open memory db");
        let sql = std::fs::read_to_string("migrations/2026-04-22-050000_add_epr_tables/up.sql")
            .expect("load epr tables sql");
        conn.batch_execute(&sql)
            .expect("apply epr tables migration");
        let a7_sql =
            std::fs::read_to_string("migrations/2026-04-25-000000_verified_at_on_epr_atoms/up.sql")
                .expect("load a7 verified_at migration");
        conn.batch_execute(&a7_sql)
            .expect("apply a7 verified_at migration");
        conn
    }

    /// Build a signed EPR using the given keypair and signer CID, with the
    /// given issued_at timestamp.
    fn signed_epr(
        kp: &AgentKeypair,
        signer_cid: cid::Cid,
        issued_at: chrono::DateTime<Utc>,
    ) -> Epr {
        let schema_ref = compute_cid(b"test-schema-ref");
        let gov = compute_cid(b"test-governance-target");
        let coupling = Coupling {
            knowledge: None,
            value: None,
            governance: Some(gov),
        };
        Epr::builder()
            .kind(EprKind::Manifest)
            .schema_ref(schema_ref)
            .schema_key("test/schema")
            .reach(Reach::Commons)
            .coupling(coupling)
            .issued_at(issued_at)
            .payload(b"test-payload".to_vec())
            .sign(kp, signer_cid)
            .expect("sign epr")
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    /// When the signer's pubkey is in the cache and covers `issued_at`, the
    /// EPR is persisted and `verified_at` + `verified_signer_fingerprint` are stamped.
    #[test]
    fn ingest_stamps_verified_at_when_signature_verifies() {
        let mut conn = setup_conn();

        let kp = AgentKeypair::from_secret(&[0xAA; 32]).unwrap();
        let pubkey = kp.public_key_bytes();
        let issued_at = t(1_000_000);

        // Pre-compute the signer CID (content-addressed from pubkey)
        let signer_cid = compute_cid(&pubkey);

        // Pre-populate the pubkey cache with a validity window covering issued_at
        let mut cache = PubkeyTimelineCache::with_capacity(8);
        let mut tl = PubkeyTimeline::new();
        tl.insert(pubkey, t(0), None, "ah-test-rotation".into());
        cache.insert(signer_cid.to_string(), tl);

        let epr = signed_epr(&kp, signer_cid, issued_at);
        let cid_str = epr.envelope.cid.to_string();

        let result = ingest_with_cache(&mut conn, &mut cache, epr)
            .expect("ingest_with_cache must succeed when signature verifies");
        assert_eq!(result.cid, cid_str);

        // Fetch the stored atom and confirm stamps are present
        let atom = epr_atoms::fetch_atom_by_cid(&mut conn, &cid_str)
            .expect("db query ok")
            .expect("atom must exist after successful ingest");

        assert!(
            atom.verified_at.is_some(),
            "verified_at must be stamped on success"
        );
        assert!(
            atom.verified_signer_fingerprint.is_some(),
            "verified_signer_fingerprint must be stamped on success"
        );

        // Fingerprint must be 32 hex chars (16 bytes / 128 bits)
        let fp = atom.verified_signer_fingerprint.unwrap();
        assert_eq!(fp.len(), 32, "fingerprint must be 32 hex chars");
        assert!(
            fp.chars().all(|c| c.is_ascii_hexdigit()),
            "fingerprint must be lowercase hex"
        );
    }

    /// When the signature uses a different key than what the cache holds, the
    /// EPR is rejected and NOT persisted.
    #[test]
    fn ingest_rejects_when_signature_does_not_verify() {
        let mut conn = setup_conn();

        // Sign with key K
        let signing_kp = AgentKeypair::from_secret(&[0xBB; 32]).unwrap();
        // Cache holds key K' (different key) for the agent
        let wrong_kp = AgentKeypair::from_secret(&[0xCC; 32]).unwrap();
        let wrong_pubkey = wrong_kp.public_key_bytes();
        let issued_at = t(2_000_000);

        let signer_cid = compute_cid(&signing_kp.public_key_bytes());

        // Cache has the wrong pubkey for this signer
        let mut cache = PubkeyTimelineCache::with_capacity(8);
        let mut tl = PubkeyTimeline::new();
        tl.insert(wrong_pubkey, t(0), None, "ah-wrong-key".into());
        cache.insert(signer_cid.to_string(), tl);

        let epr = signed_epr(&signing_kp, signer_cid, issued_at);
        let cid_str = epr.envelope.cid.to_string();

        let err = ingest_with_cache(&mut conn, &mut cache, epr)
            .expect_err("must reject when signature does not verify");
        assert!(
            matches!(err, StorageError::VerifyFailed { .. }),
            "expected VerifyFailed, got: {err:?}"
        );

        // Atom must NOT have been persisted
        let stored = epr_atoms::fetch_atom_by_cid(&mut conn, &cid_str).expect("db query ok");
        assert!(
            stored.is_none(),
            "atom must not be persisted when signature fails"
        );
    }

    /// When the cache has no entry for the signer at all, the EPR is rejected.
    #[test]
    fn ingest_rejects_when_signer_not_in_timeline() {
        let mut conn = setup_conn();

        let kp = AgentKeypair::from_secret(&[0xDD; 32]).unwrap();
        let issued_at = t(3_000_000);
        let signer_cid = compute_cid(&kp.public_key_bytes());

        // Empty cache — no entry for this signer
        let mut cache = PubkeyTimelineCache::with_capacity(8);

        let epr = signed_epr(&kp, signer_cid, issued_at);
        let cid_str = epr.envelope.cid.to_string();

        let err = ingest_with_cache(&mut conn, &mut cache, epr)
            .expect_err("must reject when signer has no timeline entry");
        assert!(
            matches!(err, StorageError::VerifyFailed { .. }),
            "expected VerifyFailed, got: {err:?}"
        );

        let stored = epr_atoms::fetch_atom_by_cid(&mut conn, &cid_str).expect("db query ok");
        assert!(
            stored.is_none(),
            "atom must not be persisted when signer not in cache"
        );
    }

    /// When `issued_at` falls outside the validity window, the EPR is rejected.
    #[test]
    fn ingest_rejects_when_issued_at_before_key_validity() {
        let mut conn = setup_conn();

        let kp = AgentKeypair::from_secret(&[0xEE; 32]).unwrap();
        let pubkey = kp.public_key_bytes();
        // Key is only valid from t(500_000) onward; EPR was issued at t(100)
        let issued_at = t(100);
        let signer_cid = compute_cid(&pubkey);

        let mut cache = PubkeyTimelineCache::with_capacity(8);
        let mut tl = PubkeyTimeline::new();
        tl.insert(pubkey, t(500_000), None, "ah-future-key".into());
        cache.insert(signer_cid.to_string(), tl);

        let epr = signed_epr(&kp, signer_cid, issued_at);
        let cid_str = epr.envelope.cid.to_string();

        let err = ingest_with_cache(&mut conn, &mut cache, epr)
            .expect_err("must reject when issued_at is before key validity window");
        assert!(
            matches!(err, StorageError::VerifyFailed { .. }),
            "expected VerifyFailed, got: {err:?}"
        );

        let stored = epr_atoms::fetch_atom_by_cid(&mut conn, &cid_str).expect("db query ok");
        assert!(stored.is_none(), "atom must not be persisted");
    }

    /// Verify that the fingerprint is the blake3-128-prefix (first 16 bytes) of
    /// the pubkey that was in the cache at `issued_at`.
    #[test]
    fn fingerprint_matches_blake3_prefix_of_signing_pubkey() {
        let mut conn = setup_conn();

        let kp = AgentKeypair::from_secret(&[0x42; 32]).unwrap();
        let pubkey = kp.public_key_bytes();
        let issued_at = t(4_000_000);
        let signer_cid = compute_cid(&pubkey);

        let mut cache = PubkeyTimelineCache::with_capacity(8);
        let mut tl = PubkeyTimeline::new();
        tl.insert(pubkey, t(0), None, "ah-fp-test".into());
        cache.insert(signer_cid.to_string(), tl);

        let epr = signed_epr(&kp, signer_cid, issued_at);
        let cid_str = epr.envelope.cid.to_string();
        ingest_with_cache(&mut conn, &mut cache, epr).expect("ingest ok");

        let atom = epr_atoms::fetch_atom_by_cid(&mut conn, &cid_str)
            .expect("db ok")
            .expect("atom present");
        let stored_fp = atom.verified_signer_fingerprint.unwrap();

        // Compute expected fingerprint independently
        let hash = blake3::hash(&pubkey);
        let expected_fp: String = hash.as_bytes()[..16]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        assert_eq!(
            stored_fp, expected_fp,
            "stored fingerprint must equal blake3-128-prefix of signing pubkey"
        );
    }
}
