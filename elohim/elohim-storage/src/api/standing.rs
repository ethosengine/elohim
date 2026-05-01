//! Standing query HTTP controller — Phase 3.5 P3.5.9.
//!
//! Route: `GET /api/v1/standing/compose-context?subject=<base64-pubkey>`
//!
//! Read-only. Returns [`ComposeContext`] for the compose-time elohim tender.
//! No writes; no DHT round-trip; sub-50ms p99 target.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::services::standing_query::compose_context;

use super::get_conn;

// ============================================================================
// Dispatcher
// ============================================================================

/// Handle `GET /api/v1/standing/*` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/standing/compose-context?subject=<base64>
        (&Method::GET, p) if p.starts_with("compose-context") => {
            get_compose_context(req, pool, ctx).await
        }

        (&Method::GET, _) => Ok(response::not_found(&format!(
            "Unknown standing route: GET /api/v1/standing/{}",
            path
        ))),

        _ => Ok(response::method_not_allowed()),
    }
}

// ============================================================================
// Handler
// ============================================================================

/// `GET /api/v1/standing/compose-context?subject=<base64-pubkey>`
///
/// Returns `ComposeContext` for the elohim tender compose-time conversation.
/// The evaluator is the local peer (derived from `AppContext.local_libp2p_peer_id`
/// if set; falls back to 32 zero-bytes for single-tenant / cold-start contexts).
async fn get_compose_context(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Parse `subject` query parameter (base64-encoded pubkey bytes).
    let query = req.uri().query().unwrap_or("");
    let subject_b64 = url::form_urlencoded::parse(query.as_bytes())
        .find(|(k, _)| k == "subject")
        .map(|(_, v)| v.into_owned());

    let subject_b64 = match subject_b64 {
        Some(s) => s,
        None => {
            return Ok(response::bad_request("missing 'subject' query parameter"));
        }
    };

    let subject = match BASE64.decode(&subject_b64) {
        Ok(b) => b,
        Err(_) => {
            return Ok(response::bad_request("invalid base64 in 'subject'"));
        }
    };

    // Evaluator: use the local libp2p peer id bytes if available; otherwise
    // fall back to 32 zero-bytes (single-tenant / dev default). The peer-id
    // string is stored as ASCII so `.as_bytes()` gives the raw string bytes —
    // consistent with how it is used throughout the standing_view queries.
    let evaluator: Vec<u8> = ctx
        .local_libp2p_peer_id
        .as_ref()
        .map(|s| s.as_bytes().to_vec())
        .unwrap_or_else(|| vec![0u8; 32]);

    let mut conn = get_conn(pool)?;
    let result = compose_context(&mut conn, &evaluator, &subject)
        .map_err(|e| StorageError::Internal(format!("standing query: {e}")))?;

    Ok(response::ok(&result))
}
