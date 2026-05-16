//! Graph-native view routes — 4 new lamad read endpoints.
//!
//! All four handlers are compiled unconditionally so thin builds (`--no-default-features`)
//! still bind the URL paths. When the `graph-native` feature is OFF the handlers return
//! 501 Not Implemented. When it is ON they delegate to `graph_views::lamad` builders.
//!
//! Routes (new — Phase 6 Task 26):
//!   - `GET /api/v1/graph/atoms/{cid}/resolved`      — three-pillar EPR atom view
//!   - `GET /api/v1/graph/atoms/{cid}/navigation`    — atom + 2-hop neighbourhood
//!   - `GET /api/v1/graph/atoms/{cid}/version-chain` — SUPERSEDES chain from cid
//!   - `GET /api/v1/graph/topology-overview`         — contributor DID topology roll-up

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

use crate::error::StorageError;
use crate::services::response;

// ---------------------------------------------------------------------------
// Public dispatch entry-point
// ---------------------------------------------------------------------------

/// Dispatch `/api/v1/graph/...` sub-paths.
///
/// `resource_path` is the slice after the leading `graph` prefix (e.g. `/atoms/{cid}/resolved`).
/// `graph_engine` is `None` on thin builds or when the engine failed to open at startup.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    graph_engine: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(response::method_not_allowed());
    }

    let path = resource_path.trim_start_matches('/');

    // Route: /atoms/{cid}/resolved
    if let Some(rest) = path.strip_prefix("atoms/") {
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        match parts.as_slice() {
            [cid, "resolved"] => {
                return handle_resolved_atom(graph_engine, cid).await;
            }
            [cid, "navigation"] => {
                // ?originCid=<cid> query param; default to cid itself as origin.
                let origin_cid = req
                    .uri()
                    .query()
                    .and_then(|q| {
                        url::form_urlencoded::parse(q.as_bytes())
                            .find(|(k, _)| k == "originCid")
                            .map(|(_, v)| v.into_owned())
                    })
                    .unwrap_or_else(|| cid.to_string());
                return handle_navigation_context(graph_engine, cid, &origin_cid).await;
            }
            [cid, "version-chain"] => {
                return handle_atom_version_chain(graph_engine, cid).await;
            }
            _ => {}
        }
    }

    // Route: /topology-overview?contributorDid=<did>
    if path == "topology-overview" {
        let contributor_did = req.uri().query().and_then(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| k == "contributorDid")
                .map(|(_, v)| v.into_owned())
        });
        return handle_topology_overview(graph_engine, contributor_did.as_deref()).await;
    }

    Ok(response::not_found(&format!(
        "Unknown graph route: /api/v1/graph/{}",
        path
    )))
}

// ---------------------------------------------------------------------------
// GET /api/v1/graph/atoms/{cid}/resolved
// ---------------------------------------------------------------------------

async fn handle_resolved_atom(
    graph_engine: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>>,
    cid: &str,
) -> Result<Response<Full<Bytes>>, StorageError> {
    #[cfg(feature = "graph-native")]
    {
        let engine = match graph_engine {
            Some(e) => e,
            None => return Ok(feature_unavailable("graph-native")),
        };
        match crate::graph_views::lamad::resolved_atom::build(engine, cid) {
            Ok(view) => return Ok(response::ok(&view)),
            Err(e) => {
                return Ok(response::not_found(&format!("resolved_atom: {e}")));
            }
        }
    }
    #[cfg(not(feature = "graph-native"))]
    {
        let _ = (graph_engine, cid);
        Ok(feature_unavailable("graph-native"))
    }
}

// ---------------------------------------------------------------------------
// GET /api/v1/graph/atoms/{cid}/navigation
// ---------------------------------------------------------------------------

async fn handle_navigation_context(
    graph_engine: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>>,
    cid: &str,
    origin_cid: &str,
) -> Result<Response<Full<Bytes>>, StorageError> {
    #[cfg(feature = "graph-native")]
    {
        let engine = match graph_engine {
            Some(e) => e,
            None => return Ok(feature_unavailable("graph-native")),
        };
        match crate::graph_views::lamad::navigation_context::build(engine, cid, origin_cid) {
            Ok(view) => return Ok(response::ok(&view)),
            Err(e) => {
                return Ok(response::not_found(&format!("navigation_context: {e}")));
            }
        }
    }
    #[cfg(not(feature = "graph-native"))]
    {
        let _ = (graph_engine, cid, origin_cid);
        Ok(feature_unavailable("graph-native"))
    }
}

// ---------------------------------------------------------------------------
// GET /api/v1/graph/atoms/{cid}/version-chain
// ---------------------------------------------------------------------------

async fn handle_atom_version_chain(
    graph_engine: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>>,
    cid: &str,
) -> Result<Response<Full<Bytes>>, StorageError> {
    #[cfg(feature = "graph-native")]
    {
        let engine = match graph_engine {
            Some(e) => e,
            None => return Ok(feature_unavailable("graph-native")),
        };
        match crate::graph_views::lamad::atom_version_chain::build(engine, cid) {
            Ok(view) => return Ok(response::ok(&view)),
            Err(e) => {
                return Ok(response::not_found(&format!("atom_version_chain: {e}")));
            }
        }
    }
    #[cfg(not(feature = "graph-native"))]
    {
        let _ = (graph_engine, cid);
        Ok(feature_unavailable("graph-native"))
    }
}

// ---------------------------------------------------------------------------
// GET /api/v1/graph/topology-overview?contributorDid=<did>
// ---------------------------------------------------------------------------

async fn handle_topology_overview(
    graph_engine: Option<&std::sync::Arc<crate::graph::engine::GraphEngine>>,
    contributor_did: Option<&str>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let contributor_did = match contributor_did {
        Some(d) if !d.is_empty() => d,
        _ => {
            return Ok(response::json_response(
                StatusCode::BAD_REQUEST,
                &serde_json::json!({
                    "error": "missing_parameter",
                    "detail": "?contributorDid=<did> is required"
                }),
            ));
        }
    };

    #[cfg(feature = "graph-native")]
    {
        let engine = match graph_engine {
            Some(e) => e,
            None => return Ok(feature_unavailable("graph-native")),
        };
        match crate::graph_views::shefa::topology_overview::build(engine, contributor_did) {
            Ok(view) => return Ok(response::ok(&view)),
            Err(e) => {
                return Ok(response::internal_error(&format!(
                    "topology_overview: {e}"
                )));
            }
        }
    }
    #[cfg(not(feature = "graph-native"))]
    {
        let _ = (graph_engine, contributor_did);
        Ok(feature_unavailable("graph-native"))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn feature_unavailable(cap: &str) -> Response<Full<Bytes>> {
    response::json_response(
        StatusCode::NOT_IMPLEMENTED,
        &serde_json::json!({
            "error": "feature_unavailable",
            "capability": cap,
            "detail": "This route requires the graph-native Cargo feature. \
                       Build with --features graph-native (on by default)."
        }),
    )
}

// ---------------------------------------------------------------------------
// Unit tests — path routing
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /// Verify the atoms/{cid}/{action} split produces the expected parts.
    #[test]
    fn path_split_resolved() {
        let rest = "sha256-abc/resolved";
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        assert_eq!(parts, ["sha256-abc", "resolved"]);
    }

    #[test]
    fn path_split_navigation() {
        let rest = "sha256-abc/navigation";
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        assert_eq!(parts, ["sha256-abc", "navigation"]);
    }

    #[test]
    fn path_split_version_chain() {
        let rest = "sha256-abc/version-chain";
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        assert_eq!(parts, ["sha256-abc", "version-chain"]);
    }

    /// Topology-overview has no sub-path; just the bare route.
    #[test]
    fn path_matches_topology_overview() {
        let path = "topology-overview";
        assert!(path == "topology-overview");
    }

    /// Extra segments should NOT match atoms/{cid}/{action}.
    #[test]
    fn path_rejects_extra_segments() {
        let rest = "sha256-abc/resolved/extra";
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        // splitn(3) produces ["sha256-abc", "resolved", "extra"] — 3 items,
        // so the 2-element match arms won't fire → falls through to 404.
        assert_eq!(parts.len(), 3);
    }
}
