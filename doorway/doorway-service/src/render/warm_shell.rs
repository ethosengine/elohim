//! Warm-boot shell cache — serve `/` through the upstream's catch-up window.
//!
//! ## The defect this closes
//!
//! A composed SSR serve needs the projected app's *bundle-carrying shell* (the
//! browser build's `index.html`, which holds the `<script>` tags an SSR render
//! must hydrate against). That shell used to be fetched from the storage
//! upstream on EVERY `/` request, under the full `EPR_DISPATCH_TIMEOUT_SECS`
//! wall. During post-deploy catch-up the upstream cannot answer, so a browser
//! navigation rode a doomed 10s fetch before falling back — 10.07s for a 200 on
//! doorway-alpha, 20.08s for a 503 on the apex — while the SAME doorway's
//! `/db/content` path shed honestly in 63ms.
//!
//! ## The cure: cache-first, never a doomed hot-path fetch
//!
//! The Mongo-backed `app_file_cache` (keyed `{slug}:{file_path}:{blob_hash}`)
//! already holds the last reconciled bundle — derivable, content-addressed
//! truth that OUTLIVES the pod, unlike the in-memory SSR hot cache. This module
//! projects that archive as a warm shell store:
//!
//! - **boot** — [`WarmShellStore::hydrate`] loads each projected app's entry
//!   file into the in-process hot layer, so the shell is servable before the
//!   upstream has answered anything;
//! - **hot path** — [`resolve_shell`] is cache-first. A warm shell AT the
//!   locally-declared head never touches the upstream, healthy or not;
//! - **cold + unavailable** — shed IMMEDIATELY (the existing 503 catching-up
//!   contract), never a 10-20s stall;
//! - **upgrade** — when the local projection declares a head we hold no bytes
//!   for, the next request re-reads the upstream (under a TIGHT budget, because
//!   we already hold a serviceable answer) and converges.
//!
//! Serving the last reconciled bundle is serving TRUE, content-addressed,
//! possibly-one-behind content, and it is marked as such on the wire
//! (`x-elohim-bundle: last-reconciled`) — it is not a violation of the
//! honest-shed design, whose target is DATA reads. API/data routes keep the shed
//! contract untouched; this seam moves only the shell document.
//!
//! ## Degrade
//!
//! No archive (Mongo unconfigured / unavailable) → the store is INERT: every
//! lookup is `Cold`, so behaviour is byte-for-byte today's fetch-per-request
//! path. Never worse.
//!
//! ## The second defect: a slug is a MOVING POINTER (2026-09-04)
//!
//! The cache above is keyed by the head the doorway's own projection declares,
//! but the fetch that filled it addressed storage by SLUG
//! (`/apps/{slug}/index.html`). Those are two different reads of a pointer that
//! moves: when storage's slug index had not yet advanced (or had advanced past)
//! the doorway's declared head, old-era bytes were stocked UNDER the new head,
//! classified `AtHead`, and served forever with no upstream re-check — a blank
//! `elohim.host` referencing a `main-*.js` that 404s.
//!
//! Three rails close it, and they are the reason for `head_bound`:
//!
//! 1. **bind the fetch to the head** — [`resolve_shell`] hands its fetch
//!    closure the declared head it classified against, so the caller addresses
//!    `/apps/{head}/{entry}` (what `routes/apps.rs::resolved_app_path` has
//!    always done) and the bytes are provably the bytes of that head;
//! 2. **`AtHead` requires proof** — a shell only classifies `AtHead` when it
//!    carries `head_bound` (it WAS fetched by this hash). An archive doc from
//!    the poisoned era carries no marker, so it reads `Behind` and buys exactly
//!    one hash-addressed upgrade: the fleet self-heals on deploy, with no
//!    operator action;
//! 3. **an unknown head is not a confirmed one** — `resolve_blob_hash`
//!    answering `None` (the live alpha shape: `head:""`,
//!    `x-projection-ready: false`) used to read `AtHead` and pin the shell
//!    permanently. It is `Behind`, re-checked at most once per
//!    [`SHELL_UPGRADE_RETRY_SECS`] so it never becomes a fetch per request.
//!
//! Spec: Task 3.4 of
//! `genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md`.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A shell document read out of the persistent archive.
#[derive(Debug, Clone)]
pub struct ArchivedShell {
    pub blob_hash: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
    /// These bytes were FETCHED by `blob_hash`, not merely stocked under it.
    /// False for every doc written before the moving-pointer fix — such a doc
    /// is servable but is never proof of the head (see the module doc).
    pub head_bound: bool,
}

/// The persistent, pod-outliving home of the last reconciled bundle.
///
/// Implemented over the Mongo-backed `app_file_cache`; a trait so the warm-shell
/// decisions are unit-testable without a live Mongo (the same mocking seam shape
/// the storage crate uses for `CommitmentFetcher`/`RateHistory`).
///
/// Every method is LOCAL by construction — a `ShellArchive` must never reach the
/// storage upstream, or the cache-first guarantee this module exists to make
/// would be a lie.
#[async_trait::async_trait]
pub trait ShellArchive: Send + Sync {
    /// The blob hash the doorway's own projection currently declares for `slug`.
    /// `None` when the projection has never seen this app.
    async fn declared_blob_hash(&self, slug: &str) -> Option<String>;

    /// Archived bytes for one specific (content-addressed) head.
    async fn load(&self, slug: &str, file_path: &str, blob_hash: &str) -> Option<ArchivedShell>;

    /// The most recently stocked copy at ANY head — the last reconciled bundle.
    /// Used when the declared head is unknown, or known but not yet stocked.
    async fn load_latest(&self, slug: &str, file_path: &str) -> Option<ArchivedShell>;

    /// Stock a freshly-read shell so the NEXT pod can serve it warm.
    ///
    /// This trait method IS the head-bound stock path: `resolve_shell` only
    /// reaches it with bytes fetched by `blob_hash`, so implementations mark
    /// what they write.
    async fn store(
        &self,
        slug: &str,
        file_path: &str,
        blob_hash: &str,
        content_type: &str,
        bytes: Vec<u8>,
    );
}

/// The live archive: the Mongo-backed `app_file_cache`, which already holds the
/// last reconciled bundle keyed `{slug}:{file_path}:{blob_hash}`. Nothing new is
/// stored — this projects what the delivery path already stocks.
#[async_trait::async_trait]
impl ShellArchive for crate::cache::AppFileCacheService {
    async fn declared_blob_hash(&self, slug: &str) -> Option<String> {
        // Reads the doorway's OWN projection store (`projected_entries`), never
        // the storage upstream — the whole point of the cache-first path.
        self.resolve_blob_hash(slug).await
    }

    async fn load(&self, slug: &str, file_path: &str, blob_hash: &str) -> Option<ArchivedShell> {
        self.get(slug, file_path, blob_hash)
            .await
            .map(|f| ArchivedShell {
                blob_hash: f.blob_hash,
                content_type: f.content_type,
                bytes: f.data,
                head_bound: f.head_bound,
            })
    }

    async fn load_latest(&self, slug: &str, file_path: &str) -> Option<ArchivedShell> {
        self.latest_file(slug, file_path)
            .await
            .map(|f| ArchivedShell {
                blob_hash: f.blob_hash,
                content_type: f.content_type,
                bytes: f.data,
                head_bound: f.head_bound,
            })
    }

    async fn store(
        &self,
        slug: &str,
        file_path: &str,
        blob_hash: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) {
        self.put_head_bound(slug, file_path, blob_hash, content_type, bytes)
            .await;
    }
}

/// A shell document ready to serve, with the head it was stocked under.
#[derive(Debug, Clone)]
pub struct WarmShell {
    pub blob_hash: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
    /// See [`ArchivedShell::head_bound`]. `lookup_with_declared` requires it
    /// for [`WarmClass::AtHead`], which is what makes the invariant
    /// "AtHead ⇒ these bytes were fetched by THIS hash" hold.
    pub head_bound: bool,
}

impl WarmShell {
    /// The shell as HTML. Lossy by design — a shell that is not valid UTF-8 is
    /// already unserviceable, and panicking on the `/` hot path is never the
    /// honest answer.
    pub fn html(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

/// Where a served shell came from — the provenance behind the
/// `x-elohim-bundle` staleness marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellProvenance {
    /// Served from the doorway's own projection of the last reconciled bundle,
    /// with NO upstream read in this request. Marked on the wire.
    LastReconciled,
    /// Read from the upstream in this request — the declared head, confirmed.
    /// Carries no staleness marker.
    DeclaredHead,
    /// Read from the upstream in this request, but addressed by SLUG because
    /// the projection declared no head to bind to. A fetch happened, so it is
    /// not `LastReconciled` — but nothing confirmed a head, so it must not
    /// borrow [`Self::DeclaredHead`]'s silence either.
    SlugResolved,
}

impl ShellProvenance {
    /// The `x-elohim-bundle` header value, or `None` when the serve confirmed
    /// the declared head this request (no marker).
    pub fn header_value(&self) -> Option<&'static str> {
        match self {
            ShellProvenance::LastReconciled => Some("last-reconciled"),
            ShellProvenance::SlugResolved => Some("slug-resolved"),
            ShellProvenance::DeclaredHead => None,
        }
    }

    /// The provenance of bytes just read from the upstream, derived from the
    /// BYTES rather than from the fact that a read happened: a hash-addressed
    /// read confirms the declared head, a slug-addressed one does not.
    pub fn for_fresh(head_bound: bool) -> Self {
        if head_bound {
            ShellProvenance::DeclaredHead
        } else {
            ShellProvenance::SlugResolved
        }
    }
}

/// How the warm store's answer relates to the locally-declared head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarmClass {
    /// We hold bytes that were FETCHED BY the head the local projection
    /// declares. Bytes merely stocked under it do not qualify.
    AtHead,
    /// We hold bytes, but they are not proof of the declared head — the
    /// projection declares a different head, declares none at all, or the
    /// bytes carry no `head_bound` marker.
    Behind,
    /// We hold nothing.
    Cold,
}

/// What to do for this shell request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellPlan {
    /// Serve what we hold; touch nothing upstream.
    ServeWarm,
    /// Try to converge to the declared head under a tight budget, falling back
    /// to the one-behind copy if the upstream cannot answer.
    UpgradeThenWarm,
    /// We hold nothing and the upstream can answer — read it.
    Fetch,
    /// We hold nothing and the upstream cannot answer — shed honestly, now.
    Shed,
}

/// The pure hot-path decision.
///
/// The load-bearing arm is `(AtHead, _) → ServeWarm`: a warm shell NEVER pays a
/// per-request upstream fetch, healthy upstream or not. That is what makes `/`
/// survive the catch-up window.
/// `upgrade_due` is the [`SHELL_UPGRADE_RETRY_SECS`] rate limit
/// ([`WarmShellStore::upgrade_due`]): a `Behind` shell that cannot converge —
/// an unknown declared head never does — must not re-read the upstream on every
/// request. Not due ⇒ serve what we hold, exactly as an unavailable upstream
/// already does.
pub fn decide_shell_serve(
    class: WarmClass,
    upstream_available: bool,
    upgrade_due: bool,
) -> ShellPlan {
    match (class, upstream_available) {
        (WarmClass::AtHead, _) => ShellPlan::ServeWarm,
        (WarmClass::Behind, true) if upgrade_due => ShellPlan::UpgradeThenWarm,
        (WarmClass::Behind, _) => ShellPlan::ServeWarm,
        (WarmClass::Cold, true) => ShellPlan::Fetch,
        (WarmClass::Cold, false) => ShellPlan::Shed,
    }
}

/// The budget a caller should give its upstream read. An upgrade already holds
/// a serviceable answer, so it must never gamble the full dispatch wall on a
/// marginally better one — a parameter-bearing distinction, not a style choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchBudget {
    /// Cold cache — nothing else to serve, so the full dispatch budget applies.
    Full,
    /// We hold a one-behind shell; converge cheaply or keep serving.
    Upgrade,
}

/// Seconds allowed for an UPGRADE read (see [`FetchBudget::Upgrade`]).
pub const SHELL_UPGRADE_TIMEOUT_SECS: u64 = 2;

/// Minimum seconds between two upgrade READS of the same shell.
///
/// A `Behind` shell whose declared head is UNKNOWN can never converge to
/// `AtHead` — so without this it would buy an upstream fetch on every single
/// `/` request. Serving the warm bytes in between is exactly the module's
/// possibly-one-behind contract; this only bounds how often we re-ask.
pub const SHELL_UPGRADE_RETRY_SECS: u64 = 30;

/// A freshly-read shell handed back by the caller's upstream fetch closure.
#[derive(Debug, Clone)]
pub struct FetchedShell {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

/// The terminal answer for one shell request.
#[derive(Debug)]
pub enum ShellOutcome {
    /// Served from the doorway's projection — no upstream read this request.
    Warm(WarmShell),
    /// Read from the upstream this request.
    Fresh(WarmShell),
    /// Nothing to serve and no upstream to ask — the caller sheds via the
    /// existing catching-up contract.
    Unavailable,
}

impl ShellOutcome {
    pub fn provenance(&self) -> Option<ShellProvenance> {
        match self {
            ShellOutcome::Warm(_) => Some(ShellProvenance::LastReconciled),
            ShellOutcome::Fresh(shell) => Some(ShellProvenance::for_fresh(shell.head_bound)),
            ShellOutcome::Unavailable => None,
        }
    }
}

/// Boot-hydrated hot layer over a [`ShellArchive`].
///
/// The hot layer is a per-pod convenience (it saves a Mongo round-trip per
/// request); the ARCHIVE is what makes the cache survive a restart, which is the
/// whole point — an in-memory-only shell cache dies exactly when the catch-up
/// window opens.
pub struct WarmShellStore {
    hot: RwLock<HashMap<String, WarmShell>>,
    archive: Option<Arc<dyn ShellArchive>>,
    /// Last upstream upgrade attempt per `{slug}:{file_path}` — the
    /// [`SHELL_UPGRADE_RETRY_SECS`] rate limit's only state.
    upgrade_attempts: RwLock<HashMap<String, std::time::Instant>>,
}

impl WarmShellStore {
    pub fn new(archive: Option<Arc<dyn ShellArchive>>) -> Self {
        Self {
            hot: RwLock::new(HashMap::new()),
            archive,
            upgrade_attempts: RwLock::new(HashMap::new()),
        }
    }

    /// A store with no persistent archive — inert, so callers keep exactly
    /// today's fetch-per-request behaviour.
    pub fn inert() -> Self {
        Self::new(None)
    }

    /// True when this store has a persistent archive behind it.
    ///
    /// An inert store is not a slower cache — it is a **disabled** one:
    /// [`Self::lookup_with_declared`] returns `Cold` before it consults the hot
    /// map at all, so [`Self::stock`] writes that can never be read. Wiring
    /// (`AppState::init_projection`) asserts on this, because for the whole of
    /// Task 3.4's deployed life every production doorway held an inert store
    /// and nobody could tell from the outside.
    pub fn is_archive_backed(&self) -> bool {
        self.archive.is_some()
    }

    fn key(slug: &str, file_path: &str) -> String {
        format!("{slug}:{file_path}")
    }

    fn hot_get(&self, slug: &str, file_path: &str) -> Option<WarmShell> {
        self.hot
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&Self::key(slug, file_path))
            .cloned()
    }

    fn hot_put(&self, slug: &str, file_path: &str, shell: WarmShell) {
        self.hot
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(Self::key(slug, file_path), shell);
    }

    /// Drop every hot shell for `slug`.
    ///
    /// The hot map is consulted BEFORE the archive, so clearing the slug's
    /// `app_file_cache` documents alone leaves the stale bytes serving — which
    /// is why `/admin/cache/clear/{slug}` could not repair a poisoned shell.
    /// Every site that clears the archive for a slug must call this too.
    pub fn evict(&self, slug: &str) {
        let prefix = format!("{slug}:");
        let mut hot = self.hot.write().unwrap_or_else(|e| e.into_inner());
        hot.retain(|k, _| !k.starts_with(&prefix));
        let mut attempts = self
            .upgrade_attempts
            .write()
            .unwrap_or_else(|e| e.into_inner());
        attempts.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Claim the one upgrade-read slot for this shell, returning whether the
    /// caller won it — see [`SHELL_UPGRADE_RETRY_SECS`].
    ///
    /// Check and record happen under ONE write lock ON PURPOSE. A
    /// `due()`-then-`note()` pair is two acquisitions, so every concurrent `/`
    /// request read "due" before any of them recorded, and they ALL launched an
    /// upgrade — the rate limit existed and did nothing under the only load
    /// that needs it. Claiming also prunes expired entries, which is what keeps
    /// the map bounded on a long-lived pod.
    pub fn try_claim_upgrade(&self, slug: &str, file_path: &str) -> bool {
        let key = Self::key(slug, file_path);
        let now = std::time::Instant::now();
        let mut attempts = self
            .upgrade_attempts
            .write()
            .unwrap_or_else(|e| e.into_inner());
        attempts.retain(|k, at| k == &key || at.elapsed().as_secs() < SHELL_UPGRADE_RETRY_SECS);
        match attempts.get(&key) {
            Some(at) if at.elapsed().as_secs() < SHELL_UPGRADE_RETRY_SECS => false,
            _ => {
                attempts.insert(key, now);
                true
            }
        }
    }

    /// Release the claim slot — called when the shell reaches `AtHead`, i.e.
    /// the upgrade this slot was rate-limiting has converged and there is
    /// nothing left to re-ask for. Probes under a READ lock first so the
    /// steady-state at-head hot path never contends for the writer.
    fn clear_upgrade_attempt(&self, slug: &str, file_path: &str) {
        let key = Self::key(slug, file_path);
        let held = self
            .upgrade_attempts
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&key);
        if held {
            self.upgrade_attempts
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&key);
        }
    }

    /// Live claim-slot count — the bound this map is asserted against.
    pub fn upgrade_attempt_count(&self) -> usize {
        self.upgrade_attempts
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    /// Load each `(slug, entry_file)` into the hot layer at boot. Returns the
    /// number of shells hydrated. Best-effort: a missing archive or a missing
    /// entry is a 0, never an error — the lazy path still converges.
    pub async fn hydrate(&self, targets: &[(String, String)]) -> usize {
        let Some(archive) = self.archive.as_ref() else {
            return 0;
        };
        let mut hydrated = 0usize;
        for (slug, entry_file) in targets {
            let declared = archive.declared_blob_hash(slug).await;
            let found = match &declared {
                Some(hash) => match archive.load(slug, entry_file, hash).await {
                    Some(a) => Some(a),
                    None => archive.load_latest(slug, entry_file).await,
                },
                None => archive.load_latest(slug, entry_file).await,
            };
            if let Some(a) = found {
                self.hot_put(
                    slug,
                    entry_file,
                    WarmShell {
                        blob_hash: a.blob_hash,
                        content_type: a.content_type,
                        bytes: a.bytes,
                        // An unmarked doc hydrates as SERVABLE but never as
                        // at-head: `lookup_with_declared` re-classifies it
                        // `Behind`, so boot cannot pin a poisoned shell.
                        head_bound: a.head_bound,
                    },
                );
                hydrated += 1;
            }
        }
        if hydrated > 0 {
            tracing::info!(
                target: "doorway::ssr",
                hydrated,
                targets = targets.len(),
                "warm-boot shell cache hydrated from the last reconciled bundle"
            );
        }
        hydrated
    }

    /// Look up the shell, classifying it against the locally-declared head.
    ///
    /// Every step is local (hot map, then archive) — no upstream read, ever.
    pub async fn lookup(&self, slug: &str, file_path: &str) -> (WarmClass, Option<WarmShell>) {
        let (class, warm, _declared) = self.lookup_with_declared(slug, file_path).await;
        (class, warm)
    }

    /// [`Self::lookup`], also returning the declared head it classified
    /// against. A caller that goes on to fetch-and-stock must key the stock by
    /// THIS head, not re-resolve after the fetch: a projection advance during
    /// the fetch would relabel old-era bytes as the new head and serve them
    /// `AtHead` with no staleness marker.
    pub async fn lookup_with_declared(
        &self,
        slug: &str,
        file_path: &str,
    ) -> (WarmClass, Option<WarmShell>, Option<String>) {
        let Some(archive) = self.archive.as_ref() else {
            // Inert store: nothing warm, so the caller takes today's path.
            return (WarmClass::Cold, None, None);
        };
        let declared = archive.declared_blob_hash(slug).await;

        if let Some(head) = declared.as_deref() {
            // `head_bound` is the load-bearing half of each test: bytes merely
            // STOCKED under `head` are not proof of it. Without that clause a
            // slug-addressed fetch from the wrong era serves `AtHead` forever.
            if let Some(hot) = self.hot_get(slug, file_path) {
                if hot.blob_hash == head && hot.head_bound {
                    self.clear_upgrade_attempt(slug, file_path);
                    return (WarmClass::AtHead, Some(hot), declared);
                }
            }
            if let Some(a) = archive.load(slug, file_path, head).await {
                if a.head_bound {
                    let shell = WarmShell {
                        blob_hash: a.blob_hash,
                        content_type: a.content_type,
                        bytes: a.bytes,
                        head_bound: true,
                    };
                    self.hot_put(slug, file_path, shell.clone());
                    self.clear_upgrade_attempt(slug, file_path);
                    return (WarmClass::AtHead, Some(shell), declared);
                }
            }
        }

        // Declared head unknown, or known but not stocked: the last reconciled
        // copy is still true content — one behind, and marked as such.
        let behind = match self.hot_get(slug, file_path) {
            Some(hot) => Some(hot),
            None => archive.load_latest(slug, file_path).await.map(|a| {
                let shell = WarmShell {
                    blob_hash: a.blob_hash,
                    content_type: a.content_type,
                    bytes: a.bytes,
                    head_bound: a.head_bound,
                };
                self.hot_put(slug, file_path, shell.clone());
                shell
            }),
        };
        match behind {
            // Bytes in hand, but nothing here proves they are the declared
            // head — including the case where the projection declares NO head
            // (live alpha, 2026-09-04: `resolve_blob_hash` → None). An unknown
            // head used to read `AtHead`, which pinned a stale shell for the
            // life of the pod. It is `Behind`: served warm, and re-checked at
            // most once per SHELL_UPGRADE_RETRY_SECS.
            Some(shell) => (WarmClass::Behind, Some(shell), declared),
            None => (WarmClass::Cold, None, declared),
        }
    }

    /// Stock a freshly-read shell into both layers.
    pub async fn stock(
        &self,
        slug: &str,
        file_path: &str,
        blob_hash: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) {
        self.hot_put(
            slug,
            file_path,
            WarmShell {
                blob_hash: blob_hash.to_string(),
                content_type: content_type.to_string(),
                bytes: bytes.clone(),
                // Only `resolve_shell`'s hash-addressed fetch reaches here.
                head_bound: true,
            },
        );
        if let Some(archive) = self.archive.as_ref() {
            archive
                .store(slug, file_path, blob_hash, content_type, bytes)
                .await;
        }
    }

    /// Stock bytes read from the upstream by SLUG, because the projection
    /// declared no head to bind them to.
    ///
    /// HOT LAYER ONLY: with no declared head there is no content address to key
    /// an archive doc under, and inventing one is how the moving-pointer defect
    /// happened. But the hot entry MUST be replaced — a caller that only logged
    /// here served the fresh page once and then re-served the stale one for the
    /// rest of the retry interval, so convergence never stuck.
    pub async fn stock_unbound(
        &self,
        slug: &str,
        file_path: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) {
        self.hot_put(
            slug,
            file_path,
            WarmShell {
                blob_hash: String::new(),
                content_type: content_type.to_string(),
                bytes,
                head_bound: false,
            },
        );
    }
}

/// A shell request's plan, with the state it was decided from.
///
/// The EPR dispatch cannot use [`resolve_shell`] wholesale — a COLD store there
/// keeps its own full-budget proxy semantics (status pass-through, so a genuine
/// 404 reaches the browser as a 404). It must still decide through the ONE
/// predicate and take the upgrade claim through the ONE atomic claim, or the
/// two paths drift — which is exactly how the dispatch arm ended up handing
/// visitors a hash-addressed 404 instead of the warm bytes it was holding.
#[derive(Debug)]
pub struct ShellDecision {
    pub plan: ShellPlan,
    pub warm: Option<WarmShell>,
    pub declared: Option<String>,
}

/// Classify the shell and decide the plan, claiming the upgrade slot atomically
/// when (and only when) the plan would be an upgrade.
pub async fn plan_shell_serve(
    store: &WarmShellStore,
    slug: &str,
    entry_file: &str,
    upstream_available: bool,
) -> ShellDecision {
    let (class, warm, declared) = store.lookup_with_declared(slug, entry_file).await;
    // Claim ONLY on the arm that would spend it — an at-head or cold request
    // must not consume the interval an upgrade is waiting for.
    let upgrade_claimed = class == WarmClass::Behind
        && upstream_available
        && store.try_claim_upgrade(slug, entry_file);
    ShellDecision {
        plan: decide_shell_serve(class, upstream_available, upgrade_claimed),
        warm,
        declared,
    }
}

/// Resolve the shell for one request: cache-first, shed-fast, upgrade-when-able.
///
/// `upstream_available` is the caller's honest read of whether the storage peer
/// can answer AT ALL right now (breaker closed + a storage URL configured) — it
/// is what turns a doomed 10s fetch into an immediate honest shed.
///
/// `fetch` performs the upstream read, honouring the [`FetchBudget`] it is
/// handed; `None` means the read failed (any cause). It is also handed the
/// DECLARED HEAD this request classified against — the fetch must be addressed
/// by that hash (`/apps/{head}/{entry}`), because it is the key the bytes will
/// be stocked under and a slug is a moving pointer. `None` means the projection
/// declares no head, so only the slug can name it and nothing is archived.
pub async fn resolve_shell<F, Fut>(
    store: &WarmShellStore,
    slug: &str,
    entry_file: &str,
    upstream_available: bool,
    fetch: F,
) -> ShellOutcome
where
    F: FnOnce(FetchBudget, Option<String>) -> Fut,
    Fut: std::future::Future<Output = Option<FetchedShell>>,
{
    let ShellDecision {
        plan,
        warm,
        declared,
    } = plan_shell_serve(store, slug, entry_file, upstream_available).await;
    match plan {
        ShellPlan::ServeWarm => match warm {
            Some(shell) => ShellOutcome::Warm(shell),
            // Unreachable by construction (ServeWarm only follows a hit); an
            // honest shed beats an unwrap on the hot path.
            None => ShellOutcome::Unavailable,
        },
        ShellPlan::UpgradeThenWarm => {
            match fetch(FetchBudget::Upgrade, declared.clone()).await {
                Some(fresh) => stock_and_return(store, slug, entry_file, declared, fresh).await,
                // The upgrade could not land — keep serving the one-behind shell.
                // Strictly better than today, which shed to a bundle fallback that
                // then paid the SAME doomed fetch again.
                None => match warm {
                    Some(shell) => ShellOutcome::Warm(shell),
                    None => ShellOutcome::Unavailable,
                },
            }
        }
        ShellPlan::Fetch => match fetch(FetchBudget::Full, declared.clone()).await {
            Some(fresh) => stock_and_return(store, slug, entry_file, declared, fresh).await,
            None => ShellOutcome::Unavailable,
        },
        ShellPlan::Shed => ShellOutcome::Unavailable,
    }
}

/// Stock a freshly-read shell under the head the local projection declared when
/// the fetch was decided, then hand it back as `Fresh`.
async fn stock_and_return(
    store: &WarmShellStore,
    slug: &str,
    entry_file: &str,
    declared: Option<String>,
    fresh: FetchedShell,
) -> ShellOutcome {
    // The fetch was ADDRESSED by `declared` (see `resolve_shell`), so stocking
    // under it is a content-addressed truth, not a label. It is never
    // re-resolved here: a projection advance during the fetch would otherwise
    // relabel bytes proven for the old head as the new one. With no declared
    // head the fetch was necessarily slug-addressed — serve the bytes hot, but
    // never archive them and never mark them head-bound.
    let shell = WarmShell {
        blob_hash: declared.clone().unwrap_or_default(),
        content_type: fresh.content_type.clone(),
        bytes: fresh.bytes.clone(),
        head_bound: declared.is_some(),
    };
    match declared {
        Some(hash) => {
            store
                .stock(slug, entry_file, &hash, &fresh.content_type, fresh.bytes)
                .await;
        }
        None => {
            store
                .stock_unbound(slug, entry_file, &fresh.content_type, fresh.bytes)
                .await;
        }
    }
    ShellOutcome::Fresh(shell)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE RAIL THAT WAS MISSING. An inert store is a DISABLED cache, not a
    /// cold one — and nothing in the suite said so, which is how Task 3.4
    /// shipped, passed nine tests, was written up as "`/` is cache-first", and
    /// then did nothing at all on every deployed doorway for its entire life.
    /// The tests all built stores WITH an archive; production built them
    /// without one and no test looked.
    #[tokio::test]
    async fn stocking_an_inert_store_still_serves_nothing() {
        let store = WarmShellStore::inert();
        assert!(!store.is_archive_backed());

        store
            .stock(
                "app",
                "index.html",
                "sha256-abc",
                "text/html",
                b"hi".to_vec(),
            )
            .await;

        let (class, warm) = store.lookup("app", "index.html").await;
        assert_eq!(
            class,
            WarmClass::Cold,
            "an inert store reports Cold even for bytes it just accepted — \
             lookup_with_declared short-circuits BEFORE the hot map"
        );
        assert!(warm.is_none());
        assert_eq!(
            decide_shell_serve(class, true, true),
            ShellPlan::Fetch,
            "so `/` pays a full upstream fetch on EVERY request — the 20s apex"
        );
    }

    #[tokio::test]
    async fn an_inert_store_hydrates_nothing() {
        let store = WarmShellStore::inert();
        let targets = vec![("app".to_string(), "index.html".to_string())];
        assert_eq!(
            store.hydrate(&targets).await,
            0,
            "hydrate short-circuits on a None archive — the boot-time log line \
             read `hydrated: 0` every time and looked like a cold archive"
        );
    }

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    /// One archived file in the fake, in stock order — last is newest.
    struct StockedFile {
        slug: String,
        file_path: String,
        blob_hash: String,
        bytes: Vec<u8>,
        head_bound: bool,
    }

    /// In-memory stand-in for the Mongo-backed `app_file_cache`.
    #[derive(Default)]
    struct FakeArchive {
        declared: Mutex<Option<String>>,
        files: Mutex<Vec<StockedFile>>,
        loads: AtomicUsize,
    }

    impl FakeArchive {
        fn with_shell(declared: &str, bytes: &str) -> Arc<Self> {
            let a = Arc::new(FakeArchive::default());
            *a.declared.lock().unwrap() = Some(declared.to_string());
            a.files.lock().unwrap().push(StockedFile {
                slug: "landing".into(),
                file_path: "index.html".into(),
                blob_hash: declared.into(),
                bytes: bytes.as_bytes().to_vec(),
                head_bound: true,
            });
            a
        }

        /// A doc from the poisoned era: stocked under a head it was never
        /// fetched by, so it carries no head-bound marker. This is exactly what
        /// the deployed alpha archive holds for `elohim-host-landing`.
        fn with_unmarked_shell(declared: &str, bytes: &str) -> Arc<Self> {
            let a = Arc::new(FakeArchive::default());
            *a.declared.lock().unwrap() = Some(declared.to_string());
            a.files.lock().unwrap().push(StockedFile {
                slug: "landing".into(),
                file_path: "index.html".into(),
                blob_hash: declared.into(),
                bytes: bytes.as_bytes().to_vec(),
                head_bound: false,
            });
            a
        }

        fn declare(&self, hash: &str) {
            *self.declared.lock().unwrap() = Some(hash.to_string());
        }
    }

    #[async_trait::async_trait]
    impl ShellArchive for FakeArchive {
        async fn declared_blob_hash(&self, _slug: &str) -> Option<String> {
            self.declared.lock().unwrap().clone()
        }

        async fn load(
            &self,
            slug: &str,
            file_path: &str,
            blob_hash: &str,
        ) -> Option<ArchivedShell> {
            self.loads.fetch_add(1, Ordering::SeqCst);
            self.files
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|f| f.slug == slug && f.file_path == file_path && f.blob_hash == blob_hash)
                .map(|f| ArchivedShell {
                    blob_hash: f.blob_hash.clone(),
                    content_type: "text/html".into(),
                    bytes: f.bytes.clone(),
                    head_bound: f.head_bound,
                })
        }

        async fn load_latest(&self, slug: &str, file_path: &str) -> Option<ArchivedShell> {
            self.loads.fetch_add(1, Ordering::SeqCst);
            self.files
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|f| f.slug == slug && f.file_path == file_path)
                .map(|f| ArchivedShell {
                    blob_hash: f.blob_hash.clone(),
                    content_type: "text/html".into(),
                    bytes: f.bytes.clone(),
                    head_bound: f.head_bound,
                })
        }

        async fn store(
            &self,
            slug: &str,
            file_path: &str,
            blob_hash: &str,
            _content_type: &str,
            bytes: Vec<u8>,
        ) {
            self.files.lock().unwrap().push(StockedFile {
                slug: slug.into(),
                file_path: file_path.into(),
                blob_hash: blob_hash.into(),
                bytes,
                // `ShellArchive::store` IS the head-bound stock path.
                head_bound: true,
            });
        }
    }

    type BoxedFetch =
        std::pin::Pin<Box<dyn std::future::Future<Output = Option<FetchedShell>> + Send>>;

    /// A fetch closure that counts upstream reads — the `x-ssr-fetches: 0`
    /// analogue for the shell seam: the whole point of this cache is that a
    /// warm doorway NEVER touches a catching-up upstream on the `/` hot path.
    fn counting_fetch(
        counter: Arc<AtomicUsize>,
        result: Option<(&'static str, &'static str)>,
    ) -> impl FnOnce(FetchBudget, Option<String>) -> BoxedFetch {
        move |_budget, _declared| {
            counter.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                result.map(|(body, ct)| FetchedShell {
                    bytes: body.as_bytes().to_vec(),
                    content_type: ct.to_string(),
                })
            })
        }
    }

    // ── (1) boot hydration ───────────────────────────────────────────────────

    #[tokio::test]
    async fn boot_hydration_makes_the_shell_servable_with_zero_upstream_fetches() {
        let archive = FakeArchive::with_shell("sha256-a", "<html><app-root></app-root></html>");
        let store = WarmShellStore::new(Some(archive.clone()));

        let hydrated = store
            .hydrate(&[("landing".to_string(), "index.html".to_string())])
            .await;
        assert_eq!(hydrated, 1, "boot hydration must load the archived shell");

        let fetches = Arc::new(AtomicUsize::new(0));
        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(fetches.clone(), None),
        )
        .await;

        match outcome {
            ShellOutcome::Warm(shell) => {
                assert!(shell.html().contains("app-root"));
                assert_eq!(shell.blob_hash, "sha256-a");
            }
            other => panic!("expected a warm shell, got {other:?}"),
        }
        assert_eq!(
            fetches.load(Ordering::SeqCst),
            0,
            "a warm shell must never ride an upstream fetch on the / hot path"
        );
    }

    /// The stocking key is the head as declared when the fetch was DECIDED, not
    /// re-read after it lands: a projection advance during the (up to 10s)
    /// upstream fetch must not relabel old-era bytes as the new head, or they
    /// would serve as `AtHead` with no staleness marker — violating the
    /// module's possibly-one-behind-is-marked-as-such contract.
    #[tokio::test]
    async fn a_projection_advance_mid_fetch_does_not_relabel_old_bytes_as_the_new_head() {
        let archive = Arc::new(FakeArchive::default());
        archive.declare("sha256-h1");
        let store = WarmShellStore::new(Some(archive.clone()));

        let racing = archive.clone();
        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            move |_budget, _declared| {
                Box::pin(async move {
                    // The declared head advances while the fetch is in flight.
                    racing.declare("sha256-h2");
                    Some(FetchedShell {
                        bytes: b"<html>h1-era</html>".to_vec(),
                        content_type: "text/html".to_string(),
                    })
                }) as BoxedFetch
            },
        )
        .await;

        match outcome {
            ShellOutcome::Fresh(shell) => assert_eq!(
                shell.blob_hash, "sha256-h1",
                "bytes fetched under h1 must not be stocked as h2"
            ),
            other => panic!("expected a fresh shell, got {other:?}"),
        }

        // The h1-era copy is honestly one-behind now, never AtHead under h2.
        let (class, warm) = store.lookup("landing", "index.html").await;
        assert_eq!(class, WarmClass::Behind);
        assert_eq!(warm.unwrap().blob_hash, "sha256-h1");
    }

    // ── the moving-pointer defect (2026-09-04) ───────────────────────────────

    /// A slug is a MOVING POINTER. The upstream read that fills this cache must
    /// be addressed by the very head the bytes will be stocked under, so
    /// `resolve_shell` hands the fetch closure the head it captured at lookup
    /// time. Addressing `/apps/{slug}/index.html` while stocking under the
    /// locally-declared head is how old-era bytes were labelled `sha256-7725d4…`
    /// on doorway-alpha and served `AtHead` forever.
    #[tokio::test]
    async fn the_fetch_closure_is_handed_the_head_the_bytes_will_be_stocked_under() {
        let archive = FakeArchive::with_shell("sha256-h1", "<html>old</html>");
        let store = WarmShellStore::new(Some(archive.clone()));
        archive.declare("sha256-h2");

        let seen: Arc<Mutex<Vec<Option<String>>>> = Arc::new(Mutex::new(Vec::new()));
        let seen_c = seen.clone();
        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            move |_budget, declared| {
                seen_c.lock().unwrap().push(declared);
                Box::pin(async move {
                    Some(FetchedShell {
                        bytes: b"<html>new</html>".to_vec(),
                        content_type: "text/html".to_string(),
                    })
                }) as BoxedFetch
            },
        )
        .await;

        assert_eq!(
            *seen.lock().unwrap(),
            vec![Some("sha256-h2".to_string())],
            "the fetch must be addressable by the head it will be stocked under"
        );
        match outcome {
            ShellOutcome::Fresh(shell) => assert_eq!(shell.blob_hash, "sha256-h2"),
            other => panic!("expected a fresh shell, got {other:?}"),
        }
    }

    /// Self-heal for the already-poisoned archive. An `app_file_cache` doc that
    /// carries no head-bound marker was stocked under a head it was never
    /// fetched by — the invariant `AtHead ⇒ these bytes were fetched by THIS
    /// hash` does not hold for it, so it must classify `Behind` and buy exactly
    /// one hash-addressed upgrade fetch. No operator action, no eviction script.
    #[tokio::test]
    async fn an_unmarked_archive_doc_at_the_declared_head_is_not_at_head() {
        let archive = FakeArchive::with_unmarked_shell("sha256-h2", "<html>h1-era</html>");
        let store = WarmShellStore::new(Some(archive.clone()));

        let (class, warm, declared) = store.lookup_with_declared("landing", "index.html").await;
        assert_eq!(
            class,
            WarmClass::Behind,
            "an unmarked doc AT the declared head is not proof of the head"
        );
        assert_eq!(declared.as_deref(), Some("sha256-h2"));
        assert!(!warm.expect("bytes are still servable").head_bound);

        let fetches = Arc::new(AtomicUsize::new(0));
        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(fetches.clone(), Some(("<html>h2-era</html>", "text/html"))),
        )
        .await;
        assert!(matches!(outcome, ShellOutcome::Fresh(_)));
        assert_eq!(fetches.load(Ordering::SeqCst), 1);

        // …and the restocked copy IS head-bound, so it converges: no further
        // fetch, and the bytes are the new era's.
        let (class, warm, _) = store.lookup_with_declared("landing", "index.html").await;
        assert_eq!(class, WarmClass::AtHead);
        let shell = warm.expect("restocked");
        assert!(shell.head_bound);
        assert!(shell.html().contains("h2-era"));
    }

    /// The live alpha shape (Loki, 2026-09-04): `head:""`, `x-projection-ready:
    /// false` — `resolve_blob_hash` answers `None`, so a hot shell stocked from
    /// a slug-addressed fetch was classified `AtHead` on the `declared.is_none()`
    /// arm and served forever with no upstream re-check. An UNKNOWN head is not
    /// a confirmed one: it is `Behind`, and it re-checks.
    #[tokio::test]
    async fn an_unknown_declared_head_is_behind_never_at_head() {
        let archive = Arc::new(FakeArchive::default()); // declares nothing
        let store = WarmShellStore::new(Some(archive.clone()));
        store.hot_put(
            "landing",
            "index.html",
            WarmShell {
                blob_hash: String::new(),
                content_type: "text/html".into(),
                bytes: b"<html>stale-era</html>".to_vec(),
                head_bound: false,
            },
        );

        let (class, warm, declared) = store.lookup_with_declared("landing", "index.html").await;
        assert!(declared.is_none(), "the projection declares no head");
        assert_eq!(
            class,
            WarmClass::Behind,
            "with no declared head there is nothing to be AT — serve warm, but re-check"
        );
        assert!(warm.is_some(), "the bytes are still servable meanwhile");
        assert_eq!(
            decide_shell_serve(class, true, true),
            ShellPlan::UpgradeThenWarm
        );
    }

    /// The rate limit that makes the previous test affordable: an unknown head
    /// never converges to `AtHead`, so an unguarded upgrade would be an upstream
    /// fetch on EVERY `/` request. One attempt per
    /// [`SHELL_UPGRADE_RETRY_SECS`]; in between, serve warm, fetch-free.
    #[tokio::test]
    async fn an_unknown_head_re_checks_at_most_once_per_retry_interval() {
        let archive = Arc::new(FakeArchive::default());
        let store = WarmShellStore::new(Some(archive.clone()));
        store.hot_put(
            "landing",
            "index.html",
            WarmShell {
                blob_hash: String::new(),
                content_type: "text/html".into(),
                bytes: b"<html>stale-era</html>".to_vec(),
                head_bound: false,
            },
        );

        // First request: due (never attempted) → one upgrade read, which lands.
        let fetches = Arc::new(AtomicUsize::new(0));
        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(
                fetches.clone(),
                Some(("<html>fresh-era</html>", "text/html")),
            ),
        )
        .await;
        assert!(matches!(outcome, ShellOutcome::Fresh(_)));
        assert_eq!(fetches.load(Ordering::SeqCst), 1);

        // The upgrade replaced the hot bytes even with no head to key them by.
        let (_, warm) = store.lookup("landing", "index.html").await;
        assert!(warm.expect("warm").html().contains("fresh-era"));

        // Second request, inside the interval: warm, and the closure is never
        // called — no fetch-per-request storm while the head stays unknown.
        let fetches2 = Arc::new(AtomicUsize::new(0));
        let again = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(fetches2.clone(), Some(("<html>never</html>", "text/html"))),
        )
        .await;
        assert!(matches!(again, ShellOutcome::Warm(_)));
        assert_eq!(fetches2.load(Ordering::SeqCst), 0);
    }

    /// Wherever the slug's archive is cleared, the HOT copy must go too — it is
    /// consulted first, so an un-evicted hot entry outlives every Mongo purge
    /// and every admin route. Nothing evicted it until 2026-09-04.
    #[tokio::test]
    async fn evicting_a_slug_drops_its_hot_shell() {
        let archive = FakeArchive::with_shell("sha256-a", "<html>old</html>");
        let store = WarmShellStore::new(Some(archive.clone()));
        store
            .hydrate(&[("landing".to_string(), "index.html".to_string())])
            .await;
        assert!(store.hot_get("landing", "index.html").is_some());

        store.evict("other-app");
        assert!(
            store.hot_get("landing", "index.html").is_some(),
            "eviction is per-slug, not a global flush"
        );

        store.evict("landing");
        assert!(store.hot_get("landing", "index.html").is_none());
    }

    /// BLOCKER-1 (round 2). With no declared head the upstream read is
    /// slug-addressed, so there is no content address to archive under — but
    /// the bytes must still REPLACE the hot entry, or a due request serves the
    /// fresh page once while the hot map keeps the stale one for the rest of
    /// the interval and convergence never sticks. `stock_unbound` is the one
    /// home for that (the dispatch path used to only log here).
    #[tokio::test]
    async fn unbound_bytes_replace_the_stale_hot_shell_for_the_whole_interval() {
        let archive = Arc::new(FakeArchive::default()); // declares no head
        let store = WarmShellStore::new(Some(archive.clone()));
        store.hot_put(
            "landing",
            "index.html",
            WarmShell {
                blob_hash: String::new(),
                content_type: "text/html".into(),
                bytes: b"<html>stale-era</html>".to_vec(),
                head_bound: false,
            },
        );

        store
            .stock_unbound(
                "landing",
                "index.html",
                "text/html",
                b"<html>fresh-era</html>".to_vec(),
            )
            .await;

        for round in 0..3 {
            let outcome = resolve_shell(
                &store,
                "landing",
                "index.html",
                true,
                counting_fetch(Arc::new(AtomicUsize::new(0)), None),
            )
            .await;
            let html = match outcome {
                ShellOutcome::Warm(shell) => shell.html(),
                other => panic!("round {round}: expected a warm shell, got {other:?}"),
            };
            assert!(
                html.contains("fresh-era") && !html.contains("stale-era"),
                "round {round}: the slug-resolved bytes must stick, not be re-served stale"
            );
        }
    }

    /// BLOCKER-2 (round 2). The EPR dispatch's upgrade must fall back to the
    /// bytes it already holds, never hand the visitor the upstream's 404/503/
    /// timeout. Holds for an UNKNOWN head too — the live alpha shape, where the
    /// upgrade read is slug-addressed and can legitimately miss.
    #[tokio::test]
    async fn a_failed_upgrade_with_an_unknown_head_serves_warm_never_unavailable() {
        let archive = Arc::new(FakeArchive::default()); // declares no head
        let store = WarmShellStore::new(Some(archive));
        store.hot_put(
            "landing",
            "index.html",
            WarmShell {
                blob_hash: String::new(),
                content_type: "text/html".into(),
                bytes: b"<html>held</html>".to_vec(),
                head_bound: false,
            },
        );

        let fetches = Arc::new(AtomicUsize::new(0));
        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(fetches.clone(), None), // 404 / 503 / timeout
        )
        .await;

        match outcome {
            ShellOutcome::Warm(shell) => assert!(shell.html().contains("held")),
            other => panic!("a failed upgrade must never shed: {other:?}"),
        }
        assert_eq!(fetches.load(Ordering::SeqCst), 1);
    }

    /// The claim is ATOMIC: `try_claim_upgrade` checks and records under ONE
    /// write lock, so concurrent `/` requests cannot all decide the upgrade is
    /// due and launch it together (a read-then-write pair did exactly that).
    #[tokio::test]
    async fn exactly_one_request_per_interval_claims_the_upgrade() {
        let archive = FakeArchive::with_shell("sha256-a", "<html>old</html>");
        let store = WarmShellStore::new(Some(archive.clone()));
        archive.declare("sha256-b");

        let mut upgrades = 0;
        for _ in 0..5 {
            if plan_shell_serve(&store, "landing", "index.html", true)
                .await
                .plan
                == ShellPlan::UpgradeThenWarm
            {
                upgrades += 1;
            }
        }
        assert_eq!(
            upgrades, 1,
            "one claim per SHELL_UPGRADE_RETRY_SECS, decided in a single lock"
        );
    }

    /// Provenance is derived from the BYTES, not from the fact that a fetch
    /// happened. Slug-resolved bytes were never confirmed against a head, so
    /// they must say so rather than borrow the confirmed-head silence.
    #[test]
    fn fresh_bytes_declare_whether_a_head_confirmed_them() {
        assert_eq!(ShellProvenance::for_fresh(true).header_value(), None);
        assert_eq!(
            ShellProvenance::for_fresh(false).header_value(),
            Some("slug-resolved")
        );
        assert_eq!(
            ShellProvenance::LastReconciled.header_value(),
            Some("last-reconciled")
        );
    }

    #[tokio::test]
    async fn slug_resolved_fresh_bytes_carry_the_slug_resolved_provenance() {
        let archive = Arc::new(FakeArchive::default()); // declares no head
        let store = WarmShellStore::new(Some(archive));
        store.hot_put(
            "landing",
            "index.html",
            WarmShell {
                blob_hash: String::new(),
                content_type: "text/html".into(),
                bytes: b"<html>held</html>".to_vec(),
                head_bound: false,
            },
        );

        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(
                Arc::new(AtomicUsize::new(0)),
                Some(("<html>slug-read</html>", "text/html")),
            ),
        )
        .await;

        assert_eq!(
            outcome.provenance(),
            Some(ShellProvenance::SlugResolved),
            "an unconfirmed read must not present as a confirmed head"
        );
    }

    /// The claim map is bounded: converging to `AtHead` releases the slot, so a
    /// long-lived pod does not accumulate one entry per app forever.
    #[tokio::test]
    async fn converging_to_at_head_releases_the_claim_slot() {
        let archive = FakeArchive::with_shell("sha256-a", "<html>old</html>");
        let store = WarmShellStore::new(Some(archive.clone()));
        store
            .hydrate(&[("landing".to_string(), "index.html".to_string())])
            .await;
        archive.declare("sha256-b");

        let _ = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(
                Arc::new(AtomicUsize::new(0)),
                Some(("<html>new</html>", "text/html")),
            ),
        )
        .await;
        assert_eq!(store.upgrade_attempt_count(), 1, "the claim was taken");

        // The next lookup finds the upgraded head and lets the slot go.
        let (class, _, _) = store.lookup_with_declared("landing", "index.html").await;
        assert_eq!(class, WarmClass::AtHead);
        assert_eq!(
            store.upgrade_attempt_count(),
            0,
            "a converged upgrade must not leave its claim behind"
        );
    }

    // ── (2) hot path, upstream unavailable, warm cache ───────────────────────

    #[tokio::test]
    async fn upstream_unavailable_with_a_warm_shell_serves_without_a_fetch() {
        let archive = FakeArchive::with_shell("sha256-a", "<html><app-root></app-root></html>");
        let store = WarmShellStore::new(Some(archive));
        let fetches = Arc::new(AtomicUsize::new(0));

        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            false, // breaker open / upstream catching up
            counting_fetch(fetches.clone(), None),
        )
        .await;

        assert!(matches!(outcome, ShellOutcome::Warm(_)));
        assert_eq!(fetches.load(Ordering::SeqCst), 0);
    }

    // ── (3) hot path, upstream unavailable, cold cache ───────────────────────

    #[tokio::test]
    async fn upstream_unavailable_with_a_cold_cache_sheds_immediately() {
        let store = WarmShellStore::new(Some(Arc::new(FakeArchive::default())));
        let fetches = Arc::new(AtomicUsize::new(0));

        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            false,
            counting_fetch(fetches.clone(), Some(("<html/>", "text/html"))),
        )
        .await;

        assert!(
            matches!(outcome, ShellOutcome::Unavailable),
            "cold cache + unavailable upstream must shed, never stall on a doomed fetch"
        );
        assert_eq!(
            fetches.load(Ordering::SeqCst),
            0,
            "shedding must be immediate — no upstream attempt at all"
        );
    }

    // ── (4) reconcile upgrade path ───────────────────────────────────────────

    #[tokio::test]
    async fn once_the_upstream_answers_the_shell_upgrades_to_the_declared_head() {
        let archive = FakeArchive::with_shell("sha256-a", "<html>old</html>");
        let store = WarmShellStore::new(Some(archive.clone()));
        store
            .hydrate(&[("landing".to_string(), "index.html".to_string())])
            .await;

        // The reconcile loop lands a new declared head; no bytes for it yet.
        archive.declare("sha256-b");

        let fetches = Arc::new(AtomicUsize::new(0));
        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(fetches.clone(), Some(("<html>new</html>", "text/html"))),
        )
        .await;

        match outcome {
            ShellOutcome::Fresh(shell) => {
                assert!(shell.html().contains("new"));
                assert_eq!(shell.blob_hash, "sha256-b");
            }
            other => panic!("expected an upgraded (fresh) shell, got {other:?}"),
        }
        assert_eq!(fetches.load(Ordering::SeqCst), 1);

        // …and the upgraded head is now warm: the NEXT request is fetch-free.
        let fetches2 = Arc::new(AtomicUsize::new(0));
        let again = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(fetches2.clone(), None),
        )
        .await;
        assert!(matches!(again, ShellOutcome::Warm(_)));
        assert_eq!(fetches2.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_failed_upgrade_falls_back_to_the_one_behind_shell_never_a_shed() {
        let archive = FakeArchive::with_shell("sha256-a", "<html>old</html>");
        let store = WarmShellStore::new(Some(archive.clone()));
        archive.declare("sha256-b");

        let fetches = Arc::new(AtomicUsize::new(0));
        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(fetches.clone(), None), // upgrade fetch fails
        )
        .await;

        match outcome {
            ShellOutcome::Warm(shell) => assert!(shell.html().contains("old")),
            other => panic!("expected the one-behind shell, got {other:?}"),
        }
        assert_eq!(fetches.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn an_upgrade_fetch_gets_the_tight_budget_a_cold_fetch_gets_the_full_one() {
        let archive = FakeArchive::with_shell("sha256-a", "<html>old</html>");
        let store = WarmShellStore::new(Some(archive.clone()));
        archive.declare("sha256-b");

        let seen = Arc::new(Mutex::new(Vec::<FetchBudget>::new()));
        let seen_c = seen.clone();
        let _ = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            move |budget, _declared| {
                seen_c.lock().unwrap().push(budget);
                Box::pin(async { None })
            },
        )
        .await;

        let cold = WarmShellStore::new(Some(Arc::new(FakeArchive::default())));
        let seen_c = seen.clone();
        let _ = resolve_shell(
            &cold,
            "landing",
            "index.html",
            true,
            move |budget, _declared| {
                seen_c.lock().unwrap().push(budget);
                Box::pin(async { None })
            },
        )
        .await;

        assert_eq!(
            *seen.lock().unwrap(),
            vec![FetchBudget::Upgrade, FetchBudget::Full],
            "holding a good answer must never cost the full upstream wall"
        );
    }

    // ── mongo-unavailable degrade ────────────────────────────────────────────

    #[tokio::test]
    async fn without_an_archive_the_store_is_inert_and_behaviour_is_todays() {
        let store = WarmShellStore::new(None);
        let fetches = Arc::new(AtomicUsize::new(0));

        let outcome = resolve_shell(
            &store,
            "landing",
            "index.html",
            true,
            counting_fetch(fetches.clone(), Some(("<html/>", "text/html"))),
        )
        .await;

        assert!(matches!(outcome, ShellOutcome::Fresh(_)));
        assert_eq!(
            fetches.load(Ordering::SeqCst),
            1,
            "no Mongo archive → exactly today's fetch-per-request behaviour"
        );
        assert_eq!(
            store
                .hydrate(&[("landing".to_string(), "index.html".to_string())])
                .await,
            0
        );
    }

    // ── the pure decision matrix ─────────────────────────────────────────────

    #[test]
    fn shell_serve_decision_matrix_is_cache_first() {
        use ShellPlan::*;
        use WarmClass::*;
        let cases = [
            // A warm shell at the declared head NEVER pays an upstream fetch,
            // healthy upstream or not — this is the cure for the born-red.
            (AtHead, true, true, ServeWarm),
            (AtHead, false, true, ServeWarm),
            // One behind: upgrade when we can, serve what we hold when we can't.
            (Behind, true, true, UpgradeThenWarm),
            (Behind, false, true, ServeWarm),
            // …and never more than once per SHELL_UPGRADE_RETRY_SECS: an
            // upgrade that cannot converge (an unknown head never does) must
            // not become a fetch on EVERY `/` request.
            (Behind, true, false, ServeWarm),
            // Cold: fetch when the upstream can answer, shed honestly when not.
            (Cold, true, true, Fetch),
            (Cold, false, true, Shed),
        ];
        for (class, upstream, due, expected) in cases {
            assert_eq!(
                decide_shell_serve(class, upstream, due),
                expected,
                "class={class:?} upstream_available={upstream} upgrade_due={due}"
            );
        }
    }
}
