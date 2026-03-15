//! Gate API controller
//!
//! Routes: `/api/v1/gate[/confirm]`

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::{response, Services};

use super::{get_conn, parse_body};

/// Handle `/api/v1/gate*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        (&Method::POST, "confirm") => handle_confirm(req, pool, ctx, services).await,

        _ => Ok(response::not_found(&format!(
            "Unknown gate route: {} /api/v1/gate/{}",
            method, path
        ))),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmRequest {
    pub confirm_token: String,
}

async fn handle_confirm(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: ConfirmRequest = parse_body(req).await?;

    let svc = services
        .as_ref()
        .ok_or_else(|| StorageError::Internal("Gate services unavailable".into()))?;

    let pending = svc
        .gate
        .pending_confirmations()
        .take(&body.confirm_token)
        .ok_or_else(|| StorageError::NotFound("Invalid or expired confirmation token".into()))?;

    // Store PauseOverride observation — the human chose to proceed despite the elohim's pause
    let obs_id = uuid::Uuid::new_v4().to_string();
    let now = crate::db::models::current_timestamp();
    let mut conn = get_conn(pool)?;
    let new_obs = crate::db::models::NewImagodeiObservation {
        id: &obs_id,
        app_id: &ctx.app_id,
        human_id: &pending.human_id,
        observed_at: &now,
        observation_type: "pause_override",
        content: "User confirmed after gate pause",
        structured_signals_json: None,
        trust_delta: -0.02_f32, // small negative signal — not punitive, just noted
        visibility_layer: "individual",
        originating_elohim: "gate",
        relevance_decay: 0.8_f32,
        superseded_by: None,
    };
    let _ = crate::db::imagodei_observations::create_observation(&mut conn, ctx, &new_obs);

    // Invalidate trust cache so next evaluation picks up the new observation
    svc.gate.trust_cache().invalidate(&pending.human_id);

    Ok(response::ok(&serde_json::json!({
        "confirmed": true,
        "message": "Proceed with your original mutation",
    })))
}
