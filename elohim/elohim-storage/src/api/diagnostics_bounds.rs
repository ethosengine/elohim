//! Diagnostic route: POST /api/v1/diagnostics/validate-bounds.
//!
//! Accepts a `ValidateBoundsRequest` (an `EventForValidation` payload), runs
//! `bounds_validator::validate` against the production `ConductorCommitmentFetcher`
//! and `DieselRateHistory`, and returns a `BoundsValidationResultView` reporting
//! pass/fail + per-check status + first violation (if any).
//!
//! Source of truth: pure function result; no persisted entity.
//!
//! Per Sprint 2 plan: until Sprint 1 wires `ConductorCommitmentFetcher` to a real
//! `mishpat::get_commitment` zome call, this route will always return
//! `pass: false, violation.kind: "commitment_not_found"` (or "conductor_unreachable"
//! depending on how the production fetcher fails). That's the correct behavior
//! for the current substrate state.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::DbPool;
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::services::bounds_validator::{validate, BoundsViolation, EventForValidation};
use crate::services::commitment_fetcher::ConductorCommitmentFetcher;
use crate::services::rate_history::DieselRateHistory;
use elohim_views::bounds::{
    BoundsChecksView, BoundsValidationResultView, BoundsViolationView,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateBoundsRequest {
    pub event: EventForValidationWire,
}

/// Wire shape for the event input. Mirrors `EventForValidation` but accepts
/// camelCase JSON. Converted to `EventForValidation` before calling validate.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventForValidationWire {
    pub action: String,
    pub performer: String,
    pub bounded_by: String,
    pub target_epr_id: String,
    pub reach: String,
    pub signed_at: String,
}

impl From<EventForValidationWire> for EventForValidation {
    fn from(w: EventForValidationWire) -> Self {
        Self {
            action: w.action,
            performer: w.performer,
            bounded_by: w.bounded_by,
            target_epr_id: w.target_epr_id,
            reach: w.reach,
            signed_at: w.signed_at,
        }
    }
}

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    pool: &DbPool,
    hc_lamad: Option<&Arc<HcClient>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::POST {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(
                r#"{"error":"only POST is supported for /api/v1/diagnostics/validate-bounds"}"#,
            )))
            .unwrap());
    }

    // Parse body
    use http_body_util::BodyExt;
    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|e| StorageError::InvalidInput(format!("read body: {e}")))?
        .to_bytes();
    let request: ValidateBoundsRequest = serde_json::from_slice(&body_bytes)
        .map_err(|e| StorageError::InvalidInput(format!("parse body: {e}")))?;
    let event: EventForValidation = request.event.into();

    // Build production dependencies.
    let hc_client = match hc_lamad {
        Some(h) => h.clone(),
        None => {
            // No conductor wired — return a violation-shaped response rather than 500
            let view = BoundsValidationResultView {
                pass: false,
                commitment_cid: event.bounded_by.clone(),
                violation: Some(BoundsViolationView {
                    kind: elohim_views::bounds::ViolationKind::CommitmentNotFound,
                    summary: "conductor not wired (Sprint 1 dependency)".into(),
                }),
                checks: BoundsChecksView::default(),
            };
            return ok_json(&view);
        }
    };
    let fetcher = ConductorCommitmentFetcher { hc_client };
    let rate = DieselRateHistory { pool: pool.clone() };

    // Run validate
    let result = validate(&event, &fetcher, &rate).await;
    let view = match result {
        Ok(checks) => BoundsValidationResultView {
            pass: true,
            commitment_cid: event.bounded_by.clone(),
            violation: None,
            checks,
        },
        Err(BoundsViolation {
            kind,
            commitment_cid,
            summary,
            checks,
        }) => BoundsValidationResultView {
            pass: false,
            commitment_cid,
            violation: Some(BoundsViolationView { kind, summary }),
            checks,
        },
    };

    ok_json(&view)
}

fn ok_json<T: serde::Serialize>(value: &T) -> Result<Response<Full<Bytes>>, StorageError> {
    let body = serde_json::to_vec(value)
        .map_err(|e| StorageError::Internal(format!("serialize: {e}")))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
