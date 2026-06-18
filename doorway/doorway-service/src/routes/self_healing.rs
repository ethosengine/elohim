//! `GET /admin/self-healing` — the unified self-healing read model.
//!
//! Cat C node-local Operational state (SELF-HEALING-CONTROL-PLANE-DESIGN §6): NO
//! DHT entry, NO table, NO coordinator fn — a fresh projection composed at
//! request time from in-process AppState snapshots + one on-demand fetch of
//! storage's projector status. Fails the swap test by design (a node serves its
//! OWN runtime state). Same legitimate doorway-local class as /admin/capability
//! and /admin/render-stats.
//!
//! Agent-consumable: plain HTTP JSON, camelCase, stable keys. `autoPreset`
//! remains a reserved PENDING field (null) until the auto-config sibling lands;
//! `admission` and `upstreams` are now LANDED — the §6 co-delivery wired the
//! shared accessors (metrics inbound-max gauge + admission_shed counter +
//! UpstreamBreakers::snapshot()), one home each, also feeding /metrics.

use std::sync::Arc;

use elohim_compute::peers::PeerHealthSnapshot;
use elohim_render::RenderTraceSnapshot;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use serde::Serialize;

/// Top-level self-healing read model. Cat C node-local. Keys are STABLE for
/// machine consumption — null (not absent) for not-yet-computable scalars,
/// empty array for not-yet-populated collections.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfHealingView {
    /// Resource snapshot + derived Auto config + reasons.
    /// FOLLOW-ON: wire when the auto-config plan lands
    /// (AppState.auto_preset via elohim_compute::limits).
    pub auto_preset: Option<serde_json::Value>,
    /// Inbound admission: { maxInflight, available, shedTotal }. LANDED (metrics
    /// inbound-max gauge + admission_shed counter + live semaphore permits).
    pub admission: Option<AdmissionView>,
    /// Per-upstream circuit/health state. LANDED (UpstreamBreakers::snapshot()).
    pub upstreams: Vec<UpstreamView>,
    pub projector: ProjectorView,
    pub peers: Vec<PeerView>,
    pub render: RenderView,
    pub warmup: WarmupView,
    pub conductor: ConductorView,
}

/// Inbound admission ceiling + headroom + shed count (LANDED).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionView {
    pub max_inflight: usize,
    pub available: usize,
    pub shed_total: u64,
}

/// One upstream's circuit/health entry (LANDED — from UpstreamBreakers::snapshot).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamView {
    pub endpoint: String,
    /// "closed" | "half-open" | "open"
    pub circuit: String,
    pub error_streak: u32,
    pub last_good: Option<String>,
    pub skipped: bool,
}

/// Projector lag + reconcile caught-up state (LANDED).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectorView {
    /// Max reconciliation lag in seconds across (pillar, kind); None when not computable.
    pub lag_seconds: Option<i64>,
    /// None when storage's reconcile task is not spawned (projectionReconcile null).
    pub caught_up: Option<bool>,
    pub divergent_anchor: Option<usize>,
}

/// One peer entry (LANDED — from PeerHealthRegistry::snapshot()).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerView {
    pub peer: String,
    /// "Healthy" | "Degraded" | "Offline" (Debug form of ServiceHealth).
    pub status: String,
    pub last_seen: Option<String>,
}

/// Render reliability proxy for projector/render health (LANDED).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderView {
    pub total: u64,
    pub degenerate_rate: f64,
}

/// Warmup progress (LANDED — WarmupState atoms, read-only).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarmupView {
    pub in_progress: bool,
    pub attempts: u32,
    pub completed: bool,
    pub last_error: Option<String>,
}

/// Conductor health (LANDED — same source as /health ConductorHealth).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConductorView {
    pub connected: bool,
    pub connected_workers: usize,
    pub total_workers: usize,
}

/// Already-collected component snapshots injected into the PURE composer.
/// Collected by the handler; plain data so the composer does no I/O.
pub struct SelfHealingInputs {
    pub projector_lag_seconds: Option<i64>,
    pub p2p_caught_up: Option<bool>,
    pub p2p_divergent_anchor: Option<usize>,
    pub peers: Vec<PeerHealthSnapshot>,
    pub render: RenderTraceSnapshot,
    /// (in_progress, attempts, completed, last_error) — None when no warmup task.
    pub warmup: Option<(bool, u32, bool, Option<String>)>,
    /// (connected, connected_workers, total_workers)
    pub conductor: (bool, usize, usize),
    /// (maxInflight, available, shedTotal) for the AdmissionView (LANDED).
    pub admission: (usize, usize, u64),
    /// Per-upstream breaker snapshots for the UpstreamView (LANDED).
    pub upstreams: Vec<crate::routes::upstream_health::BreakerSnapshot>,
}

/// PURE: compose the read model from injected snapshots. No I/O, no AppState.
/// PENDING sibling fields are reserved (None / empty) here — a sibling's
/// landing changes only the inputs the handler injects, never this function.
pub fn compose_self_healing(inputs: SelfHealingInputs) -> SelfHealingView {
    let peers = inputs
        .peers
        .into_iter()
        .map(|p| PeerView {
            peer: p.peer_id,
            status: format!("{:?}", p.health),
            last_seen: p.last_signal_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    let warmup = match inputs.warmup {
        Some((in_progress, attempts, completed, last_error)) => WarmupView {
            in_progress,
            attempts,
            completed,
            last_error,
        },
        None => WarmupView {
            in_progress: false,
            attempts: 0,
            completed: false,
            last_error: None,
        },
    };

    let (connected, connected_workers, total_workers) = inputs.conductor;
    let (max_inflight, available, shed_total) = inputs.admission;
    let upstreams = inputs
        .upstreams
        .into_iter()
        .map(|b| UpstreamView {
            endpoint: b.endpoint,
            circuit: b.circuit.to_string(),
            error_streak: b.error_streak,
            last_good: None, // the breaker does not track a last-good timestamp yet
            skipped: b.skipped,
        })
        .collect();

    SelfHealingView {
        // FOLLOW-ON: auto-config sibling sets this from AppState.auto_preset.
        auto_preset: None,
        admission: Some(AdmissionView {
            max_inflight,
            available,
            shed_total,
        }),
        upstreams,
        projector: ProjectorView {
            lag_seconds: inputs.projector_lag_seconds,
            caught_up: inputs.p2p_caught_up,
            divergent_anchor: inputs.p2p_divergent_anchor,
        },
        peers,
        render: RenderView {
            total: inputs.render.total,
            degenerate_rate: inputs.render.degenerate_rate,
        },
        warmup,
        conductor: ConductorView {
            connected,
            connected_workers,
            total_workers,
        },
    }
}

/// Parse the max `lagSeconds` across all (pillar, kind) lag entries in a
/// ProjectorStatusView JSON body. None when there are no numeric lag entries.
fn parse_projector_lag(body: &serde_json::Value) -> Option<i64> {
    body["lag"]
        .as_array()?
        .iter()
        .filter_map(|e| e["lagSeconds"].as_i64())
        .max()
}

/// Fetch storage's /api/v1/status/projector and return the max lagSeconds.
/// Fault-tolerant: any error (storage down, parse fail, no URL) → None. NEVER
/// fails the aggregate — the stability surface degrades a field, not the call.
async fn fetch_projector_status(state: &Arc<crate::server::AppState>) -> Option<i64> {
    let base = state.args.storage_url.as_ref()?;
    let url = format!("{}/api/v1/status/projector", base.trim_end_matches('/'));
    let resp = state.ssr_http_client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    parse_projector_lag(&body)
}

/// Handle `GET /admin/self-healing`. Composes the Cat-C node-local read model
/// from in-process snapshots + one on-demand storage projector fetch. No auth
/// (in-cluster reads, same rationale as /admin/capability; operator-only is an
/// ingress property, not enforced here).
pub async fn handle_self_healing(state: Arc<crate::server::AppState>) -> Response<Full<Bytes>> {
    // Non-blocking read of the cached P2P health (carries caughtUp/divergentAnchor).
    // try_read so a held write-lock never stalls the read model.
    let (p2p_caught_up, p2p_divergent_anchor) = match state.p2p_health.try_read() {
        Ok(guard) => match guard.as_ref() {
            Some(h) => (h.caught_up, h.divergent_anchor),
            None => (None, None),
        },
        Err(_) => (None, None),
    };

    let peers = state.peer_health.snapshot();
    let render = state.render_trace_stats.snapshot();

    let warmup = state.warmup_state.as_ref().map(|ws| {
        use std::sync::atomic::Ordering::Relaxed;
        (
            ws.in_progress.load(Relaxed),
            ws.attempts.load(Relaxed),
            ws.completed.load(Relaxed),
            ws.last_error.lock().ok().and_then(|g| g.clone()),
        )
    });

    let conductor = match &state.pool {
        Some(pool) => (
            pool.is_healthy(),
            pool.connected_count(),
            pool.worker_count(),
        ),
        None => (false, 0, 0),
    };

    // Admission (LANDED): ceiling from the metrics gauge, headroom from the live
    // semaphore, shed count from the metrics counter — one home for the count,
    // shared with /metrics (handoff §6 co-delivery).
    let admission = (
        crate::metrics::inbound_max() as usize,
        state.inbound_semaphore.available_permits(),
        crate::metrics::admission_shed_total(),
    );
    // Upstreams (LANDED): read-only breaker snapshot — never admits a half-open
    // trial as a side effect of being observed.
    let upstreams = state.upstream_breakers.snapshot();

    let projector_lag_seconds = fetch_projector_status(&state).await;

    let view = compose_self_healing(SelfHealingInputs {
        projector_lag_seconds,
        p2p_caught_up,
        p2p_divergent_anchor,
        peers,
        render,
        warmup,
        conductor,
        admission,
        upstreams,
    });

    match serde_json::to_string_pretty(&view) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-store")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from(
                "Failed to serialize self-healing view",
            )))
            .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_compute::peers::PeerHealthSnapshot;
    use elohim_compute::ServiceHealth;
    use elohim_render::RenderTraceSnapshot;

    fn sample_inputs() -> SelfHealingInputs {
        SelfHealingInputs {
            projector_lag_seconds: Some(7),
            p2p_caught_up: Some(false),
            p2p_divergent_anchor: Some(2),
            peers: vec![PeerHealthSnapshot {
                peer_id: "peer-a".to_string(),
                address: "addr".to_string(),
                health: ServiceHealth::Degraded,
                reason: "reconnecting".to_string(),
                signals_received: 3,
                last_signal_at: None,
                reconnect_attempts: 1,
            }],
            render: RenderTraceSnapshot::default(),
            warmup: Some((false, 4u32, true, Some("boom".to_string()))),
            conductor: (true, 2, 4),
            admission: (256, 200, 3),
            upstreams: vec![crate::routes::upstream_health::BreakerSnapshot {
                endpoint: "http://storage:8090".to_string(),
                circuit: "open",
                error_streak: 4,
                skipped: true,
            }],
        }
    }

    #[test]
    fn view_serializes_with_stable_camelcase_keys() {
        let view = compose_self_healing(SelfHealingInputs {
            projector_lag_seconds: Some(0),
            p2p_caught_up: Some(true),
            p2p_divergent_anchor: Some(0),
            peers: vec![],
            render: RenderTraceSnapshot::default(),
            warmup: Some((false, 0, true, None)),
            conductor: (true, 1, 1),
            admission: (128, 100, 0),
            upstreams: vec![],
        });
        let json = serde_json::to_value(&view).unwrap();
        // autoPreset stays reserved (null); admission + upstreams now LANDED.
        assert!(
            json.get("autoPreset").is_some(),
            "autoPreset key must be present"
        );
        assert_eq!(json["autoPreset"], serde_json::Value::Null);
        assert_eq!(json["admission"]["maxInflight"], serde_json::json!(128));
        assert_eq!(json["admission"]["available"], serde_json::json!(100));
        assert_eq!(json["admission"]["shedTotal"], serde_json::json!(0));
        assert_eq!(json["upstreams"], serde_json::json!([]));
        // LANDED scalars present and camelCase.
        assert_eq!(json["projector"]["caughtUp"], serde_json::json!(true));
        assert_eq!(json["projector"]["divergentAnchor"], serde_json::json!(0));
        assert_eq!(json["render"]["degenerateRate"], serde_json::json!(0.0));
        assert_eq!(json["warmup"]["inProgress"], serde_json::json!(false));
        assert_eq!(json["conductor"]["connectedWorkers"], serde_json::json!(1));
    }

    #[test]
    fn compose_maps_landed_fields_and_reserves_pending() {
        let view = compose_self_healing(sample_inputs());
        // auto_preset stays reserved; admission + upstreams now LANDED.
        assert!(view.auto_preset.is_none());
        let adm = view.admission.expect("admission landed");
        assert_eq!(adm.max_inflight, 256);
        assert_eq!(adm.available, 200);
        assert_eq!(adm.shed_total, 3);
        assert_eq!(view.upstreams.len(), 1);
        assert_eq!(view.upstreams[0].endpoint, "http://storage:8090");
        assert_eq!(view.upstreams[0].circuit, "open");
        assert_eq!(view.upstreams[0].error_streak, 4);
        assert!(view.upstreams[0].skipped);
        assert_eq!(view.upstreams[0].last_good, None);
        // LANDED projector
        assert_eq!(view.projector.lag_seconds, Some(7));
        assert_eq!(view.projector.caught_up, Some(false));
        assert_eq!(view.projector.divergent_anchor, Some(2));
        // LANDED peers — ServiceHealth maps to its Debug string form
        assert_eq!(view.peers.len(), 1);
        assert_eq!(view.peers[0].peer, "peer-a");
        assert_eq!(view.peers[0].status, "Degraded");
        // LANDED warmup
        assert_eq!(view.warmup.attempts, 4);
        assert_eq!(view.warmup.last_error.as_deref(), Some("boom"));
        // LANDED conductor
        assert_eq!(view.conductor.connected_workers, 2);
        assert_eq!(view.conductor.total_workers, 4);
    }

    #[test]
    fn compose_handles_absent_warmup() {
        let mut inputs = sample_inputs();
        inputs.warmup = None;
        let view = compose_self_healing(inputs);
        assert!(!view.warmup.in_progress);
        assert_eq!(view.warmup.attempts, 0);
        assert!(view.warmup.last_error.is_none());
    }

    #[test]
    fn parse_projector_lag_takes_max_across_kinds() {
        let body = serde_json::json!({
            "cursors": [],
            "lag": [
                { "pillar": "lamad", "kind": "content", "lagSeconds": 3 },
                { "pillar": "lamad", "kind": "path", "lagSeconds": 11 },
                { "pillar": "mishpat", "kind": "commitment", "lagSeconds": null }
            ]
        });
        assert_eq!(parse_projector_lag(&body), Some(11));
    }

    #[test]
    fn parse_projector_lag_none_when_no_lag_entries() {
        let body = serde_json::json!({ "cursors": [], "lag": [] });
        assert_eq!(parse_projector_lag(&body), None);
    }

    use crate::config::Args;
    use crate::server::AppState;
    use clap::Parser;

    fn test_state() -> AppState {
        let args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        AppState::new(args)
    }

    #[tokio::test]
    async fn handler_returns_200_with_reserved_keys_on_bare_state() {
        let state = Arc::new(test_state());
        let resp = handle_self_healing(state).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert!(json.get("autoPreset").is_some());
        assert_eq!(json["autoPreset"], serde_json::Value::Null);
        // admission is now LANDED (an object with the three keys); a bare state
        // has no breaker records, so upstreams stays empty.
        assert!(
            json["admission"].is_object(),
            "admission landed as an object"
        );
        assert!(json["admission"].get("maxInflight").is_some());
        assert!(json["admission"].get("available").is_some());
        assert!(json["admission"].get("shedTotal").is_some());
        assert_eq!(json["upstreams"], serde_json::json!([]));
        assert!(json.get("projector").is_some());
        assert!(json.get("peers").is_some());
        assert!(json.get("render").is_some());
        assert!(json.get("warmup").is_some());
        assert_eq!(json["conductor"]["connected"], serde_json::json!(false));
    }
}
