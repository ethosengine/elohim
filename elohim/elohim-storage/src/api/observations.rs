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
//!     → `Vec<ObservationView>` (newest first). Never an `agent-private` kind
//!     (404); see [`observations_by_subject`] and ruling R-A9.
//!   - `GET /api/v1/observations/by-observer?observerCid=X[&kind=Y]`
//!     → `Vec<ObservationView>` (newest first). The observer's own rows only:
//!     the explicit `X-Agent-Cid` must equal `observerCid` (401 / 403); see
//!     [`observations_by_observer`] and ruling R-A9.
//!   - `GET /api/v1/observations/diversity?subjectCid=X&kind=Y`
//!     → `Option<ObservationDiversitySummaryView>`. Never counts an
//!     `agent-private` kind (404); see [`observation_diversity`].
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
use crate::observation::recipe::LensSpec;
use crate::observation::stream::{
    render_stream_counted, resolve_frame, OutsideWindow, RecipeInUse, StreamParams, StreamRow,
};
use crate::observation::wire::Observation;
use crate::services::observation_kinds::ObservationKindRegistry;
use crate::services::response;
use crate::views::{
    ObservationAcceptedView, ObservationDiversitySummaryView, ObservationIntentView,
    ObservationStreamView, ObservationView,
};
use diesel::prelude::*;

use super::get_conn;

/// `observerCidNamespace` on every ack: the observer is the caller's header
/// identifier as asserted (browser: a session human id; desktop: an agent key).
/// The payload's `ref_cid` is as-asserted too: today it carries the content's
/// route slug (the id in `/lamad/resource/:id`), not a content address, and it
/// is checked only as a string equal to the intent's `subjectCid`.
pub const OBSERVER_CID_NAMESPACE_AS_ASSERTED: &str = "as-asserted";

/// `signed` on every ack until the signing graduation: rows carry no signature.
pub const SIGNATURE_ABSENT: &str = "absent";

/// The largest `POST /api/v1/observations` body the node reads (16 KiB).
pub const MAX_OBSERVATION_BODY_BYTES: usize = 16 * 1024;

/// How far ahead of the node's clock a client-supplied `observedAt` may be
/// (seconds): clock skew, not a future observation.
pub const OBSERVED_AT_MAX_SKEW_SECS: i64 = 300;

/// The content reaches whose title the lifestream may print: the reaches a
/// node serves to anyone (`blob_reach::serves_anonymously`). Inlined as a
/// `reach IN (…)` filter on the title query rather than calling the content
/// route's reach helper, which is being reshaped alongside this change.
const TITLE_READABLE_REACHES: [&str; 2] = ["commons", "public"];

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

/// The status this module answers an error with: [`route_error`] maps every
/// variant its handlers raise (`Auth` → 401, `Forbidden` → 403, `InvalidInput`
/// → 400, `NotFound` → 404).
///
/// This exists as a named function because the router's last seam
/// (`HttpServer::escaped_error_response`) turns EVERY escaping error into a bare
/// 500 by design — a handler that did not choose its own status does not get one
/// invented for it there. So an observation refused for want of an
/// `X-Agent-Cid` reached the browser as `500 Authentication error: …`: the
/// message said 401 and the status said the node had broken. A client cannot
/// tell "sign in again" from "the node is down" on a 500, and the sprint's own
/// mesh proof read it as a route that did not exist (ruling R-A12).
pub fn route_error(err: StorageError) -> Response<Full<Bytes>> {
    response::error_response(err)
}

/// Handle `/api/v1/observations*` requests.
///
/// Infallible on purpose: every handler's error is mapped by [`route_error`]
/// here, so nothing from this module can escape to the router's blanket 500.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
    manager: &Arc<ObservationManagerBackend>,
) -> Response<Full<Bytes>> {
    let path = resource_path.trim_start_matches('/');

    let answered = match (&method, path) {
        (&Method::POST, "") => handle_post(req, pool, manager).await,
        (&Method::GET, "stream") => handle_stream(req, pool),
        (&Method::GET, "by-subject") => handle_by_subject(req, pool, manager).await,
        (&Method::GET, "by-observer") => handle_by_observer(req, pool).await,
        (&Method::GET, "diversity") => handle_diversity(req, pool, manager).await,
        (_, "" | "stream" | "by-subject" | "by-observer" | "diversity") => {
            Ok(response::method_not_allowed())
        }
        _ => Ok(response::not_found(&format!(
            "Unknown observations route: /api/v1/observations/{}",
            path
        ))),
    };

    answered.unwrap_or_else(route_error)
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

/// `POST /api/v1/observations` — read the explicit header and the body, then
/// [`accept_observation`]. 201 with the ack; errors map through [`route_error`]
/// at the dispatcher (Auth → 401, Forbidden → 403, InvalidInput → 400).
async fn handle_post(
    req: Request<Incoming>,
    pool: &DbPool,
    manager: &ObservationManagerBackend,
) -> Result<Response<Full<Bytes>>, StorageError> {
    use http_body_util::{BodyExt, Limited};
    let header = super::account::extract_agent_cid_explicit(&req);
    // Read at most one byte past the cap, so `accept_observation` refuses an
    // oversized body with its own message instead of the node buffering it all.
    let body = Limited::new(req.into_body(), MAX_OBSERVATION_BODY_BYTES + 1)
        .collect()
        .await
        .map_err(|e| {
            StorageError::InvalidInput(format!(
                "observation intent: body unreadable or over {MAX_OBSERVATION_BODY_BYTES} bytes: {e}"
            ))
        })?
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
/// 2. a body over [`MAX_OBSERVATION_BODY_BYTES`] → `InvalidInput` (400);
/// 3. a body `observerCid` other than the header → `Forbidden` (403), checked
///    before anything else is read so a mismatched body learns nothing more;
/// 4. the intent must parse (unknown fields refused) → `InvalidInput` (400);
/// 5. a client-supplied `observedAt` must lie in `0 ..= now + 300` → 400;
/// 6. the kind must be manifest-declared → 400 `unknown observation kind <k>`;
/// 7. `payloadJson` must be JSON matching the kind's field map → 400 with the
///    registry's reason;
/// 8. a payload carrying `ref_cid` must name the intent's `subjectCid` → 400;
/// 9. `seq` = the observer's `max(seq) + 1`; the row is appended through the
///    manager (log head stamped and persisted with the row), with an empty
///    signature;
/// 10. the gossip gate ([`crate::p2p::observation_gossip::announcement_for`])
///     runs on the stamped row: an agent-private kind yields no announcement.
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

    if body.len() > MAX_OBSERVATION_BODY_BYTES {
        return Err(StorageError::InvalidInput(format!(
            "observation intent: body is {} bytes; the limit is {MAX_OBSERVATION_BODY_BYTES}",
            body.len()
        )));
    }

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
    if let Some(at) = intent.observed_at {
        let latest = now.saturating_add(OBSERVED_AT_MAX_SKEW_SECS);
        if !(0..=latest).contains(&at) {
            return Err(StorageError::InvalidInput(format!(
                "observation intent: observedAt {at} is outside 0..={latest} (unix seconds, at \
                 most {OBSERVED_AT_MAX_SKEW_SECS}s ahead of the node's clock)"
            )));
        }
    }

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
    if let Some(ref_cid) = payload.get("ref_cid") {
        if ref_cid.as_str() != intent.subject_cid.as_deref() {
            return Err(StorageError::InvalidInput(format!(
                "{kind}: payload ref_cid must equal subjectCid (an observation names one subject)"
            )));
        }
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
/// arrangement is [`render_stream_counted`] through the compiled-in recipe,
/// with the window pushed into SQL (in-window rows loaded, the rest counted); a window
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

    let params = StreamParams {
        requester,
        as_of: q.as_of,
        window: q.window.as_deref(),
        lens: q.lens.as_deref(),
        kind: q.kind.as_deref(),
    };
    let recipe = RecipeInUse::compiled();
    let frame = resolve_frame(&recipe, &params, now)
        .map_err(|e| StorageError::InvalidInput(format!("observations/stream: {e}")))?;

    // The window is pushed into SQL: only in-window rows are loaded; the rows
    // the lens selects outside it are counted there for `total_count` and the
    // `window:` omissions.
    let query_failed = |e: diesel::result::Error| {
        StorageError::Internal(format!("observations query failed: {e}"))
    };
    let rows: Vec<crate::db::models::ObservationRow> =
        lens_selected(requester, frame.lens, params.kind, false)
            .filter(observations::observed_at.ge(frame.window_start))
            .filter(observations::observed_at.le(frame.as_of))
            .load(conn)
            .map_err(query_failed)?;
    let older: i64 = lens_selected(requester, frame.lens, params.kind, true)
        .filter(observations::observed_at.lt(frame.window_start))
        .count()
        .get_result(conn)
        .map_err(query_failed)?;
    let later: i64 = lens_selected(requester, frame.lens, params.kind, true)
        .filter(observations::observed_at.gt(frame.as_of))
        .count()
        .get_result(conn)
        .map_err(query_failed)?;
    let outside = OutsideWindow {
        older: u64::try_from(older).unwrap_or(0),
        later: u64::try_from(later).unwrap_or(0),
    };

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

    render_stream_counted(&rows, outside, &recipe, &params, now)
        .map_err(|e| StorageError::InvalidInput(format!("observations/stream: {e}")))
}

/// The requester's rows the lens (and the request's `kind`) select, as a
/// query the caller narrows by time.
///
/// `with_dwell_floor` adds the lens's `min_dwell_ms` as a SQL predicate. The
/// counting queries need it (their rows are never loaded); the in-window load
/// leaves the floor to [`render_stream_counted`], whose payload reading is the
/// authority. The predicate mirrors that reading: a payload that is not a JSON
/// object with non-negative integer `dwell_ms` / `scroll_depth_pct` (either
/// may be absent or null) reads as zero dwell. The outer `CASE` keeps the
/// `json_*` calls off malformed text, which SQLite would raise on.
fn lens_selected<'a>(
    requester: &'a str,
    lens: &'a LensSpec,
    kind: Option<&'a str>,
    with_dwell_floor: bool,
) -> observations::BoxedQuery<'a, diesel::sqlite::Sqlite> {
    let mut q = observations::table
        .filter(observations::observer_cid.eq(requester))
        .into_boxed();
    if let Some(k) = lens.kind.as_deref() {
        q = q.filter(observations::observation_kind.eq(k));
    }
    if let Some(k) = kind {
        q = q.filter(observations::observation_kind.eq(k));
    }
    match lens.min_dwell_ms {
        // The floor is the compiled recipe's integer, never request input.
        Some(min) if with_dwell_floor && min > 0 => {
            q = q.filter(diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                "(CASE WHEN json_valid(payload_json) AND json_type(payload_json) = 'object' THEN \
                   (CASE WHEN json_type(payload_json, '$.dwell_ms') = 'integer' \
                          AND json_extract(payload_json, '$.dwell_ms') >= {min} \
                          AND (json_type(payload_json, '$.scroll_depth_pct') IS NULL \
                               OR json_type(payload_json, '$.scroll_depth_pct') = 'null' \
                               OR (json_type(payload_json, '$.scroll_depth_pct') = 'integer' \
                                   AND json_extract(payload_json, '$.scroll_depth_pct') >= 0)) \
                         THEN 1 ELSE 0 END) \
                 ELSE 0 END) = 1"
            )));
        }
        _ => {}
    }
    q
}

/// Titles of the subjects this node holds as content, keyed by the subject
/// CID (matched against `content.id` or `content.blob_cid`). Only content at
/// commons or public reach lends its title ([`TITLE_READABLE_REACHES`]): a
/// narrower-reach title never rides into the view. Optional by design: a
/// failed lookup is logged and yields no titles.
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
            .filter(content::reach.eq_any(TITLE_READABLE_REACHES))
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
    manager: &ObservationManagerBackend,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let views = observations_by_subject(
        &mut conn,
        manager.kind_registry().map(|r| r.as_ref()),
        req.uri().query().unwrap_or(""),
    )?;
    Ok(response::ok(&views))
}

/// Every observer's rows of one kind about one subject, newest first.
///
/// A cross-observer read, so it never lists an `agent-private` kind (ruling
/// R-A9): see [`refuse_agent_private`] for the 404 and why it is not `[]`.
pub fn observations_by_subject(
    conn: &mut SqliteConnection,
    registry: Option<&ObservationKindRegistry>,
    query: &str,
) -> Result<Vec<ObservationView>, StorageError> {
    let q: BySubjectQuery = serde_urlencoded::from_str(query).map_err(|e| {
        StorageError::InvalidInput(format!(
            "observations/by-subject: missing or invalid query params: {e}"
        ))
    })?;
    refuse_agent_private("by-subject", registry, &q.kind)?;
    let rows: Vec<crate::db::models::ObservationRow> = observations::table
        .filter(observations::subject_cid.eq(&q.subject_cid))
        .filter(observations::observation_kind.eq(&q.kind))
        .order(observations::observed_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("observations query failed: {e}")))?;
    Ok(rows.into_iter().map(ObservationView::from).collect())
}

/// `GET /api/v1/observations/by-observer?observerCid=X[&kind=Y]`
async fn handle_by_observer(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let header = super::account::extract_agent_cid_explicit(&req);
    let mut conn = get_conn(pool)?;
    let views = observations_by_observer(
        &mut conn,
        header.as_deref(),
        req.uri().query().unwrap_or(""),
    )?;
    Ok(response::ok(&views))
}

/// One observer's rows (optionally of one kind), newest first — shown only to
/// that observer (ruling R-A9).
///
/// `header_agent_cid` is the explicit `X-Agent-Cid` header, never the
/// `local_sessions` fallback: no (or empty) header → `Auth` (401); a header
/// other than `observerCid` → `Forbidden` (403), checked before any row is
/// read. The observer sees every kind of their own, private ones included.
pub fn observations_by_observer(
    conn: &mut SqliteConnection,
    header_agent_cid: Option<&str>,
    query: &str,
) -> Result<Vec<ObservationView>, StorageError> {
    let requester = match header_agent_cid {
        Some(h) if !h.is_empty() => h,
        _ => {
            return Err(StorageError::Auth(
                "GET /api/v1/observations/by-observer requires the X-Agent-Cid header: an \
                 observer's rows are shown only to that observer"
                    .into(),
            ))
        }
    };
    let q: ByObserverQuery = serde_urlencoded::from_str(query).map_err(|e| {
        StorageError::InvalidInput(format!(
            "observations/by-observer: missing or invalid query params: {e}"
        ))
    })?;
    if q.observer_cid != requester {
        return Err(StorageError::Forbidden(
            "observations/by-observer: observerCid names another observer; a person reads only \
             their own observations"
                .into(),
        ));
    }

    // Build a boxed query so we can optionally filter by kind.
    let base = observations::table
        .filter(observations::observer_cid.eq(&q.observer_cid))
        .order(observations::observed_at.desc())
        .into_boxed();
    let rows: Vec<crate::db::models::ObservationRow> = if let Some(kind) = &q.kind {
        base.filter(observations::observation_kind.eq(kind))
            .load(conn)
    } else {
        base.load(conn)
    }
    .map_err(|e| StorageError::Internal(format!("observations query failed: {e}")))?;
    Ok(rows.into_iter().map(ObservationView::from).collect())
}

/// `GET /api/v1/observations/diversity?subjectCid=X&kind=Y`
async fn handle_diversity(
    req: Request<Incoming>,
    pool: &DbPool,
    manager: &ObservationManagerBackend,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let summary = observation_diversity(
        &mut conn,
        manager.kind_registry().map(|r| r.as_ref()),
        req.uri().query().unwrap_or(""),
    )?;
    Ok(response::ok(&summary))
}

/// How many distinct observers witnessed one kind about one subject.
///
/// A cross-observer aggregate, so it never counts an `agent-private` kind
/// (ruling R-A9; [`refuse_agent_private`]).
pub fn observation_diversity(
    conn: &mut SqliteConnection,
    registry: Option<&ObservationKindRegistry>,
    query: &str,
) -> Result<Option<ObservationDiversitySummaryView>, StorageError> {
    let q: DiversityQuery = serde_urlencoded::from_str(query).map_err(|e| {
        StorageError::InvalidInput(format!(
            "observations/diversity: missing or invalid query params: {e}"
        ))
    })?;
    refuse_agent_private("diversity", registry, &q.kind)?;
    let row: Option<crate::db::models::ObservationDiversitySummaryRow> =
        observation_diversity_summary::table
            .filter(observation_diversity_summary::subject_cid.eq(&q.subject_cid))
            .filter(observation_diversity_summary::observation_kind.eq(&q.kind))
            .first(conn)
            .optional()
            .map_err(|e| {
                StorageError::Internal(format!("observation_diversity_summary query failed: {e}"))
            })?;
    Ok(row.map(ObservationDiversitySummaryView::from))
}

/// The privacy gate on the cross-observer reads (`by-subject`, `diversity`):
/// a kind the registry marks `agent-private` is never listed or counted
/// across observers (ruling R-A9).
///
/// It answers `NotFound` (404) with the reason, the same whether or not rows
/// exist, rather than an empty `[]` / `null`: both routes' wire shapes have no
/// `omissions` to carry a reason, and an empty answer would read as "nobody
/// observed this" — a false statement about the world. A node with no registry
/// cannot tell private kinds apart, so it refuses every cross-observer read
/// (fail closed) rather than guess.
fn refuse_agent_private(
    route: &str,
    registry: Option<&ObservationKindRegistry>,
    kind: &str,
) -> Result<(), StorageError> {
    let Some(registry) = registry else {
        return Err(StorageError::NotFound(format!(
            "observations/{route}: this node has no observation-kind registry, so it cannot tell \
             which kinds are agent-private; nothing is listed across observers"
        )));
    };
    if registry.get(kind).is_some_and(|d| d.is_agent_private()) {
        return Err(StorageError::NotFound(format!(
            "observations/{route}: {kind} is agent-private; it stays on each observer's node and \
             is never listed or counted across observers"
        )));
    }
    Ok(())
}
