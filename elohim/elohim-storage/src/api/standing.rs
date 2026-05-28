//! Standing query HTTP controller — Phase 3.5 P3.5.9 + Sprint 2 T9.
//!
//! Routes:
//!   - `GET /api/v1/standing/compose-context?subject=<base64-pubkey>`
//!   - `GET /api/v1/standing/{agent_cid}?evaluator=<cid>`
//!
//! Read-only. No writes; no DHT round-trip; sub-50ms p99 target.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bytes::Bytes;
use chrono::Utc;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::services::standing::{Standing, StandingScore};
use crate::services::standing_query::compose_context;
use elohim_views::standing::{FeedbackSignalSummary, StandingScoreTier, StandingScoreView};

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
        // Must be matched before the agent_cid catch-all below.
        (&Method::GET, p) if p.starts_with("compose-context") => {
            get_compose_context(req, pool, ctx).await
        }

        // GET /api/v1/standing/<agent_cid>?evaluator=<cid>
        // Sprint 2 Task 9: per-evaluator pluralism read projection.
        (&Method::GET, p) if !p.is_empty() => {
            get_agent_standing(req, p, pool, ctx).await
        }

        (&Method::GET, _) => Ok(response::not_found(
            "agent_cid path segment required: GET /api/v1/standing/<agent_cid>?evaluator=<cid>",
        )),

        _ => Ok(response::method_not_allowed()),
    }
}

// ============================================================================
// Handler
// ============================================================================

// ============================================================================
// Handler — agent standing (Sprint 2 T9)
// ============================================================================

/// `GET /api/v1/standing/{agent_cid}?evaluator=<cid>`
///
/// Returns the `StandingScoreView` (T1 wire shape) for the (evaluator, subject)
/// pair. Read-only projection of the `standing_view` SQLite table; recomputable
/// from FeedbackSignal evidence (Holochain DHT) via `standing_projector`.
///
/// Pluralism property: standing is always per-evaluator. The `?evaluator=<cid>`
/// query parameter is mandatory — omitting it returns 400.
///
/// `recent_breaches` is returned as an empty vec for Sprint 2. Sprint 8 audit
/// UI will require a JOIN against `feedback_signals` to populate it.
async fn get_agent_standing(
    req: Request<Incoming>,
    agent_cid: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query_str = req.uri().query().unwrap_or("");
    let evaluator_cid = match parse_evaluator_query(query_str) {
        Some(c) => c,
        None => {
            return Ok(response::bad_request(
                "?evaluator=<cid> query parameter required",
            ));
        }
    };

    let evaluator_bytes = evaluator_cid.as_bytes().to_vec();
    let subject_bytes = agent_cid.as_bytes().to_vec();

    let pool = pool.clone();
    let view: StandingScoreView =
        tokio::task::spawn_blocking(move || -> Result<StandingScoreView, StorageError> {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Database(e.to_string()))?;

            let standing = Standing::evaluate(&evaluator_bytes, &subject_bytes, &mut conn);
            let row = crate::db::standing_view::fetch(&mut conn, &evaluator_bytes, &subject_bytes)
                .map_err(|e| StorageError::Database(e.to_string()))?;

            let tier = match standing {
                Standing::Unknown => StandingScoreTier::Unknown,
                Standing::Computed { score } => match score {
                    StandingScore::Floor => StandingScoreTier::Floor,
                    StandingScore::Low => StandingScoreTier::Low,
                    StandingScore::Neutral => StandingScoreTier::Neutral,
                    StandingScore::High => StandingScoreTier::High,
                    StandingScore::Trusted => StandingScoreTier::Trusted,
                },
            };

            Ok(StandingScoreView {
                evaluator_cid: String::from_utf8(evaluator_bytes.clone())
                    .unwrap_or_default(),
                subject_cid: String::from_utf8(subject_bytes.clone()).unwrap_or_default(),
                score: tier,
                debit_weight_sum: row.as_ref().map(|r| r.debit_weight_sum).unwrap_or(0),
                recent_breaches: Vec::<FeedbackSignalSummary>::new(),
                computed_at: row
                    .as_ref()
                    .map(|r| r.last_signal_at.clone())
                    .unwrap_or_else(|| Utc::now().to_rfc3339()),
            })
        })
        .await
        .map_err(|e| StorageError::Internal(e.to_string()))??;

    let body =
        serde_json::to_vec(&view).map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(Response::builder()
        .status(hyper::StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}

/// Parse `?evaluator=<value>` from a raw query string.
/// Returns `Some(value)` only when the key is present and the value is non-empty.
fn parse_evaluator_query(query_str: &str) -> Option<String> {
    for pair in query_str.split('&') {
        let mut parts = pair.splitn(2, '=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            if k == "evaluator" && !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

// ============================================================================
// Handler — compose context
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
