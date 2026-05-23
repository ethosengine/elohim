//! Qahal API controller — Multi-collective collaboration EPR M1
//!
//! Routes:
//!   POST   /api/v1/collective                      — create first-order Collective
//!   GET    /api/v1/collective/{cid}                — fetch Collective by CID
//!   POST   /api/v1/collab/agreement                — create CollabAgreement
//!   POST   /api/v1/collab/agreement/{cid}/attest   — attest CollabAgreement
//!   GET    /api/v1/collab/{cid}                    — fetch Collab-Qahal view
//!   POST   /api/v1/collab/{cid}/withdraw           — withdraw Membership (clean-exit)
//!
//! ## Auth gating
//! Write routes require the caller to carry an `X-Agent-Id` header (doorway
//! JWT injection pattern).  The handler does NOT enforce this directly — the
//! doorway manifest declares `auth_required()` and doorway rejects unauthenticated
//! requests before they reach storage.  This is the same pattern as
//! `api/agreements.rs` and every other write route in this crate.
//!
//! ## Service availability
//! All routes require the `imagodei` HcClient to be wired in
//! `HcClientRegistry.imagodei`.  When `None`, handlers return
//! `503 IMAGODEI_BRIDGE_OFFLINE` — same machine-readable code as `api/account.rs`.
//!
//! ## Path note
//! The sub-path prefix passed into `handle()` is the string AFTER the
//! `/api/v1/` prefix has been stripped by `handle_api_request`.  Two prefixes
//! are dispatched here: `"collective"` and `"collab"`.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::error::StorageError;
use crate::hc_client_registry::HcClientRegistry;
use crate::services::qahal_service::QahalService;
use crate::services::response;

use elohim_views::{
    AttestCollabAgreementInputView, CreateCollabAgreementInputView,
    CreateCollabCollectiveInputView, WithdrawMembershipInputView,
};

use super::parse_body;

// =============================================================================
// Route dispatchers — two entry points, one for each URL prefix
// =============================================================================

/// Handle `/api/v1/collective*` requests.
///
/// `resource_path` is the portion after the `"collective"` prefix,
/// e.g. `""` for `POST /api/v1/collective` or `"/uhCkk...abc"` for
/// `GET /api/v1/collective/{cid}`.
pub async fn handle_collective(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // POST /api/v1/collective
        (&Method::POST, "") => handle_create_collective(req, hc_registry).await,

        // GET /api/v1/collective/{cid}
        (&Method::GET, cid) if !cid.is_empty() && !cid.contains('/') => {
            handle_get_collective(cid, hc_registry).await
        }

        _ => Ok(response::not_found(&format!(
            "Unknown collective route: {} /api/v1/collective/{}",
            method, path
        ))),
    }
}

/// Handle `/api/v1/collab*` requests.
///
/// `resource_path` is the portion after the `"collab"` prefix.
/// Examples:
///   `"/agreement"`               → POST create_collab_agreement
///   `"/agreement/{cid}/attest"`  → POST attest_collab_agreement
///   `"/{cid}"`                   → GET  fetch_collab_qahal
///   `"/{cid}/withdraw"`          → POST withdraw_membership
pub async fn handle_collab(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    // Must match the longer `/agreement/{cid}/attest` before `/agreement`
    // to avoid the attest path matching the create path when split.
    if let Some(rest) = path.strip_prefix("agreement/") {
        // POST /api/v1/collab/agreement/{cid}/attest
        if let Some(cid) = rest.strip_suffix("/attest") {
            if method == Method::POST {
                return handle_attest_collab_agreement(req, cid, hc_registry).await;
            }
        }
        return Ok(response::not_found(&format!(
            "Unknown collab/agreement route: {} /api/v1/collab/{}",
            method, path
        )));
    }

    match (&method, path) {
        // POST /api/v1/collab/agreement
        (&Method::POST, "agreement") => handle_create_collab_agreement(req, hc_registry).await,

        // GET /api/v1/collab/{cid}  — must not match "agreement" (handled above)
        (&Method::GET, cid) if !cid.is_empty() && !cid.contains('/') => {
            handle_get_collab(cid, hc_registry).await
        }

        // POST /api/v1/collab/{cid}/withdraw
        (&Method::POST, rest) if rest.contains('/') => {
            let mut parts = rest.splitn(2, '/');
            let cid = parts.next().unwrap_or("");
            let action = parts.next().unwrap_or("");
            if action == "withdraw" && !cid.is_empty() {
                handle_withdraw_membership(req, cid, hc_registry).await
            } else {
                Ok(response::not_found(&format!(
                    "Unknown collab route: {} /api/v1/collab/{}",
                    method, path
                )))
            }
        }

        _ => Ok(response::not_found(&format!(
            "Unknown collab route: {} /api/v1/collab/{}",
            method, path
        ))),
    }
}

// =============================================================================
// Handler implementations
// =============================================================================

/// POST /api/v1/collective
async fn handle_create_collective(
    req: Request<Incoming>,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let svc = match require_qahal_service(hc_registry) {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };
    let input: CreateCollabCollectiveInputView = parse_body(req).await?;
    match svc.create_collective(input).await {
        Ok(view) => Ok(response::created(&view)),
        Err(e) => Ok(map_zome_err(e)),
    }
}

/// GET /api/v1/collective/{cid}
async fn handle_get_collective(
    cid: &str,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let svc = match require_qahal_service(hc_registry) {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };
    match svc.fetch_collective(cid).await {
        Ok(view) => Ok(response::ok(&view)),
        Err(StorageError::NotFound(msg)) => Ok(response::not_found(&msg)),
        Err(e) => Ok(map_zome_err(e)),
    }
}

/// POST /api/v1/collab/agreement
async fn handle_create_collab_agreement(
    req: Request<Incoming>,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let svc = match require_qahal_service(hc_registry) {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };
    let input: CreateCollabAgreementInputView = parse_body(req).await?;
    match svc.create_collab_agreement(input).await {
        Ok(view) => Ok(response::created(&view)),
        Err(e) => Ok(map_zome_err(e)),
    }
}

/// POST /api/v1/collab/agreement/{cid}/attest
///
/// The path parameter `path_cid` carries the agreement CID extracted from the URL.
/// The request body also carries `agreementCid` — both must agree; a mismatch
/// returns 400 to guard against copy-paste attacks.
async fn handle_attest_collab_agreement(
    req: Request<Incoming>,
    path_cid: &str,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let svc = match require_qahal_service(hc_registry) {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };
    let input: AttestCollabAgreementInputView = parse_body(req).await?;

    // Guard: path CID and body CID must agree.
    if !path_cid.is_empty() && input.agreement_cid != path_cid {
        return Ok(response::bad_request(&format!(
            "path cid '{}' does not match body agreementCid '{}'",
            path_cid, input.agreement_cid
        )));
    }

    match svc.attest_collab_agreement(input).await {
        Ok(view) => Ok(response::ok(&view)),
        Err(StorageError::NotFound(msg)) => Ok(response::not_found(&msg)),
        Err(e) => Ok(map_zome_err(e)),
    }
}

/// GET /api/v1/collab/{cid}
async fn handle_get_collab(
    cid: &str,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let svc = match require_qahal_service(hc_registry) {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };
    match svc.fetch_collab_qahal(cid).await {
        Ok(view) => Ok(response::ok(&view)),
        Err(StorageError::NotFound(msg)) => Ok(response::not_found(&msg)),
        Err(e) => Ok(map_zome_err(e)),
    }
}

/// POST /api/v1/collab/{cid}/withdraw
///
/// The `path_cid` is the Collab-Qahal CID from the URL path.  The request body
/// carries the full `WithdrawMembershipInputView`.  Per spec §6.4 the body is
/// authoritative for `membershipCid` and `collabQahalCid`; the path CID is an
/// additional URL-scope guard (must match `collabQahalCid`).
async fn handle_withdraw_membership(
    req: Request<Incoming>,
    path_cid: &str,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let svc = match require_qahal_service(hc_registry) {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };
    let input: WithdrawMembershipInputView = parse_body(req).await?;

    // Guard: path CID and body collabQahalCid must agree.
    if !path_cid.is_empty() && input.collab_qahal_cid != path_cid {
        return Ok(response::bad_request(&format!(
            "path cid '{}' does not match body collabQahalCid '{}'",
            path_cid, input.collab_qahal_cid
        )));
    }

    match svc.withdraw_membership(input).await {
        Ok(view) => Ok(response::ok(&view)),
        Err(StorageError::NotFound(msg)) => Ok(response::not_found(&msg)),
        Err(e) => Ok(map_zome_err(e)),
    }
}

// =============================================================================
// Helper — resolve QahalService from registry
// =============================================================================

/// Resolve a `QahalService` from the HcClientRegistry's imagodei slot.
///
/// Returns `Ok(QahalService)` when the imagodei HcClient is wired in.
/// Returns `Err(Response<...>)` containing a `503 IMAGODEI_BRIDGE_OFFLINE`
/// response when the registry or imagodei slot is absent — same machine-readable
/// contract as `api/account.rs`.
///
/// Note: We return `Ok(QahalService { … })` on each call rather than holding
/// a cached `Arc<QahalService>` in `Services`.  The `HcClient` it wraps is
/// already behind an `Arc`, so no extra allocation cost is incurred in practice.
#[allow(clippy::result_large_err)]
fn require_qahal_service(
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<QahalService, Response<Full<Bytes>>> {
    let registry = match hc_registry {
        Some(r) => r,
        None => return Err(response_503_imagodei_bridge_offline()),
    };
    let hc = match registry.imagodei.clone() {
        Some(h) => h,
        None => return Err(response_503_imagodei_bridge_offline()),
    };
    Ok(QahalService::new(hc))
}

/// 503 response body for the IMAGODEI_BRIDGE_OFFLINE contract.
fn response_503_imagodei_bridge_offline() -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "imagodei bridge offline",
        "code": "IMAGODEI_BRIDGE_OFFLINE",
        "message": "The storage process did not connect to the imagodei coordinator \
                    zome at startup. Qahal write routes are unavailable until \
                    storage restarts with the imagodei DNA installed.",
    });
    response::json_response(hyper::StatusCode::SERVICE_UNAVAILABLE, &body)
}

// =============================================================================
// Helper — map zome errors to HTTP responses
// =============================================================================

/// Map a `StorageError` returned by `QahalService` to an appropriate HTTP response.
///
/// Most errors are `StorageError::Internal` wrapping a conductor message; those
/// surface as 500.  `StorageError::InvalidInput` maps to 400.
/// `StorageError::NotFound` is handled by callers before reaching here.
fn map_zome_err(err: StorageError) -> Response<Full<Bytes>> {
    match &err {
        StorageError::InvalidInput(msg) => {
            let body = serde_json::json!({
                "error": "bad request",
                "code": "INVALID_INPUT",
                "message": msg,
            });
            response::json_response(hyper::StatusCode::BAD_REQUEST, &body)
        }
        _ => {
            let body = serde_json::json!({
                "error": "internal error",
                "code": "ZOME_CALL_FAILED",
                "message": err.to_string(),
            });
            response::json_response(hyper::StatusCode::INTERNAL_SERVER_ERROR, &body)
        }
    }
}
