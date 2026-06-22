//! Identity API controller
//!
//! Routes:
//!   POST /api/v1/identity/register
//!   POST /api/v1/identity/heal                        → STOPGAP NULL-only identity heal (resolver spec §3.4)
//!   GET  /api/v1/identity/me
//!   PUT  /api/v1/identity/me
//!   GET  /api/v1/identity/{agentId}/session          → SessionHumanView (M-AGGR-1)
//!   GET  /api/v1/identity/{agentId}/upgrade-prompts  → UpgradePromptView (M-AGGR-1)
//!
//! The "me" endpoints resolve the current human via the `X-Agent-Id` header,
//! which doorway injects after JWT validation. In Tauri/direct mode the app
//! sends the same header. If the header is absent, the active local session's
//! agent_pub_key is used as a fallback.
//!
//! The {agentId} endpoints serve per-agent projections. The agentId is the
//! agent's pubkey (same format as X-Agent-Id). The caller must supply their
//! own agentId in the path; cross-agent projection reads are not scope-checked
//! at this layer (policy is enforced at the doorway JWT/auth boundary).

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::humans::{CreateHumanInput, UpdateHumanInput};
use crate::db::{humans, session_human_view, upgrade_prompt_view, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{CreateHumanInputView, HumanView, UpdateHumanInputView};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Route handler
// ---------------------------------------------------------------------------

/// Dispatch `/api/v1/identity/*` requests.
///
/// Pattern matching order matters — more-specific `/{agentId}/session` and
/// `/{agentId}/upgrade-prompts` patterns are matched before the catch-all.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Dispatch {agentId}/session and {agentId}/upgrade-prompts BEFORE the
    // fixed /register and /me routes to avoid false prefix matches.
    if method == Method::GET {
        if let Some(agent_id) = resource_path
            .strip_suffix("/session")
            .and_then(|p| p.strip_prefix('/'))
        {
            return get_agent_session(agent_id, pool, ctx).await;
        }
        if let Some(agent_id) = resource_path
            .strip_suffix("/upgrade-prompts")
            .and_then(|p| p.strip_prefix('/'))
        {
            return get_agent_upgrade_prompts(agent_id, pool, ctx).await;
        }
        if let Some(subject_id) = resource_path
            .strip_suffix("/profile")
            .and_then(|p| p.strip_prefix('/'))
        {
            return get_profile_lens(&req, subject_id, pool, ctx).await;
        }
    }

    match (method, resource_path) {
        (Method::POST, "/register") => register_human(req, pool).await,
        (Method::POST, "/heal") => heal_human(req, pool).await,
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
    // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
    // Currently null for direct storage writes. Backfill needed for pre-coherence data.
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
        h_app_id: "imagodei".to_string(),
        // Short-term explicit bridge (while the DHT humans-replayer is a stub):
        // the create surface may seed household membership directly so the
        // load-bearing resilience-snapshot junction is populated. When absent,
        // membership is still filled by the seeder pipeline or the
        // household_backfill startup pass (which never overwrites a set value).
        household_id: body.household_id,
    };

    let mut conn = get_conn(pool)?;
    let human = humans::create_human(&mut conn, input)?;
    Ok(response::created(&HumanView::from(human)))
}

/// Request body for `POST /api/v1/identity/heal`.
///
/// STOPGAP (resolver spec §3.4) — the NULL-only heal payload. The seeder
/// supplies the slug `id` (e.g. `human-matthew-manager`) plus the truthful
/// `agentPubKey` it fetched from the pod's `GET /auth/me`. `householdId` is
/// optional (the seeder leaves it None — `seed-humans.ts` already set it).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct HealHumanInputView {
    id: String,
    agent_pub_key: Option<String>,
    household_id: Option<String>,
}

/// POST /api/v1/identity/heal
///
/// STOPGAP (resolver spec §3.4 / §5 step 1): fill a slug-keyed human row's
/// currently-NULL `agent_pub_key` (and/or `household_id`) without overwriting a
/// set value. Lights the `commitment_backed_collectives` column of the
/// resilience card by making `humans.agent_pub_key` equal the
/// `rea_commitments.provider` (`uhCAk…`) the snapshot join compares against.
/// Idempotent + reachable through doorway's transparent `/api/v1/identity/*`
/// proxy (no doorway change needed).
async fn heal_human(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: HealHumanInputView = parse_body(req).await?;
    let mut conn = get_conn(pool)?;
    let human = humans::heal_human_identity(
        &mut conn,
        &body.id,
        body.agent_pub_key.as_deref(),
        body.household_id.as_deref(),
    )?;
    Ok(response::ok(&HumanView::from(human)))
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
// M-AGGR-1: Per-agent session + upgrade-prompt projection routes
// ---------------------------------------------------------------------------

/// GET /api/v1/identity/{agentId}/session
///
/// Returns the SessionHumanView projection for the specified agent.
/// The projection is derived from the EconomicEvent stream filtered by
/// lamadEventType and aggregated per-agent. If no events have been recorded
/// yet for this agent, returns a zero-count view.
///
/// Source of truth: economic_events projection table (Category C operational).
async fn get_agent_session(
    agent_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let h_app_id = &ctx.h_app_id;

    // Recompute on read — this is cheap (count aggregations on an indexed table)
    // and ensures the caller always gets a fresh projection without a separate
    // signal trigger requirement.
    let view = session_human_view::project_session_human_view(&mut conn, agent_id, h_app_id)?;
    Ok(response::ok(&view))
}

/// GET /api/v1/identity/{agentId}/upgrade-prompts
///
/// Returns the UpgradePromptView projection for the specified agent.
/// The projection is derived from the SessionHumanView × onboarding Manifest.
/// If no onboarding Manifest has been authored yet, active_prompts is empty.
///
/// Source of truth: session_human_view + manifests tables (Category C operational).
async fn get_agent_upgrade_prompts(
    agent_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let h_app_id = &ctx.h_app_id;

    // Compute the SessionHumanView first (source for trigger evaluation).
    let session = session_human_view::project_session_human_view(&mut conn, agent_id, h_app_id)?;
    // Then derive upgrade prompts from the session + onboarding Manifest.
    let view =
        upgrade_prompt_view::project_upgrade_prompt_view(&mut conn, agent_id, h_app_id, &session)?;
    Ok(response::ok(&view))
}

/// GET /api/v1/identity/{id}/profile
///
/// The viewer-relative disclosure lens: returns the facets of subject `{id}`
/// that the **authenticated session viewer** may see (spec §3 / §5). The viewer
/// is resolved from the session identity (`X-Agent-Id` → the viewer's `Human`
/// row); there is NEVER a `?viewer=` query param (a raw-identity spoof surface).
/// An anonymous caller (no session) resolves to the commons tier.
///
/// Not `auth_required` — anonymous must reach it (→ commons face). The handler
/// resolves the viewer optionally; `None` means anon.
///
/// `{id}` is the subject's human `id`. The viewer is joined to the subject on
/// the SAME human-`id` namespace inside the consent read (NOT an agent pubkey —
/// feeding a pubkey into the id-keyed consent read would empty the join and
/// silently collapse every viewer to the commons floor).
async fn get_profile_lens(
    req: &Request<Incoming>,
    subject_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    // Resolve the viewer's session agent key, then to the viewer's human id.
    // Anonymous (no header, no session) → viewer_id = None → commons floor.
    let viewer_human_id: Option<String> = match extract_agent_key(req, &mut conn)? {
        Some(agent_key) => humans::get_human_by_agent_key(&mut conn, &agent_key)?.map(|h| h.id),
        None => None,
    };

    let view = crate::services::viewer_lens_facing::build_profile_lens_view(
        &mut conn,
        ctx,
        subject_id,
        viewer_human_id.as_deref(),
    )?;

    Ok(response::from_option(
        Ok(view),
        &format!("No human record found for subject {}", subject_id),
    ))
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
