//! Identity API controller
//!
//! Routes: `/api/v1/identity/register`, `/api/v1/identity/me`
//!
//! The "me" endpoints resolve the current human via the `X-Agent-Id` header,
//! which doorway injects after JWT validation. In Tauri/direct mode the app
//! sends the same header. If the header is absent, the active local session's
//! agent_pub_key is used as a fallback.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::humans::{CreateHumanInput, UpdateHumanInput};
use crate::db::{humans, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{CreateHumanInputView, HumanView, UpdateHumanInputView};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Route handler
// ---------------------------------------------------------------------------

/// Dispatch `/api/v1/identity/*` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    match (method, resource_path) {
        (Method::POST, "/register") => register_human(req, pool).await,
        (Method::GET, "/me") => get_me(req, pool).await,
        (Method::PUT, "/me") => update_me(req, pool).await,
        _ => Ok(response::not_found(&format!(
            "Unknown identity route: {}",
            resource_path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/identity/register
///
/// Create a human identity record. The caller supplies a stable `id`
/// (typically a UUID derived from the agent public key) and optional profile
/// fields.
async fn register_human(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: CreateHumanInputView = parse_body(req).await?;

    let affinities_json = serde_json::to_string(&body.affinities)
        .map_err(|e| StorageError::InvalidInput(format!("Invalid affinities: {}", e)))?;

    let input = CreateHumanInput {
        id: body.id,
        agent_pub_key: body.agent_pub_key,
        display_name: body.display_name,
        bio: body.bio,
        affinities: affinities_json,
        profile_reach: body.profile_reach,
        location: body.location,
        profile_photo_url: body.profile_photo_url,
        app_id: "imagodei".to_string(),
    };

    let mut conn = get_conn(pool)?;
    let human = humans::create_human(&mut conn, input)?;
    Ok(response::created(&HumanView::from(human)))
}

/// GET /api/v1/identity/me
///
/// Fetch the human record for the currently authenticated agent.
/// Resolution order:
/// 1. `X-Agent-Id` header (set by doorway JWT middleware)
/// 2. Active local session's `agent_pub_key` (Tauri fallback)
async fn get_me(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    let agent_key = extract_agent_key(&req, &mut conn)?;

    match agent_key {
        Some(key) => {
            let result = humans::get_human_by_agent_key(&mut conn, &key)?;
            Ok(response::from_option(
                Ok(result.map(HumanView::from)),
                "No human record found for current agent",
            ))
        }
        None => Ok(response::bad_request(
            "Cannot resolve current identity: no X-Agent-Id header and no active session",
        )),
    }
}

/// PUT /api/v1/identity/me
///
/// Update mutable profile fields for the currently authenticated agent.
async fn update_me(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    let agent_key = extract_agent_key(&req, &mut conn)?;
    let agent_key = agent_key.ok_or_else(|| {
        StorageError::InvalidInput(
            "Cannot resolve current identity: no X-Agent-Id header and no active session"
                .to_string(),
        )
    })?;

    // Resolve human id from agent key before consuming `req` into body parse
    let human = humans::get_human_by_agent_key(&mut conn, &agent_key)?.ok_or_else(|| {
        StorageError::NotFound("No human record found for current agent".to_string())
    })?;
    let human_id = human.id.clone();

    let body: UpdateHumanInputView = parse_body(req).await?;

    let affinities_json = body
        .affinities
        .map(|a| {
            serde_json::to_string(&a)
                .map_err(|e| StorageError::InvalidInput(format!("Invalid affinities: {}", e)))
        })
        .transpose()?;

    let input = UpdateHumanInput {
        display_name: body.display_name,
        bio: body.bio,
        affinities: affinities_json,
        profile_reach: body.profile_reach,
        location: body.location,
        profile_photo_url: body.profile_photo_url,
    };

    let updated = humans::update_human(&mut conn, &human_id, input)?;
    Ok(response::ok(&HumanView::from(updated)))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the current agent public key from the request.
///
/// Returns `Ok(Some(key))` when resolved, `Ok(None)` when no identity
/// signal is present (caller decides whether that is an error).
fn extract_agent_key(
    req: &Request<Incoming>,
    conn: &mut diesel::SqliteConnection,
) -> Result<Option<String>, StorageError> {
    // 1. Prefer the header doorway injects after JWT validation
    if let Some(key) = req
        .headers()
        .get("X-Agent-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        return Ok(Some(key));
    }

    // 2. Tauri fallback: use the active local session
    if let Some(session) = crate::db::local_sessions::get_active_session(conn)? {
        return Ok(Some(session.agent_pub_key));
    }

    Ok(None)
}
