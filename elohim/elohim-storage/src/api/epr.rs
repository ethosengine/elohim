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

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::epr_service::FetchedEpr;
use crate::services::epr_store::{default_epr_store, EprStore};
use crate::services::response;
use crate::views::{EprCouplingView, EprEnvelopeView, EprSignatureView, EprView};

use super::get_conn;

// ---------------------------------------------------------------------------
// Dispatcher — matches existing controller signature convention
// ---------------------------------------------------------------------------

/// Handle `/api/v1/epr*` requests.
///
/// `swarm_tx` is threaded from `HttpServer` so that `PUT /api/v1/epr/:cid`
/// can issue `KadStartProviding` after a successful local put (D.2). When
/// `None` (no P2P swarm configured) Kad advertisement is silently skipped.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
    swarm_tx: Option<tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Normalise: strip leading slash, giving us "", "abc123", "abc123/envelope" …
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/epr
        (&Method::GET, "") => list_epr(req, pool, ctx).await,

        // PUT /api/v1/epr/:cid  (content-addressed idempotent put)
        (&Method::PUT, cid) if !cid.is_empty() && !cid.contains('/') => {
            put_epr(req, cid, pool, ctx, swarm_tx).await
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
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let include_canonical = req
        .uri()
        .query()
        .map(|q| q.contains("includeCanonical=true"))
        .unwrap_or(false);

    // TODO(phase-2b): wire ctx.local_libp2p_peer_id once fetch dedup is needed
    let store = default_epr_store(None, None, None, None);
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
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // TODO(phase-2b): wire ctx.local_libp2p_peer_id once fetch dedup is needed
    let store = default_epr_store(None, None, None, None);
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
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // TODO(phase-2b): wire ctx.local_libp2p_peer_id once fetch dedup is needed
    let store = default_epr_store(None, None, None, None);
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
    _ctx: &AppContext,
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

    // TODO(phase-2b): wire ctx.local_libp2p_peer_id once verify dedup is needed
    let store = default_epr_store(None, None, None, None);
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

    let payload = hex::decode(&input.payload)
        .map_err(|e| StorageError::InvalidInput(format!("bad payload hex: {e}")))?;

    let epr = Epr { envelope, payload };

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
    // TODO(phase-2b): wire ctx.local_libp2p_peer_id if list gains peer-aware filtering

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
