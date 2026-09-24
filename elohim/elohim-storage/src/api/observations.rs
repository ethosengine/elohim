//! HTTP routes for the observation layer.
//!
//! Routes:
//!   - `POST /api/v1/observations` — the caller witnesses one of their OWN
//!     observations (`ObservationIntentView` → `ObservationAcceptedView`, 201).
//!     Self-rows only: the observer is the explicit `X-Agent-Cid` header,
//!     verbatim (401 without it; 403 if the body names another observer). The
//!     kind must be manifest-declared and the payload must match its declared
//!     field map (400). The row is unsigned and the ack says so. See
//!     [`accept_observation`] and ruling R-A2 of the post-station-4 plan.
//!   - `GET /api/v1/observations/by-subject?subjectCid=X&kind=Y`
//!     → `Vec<ObservationView>` (newest first)
//!   - `GET /api/v1/observations/by-observer?observerCid=X[&kind=Y]`
//!     → `Vec<ObservationView>` (newest first)
//!   - `GET /api/v1/observations/diversity?subjectCid=X&kind=Y`
//!     → `Option<ObservationDiversitySummaryView>`
//!
//! Source of truth for `observations`: libp2p gossip plane (agent-authored,
//! projected into SQLite by the observation manager). Category B.
//! Source of truth for `observation_diversity_summary`: SQLite aggregation
//! view over `observations`. Category C.
//!
//! Template: `agreements.rs` (serde_urlencoded query parsing,
//! `get_conn` + `response::ok` response helpers).

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::diesel_schema::{observation_diversity_summary, observations};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::observation::manager::ObservationManagerBackend;
use crate::observation::wire::Observation;
use crate::services::response;
use crate::views::{
    ObservationAcceptedView, ObservationDiversitySummaryView, ObservationIntentView,
    ObservationView,
};
use diesel::prelude::*;

use super::get_conn;

/// `observerCidNamespace` on every ack: the observer is the caller's header
/// identifier as asserted (browser: a session human id; desktop: an agent key).
pub const OBSERVER_CID_NAMESPACE_AS_ASSERTED: &str = "as-asserted";

/// `signed` on every ack until the signing graduation: rows carry no signature.
pub const SIGNATURE_ABSENT: &str = "absent";

// ---------------------------------------------------------------------------
// Query param structs
// ---------------------------------------------------------------------------

/// Query params for `GET /api/v1/observations/by-subject`.
/// camelCase because `CLAUDE.md` §Query Parameter Convention applies.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BySubjectQuery {
    pub subject_cid: String,
    pub kind: String,
}

/// Query params for `GET /api/v1/observations/by-observer`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ByObserverQuery {
    pub observer_cid: String,
    #[serde(default)]
    pub kind: Option<String>,
}

/// Query params for `GET /api/v1/observations/diversity`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiversityQuery {
    pub subject_cid: String,
    pub kind: String,
}

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/observations*` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
    manager: &Arc<ObservationManagerBackend>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        (&Method::POST, "") => handle_post(req, pool, manager).await,
        (&Method::GET, "by-subject") => handle_by_subject(req, pool).await,
        (&Method::GET, "by-observer") => handle_by_observer(req, pool).await,
        (&Method::GET, "diversity") => handle_diversity(req, pool).await,
        (_, "" | "by-subject" | "by-observer" | "diversity") => Ok(response::method_not_allowed()),
        _ => Ok(response::not_found(&format!(
            "Unknown observations route: /api/v1/observations/{}",
            path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

/// `POST /api/v1/observations` — read the explicit header and the body, then
/// [`accept_observation`]. 201 with the ack; errors map through
/// `response::error_response` (Auth → 401, Forbidden → 403, InvalidInput → 400).
async fn handle_post(
    req: Request<Incoming>,
    pool: &DbPool,
    manager: &ObservationManagerBackend,
) -> Result<Response<Full<Bytes>>, StorageError> {
    use http_body_util::BodyExt;
    let header = super::account::extract_agent_cid_explicit(&req);
    let body = req
        .into_body()
        .collect()
        .await
        .map_err(|e| StorageError::InvalidInput(format!("Failed to read body: {e}")))?
        .to_bytes();
    let mut conn = get_conn(pool)?;
    let now = chrono::Utc::now().timestamp();
    let ack = accept_observation(manager, &mut conn, header.as_deref(), &body, now).await?;
    Ok(response::created(&ack))
}

/// Accept one self-authored observation: the write path's single chokepoint.
///
/// `header_agent_cid` is the explicit `X-Agent-Cid` header (never the
/// `local_sessions` fallback — a request that asserts no identity writes
/// nothing). `now` is unix epoch seconds, used when the intent carries no
/// `observedAt`.
///
/// In order:
/// 1. no (or empty) header → `Auth` (401);
/// 2. a body `observerCid` other than the header → `Forbidden` (403), checked
///    before anything else is read so a mismatched body learns nothing more;
/// 3. the intent must parse (unknown fields refused) → `InvalidInput` (400);
/// 4. the kind must be manifest-declared → 400 `unknown observation kind <k>`;
/// 5. `payloadJson` must be JSON matching the kind's field map → 400 with the
///    registry's reason;
/// 6. `seq` = the observer's `max(seq) + 1`; the row is appended through the
///    manager (log head stamped and persisted with the row), with an empty
///    signature;
/// 7. the gossip gate ([`crate::p2p::observation_gossip::announcement_for`])
///    runs on the stamped row: an agent-private kind yields no announcement.
///
/// `seq` is read before the manager's append lock is taken, so two writes by
/// the same observer racing each other can share a `seq`; `log_offset` (under
/// the lock) stays the observer's total order.
pub async fn accept_observation(
    manager: &ObservationManagerBackend,
    conn: &mut SqliteConnection,
    header_agent_cid: Option<&str>,
    body: &[u8],
    now: i64,
) -> Result<ObservationAcceptedView, StorageError> {
    let observer = match header_agent_cid {
        Some(h) if !h.is_empty() => h.to_string(),
        _ => {
            return Err(StorageError::Auth(
                "POST /api/v1/observations requires the X-Agent-Cid header: an observation is \
                 written only by the person it belongs to"
                    .into(),
            ))
        }
    };

    let raw: serde_json::Value = serde_json::from_slice(body).map_err(|e| {
        StorageError::InvalidInput(format!("observation intent: invalid JSON: {e}"))
    })?;
    if let Some(named) = raw.get("observerCid") {
        if named.as_str() != Some(observer.as_str()) {
            return Err(StorageError::Forbidden(
                "observerCid names another observer: an observation is only ever written by \
                 the caller"
                    .into(),
            ));
        }
    }
    let intent: ObservationIntentView = serde_json::from_value(raw)
        .map_err(|e| StorageError::InvalidInput(format!("observation intent: {e}")))?;

    let kind = intent.observation_kind.as_str();
    let decl = manager
        .kind_registry()
        .and_then(|registry| registry.get(kind))
        .cloned()
        .ok_or_else(|| StorageError::InvalidInput(format!("unknown observation kind {kind}")))?;

    let payload: serde_json::Value = serde_json::from_str(&intent.payload_json).map_err(|e| {
        StorageError::InvalidInput(format!("{kind}: payloadJson is not valid JSON: {e}"))
    })?;
    if let Some(registry) = manager.kind_registry() {
        registry
            .validate_payload(kind, &payload)
            .map_err(StorageError::InvalidInput)?;
    }

    let last_seq: Option<i64> = observations::table
        .filter(observations::observer_cid.eq(&observer))
        .select(diesel::dsl::max(observations::seq))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("observation seq read failed: {e}")))?;
    let seq = u64::try_from(last_seq.unwrap_or(0)).unwrap_or(0) + 1;

    let observation = Observation {
        observer_cid: observer,
        log_cid: String::new(),
        log_offset: 0,
        observed_at: intent.observed_at.unwrap_or(now),
        seq,
        observation_kind: intent.observation_kind,
        subject_cid: intent.subject_cid,
        subject_kind: intent.subject_kind,
        payload_json: intent.payload_json,
        observer_household_cid: None,
        observer_collective_cid: None,
        observer_region: None,
        observer_archetype: None,
        observer_compute_class: None,
        signature: Vec::new(),
    };
    let stamped = manager
        .append_local(conn, observation)
        .await
        .map_err(|e| StorageError::Internal(format!("observation append failed: {e}")))?;

    #[cfg(feature = "p2p")]
    if let Some(announcement) = crate::p2p::observation_gossip::announcement_for(&decl, &stamped) {
        // No publisher exists yet (`publish_announcement` has no caller); the
        // gate's verdict is counted either way.
        tracing::debug!(
            target: "observation::gossip",
            kind = %announcement.kind,
            latest_offset = announcement.latest_offset,
            "observation cursor announceable (no publisher wired)"
        );
    }
    #[cfg(not(feature = "p2p"))]
    let _ = &decl;

    Ok(ObservationAcceptedView {
        observer_cid: stamped.observer_cid,
        observer_cid_namespace: OBSERVER_CID_NAMESPACE_AS_ASSERTED.to_string(),
        log_cid: stamped.log_cid,
        log_offset: stamped.log_offset,
        seq: stamped.seq,
        signed: SIGNATURE_ABSENT.to_string(),
    })
}

/// `GET /api/v1/observations/by-subject?subjectCid=X&kind=Y`
async fn handle_by_subject(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let q: BySubjectQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).map_err(|e| {
            StorageError::InvalidInput(format!(
                "observations/by-subject: missing or invalid query params: {}",
                e
            ))
        })?;

    let mut conn = get_conn(pool)?;
    let rows: Vec<crate::db::models::ObservationRow> = observations::table
        .filter(observations::subject_cid.eq(&q.subject_cid))
        .filter(observations::observation_kind.eq(&q.kind))
        .order(observations::observed_at.desc())
        .load(&mut conn)
        .map_err(|e| StorageError::Internal(format!("observations query failed: {}", e)))?;

    let views: Vec<ObservationView> = rows.into_iter().map(ObservationView::from).collect();
    Ok(response::ok(&views))
}

/// `GET /api/v1/observations/by-observer?observerCid=X[&kind=Y]`
async fn handle_by_observer(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let q: ByObserverQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).map_err(|e| {
            StorageError::InvalidInput(format!(
                "observations/by-observer: missing or invalid query params: {}",
                e
            ))
        })?;

    let mut conn = get_conn(pool)?;

    // Build a boxed query so we can optionally filter by kind.
    let base = observations::table
        .filter(observations::observer_cid.eq(&q.observer_cid))
        .order(observations::observed_at.desc())
        .into_boxed();

    let rows: Vec<crate::db::models::ObservationRow> = if let Some(kind) = &q.kind {
        base.filter(observations::observation_kind.eq(kind))
            .load(&mut conn)
    } else {
        base.load(&mut conn)
    }
    .map_err(|e| StorageError::Internal(format!("observations query failed: {}", e)))?;

    let views: Vec<ObservationView> = rows.into_iter().map(ObservationView::from).collect();
    Ok(response::ok(&views))
}

/// `GET /api/v1/observations/diversity?subjectCid=X&kind=Y`
async fn handle_diversity(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let q: DiversityQuery =
        serde_urlencoded::from_str(req.uri().query().unwrap_or("")).map_err(|e| {
            StorageError::InvalidInput(format!(
                "observations/diversity: missing or invalid query params: {}",
                e
            ))
        })?;

    let mut conn = get_conn(pool)?;
    let row: Option<crate::db::models::ObservationDiversitySummaryRow> =
        observation_diversity_summary::table
            .filter(observation_diversity_summary::subject_cid.eq(&q.subject_cid))
            .filter(observation_diversity_summary::observation_kind.eq(&q.kind))
            .first(&mut conn)
            .optional()
            .map_err(|e| {
                StorageError::Internal(format!("observation_diversity_summary query failed: {}", e))
            })?;

    Ok(response::ok(
        &row.map(ObservationDiversitySummaryView::from),
    ))
}
