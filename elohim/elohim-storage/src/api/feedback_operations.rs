//! The submission outbox — `POST /api/v1/feedback/operations` (contract §8).
//!
//! **The problem this solves.** A zome call that times out has no answer: the
//! act may be on chain or it may not. Retrying blindly mints a second act;
//! never retrying loses the intent. §8's answer is a durable, client-named
//! operation plus a two-phase submission whose FIRST phase is content-addressed
//! by the operation id, so recovery is a lookup rather than a guess.
//!
//! **Phase 1 — author the evidence.** A Correction EPR is written with Content
//! id `correction:<operation_id>` and the immutable request embedded in its
//! body. Recovery after an uncertain call resolves that id through the
//! ID-anchor links; a double-mint is settled by the LOWEST action hash,
//! deterministically, BEFORE phase 2 begins.
//!
//! **Phase 2 — file the feedback.** `create_feedback_signal` cites the phase-1
//! action. The evidence action hash IS the operation's group key on chain
//! (§7), so a concurrent double-mint of phase 2 produces two GROUP MEMBERS, not
//! two contributions.
//!
//! **What this does NOT claim.** An SQL outbox plus two-phase recovery is not
//! an exactly-once remote commit protocol. It guarantees at most one
//! CONTRIBUTION per operation — not at most one action. Zero matches after an
//! uncertain call is `unresolved` and is NEVER auto-resubmitted: absence from
//! an eventually-consistent index does not authorise a second create.

use std::sync::Arc;

use bytes::Bytes;
use chrono::Utc;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db::feedback_operations::{
    self as ops_db, OperationRow, PHASE_DONE, PHASE_EVIDENCE, PHASE_FEEDBACK, STATUS_PENDING,
    STATUS_RESOLVED, STATUS_UNRESOLVED,
};
use crate::db::DbPool;
use crate::error::StorageError;
use crate::hc_client::HcClient;

const ZOME: &str = "content_store";
/// The Content id phase 1 writes under. Content-addressed BY THE OPERATION —
/// that is what makes phase-1 recovery a lookup instead of a guess.
pub const EVIDENCE_ID_PREFIX: &str = "correction:";

// ---------------------------------------------------------------------------
// Wire shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOperationRequest {
    /// Client-minted UUID. Reuse with DIFFERENT request bytes is REFUSED: the
    /// request is immutable, and a changed impact would change the very
    /// contribution the group applies.
    pub operation_id: String,
    pub target_action_hash: String,
    pub signal_kind: String,
    pub standing_impact: String,
    #[serde(default)]
    pub vouch_kind: Option<String>,
    /// Human-readable body of the Correction EPR. Not part of the immutable
    /// request tuple — the request is what the act is BOUND to, not its prose.
    #[serde(default)]
    pub body: Option<String>,
}

/// The canonical, IMMUTABLE request an operation pins.
///
/// This exact object is embedded in the evidence Content's `metadata_json`
/// under `correctionRequest`, where the coordinator's admission gate reads it
/// back and matches it against the act's own fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionRequest {
    pub operation_id: String,
    pub target_action_hash: String,
    pub signal_kind: String,
    pub standing_impact: String,
}

impl CorrectionRequest {
    pub fn canonical_bytes(&self) -> String {
        // Field order is the struct's declaration order and the struct is the
        // schema — the same bytes on both sides of a restart.
        serde_json::to_string(self).unwrap_or_default()
    }
    pub fn digest(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.canonical_bytes().as_bytes());
        format!("sha256:{}", hex::encode(h.finalize()))
    }
    pub fn evidence_content_id(&self) -> String {
        format!("{EVIDENCE_ID_PREFIX}{}", self.operation_id)
    }
    pub fn metadata_json(&self) -> String {
        serde_json::json!({ "correctionRequest": self }).to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationView {
    pub operation_id: String,
    pub origin_dna_hash: String,
    pub submitting_cell_agent: String,
    pub request_bytes_cid: String,
    pub phase: String,
    pub status: String,
    pub evidence_action_hash: Option<String>,
    pub feedback_action_hash: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<OperationRow> for OperationView {
    fn from(r: OperationRow) -> Self {
        Self {
            operation_id: r.operation_id,
            origin_dna_hash: r.origin_dna_hash,
            submitting_cell_agent: r.submitting_cell_agent,
            request_bytes_cid: r.request_bytes_cid,
            phase: r.phase,
            status: r.status,
            evidence_action_hash: r.evidence_action_hash,
            feedback_action_hash: r.feedback_action_hash,
            last_error: r.last_error,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Routing
// ---------------------------------------------------------------------------

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    path: &str,
    pool: &DbPool,
    hc: Option<&Arc<HcClient>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    match (&method, path) {
        (&Method::POST, "/api/v1/feedback/operations") => create(req, pool, hc).await,
        (&Method::GET, p) if p.starts_with("/api/v1/feedback/operations/") => {
            let id = p.trim_start_matches("/api/v1/feedback/operations/");
            get_one(id, pool)
        }
        _ => Ok(json_status(
            StatusCode::METHOD_NOT_ALLOWED,
            &serde_json::json!({"error": "unsupported method or path"}),
        )),
    }
}

fn get_one(operation_id: &str, pool: &DbPool) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Database(e.to_string()))?;
    match ops_db::fetch(&mut conn, operation_id)
        .map_err(|e| StorageError::Database(e.to_string()))?
    {
        Some(row) => ok_json(&OperationView::from(row)),
        None => Ok(json_status(
            StatusCode::NOT_FOUND,
            &serde_json::json!({"error": "no such operation"}),
        )),
    }
}

async fn create(
    req: Request<Incoming>,
    pool: &DbPool,
    hc: Option<&Arc<HcClient>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body = req
        .into_body()
        .collect()
        .await
        .map_err(|e| StorageError::InvalidInput(format!("read body: {e}")))?
        .to_bytes();
    let request: CreateOperationRequest = serde_json::from_slice(&body)
        .map_err(|e| StorageError::InvalidInput(format!("parse body: {e}")))?;

    let Some(hc) = hc else {
        return Ok(json_status(
            StatusCode::SERVICE_UNAVAILABLE,
            &serde_json::json!({"error": "no content cell on this node — cannot submit"}),
        ));
    };

    if request.operation_id.trim().is_empty() {
        return Ok(json_status(
            StatusCode::BAD_REQUEST,
            &serde_json::json!({"error": "operationId is required"}),
        ));
    }

    let pinned = CorrectionRequest {
        operation_id: request.operation_id.clone(),
        target_action_hash: request.target_action_hash.clone(),
        signal_kind: request.signal_kind.clone(),
        standing_impact: request.standing_impact.clone(),
    };
    let digest = pinned.digest();
    let now = Utc::now().to_rfc3339();
    let origin_dna_hash = hc.cell_id().dna_hash().to_string();
    let submitting_cell_agent = hc.agent_key_uhcak();

    // ------------------------------------------------------------------
    // Claim the operation. Reuse with different bytes is REFUSED; reuse with
    // the SAME bytes is the documented recovery path and continues from
    // whatever phase the row is in.
    // ------------------------------------------------------------------
    let existing = {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        let row = OperationRow {
            operation_id: pinned.operation_id.clone(),
            origin_dna_hash: origin_dna_hash.clone(),
            submitting_cell_agent: submitting_cell_agent.clone(),
            request_bytes_cid: digest.clone(),
            request_bytes: pinned.canonical_bytes(),
            target_action_hash: pinned.target_action_hash.clone(),
            signal_kind: pinned.signal_kind.clone(),
            standing_impact: pinned.standing_impact.clone(),
            vouch_kind: request.vouch_kind.clone(),
            phase: PHASE_EVIDENCE.to_string(),
            evidence_action_hash: None,
            feedback_action_hash: None,
            status: STATUS_PENDING.to_string(),
            last_error: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        if ops_db::insert_new(&mut conn, &row).map_err(|e| StorageError::Database(e.to_string()))? {
            row
        } else {
            let found = ops_db::fetch(&mut conn, &pinned.operation_id)
                .map_err(|e| StorageError::Database(e.to_string()))?
                .ok_or_else(|| {
                    StorageError::Internal("operation vanished between insert and read".into())
                })?;
            if found.request_bytes_cid != digest {
                return Ok(json_status(
                    StatusCode::CONFLICT,
                    &serde_json::json!({
                        "error": "operationId reused with DIFFERENT request bytes — the request \
                                  an operation pins is immutable",
                        "operationId": pinned.operation_id,
                        "pinnedRequestBytesCid": found.request_bytes_cid,
                        "submittedRequestBytesCid": digest,
                    }),
                ));
            }
            if found.status == STATUS_RESOLVED {
                // Already done. Idempotent replay of a completed operation.
                return ok_json(&OperationView::from(found));
            }
            found
        }
    };

    // SINGLE-FLIGHT: only one caller may drive an operation at a time. The
    // loser reports the operation's current state rather than issuing a second
    // zome call.
    {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;
        let claimed = ops_db::claim_for_phase(
            &mut conn,
            &existing.operation_id,
            &existing.status,
            &existing.phase,
            "in-flight",
            &now,
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;
        if !claimed {
            let found = ops_db::fetch(&mut conn, &existing.operation_id)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            return match found {
                Some(row) => Ok(json_status(
                    StatusCode::ACCEPTED,
                    &serde_json::to_value(OperationView::from(row)).unwrap_or_default(),
                )),
                None => Ok(json_status(
                    StatusCode::CONFLICT,
                    &serde_json::json!({"error": "operation is in flight"}),
                )),
            };
        }
    }

    let mut row = existing;

    // ------------------------------------------------------------------
    // PHASE 1 — author (or recover) the evidence.
    // ------------------------------------------------------------------
    if row.evidence_action_hash.is_none() {
        match author_or_recover_evidence(hc, &pinned, request.body.as_deref()).await {
            Ok(Some(action_hash)) => {
                row.evidence_action_hash = Some(action_hash);
                row.phase = PHASE_FEEDBACK.to_string();
                row.status = STATUS_PENDING.to_string();
                row.last_error = None;
            }
            Ok(None) => {
                // Uncertain phase 1 with ZERO resolvable evidence. This is
                // `unresolved`, not a licence to author again.
                row.status = STATUS_UNRESOLVED.to_string();
                row.last_error = Some(
                    "phase 1 could not be resolved: no evidence action found for this \
                     operation id. NOT auto-retried — resubmit explicitly under the same \
                     operation id."
                        .to_string(),
                );
                return persist_and_return(pool, &mut row, StatusCode::ACCEPTED);
            }
            Err(e) => {
                row.status = STATUS_PENDING.to_string();
                row.last_error = Some(format!("phase 1 failed: {e}"));
                return persist_and_return(pool, &mut row, StatusCode::BAD_GATEWAY);
            }
        }
    }

    // ------------------------------------------------------------------
    // PHASE 2 — file the feedback against the phase-1 action.
    // ------------------------------------------------------------------
    let evidence = row
        .evidence_action_hash
        .clone()
        .expect("phase 1 set the evidence action");
    match file_feedback(hc, &pinned, &evidence).await {
        Ok(action_hash) => {
            row.feedback_action_hash = Some(action_hash);
            row.phase = PHASE_DONE.to_string();
            row.status = STATUS_RESOLVED.to_string();
            row.last_error = None;
            persist_and_return(pool, &mut row, StatusCode::CREATED)
        }
        Err(e) => {
            // An uncertain phase 2 is recovered by enumerating THIS cell's own
            // acts and matching the operation tuple. Several matches are ONE
            // group (§7), so a concurrent double-mint changes nothing served.
            match recover_feedback(hc, &pinned, &evidence).await {
                Ok(Some(action_hash)) => {
                    row.feedback_action_hash = Some(action_hash);
                    row.phase = PHASE_DONE.to_string();
                    row.status = STATUS_RESOLVED.to_string();
                    row.last_error = None;
                    persist_and_return(pool, &mut row, StatusCode::CREATED)
                }
                Ok(None) => {
                    row.status = STATUS_UNRESOLVED.to_string();
                    row.last_error = Some(format!(
                        "phase 2 uncertain and recovery found ZERO matches: {e}. NOT \
                         auto-resubmitted — absence from an eventually-consistent index does \
                         not authorise a second create."
                    ));
                    persist_and_return(pool, &mut row, StatusCode::ACCEPTED)
                }
                Err(recovery_err) => {
                    row.status = STATUS_PENDING.to_string();
                    row.last_error = Some(format!(
                        "phase 2 failed: {e}; recovery failed: {recovery_err}"
                    ));
                    persist_and_return(pool, &mut row, StatusCode::BAD_GATEWAY)
                }
            }
        }
    }
}

fn persist_and_return(
    pool: &DbPool,
    row: &mut OperationRow,
    status: StatusCode,
) -> Result<Response<Full<Bytes>>, StorageError> {
    row.updated_at = Utc::now().to_rfc3339();
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Database(e.to_string()))?;
    ops_db::update(&mut conn, row).map_err(|e| StorageError::Database(e.to_string()))?;
    Ok(json_status(
        status,
        &serde_json::to_value(OperationView::from(row.clone())).unwrap_or_default(),
    ))
}

// ---------------------------------------------------------------------------
// Conductor round trips
// ---------------------------------------------------------------------------

/// Phase 1. Author the Correction EPR, or RECOVER the one a lost response
/// already wrote.
///
/// `Ok(None)` = the authoring call failed AND no evidence exists for this
/// operation id: genuinely unresolved.
async fn author_or_recover_evidence(
    hc: &Arc<HcClient>,
    pinned: &CorrectionRequest,
    body: Option<&str>,
) -> Result<Option<String>, StorageError> {
    // Recover FIRST. An operation id is content-addressed by construction, so
    // "did I already write this?" is a lookup, and asking it before writing
    // avoids minting a second root on every retry.
    if let Some(existing) = resolve_evidence_action(hc, pinned).await? {
        return Ok(Some(existing));
    }

    let input = lamad_types::CreateContentInput {
        id: pinned.evidence_content_id(),
        content_type: "correction".to_string(),
        title: format!("Correction: {}", pinned.target_action_hash),
        description: "Correction EPR — evidence for an accountable correction".to_string(),
        content: body.unwrap_or("").to_string(),
        content_format: "markdown".to_string(),
        // PUBLIC by requirement (§8): a correction whose evidence its addressee
        // cannot read is not accountable, and the coordinator refuses it.
        reach: "public".to_string(),
        metadata_json: pinned.metadata_json(),
        summary: None,
        tags: vec!["correction".to_string()],
        source_path: None,
        related_node_ids: vec![],
        estimated_minutes: None,
        thumbnail_url: None,
        blob_cid: None,
        content_size_bytes: None,
        content_hash: None,
    };
    match crate::services::conductor_writes::call_create_content(hc, &input).await {
        Ok(bytes) => {
            let out: lamad_types::ContentOutput = rmp_serde::from_slice(&bytes)
                .map_err(|e| StorageError::Serialization(format!("decode ContentOutput: {e}")))?;
            Ok(Some(out.action_hash.to_string()))
        }
        Err(_) => {
            // UNCERTAIN. Re-resolve: the write may have landed.
            resolve_evidence_action(hc, pinned).await
        }
    }
}

/// Resolve `correction:<operation_id>` to ONE evidence action, deterministically.
///
/// A double-mint (two roots under one id) is settled by the LOWEST action hash
/// — chosen BEFORE phase 2 begins, so every recovery of this operation picks
/// the same evidence and therefore the same group key.
async fn resolve_evidence_action(
    hc: &Arc<HcClient>,
    pinned: &CorrectionRequest,
) -> Result<Option<String>, StorageError> {
    let content_id = pinned.evidence_content_id();
    let payload = rmp_serde::to_vec_named(&serde_json::json!({ "id": content_id }))
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let bytes = match hc.call_zome(ZOME, "get_content_by_id", payload).await {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };
    let found: Option<lamad_types::ContentOutput> = rmp_serde::from_slice(&bytes)
        .map_err(|e| StorageError::Serialization(format!("decode get_content_by_id: {e}")))?;
    let Some(found) = found else {
        return Ok(None);
    };

    // One action is enough to reach the lineage, which enumerates EVERY
    // IdToContent link for the id — including candidates under a different root
    // (a double-mint), which is exactly what the lowest-hash rule has to see.
    let lineage_payload = rmp_serde::to_vec_named(&serde_json::json!({
        "action_hash": found.action_hash,
        "local": true,
    }))
    .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let Ok(lineage_bytes) = hc
        .call_zome(ZOME, "get_content_lineage", lineage_payload)
        .await
    else {
        return Ok(Some(found.action_hash.to_string()));
    };
    let lineage: crate::services::feedback_projector::ContentLineage =
        match rmp_serde::from_slice(&lineage_bytes) {
            Ok(l) => l,
            Err(_) => return Ok(Some(found.action_hash.to_string())),
        };
    // Root Creates only: a root names no predecessor.
    let mut roots: Vec<String> = lineage
        .candidates
        .iter()
        .filter(|c| c.predecessor.is_none() && c.fetch_outcome != "not-found")
        .map(|c| c.action_hash.clone())
        .collect();
    if roots.is_empty() {
        return Ok(Some(found.action_hash.to_string()));
    }
    roots.sort();
    Ok(roots.into_iter().next())
}

/// Phase 2. File the FeedbackSignal citing the phase-1 evidence action.
async fn file_feedback(
    hc: &Arc<HcClient>,
    pinned: &CorrectionRequest,
    evidence_action_hash: &str,
) -> Result<String, StorageError> {
    let payload = rmp_serde::to_vec_named(&serde_json::json!({
        "target_action_hash": pinned.target_action_hash,
        "signal_kind": pinned.signal_kind,
        "evidence_action_hash": evidence_action_hash,
        "standing_impact": pinned.standing_impact,
    }))
    .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let bytes = hc
        .call_zome(ZOME, "create_feedback_signal", payload)
        .await?;
    let action_hash: holochain_types::prelude::ActionHash = rmp_serde::from_slice(&bytes)
        .map_err(|e| StorageError::Serialization(format!("decode ActionHash: {e}")))?;
    Ok(action_hash.to_string())
}

/// Phase-2 recovery. Enumerate THIS cell's own acts and match the operation
/// tuple `(submitting_cell_agent, target, evidence action, signal_kind)`.
///
/// Several matches are ONE GROUP (§7), so the lowest action hash is returned
/// purely for determinism — it is a representative, never a second contribution.
async fn recover_feedback(
    hc: &Arc<HcClient>,
    pinned: &CorrectionRequest,
    evidence_action_hash: &str,
) -> Result<Option<String>, StorageError> {
    let payload = rmp_serde::to_vec_named(&serde_json::json!({
        "signer_pubkey": hc.agent_key_uhcak(),
        "resolve": true,
    }))
    .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let bytes = hc
        .call_zome(ZOME, "list_feedback_signal_refs_by_signer", payload)
        .await?;

    #[derive(Deserialize)]
    struct RefEntry {
        target_cid: String,
        signal_kind: String,
        #[serde(default)]
        evidence_cid: Option<String>,
        #[serde(default)]
        standing_impact: String,
    }
    #[derive(Deserialize)]
    struct RefRow {
        action_hash: holochain_types::prelude::ActionHash,
        #[serde(default)]
        entry: Option<RefEntry>,
    }
    #[derive(Deserialize)]
    struct Refs {
        refs: Vec<RefRow>,
    }
    let refs: Refs = rmp_serde::from_slice(&bytes)
        .map_err(|e| StorageError::Serialization(format!("decode signer refs: {e}")))?;

    let mut matches: Vec<String> = refs
        .refs
        .into_iter()
        .filter_map(|r| {
            let e = r.entry?;
            let ok = e.target_cid == pinned.target_action_hash
                && e.signal_kind == pinned.signal_kind
                && e.evidence_cid.as_deref() == Some(evidence_action_hash)
                && e.standing_impact == pinned.standing_impact;
            ok.then(|| r.action_hash.to_string())
        })
        .collect();
    matches.sort();
    Ok(matches.into_iter().next())
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

fn ok_json<T: Serialize>(value: &T) -> Result<Response<Full<Bytes>>, StorageError> {
    let body =
        serde_json::to_vec(value).map_err(|e| StorageError::Internal(format!("serialize: {e}")))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

fn json_status(status: StatusCode, value: &serde_json::Value) -> Response<Full<Bytes>> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pinned() -> CorrectionRequest {
        CorrectionRequest {
            operation_id: "op-abc".to_string(),
            target_action_hash: "uhCkkTARGET".to_string(),
            signal_kind: "correction".to_string(),
            standing_impact: "debit-soft".to_string(),
        }
    }

    /// The evidence id is content-addressed BY THE OPERATION — that is what
    /// turns phase-1 recovery into a lookup instead of a guess.
    #[test]
    fn evidence_id_is_addressed_by_the_operation() {
        assert_eq!(pinned().evidence_content_id(), "correction:op-abc");
    }

    /// The digest is stable across runs (it is the immutability check a reuse
    /// with different bytes fails).
    #[test]
    fn request_digest_is_stable_and_field_sensitive() {
        let a = pinned().digest();
        assert_eq!(a, pinned().digest());
        let mut b = pinned();
        b.standing_impact = "debit-firm".to_string();
        assert_ne!(a, b.digest(), "a changed impact MUST change the digest");
        let mut c = pinned();
        c.target_action_hash = "uhCkkOTHER".to_string();
        assert_ne!(a, c.digest());
    }

    /// The embedded request is exactly what the coordinator's admission gate
    /// parses back out of `metadata_json`.
    #[test]
    fn metadata_json_embeds_the_request_under_the_expected_key() {
        let md = pinned().metadata_json();
        let v: serde_json::Value = serde_json::from_str(&md).expect("valid JSON");
        let req = v.get("correctionRequest").expect("key present");
        assert_eq!(req["operationId"], "op-abc");
        assert_eq!(req["targetActionHash"], "uhCkkTARGET");
        assert_eq!(req["signalKind"], "correction");
        assert_eq!(req["standingImpact"], "debit-soft");
        let round: CorrectionRequest =
            serde_json::from_value(req.clone()).expect("round-trips into the pinned request");
        assert_eq!(round, pinned());
    }
}
