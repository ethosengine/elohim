//! `GET /admin/bootstrap-coherence` — kitsune2 bootstrap skew read-model (Cat-C).
//!
//! A node-local Operational read-model that makes per-backend bootstrap state
//! observable: how many spaces/agents the bootstrap store currently holds and
//! which backend (`mem` | `mongo`) is serving them. With the shared mongo
//! backend live on both doorways, this is the surface that shows the genesis
//! pair has actually converged on ONE bootstrap table (vs. the old per-pod
//! islands).
//!
//! Cat-C: counts only, composed fresh per request from the in-process store —
//! no canonical authorship, fails the swap test by design (a node serves its
//! OWN runtime state), same class as `/admin/self-healing` and
//! `/admin/capability`. No auth (operator-only is an ingress property).

use std::sync::Arc;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use serde::Serialize;

use crate::server::AppState;

/// Per-space agent count (skew detail). Empty until a per-space stats method
/// lands on the `K2Store` trait — a forward seam, not a populated field today.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceSkew {
    pub space: String,
    pub agents: usize,
}

/// Bootstrap coherence read-model.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapCoherenceView {
    /// Backend serving the bootstrap table: `"mem"` (per-pod) | `"mongo"` (shared).
    pub backend: String,
    /// Distinct spaces currently held.
    pub spaces: usize,
    /// Total agents currently held across all spaces.
    pub agents: usize,
    /// Per-space agent counts. Empty today (totals-only stats); forward seam.
    pub per_space: Vec<SpaceSkew>,
}

/// Handle `GET /admin/bootstrap-coherence`.
pub async fn handle_bootstrap_coherence(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let view = match state.bootstrap.as_ref() {
        Some(store) => {
            let k2 = store.k2();
            let (agents, spaces) = k2.stats().await;
            BootstrapCoherenceView {
                backend: k2.backend_name().to_string(),
                spaces,
                agents,
                per_space: Vec::new(),
            }
        }
        None => BootstrapCoherenceView {
            backend: "disabled".to_string(),
            spaces: 0,
            agents: 0,
            per_space: Vec::new(),
        },
    };

    match serde_json::to_string(&view) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-store")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from(
                "Failed to serialize bootstrap-coherence view",
            )))
            .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coherence_view_serializes_camel_case() {
        let v = BootstrapCoherenceView {
            backend: "mongo".into(),
            spaces: 1,
            agents: 2,
            per_space: vec![SpaceSkew {
                space: "s".into(),
                agents: 2,
            }],
        };
        let j = serde_json::to_string(&v).unwrap();
        assert!(j.contains("\"perSpace\""), "{j}");
        assert!(j.contains("\"backend\":\"mongo\""), "{j}");
    }
}
