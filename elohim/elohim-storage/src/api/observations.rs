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
//!   - `GET /api/v1/observations/stream?asOf&window&lens&kind` — the caller's
//!     OWN lifestream (`ObservationStreamView`), rendered through the
//!     observation-lifestream recipe. Reach is `observer_cid == X-Agent-Cid`
//!     (401 without it; no `local_sessions` fallback). See
//!     [`stream_observations`] and ruling R-A4.
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

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::diesel_schema::{content, observation_diversity_summary, observations};
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::observation::manager::ObservationManagerBackend;
use crate::observation::stream::{render_stream, RecipeInUse, StreamParams, StreamRow};
use crate::observation::wire::Observation;
use crate::services::response;
use crate::views::{
    ObservationAcceptedView, ObservationDiversitySummaryView, ObservationIntentView,
    ObservationStreamView, ObservationView,
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

/// Query params for `GET /api/v1/observations/stream`. Every field is
/// optional; an absent one takes the recipe's default. There is no observer
/// field: the stream is always the header's own.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamQuery {
    /// Unix epoch seconds the window ends at (default: now).
    #[serde(default)]
    pub as_of: Option<i64>,
    /// `Nd` | `Nh` (default: the recipe's `window_default`).
    #[serde(default)]
    pub window: Option<String>,
    /// A key of the recipe's lens table (default: `all`).
    #[serde(default)]
    pub lens: Option<String>,
    /// Keep only this observation kind.
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
        (&Method::GET, "stream") => handle_stream(req, pool),
        (&Method::GET, "by-subject") => handle_by_subject(req, pool).await,
        (&Method::GET, "by-observer") => handle_by_observer(req, pool).await,
        (&Method::GET, "diversity") => handle_diversity(req, pool).await,
        (_, "" | "stream" | "by-subject" | "by-observer" | "diversity") => {
            Ok(response::method_not_allowed())
        }
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

/// `GET /api/v1/observations/stream` — read the explicit header and the query,
/// then [`stream_observations`].
fn handle_stream(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let header = super::account::extract_agent_cid_explicit(&req);
    let mut conn = get_conn(pool)?;
    let now = chrono::Utc::now().timestamp();
    let view = stream_observations(
        &mut conn,
        header.as_deref(),
        req.uri().query().unwrap_or(""),
        now,
    )?;
    Ok(response::ok(&view))
}

/// Render the caller's own lifestream: the route's single chokepoint.
///
/// `header_agent_cid` is the explicit `X-Agent-Cid` header — reach is
/// `observer_cid == header`, and there is no `local_sessions` fallback: a
/// request that asserts no identity sees nothing (`Auth`, 401). `query` is the
/// raw query string ([`StreamQuery`]; a malformed value is 400). Only the
/// requester's rows are loaded; titles are looked up by `subject_cid` against
/// `content.id` / `content.blob_cid` when the node holds the content, and a
/// failed lookup renders without titles rather than failing the stream. The
/// arrangement is [`render_stream`] through the compiled-in recipe; a window
/// or lens the recipe cannot render is `InvalidInput` (400).
pub fn stream_observations(
    conn: &mut SqliteConnection,
    header_agent_cid: Option<&str>,
    query: &str,
    now: i64,
) -> Result<ObservationStreamView, StorageError> {
    let requester =
        match header_agent_cid {
            Some(h) if !h.is_empty() => h,
            _ => return Err(StorageError::Auth(
                "GET /api/v1/observations/stream requires the X-Agent-Cid header: a lifestream \
                 is shown only to the person it belongs to"
                    .into(),
            )),
        };
    let q: StreamQuery = serde_urlencoded::from_str(query).map_err(|e| {
        StorageError::InvalidInput(format!("observations/stream: invalid query params: {e}"))
    })?;

    let rows: Vec<crate::db::models::ObservationRow> = observations::table
        .filter(observations::observer_cid.eq(requester))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("observations query failed: {e}")))?;

    let titles = subject_titles(conn, &rows);
    let rows: Vec<StreamRow> = rows
        .into_iter()
        .map(|observation| {
            let title = observation
                .subject_cid
                .as_ref()
                .and_then(|cid| titles.get(cid).cloned());
            StreamRow { observation, title }
        })
        .collect();

    let params = StreamParams {
        requester,
        as_of: q.as_of,
        window: q.window.as_deref(),
        lens: q.lens.as_deref(),
        kind: q.kind.as_deref(),
    };
    render_stream(&rows, &RecipeInUse::compiled(), &params, now)
        .map_err(|e| StorageError::InvalidInput(format!("observations/stream: {e}")))
}

/// Titles of the subjects this node holds as content, keyed by the subject
/// CID (matched against `content.id` or `content.blob_cid`). Optional by
/// design: a failed lookup is logged and yields no titles.
fn subject_titles(
    conn: &mut SqliteConnection,
    rows: &[crate::db::models::ObservationRow],
) -> HashMap<String, String> {
    let subjects: BTreeSet<&str> = rows
        .iter()
        .filter_map(|r| r.subject_cid.as_deref())
        .collect();
    let subjects: Vec<&str> = subjects.into_iter().collect();
    let mut titles = HashMap::new();
    for chunk in subjects.chunks(400) {
        let found: Result<Vec<(String, Option<String>, String)>, _> = content::table
            .filter(
                content::id
                    .eq_any(chunk)
                    .or(content::blob_cid.eq_any(chunk)),
            )
            .select((content::id, content::blob_cid, content::title))
            .load(conn);
        match found {
            Ok(found) => {
                for (id, blob_cid, title) in found {
                    if let Some(blob_cid) = blob_cid.filter(|c| chunk.contains(&c.as_str())) {
                        titles.entry(blob_cid).or_insert_with(|| title.clone());
                    }
                    if chunk.contains(&id.as_str()) {
                        titles.entry(id).or_insert(title);
                    }
                }
            }
            Err(e) => {
                tracing::debug!(error = %e, "observation stream: title lookup failed; rendering without titles");
                break;
            }
        }
    }
    titles
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
