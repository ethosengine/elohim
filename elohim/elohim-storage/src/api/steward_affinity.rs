//! Steward Affinity API controller
//!
//! Routes: `/api/v1/steward-affinity[/*]`

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::steward_affinity::{self, AffinityQuery};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::services::Services;
use crate::views::{
    BulkCreateStewardAffinityInputView, CreateStewardAffinityInputView, StewardAffinityView,
};

use super::{get_conn, parse_body};

/// Handle `/api/v1/steward-affinity*` requests
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
        (&Method::GET, "") => handle_list(req, pool, ctx).await,
        (&Method::POST, "") => handle_create(req, pool, ctx).await,
        (&Method::POST, "bulk") => handle_bulk_create(req, pool, ctx).await,
        (&Method::POST, "curation-event") => handle_curation_event(req, pool, ctx, services).await,
        (&Method::GET, id) if !id.contains('/') => handle_get_by_id(id, pool, ctx).await,

        _ => Ok(response::not_found(&format!(
            "Unknown steward-affinity route: {} /api/v1/steward-affinity/{}",
            method, path
        ))),
    }
}

async fn handle_list(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query: AffinityQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).unwrap_or_default();
    let mut conn = get_conn(pool)?;
    let affinities = steward_affinity::list_affinities(&mut conn, ctx, &query)?;
    let views: Vec<StewardAffinityView> = affinities.into_iter().map(Into::into).collect();
    Ok(response::ok(&views))
}

async fn handle_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: CreateStewardAffinityInputView = parse_body(req).await?;
    let db_input = input.into();
    let mut conn = get_conn(pool)?;
    let affinity = steward_affinity::create_affinity(&mut conn, ctx, &db_input)?;
    let view = StewardAffinityView::from(affinity);
    Ok(response::created(&view))
}

async fn handle_bulk_create(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: BulkCreateStewardAffinityInputView = parse_body(req).await?;
    let db_inputs: Vec<_> = input.affinities.into_iter().map(Into::into).collect();
    let mut conn = get_conn(pool)?;
    let (created, errors) = steward_affinity::bulk_create_affinities(&mut conn, ctx, &db_inputs)?;
    Ok(response::ok(&serde_json::json!({
        "created": created,
        "errors": errors,
    })))
}

async fn handle_curation_event(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
    services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let input: crate::views::CurationEventInputView = parse_body(req).await?;

    // Evaluate curation event through the ElohimGate (Sprint 1: always PassThrough)
    if let Some(ref svc) = services {
        use crate::services::elohim_gate::{MutationType, TrustContext};
        use constitution::ConstitutionalLayer;

        // Sprint 1: placeholder TrustContext with neutral signals
        // Sprint 2: gather real signals from session + steward standing
        let trust_ctx = TrustContext {
            human_id: input.steward_id.clone(),
            session_id: String::new(),
            mastery_depth: 0.5,
            steward_standing: 0.5,
            relationship_density: 0.5,
            governance_health: 0.5,
            behavioral_trust: 0.5,
            intent_divergence: 0.5,
            composite_trust: 0.5,
            constitutional_layer: ConstitutionalLayer::Individual,
            community_id: None,
            family_id: None,
            declared_intent: None,
            computed_at: String::new(),
        };

        let gate_result = svc
            .gate
            .evaluate(
                MutationType::CurationEvent,
                &trust_ctx,
                serde_json::json!({
                    "stewardId": input.steward_id,
                    "contentId": input.content_id,
                    "activityType": input.activity_type,
                }),
            )
            .await;
        tracing::info!(tier = ?gate_result.tier(), "ElohimGate evaluated curation event");
    }

    let mut conn = get_conn(pool)?;

    let result = crate::services::steward_affinity_service::record_curation_activity(
        &mut conn,
        ctx,
        &input.steward_id,
        &input.content_id,
        &input.activity_type,
    )?;

    let view = StewardAffinityView::from(result);
    Ok(response::created(&view))
}

async fn handle_get_by_id(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let affinity = steward_affinity::get_affinity_by_id(&mut conn, ctx, id)?;
    let view = StewardAffinityView::from(affinity);
    Ok(response::ok(&view))
}
