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
            })
    }

    async fn load_latest(&self, slug: &str, file_path: &str) -> Option<ArchivedShell> {
        self.latest_file(slug, file_path)
            .await
            .map(|f| ArchivedShell {
                blob_hash: f.blob_hash,
                content_type: f.content_type,
                bytes: f.data,
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
        self.put(slug, file_path, blob_hash, content_type, bytes)
            .await;
    }
}

/// A shell document ready to serve, with the head it was stocked under.
#[derive(Debug, Clone)]
pub struct WarmShell {
    pub blob_hash: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
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
}

impl ShellProvenance {
    /// The `x-elohim-bundle` header value, or `None` when the serve confirmed
    /// the declared head this request (no marker).
    pub fn header_value(&self) -> Option<&'static str> {
        match self {
            ShellProvenance::LastReconciled => Some("last-reconciled"),
            ShellProvenance::DeclaredHead => None,
        }
    }
}

/// How the warm store's answer relates to the locally-declared head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarmClass {
    /// We hold bytes for the head the local projection declares.
    AtHead,
    /// We hold bytes, but the projection has since declared a different head.
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
pub fn decide_shell_serve(class: WarmClass, upstream_available: bool) -> ShellPlan {
    match (class, upstream_available) {
        (WarmClass::AtHead, _) => ShellPlan::ServeWarm,
        (WarmClass::Behind, true) => ShellPlan::UpgradeThenWarm,
        (WarmClass::Behind, false) => ShellPlan::ServeWarm,
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
            ShellOutcome::Fresh(_) => Some(ShellProvenance::DeclaredHead),
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
}

impl WarmShellStore {
    pub fn new(archive: Option<Arc<dyn ShellArchive>>) -> Self {
        Self {
            hot: RwLock::new(HashMap::new()),
            archive,
        }
    }

    /// A store with no persistent archive — inert, so callers keep exactly
    /// today's fetch-per-request behaviour.
    pub fn inert() -> Self {
        Self::new(None)
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
            if let Some(hot) = self.hot_get(slug, file_path) {
                if hot.blob_hash == head {
                    return (WarmClass::AtHead, Some(hot), declared);
                }
            }
            if let Some(a) = archive.load(slug, file_path, head).await {
                let shell = WarmShell {
                    blob_hash: a.blob_hash,
                    content_type: a.content_type,
                    bytes: a.bytes,
                };
                self.hot_put(slug, file_path, shell.clone());
                return (WarmClass::AtHead, Some(shell), declared);
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
                };
                self.hot_put(slug, file_path, shell.clone());
                shell
            }),
        };
        match behind {
            // No declared head to compare against and bytes in hand: treat it as
            // the head we know of, so a projection that never names a hash still
            // serves fetch-free.
            Some(shell) if declared.is_none() => (WarmClass::AtHead, Some(shell), None),
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
            },
        );
        if let Some(archive) = self.archive.as_ref() {
            archive
                .store(slug, file_path, blob_hash, content_type, bytes)
                .await;
        }
    }
}

/// Resolve the shell for one request: cache-first, shed-fast, upgrade-when-able.
///
/// `upstream_available` is the caller's honest read of whether the storage peer
/// can answer AT ALL right now (breaker closed + a storage URL configured) — it
/// is what turns a doomed 10s fetch into an immediate honest shed.
///
/// `fetch` performs the upstream read, honouring the [`FetchBudget`] it is
/// handed; `None` means the read failed (any cause).
pub async fn resolve_shell<F, Fut>(
    store: &WarmShellStore,
    slug: &str,
    entry_file: &str,
    upstream_available: bool,
    fetch: F,
) -> ShellOutcome
where
    F: FnOnce(FetchBudget) -> Fut,
    Fut: std::future::Future<Output = Option<FetchedShell>>,
{
    let (class, warm, declared) = store.lookup_with_declared(slug, entry_file).await;
    match decide_shell_serve(class, upstream_available) {
        ShellPlan::ServeWarm => match warm {
            Some(shell) => ShellOutcome::Warm(shell),
            // Unreachable by construction (ServeWarm only follows a hit); an
            // honest shed beats an unwrap on the hot path.
            None => ShellOutcome::Unavailable,
        },
        ShellPlan::UpgradeThenWarm => match fetch(FetchBudget::Upgrade).await {
            Some(fresh) => stock_and_return(store, slug, entry_file, declared, fresh).await,
            // The upgrade could not land — keep serving the one-behind shell.
            // Strictly better than today, which shed to a bundle fallback that
            // then paid the SAME doomed fetch again.
            None => match warm {
                Some(shell) => ShellOutcome::Warm(shell),
                None => ShellOutcome::Unavailable,
            },
        },
        ShellPlan::Fetch => match fetch(FetchBudget::Full).await {
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
    // The upstream `/apps/{slug}/{entry}` surface returns no head marker, so the
    // stocking key is the head the local projection declared AT LOOKUP TIME —
    // never re-resolved here, or a projection advance during the fetch would
    // relabel old-era bytes as the new head. With no declared head there is no
    // content-addressed key to stock under — serve the bytes, skip the archive
    // write rather than invent an address.
    let shell = WarmShell {
        blob_hash: declared.clone().unwrap_or_default(),
        content_type: fresh.content_type.clone(),
        bytes: fresh.bytes.clone(),
    };
    match declared {
        Some(hash) => {
            store
                .stock(slug, entry_file, &hash, &fresh.content_type, fresh.bytes)
                .await;
        }
        None => {
            store.hot_put(slug, entry_file, shell.clone());
        }
    }
    ShellOutcome::Fresh(shell)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    /// One archived file in the fake, in stock order — last is newest.
    struct StockedFile {
        slug: String,
        file_path: String,
        blob_hash: String,
        bytes: Vec<u8>,
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
    ) -> impl FnOnce(FetchBudget) -> BoxedFetch {
        move |_budget| {
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
        let outcome = resolve_shell(&store, "landing", "index.html", true, move |_budget| {
            Box::pin(async move {
                // The declared head advances while the fetch is in flight.
                racing.declare("sha256-h2");
                Some(FetchedShell {
                    bytes: b"<html>h1-era</html>".to_vec(),
                    content_type: "text/html".to_string(),
                })
            }) as BoxedFetch
        })
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
        let _ = resolve_shell(&store, "landing", "index.html", true, move |budget| {
            seen_c.lock().unwrap().push(budget);
            Box::pin(async { None })
        })
        .await;

        let cold = WarmShellStore::new(Some(Arc::new(FakeArchive::default())));
        let seen_c = seen.clone();
        let _ = resolve_shell(&cold, "landing", "index.html", true, move |budget| {
            seen_c.lock().unwrap().push(budget);
            Box::pin(async { None })
        })
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
            (AtHead, true, ServeWarm),
            (AtHead, false, ServeWarm),
            // One behind: upgrade when we can, serve what we hold when we can't.
            (Behind, true, UpgradeThenWarm),
            (Behind, false, ServeWarm),
            // Cold: fetch when the upstream can answer, shed honestly when not.
            (Cold, true, Fetch),
            (Cold, false, Shed),
        ];
        for (class, upstream, expected) in cases {
            assert_eq!(
                decide_shell_serve(class, upstream),
                expected,
                "class={class:?} upstream_available={upstream}"
            );
        }
    }
}
