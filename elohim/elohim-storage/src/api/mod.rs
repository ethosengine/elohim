//! Enriched API endpoints for pillar-level business logic
//!
//! These `/api/v1/*` routes encapsulate domain logic that was previously
//! in doorway route handlers. By living in storage, both browser (via doorway
//! proxy) and Tauri (direct to :8090) get identical behavior.
//!
//! ## Architecture (Controller → Service → Model)
//!
//! ```text
//! api/*.rs (controllers)      — HTTP handlers, serde request/response types
//!     ↓
//! services/*_service.rs       — Business logic, validation, compound operations
//!     ↓
//! db/*.rs (models)            — Diesel queries, ORM models
//! ```

pub mod account;
pub mod agreements;
pub mod attestations;
pub mod blob;
pub mod cluster;
pub mod comments;
pub mod compute;
pub mod contributors;
pub mod custodians;
pub mod dashboard;
pub mod economic_events;
pub mod epr;
pub mod exchange;
pub mod flow_planning;
pub mod gate;
pub mod governance;
pub mod governance_actions;
pub mod graph_views;
pub mod hazards;
pub mod identity;
pub mod mastery;
pub mod network_posture;
pub mod node_shape;
pub mod observations;
pub mod peer_statuses;
pub mod peer_topology;
pub mod placement_gaps;
pub mod places;
pub mod presence;
pub mod projector_status;
pub mod rea_commitments;
pub mod reciprocity;
pub mod recognition;
pub mod registry;
pub mod resilience;
pub mod resilience_hub;
pub mod resources;
pub mod risk;
pub mod routing;
pub mod schedules;
pub mod signal_emit;
pub mod spatial;
pub mod standing;
pub mod steward;
pub mod steward_affinity;
pub mod stewardship;
pub mod token;
pub mod weather;
pub mod write_through_admin;
pub mod write_through_status;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::{response, Services};

use std::sync::Arc;

/// Handle all `/api/v1/*` requests by dispatching to domain controllers
#[allow(clippy::too_many_arguments)]
pub async fn handle_api_request(
    req: Request<Incoming>,
    method: Method,
    path: &str,
    pool: DbPool,
    services: Option<Arc<Services>>,
    hc_registry: Option<Arc<crate::hc_client_registry::HcClientRegistry>>,
    swarm_tx: Option<tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>>,
    local_peer_id: Option<String>,
    fan_out_ctx: Option<Arc<crate::api::epr::EprFanOutCtx>>,
    p2p_handle: Option<crate::p2p::P2PHandle>,
    elohim_capability: Option<crate::views::ElohimCapabilityProfile>,
    render_capability: Option<crate::views::RenderCapabilityProfile>,
    extensions: Option<crate::views::CapabilityExtensions>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Strip /api/v1/ prefix
    let sub_path = path.strip_prefix("/api/v1/").unwrap_or("");

    // Extract app context from X-App-Id header, default to "lamad".
    // D.7: populate local_libp2p_peer_id so epr routes can dedup self-reports.
    let app_ctx = extract_app_context(&req, local_peer_id);

    // Extract graph_engine from fan-out context for graph-native route branches.
    // Available when the graph-native feature is ON and the engine opened at startup.
    #[cfg(feature = "graph-native")]
    let graph_engine_ref = fan_out_ctx
        .as_ref()
        .and_then(|ctx| ctx.graph_engine.as_ref());
    #[cfg(not(feature = "graph-native"))]
    let graph_engine_ref: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>> = None;

    // Dispatch to domain controllers
    if sub_path.starts_with("account") {
        let resource_path = sub_path.strip_prefix("account").unwrap_or("");
        return account::handle(req, method, resource_path, &pool, hc_registry.as_ref()).await;
    } else if sub_path.starts_with("agreements") {
        let resource_path = sub_path.strip_prefix("agreements").unwrap_or("");
        agreements::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("presence") {
        let resource_path = sub_path.strip_prefix("presence").unwrap_or("");
        presence::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("stewardship") {
        let resource_path = sub_path.strip_prefix("stewardship").unwrap_or("");
        stewardship::handle(req, method, resource_path, &pool, &app_ctx, services).await
    } else if sub_path.starts_with("economic-events") {
        let resource_path = sub_path.strip_prefix("economic-events").unwrap_or("");
        economic_events::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("epr") {
        let resource_path = sub_path.strip_prefix("epr").unwrap_or("");
        epr::handle(
            req,
            method,
            resource_path,
            &pool,
            &app_ctx,
            swarm_tx,
            fan_out_ctx,
        )
        .await
    } else if sub_path.starts_with("resources") {
        let resource_path = sub_path.strip_prefix("resources").unwrap_or("");
        resources::handle(req, method, resource_path, &pool, &app_ctx, services).await
    } else if sub_path.starts_with("exchange") {
        let resource_path = sub_path.strip_prefix("exchange").unwrap_or("");
        exchange::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("identity") {
        let resource_path = sub_path.strip_prefix("identity").unwrap_or("");
        identity::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("mastery") {
        let resource_path = sub_path.strip_prefix("mastery").unwrap_or("");
        mastery::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("flow-planning") {
        let resource_path = sub_path.strip_prefix("flow-planning").unwrap_or("");
        flow_planning::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("governance-actions") {
        // Must precede "governance" to avoid prefix collision.
        let resource_path = sub_path.strip_prefix("governance-actions").unwrap_or("");
        governance_actions::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("governance") {
        let resource_path = sub_path.strip_prefix("governance").unwrap_or("");
        governance::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("compute") {
        let resource_path = sub_path.strip_prefix("compute").unwrap_or("");
        compute::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("custodians") {
        let resource_path = sub_path.strip_prefix("custodians").unwrap_or("");
        custodians::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("commitments") {
        let resource_path = sub_path.strip_prefix("commitments").unwrap_or("");
        rea_commitments::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("attestations") {
        let resource_path = sub_path.strip_prefix("attestations").unwrap_or("");
        attestations::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("blob/") {
        // Phase 5 T29: /api/v1/blob/{hash}/distribution/details
        // Note: the `/blob/` raw-bytes route is NOT under /api/v1/ (handled at
        // the outer http.rs dispatcher); this prefix is for blob-scoped views only.
        let resource_path = sub_path.strip_prefix("blob").unwrap_or("");
        blob::handle(
            req,
            method,
            resource_path,
            &pool,
            &app_ctx,
            graph_engine_ref,
        )
        .await
    } else if sub_path == "cluster" {
        // Phase 5 T30: GET /api/v1/cluster — agent-scoped cluster view.
        // Auth implicit (Holochain idiom): the calling agent's identity (resolved
        // via X-Agent-Cid or local_sessions) determines scope. Future hub variants
        // (`/cluster/household/{id}`, `/cluster/hub/{id}`) extend the path tree.
        cluster::handle(req, method, &pool, p2p_handle.as_ref(), graph_engine_ref).await
    } else if sub_path == "peer-topology" {
        // Phase 5 T31: GET /api/v1/peer-topology — agent-scoped peer-household
        // topology view. Same auth-implicit pattern as `/cluster`; federated
        // through the swarm via Federator.
        peer_topology::handle(req, method, &pool, p2p_handle.as_ref(), graph_engine_ref).await
    } else if sub_path == "reciprocity" {
        // Phase 5 T32: GET /api/v1/reciprocity — agent-scoped reciprocity ledger.
        // Local SQLite roll-up over the steward's REA projection (no federation),
        // so no `p2p_handle` plumbed. Auth-implicit identity determines scope.
        reciprocity::handle(req, method, &pool, graph_engine_ref).await
    } else if sub_path.starts_with("comments") {
        let resource_path = sub_path.strip_prefix("comments").unwrap_or("");
        comments::handle(req, method, resource_path, &pool, &app_ctx, services).await
    } else if sub_path.starts_with("contributors") {
        let resource_path = sub_path.strip_prefix("contributors").unwrap_or("");
        contributors::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("registry") {
        let resource_path = sub_path.strip_prefix("registry").unwrap_or("");
        registry::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("recognition") {
        let resource_path = sub_path.strip_prefix("recognition").unwrap_or("");
        recognition::handle(req, method, resource_path, &pool, &app_ctx, services).await
    } else if sub_path.starts_with("routing") {
        let resource_path = sub_path.strip_prefix("routing").unwrap_or("");
        routing::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("weather") {
        let resource_path = sub_path.strip_prefix("weather").unwrap_or("");
        weather::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("resilience") {
        let resource_path = sub_path.strip_prefix("resilience").unwrap_or("");
        resilience::handle(
            req,
            method,
            resource_path,
            &pool,
            &app_ctx,
            graph_engine_ref,
        )
        .await
    } else if sub_path.starts_with("risk") {
        let resource_path = sub_path.strip_prefix("risk").unwrap_or("");
        risk::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("hazards") {
        let resource_path = sub_path.strip_prefix("hazards").unwrap_or("");
        hazards::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("observations") {
        let resource_path = sub_path.strip_prefix("observations").unwrap_or("");
        observations::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("placement-gaps") {
        let resource_path = sub_path.strip_prefix("placement-gaps").unwrap_or("");
        placement_gaps::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("dashboard") {
        let resource_path = sub_path.strip_prefix("dashboard").unwrap_or("");
        dashboard::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("places") {
        let resource_path = sub_path.strip_prefix("places").unwrap_or("");
        places::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("schedules") {
        let resource_path = sub_path.strip_prefix("schedules").unwrap_or("");
        schedules::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("standing") {
        let resource_path = sub_path.strip_prefix("standing").unwrap_or("");
        standing::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("spatial-contexts") {
        let resource_path = sub_path.strip_prefix("spatial-contexts").unwrap_or("");
        spatial::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("steward-affinity") {
        let resource_path = sub_path.strip_prefix("steward-affinity").unwrap_or("");
        steward_affinity::handle(req, method, resource_path, &pool, &app_ctx, services).await
    } else if sub_path.starts_with("steward") && !sub_path.starts_with("stewardship") {
        let resource_path = sub_path.strip_prefix("steward").unwrap_or("");
        steward::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("peer-statuses") {
        let resource_path = sub_path.strip_prefix("peer-statuses").unwrap_or("");
        peer_statuses::handle(
            req,
            method,
            resource_path,
            &pool,
            &app_ctx,
            elohim_capability.as_ref(),
            render_capability.as_ref(),
            extensions.as_ref(),
        )
        .await
    } else if sub_path.starts_with("nodes") {
        let resource_path = sub_path.strip_prefix("nodes").unwrap_or("");
        node_shape::handle_nodes(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("households") {
        let resource_path = sub_path.strip_prefix("households").unwrap_or("");
        node_shape::handle_households(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("network") {
        let resource_path = sub_path.strip_prefix("network").unwrap_or("");
        network_posture::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path.starts_with("gate") {
        let resource_path = sub_path.strip_prefix("gate").unwrap_or("");
        gate::handle(req, method, resource_path, &pool, &app_ctx, services).await
    } else if sub_path.starts_with("token") {
        let resource_path = sub_path
            .strip_prefix("token")
            .unwrap_or("")
            .trim_start_matches('/');
        token::handle(req, method, resource_path, &pool, &app_ctx).await
    } else if sub_path == "graphql" || sub_path == "graphql/" {
        // Phase 7: GraphQL surface — Apollo Federation v2 subgraph.
        // Hand-rolled hyper handler; reads body, executes against async-graphql schema,
        // serializes response. Returns 503 when graph engine is not available.
        //
        // Epic A (Task A2): also inject pool + Federator so viewer.hub resolvers work.
        // Federator is built from p2p_handle (same pattern as cluster.rs); both are
        // Optional — when absent the schema falls back gracefully.
        #[cfg(feature = "graph-native")]
        {
            let gql_federator: Option<std::sync::Arc<crate::services::federator::Federator>> =
                p2p_handle.as_ref().map(|h| {
                    std::sync::Arc::new(crate::services::federator::Federator::new(h.clone()))
                });
            return crate::graphql::server::handle(
                req,
                method,
                graph_engine_ref,
                Some(&pool),
                gql_federator.as_ref(),
            )
            .await;
        }
        #[cfg(not(feature = "graph-native"))]
        {
            return Ok(crate::services::response::json_response(
                hyper::StatusCode::NOT_IMPLEMENTED,
                &serde_json::json!({"error": "graph-native feature not enabled"}),
            ));
        }
    } else if sub_path == "vf-graphql" || sub_path == "vf-graphql/" {
        // Wave 3 M1 — valueflows bridge endpoint.
        // Stub-stage: serves fixture EconomicEvent via the bridge's GraphQL schema.
        // M2+ adds identity bridge (qahal-authority), M3+ adds real hREA projection.
        // See genesis/docs/superpowers/specs/2026-05-20-wave3-valueflows-hrea-interop-design.md
        // M1: every BridgeError variant is an infrastructure failure (body
        // read, response serialize, response build) — Internal (500) is
        // correct. M2+ will introduce auth/binding/authority denial variants
        // that map to 4xx; this map_err must be updated at that boundary,
        // ideally by adding `impl From<BridgeError> for StorageError` in the
        // bridge crate so missing mappings fail at compile time.
        let bridge_ctx = valueflows_bridge::BridgeContext { pool: pool.clone() };
        valueflows_bridge::handle_request(req, bridge_ctx)
            .await
            .map_err(|e| StorageError::Internal(format!("vf-graphql bridge: {e}")))
    } else if sub_path.starts_with("graph") {
        // Phase 6 Task 26: graph-native view routes.
        // Routes register unconditionally — handlers return 501 when the feature is off.
        let resource_path = sub_path.strip_prefix("graph").unwrap_or("");
        graph_views::handle(req, method, resource_path, graph_engine_ref).await
    } else {
        Ok(response::not_found(&format!(
            "Unknown API route: /api/v1/{}",
            sub_path
        )))
    }
}

/// Extract app context from request headers, defaulting to "lamad".
/// `local_peer_id` is the swarm's PeerId (base58); passed through for EPR dedup.
fn extract_app_context(req: &Request<Incoming>, local_peer_id: Option<String>) -> AppContext {
    let h_app_id = req
        .headers()
        .get("X-App-Id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("lamad")
        .to_string();
    AppContext {
        h_app_id,
        local_libp2p_peer_id: local_peer_id,
    }
}

/// Helper to get a Diesel connection from the pool
pub fn get_conn(pool: &DbPool) -> Result<crate::db::PooledConn, StorageError> {
    pool.get()
        .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))
}

// =============================================================================
// ElohimGate evaluation helper
// =============================================================================

use crate::services::elohim_gate::{
    GateResult, InferenceTier, MutationType, TrustContext, TrustSignals,
};
use crate::views::{GateEvaluationView, TrustContextView};

/// Evaluate a mutation through the ElohimGate.
/// Returns (GateResult, Option<GateEvaluationView>) for inclusion in response.
/// If services unavailable, returns PassThrough with no view.
pub async fn evaluate_gate(
    services: &Option<Arc<Services>>,
    pool: &DbPool,
    ctx: &AppContext,
    mutation: MutationType,
    mutation_content: serde_json::Value,
    human_id: Option<&str>,
) -> (GateResult, Option<GateEvaluationView>) {
    let Some(svc) = services else {
        return (
            GateResult::PassThrough {
                tier: InferenceTier::None,
            },
            None,
        );
    };

    // Query observations once for both behavioral trust and anomaly detection
    let observations = match human_id {
        Some(hid) => match get_conn(pool) {
            Ok(mut conn) => crate::db::imagodei_observations::list_observations_for_human(
                &mut conn,
                ctx,
                hid,
                "individual",
            )
            .unwrap_or_default(),
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };

    let behavioral_trust = crate::services::behavioral_trust::compute(&observations);
    let intent_divergence =
        crate::services::anomaly_detection::compute_anomaly_score(&observations);

    // Query mastery records for mastery depth
    let mastery_depth = match human_id {
        Some(hid) => match get_conn(pool) {
            Ok(mut conn) => {
                let records =
                    crate::db::content_mastery::get_mastery_for_human(&mut conn, ctx, hid)
                        .unwrap_or_default();
                crate::services::mastery_depth::compute(&records)
            }
            Err(_) => 0.5,
        },
        None => 0.5,
    };

    // Query allocations once for both steward standing and governance health
    let allocations = match human_id {
        Some(hid) => match get_conn(pool) {
            Ok(mut conn) => {
                crate::db::stewardship_allocations::get_allocations_for_steward(&mut conn, ctx, hid)
                    .unwrap_or_default()
            }
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    let steward_standing = crate::services::steward_standing::compute(&allocations);
    let governance_health = crate::services::governance_health::compute(&allocations);

    // Query relationships for relationship density
    let relationship_density = match human_id {
        Some(hid) => match get_conn(pool) {
            Ok(mut conn) => {
                let relationships = crate::db::human_relationships::get_relationships_for_human(
                    &mut conn, ctx, hid,
                )
                .unwrap_or_default();
                crate::services::relationship_density::compute(&relationships)
            }
            Err(_) => 0.5,
        },
        None => 0.5,
    };

    let trust_ctx = TrustContext::compute(TrustSignals {
        mastery_depth,        // from mastery records
        steward_standing,     // from stewardship allocations
        relationship_density, // from human relationships
        governance_health,    // from allocation governance states
        behavioral_trust,     // from observation history
        intent_divergence,    // from anomaly detection
    });

    let mutation_content_for_cache = mutation_content.clone();
    let result = svc
        .gate
        .evaluate(mutation, &trust_ctx, mutation_content)
        .await;

    // Store observations from gate evaluation (closes the feedback loop)
    if let GateResult::Enriched { observations, .. } = &result {
        if let Some(hid) = human_id {
            if let Ok(mut conn) = get_conn(pool) {
                for draft in observations {
                    let obs_id = uuid::Uuid::new_v4().to_string();
                    let now = crate::db::models::current_timestamp();
                    let signals_json = draft
                        .structured_signals
                        .as_ref()
                        .map(|v| serde_json::to_string(v).unwrap_or_default());
                    let new_obs = crate::db::models::NewImagodeiObservation {
                        id: &obs_id,
                        h_app_id: &ctx.h_app_id,
                        human_id: hid,
                        observed_at: &now,
                        observation_type: &draft.observation_type,
                        content: &draft.content,
                        structured_signals_json: signals_json.as_deref(),
                        trust_delta: draft.trust_delta as f32,
                        visibility_layer: &draft.visibility_layer,
                        originating_elohim: "sidecar",
                        relevance_decay: 1.0,
                        superseded_by: None,
                        dht_anchor_hash: None, // TODO(p2p-coherence): populate when trust attestation issued
                    };
                    if let Err(e) = crate::db::imagodei_observations::create_observation(
                        &mut conn, ctx, &new_obs,
                    ) {
                        tracing::warn!("Failed to store gate observation: {}", e);
                    }
                }
            }
        }
    }

    // Store implicit GrowthSignal on Light-tier PassThrough
    if let GateResult::PassThrough {
        tier: InferenceTier::Light,
    } = &result
    {
        if let Some(hid) = human_id {
            if let Ok(mut conn) = get_conn(pool) {
                let obs_id = uuid::Uuid::new_v4().to_string();
                let now = crate::db::models::current_timestamp();
                let new_obs = crate::db::models::NewImagodeiObservation {
                    id: &obs_id,
                    h_app_id: &ctx.h_app_id,
                    human_id: hid,
                    observed_at: &now,
                    observation_type: "growth_signal",
                    content: "Light-tier mutation proceeded without issues",
                    structured_signals_json: None,
                    trust_delta: 0.01,
                    visibility_layer: "individual",
                    originating_elohim: "gate",
                    relevance_decay: 0.5,
                    superseded_by: None,
                    dht_anchor_hash: None, // TODO(p2p-coherence): populate when trust attestation issued
                };
                let _ =
                    crate::db::imagodei_observations::create_observation(&mut conn, ctx, &new_obs);
            }
        }
    }

    // Store pending confirmation for Pause flow
    if let GateResult::Pause { confirm_token, .. } = &result {
        if let Some(hid) = human_id {
            use crate::services::elohim_gate::PendingConfirmation;
            svc.gate.pending_confirmations().store(
                confirm_token,
                PendingConfirmation {
                    mutation,
                    mutation_content: mutation_content_for_cache,
                    human_id: hid.to_string(),
                    created_at: std::time::Instant::now(),
                },
            );
        }
    }

    let view = build_gate_view(&result, &trust_ctx);
    (result, Some(view))
}

/// Build a GateEvaluationView from a GateResult + TrustContext
fn build_gate_view(result: &GateResult, ctx: &TrustContext) -> GateEvaluationView {
    let trust_view = TrustContextView {
        composite_trust: ctx.composite_trust,
        mastery_depth: ctx.mastery_depth,
        steward_standing: ctx.steward_standing,
        relationship_density: ctx.relationship_density,
        governance_health: ctx.governance_health,
        behavioral_trust: ctx.behavioral_trust,
        intent_divergence: ctx.intent_divergence,
        declared_intent: ctx.declared_intent.clone(),
    };
    match result {
        GateResult::PassThrough { tier } => GateEvaluationView {
            tier: format!("{:?}", tier),
            trust_context: trust_view,
            pause_prompt: None,
            confirm_token: None,
            settlement_boundary: None,
            appeal_path: None,
        },
        GateResult::Enriched { tier, .. } => GateEvaluationView {
            tier: format!("{:?}", tier),
            trust_context: trust_view,
            pause_prompt: None,
            confirm_token: None,
            settlement_boundary: None,
            appeal_path: None,
        },
        GateResult::Pause {
            tier,
            prompt,
            confirm_token,
            ..
        } => GateEvaluationView {
            tier: format!("{:?}", tier),
            trust_context: trust_view,
            pause_prompt: Some(prompt.clone()),
            confirm_token: Some(confirm_token.clone()),
            settlement_boundary: None,
            appeal_path: None,
        },
        GateResult::Settlement {
            tier,
            boundary,
            appeal_path,
            ..
        } => GateEvaluationView {
            tier: format!("{:?}", tier),
            trust_context: trust_view,
            pause_prompt: None,
            confirm_token: None,
            settlement_boundary: Some(boundary.clone()),
            appeal_path: appeal_path.clone(),
        },
    }
}

/// Helper to parse JSON request body
pub async fn parse_body<T: serde::de::DeserializeOwned>(
    req: Request<Incoming>,
) -> Result<T, StorageError> {
    use http_body_util::BodyExt;
    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|e| StorageError::InvalidInput(format!("Failed to read body: {}", e)))?
        .to_bytes();
    serde_json::from_slice(&body_bytes)
        .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))
}
