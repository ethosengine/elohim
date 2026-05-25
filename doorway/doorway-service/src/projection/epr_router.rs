//! EPR Router — consults active project-epr commitments on this doorway
//! and dispatches incoming URLs to the right EPR by longest-prefix match.
//!
//! See `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md`
//! §5.1 (Scenario A — Hard browser navigation to a pillar URL).

use std::collections::HashMap;
use std::sync::RwLock;

use elohim_views::projection::EprProjectionView;

/// In-memory routing table for this doorway's projected EPRs.
///
/// Populated at boot via storage HTTP API (`GET /db/rea_commitments?
/// action=project-epr&doorwayId=...`) and refreshed when SSE events
/// (`projection.registered` / `projection.revoked`) arrive.
///
/// Concurrent access is read-mostly: requests read; only boot + SSE events
/// write. An `RwLock<HashMap>` is appropriate (request reads don't block
/// each other; writes are rare).
#[derive(Debug, Default)]
pub struct EprRouter {
    /// urlPath → projection.
    table: RwLock<HashMap<String, EprProjectionView>>,
}

impl EprRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the entire routing table atomically.
    pub fn replace_all(&self, projections: Vec<EprProjectionView>) {
        let mut table = self.table.write().expect("router lock poisoned");
        table.clear();
        for p in projections {
            table.insert(p.url_path.clone(), p);
        }
    }

    /// Dispatch a request path → the projection whose `url_path` is the
    /// longest prefix of `request_path`. Returns `None` if no projection
    /// matches.
    ///
    /// `"/"` matches every request as the universal root.
    pub fn dispatch(&self, request_path: &str) -> Option<EprProjectionView> {
        let table = self.table.read().expect("router lock poisoned");
        table
            .values()
            .filter(|p| Self::path_matches_prefix(request_path, &p.url_path))
            .max_by_key(|p| p.url_path.len())
            .cloned()
    }

    /// True iff `projection_path` is a path prefix of `request_path`
    /// on a segment boundary (so `/lamad` matches `/lamad`, `/lamad/`,
    /// `/lamad/foo` but NOT `/lamadx`). `"/"` matches everything.
    fn path_matches_prefix(request_path: &str, projection_path: &str) -> bool {
        if projection_path == "/" {
            return true;
        }
        request_path == projection_path
            || request_path.starts_with(&format!("{}/", projection_path))
    }

    /// How many projections are currently in the table (telemetry).
    pub fn len(&self) -> usize {
        self.table.read().expect("router lock poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_views::projection::ProjectionMode;

    fn make_projection(epr_id: &str, url_path: &str) -> EprProjectionView {
        EprProjectionView {
            commitment_id: format!("test-{}", epr_id),
            epr_id: epr_id.into(),
            doorway_id: "doorway:test".into(),
            url_path: url_path.into(),
            mode: ProjectionMode::Cached,
            reach: "commons".into(),
            base_href: if url_path == "/" {
                "/".into()
            } else {
                format!("{}/", url_path)
            },
            entry_file: "index.html".into(),
            redirects_from: vec![],
            preview_epr_ref: None,
            gate_hints: vec![],
            dead_end: false,
            steward_direct_endpoint: None,
            seeded_at: "2026-05-25T00:00:00Z".into(),
            seeded_by: "test".into(),
        }
    }

    #[test]
    fn dispatch_returns_none_for_empty_router() {
        let router = EprRouter::new();
        assert!(router.dispatch("/anything").is_none());
        assert!(router.is_empty());
    }

    #[test]
    fn dispatch_returns_landing_for_root() {
        let router = EprRouter::new();
        router.replace_all(vec![make_projection("landing", "/")]);
        assert_eq!(router.dispatch("/").unwrap().epr_id, "landing");
        assert_eq!(router.dispatch("/anything").unwrap().epr_id, "landing");
        assert_eq!(
            router.dispatch("/deep/path/here").unwrap().epr_id,
            "landing"
        );
    }

    #[test]
    fn dispatch_longest_prefix_wins() {
        let router = EprRouter::new();
        router.replace_all(vec![
            make_projection("landing", "/"),
            make_projection("lamad", "/lamad"),
        ]);
        assert_eq!(router.dispatch("/").unwrap().epr_id, "landing");
        assert_eq!(router.dispatch("/lamad").unwrap().epr_id, "lamad");
        assert_eq!(router.dispatch("/lamad/concept/x").unwrap().epr_id, "lamad");
        assert_eq!(router.dispatch("/other").unwrap().epr_id, "landing");
    }

    #[test]
    fn dispatch_does_not_match_partial_segment() {
        let router = EprRouter::new();
        router.replace_all(vec![make_projection("lamad", "/lamad")]);
        assert!(router.dispatch("/lamadx").is_none());
        assert!(router.dispatch("/lamadextra").is_none());
    }

    #[test]
    fn replace_all_drops_previous_state() {
        let router = EprRouter::new();
        router.replace_all(vec![make_projection("a", "/a")]);
        assert_eq!(router.len(), 1);
        router.replace_all(vec![make_projection("b", "/b")]);
        assert_eq!(router.len(), 1);
        assert!(router.dispatch("/a").is_none());
        assert_eq!(router.dispatch("/b").unwrap().epr_id, "b");
    }
}
