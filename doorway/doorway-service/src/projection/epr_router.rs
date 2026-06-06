//! EPR Router — consults active project-epr commitments on this doorway
//! and dispatches incoming URLs to the right EPR by longest-prefix match.
//!
//! See `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md`
//! §5.1 (Scenario A — Hard browser navigation to a pillar URL).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use elohim_views::projection::EprProjectionView;

/// Fetch active project-epr commitments from elohim-storage at doorway boot.
///
/// Calls `GET {storage_base_url}/db/rea_commitments?action=project-epr&doorwayId={doorway_id}`.
///
/// Non-fatal: callers MUST wrap this in a `match` / `Result` and continue on error —
/// the EPR router starts empty and SSE events will repopulate it when storage comes up.
pub async fn fetch_projections_from_storage(
    storage_base_url: &str,
    doorway_id: &str,
    http: &reqwest::Client,
) -> Result<Vec<EprProjectionView>, anyhow::Error> {
    let url = format!(
        "{}/db/rea_commitments?action=project-epr&doorwayId={}",
        storage_base_url.trim_end_matches('/'),
        doorway_id
    );
    let response = http
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?
        .error_for_status()?;
    let projections: Vec<EprProjectionView> = response.json().await?;
    Ok(projections)
}

/// In-memory routing table for this doorway's projected EPRs.
///
/// Populated at boot via storage HTTP API (`GET /db/rea_commitments?
/// action=project-epr&doorwayId=...`) and refreshed when SSE events
/// (`projection.registered` / `projection.revoked`) arrive.
///
/// Concurrent access is read-mostly: requests read; only boot + SSE events
/// write. An `RwLock<HashMap>` is appropriate (request reads don't block
/// each other; writes are rare).
/// A granted claim binding compiled at table-load (spec §8.5): contentType →
/// (mount, template). Pre-resolved so dispatch stays a pure lookup (R1).
#[derive(Debug, Clone)]
struct ClaimBinding {
    mount_url_path: String,
    template: String,
}

#[derive(Debug, Default)]
pub struct EprRouter {
    /// urlPath → projection.
    table: RwLock<HashMap<String, EprProjectionView>>,
    /// contentType → claimed mount binding (compiled from grants in replace_all).
    claims: RwLock<HashMap<String, ClaimBinding>>,
    /// Monotonic generation counter, bumped on every `replace_all`. Drives the
    /// sitemap's event-invalidated materialized projection (spec §7.5).
    generation: AtomicU64,
}

impl EprRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the entire routing table atomically.
    pub fn replace_all(&self, projections: Vec<EprProjectionView>) {
        // §12.1: reserved service prefixes can never be projection mounts.
        let (legal, reserved): (Vec<_>, Vec<_>) = projections
            .into_iter()
            .partition(|v| !crate::server::http::is_reserved_url_path(&v.url_path));
        for v in &reserved {
            tracing::warn!(epr_id = %v.epr_id, url_path = %v.url_path,
                "skipping projection: url_path collides with a reserved service prefix (§12.1)");
        }
        {
            let mut table = self.table.write().expect("router lock poisoned");
            table.clear();
            for p in legal {
                table.insert(p.url_path.clone(), p);
            }
        }

        // Compile the granted-claims index (spec §3.3: conflicts were rejected
        // at grant time; the router stays defensive — deterministic first-wins
        // by ascending url_path, with a warning, if a conflict slips through).
        let mut claim_index: HashMap<String, ClaimBinding> = HashMap::new();
        {
            let table = self.table.read().expect("router lock poisoned");
            let mut sorted: Vec<&EprProjectionView> = table.values().collect();
            sorted.sort_by(|a, b| a.url_path.cmp(&b.url_path));
            for p in sorted {
                if let Some(grant) = &p.route_claims {
                    for c in &grant.claims {
                        if let Some(existing) = claim_index.get(&c.content_type) {
                            tracing::warn!(content_type = %c.content_type,
                                kept = %existing.mount_url_path, dropped = %p.url_path,
                                "claim conflict at router build — grant-time validation should have prevented this");
                            continue;
                        }
                        claim_index.insert(
                            c.content_type.clone(),
                            ClaimBinding {
                                mount_url_path: p.url_path.clone(),
                                template: c.template.clone(),
                            },
                        );
                    }
                }
            }
        }
        *self.claims.write().expect("router lock poisoned") = claim_index;

        // Bump the generation last — readers (sitemap) compare against this to
        // decide whether their materialized projection is stale (spec §7.5).
        self.generation.fetch_add(1, Ordering::SeqCst);
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

    /// The current routing-table generation (bumped on every `replace_all`).
    /// The sitemap materializer compares its cached generation against this to
    /// decide whether to re-render (spec §7.5).
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Snapshot of the live mount url_paths (telemetry + sitemap projection).
    pub fn mount_url_paths(&self) -> Vec<String> {
        let table = self.table.read().expect("router lock poisoned");
        table.values().map(|p| p.url_path.clone()).collect()
    }

    /// Mint the pretty-mount Location for a claimed contentType (spec §5.1).
    /// The id is substituted as-received from the request path — already
    /// percent-encoded, never re-encoded.
    pub(crate) fn mint_mount_location(mount_url_path: &str, template: &str, id: &str) -> String {
        let sub = template.replace("{id}", id);
        if mount_url_path == "/" {
            format!("/{sub}")
        } else {
            format!("{mount_url_path}/{sub}")
        }
    }

    /// Segment-wise template match: `{xxx}` binds exactly ONE non-empty
    /// segment. Returns the substituted `to` on match.
    pub(crate) fn match_alias_template(request_path: &str, from: &str, to: &str) -> Option<String> {
        let req: Vec<&str> = request_path.split('/').filter(|s| !s.is_empty()).collect();
        let pat: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
        if req.len() != pat.len() {
            return None;
        }
        let mut bindings: Vec<(&str, &str)> = Vec::new();
        for (r, p) in req.iter().zip(pat.iter()) {
            if p.starts_with('{') && p.ends_with('}') {
                bindings.push((p, r));
            } else if r != p {
                return None;
            }
        }
        let mut out = to.to_string();
        for (placeholder, value) in bindings {
            out = out.replace(placeholder, value);
        }
        Some(out)
    }

    /// Bare alias (redirects_from): prefix-swap the alias for the mount.
    pub(crate) fn match_bare_alias(
        request_path: &str,
        bare_from: &str,
        mount: &str,
    ) -> Option<String> {
        if request_path == bare_from {
            return Some(mount.to_string());
        }
        request_path
            .strip_prefix(&format!("{bare_from}/"))
            .map(|rest| {
                if mount == "/" {
                    format!("/{rest}")
                } else {
                    format!("{mount}/{rest}")
                }
            })
    }

    /// Resolve an alias 302 for a request path (spec §4): template aliases
    /// first (they may live UNDER a live mount, e.g. /lamad/resource/{id}),
    /// then bare redirects_from prefix swaps. None = not an alias.
    pub fn resolve_alias(&self, request_path: &str) -> Option<String> {
        let table = self.table.read().expect("router lock poisoned");
        for p in table.values() {
            for t in &p.redirect_templates {
                if let Some(loc) = Self::match_alias_template(request_path, &t.from, &t.to) {
                    return Some(loc);
                }
            }
        }
        for p in table.values() {
            for bare in &p.redirects_from {
                if let Some(loc) = Self::match_bare_alias(request_path, bare, &p.url_path) {
                    return Some(loc);
                }
            }
        }
        None
    }

    /// The set of contentTypes that currently have a granted claim (sitemap
    /// materializer: one commons-id fetch per claimed type, spec §7.5).
    pub fn claimed_content_types(&self) -> Vec<String> {
        let claims = self.claims.read().expect("router lock poisoned");
        claims.keys().cloned().collect()
    }

    /// The claimed pretty-mount Location for a contentType, if granted.
    pub fn claimed_mount_location(&self, content_type: &str, id: &str) -> Option<String> {
        let claims = self.claims.read().expect("router lock poisoned");
        claims
            .get(content_type)
            .map(|b| Self::mint_mount_location(&b.mount_url_path, &b.template, id))
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
            spa_fallback: true,
            redirects_from: vec![],
            redirect_templates: vec![],
            route_claims: None,
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

    #[test]
    fn replace_all_skips_reserved_url_path() {
        // §12.1: a projection registered at /epr (a reserved service prefix) must
        // be silently dropped; a /lamad projection must survive.
        let router = EprRouter::new();
        router.replace_all(vec![
            make_projection("bad-epr-mount", "/epr"),
            make_projection("lamad", "/lamad"),
        ]);
        // /epr projection must be silently dropped.
        assert_eq!(
            router.len(),
            1,
            "only the legal /lamad projection must be in the table"
        );
        assert!(
            router.dispatch("/epr").is_none(),
            "reserved /epr mount must not be installed in the router"
        );
        // /lamad projection must survive.
        assert_eq!(
            router.dispatch("/lamad").unwrap().epr_id,
            "lamad",
            "legal /lamad mount must remain in the router"
        );
    }

    // ── Slice-3 claims + alias indexes (spec §8.5, §4) ───────────────────────
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct AliasVector {
        note: String,
        #[serde(default)]
        from: Option<String>,
        #[serde(default)]
        to: Option<String>,
        #[serde(default)]
        bare_from: Option<String>,
        #[serde(default)]
        mount_url_path: Option<String>,
        request_path: String,
        expect_location: Option<String>,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DispatchVector {
        note: String,
        mount_url_path: String,
        template: String,
        id: String,
        expect_location: String,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ClaimsFixture {
        reserved_prefixes: Vec<String>,
        alias_vectors: Vec<AliasVector>,
        dispatch_vectors: Vec<DispatchVector>,
    }
    fn claims_fixture() -> ClaimsFixture {
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../elohim/sdk/fixtures/route-claims.vectors.json"
        ));
        serde_json::from_str(raw).expect("route-claims fixture must parse")
    }

    #[test]
    fn reserved_prefixes_fixture_agrees_with_is_service_path() {
        // Two-layer guard for the reserved list itself (spec §4 validation).
        for p in claims_fixture().reserved_prefixes {
            assert!(
                crate::server::http::is_reserved_url_path(&p),
                "fixture reserved prefix {p:?} must be reserved per is_reserved_url_path"
            );
        }
    }

    #[test]
    fn template_alias_vectors_resolve() {
        for v in claims_fixture().alias_vectors {
            let got = if let (Some(from), Some(to)) = (v.from.as_deref(), v.to.as_deref()) {
                EprRouter::match_alias_template(&v.request_path, from, to)
            } else {
                EprRouter::match_bare_alias(
                    &v.request_path,
                    v.bare_from.as_deref().unwrap(),
                    v.mount_url_path.as_deref().unwrap(),
                )
            };
            assert_eq!(got, v.expect_location, "alias vector failed: {}", v.note);
        }
    }

    #[test]
    fn mount_location_vectors_mint() {
        for v in claims_fixture().dispatch_vectors {
            assert_eq!(
                EprRouter::mint_mount_location(&v.mount_url_path, &v.template, &v.id),
                v.expect_location,
                "dispatch vector failed: {}",
                v.note
            );
        }
    }

    #[test]
    fn resolve_alias_consults_projection_tables() {
        let router = EprRouter::new();
        let mut lamad = make_projection("lamad", "/lamad");
        lamad.redirect_templates = vec![elohim_views::projection::RedirectTemplate {
            from: "/lamad/resource/{id}".into(),
            to: "/epr/{id}".into(),
        }];
        lamad.redirects_from = vec!["/learn".into()];
        router.replace_all(vec![lamad]);
        assert_eq!(
            router.resolve_alias("/lamad/resource/fct-module-01-church-dilemma"),
            Some("/epr/fct-module-01-church-dilemma".into())
        );
        assert_eq!(
            router.resolve_alias("/learn/path/x"),
            Some("/lamad/path/x".into())
        );
        assert_eq!(
            router.resolve_alias("/lamad/path/x"),
            None,
            "live mounts are not aliases"
        );
    }

    #[test]
    fn claim_binding_resolves_content_type_to_mount() {
        let router = EprRouter::new();
        let mut lamad = make_projection("lamad", "/lamad");
        lamad.route_claims = Some(elohim_views::projection::RouteClaimGrant {
            schema_version: 1,
            claims_manifest_cid: None,
            claims: vec![elohim_views::projection::RouteClaimTemplate {
                content_type: "path".into(),
                template: "path/{id}".into(),
                fragments: Default::default(),
            }],
        });
        router.replace_all(vec![lamad]);
        assert_eq!(
            router.claimed_mount_location("path", "foundations-christian-technology"),
            Some("/lamad/path/foundations-christian-technology".into())
        );
        assert_eq!(router.claimed_mount_location("concept", "x"), None);
    }

    #[test]
    fn replace_all_bumps_generation() {
        let router = EprRouter::new();
        let g0 = router.generation();
        router.replace_all(vec![make_projection("a", "/a")]);
        assert!(router.generation() > g0);
    }
}
