//! **DEV/FIXTURE ONLY** doorway-local operational controls.
//!
//! These endpoints exist purely so a2o fixtures can stage doorway-local
//! OPERATIONAL state that has no real-world equivalent reachable from CI — most
//! notably a portal host at a non-resolving `.example` origin that the live
//! `/healthz` HEAD probe could never reach.
//!
//! ## Endpoints
//!
//! - `PUT /admin/dev/portal-health` — set a portal host's health override
//!
//! ## Hard dev-mode gate
//!
//! Every handler here is gated behind `state.args.dev_mode` and returns
//! `403 FIXTURE_ONLY` when dev mode is off — mirroring the steward-grant surface
//! in `admin_users.rs`. None of this state is reachable on a production doorway.
//!
//! ## Truth-layer note
//!
//! The portal-health override is doorway-local OPERATIONAL state
//! (`AppState::portal_health_override`). It is NOT a projection of the notarized
//! `portal_hosts` DHT entry (Category A) — the override never touches the zome,
//! the storage projection, or the view. doorway/CLAUDE.md explicitly permits
//! "doorway-local Operational state" as legitimate doorway-resident state.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;

use crate::server::AppState;

type FullBody = Full<Bytes>;

/// Request body for `PUT /admin/dev/portal-health`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortalHealthRequest {
    /// The portal host URL whose health is being overridden. Must match the
    /// `hostUrl` the human registered via `/api/v1/account/portal-hosts` exactly
    /// (the probe loop keys on the storage row's `host_url`).
    host_url: String,
    /// `true` → the probe treats this host as reachable (returns it without a
    /// live HEAD); `false` → the probe skips this host.
    healthy: bool,
}

fn json_response(status: StatusCode, body: &serde_json::Value) -> Response<FullBody> {
    let bytes = serde_json::to_vec(body).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(bytes)))
        .unwrap()
}

fn error_response(status: StatusCode, error: &str, code: &str) -> Response<FullBody> {
    json_response(status, &serde_json::json!({ "error": error, "code": code }))
}

/// Guard a dev/fixture surface behind dev mode.
///
/// Returns `Some(403)` with code `FIXTURE_ONLY` when dev mode is OFF — this
/// surface must NEVER be reachable on a production doorway. Returns `None` when
/// dev mode is ON. Factored out so the gate is unit-testable without
/// synthesizing a hyper `Request<Incoming>` (mirrors `admin_users::fixture_only_gate`).
fn fixture_only_gate(state: &AppState) -> Option<Response<FullBody>> {
    if state.args.dev_mode {
        None
    } else {
        Some(error_response(
            StatusCode::FORBIDDEN,
            "portal-health override is a dev/fixture surface",
            "FIXTURE_ONLY",
        ))
    }
}

/// `PUT /admin/dev/portal-health` — set a doorway-local portal-host health override.
///
/// Body: `{ "hostUrl": string, "healthy": bool }`.
///
/// Hard-gated behind dev mode (403 FIXTURE_ONLY first, before body parsing).
/// Writes into `AppState::portal_health_override`, which the portal-host probe
/// consults ONLY when dev mode is on. No Mongo, no DHT, no projection.
pub async fn handle_set_portal_health(
    req: Request<Incoming>,
    state: Arc<AppState>,
) -> Response<FullBody> {
    // (1) HARD dev-mode gate FIRST — before any body parsing. Invisible on prod.
    if let Some(forbidden) = fixture_only_gate(&state) {
        return forbidden;
    }

    // (2) Parse the body.
    let body_bytes = match req.into_body().collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid body", "BAD_BODY"),
    };
    let request: PortalHealthRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid JSON: {e}"),
                "BAD_JSON",
            )
        }
    };

    // (3) Apply the override.
    {
        let mut overrides = state.portal_health_override.write().await;
        overrides.insert(request.host_url.clone(), request.healthy);
    }
    info!(
        target: "doorway::admin_dev",
        host_url = %request.host_url,
        healthy = request.healthy,
        "dev portal-health override set"
    );

    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "hostUrl": request.host_url,
            "healthy": request.healthy,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Args;
    use clap::Parser;

    fn test_state(dev_mode: bool) -> AppState {
        let mut args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        args.dev_mode = dev_mode;
        AppState::new(args)
    }

    #[test]
    fn fixture_gate_blocks_when_dev_mode_off() {
        let state = test_state(false);
        let resp = fixture_only_gate(&state).expect("dev_mode off must produce a 403 response");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn fixture_gate_allows_when_dev_mode_on() {
        let state = test_state(true);
        assert!(
            fixture_only_gate(&state).is_none(),
            "dev_mode on must allow the handler to proceed"
        );
    }

    #[tokio::test]
    async fn override_round_trips_through_state_map() {
        // The handler writes into AppState::portal_health_override; assert the
        // map reflects the write so the probe (which reads the same map) sees it.
        let state = test_state(true);
        {
            let mut overrides = state.portal_health_override.write().await;
            overrides.insert("https://matthew.steward.example/account".into(), true);
        }
        let overrides = state.portal_health_override.read().await;
        assert_eq!(
            overrides.get("https://matthew.steward.example/account"),
            Some(&true)
        );
    }
}
