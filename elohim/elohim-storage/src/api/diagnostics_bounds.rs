//! Diagnostic route: POST /api/v1/diagnostics/validate-bounds.
//!
//! Accepts a `ValidateBoundsRequest` (an `EventForValidation` payload), runs
//! `bounds_validator::validate` against the production `ProjectionCommitmentFetcher`
//! (reads the `mishpat_commitments` projection table — P1: storage is the
//! projection of DHT truth) and `DieselRateHistory`, and returns a
//! `BoundsValidationResultView` reporting pass/fail + per-check status + first
//! violation (if any).
//!
//! Source of truth: pure function result; no persisted entity.
//!
//! Note: un-notarized rows (`dht_anchor_hash IS NULL`) cause the fetcher to
//! return `FetchError::NotarizedRequired`, which `bounds_validator` maps to
//! `ViolationKind::CommitmentNotFound` (fail-closed — spec §6.5). Commitments
//! not yet in the projection table return `Ok(None)` → same violation.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::DbPool;
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::services::bounds_validator::{validate, BoundsViolation, EventForValidation};
use crate::services::commitment_fetcher::ProjectionCommitmentFetcher;
use crate::services::rate_history::DieselRateHistory;
use elohim_views::bounds::{BoundsValidationResultView, BoundsViolationView};

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
    // hc_lamad is retained in the signature for API compatibility; the
    // production fetcher now reads the mishpat_commitments projection table
    // (P1 path) rather than a live conductor call. The conductor path
    // (ConductorCommitmentFetcher) will be re-wired in T8/Sprint-1 once the
    // mishpat::get_commitment zome function lands.
    _hc_lamad: Option<&Arc<HcClient>>,
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
    // ProjectionCommitmentFetcher reads the mishpat_commitments projection
    // table (P1: storage as reconciliation controller). Bounds-checks read
    // the read-optimised cache, not a live conductor.
    let fetcher = ProjectionCommitmentFetcher::new(pool.clone());
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
    let body =
        serde_json::to_vec(value).map_err(|e| StorageError::Internal(format!("serialize: {e}")))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
