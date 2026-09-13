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

use elohim_views::projection::Channel;

use crate::render::warm_shell::WarmShellStore;

/// Seconds between full reconcile passes. Env-tunable via
/// `BUNDLE_HEADS_TICK_SECS`; 0 disables the tick (the event arm still fires).
pub const BUNDLE_HEADS_TICK_SECS: u64 = 30;

/// Per-read ceiling for `GET /db/content/{slug}`. A healthy peer answers in
/// milliseconds; this only has to cover a slow-but-alive upstream, because a
/// stalled one must not hold the tick.
pub const BUNDLE_HEADS_READ_TIMEOUT_SECS: u64 = 2;

/// An app this doorway declares a head for, ON ONE CHANNEL. `entry_file` is
/// `None` for a slug configured for SSR but not EPR-mounted — there is no shell
/// to evict, but its server head still reconciles.
///
/// The channel comes from the CONTRACT (`EprProjectionView::channel`), which is
/// why one slug can appear twice: a `converged` contract and a `candidate`
/// contract for the same app are two targets, reconciled independently and
/// stored under two keys. Nothing configures a channel per doorway — the
/// contract says which tier its hostnames serve, and the doorway reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleTarget {
    pub slug: String,
    pub entry_file: Option<String>,
    pub channel: Channel,
}

/// The heads a storage content row declares. Every field is optional because
/// absence is an honest state, never an error: a browser-only app declares no
/// `serverBlobHash`, and a peer mid-deploy may declare neither.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeadDoc {
    pub browser: Option<String>,
    pub server: Option<String>,
    /// The STAGING canonical-head declaration this channel resolved to,
    /// addressed by the ActionHash of the DECLARATION (never a blob CID).
    ///
    /// Only ever `Some` on [`Channel::Candidate`]. The converged channel
    /// resolves the earned winner, which is what `browser`/`server` already
    /// name — a converged doc carrying a staging declaration would mean the
    /// two tiers had been confused.
    pub staging_declaration: Option<String>,
}

/// What ONE channel resolved to for ONE slug.
///
/// The second arm is the whole reason this is an enum rather than an
/// `Option<HeadDoc>`: a candidate channel with nothing staged beneath the
/// winner has a NAMED answer, and that answer is not the converged head.
/// Collapsing it into "no heads" would let a caller fall back, which is exactly
/// the failure — silently serving production bytes at a staging name is how a
/// candidate channel stops meaning anything (C4 honest absence).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelHead {
    /// The channel resolved to a declaration.
    Resolved(HeadDoc),
    /// `channel: candidate`, and no staging declaration stands beneath the
    /// earned winner. NEVER the converged head.
    NoCandidateStaged,
}

/// Reads a slug's declared heads from the storage upstream. A trait so the
/// reconciler is unit-testable without a live peer — the same mocking seam
/// shape `ShellArchive` and the storage crate's `CommitmentFetcher` use.
#[async_trait::async_trait]
pub trait HeadSource: Send + Sync {
    /// Resolve `slug` on `channel`. An `Err` is a TRANSPORT failure (the ask
    /// could not be put) and never an answer — `NoCandidateStaged` is the
    /// answer for "asked, nothing staged".
    async fn fetch(&self, slug: &str, channel: Channel) -> Result<ChannelHead, String>;
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

impl HttpHeadSource {
    /// GET `url`, returning the body or a transport-shaped error.
    async fn get_body(&self, url: &str) -> Result<String, String> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("content GET failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("content GET: HTTP {} for {url}", resp.status()));
        }
        resp.text()
            .await
            .map_err(|e| format!("content GET read body: {e}"))
    }
}

#[async_trait::async_trait]
impl HeadSource for HttpHeadSource {
    async fn fetch(&self, slug: &str, channel: Channel) -> Result<ChannelHead, String> {
        match channel {
            // Byte-for-byte the pre-rung-4 read: the earned/declared head's
            // blob hashes off the content row.
            Channel::Converged => {
                let body = self
                    .get_body(&format!("{}/db/content/{}", self.storage_base, slug))
                    .await?;
                Ok(ChannelHead::Resolved(parse_head_doc(&body)))
            }
            // The candidate is not a property of the content row — it is a
            // pure function of the canonical-head link set, which only the
            // storage peer's conductor can evaluate. `/head` is where storage
            // surfaces it.
            Channel::Candidate => {
                let body = self
                    .get_body(&format!("{}/db/content/{}/head", self.storage_base, slug))
                    .await?;
                Ok(parse_candidate_head(&body))
            }
        }
    }
}

/// Parse a `/db/content/{slug}/head` body into this slug's candidate answer.
///
/// Absent, null or empty `stagingCandidate` is [`ChannelHead::NoCandidateStaged`]
/// — the NAMED absence, which the caller answers with and never falls back
/// from. A malformed body is treated the same way rather than as a head: this
/// function's contract is that it can only ever produce a candidate the peer
/// actually declared.
pub fn parse_candidate_head(body: &str) -> ChannelHead {
    let declaration = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("stagingCandidate")
                .and_then(|c| c.as_str())
                .map(str::to_string)
        })
        .filter(|c| !c.is_empty());
    match declaration {
        Some(d) => ChannelHead::Resolved(HeadDoc {
            browser: None,
            server: None,
            staging_declaration: Some(d),
        }),
        None => ChannelHead::NoCandidateStaged,
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
        // A converged read never carries one: the earned winner IS the head
        // these blob hashes name.
        staging_declaration: None,
    }
}

/// One (slug, channel)'s last observed declaration, with when it was observed.
#[derive(Debug, Clone)]
pub struct ObservedHeads {
    pub browser: Option<String>,
    pub server: Option<String>,
    /// The staging declaration this channel resolved to. Only ever `Some` on
    /// [`Channel::Candidate`].
    pub staging_declaration: Option<String>,
    /// TRUE when the last observation on this channel was the candidate
    /// channel's NAMED ABSENCE: asked, and nothing is staged beneath the
    /// earned winner.
    ///
    /// It is a distinct state from "no entry at all" (`get` returns `None`),
    /// which means never asked or never answered. A serving path may say
    /// "nothing staged" only for the former; for the latter it knows nothing.
    /// Neither is ever a licence to serve the converged head.
    pub no_candidate_staged: bool,
    pub observed_at: Instant,
}

/// The reconciled heads, shared between the reconciler (writer) and the SSR
/// adoption pass (reader) so the server head is read ONCE per tick rather than
/// re-fetched by every consumer.
#[derive(Default)]
pub struct BundleHeadStore {
    /// `(slug, channel)` → last observation. Keyed by the PAIR because one app
    /// has two answers: the earned winner its converged hostnames serve, and
    /// the staging declaration its candidate hostname serves. One key would
    /// make the candidate overwrite the head production is serving.
    inner: RwLock<HashMap<(String, Channel), ObservedHeads>>,
}

impl BundleHeadStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// The CONVERGED observation for `slug` — what every pre-rung-4 caller
    /// means by "this slug's head", and what the SSR adoption pass reads.
    pub fn get(&self, slug: &str) -> Option<ObservedHeads> {
        self.get_channel(slug, Channel::Converged)
    }

    /// The observation for one (slug, channel).
    pub fn get_channel(&self, slug: &str, channel: Channel) -> Option<ObservedHeads> {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&(slug.to_string(), channel))
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
        // CONVERGED by construction: the SSR adoption pass serves the earned
        // winner, never a candidate.
        let observed = self.get(slug)?;
        if observed.observed_at.elapsed() > max_age {
            return None;
        }
        observed.server
    }

    /// Record a fresh observation on one channel, returning how it moved.
    pub fn record(&self, slug: &str, channel: Channel, head: &ChannelHead) -> HeadMove {
        let (doc, no_candidate_staged) = match head {
            ChannelHead::Resolved(doc) => (doc.clone(), false),
            // The named absence is RECORDED, not skipped: "asked, nothing
            // staged" is an answer a serving path may act on, and it is
            // distinguishable from "never asked" (no entry) only if it is
            // written down.
            ChannelHead::NoCandidateStaged => (HeadDoc::default(), true),
        };
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let key = (slug.to_string(), channel);
        let previous = inner.get(&key).cloned();
        inner.insert(
            key,
            ObservedHeads {
                browser: doc.browser.clone(),
                server: doc.server.clone(),
                staging_declaration: doc.staging_declaration.clone(),
                no_candidate_staged,
                observed_at: Instant::now(),
            },
        );
        HeadMove {
            slug: slug.to_string(),
            channel,
            browser_from: previous.as_ref().and_then(|p| p.browser.clone()),
            browser_to: doc.browser.clone(),
            server_from: previous.as_ref().and_then(|p| p.server.clone()),
            server_to: doc.server.clone(),
            candidate_from: previous
                .as_ref()
                .and_then(|p| p.staging_declaration.clone()),
            candidate_to: doc.staging_declaration.clone(),
        }
    }
}

/// How one (slug, channel)'s declaration changed across a reconcile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadMove {
    pub slug: String,
    pub channel: Channel,
    pub browser_from: Option<String>,
    pub browser_to: Option<String>,
    pub server_from: Option<String>,
    pub server_to: Option<String>,
    /// The staging declaration, before and after. Both are `None` on the
    /// converged channel; `Some → None` on the candidate channel is a
    /// candidate being promoted or withdrawn, which is a real move.
    pub candidate_from: Option<String>,
    pub candidate_to: Option<String>,
}

impl HeadMove {
    pub fn browser_moved(&self) -> bool {
        self.browser_from != self.browser_to
    }

    pub fn server_moved(&self) -> bool {
        self.server_from != self.server_to
    }

    pub fn candidate_moved(&self) -> bool {
        self.candidate_from != self.candidate_to
    }

    pub fn moved(&self) -> bool {
        self.browser_moved() || self.server_moved() || self.candidate_moved()
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
    reconcile_lock: tokio::sync::Mutex<()>,
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
            reconcile_lock: tokio::sync::Mutex::new(()),
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
    pub async fn reconcile_slug(
        &self,
        slug: &str,
        channel: Channel,
        entry_file: Option<&str>,
    ) -> Option<HeadMove> {
        // Tick and SSE reconciliation must not interleave old reads with newer writes.
        let _guard = self.reconcile_lock.lock().await;
        let head = match self.source.fetch(slug, channel).await {
            Ok(head) => head,
            Err(e) => {
                // Keep the last state. A head is NEVER fabricated, and an
                // unreachable peer must not un-declare a head we already hold.
                // NOTE this is the TRANSPORT arm only: "asked, nothing staged"
                // arrives as `Ok(NoCandidateStaged)` and IS recorded below.
                tracing::debug!(
                    target: "doorway::ssr",
                    slug = %slug,
                    channel = ?channel,
                    error = %e,
                    "bundle heads: declared-head read failed — keeping last state, retrying next tick"
                );
                return None;
            }
        };

        // Re-assert storage truth even when its head did not move. A Mongo outage,
        // late bulk projection, or slug-index eviction can undo a previous write.
        // Observing a head is not evidence that its projection still holds it.
        //
        // CONVERGED ONLY. `write_heads` is the doorway's own per-SLUG
        // declaration — the head its serving path resolves bytes from — and a
        // candidate observation must never touch it. Writing a candidate
        // through here would publish an unpromoted build as the doorway's
        // declared head for every name it serves, which is the exact inversion
        // the channel split exists to prevent.
        if let (Channel::Converged, Some(projection), ChannelHead::Resolved(doc)) =
            (channel, self.projection.as_ref(), &head)
        {
            projection
                .write_heads(slug, doc.browser.as_deref(), doc.server.as_deref())
                .await;
        }
        let mv = self.heads.record(slug, channel, &head);
        if !mv.moved() {
            return None;
        }

        tracing::info!(
            target: "doorway::ssr",
            slug = %slug,
            channel = ?channel,
            entry_file = entry_file.unwrap_or("-"),
            "bundle heads: {} [{:?}] browser {}->{} server {}->{} candidate {}->{}",
            slug,
            channel,
            head12(mv.browser_from.as_deref()),
            head12(mv.browser_to.as_deref()),
            head12(mv.server_from.as_deref()),
            head12(mv.server_to.as_deref()),
            head12(mv.candidate_from.as_deref()),
            head12(mv.candidate_to.as_deref()),
        );

        if mv.browser_moved() && channel == Channel::Converged {
            // Converged only: the warm shell is the shell this doorway serves
            // from its declared head, and a candidate observation must not
            // evict it.
            //
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
                .reconcile_slug(&target.slug, target.channel, target.entry_file.as_deref())
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
    /// Every channel this doorway declares for that slug reconciles — a
    /// content event on an app moves the earned winner AND can promote or
    /// withdraw the candidate standing beneath it, and both are read from the
    /// same event.
    pub async fn on_content_event(&self, id: &str) -> Option<HeadMove> {
        let targets: Vec<BundleTarget> = self
            .targets()
            .into_iter()
            .filter(|t| t.slug == id)
            .collect();
        let mut first = None;
        for target in targets {
            let mv = self
                .reconcile_slug(&target.slug, target.channel, target.entry_file.as_deref())
                .await;
            // The converged move is the one callers log; a candidate move is
            // still reconciled, it just does not displace the answer.
            if first.is_none() || target.channel == Channel::Converged {
                if let Some(mv) = mv {
                    first = Some(mv);
                }
            }
        }
        first
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

    /// In-memory stand-in for the storage peer. Answers per channel: the
    /// converged doc, and the candidate's staging declaration (absent by
    /// default — the named absence).
    #[derive(Default)]
    struct FakeSource {
        doc: Mutex<Option<HeadDoc>>,
        candidate: Mutex<Option<String>>,
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
                staging_declaration: None,
            });
            *self.fail.lock().unwrap() = None;
        }

        /// Stage (or withdraw, with `None`) a candidate beneath the winner.
        fn stage_candidate(&self, declaration: Option<&str>) {
            *self.candidate.lock().unwrap() = declaration.map(str::to_string);
        }

        fn go_dark(&self, why: &str) {
            *self.fail.lock().unwrap() = Some(why.to_string());
        }
    }

    #[async_trait::async_trait]
    impl HeadSource for FakeSource {
        async fn fetch(&self, _slug: &str, channel: Channel) -> Result<ChannelHead, String> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            if let Some(e) = self.fail.lock().unwrap().clone() {
                return Err(e);
            }
            match channel {
                Channel::Converged => self
                    .doc
                    .lock()
                    .unwrap()
                    .clone()
                    .map(ChannelHead::Resolved)
                    .ok_or_else(|| "no doc".into()),
                Channel::Candidate => Ok(match self.candidate.lock().unwrap().clone() {
                    Some(d) => ChannelHead::Resolved(HeadDoc {
                        browser: None,
                        server: None,
                        staging_declaration: Some(d),
                    }),
                    None => ChannelHead::NoCandidateStaged,
                }),
            }
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
                channel: Channel::Converged,
            }]
        })
    }

    /// The same app on BOTH channels — a converged contract and a candidate
    /// contract for one slug, which is the whole rung-4 shape.
    fn both_channel_targets() -> TargetSource {
        Arc::new(|| {
            vec![
                BundleTarget {
                    slug: "landing".into(),
                    entry_file: Some("index.html".into()),
                    channel: Channel::Converged,
                },
                BundleTarget {
                    slug: "landing".into(),
                    entry_file: Some("index.html".into()),
                    channel: Channel::Candidate,
                },
            ]
        })
    }

    fn reconciler_with_targets(
        source: Arc<FakeSource>,
        projection: Arc<dyn HeadProjection>,
        warm: Arc<WarmShellStore>,
        targets: TargetSource,
    ) -> BundleHeadsReconciler {
        BundleHeadsReconciler::new(
            source,
            Some(projection),
            warm,
            Arc::new(BundleHeadStore::new()),
            targets,
        )
    }

    fn reconciler(
        source: Arc<FakeSource>,
        projection: Arc<dyn HeadProjection>,
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
    async fn a_steady_head_repairs_projection_without_reporting_a_head_move() {
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
            after_first + 3,
            "a steady head must repair projection drift on every tick"
        );
    }

    #[tokio::test]
    async fn transient_projection_failure_is_retried_without_another_head_move() {
        struct UnavailableOnce {
            calls: std::sync::atomic::AtomicUsize,
            persisted: Mutex<Option<String>>,
        }
        #[async_trait::async_trait]
        impl HeadProjection for UnavailableOnce {
            async fn write_heads(&self, _slug: &str, browser: Option<&str>, _server: Option<&str>) {
                if self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    return; // Mongo is unavailable on the first observation.
                }
                *self.persisted.lock().unwrap() = browser.map(str::to_string);
            }
        }
        let source = FakeSource::declaring(Some("sha256-current"), None);
        let projection = Arc::new(UnavailableOnce {
            calls: std::sync::atomic::AtomicUsize::new(0),
            persisted: Mutex::new(None),
        });
        let r = reconciler(
            source,
            projection.clone(),
            Arc::new(WarmShellStore::inert()),
        );
        assert_eq!(r.tick().await.len(), 1);
        assert!(projection.persisted.lock().unwrap().is_none());
        assert!(r.tick().await.is_empty());
        assert_eq!(
            projection.persisted.lock().unwrap().as_deref(),
            Some("sha256-current")
        );
        // A late stale projection must heal too, even after a successful write.
        *projection.persisted.lock().unwrap() = Some("sha256-stale".into());
        assert!(r.tick().await.is_empty());
        assert_eq!(
            projection.persisted.lock().unwrap().as_deref(),
            Some("sha256-current")
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

    // =======================================================================
    // Rung 4 slice 1 — heads keyed by (slug, channel)
    //
    // The one behaviour that had to be DESIGNED rather than inherited: a
    // candidate channel with nothing staged answers a NAMED ABSENCE, and never
    // the converged head. Everything else here guards the blast radius —
    // a candidate observation must not touch the doorway's own declaration,
    // its warm shell, or what the SSR adoption pass reads.
    // =======================================================================

    #[tokio::test]
    async fn a_candidate_with_nothing_staged_is_a_named_absence_never_the_converged_head() {
        let source = FakeSource::declaring(Some("sha256-production"), Some("sha256-srv"));
        // No candidate staged — the default.
        let r = reconciler_with_targets(
            source.clone(),
            Arc::new(FakeProjection::default()),
            Arc::new(WarmShellStore::inert()),
            both_channel_targets(),
        );
        r.tick().await;

        let heads = r.heads();
        let candidate = heads
            .get_channel("landing", Channel::Candidate)
            .expect("the absence is RECORDED — distinguishable from never asked");
        assert!(candidate.no_candidate_staged);
        assert_eq!(candidate.staging_declaration, None);
        assert_eq!(
            candidate.browser, None,
            "the candidate channel must NOT inherit the converged browser head"
        );
        assert_eq!(candidate.server, None);
        // …while the converged channel is untouched and complete.
        assert_eq!(
            heads.get("landing").unwrap().browser.as_deref(),
            Some("sha256-production")
        );
    }

    #[tokio::test]
    async fn a_staged_candidate_resolves_its_own_declaration_beside_the_converged_head() {
        let source = FakeSource::declaring(Some("sha256-production"), Some("sha256-srv"));
        source.stage_candidate(Some("uhCkkCANDIDATE01"));
        let r = reconciler_with_targets(
            source.clone(),
            Arc::new(FakeProjection::default()),
            Arc::new(WarmShellStore::inert()),
            both_channel_targets(),
        );
        r.tick().await;

        let heads = r.heads();
        let candidate = heads.get_channel("landing", Channel::Candidate).unwrap();
        assert_eq!(
            candidate.staging_declaration.as_deref(),
            Some("uhCkkCANDIDATE01")
        );
        assert!(!candidate.no_candidate_staged);
        // Two keys, two independent answers, neither overwriting the other.
        assert_eq!(
            heads.get("landing").unwrap().browser.as_deref(),
            Some("sha256-production")
        );
    }

    #[tokio::test]
    async fn a_candidate_observation_never_writes_the_doorways_own_declaration() {
        // The inversion this guards: publishing an unpromoted build as the
        // head every name this doorway serves resolves bytes from.
        let source = FakeSource::declaring(Some("sha256-production"), Some("sha256-srv"));
        source.stage_candidate(Some("uhCkkCANDIDATE01"));
        let projection = Arc::new(FakeProjection::default());
        let r = reconciler_with_targets(
            source.clone(),
            projection.clone(),
            Arc::new(WarmShellStore::inert()),
            both_channel_targets(),
        );
        r.tick().await;

        let writes = projection.writes.lock().unwrap().clone();
        assert_eq!(
            writes.len(),
            1,
            "exactly one write-through: the converged one"
        );
        assert_eq!(writes[0].browser.as_deref(), Some("sha256-production"));
    }

    #[tokio::test]
    async fn the_ssr_adoption_pass_only_ever_reads_the_converged_head() {
        let source = FakeSource::declaring(Some("sha256-b"), Some("sha256-serverEARNED"));
        source.stage_candidate(Some("uhCkkCANDIDATE01"));
        let r = reconciler_with_targets(
            source,
            Arc::new(FakeProjection::default()),
            Arc::new(WarmShellStore::inert()),
            both_channel_targets(),
        );
        r.tick().await;
        assert_eq!(
            r.heads()
                .server_head_fresh("landing", Duration::from_secs(60))
                .as_deref(),
            Some("sha256-serverEARNED"),
            "server_head_fresh is converged by construction"
        );
    }

    #[tokio::test]
    async fn promoting_and_withdrawing_a_candidate_are_both_moves() {
        let source = FakeSource::declaring(Some("sha256-production"), None);
        let r = reconciler_with_targets(
            source.clone(),
            Arc::new(FakeProjection::default()),
            Arc::new(WarmShellStore::inert()),
            both_channel_targets(),
        );
        r.tick().await; // learns: converged head + no candidate

        source.stage_candidate(Some("uhCkkCANDIDATE01"));
        let staged = r.tick().await;
        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].channel, Channel::Candidate);
        assert!(staged[0].candidate_moved());

        // Promotion (or withdrawal) removes the staging declaration.
        source.stage_candidate(None);
        let withdrawn = r.tick().await;
        assert_eq!(withdrawn.len(), 1);
        assert_eq!(withdrawn[0].candidate_to, None);
        assert!(
            r.heads()
                .get_channel("landing", Channel::Candidate)
                .unwrap()
                .no_candidate_staged
        );
    }

    #[tokio::test]
    async fn an_unreachable_peer_is_not_an_absent_candidate() {
        // A transport failure must keep the last state — it is NOT the answer
        // "nothing staged", which a caller is allowed to act on.
        let source = FakeSource::declaring(Some("sha256-production"), None);
        source.stage_candidate(Some("uhCkkCANDIDATE01"));
        let r = reconciler_with_targets(
            source.clone(),
            Arc::new(FakeProjection::default()),
            Arc::new(WarmShellStore::inert()),
            both_channel_targets(),
        );
        r.tick().await;

        source.go_dark("connection refused");
        assert!(r.tick().await.is_empty());
        let candidate = r
            .heads()
            .get_channel("landing", Channel::Candidate)
            .unwrap();
        assert_eq!(
            candidate.staging_declaration.as_deref(),
            Some("uhCkkCANDIDATE01"),
            "the last known candidate survives the outage"
        );
        assert!(!candidate.no_candidate_staged);
    }

    #[tokio::test]
    async fn a_content_event_reconciles_every_channel_that_slug_is_mounted_on() {
        let source = FakeSource::declaring(Some("sha256-production"), None);
        source.stage_candidate(Some("uhCkkCANDIDATE01"));
        let r = reconciler_with_targets(
            source.clone(),
            Arc::new(FakeProjection::default()),
            Arc::new(WarmShellStore::inert()),
            both_channel_targets(),
        );
        assert!(r.on_content_event("landing").await.is_some());
        assert_eq!(source.reads.load(Ordering::SeqCst), 2, "both channels read");
        assert!(r.heads().get("landing").is_some());
        assert!(r
            .heads()
            .get_channel("landing", Channel::Candidate)
            .is_some());
    }

    #[test]
    fn a_head_read_with_no_staging_candidate_parses_as_the_named_absence() {
        for body in [
            r#"{"contentId":"landing","headActionHash":"uhCkkEARNED","declared":true,"trust":"notarized"}"#,
            r#"{"contentId":"landing","stagingCandidate":null}"#,
            r#"{"contentId":"landing","stagingCandidate":""}"#,
            "not json at all",
        ] {
            assert_eq!(
                parse_candidate_head(body),
                ChannelHead::NoCandidateStaged,
                "body {body:?} must name absence, never a head"
            );
        }
    }

    #[test]
    fn a_head_read_carrying_a_staging_candidate_resolves_the_declaration() {
        let head = parse_candidate_head(
            r#"{"contentId":"landing","headActionHash":"uhCkkEARNED","stagingCandidate":"uhCkkCAND"}"#,
        );
        match head {
            ChannelHead::Resolved(doc) => {
                assert_eq!(doc.staging_declaration.as_deref(), Some("uhCkkCAND"));
                assert_eq!(
                    doc.browser, None,
                    "a candidate names a DECLARATION, not a blob head"
                );
            }
            ChannelHead::NoCandidateStaged => panic!("expected a resolved candidate"),
        }
    }
}
