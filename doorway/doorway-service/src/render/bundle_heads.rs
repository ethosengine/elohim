//! Storage-authoritative bundle heads, reconciled on a tick.
//!
//! ## The defect this closes (measured 2026-09-08)
//!
//! Everything downstream of "which head does this doorway declare for this app"
//! was already correct — the shell is fetched by that head, an unmarked doc is
//! never `AtHead`, an unknown head buys one upgrade per interval. But the
//! DECLARATION itself never moved. The doorway's `projected_entries` store is
//! refreshed only by the boot-time bulk pull and by conductor `post_commit`
//! signals, and those are **cell-local**: only a doorway subscribed to the
//! AUTHORING conductor can ever receive one. The storage `content.updated`
//! bridge deliberately clears only the app-file slug index — and
//! `resolve_blob_hash`'s fallback then re-resolves out of the same stale
//! projection. So on 2026-09-08 both fleet doorways served `/` at browser head
//! `sha256-6899…` for 15 hours after storage had moved to `sha256-e0e2f7…`,
//! serving a page that named a `main-*.js` the current bundle no longer holds.
//!
//! ## The cure
//!
//! One reconciler owns the declaration. For every configured app slug it reads
//! `GET {storage}/db/content/{slug}` — the same primary-scoped storage the shell
//! path reads, under a 2s ceiling — every [`BUNDLE_HEADS_TICK_SECS`] AND on
//! every `content.created|updated` event for that slug, and writes `blobHash` +
//! `serverBlobHash` THROUGH to the doorway's own projected entry and in-memory
//! slug index. When the browser head moves it evicts the warm shell so the next
//! lookup reclassifies against the new head; the reconciled server head is
//! published for the SSR adoption pass to read.
//!
//! **Storage unreachable keeps the last state.** No head is ever fabricated: a
//! failed read logs at debug and retries next tick, exactly as the SSR
//! reconcile's declared-head arm does. The falsifier the spec names — a doorway
//! whose conductor subscription is dead, or that booted while storage was down —
//! converges within one tick after storage answers.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::render::warm_shell::WarmShellStore;

/// Seconds between full reconcile passes. Env-tunable via
/// `BUNDLE_HEADS_TICK_SECS`; 0 disables the tick (the event arm still fires).
pub const BUNDLE_HEADS_TICK_SECS: u64 = 30;

/// Per-read ceiling for `GET /db/content/{slug}`. A healthy peer answers in
/// milliseconds; this only has to cover a slow-but-alive upstream, because a
/// stalled one must not hold the tick.
pub const BUNDLE_HEADS_READ_TIMEOUT_SECS: u64 = 2;

/// An app this doorway declares a head for. `entry_file` is `None` for a slug
/// configured for SSR but not EPR-mounted — there is no shell to evict, but its
/// server head still reconciles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleTarget {
    pub slug: String,
    pub entry_file: Option<String>,
}

/// The heads a storage content row declares. Both fields are optional because
/// absence is an honest state, never an error: a browser-only app declares no
/// `serverBlobHash`, and a peer mid-deploy may declare neither.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeadDoc {
    pub browser: Option<String>,
    pub server: Option<String>,
}

/// Reads a slug's declared heads from the storage upstream. A trait so the
/// reconciler is unit-testable without a live peer — the same mocking seam
/// shape `ShellArchive` and the storage crate's `CommitmentFetcher` use.
#[async_trait::async_trait]
pub trait HeadSource: Send + Sync {
    async fn fetch(&self, slug: &str) -> Result<HeadDoc, String>;
}

/// Write-through target for a reconciled head: the doorway's own projection.
#[async_trait::async_trait]
pub trait HeadProjection: Send + Sync {
    /// Persist `browser`/`server` as this doorway's declaration for `slug`, in
    /// BOTH the in-memory slug index and the durable projected entry.
    async fn write_heads(&self, slug: &str, browser: Option<&str>, server: Option<&str>);
}

/// The live head source: the primary-scoped storage peer.
pub struct HttpHeadSource {
    storage_base: String,
    client: reqwest::Client,
}

impl HttpHeadSource {
    pub fn new(storage_base_url: String) -> Self {
        Self {
            storage_base: storage_base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(BUNDLE_HEADS_READ_TIMEOUT_SECS))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait::async_trait]
impl HeadSource for HttpHeadSource {
    async fn fetch(&self, slug: &str) -> Result<HeadDoc, String> {
        let url = format!("{}/db/content/{}", self.storage_base, slug);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("content GET failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("content GET: HTTP {} for {url}", resp.status()));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| format!("content GET read body: {e}"))?;
        Ok(parse_head_doc(&body))
    }
}

/// Parse a `/db/content/{slug}` body into its declared heads.
///
/// Reuses the ONE parse seam the SSR bundle source already owns
/// (`crate::ssr::parse_blob_hash` / `parse_server_blob_hash`), so the doorway
/// cannot grow a second opinion about which JSON field is which head. A missing
/// or empty field is `None` — honest absence, never an error and never a
/// fabricated head.
pub fn parse_head_doc(body: &str) -> HeadDoc {
    let non_empty = |h: String| if h.is_empty() { None } else { Some(h) };
    HeadDoc {
        browser: crate::ssr::parse_blob_hash(body).ok().and_then(non_empty),
        server: crate::ssr::parse_server_blob_hash(body)
            .ok()
            .and_then(non_empty),
    }
}

/// One slug's last observed declaration, with when it was observed.
#[derive(Debug, Clone)]
pub struct ObservedHeads {
    pub browser: Option<String>,
    pub server: Option<String>,
    pub observed_at: Instant,
}

/// The reconciled heads, shared between the reconciler (writer) and the SSR
/// adoption pass (reader) so the server head is read ONCE per tick rather than
/// re-fetched by every consumer.
#[derive(Default)]
pub struct BundleHeadStore {
    inner: RwLock<HashMap<String, ObservedHeads>>,
}

impl BundleHeadStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, slug: &str) -> Option<ObservedHeads> {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(slug)
            .cloned()
    }

    /// The reconciled SERVER head, but only when it was observed within
    /// `max_age`.
    ///
    /// The freshness clause is load-bearing, not decoration: the SSR adoption
    /// pass reads a declared head TWICE around a materialize (before, to know
    /// what to build; after, to attest that it did not move). Serving both reads
    /// from a value that only changes on a tick would weaken that TOCTOU guard
    /// into a tautology, so the post-materialize read asks for `max_age` zero
    /// and gets a live fetch.
    pub fn server_head_fresh(&self, slug: &str, max_age: Duration) -> Option<String> {
        let observed = self.get(slug)?;
        if observed.observed_at.elapsed() > max_age {
            return None;
        }
        observed.server
    }

    /// Record a fresh observation, returning how the heads moved.
    pub fn record(&self, slug: &str, doc: &HeadDoc) -> HeadMove {
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let previous = inner.get(slug).cloned();
        inner.insert(
            slug.to_string(),
            ObservedHeads {
                browser: doc.browser.clone(),
                server: doc.server.clone(),
                observed_at: Instant::now(),
            },
        );
        HeadMove {
            slug: slug.to_string(),
            browser_from: previous.as_ref().and_then(|p| p.browser.clone()),
            browser_to: doc.browser.clone(),
            server_from: previous.as_ref().and_then(|p| p.server.clone()),
            server_to: doc.server.clone(),
        }
    }
}

/// How one slug's declaration changed across a reconcile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadMove {
    pub slug: String,
    pub browser_from: Option<String>,
    pub browser_to: Option<String>,
    pub server_from: Option<String>,
    pub server_to: Option<String>,
}

impl HeadMove {
    pub fn browser_moved(&self) -> bool {
        self.browser_from != self.browser_to
    }

    pub fn server_moved(&self) -> bool {
        self.server_from != self.server_to
    }

    pub fn moved(&self) -> bool {
        self.browser_moved() || self.server_moved()
    }
}

/// Render a head for a log line: 12 significant characters, or `-` for absent.
fn head12(head: Option<&str>) -> String {
    match head {
        None => "-".to_string(),
        Some(h) => {
            let body = h.strip_prefix("sha256-").unwrap_or(h);
            body.get(..12).unwrap_or(body).to_string()
        }
    }
}

/// Produces the apps this doorway declares heads for. A closure rather than a
/// concrete type so production can compose the EPR router's mounts with the SSR
/// registry's configured slugs while tests hand over a fixed list.
pub type TargetSource = Arc<dyn Fn() -> Vec<BundleTarget> + Send + Sync>;

/// The reconciler. One per doorway.
pub struct BundleHeadsReconciler {
    source: Arc<dyn HeadSource>,
    projection: Option<Arc<dyn HeadProjection>>,
    warm_shell: Arc<WarmShellStore>,
    heads: Arc<BundleHeadStore>,
    targets: TargetSource,
}

impl BundleHeadsReconciler {
    /// `heads` is handed IN rather than created here: the SSR renderer registry
    /// owns the table (it is a reader of the server head), so there is exactly
    /// one table and the adoption pass cannot read a different one than the
    /// reconciler writes.
    pub fn new(
        source: Arc<dyn HeadSource>,
        projection: Option<Arc<dyn HeadProjection>>,
        warm_shell: Arc<WarmShellStore>,
        heads: Arc<BundleHeadStore>,
        targets: TargetSource,
    ) -> Self {
        Self {
            source,
            projection,
            warm_shell,
            heads,
            targets,
        }
    }

    /// The shared head table the SSR adoption pass reads.
    pub fn heads(&self) -> Arc<BundleHeadStore> {
        Arc::clone(&self.heads)
    }

    /// The apps this pass will walk.
    pub fn targets(&self) -> Vec<BundleTarget> {
        (self.targets)()
    }

    /// Reconcile ONE slug. Returns the move when a head actually changed.
    ///
    /// `entry_file` is only used to name the shell being evicted; eviction is
    /// slug-scoped, so `None` still evicts correctly.
    pub async fn reconcile_slug(&self, slug: &str, entry_file: Option<&str>) -> Option<HeadMove> {
        let doc = match self.source.fetch(slug).await {
            Ok(doc) => doc,
            Err(e) => {
                // Keep the last state. A head is NEVER fabricated, and an
                // unreachable peer must not un-declare a head we already hold.
                tracing::debug!(
                    target: "doorway::ssr",
                    slug = %slug,
                    error = %e,
                    "bundle heads: declared-head read failed — keeping last state, retrying next tick"
                );
                return None;
            }
        };

        let mv = self.heads.record(slug, &doc);
        if !mv.moved() {
            return None;
        }

        tracing::info!(
            target: "doorway::ssr",
            slug = %slug,
            entry_file = entry_file.unwrap_or("-"),
            "bundle heads: {} browser {}->{} server {}->{}",
            slug,
            head12(mv.browser_from.as_deref()),
            head12(mv.browser_to.as_deref()),
            head12(mv.server_from.as_deref()),
            head12(mv.server_to.as_deref()),
        );

        // Write-through FIRST, so a request racing the eviction below
        // re-resolves against the new declaration rather than the old one.
        if let Some(projection) = self.projection.as_ref() {
            projection
                .write_heads(slug, mv.browser_to.as_deref(), mv.server_to.as_deref())
                .await;
        }

        if mv.browser_moved() {
            // The hot map is consulted BEFORE the archive, so without this the
            // old shell keeps serving under the new declaration.
            //
            // The per-file archive documents are deliberately NOT purged: they
            // are keyed `{slug}:{file}:{blob_hash}`, so old-head entries are
            // already unreachable by key and TTL out on their own — while
            // deleting them would destroy the last-reconciled bytes the Behind
            // path serves when the new head is not yet deliverable.
            self.warm_shell.evict(slug);
        }

        Some(mv)
    }

    /// Reconcile every configured slug once.
    pub async fn tick(&self) -> Vec<HeadMove> {
        let mut moves = Vec::new();
        for target in self.targets() {
            if let Some(mv) = self
                .reconcile_slug(&target.slug, target.entry_file.as_deref())
                .await
            {
                moves.push(mv);
            }
        }
        moves
    }

    /// Reconcile the slug named by a storage `content.{created,updated}` event,
    /// but only when it is one this doorway declares a head for. An event for
    /// any other content row is not this reconciler's business.
    pub async fn on_content_event(&self, id: &str) -> Option<HeadMove> {
        let target = self.targets().into_iter().find(|t| t.slug == id)?;
        self.reconcile_slug(&target.slug, target.entry_file.as_deref())
            .await
    }
}

/// The tick period, from `BUNDLE_HEADS_TICK_SECS` (default
/// [`BUNDLE_HEADS_TICK_SECS`]). `0` disables the tick.
pub fn configured_tick_period() -> Option<Duration> {
    let secs = std::env::var("BUNDLE_HEADS_TICK_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(BUNDLE_HEADS_TICK_SECS);
    (secs > 0).then(|| Duration::from_secs(secs))
}

/// Spawn the reconcile tick.
pub fn spawn_bundle_heads_task(
    reconciler: Arc<BundleHeadsReconciler>,
    period: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!(
            target: "doorway::ssr",
            tick_secs = period.as_secs(),
            "bundle-heads reconcile tick started"
        );
        loop {
            tokio::time::sleep(period).await;
            let moves = reconciler.tick().await;
            if !moves.is_empty() {
                tracing::info!(
                    target: "doorway::ssr",
                    moved = moves.len(),
                    "bundle heads: {} declaration(s) advanced this tick",
                    moves.len()
                );
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// In-memory stand-in for the storage peer.
    #[derive(Default)]
    struct FakeSource {
        doc: Mutex<Option<HeadDoc>>,
        fail: Mutex<Option<String>>,
        reads: AtomicUsize,
    }

    impl FakeSource {
        fn declaring(browser: Option<&str>, server: Option<&str>) -> Arc<Self> {
            let s = Arc::new(FakeSource::default());
            s.declare(browser, server);
            s
        }

        fn declare(&self, browser: Option<&str>, server: Option<&str>) {
            *self.doc.lock().unwrap() = Some(HeadDoc {
                browser: browser.map(str::to_string),
                server: server.map(str::to_string),
            });
            *self.fail.lock().unwrap() = None;
        }

        fn go_dark(&self, why: &str) {
            *self.fail.lock().unwrap() = Some(why.to_string());
        }
    }

    #[async_trait::async_trait]
    impl HeadSource for FakeSource {
        async fn fetch(&self, _slug: &str) -> Result<HeadDoc, String> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            if let Some(e) = self.fail.lock().unwrap().clone() {
                return Err(e);
            }
            self.doc
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| "no doc".into())
        }
    }

    /// One recorded write-through: which slug, and the two heads declared.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct RecordedWrite {
        slug: String,
        browser: Option<String>,
        server: Option<String>,
    }

    #[derive(Default)]
    struct FakeProjection {
        writes: Mutex<Vec<RecordedWrite>>,
    }

    #[async_trait::async_trait]
    impl HeadProjection for FakeProjection {
        async fn write_heads(&self, slug: &str, browser: Option<&str>, server: Option<&str>) {
            self.writes.lock().unwrap().push(RecordedWrite {
                slug: slug.to_string(),
                browser: browser.map(str::to_string),
                server: server.map(str::to_string),
            });
        }
    }

    fn one_target() -> TargetSource {
        Arc::new(|| {
            vec![BundleTarget {
                slug: "landing".into(),
                entry_file: Some("index.html".into()),
            }]
        })
    }

    fn reconciler(
        source: Arc<FakeSource>,
        projection: Arc<FakeProjection>,
        warm: Arc<WarmShellStore>,
    ) -> BundleHeadsReconciler {
        BundleHeadsReconciler::new(
            source,
            Some(projection),
            warm,
            Arc::new(BundleHeadStore::new()),
            one_target(),
        )
    }

    /// THE 2026-09-08 defect, from the reconciler's side: storage has moved and
    /// the doorway's declaration has not. One tick must move it.
    #[tokio::test]
    async fn a_moved_storage_head_advances_the_doorways_declaration_in_one_tick() {
        let source = FakeSource::declaring(Some("sha256-6899aaaa"), Some("sha256-serverA"));
        let projection = Arc::new(FakeProjection::default());
        let warm = Arc::new(WarmShellStore::inert());
        let r = reconciler(source.clone(), projection.clone(), warm);

        // First pass: the doorway learns the head it had never observed.
        let first = r.tick().await;
        assert_eq!(first.len(), 1);
        assert_eq!(
            r.heads().get("landing").unwrap().browser.as_deref(),
            Some("sha256-6899aaaa")
        );

        // Storage moves — exactly the 2026-09-08 shape.
        source.declare(Some("sha256-e0e2f7bb"), Some("sha256-serverA"));
        let moves = r.tick().await;
        assert_eq!(moves.len(), 1, "the tick must observe the move");
        let mv = &moves[0];
        assert!(mv.browser_moved());
        assert!(!mv.server_moved());
        assert_eq!(mv.browser_from.as_deref(), Some("sha256-6899aaaa"));
        assert_eq!(mv.browser_to.as_deref(), Some("sha256-e0e2f7bb"));

        // …and it is written THROUGH to the doorway's own projection, which is
        // what the shell path re-resolves against.
        let writes = projection.writes.lock().unwrap().clone();
        assert_eq!(writes.len(), 2, "first observation + the move");
        assert_eq!(writes[1].browser.as_deref(), Some("sha256-e0e2f7bb"));
        assert_eq!(writes[1].slug, "landing");
    }

    #[tokio::test]
    async fn a_steady_head_writes_nothing_and_logs_nothing() {
        let source = FakeSource::declaring(Some("sha256-aaaa1111"), None);
        let projection = Arc::new(FakeProjection::default());
        let r = reconciler(
            source.clone(),
            projection.clone(),
            Arc::new(WarmShellStore::inert()),
        );

        r.tick().await;
        let after_first = projection.writes.lock().unwrap().len();
        for _ in 0..3 {
            assert!(
                r.tick().await.is_empty(),
                "an unmoved head is not a move — the tick must be silent"
            );
        }
        assert_eq!(
            projection.writes.lock().unwrap().len(),
            after_first,
            "a steady head must not re-write the projection every 30s"
        );
    }

    /// Storage unreachable ⇒ keep the last state. Never fabricate, never
    /// un-declare.
    #[tokio::test]
    async fn an_unreachable_storage_keeps_the_last_declared_head() {
        let source = FakeSource::declaring(Some("sha256-known999"), Some("sha256-srv"));
        let projection = Arc::new(FakeProjection::default());
        let r = reconciler(
            source.clone(),
            projection.clone(),
            Arc::new(WarmShellStore::inert()),
        );
        r.tick().await;
        let writes_before = projection.writes.lock().unwrap().len();

        source.go_dark("connection refused");
        let moves = r.tick().await;

        assert!(moves.is_empty(), "an unreachable peer reports no move");
        assert_eq!(
            r.heads().get("landing").unwrap().browser.as_deref(),
            Some("sha256-known999"),
            "the last known head survives the outage — it is never un-declared"
        );
        assert_eq!(
            projection.writes.lock().unwrap().len(),
            writes_before,
            "nothing is written through from a read that never answered"
        );
    }

    #[tokio::test]
    async fn a_content_event_reconciles_only_a_slug_this_doorway_declares() {
        let source = FakeSource::declaring(Some("sha256-h1"), None);
        let r = reconciler(
            source.clone(),
            Arc::new(FakeProjection::default()),
            Arc::new(WarmShellStore::inert()),
        );

        assert!(
            r.on_content_event("some-unrelated-article").await.is_none(),
            "an event for a non-bundled row is not this reconciler's business"
        );
        assert_eq!(
            source.reads.load(Ordering::SeqCst),
            0,
            "and it must not spend an upstream read to find that out"
        );

        assert!(r.on_content_event("landing").await.is_some());
        assert_eq!(source.reads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn the_shared_head_table_publishes_the_server_head_with_a_freshness_bound() {
        let source = FakeSource::declaring(Some("sha256-b"), Some("sha256-serverXYZ"));
        let r = reconciler(
            source,
            Arc::new(FakeProjection::default()),
            Arc::new(WarmShellStore::inert()),
        );
        r.tick().await;

        let heads = r.heads();
        assert_eq!(
            heads
                .server_head_fresh("landing", Duration::from_secs(60))
                .as_deref(),
            Some("sha256-serverXYZ"),
            "the SSR adoption pass reads the reconciled server head"
        );
        assert_eq!(
            heads.server_head_fresh("landing", Duration::ZERO),
            None,
            "a zero max_age forces the caller to fetch live — the TOCTOU guard"
        );
    }

    #[test]
    fn an_absent_server_head_is_absence_not_an_error() {
        let doc = parse_head_doc(r#"{"blobHash":"sha256-abc","id":"landing"}"#);
        assert_eq!(doc.browser.as_deref(), Some("sha256-abc"));
        assert_eq!(
            doc.server, None,
            "adam's row carried serverBlobHash: null on 2026-09-08 — absence, not failure"
        );
    }

    #[test]
    fn an_empty_head_string_is_absence_too() {
        // The live alpha shape the 2026-09-04 rails found: `head:""`.
        let doc = parse_head_doc(r#"{"blobHash":"","serverBlobHash":""}"#);
        assert_eq!(doc.browser, None);
        assert_eq!(doc.server, None);
    }

    #[test]
    fn head_log_rendering_is_bounded_and_names_absence() {
        assert_eq!(head12(Some("sha256-e0e2f7bbccdd1122")), "e0e2f7bbccdd");
        assert_eq!(head12(Some("short")), "short");
        assert_eq!(head12(None), "-");
    }
}
