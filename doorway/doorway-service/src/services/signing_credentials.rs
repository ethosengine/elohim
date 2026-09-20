//! Process-lifetime reuse of the doorway's OWN conductor signing credentials.
//!
//! # Why this exists
//!
//! `authorize_signing_credentials` is not a read. It COMMITS a `CapGrant` to
//! the conductor agent's source chain, and nothing ever revokes one. The
//! conductor then re-reads **every** grant of the matching access class on
//! **every** zome call (`DhtStoreRead::valid_cap_grants` — load-all-then-filter,
//! three SQL queries per surviving row plus a 500-hash entry batch). At the
//! 11 619 / 15 768 / 18 516 grants measured on matthew and adam on 2026-09-19
//! that is ~47 000 queries and 4–9 s **per call**, which is what held those two
//! peers' admission permits for their whole 60 s timeout.
//!
//! [`super::zome_caller::connect_endpoint`] used to authorize credentials for
//! every provisioned cell on **every connect**, and the same module records the
//! NXDOMAIN / WebSocket-reset churn that makes connects frequent. Each churn
//! cycle was one permanent `CapGrant` per cell. This cache removes that: the
//! doorway mints at most **one grant per cell per process**, plus at most one
//! bounded heal.
//!
//! # The discipline, ported from `elohim-storage`'s closed-chain fence
//!
//! `elohim-storage/src/closed_chain_fence.rs` solved the same problem for the
//! peer: decide before you author, reuse by default, heal exactly once. What is
//! ported here is that DISCIPLINE, not its file vault — this cache keeps
//! **nothing at rest**. Credentials live in memory for the lifetime of the
//! process and die with it. A doorway restart therefore re-mints once per cell,
//! which is acceptable and deliberate: restarts are rare and bounded, whereas
//! reconnect churn is unbounded and was the actual growth.
//!
//! # Concurrency: the mint is serialized PER CELL
//!
//! `decide` and `add_credentials` straddle the `authorize_signing_credentials`
//! round trip. Two `connect_endpoint` runs for one endpoint overlap in
//! practice — a primary connect racing a half-open re-probe, startup fan-out,
//! or two provisions of the same hosted app — and without serialization both
//! would observe `Mint` and both would author a `CapGrant`.
//!
//! [`SigningCredentialCache::lease`] takes a per-(conductor, app, role) async
//! lock ([`crate::keyed_lock`]) that the caller holds across `decide` → mint →
//! `add_credentials`. Distinct cells and distinct conductors never contend, so
//! one slow conductor cannot stall another's connect. The wait is bounded by
//! the same per-conductor-call deadline every other step uses; on expiry the
//! caller proceeds UNSERIALIZED, which is exactly the pre-lock behaviour.
//!
//! # The heal, and why it is bounded
//!
//! A cached credential can be perfectly valid while the grant it names has
//! vanished from the chain — a conductor reinstalled, or a database restored
//! from an older snapshot under the same `CellId`. Nothing at connect time can
//! see that; the first zome call is where the truth appears, as an
//! unauthorized-shaped error. So the heal is driven from the error path
//! ([`looks_like_a_rejected_cap_grant`]), and it is bounded to **once per
//! (conductor, role) per process**: after one re-mint, a second rejection is
//! surfaced to the caller unchanged. A repair is not a retry loop — an
//! unbounded heal would be the per-connect minting we are removing, wearing a
//! different hat.

use holochain_client::{AgentSigner, CellId, ClientAgentSigner};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use crate::keyed_lock::{KeyedGuard, KeyedLock};

/// Error-message fragments that say the conductor no longer honours a grant we
/// are signing with.
///
/// Deliberately a COPY of `elohim-storage`'s `CAP_GRANT_REJECTION_MARKERS`
/// (`hc_client.rs`), not a dependency on it: the doorway must not take a build
/// dependency on the peer crate, and the two lists answer the same question
/// about the same conductor error strings. Keep them in step by hand — they are
/// six lowercase substrings, and drift would only ever cost one extra
/// (bounded) mint or one missed heal.
pub const CAP_GRANT_REJECTION_MARKERS: &[&str] = &[
    "unauthorized",
    "capability",
    "cap grant",
    "capgrant",
    "cap secret",
    "capsecret",
];

/// Does this zome-call error say our cached grant is no longer honoured?
pub fn looks_like_a_rejected_cap_grant(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    CAP_GRANT_REJECTION_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
}

/// What the cache will do about one cell's signing credentials — the decision,
/// separated from its enactment so it can be asserted without a conductor.
///
/// The same split `closed_chain_fence::decide` makes, and for the same reason:
/// the expensive, irreversible half (a chain write) must be testable as a
/// prediction before anything dials a conductor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialDecision {
    /// Credentials for this cell are already in the process cache.
    /// **Nothing is authored on the conductor.**
    Reuse,
    /// No cached credential for this cell, or a heal discarded it.
    /// Authorize ONCE, then cache.
    Mint,
}

/// The whole policy, as a pure function of two facts.
///
/// `cached` — the process already holds credentials for this cell.
/// `marked_for_remint` — a zome call was refused in a cap-grant-shaped way and
/// the (bounded) heal asked for exactly one replacement.
pub fn decide_credential(cached: bool, marked_for_remint: bool) -> CredentialDecision {
    if cached && !marked_for_remint {
        CredentialDecision::Reuse
    } else {
        CredentialDecision::Mint
    }
}

/// The key a conductor's credentials are cached under.
///
/// Both halves are load-bearing. A grant is issued by ONE conductor on ONE
/// cell and only that conductor will verify the resulting signature (the
/// per-conductor credential crux in [`super::zome_caller`]), and a hosted
/// human's app is a DIFFERENT installed app on the same conductor with its own
/// cells — so a cache keyed on the conductor alone would hand one app's
/// credentials to another's cells.
pub fn endpoint_key(admin_addr: &str, installed_app_id: &str) -> String {
    format!(
        "{}|{installed_app_id}",
        canonical_conductor_addr(admin_addr)
    )
}

/// One conductor, one spelling.
///
/// The same admin interface reaches this cache written several ways — with or
/// without a `ws://`/`wss://` scheme (`ConductorEndpoint::new` strips it,
/// `ConductorEndpoint::from_app_url` derives it, config supplies it raw), with
/// a trailing slash, or with the host in a different case. Two spellings that
/// miss each other mint a second `CapGrant` for a cell that already has one:
/// it fails SAFE (an extra grant, never a wrong one), but it is avoidable, and
/// the whole point of this module is that avoidable mints are the growth.
///
/// Normalised: surrounding whitespace, the websocket scheme, anything from the
/// first `/`, `?` or `#`, a trailing slash, and ASCII case. Ports are left
/// exactly as written and none is invented: `resolve_host_port` dials through
/// `tokio::net::lookup_host`, which REQUIRES `host:port`, so a portless address
/// cannot reach a conductor at all — there is no crate-wide default admin port
/// to supply (the convention is per-endpoint, app port minus one). Inventing
/// one here would normalise an address that can never occur in a working
/// configuration.
fn canonical_conductor_addr(admin_addr: &str) -> String {
    let trimmed = admin_addr.trim();
    let no_scheme = trimmed
        .strip_prefix("wss://")
        .or_else(|| trimmed.strip_prefix("ws://"))
        .unwrap_or(trimmed);
    let authority = no_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(no_scheme)
        .trim_end_matches('/');
    authority.to_ascii_lowercase()
}

fn role_key(endpoint_key: &str, role_name: &str) -> String {
    format!("{endpoint_key}|{role_name}")
}

/// Per-conductor signing credentials, held for the life of the process.
///
/// Constructed with nothing, so every test drives a real one rather than a mock.
#[derive(Debug, Default)]
pub struct SigningCredentialCache {
    /// One async lock per `role_key`, so `decide` → mint → `add_credentials`
    /// cannot interleave with itself for a cell. See the module header.
    leases: KeyedLock,
    /// `endpoint_key` → the signer holding that conductor's per-cell credentials.
    /// `ClientAgentSigner` is internally `Arc<RwLock<HashMap<CellId, _>>>`, so
    /// one signer is safely shared across every connection to that endpoint.
    signers: Mutex<HashMap<String, Arc<ClientAgentSigner>>>,
    /// `role_key`s whose credential the next connect must replace (the heal).
    remint: Mutex<HashSet<String>>,
    /// `role_key`s already healed once in this process. Not persisted, and
    /// never cleared: the bound is per-process on purpose, so a genuine repair
    /// survives a restart while a rejection LOOP cannot.
    healed: Mutex<HashSet<String>>,
}

impl SigningCredentialCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The signer for one conductor+app, created on first use.
    ///
    /// Returns the SAME `Arc` for the same key for the life of the process —
    /// that identity is what makes a reconnect free.
    pub fn signer_for(&self, endpoint_key: &str) -> Arc<ClientAgentSigner> {
        let mut signers = self.signers.lock().unwrap_or_else(|e| e.into_inner());
        Arc::clone(
            signers
                .entry(endpoint_key.to_string())
                .or_insert_with(|| Arc::new(ClientAgentSigner::default())),
        )
    }

    /// Take the per-cell lease that makes `decide` → mint → `add_credentials`
    /// atomic for THIS (conductor, app, role). Hold the returned guard across
    /// all three.
    ///
    /// `None` means the previous holder outlived `deadline`; the caller then
    /// proceeds unserialized — the pre-lock behaviour. Distinct cells and
    /// distinct conductors never wait on each other.
    pub async fn lease(
        &self,
        endpoint_key: &str,
        role_name: &str,
        deadline: Duration,
    ) -> Option<KeyedGuard<'_>> {
        self.leases
            .acquire_within(&role_key(endpoint_key, role_name), deadline)
            .await
    }

    /// How many cell leases are live. Diagnostics and tests — must return to
    /// zero once every in-flight connect finishes.
    pub fn live_leases(&self) -> usize {
        self.leases.live_keys()
    }

    /// Reuse or mint, for ONE cell. Consumes any pending heal mark, so a heal
    /// buys exactly one replacement.
    ///
    /// Only meaningful under the matching [`Self::lease`]: without it, a
    /// concurrent connect to the same cell can read the same `Mint`.
    pub fn decide(
        &self,
        signer: &ClientAgentSigner,
        endpoint_key: &str,
        role_name: &str,
        cell: &CellId,
    ) -> CredentialDecision {
        let marked = self
            .remint
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&role_key(endpoint_key, role_name));
        decide_credential(signer.get_provenance(cell).is_some(), marked)
    }

    /// A zome call on `role_name` was refused in a way that says the conductor
    /// no longer honours our cached grant. Ask for exactly ONE replacement.
    ///
    /// Returns `true` when the heal was accepted (the caller should drop the
    /// socket so the next call reconnects and re-mints), `false` when this
    /// (conductor, role) has already been healed in this process — in which
    /// case the fault is not a stale credential and the error must simply
    /// surface. Never loops.
    pub fn mark_for_remint(&self, endpoint_key: &str, role_name: &str) -> bool {
        let key = role_key(endpoint_key, role_name);
        {
            let mut healed = self.healed.lock().unwrap_or_else(|e| e.into_inner());
            if !healed.insert(key.clone()) {
                return false;
            }
        }
        self.remint
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key);
        true
    }

    /// Whether a heal is pending for this (conductor, role). Test/diagnostic
    /// read — [`Self::decide`] is what consumes it.
    pub fn remint_pending(&self, endpoint_key: &str, role_name: &str) -> bool {
        self.remint
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&role_key(endpoint_key, role_name))
    }
}

/// The process-wide cache. Reuse is only reuse if everyone shares one.
pub fn cache() -> &'static SigningCredentialCache {
    static CACHE: OnceLock<SigningCredentialCache> = OnceLock::new();
    CACHE.get_or_init(SigningCredentialCache::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(seed: u8) -> CellId {
        use holo_hash::DnaHash;
        use holochain_client::AgentPubKey;
        CellId::new(
            DnaHash::from_raw_36(vec![seed; 36]),
            AgentPubKey::from_raw_36(vec![seed.wrapping_add(1); 36]),
        )
    }

    // ── the decision, as a pure function ────────────────────────────────────

    /// THE INVARIANT. A cached credential is reused, and reuse authors nothing.
    /// This is the whole point: the second connect to a conductor must not add
    /// a row to the table the conductor re-reads on every zome call.
    #[test]
    fn a_cached_credential_is_reused_and_authors_nothing() {
        assert_eq!(decide_credential(true, false), CredentialDecision::Reuse);
    }

    #[test]
    fn a_missing_credential_is_minted_once() {
        assert_eq!(decide_credential(false, false), CredentialDecision::Mint);
    }

    /// The heal overrides reuse — but only because something already decided,
    /// once, that the cached grant is gone.
    #[test]
    fn a_heal_mark_overrides_reuse() {
        assert_eq!(decide_credential(true, true), CredentialDecision::Mint);
        assert_eq!(decide_credential(false, true), CredentialDecision::Mint);
    }

    // ── the cache ───────────────────────────────────────────────────────────

    /// A reconnect to the same conductor+app gets the SAME signer, which is
    /// what makes the second connect free. A different conductor, or the same
    /// conductor with a different installed app, must NOT borrow it: a grant is
    /// issued by one conductor on one cell and is worthless anywhere else.
    #[test]
    fn one_signer_per_conductor_and_app_reused_across_connects() {
        let cache = SigningCredentialCache::new();
        let a1 = cache.signer_for(&endpoint_key("adam:4444", "elohim"));
        let a2 = cache.signer_for(&endpoint_key("adam:4444", "elohim"));
        let other_conductor = cache.signer_for(&endpoint_key("matthew:8444", "elohim"));
        let other_app = cache.signer_for(&endpoint_key("adam:4444", "elohim-hosted-jane"));

        assert!(
            Arc::ptr_eq(&a1, &a2),
            "a reconnect must reuse the SAME signer — a fresh one per connect is \
             exactly the per-connect minting this cache removes"
        );
        assert!(
            !Arc::ptr_eq(&a1, &other_conductor),
            "credentials are per-conductor; borrowing across conductors would sign \
             calls the other peer rejects"
        );
        assert!(
            !Arc::ptr_eq(&a1, &other_app),
            "a hosted human's app has its own cells on the same conductor"
        );
    }

    /// Mint-only-missing-cells: an empty signer knows no cell, so every cell of
    /// a first connect is a `Mint`. Nothing else in the cache may turn that
    /// into a second mint for the same cell.
    #[test]
    fn a_first_connect_mints_each_cell_and_nothing_else() {
        let cache = SigningCredentialCache::new();
        let key = endpoint_key("adam:4444", "elohim");
        let signer = cache.signer_for(&key);

        for (role, c) in [("lamad", cell(1)), ("imagodei", cell(2))] {
            assert_eq!(
                cache.decide(&signer, &key, role, &c),
                CredentialDecision::Mint,
                "an unknown cell must be authorized once"
            );
        }
    }

    /// HEAL, BOUNDED. The first cap-grant-shaped rejection buys exactly one
    /// replacement; the second must be refused so the error surfaces instead of
    /// minting again. This is the difference between a repair and the retry
    /// loop that would re-create the growth.
    #[test]
    fn a_role_is_healed_once_then_the_error_surfaces() {
        let cache = SigningCredentialCache::new();
        let key = endpoint_key("adam:4444", "elohim");

        assert!(
            cache.mark_for_remint(&key, "infrastructure"),
            "the first rejection must be healed"
        );
        assert!(cache.remint_pending(&key, "infrastructure"));

        assert!(
            !cache.mark_for_remint(&key, "infrastructure"),
            "a second rejection on the same role must NOT mint again — the fault \
             is no longer a stale credential"
        );

        // …and no amount of repetition changes that.
        for _ in 0..10 {
            assert!(!cache.mark_for_remint(&key, "infrastructure"));
        }
    }

    /// The heal mark is CONSUMED by the decision it causes: one mark, one mint,
    /// and the cell is reusable again immediately after.
    #[test]
    fn a_heal_mark_is_consumed_by_the_mint_it_causes() {
        let cache = SigningCredentialCache::new();
        let key = endpoint_key("adam:4444", "elohim");
        let signer = cache.signer_for(&key);
        let c = cell(3);

        assert!(cache.mark_for_remint(&key, "imagodei"));
        assert_eq!(
            cache.decide(&signer, &key, "imagodei", &c),
            CredentialDecision::Mint
        );
        assert!(
            !cache.remint_pending(&key, "imagodei"),
            "the mark must be consumed, or every later connect would re-mint"
        );
    }

    /// Healing one role must not disturb another role's credential — the heal
    /// is per-cell, not per-conductor.
    #[test]
    fn a_heal_is_scoped_to_one_role() {
        let cache = SigningCredentialCache::new();
        let key = endpoint_key("adam:4444", "elohim");

        assert!(cache.mark_for_remint(&key, "infrastructure"));
        assert!(!cache.remint_pending(&key, "imagodei"));
        assert!(
            cache.mark_for_remint(&key, "imagodei"),
            "a different role has its own one-heal budget"
        );
    }

    // ── concurrency: the mint is serialized PER CELL ────────────────────────
    //
    // `decide` … authorize … `add_credentials` straddles a real `.await`. These
    // use genuine concurrency (`tokio::spawn`) and a stub that PARKS inside the
    // authorize window on a `Notify`, so both tasks are provably in flight at
    // once — a sequential `for` loop would assert the invariant away rather
    // than prove it.

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::sync::Notify;

    const LONG: Duration = Duration::from_secs(30);

    fn fake_credentials(seed: u8) -> holochain_client::SigningCredentials {
        use holochain_client::AgentPubKey;
        use holochain_zome_types::prelude::CapSecret;
        let cap_secret: CapSecret = [seed; 64]
            .as_slice()
            .try_into()
            .expect("64-byte cap secret");
        holochain_client::SigningCredentials {
            signing_agent_key: AgentPubKey::from_raw_36(vec![seed; 36]),
            keypair: ed25519_dalek::SigningKey::from_bytes(&[seed.max(1); 32]),
            cap_secret,
        }
    }

    /// Stands in for `authorize_signing_credentials`: counts its calls, signals
    /// that it is INSIDE the window, and parks there until the test releases it.
    #[derive(Clone)]
    struct MintStub {
        calls: Arc<AtomicUsize>,
        entered: Arc<Notify>,
        release: Option<Arc<Notify>>,
        succeeds: bool,
        seed: u8,
    }

    impl MintStub {
        fn new(calls: &Arc<AtomicUsize>, entered: &Arc<Notify>) -> Self {
            Self {
                calls: Arc::clone(calls),
                entered: Arc::clone(entered),
                release: None,
                succeeds: true,
                seed: 1,
            }
        }
        fn parking(mut self, release: &Arc<Notify>) -> Self {
            self.release = Some(Arc::clone(release));
            self
        }
        fn failing(mut self) -> Self {
            self.succeeds = false;
            self
        }
        fn seeded(mut self, seed: u8) -> Self {
            self.seed = seed;
            self
        }
        async fn authorize(&self) -> Option<holochain_client::SigningCredentials> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.entered.notify_one();
            if let Some(release) = &self.release {
                release.notified().await;
            }
            self.succeeds.then(|| fake_credentials(self.seed))
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct CellRun {
        decision: CredentialDecision,
        minted: bool,
        serialized: bool,
    }

    /// One cell of one `connect_endpoint` run, in the exact order the real loop
    /// uses: lease → decide → authorize → add_credentials.
    async fn connect_cell(
        cache: Arc<SigningCredentialCache>,
        signer: Arc<ClientAgentSigner>,
        ep_key: String,
        role: String,
        cell: CellId,
        deadline: Duration,
        stub: MintStub,
    ) -> CellRun {
        let lease = cache.lease(&ep_key, &role, deadline).await;
        let serialized = lease.is_some();
        let decision = cache.decide(&signer, &ep_key, &role, &cell);
        let mut minted = false;
        if decision == CredentialDecision::Mint {
            if let Some(credentials) = stub.authorize().await {
                signer.add_credentials(cell.clone(), credentials);
                minted = true;
            }
        }
        drop(lease);
        CellRun {
            decision,
            minted,
            serialized,
        }
    }

    /// THE RACE, closed. Two connects reach one cell at once; the second must
    /// WAIT, re-decide, and see `Reuse`. Without the lease both would observe
    /// `Mint` and both would author a permanent `CapGrant`.
    #[tokio::test]
    async fn two_concurrent_connects_to_one_cell_mint_exactly_once() {
        let cache = Arc::new(SigningCredentialCache::new());
        let signer = Arc::new(ClientAgentSigner::default());
        let key = endpoint_key("adam:4444", "elohim");
        let calls = Arc::new(AtomicUsize::new(0));
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());

        let a = tokio::spawn(connect_cell(
            Arc::clone(&cache),
            Arc::clone(&signer),
            key.clone(),
            "imagodei".into(),
            cell(1),
            LONG,
            MintStub::new(&calls, &entered).parking(&release),
        ));

        // A is provably INSIDE the authorize window.
        entered.notified().await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(cache.live_leases(), 1);

        let b = tokio::spawn(connect_cell(
            Arc::clone(&cache),
            Arc::clone(&signer),
            key.clone(),
            "imagodei".into(),
            cell(1),
            LONG,
            MintStub::new(&calls, &entered).seeded(2),
        ));

        // B must be parked on the lease, not authorizing.
        tokio::task::yield_now().await;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "B entered the authorize window while A held the cell"
        );

        // `notify_one` STORES a permit when no waiter is registered yet, so the
        // release can never be lost to a scheduling order — unlike
        // `notify_waiters`, which only wakes already-registered waiters.
        release.notify_one();
        let a = a.await.unwrap();
        let b = b.await.unwrap();

        assert_eq!(a.decision, CredentialDecision::Mint);
        assert!(a.minted && a.serialized);
        assert_eq!(
            b.decision,
            CredentialDecision::Reuse,
            "the second connect must observe the first's credential"
        );
        assert!(!b.minted && b.serialized);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "exactly ONE CapGrant may be authored for one cell"
        );
        assert_eq!(cache.live_leases(), 0, "the lease map must be reclaimed");
    }

    /// NO LOST MINT. If the first caller's authorize FAILS, nothing was cached,
    /// so the second must go ahead and mint — the lease must not convert a
    /// failure into a permanent skip.
    #[tokio::test]
    async fn a_failed_mint_lets_the_next_connect_mint() {
        let cache = Arc::new(SigningCredentialCache::new());
        let signer = Arc::new(ClientAgentSigner::default());
        let key = endpoint_key("adam:4444", "elohim");
        let calls = Arc::new(AtomicUsize::new(0));
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());

        let a = tokio::spawn(connect_cell(
            Arc::clone(&cache),
            Arc::clone(&signer),
            key.clone(),
            "imagodei".into(),
            cell(1),
            LONG,
            MintStub::new(&calls, &entered).parking(&release).failing(),
        ));
        entered.notified().await;

        let b = tokio::spawn(connect_cell(
            Arc::clone(&cache),
            Arc::clone(&signer),
            key.clone(),
            "imagodei".into(),
            cell(1),
            LONG,
            MintStub::new(&calls, &entered).seeded(3),
        ));

        // `notify_one` STORES a permit when no waiter is registered yet, so the
        // release can never be lost to a scheduling order — unlike
        // `notify_waiters`, which only wakes already-registered waiters.
        release.notify_one();
        let a = a.await.unwrap();
        let b = b.await.unwrap();

        assert!(!a.minted, "A's authorize failed");
        assert_eq!(b.decision, CredentialDecision::Mint);
        assert!(b.minted, "a failed first mint must not be read as done");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(cache.live_leases(), 0);
    }

    /// NO GLOBAL SERIALIZATION. A conductor wedged on one cell must not stall a
    /// different cell — one sick role cannot take the whole connect down.
    #[tokio::test]
    async fn two_different_cells_mint_in_parallel() {
        let cache = Arc::new(SigningCredentialCache::new());
        let signer = Arc::new(ClientAgentSigner::default());
        let key = endpoint_key("adam:4444", "elohim");
        let calls = Arc::new(AtomicUsize::new(0));
        let entered = Arc::new(Notify::new());
        let wedged = Arc::new(Notify::new());

        let stuck = tokio::spawn(connect_cell(
            Arc::clone(&cache),
            Arc::clone(&signer),
            key.clone(),
            "imagodei".into(),
            cell(1),
            LONG,
            MintStub::new(&calls, &entered).parking(&wedged),
        ));
        entered.notified().await;

        // The other cell completes WHILE the first is still inside its window.
        let other = tokio::time::timeout(
            Duration::from_secs(2),
            connect_cell(
                Arc::clone(&cache),
                Arc::clone(&signer),
                key.clone(),
                "infrastructure".into(),
                cell(9),
                LONG,
                MintStub::new(&calls, &entered).seeded(9),
            ),
        )
        .await
        .expect("a wedged cell must not stall a different cell");
        assert_eq!(other.decision, CredentialDecision::Mint);
        assert!(other.minted && other.serialized);

        wedged.notify_one();
        assert!(stuck.await.unwrap().minted);
        assert_eq!(cache.live_leases(), 0);
    }

    /// BOUNDED WAITING. A connect queued behind a wedged holder is released by
    /// the per-conductor-call deadline and falls through UNSERIALIZED — which is
    /// exactly the pre-lease behaviour (a possible duplicate mint), never a
    /// failed connect and never a skipped credential that was never cached.
    #[tokio::test]
    async fn a_waiter_past_the_deadline_falls_through_to_the_pre_lease_behaviour() {
        let cache = Arc::new(SigningCredentialCache::new());
        let signer = Arc::new(ClientAgentSigner::default());
        let key = endpoint_key("adam:4444", "elohim");
        let calls = Arc::new(AtomicUsize::new(0));
        let entered = Arc::new(Notify::new());
        let wedged = Arc::new(Notify::new());

        let stuck = tokio::spawn(connect_cell(
            Arc::clone(&cache),
            Arc::clone(&signer),
            key.clone(),
            "imagodei".into(),
            cell(1),
            LONG,
            MintStub::new(&calls, &entered).parking(&wedged),
        ));
        entered.notified().await;

        let waiter = connect_cell(
            Arc::clone(&cache),
            Arc::clone(&signer),
            key.clone(),
            "imagodei".into(),
            cell(1),
            Duration::from_millis(50),
            MintStub::new(&calls, &entered).seeded(4),
        )
        .await;

        assert!(
            !waiter.serialized,
            "the waiter must be released by the deadline, not parked forever"
        );
        assert!(
            waiter.minted,
            "expiry degrades to the pre-lease behaviour — the connect still succeeds"
        );

        wedged.notify_one();
        stuck.await.unwrap();
        assert_eq!(cache.live_leases(), 0);
    }

    // ── endpoint_key canonicalisation ───────────────────────────────────────

    /// ONE CONDUCTOR, ONE KEY. Two spellings of the same admin interface that
    /// miss each other mint a second `CapGrant` for a cell that already has one.
    #[test]
    fn one_conductor_written_several_ways_is_one_cache_key() {
        let canonical = endpoint_key("elohim-adam-alpha:4444", "elohim");
        for spelling in [
            "ws://elohim-adam-alpha:4444",
            "wss://elohim-adam-alpha:4444",
            "ws://elohim-adam-alpha:4444/",
            "  ws://Elohim-Adam-Alpha:4444  ",
            "ELOHIM-ADAM-ALPHA:4444",
            "ws://elohim-adam-alpha:4444/admin?x=1",
        ] {
            assert_eq!(
                endpoint_key(spelling, "elohim"),
                canonical,
                "{spelling:?} must resolve to the same conductor key"
            );
        }
    }

    /// …while genuinely different conductors, ports, or apps stay apart. A key
    /// that over-merges would hand one conductor's credential to another, which
    /// is the failure the per-conductor crux forbids.
    #[test]
    fn different_conductors_ports_and_apps_stay_distinct() {
        let base = endpoint_key("adam:4444", "elohim");
        assert_ne!(base, endpoint_key("matthew:4444", "elohim"));
        assert_ne!(base, endpoint_key("adam:8444", "elohim"), "port matters");
        assert_ne!(
            base,
            endpoint_key("adam:4444", "elohim-hosted-jane"),
            "a hosted human's app has its own cells"
        );
    }

    // ── the rejection classifier ────────────────────────────────────────────

    /// What the heal listens for. These are the shapes a conductor answers with
    /// when the grant behind our signature is gone.
    #[test]
    fn cap_grant_rejections_are_recognised() {
        for message in [
            "Zome call failed: Unauthorized",
            "InvalidCommit: CapabilityCheckFailed",
            "no cap grant found for secret",
            "Ribosome error: CapGrant not found",
            "invalid cap secret",
        ] {
            assert!(
                looks_like_a_rejected_cap_grant(message),
                "must be treated as a stale grant: {message}"
            );
        }
    }

    /// …and what it must NOT listen for. A validation failure or a missing zome
    /// function is the conductor answering correctly; healing there would mint
    /// a grant for no reason on every such error.
    #[test]
    fn ordinary_zome_errors_are_not_cap_grant_rejections() {
        for message in [
            "Zome function find_publishers doesn't exist",
            "validation failed: content too large",
            "source chain head has moved",
            "Zome call timed out after 10000ms",
        ] {
            assert!(
                !looks_like_a_rejected_cap_grant(message),
                "must NOT be read as a stale grant: {message}"
            );
        }
    }
}
