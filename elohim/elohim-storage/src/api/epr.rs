//! EPR REST controller — routes under /api/v1/epr.
//!
//! See Integrator Compatibility Contract §2.2.
//! Controller → EprService → db::epr_atoms.
//!
//! Route table:
//!   GET  /api/v1/epr/:cid              → get_epr
//!   GET  /api/v1/epr/:cid/envelope     → get_envelope  (Task 13)
//!   GET  /api/v1/epr/:cid/payload      → get_payload   (Task 14)
//!   GET  /api/v1/epr/:cid/verify       → get_verify    (Task 15)
//!   GET  /api/v1/epr                   → list_epr      (Task 16)
//!   POST /api/v1/epr                   → post_epr      (Task 17)

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::epr_service::{self, FetchedEpr};
use crate::services::response;
use crate::views::{EprCouplingView, EprEnvelopeView, EprSignatureView, EprView};

use super::get_conn;

// ---------------------------------------------------------------------------
// Dispatcher — matches existing controller signature convention
// ---------------------------------------------------------------------------

/// Handle `/api/v1/epr*` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Normalise: strip leading slash, giving us "", "abc123", "abc123/envelope" …
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/epr
        (&Method::GET, "") => list_epr(req, pool, ctx).await,

        // POST /api/v1/epr
        (&Method::POST, "") => post_epr(req, pool, ctx).await,

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

    let mut conn = get_conn(pool)?;
    let Some(fetched) = epr_service::fetch_by_cid(&mut conn, cid)? else {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    };

    if !reach_visible_to(&fetched.atom.reach, &req) {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    }

    let view = to_epr_view(&fetched, include_canonical);
    Ok(response::ok(&view))
}

// ---------------------------------------------------------------------------
// Stubs for Tasks 13–17 (replaced in subsequent commits)
// ---------------------------------------------------------------------------

async fn get_envelope(
    req: Request<Incoming>,
    cid: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let Some(fetched) = epr_service::fetch_by_cid(&mut conn, cid)? else {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    };
    if !reach_visible_to(&fetched.atom.reach, &req) {
        return Ok(response::not_found(&format!("epr not found: {cid}")));
    }
    Ok(response::ok(&to_envelope_view(&fetched)))
}

async fn get_payload(
    _req: Request<Incoming>,
    _cid: &str,
    _pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    Ok(response::not_found("not implemented — Task 14")) // Task 14
}

async fn get_verify(
    _req: Request<Incoming>,
    _cid: &str,
    _pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    Ok(response::not_found("not implemented — Task 15")) // Task 15
}

async fn list_epr(
    _req: Request<Incoming>,
    _pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    Ok(response::not_found("not implemented — Task 16")) // Task 16
}

async fn post_epr(
    _req: Request<Incoming>,
    _pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    Ok(response::not_found("not implemented — Task 17")) // Task 17
}
