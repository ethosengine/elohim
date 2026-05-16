//! EPR REST controller — routes under /api/v1/epr.
//!
//! Routes talk to an EprStore trait so P2P federation (Phase 2c) can be added
//! without route changes. Phase 2a ships FederatedEprStore with the libp2p
//! bridge stubbed — all calls fall through to LocalEprStore.
//!
//! Route table:
//!   GET  /api/v1/epr                        → list_epr
//!   GET  /api/v1/epr/:cid                   → get_epr
//!   GET  /api/v1/epr/:cid/envelope          → get_envelope
//!   GET  /api/v1/epr/:cid/payload           → get_payload
//!   GET  /api/v1/epr/:cid/verify            → get_verify
//!   GET  /api/v1/epr/:cid/providers         → get_providers
//!   PUT  /api/v1/epr/:cid                   → put_epr
//!
//! Phase 3.5 — Light Up the Graph (T17/T19):
//! FeedbackSignal arrival fan-out is wired in `put_epr`. When a FeedbackSignal
//! EPR arrives, the fan-out (project_signal + back_prop_one_hop + flood_feedback)
//! runs if the EPR is not local-origin. Dependencies (outbound_sink,
//! gossip_publisher, manifest_registry, sealing_keys) are injected via
//! `EprFanOutCtx`; T22 will wire real values from `main.rs`. Until then,
//! call sites pass `None` and the fan-out steps are skipped gracefully.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::back_prop::OutboundSink;
use crate::services::epr_service::FetchedEpr;
use crate::services::epr_store::{default_epr_store, EprStore};
use crate::services::gossip_flood::GossipPublisher;
use crate::services::manifest_registry::ManifestRegistry;
use crate::services::response;
use crate::views::{EprCouplingView, EprEnvelopeView, EprSignatureView, EprView};

use super::get_conn;

// ---------------------------------------------------------------------------
// T17: Local-origin dedup helper
// ---------------------------------------------------------------------------

/// Returns true if the FeedbackSignal was sent by us (local node) — used to
/// skip back-prop / flood / project fan-out to avoid loops when our own
/// gossip-flood arrives back to us.
///
/// Comparison is a simple string equality on the peer ID. The caller passes
/// the `signed_by` field from the EPR envelope and the local node's peer ID
/// as reported by the swarm.
fn is_local_origin(envelope_signed_by: &str, local_peer_id: &str) -> bool {
    envelope_signed_by == local_peer_id
}

// ---------------------------------------------------------------------------
// T19: FeedbackSignal fan-out context
//
// Carries the optional runtime dependencies for the FeedbackSignal fan-out in
// `put_epr`. Fields are `None` until T22 wires the real values from `main.rs`.
// All fan-out steps that require a `None` field are silently skipped.
// ---------------------------------------------------------------------------

/// Optional runtime context for FeedbackSignal arrival fan-out.
///
/// All fields default to `None`. T22 constructs and injects a fully-wired
/// instance from `main.rs` startup. Call sites that do not need fan-out
/// (tests, non-P2P paths) continue passing `None`.
pub struct EprFanOutCtx {
    /// Manifest registry providing the active standing-policy debit weights.
    pub manifest_registry: Option<Arc<ManifestRegistry>>,
    /// Outbound sink for back-prop one-hop sends (LibP2POutboundSink in production).
    pub outbound_sink: Option<Arc<dyn OutboundSink>>,
    /// Gossip publisher for flood-feedback broadcasts (LibP2PGossipPublisher in production).
    pub gossip_publisher: Option<Arc<dyn GossipPublisher>>,
    /// Sealing PUBLIC keys for `record_predecessor` (stores predecessor under 2-of-2).
    pub sealing_keys: Option<Arc<SealingKeyPair>>,
    /// Unsealing key bundle for `back_prop_one_hop` (both pub+priv keys required to
    /// decrypt predecessor records). Absent in dev/test environments where key files
    /// are not configured.
    ///
    /// TODO(T22-followup): Load from a persistent node-key file or PKCS#11 provider.
    /// For local dev this is populated from ephemeral keys generated at startup.
    pub unsealing_keys: Option<Arc<UnsealingKeyBundle>>,
    /// Local peer ID (base58 multibase) for local-origin dedup.
    pub local_peer_id: Option<String>,
    /// Local agent pubkey (ed25519 raw bytes) passed as evaluator to project_signal.
    pub local_pubkey: Option<Vec<u8>>,
    /// CID of the active standing-policy manifest for standing projector provenance.
    pub standing_policy_cid: Option<String>,
    /// Graph engine for post-put EprHead projection (graph-native feature).
    ///
    /// When `Some`, a successful `PUT /api/v1/epr` decodes the atom's canonical
    /// bytes as an `EprHead` and calls `GraphProjector::project_head` after the
    /// diesel transaction commits (sequential semantics per spec §6.2). Graph
    /// errors are logged at `warn!` and never bubble to the HTTP response.
    #[cfg(feature = "graph-native")]
    pub graph_engine: Option<std::sync::Arc<crate::graph::engine::GraphEngine>>,
}

/// Private-key bundle for back_prop unsealing.
///
/// Wraps the four key components that `back_prop::UnsealingKeys` borrows.
/// Stored in `Arc<UnsealingKeyBundle>` on the fan-out context so the key
/// material is heap-allocated and reference-counted, never copied by value.
///
/// TODO(T22-followup): in production these should be loaded from a persisted
/// node-key file (or HSM/PKCS#11 slot). Current dev path: ephemeral keys
/// generated from deterministic seeds at startup (safe for dev/test; NOT for
/// production use where predecessor records must survive process restart).
pub struct UnsealingKeyBundle {
    pub mishpat_pk: crate::services::sealed_against_self::MishpatQuorumPubKey,
    pub mishpat_sk: crate::services::sealed_against_self::MishpatQuorumSecretKey,
    pub imagodei_pk: crate::services::sealed_against_self::ImagodeiPubKey,
    pub imagodei_sk: crate::services::sealed_against_self::ImagodeiSecretKey,
}

/// Public-key pair for `record_predecessor` sealing.
///
/// Wraps the two keys that `back_prop::SealingPubKeys` borrows. We own them
/// here so they can be stored in `Arc<SealingKeyPair>` on the fan-out context.
pub struct SealingKeyPair {
    pub mishpat_pk: crate::services::sealed_against_self::MishpatQuorumPubKey,
    pub imagodei_pk: crate::services::sealed_against_self::ImagodeiPubKey,
}

// ---------------------------------------------------------------------------
// Dispatcher — matches existing controller signature convention
// ---------------------------------------------------------------------------

/// Handle `/api/v1/epr*` requests.
///
/// `swarm_tx` is threaded from `HttpServer` so that `PUT /api/v1/epr/:cid`
/// can issue `KadStartProviding` after a successful local put (D.2). When
/// `None` (no P2P swarm configured) Kad advertisement is silently skipped.
///
/// `fan_out_ctx` carries the optional Phase 3.5 dependencies for FeedbackSignal
/// fan-out. Pass `None` until T22 wires the real values from `main.rs`.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
    swarm_tx: Option<tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>>,
    fan_out_ctx: Option<Arc<EprFanOutCtx>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Normalise: strip leading slash, giving us "", "abc123", "abc123/envelope" …
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/epr
        (&Method::GET, "") => list_epr(req, pool, ctx).await,

        // PUT /api/v1/epr/:cid  (content-addressed idempotent put)
        (&Method::PUT, cid) if !cid.is_empty() && !cid.contains('/') => {
            put_epr(req, cid, pool, ctx, swarm_tx, fan_out_ctx).await
        }

        // GET /api/v1/epr/:cid/envelope
        (&Method::GET, p) if p.ends_with("/envelope") && p.split('/').count() == 2 => {
            let cid = p.trim_end_matches("/envelope");
            get_envelope(req, cid, pool, ctx).await
        }

        // GET /api/v1/epr/:cid/payload
        (&Method::GET, p) if p.ends_with("/payload") && p.split('/').count() == 2 => {
            let cid = p.trim_end_matches("/payload");
            get_payload(req, cid, pool, ctx).await
        }

        // GET /api/v1/epr/:cid/verify
        (&Method::GET, p) if p.ends_with("/verify") && p.split('/').count() == 2 => {
            let cid = p.trim_end_matches("/verify");
            get_verify(req, cid, pool, ctx).await
        }

        // GET /api/v1/epr/:cid/providers
        (&Method::GET, p) if p.ends_with("/providers") && p.split('/').count() == 2 => {
            let cid = p.trim_end_matches("/providers");
            get_providers(req, cid, pool, ctx, swarm_tx).await
        }

        // GET /api/v1/epr/:cid  (plain CID — must not contain '/')
        (&Method::GET, cid) if !cid.contains('/') => get_epr(req, cid, pool, ctx).await,

        _ => Ok(response::not_found(&format!(
            "Unknown epr route: {} /api/v1/epr/{}",
            method, path
        ))),
    }
}

// ---------------------------------------------------------------------------
// View builders
// ---------------------------------------------------------------------------

pub(crate) fn to_envelope_view(fetched: &FetchedEpr) -> EprEnvelopeView {
    let mut coupling = EprCouplingView::default();
    for row in &fetched.coupling {
        match row.leg.as_str() {
            "knowledge" => coupling.knowledge = Some(row.target_cid.clone()),
            "value" => coupling.value = Some(row.target_cid.clone()),
            "governance" => coupling.governance = Some(row.target_cid.clone()),
            _ => {}
        }
    }

    EprEnvelopeView {
        cid: fetched.atom.cid.clone(),
        kind: fetched.atom.kind.clone(),
        schema_ref: fetched.atom.schema_ref.clone(),
        schema_key: fetched.atom.schema_key.clone(),
        reach: fetched.atom.reach.clone(),
        coupling,
        claims: fetched.claims.iter().map(|c| c.claim_cid.clone()).collect(),
        supersedes: fetched.atom.supersedes.clone(),
        superseded_by: fetched.superseded_by.clone(),
        issued_at: fetched.atom.issued_at.clone(),
        proof: EprSignatureView {
            signer: fetched.atom.signer_cid.clone(),
            algorithm: fetched.atom.proof_algorithm.clone(),
            signature: hex::encode(&fetched.atom.proof_bytes),
        },
    }
}

fn to_epr_view(fetched: &FetchedEpr, include_canonical: bool) -> EprView {
    EprView {
        envelope: to_envelope_view(fetched),
        payload: hex::encode(&fetched.atom.payload_bytes),
        canonical_bytes: if include_canonical {
            Some(hex::encode(&fetched.atom.canonical_bytes))
        } else {
            None
        },
    }
}

// ---------------------------------------------------------------------------
// Reach enforcement (envelope-level, no payload parse)
// ---------------------------------------------------------------------------

/// Returns true when the EPR's reach is visible to the caller.
///
/// Phase 2a: commons + public = anyone. Everything else requires an authenticated
/// caller (non-empty Authorization header). Real identity check is deferred to
/// Phase 2b middleware. 404 (not 403) is returned on denial so that EPR existence
/// cannot be probed by unauthenticated callers.
fn reach_visible_to(reach: &str, req: &Request<Incoming>) -> bool {
    match reach {
        "commons" | "public" => true,
        _ => req
            .headers()
            .get(hyper::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false),
    }
}

// ---------------------------------------------------------------------------
// GET /api/v1/epr/:cid
// ---------------------------------------------------------------------------

async fn get_epr(
    req: Request<Incoming>,
    cid: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let include_canonical = req
        .uri()
        .query()
        .map(|q| q.contains("includeCanonical=true"))
        .unwrap_or(false);

    let store = default_epr_store(None, None, None, ctx.local_libp2p_peer_id.clone());
    let mut conn = get_conn(pool)?;

    let Some(outcome) = store.fetch(&mut conn, cid)? else {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    };
    if !reach_visible_to(&outcome.fetched.atom.reach, &req) {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    }

    let view = to_epr_view(&outcome.fetched, include_canonical);
    let body =
        serde_json::to_vec(&view).map_err(|e| StorageError::Database(format!("serialize: {e}")))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header("X-Epr-Source", outcome.source.header_value())
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

// ---------------------------------------------------------------------------
// GET /api/v1/epr/:cid/envelope
// ---------------------------------------------------------------------------

async fn get_envelope(
    req: Request<Incoming>,
    cid: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let store = default_epr_store(None, None, None, ctx.local_libp2p_peer_id.clone());
    let mut conn = get_conn(pool)?;

    let Some(outcome) = store.fetch(&mut conn, cid)? else {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    };
    if !reach_visible_to(&outcome.fetched.atom.reach, &req) {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    }

    let body = serde_json::to_vec(&to_envelope_view(&outcome.fetched))
        .map_err(|e| StorageError::Database(format!("serialize: {e}")))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header("X-Epr-Source", outcome.source.header_value())
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

// ---------------------------------------------------------------------------
// GET /api/v1/epr/:cid/payload
// ---------------------------------------------------------------------------

async fn get_payload(
    req: Request<Incoming>,
    cid: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let store = default_epr_store(None, None, None, ctx.local_libp2p_peer_id.clone());
    let mut conn = get_conn(pool)?;

    let Some(outcome) = store.fetch(&mut conn, cid)? else {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    };
    if !reach_visible_to(&outcome.fetched.atom.reach, &req) {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    }

    // Raw bytes; Content-Type = application/octet-stream.
    // Phase 3 will look up the real MIME via the manifest schema resolver.
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/octet-stream")
        .header("X-Epr-Cid", &outcome.fetched.atom.cid)
        .header("X-Epr-Source", outcome.source.header_value())
        .body(Full::new(Bytes::from(
            outcome.fetched.atom.payload_bytes.clone(),
        )))
        .unwrap())
}

// ---------------------------------------------------------------------------
// GET /api/v1/epr/:cid/verify?publicKey=<hex>
// ---------------------------------------------------------------------------

async fn get_verify(
    req: Request<Incoming>,
    cid: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Caller provides publicKey as hex in query string:
    //   GET /api/v1/epr/:cid/verify?publicKey=<64-hex>
    let query = req.uri().query().unwrap_or("");
    let Some(pk_hex) = query.split('&').find_map(|p| p.strip_prefix("publicKey=")) else {
        return Ok(response::bad_request("publicKey query parameter required"));
    };

    let Ok(pk_bytes) = hex::decode(pk_hex) else {
        return Ok(response::bad_request("publicKey must be valid hex"));
    };
    if pk_bytes.len() != 32 {
        return Ok(response::bad_request("publicKey must decode to 32 bytes"));
    }
    let mut pk = [0u8; 32];
    pk.copy_from_slice(&pk_bytes);

    let store = default_epr_store(None, None, None, ctx.local_libp2p_peer_id.clone());
    let mut conn = get_conn(pool)?;

    // Reach check: if the EPR isn't visible to the caller, return 404.
    let Some(outcome) = store.fetch(&mut conn, cid)? else {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    };
    if !reach_visible_to(&outcome.fetched.atom.reach, &req) {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    }

    let report = store.verify(&mut conn, cid, &pk)?;

    let view = crate::views::EprVerifyView {
        cid: report.cid,
        verified: report.verified,
        stages_run: report.stages_run,
        stages_skipped: report.stages_skipped,
        error: report.error.map(|e| crate::views::EprVerifyErrorView {
            stage: e.stage,
            message: e.message,
        }),
    };

    let body =
        serde_json::to_vec(&view).map_err(|e| StorageError::Database(format!("serialize: {e}")))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header("X-Epr-Source", outcome.source.header_value())
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

// ---------------------------------------------------------------------------
// GET /api/v1/epr/:cid/providers
// ---------------------------------------------------------------------------

async fn get_providers(
    req: Request<Incoming>,
    cid: &str,
    pool: &DbPool,
    ctx: &AppContext,
    swarm_tx: Option<tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // D.7: pass local PeerId so FederatedEprStore deduplicates self from providers list
    let store = default_epr_store(
        swarm_tx,
        Some(pool.clone()),
        None,
        ctx.local_libp2p_peer_id.clone(),
    );
    let mut conn = get_conn(pool)?;

    // Reach check: we need to know the atom's reach to enforce, but if the
    // atom isn't locally known and we can't reach peers yet (Phase 2c), we
    // can't enforce reach before returning providers. For Phase 2a: if the
    // atom isn't local, return [] (no-op disclosure).
    if let Some(outcome) = store.fetch(&mut conn, cid)? {
        if !reach_visible_to(&outcome.fetched.atom.reach, &req) {
            return Ok(response::not_found(&format!("epr not found: {cid}")));
        }
    }

    let providers = store.providers(&mut conn, cid)?;
    let provider_strings: Vec<String> = providers.into_iter().map(|p| p.peer_id).collect();

    let view = crate::views::EprProvidersView {
        cid: cid.to_string(),
        providers: provider_strings,
    };
    let body =
        serde_json::to_vec(&view).map_err(|e| StorageError::Database(format!("serialize: {e}")))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

// ---------------------------------------------------------------------------
// PUT /api/v1/epr/:cid  (idempotent content-addressed put)
// ---------------------------------------------------------------------------

async fn put_epr(
    req: Request<Incoming>,
    path_cid: &str,
    pool: &DbPool,
    ctx: &AppContext,
    swarm_tx: Option<tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>>,
    fan_out_ctx: Option<Arc<EprFanOutCtx>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    use elohim_epr::{Coupling, Envelope, Epr, EprKind, Reach, Signature};
    use std::str::FromStr;

    let input: crate::views::EprPublishInput = super::parse_body(req).await?;

    // Path CID must match envelope CID — enforces the content-addressed contract
    // at the route level.
    if input.envelope.cid != path_cid {
        return Ok(response::bad_request(&format!(
            "path cid {} does not match envelope cid {}",
            path_cid, input.envelope.cid
        )));
    }

    // Rehydrate the Rust Epr from wire view.
    let cid = cid::Cid::from_str(&input.envelope.cid)
        .map_err(|e| StorageError::InvalidInput(format!("bad cid: {e}")))?;
    let schema_ref = cid::Cid::from_str(&input.envelope.schema_ref)
        .map_err(|e| StorageError::InvalidInput(format!("bad schemaRef: {e}")))?;
    let signer = cid::Cid::from_str(&input.envelope.proof.signer)
        .map_err(|e| StorageError::InvalidInput(format!("bad signer: {e}")))?;

    let kind = match input.envelope.kind.as_str() {
        "Content" => EprKind::Content,
        "Agent" => EprKind::Agent,
        "Manifest" => EprKind::Manifest,
        "Claim" => EprKind::Claim,
        "Observation" => EprKind::Observation,
        "EconomicEvent" => EprKind::EconomicEvent,
        "Commitment" => EprKind::Commitment,
        "Attestation" => EprKind::Attestation,
        "Delegation" => EprKind::Delegation,
        "FeedbackSignal" => EprKind::FeedbackSignal,
        other => return Ok(response::bad_request(&format!("unknown kind: {other}"))),
    };

    let reach = match input.envelope.reach.as_str() {
        "private" => Reach::Private,
        "self" => Reach::SelfScope,
        "intimate" => Reach::Intimate,
        "trusted" => Reach::Trusted,
        "familiar" => Reach::Familiar,
        "community" => Reach::Community,
        "public" => Reach::Public,
        "commons" => Reach::Commons,
        other => return Ok(response::bad_request(&format!("unknown reach: {other}"))),
    };

    let coupling = Coupling {
        knowledge: input
            .envelope
            .coupling
            .knowledge
            .as_deref()
            .map(cid::Cid::from_str)
            .transpose()
            .map_err(|e| StorageError::InvalidInput(format!("bad knowledge cid: {e}")))?,
        value: input
            .envelope
            .coupling
            .value
            .as_deref()
            .map(cid::Cid::from_str)
            .transpose()
            .map_err(|e| StorageError::InvalidInput(format!("bad value cid: {e}")))?,
        governance: input
            .envelope
            .coupling
            .governance
            .as_deref()
            .map(cid::Cid::from_str)
            .transpose()
            .map_err(|e| StorageError::InvalidInput(format!("bad governance cid: {e}")))?,
    };

    let claims = input
        .envelope
        .claims
        .iter()
        .map(|s| cid::Cid::from_str(s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::InvalidInput(format!("bad claims cid: {e}")))?;

    let supersedes = input
        .envelope
        .supersedes
        .as_deref()
        .map(cid::Cid::from_str)
        .transpose()
        .map_err(|e| StorageError::InvalidInput(format!("bad supersedes cid: {e}")))?;

    let sig_bytes = hex::decode(&input.envelope.proof.signature)
        .map_err(|e| StorageError::InvalidInput(format!("bad signature hex: {e}")))?;
    if sig_bytes.len() != 64 {
        return Ok(response::bad_request("signature must decode to 64 bytes"));
    }

    // issued_at is an RFC3339 String in the wire view. Parse for the Envelope.
    let issued_at = chrono::DateTime::parse_from_rfc3339(&input.envelope.issued_at)
        .map_err(|e| StorageError::InvalidInput(format!("bad issuedAt: {e}")))?
        .with_timezone(&chrono::Utc);

    let envelope = Envelope {
        cid,
        kind,
        schema_ref,
        schema_key: input.envelope.schema_key,
        reach,
        coupling,
        claims,
        supersedes,
        superseded_by: None, // server-derived
        issued_at,
        proof: Signature::ed25519(signer, sig_bytes),
    };

    // Capture `signer` field BEFORE the envelope is moved into Epr.
    // Used for local-origin dedup in the T19 FeedbackSignal fan-out below.
    let envelope_signer = input.envelope.proof.signer.clone();

    // Decode payload bytes once; shared by Epr construction and fan-out decode.
    let raw_payload = hex::decode(&input.payload)
        .map_err(|e| StorageError::InvalidInput(format!("bad payload hex: {e}")))?;

    let epr = Epr {
        envelope,
        payload: raw_payload.clone(),
    };

    // D.4: pass pool so FederatedEprStore can resolve signer_is_known_agent.
    // local_agent_cid is None until the conductor signing client is wired (see TODO).
    // D.7: pass local PeerId for self-dedup consistency with get_providers.
    let store = default_epr_store(
        swarm_tx,
        Some(pool.clone()),
        None,
        ctx.local_libp2p_peer_id.clone(),
    );
    let mut conn = get_conn(pool)?;
    let result = store.put(&mut conn, epr)?;

    // Phase 2 graph-native fan-out: after diesel commits, project EprHead into CozoDB.
    // Sequential semantics: diesel first, graph second (spec §6.2). Graph errors are
    // non-fatal — logged at warn level and never bubbled to the HTTP response.
    //
    // EprHead is decoded from payload_bytes (Content-kind EPRs carry the EprHead JSON as
    // payload). Other kinds (EconomicEvent, FeedbackSignal, etc.) have non-EprHead payloads
    // and are silently skipped on decode failure — they don't belong in the content graph.
    #[cfg(feature = "graph-native")]
    if kind == EprKind::Content {
        if let Some(engine) = fan_out_ctx.as_ref().and_then(|f| f.graph_engine.as_ref()) {
            match crate::db::epr_atoms::fetch_atom_by_cid(&mut conn, &result.cid) {
                Ok(Some(atom)) => {
                    match crate::epr_codec::decode_epr_head(&atom.payload_bytes) {
                        Ok(head) => {
                            let projector = crate::graph::projector::GraphProjector::new(engine);
                            if let Err(e) = projector.project_head(&result.cid, &head) {
                                tracing::warn!(cid = %result.cid, err = %e, "put_epr: graph projection failed (non-fatal)");
                            }
                            // If this atom supersedes a predecessor, write the SUPERSEDES edge.
                            if let Some(ref predecessor) = atom.supersedes {
                                if let Err(e) =
                                    projector.project_supersedence(predecessor, &result.cid)
                                {
                                    tracing::warn!(predecessor = %predecessor, successor = %result.cid, err = %e, "put_epr: graph supersedence edge failed (non-fatal)");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(cid = %result.cid, err = %e, "put_epr: EprHead decode from payload_bytes failed (non-fatal)");
                        }
                    }
                }
                Ok(None) => {
                    tracing::warn!(cid = %result.cid, "put_epr: atom not found after put — graph projection skipped");
                }
                Err(e) => {
                    tracing::warn!(cid = %result.cid, err = %e, "put_epr: fetch for graph projection failed (non-fatal)");
                }
            }
        }
    }

    // T18 / T22 wiring landed: see p2p/mod.rs::handle_epr_atom_request —
    // Content-kind Announce ingests record sender PeerId via
    // services::back_prop::record_predecessor (W2A, closes T18 and T22).
    //
    // HTTP put_epr (this function) does NOT record predecessors because the
    // caller is the local HTTP client, not a remote peer; back-prop is for
    // cross-peer content provenance. The sender PeerId is only available at
    // the libp2p protocol boundary (EprAtomRequest::Announce handler).

    // Phase 3.5 — Light Up the Graph T19: FeedbackSignal arrival fan-out.
    //
    // When a FeedbackSignal EPR arrives from a REMOTE peer (not our own message
    // looping back), run three downstream effects:
    //   1. project_signal  — update standing_view (DB write, non-fatal on error)
    //   2. back_prop_one_hop — forward signal to sealed predecessors (best-effort)
    //   3. flood_feedback  — gossip-flood to content reach topic (best-effort)
    //
    // All three steps are skipped when their runtime dependency is None (T22 not yet
    // wired). DB errors from step 1 are logged and continued; network errors from
    // steps 2-3 are logged at warn level and never bubble to the HTTP response.
    if kind == EprKind::FeedbackSignal {
        let local_peer_id_for_dedup = fan_out_ctx
            .as_ref()
            .and_then(|f| f.local_peer_id.as_deref())
            .unwrap_or("");

        if !is_local_origin(&envelope_signer, local_peer_id_for_dedup)
            || local_peer_id_for_dedup.is_empty()
        {
            // Decode payload as FeedbackSignal via rmp_serde (MessagePack).
            // Per memory: use rmp_serde, never serde_json::Value, to avoid
            // SerializedBytes boundary corruption.
            match rmp_serde::from_slice::<crate::p2p::feedback_signal::FeedbackSignal>(&raw_payload)
            {
                Ok(signal) => {
                    // 1. project_signal — non-fatal, transactional with existing conn.
                    if let Some(fan_out) = fan_out_ctx.as_ref() {
                        if let (Some(registry), Some(local_pubkey)) = (
                            fan_out.manifest_registry.as_ref(),
                            fan_out.local_pubkey.as_ref(),
                        ) {
                            let policy = crate::services::standing_projector::ManifestDebitWeightPolicy::from_registry(registry);
                            let manifest_cid = fan_out
                                .standing_policy_cid
                                .as_deref()
                                .unwrap_or("bootstrap");
                            if let Err(e) = crate::services::standing_projector::project_signal(
                                &mut conn,
                                &policy,
                                local_pubkey.as_slice(),
                                &signal,
                                manifest_cid,
                            ) {
                                tracing::warn!(
                                    ?e,
                                    %path_cid,
                                    "put_epr: project_signal failed (non-fatal)"
                                );
                            }
                        }

                        // 2. back_prop_one_hop — best-effort, unseals predecessor records
                        //    and forwards signal one hop upstream. Skipped if unsealing
                        //    keys or outbound sink are absent.
                        if let (Some(bundle), Some(sink)) = (
                            fan_out.unsealing_keys.as_ref(),
                            fan_out.outbound_sink.as_ref(),
                        ) {
                            let unseal = crate::services::back_prop::UnsealingKeys {
                                mishpat_pk: &bundle.mishpat_pk,
                                mishpat_sk: &bundle.mishpat_sk,
                                imagodei_pk: &bundle.imagodei_pk,
                                imagodei_sk: &bundle.imagodei_sk,
                            };
                            let local_id = fan_out.local_peer_id.as_deref();
                            match crate::services::back_prop::back_prop_one_hop(
                                &mut conn,
                                &signal,
                                sink.as_ref(),
                                &unseal,
                                local_id,
                            ) {
                                Ok(forwarded) if !forwarded.is_empty() => {
                                    tracing::debug!(
                                        %path_cid,
                                        peers = forwarded.len(),
                                        "put_epr: back_prop_one_hop forwarded signal"
                                    );
                                }
                                Ok(_) => {} // no predecessors recorded or origin
                                Err(e) => {
                                    tracing::warn!(
                                        ?e,
                                        %path_cid,
                                        "put_epr: back_prop_one_hop failed (non-fatal)"
                                    );
                                }
                            }
                        }

                        // 3. flood_feedback — best-effort publish on content reach topic.
                        if let Some(publisher) = fan_out.gossip_publisher.as_ref() {
                            let topic = format!("/elohim/feedback-signal/{}", signal.target_cid);
                            // signal_cid: derive a stable identifier from the EPR CID we just ingested.
                            if let Err(e) = crate::services::gossip_flood::flood_feedback(
                                &signal,
                                path_cid,
                                &topic,
                                publisher.as_ref(),
                            ) {
                                tracing::warn!(
                                    ?e,
                                    %path_cid,
                                    "put_epr: flood_feedback failed (non-fatal)"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        ?e,
                        %path_cid,
                        "put_epr: FeedbackSignal payload decode failed — skipping fan-out"
                    );
                }
            }
        } else {
            tracing::debug!(
                %path_cid,
                "put_epr: FeedbackSignal local-origin — skipping fan-out"
            );
        }
    }

    // Idempotent: 200 on both new and exact-match re-put. Mismatched bytes under
    // the same CID are rejected as InvalidInput by LocalEprStore::put.
    let body = serde_json::to_vec(&result)
        .map_err(|e| StorageError::Database(format!("serialize: {e}")))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

// ---------------------------------------------------------------------------
// GET /api/v1/epr  (list with filters + pagination)
// ---------------------------------------------------------------------------

async fn list_epr(
    req: Request<Incoming>,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    use crate::db::epr_atoms::EprListQuery;

    let query = req.uri().query().unwrap_or("");

    let caller_authed = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    let mut list_query = EprListQuery {
        limit: 50,
        ..Default::default()
    };

    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("kind=") {
            list_query.kind = Some(v.into());
        } else if let Some(v) = kv.strip_prefix("reach=") {
            list_query.reach = Some(v.into());
        } else if let Some(v) = kv.strip_prefix("schemaRef=") {
            list_query.schema_ref = Some(v.into());
        } else if let Some(v) = kv.strip_prefix("after=") {
            list_query.after_cid = Some(v.into());
        } else if let Some(v) = kv.strip_prefix("limit=") {
            if let Ok(n) = v.parse::<i64>() {
                list_query.limit = n.clamp(1, 200);
            }
        }
    }

    // Unauthed callers: restrict to commons/public.
    if !caller_authed {
        if let Some(r) = &list_query.reach {
            if !matches!(r.as_str(), "commons" | "public") {
                // Caller asked for a restricted reach they cannot access — return empty.
                return Ok(response::ok(&crate::views::EprListView {
                    items: vec![],
                    next_cursor: None,
                }));
            }
        } else {
            // No reach filter supplied — default to commons so unauthenticated callers
            // only see public-domain EPRs. Authed callers see everything by default.
            list_query.reach = Some("commons".into());
        }
    }

    let store = default_epr_store(None, None, None, None);
    let mut conn = get_conn(pool)?;
    let (atoms, next_cursor) = store.list(&mut conn, &list_query)?;

    // Rehydrate each atom to build EprEnvelopeView (N+1 is acceptable for Phase 2a;
    // page size is clamped to 200, and a joined query can land in Phase 2b if needed).
    let mut items: Vec<EprEnvelopeView> = Vec::with_capacity(atoms.len());
    for atom in &atoms {
        if let Some(outcome) = store.fetch(&mut conn, &atom.cid)? {
            items.push(to_envelope_view(&outcome.fetched));
        }
    }

    Ok(response::ok(&crate::views::EprListView {
        items,
        next_cursor,
    }))
}

// ---------------------------------------------------------------------------
// T17 unit tests — local-origin dedup helper
// ---------------------------------------------------------------------------

#[cfg(test)]
mod dedup_tests {
    use super::*;

    #[test]
    fn local_origin_match() {
        assert!(
            is_local_origin("12D3KooWXyz", "12D3KooWXyz"),
            "same peer ID should be identified as local origin"
        );
    }

    #[test]
    fn external_origin_no_match() {
        assert!(
            !is_local_origin("12D3KooWXyz", "12D3KooWQRS"),
            "different peer IDs should not be identified as local origin"
        );
    }

    #[test]
    fn empty_local_peer_id_is_not_local_origin() {
        // When local_peer_id is empty (T22 not yet wired), we treat the message
        // as external so fan-out is not skipped inadvertently.
        assert!(
            !is_local_origin("12D3KooWXyz", ""),
            "non-empty signed_by vs empty local_peer_id should NOT be local origin"
        );
    }
}
