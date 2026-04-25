//! Signal-emit endpoint — `POST /api/v1/signal/emit` (EPR Phase 2B Task C.2).
//!
//! Composes an EPR `Envelope` from a [`SignalIntent`], requests a signature
//! from the conductor (via [`crate::signing::ConductorSigningClient`]), and
//! ingests the resulting `Epr` through [`crate::services::epr_service::ingest`].
//!
//! ## Why this endpoint exists
//!
//! Browsers cannot sign EPRs in-process — agent keys live in the conductor's
//! lair, not in the browser. The signal harness emits a `SignalIntent`
//! describing what the renderer observed; this endpoint takes the round-trip
//! through the conductor and persists the resulting Envelope. The transient
//! HTTP payload is Category C (operational); the resulting persisted Envelope
//! is Category A (DHT-notarized).
//!
//! ## Wire schema
//! `elohim/sdk/schemas/v1/signal-intent.schema.json` (request)
//!
//! ## Architecture
//!
//! ```text
//! SignalIntent (browser/Tauri)
//!   ↓ POST /api/v1/signal/emit
//! prepare_signing_request    — pure builder: intent + manifest → canonical bytes
//!   ↓ canonical_bytes
//! ConductorSigningClient.sign — async: bytes → (signature, signer_pubkey)
//!   ↓ signature
//! assemble_signed_epr        — pure builder: prepared + signature → Epr
//!   ↓ epr
//! EprService::ingest         — persists; returns CID
//!   ↓
//! SignalEmitResponse { eprCid, eventCid }
//! ```
//!
//! The split between `prepare_signing_request` and `assemble_signed_epr` makes
//! the composition logic unit-testable without a real conductor or HTTP server
//! (carryover #5 from Batch B — builder-function pattern over test-server
//! scaffold for now).
//!
//! ## What is NOT done in C.2
//!
//! - **Schema validation of `payload`** — Phase 3 (manifest-graph resolver).
//!   Today the payload is encoded as canonical CBOR without per-pillar schema
//!   checks; the manifest registry only knows column projections.
//! - **Resolver-backed `verified_at` stamping** — uses [`ingest`] (structural
//!   verify only), not `ingest_with_cache`. The reconcile controller's sweep
//!   path stamps `verified_at` after the cache has the agent's pubkey timeline
//!   (Batch A's responsibility).
//! - **VF-GraphQL semantics** — per memory pin
//!   `project_epr_substrate_vs_vf_graphql`, the substrate is graph-primitive;
//!   pillar-specific vocabularies are app-layer (Phase 4).
//! - **Write-through flag** — Tasks C.4–C.6 wire the per-pillar enablement
//!   flag and integrity-event exception. Until then, all valid intents ingest.

use std::str::FromStr;
use std::sync::Arc;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use cid::Cid;
use elohim_epr::{cid::compute_cid, Coupling, Envelope, Epr, EprKind, Reach, Signature};
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, header, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, warn};

use crate::db::DbPool;
use crate::error::StorageError;
use crate::projector::mapping::{ManifestRegistry, RegisteredProjection};
use crate::services::epr_service::{ingest, EprIngestResult};
use crate::services::response;
use crate::signing::{ConductorSigningClient, SigningError};

use super::get_conn;

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// Request body for `POST /api/v1/signal/emit`.
///
/// Field names match `elohim/sdk/schemas/v1/signal-intent.schema.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalIntent {
    /// Pillar that authored this signal.
    pub pillar: String,
    /// Pillar-defined signal name. Today this MUST equal an `EprKind` value
    /// (e.g. "EconomicEvent"); future pillars may add a `signalType → kind`
    /// mapping in their manifest. The conservative single-mapping rule keeps
    /// shefa's Batch C migration unblocked without overgeneralizing.
    pub signal_type: String,
    /// CIDv1 of the Agent EPR who authored this signal.
    pub agent_cid: String,
    /// Pillar-specific payload (encoded as canonical CBOR for the Envelope).
    pub payload: serde_json::Value,
    /// Three-leg coupling refs (knowledge / value / governance).
    pub coupling_refs: CouplingRefsInput,
    /// Optional reach hint. Defaults to `community` when absent.
    #[serde(default)]
    pub reach: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CouplingRefsInput {
    #[serde(default)]
    pub knowledge: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub governance: Option<String>,
}

/// Response body for `POST /api/v1/signal/emit`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalEmitResponse {
    /// CIDv1 of the persisted Envelope.
    pub epr_cid: String,
    /// Application-layer event id. Today equal to `eprCid`; reserved for
    /// future cases where one intent emits multiple EPRs.
    pub event_cid: String,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum SignalEmitError {
    #[error("invalid request body: {0}")]
    BadRequest(String),

    #[error("no projection registered for pillar='{pillar}' kind='{kind}'")]
    NoProjection { pillar: String, kind: String },

    #[error("unsupported signalType '{0}' — must equal an EprKind name")]
    UnsupportedKind(String),

    #[error("invalid agentCid: {0}")]
    BadAgentCid(String),

    #[error("invalid coupling ref: {0}")]
    BadCouplingRef(String),

    #[error("invalid reach '{0}'")]
    BadReach(String),

    #[error("payload encoding failed: {0}")]
    PayloadEncode(String),

    #[error("envelope canonicalization failed: {0}")]
    Canonicalize(String),

    #[error("conductor signing failed: {0}")]
    Sign(#[from] SigningError),

    #[error("ingest failed: {0}")]
    Ingest(String),

    #[error("signature length must be 64 bytes; got {0}")]
    BadSignatureLength(usize),
}

impl SignalEmitError {
    fn http_status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_)
            | Self::UnsupportedKind(_)
            | Self::BadAgentCid(_)
            | Self::BadCouplingRef(_)
            | Self::BadReach(_)
            | Self::PayloadEncode(_)
            | Self::BadSignatureLength(_) => StatusCode::BAD_REQUEST,
            Self::NoProjection { .. } => StatusCode::NOT_FOUND,
            Self::Canonicalize(_) | Self::Ingest(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Sign(_) => StatusCode::BAD_GATEWAY,
        }
    }
}

// ---------------------------------------------------------------------------
// Pure builder — phase 1: prepare canonical bytes for signing
// ---------------------------------------------------------------------------

/// Everything `assemble_signed_epr` needs to finalize the Envelope after the
/// conductor returns a signature. Carries the canonical bytes so callers can
/// pass them directly to the signing client.
#[derive(Debug)]
pub struct PreparedSigningRequest {
    pub agent_cid: Cid,
    pub kind: EprKind,
    pub schema_ref: Cid,
    pub schema_key: String,
    pub reach: Reach,
    pub coupling: Coupling,
    pub issued_at: DateTime<Utc>,
    pub payload: Vec<u8>,
    pub canonical_bytes: Vec<u8>,
}

/// Build a `PreparedSigningRequest` from a `SignalIntent` + the manifest
/// registry. Pure: no I/O, no async, fully unit-testable.
///
/// Steps:
/// 1. Parse `signalType` → `EprKind` (must match a kind name).
/// 2. Look up the pillar's projection for that kind. If absent, error
///    `NoProjection` (Invariant I2: manifest-authority).
/// 3. Resolve `agentCid` to a `Cid`.
/// 4. Compute the schema-ref placeholder. **Phase 3 will resolve this from a
///    real Manifest EPR**; for C.2 we use a deterministic placeholder so the
///    same pillar/kind always gets the same schema_ref CID. This makes the
///    composed Envelopes structurally valid even before manifest-graph
///    resolution lands.
/// 5. Resolve `reach` (intent override or default `community`).
/// 6. Resolve coupling refs.
/// 7. Encode `payload` as canonical CBOR via `serde_cbor` deterministic mode.
/// 8. Build a provisional `Envelope` (placeholder cid + zero-byte signature)
///    and compute its canonical bytes.
pub fn prepare_signing_request(
    intent: &SignalIntent,
    registry: &ManifestRegistry,
) -> Result<PreparedSigningRequest, SignalEmitError> {
    let kind = parse_epr_kind(&intent.signal_type)?;

    let projection = registry
        .find_projection(&intent.pillar, kind_canonical(kind))
        .ok_or_else(|| SignalEmitError::NoProjection {
            pillar: intent.pillar.clone(),
            kind: kind_canonical(kind).to_string(),
        })?;

    let agent_cid = Cid::from_str(&intent.agent_cid)
        .map_err(|e| SignalEmitError::BadAgentCid(e.to_string()))?;

    let schema_ref = placeholder_schema_ref(&intent.pillar, projection);
    let schema_key = projection.schema_key.clone();

    let reach = match intent.reach.as_deref() {
        Some(r) => parse_reach(r)?,
        None => Reach::Community,
    };

    let coupling = parse_coupling(&intent.coupling_refs)?;
    let payload = encode_payload(&intent.payload)?;
    let issued_at = Utc::now();

    let provisional = Envelope {
        cid: compute_cid(&[0]),
        kind,
        schema_ref,
        schema_key: schema_key.clone(),
        reach,
        coupling: coupling.clone(),
        claims: vec![],
        supersedes: None,
        superseded_by: None,
        issued_at,
        proof: Signature::ed25519(agent_cid, vec![0u8; 64]),
    };

    let canonical_bytes = provisional
        .canonical_bytes(&payload)
        .map_err(|e| SignalEmitError::Canonicalize(e.to_string()))?;

    Ok(PreparedSigningRequest {
        agent_cid,
        kind,
        schema_ref,
        schema_key,
        reach,
        coupling,
        issued_at,
        payload,
        canonical_bytes,
    })
}

// ---------------------------------------------------------------------------
// Pure builder — phase 2: assemble final Epr from signed canonical bytes
// ---------------------------------------------------------------------------

/// Assemble the final `Epr` once the conductor has returned a signature for
/// `prepared.canonical_bytes`. The CID is derived from those same canonical
/// bytes; the proof carries `agent_cid` as the signer (matching how
/// `ingest` resolves the signer's public key in Batch A's pubkey timeline).
pub fn assemble_signed_epr(
    prepared: PreparedSigningRequest,
    signature: Vec<u8>,
) -> Result<Epr, SignalEmitError> {
    if signature.len() != 64 {
        return Err(SignalEmitError::BadSignatureLength(signature.len()));
    }

    let cid = compute_cid(&prepared.canonical_bytes);
    let signature = Signature::ed25519(prepared.agent_cid, signature);

    let envelope = Envelope {
        cid,
        kind: prepared.kind,
        schema_ref: prepared.schema_ref,
        schema_key: prepared.schema_key,
        reach: prepared.reach,
        coupling: prepared.coupling,
        claims: vec![],
        supersedes: None,
        superseded_by: None,
        issued_at: prepared.issued_at,
        proof: signature,
    };

    Ok(Epr {
        envelope,
        payload: prepared.payload,
    })
}

// ---------------------------------------------------------------------------
// HTTP dispatcher
// ---------------------------------------------------------------------------

/// Handle `POST /api/v1/signal/emit`.
///
/// `signing_client` is `Option` so storage instances without a conductor
/// connection (test fixtures, dev environments) gracefully return 503 rather
/// than panicking. In production all instances should have it wired at startup.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    pool: &DbPool,
    registry: &ManifestRegistry,
    signing_client: Option<&Arc<ConductorSigningClient>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::POST {
        return Ok(response::method_not_allowed());
    }

    let Some(signing_client) = signing_client else {
        return Ok(response::service_unavailable(
            "ConductorSigningClient not configured; /api/v1/signal/emit unavailable",
        ));
    };

    // Read body (bounded — request_semaphore in http.rs gates concurrency,
    // and the request size cap is applied at the listener layer).
    let body_bytes = match req.into_body().collect().await {
        Ok(b) => b.to_bytes(),
        Err(e) => {
            return Ok(error_response(SignalEmitError::BadRequest(format!(
                "body read failed: {e}"
            ))));
        }
    };

    let intent: SignalIntent = match serde_json::from_slice(&body_bytes) {
        Ok(i) => i,
        Err(e) => {
            return Ok(error_response(SignalEmitError::BadRequest(format!(
                "JSON deserialize failed: {e}"
            ))));
        }
    };

    let prepared = match prepare_signing_request(&intent, registry) {
        Ok(p) => p,
        Err(e) => return Ok(error_response(e)),
    };

    debug!(
        pillar = %intent.pillar,
        signal_type = %intent.signal_type,
        agent_cid = %intent.agent_cid,
        canonical_len = prepared.canonical_bytes.len(),
        "signal_emit: requesting conductor signature"
    );

    let sig_response = match signing_client
        .sign(&intent.agent_cid, prepared.canonical_bytes.clone())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "signal_emit: conductor signing failed");
            return Ok(error_response(SignalEmitError::Sign(e)));
        }
    };

    let epr = match assemble_signed_epr(prepared, sig_response.signature) {
        Ok(e) => e,
        Err(e) => return Ok(error_response(e)),
    };

    let mut conn = get_conn(pool)?;

    let result: EprIngestResult = match ingest(&mut conn, epr) {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "signal_emit: ingest failed");
            return Ok(error_response(SignalEmitError::Ingest(e.to_string())));
        }
    };

    let response_body = SignalEmitResponse {
        epr_cid: result.cid.clone(),
        event_cid: result.cid,
    };

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(
            serde_json::to_string(&response_body).unwrap_or_default(),
        )))
        .unwrap())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn error_response(err: SignalEmitError) -> Response<Full<Bytes>> {
    let status = err.http_status();
    let body = serde_json::json!({ "error": err.to_string() });
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

fn parse_epr_kind(s: &str) -> Result<EprKind, SignalEmitError> {
    Ok(match s {
        "Content" => EprKind::Content,
        "Agent" => EprKind::Agent,
        "Manifest" => EprKind::Manifest,
        "Claim" => EprKind::Claim,
        "Observation" => EprKind::Observation,
        "EconomicEvent" => EprKind::EconomicEvent,
        "Commitment" => EprKind::Commitment,
        "Attestation" => EprKind::Attestation,
        "Delegation" => EprKind::Delegation,
        other => return Err(SignalEmitError::UnsupportedKind(other.to_string())),
    })
}

fn kind_canonical(k: EprKind) -> &'static str {
    match k {
        EprKind::Content => "Content",
        EprKind::Agent => "Agent",
        EprKind::Manifest => "Manifest",
        EprKind::Claim => "Claim",
        EprKind::Observation => "Observation",
        EprKind::EconomicEvent => "EconomicEvent",
        EprKind::Commitment => "Commitment",
        EprKind::Attestation => "Attestation",
        EprKind::Delegation => "Delegation",
    }
}

fn parse_reach(s: &str) -> Result<Reach, SignalEmitError> {
    Ok(match s {
        "private" => Reach::Private,
        "selfScope" | "self" => Reach::SelfScope,
        "intimate" => Reach::Intimate,
        "trusted" => Reach::Trusted,
        "familiar" => Reach::Familiar,
        "community" => Reach::Community,
        "public" => Reach::Public,
        "commons" => Reach::Commons,
        other => return Err(SignalEmitError::BadReach(other.to_string())),
    })
}

fn parse_coupling(refs: &CouplingRefsInput) -> Result<Coupling, SignalEmitError> {
    let parse_one = |s: &Option<String>, leg: &str| -> Result<Option<Cid>, SignalEmitError> {
        s.as_deref()
            .map(|s| {
                Cid::from_str(s).map_err(|e| SignalEmitError::BadCouplingRef(format!("{leg}: {e}")))
            })
            .transpose()
    };

    Ok(Coupling {
        knowledge: parse_one(&refs.knowledge, "knowledge")?,
        value: parse_one(&refs.value, "value")?,
        governance: parse_one(&refs.governance, "governance")?,
    })
}

fn encode_payload(value: &serde_json::Value) -> Result<Vec<u8>, SignalEmitError> {
    // serde_ipld_dagcbor produces deterministic CBOR (RFC 8949 Core Deterministic
    // Encoding) — the same encoding used by elohim-epr for envelope canonicalization.
    // Same input → same bytes, so the resulting Envelope CID is reproducible.
    serde_ipld_dagcbor::to_vec(value).map_err(|e| SignalEmitError::PayloadEncode(e.to_string()))
}

/// Derive a deterministic placeholder schema_ref CID for a given (pillar,
/// kind, schema_key) tuple. Same inputs → same CID across runs, so EPRs
/// composed before/after Phase 3 manifest-graph resolution can be compared.
///
/// **Phase 3 replaces this** with the real Manifest EPR's CID resolved from
/// the manifest graph. For Batch C the placeholder is sufficient: storage
/// `ingest` performs structural-only verification and does not yet
/// dereference `schema_ref` to validate the payload schema.
fn placeholder_schema_ref(pillar: &str, projection: &RegisteredProjection) -> Cid {
    let key = format!(
        "placeholder:{}:{}:{}",
        pillar, projection.kind, projection.schema_key
    );
    compute_cid(key.as_bytes())
}

// ---------------------------------------------------------------------------
// Tests — builder-function level only (carryover #5 from Batch B).
// HTTP path is exercised end-to-end by C.7's integration test.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projector::mapping::ManifestRegistry;

    fn test_cid(seed: &[u8]) -> String {
        compute_cid(seed).to_string()
    }

    fn sample_intent() -> SignalIntent {
        SignalIntent {
            pillar: "shefa".to_string(),
            signal_type: "EconomicEvent".to_string(),
            agent_cid: test_cid(b"alice-agent-epr"),
            payload: serde_json::json!({
                "action": "produce",
                "provider": "did:agent:alice",
                "receiver": "did:agent:bob"
            }),
            coupling_refs: CouplingRefsInput {
                value: Some(test_cid(b"value-leg-epr")),
                ..Default::default()
            },
            reach: None,
        }
    }

    #[test]
    fn prepare_succeeds_for_registered_projection() {
        let registry = ManifestRegistry::with_shefa_economic_event();
        let prepared = prepare_signing_request(&sample_intent(), &registry).expect("prepare ok");
        assert_eq!(prepared.kind, EprKind::EconomicEvent);
        assert_eq!(prepared.schema_key, "economic-event");
        assert_eq!(prepared.reach, Reach::Community);
        assert!(prepared.coupling.value.is_some());
        assert!(prepared.coupling.knowledge.is_none());
        assert!(!prepared.canonical_bytes.is_empty());
        assert!(!prepared.payload.is_empty());
    }

    #[test]
    fn prepare_rejects_unregistered_pillar_kind() {
        let registry = ManifestRegistry::empty();
        let err = prepare_signing_request(&sample_intent(), &registry).unwrap_err();
        assert!(matches!(err, SignalEmitError::NoProjection { .. }));
    }

    #[test]
    fn prepare_rejects_unknown_kind() {
        let registry = ManifestRegistry::with_shefa_economic_event();
        let mut intent = sample_intent();
        intent.signal_type = "NotARealKind".to_string();
        let err = prepare_signing_request(&intent, &registry).unwrap_err();
        assert!(matches!(err, SignalEmitError::UnsupportedKind(_)));
    }

    #[test]
    fn prepare_rejects_malformed_agent_cid() {
        let registry = ManifestRegistry::with_shefa_economic_event();
        let mut intent = sample_intent();
        intent.agent_cid = "not-a-cid".to_string();
        let err = prepare_signing_request(&intent, &registry).unwrap_err();
        assert!(matches!(err, SignalEmitError::BadAgentCid(_)));
    }

    #[test]
    fn prepare_rejects_invalid_reach() {
        let registry = ManifestRegistry::with_shefa_economic_event();
        let mut intent = sample_intent();
        intent.reach = Some("invalid".to_string());
        let err = prepare_signing_request(&intent, &registry).unwrap_err();
        assert!(matches!(err, SignalEmitError::BadReach(_)));
    }

    #[test]
    fn assemble_rejects_wrong_signature_length() {
        let registry = ManifestRegistry::with_shefa_economic_event();
        let prepared = prepare_signing_request(&sample_intent(), &registry).unwrap();
        let err = assemble_signed_epr(prepared, vec![0; 32]).unwrap_err();
        assert!(matches!(err, SignalEmitError::BadSignatureLength(32)));
    }

    #[test]
    fn assemble_produces_epr_with_matching_cid() {
        let registry = ManifestRegistry::with_shefa_economic_event();
        let prepared = prepare_signing_request(&sample_intent(), &registry).unwrap();
        let canonical = prepared.canonical_bytes.clone();
        let epr = assemble_signed_epr(prepared, vec![0xAB; 64]).expect("assemble");
        // CID is derived from the same canonical bytes the conductor signed.
        assert_eq!(
            epr.envelope.cid.to_string(),
            compute_cid(&canonical).to_string()
        );
        assert_eq!(epr.envelope.proof.signature.len(), 64);
        assert_eq!(epr.envelope.proof.algorithm, "ed25519");
    }

    #[test]
    fn placeholder_schema_ref_is_stable() {
        let registry = ManifestRegistry::with_shefa_economic_event();
        let p1 = prepare_signing_request(&sample_intent(), &registry).unwrap();
        let p2 = prepare_signing_request(&sample_intent(), &registry).unwrap();
        assert_eq!(p1.schema_ref, p2.schema_ref);
    }

    #[test]
    fn error_status_codes_match_intent() {
        assert_eq!(
            SignalEmitError::BadRequest("x".into()).http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            SignalEmitError::NoProjection {
                pillar: "x".into(),
                kind: "y".into()
            }
            .http_status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            SignalEmitError::Ingest("x".into()).http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
