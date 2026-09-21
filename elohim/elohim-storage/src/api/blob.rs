//! Blob-scoped views API controller.
//!
//! Routes:
//!   - `GET /api/v1/blob/{hash}/distribution/details` — visitor/steward branch over
//!     the same `compose_distribution_details` view service. Visitor sees the public
//!     replica/health surface; steward sees their own role + reciprocity hint.
//!   - `GET /api/v1/blob/{hash}/distribution/summary` — same Visitor/Steward branch
//!     over `compose_distribution_summary`. Added by the epr-head-envelope design
//!     (Option A, 2026-09-21): this is the per-peer distribution facts' new home
//!     after they were removed from the `GET /epr-head/{id}` response body, which
//!     must be a function of the declared head alone. Same visibility as
//!     `/details` — this design SHRINKS the anonymous topology-census exposure
//!     the head used to leak; it does not widen this route's own consent gradient.
//!
//! ## Source of Truth
//!
//! Operational (Category C). The view composes over notarized state:
//!   - `peer_blob_inventory` — replica observations (Category C derived from
//!     libp2p inventory exchange)
//!   - `content.reach` — content reach class (Category A from imagodei DHT)
//!   - `peer_identity_bindings` — agent→peer bindings (Category A derived from
//!     imagodei AgentPeerBinding entries)
//!   - `rea_commitments` / `economic_events` — reciprocity hint (Category A
//!     from REA notarization)
//!
//! No new entity is introduced here. The DHT remains canonical.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::models::PeerIdentityBindingRow;
use crate::db::{peer_identity_bindings, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::distribution_view::{
    compose_distribution_details, compose_distribution_summary, DistributionContext,
};
use crate::services::response;

use super::{account, get_conn};

/// Resolve the caller's identity inputs for the Visitor/Steward distribution
/// consent gradient — ONE computation shared by `/details` and `/summary`
/// (C13: two copies of a consent gradient is how it drifts). Returns owned
/// `(agent_cid, bindings)`; callers build a [`DistributionContext`] from these
/// via [`distribution_context_from`] since the context type borrows.
fn resolve_distribution_identity(
    req: &Request<Incoming>,
    conn: &mut diesel::SqliteConnection,
) -> Result<(Option<String>, Vec<PeerIdentityBindingRow>), StorageError> {
    let agent_cid_opt = account::extract_agent_cid(req, conn)?;

    let bindings = if let Some(ref cid) = agent_cid_opt {
        let now_iso = chrono::Utc::now().to_rfc3339();
        peer_identity_bindings::list_active_for_agent(conn, cid, &now_iso)
            .map_err(|e| StorageError::Internal(format!("bindings lookup failed: {}", e)))?
    } else {
        vec![]
    };

    Ok((agent_cid_opt, bindings))
}

/// Build the [`DistributionContext`] the identity tier wiring resolves to:
///   - No agent_cid → Visitor
///   - agent_cid + non-empty bindings → Steward
///   - agent_cid but empty bindings → Visitor (authed, no peer bindings yet —
///     no role to attribute)
fn distribution_context_from<'a>(
    agent_cid_opt: &'a Option<String>,
    bindings: &'a [PeerIdentityBindingRow],
) -> DistributionContext<'a> {
    match (agent_cid_opt, bindings.is_empty()) {
        (Some(cid), false) => DistributionContext::Steward {
            agent_cid: cid.as_str(),
            bindings,
        },
        _ => DistributionContext::Visitor,
    }
}

/// Dispatch `/api/v1/blob/...` sub-paths.
///
/// `resource_path` is the slice after the `blob` prefix: `/{hash}/distribution/details`,
/// `/{hash}/distribution/summary`, etc.
///
/// `graph_engine` is `None` when the `graph-native` feature is off or the engine
/// failed to open at startup. When `Some`, the graph-backed distribution path is preferred.
///
/// NOTE: graph-backed path returns graph-derived fields (steward counts, reach class)
/// with placeholders for system_metrics / blob_inventory / REA event fields. Full
/// composition lands in the follow-on sprint per Phase 5 architectural decision.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
    graph_engine: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Strip query string; we don't accept query params on these endpoints.
    let (path_only, _) = resource_path.split_once('?').unwrap_or((resource_path, ""));

    // Path shape: /{hash}/distribution/{summary|details}
    let parts: Vec<&str> = path_only.trim_start_matches('/').split('/').collect();

    match (&method, parts.as_slice()) {
        (&Method::GET, [hash, "distribution", "details"]) if !hash.is_empty() => {
            handle_distribution_details(req, pool, hash, graph_engine).await
        }
        (&Method::GET, [hash, "distribution", "summary"]) if !hash.is_empty() => {
            handle_distribution_summary(req, pool, hash).await
        }
        _ => Ok(response::not_found(&format!(
            "Unknown blob route: {} /api/v1/blob{}",
            method, resource_path
        ))),
    }
}

/// `GET /api/v1/blob/{hash}/distribution/details`
///
/// Visitor: returns the public surface (replicas, health, reach, freshness) with
///   `myRole = None` and `reciprocityEdges = None`.
/// Steward: same surface plus role + reciprocity edges.
///
/// Identity tier wiring (per progressive onboarding model):
///   - No `X-Agent-Cid` header AND no active local_session → Visitor
///   - Header or session yields agent_cid AND non-empty bindings → Steward
///   - Header or session yields agent_cid but empty bindings → Visitor (the
///     caller is authed but has no peer bindings yet, so no role to attribute)
///
/// NOTE: graph-backed path returns graph-derived fields with placeholders
/// for system_metrics / blob_inventory / REA event fields. Full composition
/// lands in the follow-on sprint per Phase 5 architectural decision.
async fn handle_distribution_details(
    req: Request<Incoming>,
    pool: &DbPool,
    hash: &str,
    graph_engine: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // graph-native branch: derive distribution details from STEWARDS + epr_qahal edges.
    #[cfg(feature = "graph-native")]
    if let Some(engine) = graph_engine {
        return match crate::graph_views::shefa::distribution::build_details(engine, hash) {
            Ok(view) => Ok(response::ok(&view)),
            Err(e) => Ok(response::internal_error(&format!(
                "graph distribution build_details failed: {e}"
            ))),
        };
    }
    let _ = graph_engine; // suppress unused warning on non-graph builds

    let mut conn = get_conn(pool)?;

    let (agent_cid_opt, bindings) = resolve_distribution_identity(&req, &mut conn)?;
    let ctx = distribution_context_from(&agent_cid_opt, &bindings);

    match compose_distribution_details(pool, hash, ctx).await {
        Ok(details) => Ok(response::ok(&details)),
        Err(e) => Ok(response::internal_error(&format!(
            "compose_distribution_details failed: {}",
            e
        ))),
    }
}

/// `GET /api/v1/blob/{hash}/distribution/summary`
///
/// Added by the epr-head-envelope design (Option A, 2026-09-21): the per-peer
/// distribution facts (replica count, projector count, diversity, caller
/// role) that used to be hydrated inline onto `GET /epr-head/{id}` now live
/// here exclusively — the head's body must be a function of the declared
/// head alone, and these facts genuinely differ between honest peers.
///
/// Same Visitor/Steward identity tier wiring as `/details`
/// ([`resolve_distribution_identity`] / [`distribution_context_from`]): one
/// consent gradient, shared, not duplicated (C13).
async fn handle_distribution_summary(
    req: Request<Incoming>,
    pool: &DbPool,
    hash: &str,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    let (agent_cid_opt, bindings) = resolve_distribution_identity(&req, &mut conn)?;
    let ctx = distribution_context_from(&agent_cid_opt, &bindings);

    match compose_distribution_summary(pool, hash, ctx).await {
        Ok(summary) => Ok(response::ok(&summary)),
        Err(e) => Ok(response::internal_error(&format!(
            "compose_distribution_summary failed: {}",
            e
        ))),
    }
}

// ---------------------------------------------------------------------------
// Unit tests — path parsing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /// The dispatcher's path-shape match must accept exactly
    /// `/{hash}/distribution/details`, where `{hash}` is non-empty. Anything
    /// else (extra segments, missing hash, wrong final segment) must miss.
    #[test]
    fn path_shape_matches_distribution_details() {
        let p = "/sha256-abc123/distribution/details";
        let parts: Vec<&str> = p.trim_start_matches('/').split('/').collect();
        assert_eq!(
            parts.as_slice(),
            ["sha256-abc123", "distribution", "details"]
        );
    }

    /// Empty-hash variant (URL `//distribution/details`): `trim_start_matches('/')`
    /// collapses the leading double slash, leaving 2 parts instead of 3, so
    /// the 3-element match arm never fires — the request falls through to 404.
    #[test]
    fn path_shape_rejects_empty_hash_via_segment_collapse() {
        let p = "//distribution/details";
        let parts: Vec<&str> = p.trim_start_matches('/').split('/').collect();
        // Only 2 parts: leading slashes are all stripped.
        assert_eq!(parts.as_slice(), ["distribution", "details"]);
        // Match arm requires 3 parts → this case naturally misses → 404.
    }

    /// Unknown final segment must miss. `summary` is now a real arm (added by
    /// the epr-head-envelope design, Option A) — this test used a `summary`
    /// path before that route existed, which would now be a false claim, so
    /// it asserts against a segment that is still unrecognised.
    #[test]
    fn path_shape_rejects_unknown_distribution_subroute() {
        let p = "/abc/distribution/ledger";
        let parts: Vec<&str> = p.trim_start_matches('/').split('/').collect();
        assert_eq!(parts.as_slice(), ["abc", "distribution", "ledger"]);
        assert_ne!(parts[2], "details");
        assert_ne!(parts[2], "summary");
    }

    /// The dispatcher's path-shape match must accept exactly
    /// `/{hash}/distribution/summary`, where `{hash}` is non-empty — the
    /// sibling arm added by the epr-head-envelope design (Option A): the
    /// per-peer distribution facts removed from `GET /epr-head/{id}` now
    /// live at this existing blob-scoped route instead of a new one.
    #[test]
    fn path_shape_matches_distribution_summary() {
        let p = "/sha256-abc123/distribution/summary";
        let parts: Vec<&str> = p.trim_start_matches('/').split('/').collect();
        assert_eq!(
            parts.as_slice(),
            ["sha256-abc123", "distribution", "summary"]
        );
    }
}
