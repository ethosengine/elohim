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

/// What the pool fallback fetch decided, and why — so callers can log at the
/// right level (Fix A: the degraded-primary state was invisible at DEBUG for
/// days; observability is part of the fix).
///
/// See `.claude/deliver/journal-resilient-dual-doorway.md` iter-0 root cause #2.
#[derive(Debug)]
pub enum FallbackOutcome {
    /// The primary (first) URL returned a non-empty projection set. Normal path.
    PrimaryNonEmpty {
        url: String,
        projections: Vec<EprProjectionView>,
    },
    /// The primary was empty or unreachable, but a pool peer supplied rows.
    /// Caller MUST log this at WARN naming both URLs — this is the degraded
    /// state that hid for days.
    PeerServed {
        primary_url: String,
        /// `None` when the primary errored; `Some(0)` when it returned empty.
        primary_empty: bool,
        serving_url: String,
        projections: Vec<EprProjectionView>,
    },
    /// Primary AND every pool peer returned an EMPTY (but successful) fetch.
    /// This is a genuine empty state — replace the router, log at INFO.
    AllEmpty { urls_tried: Vec<String> },
    /// Primary AND every pool peer ERRORED (none returned a usable response).
    /// Caller MUST preserve the last-good table (never clear on transient
    /// unavailability).
    AllUnreachable {
        urls_tried: Vec<String>,
        last_error: String,
    },
}

/// Consult an ordered pool of storage base URLs for active project-epr
/// projections, falling back through the pool when the primary is empty or
/// unreachable.
///
/// Selection (Fix A):
/// 1. Try `base_urls[0]` (the primary). If it returns a set with at least one
///    INSTALLABLE row, use it.
/// 2. Otherwise (error OR empty OR wholly-poisoned), try each remaining URL in
///    order until one returns a set with an installable row. Use the first such
///    result.
/// 3. If primary and all peers return empty/poisoned (successfully), that is a
///    genuine empty state (`AllEmpty`).
/// 4. If every URL errored, that is total unreachability (`AllUnreachable`) —
///    the caller preserves the last-good table.
///
/// Fix 2 — truthful classification: "non-empty" is measured AFTER validation,
/// not on the raw row count. A primary that returns N rows of which 0 are
/// installable (every url_path reserved/invalid) is the Welcome-at-`/` incident
/// class: `replace_all` would install 0 rows under an INFO "success" log. So a
/// wholly-poisoned batch is classified exactly like an empty fetch — it falls
/// through to pool peers, and `primary_empty` records it as the empty (not
/// errored) primary. `batch_has_installable_row` reuses the same free predicate
/// `replace_all` uses, so the validity definition lives in exactly one place.
///
/// The outcome variant carries enough to log at the right level and to decide
/// whether to call `replace_all`.
pub async fn fetch_projections_with_fallback(
    base_urls: &[String],
    doorway_id: &str,
    http: &reqwest::Client,
) -> FallbackOutcome {
    let mut urls_tried: Vec<String> = Vec::new();
    let mut primary_url: Option<String> = None;
    let mut primary_empty = false;
    let mut any_success = false;
    let mut last_error: Option<String> = None;

    for (idx, base_url) in base_urls.iter().enumerate() {
        urls_tried.push(base_url.clone());
        match fetch_projections_from_storage(base_url, doorway_id, http).await {
            // "Usable" = fetched AND at least one row would survive validation.
            // A wholly-poisoned batch (rows present, none installable) is treated
            // exactly like an empty fetch (the `Ok(_)` arm below).
            Ok(projections) if batch_has_installable_row(&projections) => {
                if idx == 0 {
                    return FallbackOutcome::PrimaryNonEmpty {
                        url: base_url.clone(),
                        projections,
                    };
                }
                // A pool peer supplied rows the primary couldn't. Degraded.
                let primary = primary_url.unwrap_or_else(|| base_urls[0].clone());
                return FallbackOutcome::PeerServed {
                    primary_url: primary,
                    primary_empty,
                    serving_url: base_url.clone(),
                    projections,
                };
            }
            // Fetched successfully but no installable row (empty OR wholly
            // poisoned) — a genuine empty/degraded primary, not a usable set.
            Ok(_no_installable_row) => {
                any_success = true;
                if idx == 0 {
                    primary_url = Some(base_url.clone());
                    primary_empty = true;
                }
            }
            Err(e) => {
                last_error = Some(e.to_string());
                if idx == 0 {
                    // Primary errored; record it so PeerServed can name it.
                    primary_url = Some(base_url.clone());
                    primary_empty = false;
                }
            }
        }
    }

    if any_success {
        FallbackOutcome::AllEmpty { urls_tried }
    } else {
        FallbackOutcome::AllUnreachable {
            urls_tried,
            last_error: last_error.unwrap_or_else(|| "no storage URLs configured".to_string()),
        }
    }
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

/// The post-validation truth of a `replace_all`: how many rows were actually
/// installed into the table, and how many were skipped as poisoned.
///
/// Callers MUST log `installed` (not the pre-validation row count) — a fetch
/// that returned N rows but installed 0 is the Welcome-at-`/` incident class,
/// and pre-validation counts hide it (the INFO log claimed success with a
/// non-zero count while the router was empty). `rejected` rides alongside so
/// operators see degradation without grepping WARN lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaceOutcome {
    /// Rows that passed validation and are now reachable in the table.
    pub installed: usize,
    /// Rows skipped as poisoned (reserved/structurally-invalid url_path).
    pub rejected: usize,
}

/// Why a projection row was rejected at table-load. `None` = the row is a
/// syntactically valid, non-reserved mount and may be inserted.
///
/// Per-row degradation (N4): one poisoned row must never empty the whole
/// router. This mirrors the resolver-side per-row degradation precedent
/// (commit f38be2635) — skip-and-WARN the bad row, install the valid remainder.
fn projection_reject_reason(view: &EprProjectionView) -> Option<&'static str> {
    let path = &view.url_path;
    // Structural validity: a mount must be an absolute path with no internal
    // whitespace or control characters (a junk url_path would otherwise become
    // an unreachable HashMap key and silently bloat the table).
    if path.is_empty() {
        return Some("url_path is empty");
    }
    if !path.starts_with('/') {
        return Some("url_path is not absolute (must start with '/')");
    }
    if path.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Some("url_path contains whitespace or control characters");
    }
    // §12.1: reserved service prefixes can never be projection mounts.
    if crate::server::http::is_reserved_url_path(path) {
        return Some("url_path collides with a reserved service prefix (§12.1)");
    }
    None
}

/// True iff at least one row in the batch would survive `replace_all`'s
/// per-row validation (i.e. `projection_reject_reason(...) == None`).
///
/// This is the classification seam (Fix 2): the pool fallback consults
/// post-validation reality, so a wholly-poisoned primary batch (N rows, 0
/// installable) is NOT treated as `PrimaryNonEmpty`/`PeerServed` — it falls
/// through to pool peers or `AllEmpty`, never installing 0 rows under an INFO
/// "success" log. Reuses the same free predicate `replace_all` uses, so there
/// is exactly ONE validation definition.
fn batch_has_installable_row(projections: &[EprProjectionView]) -> bool {
    projections
        .iter()
        .any(|v| projection_reject_reason(v).is_none())
}

impl EprRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the entire routing table atomically. Returns the post-validation
    /// truth ([`ReplaceOutcome`]) — `installed` is the count actually reachable,
    /// `rejected` the count skipped as poisoned. Callers MUST log `installed`,
    /// never the input length (a batch of N that installs 0 is the
    /// Welcome-at-`/` incident class, and the input length hides it).
    ///
    /// Per-row degraded insert (N4): each row is validated independently in a
    /// single pass. A syntactically invalid or reserved-path row is skipped with
    /// a WARN naming the row id and reason; the valid remainder is still
    /// installed. One poisoned row never empties the whole router (the live
    /// incident this guards: Welcome at `/`, 404 on `/lamad`).
    pub fn replace_all(&self, projections: Vec<EprProjectionView>) -> ReplaceOutcome {
        // Single pass: classify each row once, carrying its rejection reason so
        // the WARN prints the reason exactly once and the predicate is never
        // re-evaluated (the `.unwrap_or(...)` double-eval fallback is gone).
        let mut legal: Vec<EprProjectionView> = Vec::with_capacity(projections.len());
        let mut rejected = 0usize;
        for v in projections {
            match projection_reject_reason(&v) {
                Some(reason) => {
                    rejected += 1;
                    tracing::warn!(epr_id = %v.epr_id, url_path = %v.url_path, reason = %reason,
                        "skipping projection row (per-row degraded insert)");
                }
                None => legal.push(v),
            }
        }
        let mut installed = 0usize;
        {
            let mut table = self.table.write().expect("router lock poisoned");
            table.clear();
            for p in legal {
                // Duplicate url_path keys are last-write-wins in a HashMap, which
                // is silent data loss. Mirror the claim-index conflict WARN below
                // so a colliding mount is visible (the duplicate row still wins,
                // last-write — semantics documented in the test suite).
                if let Some(existing) = table.get(&p.url_path) {
                    tracing::warn!(url_path = %p.url_path,
                        dropped_epr_id = %existing.epr_id, kept_epr_id = %p.epr_id,
                        "duplicate projection url_path at router build — last-write-wins (earlier mount dropped)");
                } else {
                    installed += 1;
                }
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

        ReplaceOutcome {
            installed,
            rejected,
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

    /// Snapshot of the live `(url_path, epr_id)` head fingerprints — the input
    /// to the cross-edge coherence digest (`routes::coherence::router_fingerprint`).
    /// Pure read; sibling of `mount_url_paths`.
    pub fn head_fingerprints(&self) -> Vec<(String, String)> {
        let table = self.table.read().expect("router lock poisoned");
        table
            .values()
            .map(|p| (p.url_path.clone(), p.epr_id.clone()))
            .collect()
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

    #[test]
    fn replace_all_degrades_per_row_keeping_valid_remainder() {
        // N4: a mixed batch — one valid row, one reserved-path row, one
        // structurally invalid row (non-absolute url_path) — must install the
        // valid row and skip-and-WARN the rest. The router must NOT end empty:
        // the live incident this guards is one poisoned row collapsing the
        // whole router (Welcome at /, 404 on /lamad).
        let router = EprRouter::new();
        let outcome = router.replace_all(vec![
            make_projection("lamad", "/lamad"),       // valid → must land
            make_projection("bad-epr-mount", "/epr"), // reserved → skipped
            make_projection("not-absolute", "lamad"), // structurally invalid → skipped
            make_projection("empty-path", ""),        // structurally invalid → skipped
        ]);

        // The returned outcome must carry the post-validation truth: 1 installed,
        // 3 rejected — the count callers log, not the input length of 4.
        assert_eq!(
            outcome,
            ReplaceOutcome {
                installed: 1,
                rejected: 3
            },
            "replace_all must report installed=1 rejected=3, not the fetched count of 4"
        );

        // Exactly the one valid row landed; the router is non-empty.
        assert_eq!(
            router.len(),
            1,
            "only the valid /lamad row may be installed; the batch must not be dropped wholesale"
        );
        assert!(
            !router.is_empty(),
            "a poisoned row must not empty the router"
        );
        assert_eq!(
            router.dispatch("/lamad").unwrap().epr_id,
            "lamad",
            "the valid /lamad mount must remain reachable"
        );
        // The reserved + structurally-invalid rows must not be reachable.
        assert!(router.dispatch("/epr").is_none());
        assert!(
            router.dispatch("/lamadx").is_none(),
            "the non-absolute 'lamad' junk key must not be installed as a reachable mount"
        );
    }

    #[test]
    fn replace_all_reports_installed_and_rejected_counts() {
        // Fix 1: a mixed batch returns the post-validation truth — installed
        // counts the rows actually reachable, rejected the rows skipped.
        let router = EprRouter::new();
        let outcome = router.replace_all(vec![
            make_projection("a", "/a"),               // valid
            make_projection("b", "/b"),               // valid
            make_projection("bad-epr-mount", "/epr"), // reserved → rejected
            make_projection("junk", "no-slash"),      // invalid → rejected
        ]);
        assert_eq!(
            outcome,
            ReplaceOutcome {
                installed: 2,
                rejected: 2
            }
        );
        assert_eq!(router.len(), 2);
    }

    #[test]
    fn replace_all_wholly_poisoned_installs_nothing() {
        // Fix 1: a batch where EVERY row is poisoned must install 0 — the router
        // ends empty, and the outcome says so (installed=0). The caller logs
        // installed=0, never the fetched count, so the Welcome-at-`/` incident
        // (0 rows installed under a non-zero "success" count) cannot recur.
        let router = EprRouter::new();
        // Pre-load a good row so we can prove the poisoned replace truly clears.
        let pre = router.replace_all(vec![make_projection("good", "/good")]);
        assert_eq!(pre.installed, 1);

        let outcome = router.replace_all(vec![
            make_projection("bad-epr-mount", "/epr"), // reserved
            make_projection("not-absolute", "lamad"), // structurally invalid
            make_projection("empty-path", ""),        // structurally invalid
        ]);
        assert_eq!(
            outcome,
            ReplaceOutcome {
                installed: 0,
                rejected: 3
            },
            "a wholly-poisoned batch installs 0 rows"
        );
        assert!(
            router.is_empty(),
            "replace_all clears the table even when nothing installs (atomic replace)"
        );
    }

    #[test]
    fn replace_all_duplicate_url_path_is_last_write_wins() {
        // Fix 1: duplicate url_path keys are deterministic last-write-wins (the
        // earlier mount is dropped, the later one kept). The WARN is observable;
        // `installed` counts only the FIRST insert at each key (the table holds
        // exactly one row per url_path). Documented semantics: last writer wins.
        let router = EprRouter::new();
        let first = make_projection("first-owner", "/dup");
        let second = make_projection("second-owner", "/dup");
        let outcome = router.replace_all(vec![first, second]);

        // One key in the table; installed counts the single distinct key.
        assert_eq!(router.len(), 1);
        assert_eq!(
            outcome,
            ReplaceOutcome {
                installed: 1,
                rejected: 0
            },
            "duplicate keys collapse to one installed slot (last-write-wins)"
        );
        // Last writer wins: the second projection is the one that dispatches.
        assert_eq!(
            router.dispatch("/dup").unwrap().epr_id,
            "second-owner",
            "the later projection at a duplicate url_path must win (last-write-wins)"
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

    // ── Fix A: storage-pool fallback for boot + refresh ──────────────────────
    // journal: .claude/deliver/journal-resilient-dual-doorway.md iter-0 RC #2.
    mod pool_fallback {
        use super::super::{fetch_projections_with_fallback, FallbackOutcome};
        use super::make_projection;
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        /// Mount the project-epr query on a mock server, returning the given
        /// projection set as JSON (an empty Vec models the "0 rows" primary).
        async fn mock_project_epr(server: &MockServer, projections: &[&str]) {
            let body: Vec<_> = projections
                .iter()
                .map(|epr| make_projection(epr, &format!("/{epr}")))
                .collect();
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/db/rea_commitments"))
                .and(matchers::query_param("action", "project-epr"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&body))
                .mount(server)
                .await;
        }

        /// Mount a project-epr query returning a WHOLLY-POISONED batch: rows are
        /// present (non-zero length) but every url_path is reserved/invalid, so
        /// `replace_all` would install 0 of them. Models the Welcome-at-`/`
        /// incident: a primary that "answers" with junk only.
        async fn mock_project_epr_poisoned(server: &MockServer) {
            let body = vec![
                make_projection("bad-epr-mount", "/epr"), // reserved prefix
                make_projection("not-absolute", "lamad"), // missing leading '/'
            ];
            Mock::given(matchers::method("GET"))
                .and(matchers::path("/db/rea_commitments"))
                .and(matchers::query_param("action", "project-epr"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&body))
                .mount(server)
                .await;
        }

        #[tokio::test]
        async fn primary_non_empty_short_circuits_the_pool() {
            let primary = MockServer::start().await;
            mock_project_epr(&primary, &["landing", "lamad"]).await;
            // A second peer that, if consulted, would also answer — but must NOT
            // be reached because the primary already satisfied the fetch.
            let peer = MockServer::start().await;
            mock_project_epr(&peer, &["landing"]).await;

            let http = reqwest::Client::new();
            let urls = vec![primary.uri(), peer.uri()];
            let outcome = fetch_projections_with_fallback(&urls, "apex-elohim-host", &http).await;

            match outcome {
                FallbackOutcome::PrimaryNonEmpty { url, projections } => {
                    assert_eq!(url, primary.uri());
                    assert_eq!(projections.len(), 2);
                }
                other => panic!("expected PrimaryNonEmpty, got {other:?}"),
            }
            // The peer must not have been hit (primary short-circuit).
            assert!(
                peer.received_requests().await.unwrap().is_empty(),
                "pool peer must not be consulted when primary is non-empty"
            );
        }

        #[tokio::test]
        async fn empty_primary_falls_back_to_a_peer_with_rows() {
            // This is the apex bug: primary (adam) returns 0 rows, peer (matthew)
            // has the 3 rows.
            let primary = MockServer::start().await;
            mock_project_epr(&primary, &[]).await; // 0 rows
            let peer = MockServer::start().await;
            mock_project_epr(&peer, &["landing", "lamad", "portal"]).await;

            let http = reqwest::Client::new();
            let urls = vec![primary.uri(), peer.uri()];
            let outcome = fetch_projections_with_fallback(&urls, "apex-elohim-host", &http).await;

            match outcome {
                FallbackOutcome::PeerServed {
                    primary_url,
                    primary_empty,
                    serving_url,
                    projections,
                } => {
                    assert_eq!(primary_url, primary.uri());
                    assert!(primary_empty, "primary returned an empty (not errored) set");
                    assert_eq!(serving_url, peer.uri());
                    assert_eq!(projections.len(), 3);
                }
                other => panic!("expected PeerServed, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn unreachable_primary_falls_back_to_a_peer_with_rows() {
            // Primary URL points at a dead port → fetch errors; peer answers.
            let peer = MockServer::start().await;
            mock_project_epr(&peer, &["landing"]).await;

            let http = reqwest::Client::new();
            let urls = vec!["http://127.0.0.1:1/never".to_string(), peer.uri()];
            let outcome = fetch_projections_with_fallback(&urls, "apex-elohim-host", &http).await;

            match outcome {
                FallbackOutcome::PeerServed {
                    primary_url,
                    primary_empty,
                    serving_url,
                    projections,
                } => {
                    assert_eq!(primary_url, "http://127.0.0.1:1/never");
                    assert!(!primary_empty, "primary errored, it was not empty");
                    assert_eq!(serving_url, peer.uri());
                    assert_eq!(projections.len(), 1);
                }
                other => panic!("expected PeerServed, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn all_empty_is_a_genuine_empty_state() {
            let primary = MockServer::start().await;
            mock_project_epr(&primary, &[]).await;
            let peer = MockServer::start().await;
            mock_project_epr(&peer, &[]).await;

            let http = reqwest::Client::new();
            let urls = vec![primary.uri(), peer.uri()];
            let outcome = fetch_projections_with_fallback(&urls, "apex-elohim-host", &http).await;

            match outcome {
                FallbackOutcome::AllEmpty { urls_tried } => {
                    assert_eq!(urls_tried.len(), 2);
                }
                other => panic!("expected AllEmpty, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn all_unreachable_preserves_last_good() {
            let http = reqwest::Client::new();
            let urls = vec![
                "http://127.0.0.1:1/never".to_string(),
                "http://127.0.0.1:2/never".to_string(),
            ];
            let outcome = fetch_projections_with_fallback(&urls, "apex-elohim-host", &http).await;

            match outcome {
                FallbackOutcome::AllUnreachable {
                    urls_tried,
                    last_error,
                } => {
                    assert_eq!(urls_tried.len(), 2);
                    assert!(!last_error.is_empty());
                }
                other => panic!("expected AllUnreachable, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn single_primary_empty_is_genuine_empty_not_peer_served() {
            // Pool of one: an empty primary with no peers is a genuine empty
            // state (not a degraded fallback).
            let primary = MockServer::start().await;
            mock_project_epr(&primary, &[]).await;

            let http = reqwest::Client::new();
            let urls = vec![primary.uri()];
            let outcome = fetch_projections_with_fallback(&urls, "apex-elohim-host", &http).await;

            match outcome {
                FallbackOutcome::AllEmpty { urls_tried } => {
                    assert_eq!(urls_tried, vec![primary.uri()]);
                }
                other => panic!("expected AllEmpty for single empty primary, got {other:?}"),
            }
        }

        #[tokio::test]
        async fn empty_url_pool_is_unreachable_not_panic() {
            let http = reqwest::Client::new();
            let urls: Vec<String> = vec![];
            let outcome = fetch_projections_with_fallback(&urls, "apex-elohim-host", &http).await;
            assert!(matches!(outcome, FallbackOutcome::AllUnreachable { .. }));
        }

        #[tokio::test]
        async fn wholly_poisoned_primary_is_not_classified_primary_non_empty() {
            // Fix 2 — the exact Welcome-at-`/` class: a primary that returns rows
            // but ZERO installable rows must NOT be classified PrimaryNonEmpty
            // (which would install 0 under an INFO "success" log). With a peer
            // that has real rows, it must fall through to PeerServed naming the
            // poisoned primary as `empty`.
            let primary = MockServer::start().await;
            mock_project_epr_poisoned(&primary).await; // rows present, none installable
            let peer = MockServer::start().await;
            mock_project_epr(&peer, &["landing", "lamad"]).await;

            let http = reqwest::Client::new();
            let urls = vec![primary.uri(), peer.uri()];
            let outcome = fetch_projections_with_fallback(&urls, "apex-elohim-host", &http).await;

            match outcome {
                FallbackOutcome::PeerServed {
                    primary_url,
                    primary_empty,
                    serving_url,
                    projections,
                } => {
                    assert_eq!(primary_url, primary.uri());
                    assert!(
                        primary_empty,
                        "a wholly-poisoned primary is classified as empty, not errored"
                    );
                    assert_eq!(serving_url, peer.uri());
                    assert_eq!(projections.len(), 2);
                }
                other => panic!(
                    "wholly-poisoned primary must NOT win classification; expected PeerServed, got {other:?}"
                ),
            }
        }

        #[tokio::test]
        async fn wholly_poisoned_pool_is_all_empty_not_peer_served() {
            // Fix 2: when EVERY pool member answers with only poisoned rows, the
            // pool is a genuine empty state (AllEmpty) — never a usable set. The
            // caller's AllEmpty arm replaces with an honest empty router rather
            // than installing 0 rows under a degraded-but-served WARN.
            let primary = MockServer::start().await;
            mock_project_epr_poisoned(&primary).await;
            let peer = MockServer::start().await;
            mock_project_epr_poisoned(&peer).await;

            let http = reqwest::Client::new();
            let urls = vec![primary.uri(), peer.uri()];
            let outcome = fetch_projections_with_fallback(&urls, "apex-elohim-host", &http).await;

            match outcome {
                FallbackOutcome::AllEmpty { urls_tried } => {
                    assert_eq!(urls_tried.len(), 2);
                }
                other => panic!(
                    "a wholly-poisoned pool is a genuine empty state; expected AllEmpty, got {other:?}"
                ),
            }
        }

        #[tokio::test]
        async fn single_poisoned_primary_is_all_empty_not_primary_non_empty() {
            // Fix 2, pool-of-one: a lone primary that answers with only poisoned
            // rows is AllEmpty (not PrimaryNonEmpty) — the caller installs an
            // honest empty router instead of claiming success with 0 rows.
            let primary = MockServer::start().await;
            mock_project_epr_poisoned(&primary).await;

            let http = reqwest::Client::new();
            let urls = vec![primary.uri()];
            let outcome = fetch_projections_with_fallback(&urls, "apex-elohim-host", &http).await;

            match outcome {
                FallbackOutcome::AllEmpty { urls_tried } => {
                    assert_eq!(urls_tried, vec![primary.uri()]);
                }
                other => panic!(
                    "a lone poisoned primary is a genuine empty state; expected AllEmpty, got {other:?}"
                ),
            }
        }
    }
}
