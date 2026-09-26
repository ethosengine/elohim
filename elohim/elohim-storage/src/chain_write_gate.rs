//! CHAIN WRITE GATE — one source chain, one writer at a time, per cell.
//!
//! ## The failure this closes
//!
//! A Holochain cell has exactly ONE source chain, and a commit is an optimistic
//! compare-and-flush against its head. Any bundle that *began* on a head which
//! has since moved is refused whole:
//!
//! ```text
//! Source chain error: Attempted to commit a bundle to the source chain,
//! but the source chain head has moved since the bundle began
//! ```
//!
//! elohim-storage authors on its own cell from at least six independent tasks —
//! the reconcile sweep's adopt-before-author declare, the sync-triggered head
//! adoption worker, the re-anchor backfill, provide-reconcile's commons author,
//! the REA/ordered-signal path, and HTTP-driven author writes. None of them knew
//! about the others, so every one of them was racing the other five *inside a
//! single process*, and the loser surfaced the conductor's refusal verbatim to
//! its caller (measured 2026-09-17: `HTTP 502 {"error":"Conductor error: … head
//! has moved…"}` on a peer's own declare; `HTTP 503` on `seed-commitments`).
//!
//! Losing a race against *ourselves* is not a distributed-systems fact, it is a
//! missing mutual exclusion. Losing one against an EXTERNAL writer (a fixture,
//! the steward's app authoring on the same cell) IS a real race — but a
//! `HeadMoved` failure commits nothing, so the identical call can simply be
//! re-offered. Neither belongs in a caller's error.
//!
//! ## Shape
//!
//! [`dispatch`] is the single choke point. Every zome call
//! [`crate::hc_client::HcClient`] makes passes through it:
//!
//! * a READ ([`is_chain_write`] says no) goes straight through — no lock, no
//!   metric, byte-identical to the pre-gate path;
//! * a WRITE takes the per-chain async mutex, makes ONE conductor call while
//!   holding it, and releases.
//!
//! The lock is held across nothing but the conductor call itself — not across a
//! backoff sleep. A `HeadMoved` retry RE-QUEUES rather than camping on the lock.
//!
//! ## Capacity is acquired INSIDE the lock, and that order is load-bearing
//!
//! The conductor's read-permit pool ([`crate::conductor_admission`]) is sized
//! `max(2 * available_parallelism, 8) - 3`, which on a cgroup-capped 1–2 CPU pod
//! is FIVE permits. The reconcile sweep fans out eight concurrent adopt→declare
//! writes on one cell. If a writer took its permit and THEN queued on this
//! mutex, five of those eight would hold the entire pool while four of them sat
//! doing nothing but waiting on each other — `in_flight` pinned at capacity,
//! every interactive HTTP read waiting out `DEFAULT_INTERACTIVE_WAIT` and
//! returning a 503 shed. The pre-gate code could not produce that: those permits
//! turned over at conductor RTT.
//!
//! So the order is **chain lock → admission permit**, uniformly: [`dispatch`]
//! takes the lock, and the caller's closure acquires and releases its own permit
//! around the single websocket call. Nothing in the crate nests the other way,
//! so there is no lock-order inversion. It also keeps admission's own instrument
//! honest — `elohim_conductor_admission_hold_ms` is **W**, service time, and it
//! measures the conductor again rather than our serialization plus backoff.
//!
//! The cost of the inversion is real but local: a writer may hold the chain lock
//! while queued for capacity, which blocks only same-chain writers — who were
//! blocked anyway, by the writer ahead of them.
//!
//! ## Fairness, and the bound on it
//!
//! `tokio::sync::Mutex` is FIFO-fair. An `Interactive` writer that arrives while
//! a `Background` batch is running therefore waits for AT MOST the one in-flight
//! conductor call plus whatever was already queued ahead of it — never for the
//! whole batch, because a batch holds the lock per CALL, not per batch. That is
//! the bound; there is no priority lane and this module deliberately does not
//! add one (a priority mutex would need its own starvation story, and the
//! measured hold is a single zome call). Admission keeps its own class priority,
//! unchanged.
//!
//! ## Why the classifier defaults to WRITE
//!
//! Guessing "read" for a function that commits loses the guarantee silently;
//! guessing "write" for a read costs a little serialization. So the table is an
//! explicit, DNA-verified allowlist of READS and everything else — including
//! every name nobody has classified yet — is a write. The derivation procedure,
//! and the `get_content_by_id` defect that retired the earlier prefix rule, are
//! recorded on [`CRATE_READ_FNS`].
//!
//! ## What this does NOT close
//!
//! * **Cross-DNA bridge commits.** An extern reached on cell A that calls
//!   `CallTargetCell::OtherRole("elohim")` commits on cell B's chain while this
//!   gate holds cell A's lock. `imagodei::create_self_revocation` and
//!   `submit_revocation_vote` are the two live instances; both are classified
//!   WRITE so they serialize against each other, but they do not serialize
//!   against a concurrent elohim-cell declare. Closing it needs the gate to know
//!   the bridge target, which only the DNA knows.
//! * **The admin-socket CapGrant.** See [`grant_capability_serialized`].
//! * **`conductor_client::ConductorClient`** — a second raw websocket used only
//!   by `content_server::ContentServerBridge`, which nothing in the binary
//!   constructs today (it survives as a `&mut` parameter in `blob_store.rs`).
//!   Its three infrastructure writes would bypass this gate if it were ever
//!   wired up; route it through [`dispatch`] on that day.
//! * **Cross-process races.** Two storage processes on one cell (which the
//!   protocol does not do) still race, and land in the bounded retry exactly as
//!   an external writer does.
//!
//! Not a timeout: nothing dispatched is ever abandoned here.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use holochain_client::CellId;
use tokio::time::Instant;

use crate::error::StorageError;

// =============================================================================
// Writer vocabulary
// =============================================================================

/// The CLOSED vocabulary of in-process chain writers, used as the single metric
/// label on this module's series.
///
/// Closed on purpose: a metric label that can take a content id (or any
/// unbounded value) is a cardinality bomb in a Prometheus scrape. A writer with
/// no entry here reports as [`WriterKind::Other`] — which is a legible answer,
/// not a gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterKind {
    /// The reconcile sweep's adopt-before-author declare leg.
    SweepAdopt,
    /// The sync-triggered head-adoption worker
    /// ([`crate::services::head_adoption_trigger`]).
    TriggerAdopt,
    /// [`crate::services::reanchor_backfill`] re-authoring a dead/NULL anchor.
    Reanchor,
    /// Provide-reconcile's commons author.
    Provide,
    /// The REA ledger / ordered-signal author path.
    ReaSignal,
    /// An operator-driven HTTP write (PATCH /db/content → declare, and friends).
    HttpAuthor,
    /// Anything that did not name itself.
    Other,
}

impl WriterKind {
    /// Every variant, for metric pre-touch.
    pub const ALL: [WriterKind; 7] = [
        WriterKind::SweepAdopt,
        WriterKind::TriggerAdopt,
        WriterKind::Reanchor,
        WriterKind::Provide,
        WriterKind::ReaSignal,
        WriterKind::HttpAuthor,
        WriterKind::Other,
    ];

    /// The metric label. Stable wire vocabulary — do not rename without moving
    /// the dashboards that read it.
    pub fn label(self) -> &'static str {
        match self {
            WriterKind::SweepAdopt => "sweep_adopt",
            WriterKind::TriggerAdopt => "trigger_adopt",
            WriterKind::Reanchor => "reanchor",
            WriterKind::Provide => "provide",
            WriterKind::ReaSignal => "rea_signal",
            WriterKind::HttpAuthor => "http_author",
            WriterKind::Other => "other",
        }
    }
}

tokio::task_local! {
    /// Which writer the CURRENT task is acting as.
    ///
    /// A task-local rather than a parameter because the alternative is threading
    /// a label through a dozen `conductor_writes` helpers and their callers — a
    /// crate-wide signature change for an observability field. It does NOT cross
    /// `tokio::spawn`, so a writer that spawns must set it inside the spawned
    /// future; the default is [`WriterKind::Other`], so forgetting mislabels but
    /// never breaks.
    static WRITER_KIND: WriterKind;

    /// `(reached the gate's queue, offered to the conductor)` — set by
    /// [`witnessed`] so an OUTER timeout can ask whether the call it abandoned
    /// was merely parked or actually offered.
    static DISPATCH_FLAGS: (Arc<AtomicBool>, Arc<AtomicBool>);
}

/// Run `fut` labelled as `kind`.
pub async fn as_writer<F: Future>(kind: WriterKind, fut: F) -> F::Output {
    WRITER_KIND.scope(kind, fut).await
}

/// The current task's writer kind, or [`WriterKind::Other`].
pub fn current_writer_kind() -> WriterKind {
    WRITER_KIND.try_with(|k| *k).unwrap_or(WriterKind::Other)
}

/// "Did the call I just gave up on actually reach the conductor?"
///
/// A caller that wraps a write in `tokio::time::timeout` needs this, because the
/// two outcomes are not the same fact and the heal ledger records them
/// differently. A call that was DISPATCHED and then abandoned may still commit
/// — the wasm body is uncancellable — which is what
/// `p2p::projection_reconcile`'s `HEAL_SYNTHETIC_TIMEOUT_MARKER` means. A call
/// that timed out while still QUEUED behind this cell's other writers committed
/// nothing and is an admission-shed shape; recording it as maybe-committed
/// poisons the ledger with a write that never happened.
///
/// # Why two flags and not one
///
/// "Not dispatched" and "never asked" are different, and collapsing them is the
/// same mistake in the other direction. A future that never reaches [`dispatch`]
/// at all (a read, a non-zome operation, a test fake) leaves both flags clear —
/// and MUST keep the conservative maybe-committed reading, because this module
/// knows nothing about it. Only `queued && !dispatched` is positive evidence
/// that the gate itself held the call back, and only that answers
/// [`Self::parked_without_dispatch`] true.
#[derive(Debug, Clone, Default)]
pub struct DispatchWitness {
    queued: Arc<AtomicBool>,
    dispatched: Arc<AtomicBool>,
}

impl DispatchWitness {
    /// A fresh witness: nothing queued, nothing dispatched.
    pub fn new() -> Self {
        Self::default()
    }

    /// Did the conductor call start?
    pub fn dispatched(&self) -> bool {
        self.dispatched.load(Ordering::Relaxed)
    }

    /// Positive evidence that the gate parked this call and never offered it —
    /// so nothing can have committed. False for anything this module did not
    /// see, which keeps the caller's conservative reading intact.
    pub fn parked_without_dispatch(&self) -> bool {
        self.queued.load(Ordering::Relaxed) && !self.dispatched()
    }
}

/// Run `fut` so the gate can report dispatch through `witness`.
///
/// Wrap the future INSIDE the timeout, not outside: the flags must be set by the
/// same task the timeout is racing.
pub async fn witnessed<F: Future>(witness: &DispatchWitness, fut: F) -> F::Output {
    DISPATCH_FLAGS
        .scope((witness.queued.clone(), witness.dispatched.clone()), fut)
        .await
}

/// Mark the current task's witness, if any, as having reached the gate's queue.
fn mark_queued() {
    let _ = DISPATCH_FLAGS.try_with(|(queued, _)| queued.store(true, Ordering::Relaxed));
}

/// Mark the current task's witness, if any, as dispatched.
fn mark_dispatched() {
    let _ = DISPATCH_FLAGS.try_with(|(_, dispatched)| dispatched.store(true, Ordering::Relaxed));
}

// =============================================================================
// Write / read classification
// =============================================================================

/// Coordinator externs this crate calls that are VERIFIED commit-free.
///
/// # How this table was built, and why it is not a prefix rule
///
/// It was a prefix rule, and the prefix rule was WRONG on the hottest extern in
/// the crate: `content_store::get_content_by_id` has a healing fallback that
/// does `create_entry(&EntryTypes::Content(..))` + `create_id_to_content_link`
/// (`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:7283-7284`). A
/// name that reads like a read committed unserialized on every v2 miss, so the
/// class this module exists to close survived — and, worse, the resulting loss
/// read as an EXTERNAL race and inflated the retry counter that claims to prove
/// one. English is not evidence about a source chain.
///
/// So this table is derived from the DNA, by the procedure recorded here so the
/// next maintainer can repeat it rather than trust it:
///
/// 1. Every `fn_name` argument reaching the three
///    [`crate::hc_client::HcClient`] zome-call methods was enumerated from the
///    call sites — literals, the `const … : &str` names, and the two generic
///    forwarders (`api::account::forward_to_imagodei`,
///    `services::feedback_projector::conductor::ConductorReader::call`).
/// 2. For each name, the extern was located in `elohim/holochain/dna/*/zomes/`
///    and its body taken by brace matching, then its TRANSITIVE call closure
///    walked, looking for `create_entry` / `update_entry` / `delete_entry` /
///    `create_link` / `delete_link` / cap-grant primitives.
/// 3. A name lands here ONLY if that whole closure is commit-free. Everything
///    else — including a name whose closure could not be fully resolved — is a
///    write.
///
/// Cross-DNA bridge calls count as writes: `imagodei::create_self_revocation`
/// and `imagodei::submit_revocation_vote` commit NOTHING on the imagodei chain
/// but reach `CallTargetCell::OtherRole("elohim")`, which commits on the elohim
/// cell's chain under the same agent. See the module doc's "what this does not
/// close" for the hole that leaves.
///
/// The first eight entries ARE [`crate::closed_chain_fence::CLOSED_CHAIN_READ_FNS`],
/// spliced in below rather than re-listed, so the crate has ONE answer to "does
/// this extern write?" — see [`READ_FNS`].
const CRATE_READ_FNS: &[&str] = &[
    // --- elohim / content_store -------------------------------------------
    "get_attestations_for_subject",
    "get_canonical_election_evidence",
    "get_content",
    "get_content_lineage",
    "get_feedback_signal_record",
    "get_feedback_signal_refs_for_target",
    "get_rea_commitment",
    "get_rea_economic_event",
    "get_record_for_action",
    "list_feedback_signal_refs_by_signer",
    "resolve_canonical_election",
    "resolve_canonical_elections",
    "resolve_content_head",
    "resolve_content_head_local",
    "resolve_content_heads_local",
    "validate_carried_head_record",
    "verify_carried_election",
    "verify_carried_head_evidence",
    // --- imagodei ----------------------------------------------------------
    "get_bindings_for_peer",
    "get_collab_qahal_cid_for_agreement",
    "get_collab_status",
    "get_collective_by_action",
    "get_membership_by_action",
    "get_my_household_collective_cids",
    "list_memberships_for_collective_cid",
    "query_my_source_chain",
    "query_my_source_chain_links",
    "sign_for_agent",
    // --- mishpat -----------------------------------------------------------
    "get_commitment",
    "get_commitment_authority_links",
    "get_commitment_record",
    "get_commitment_state_links",
    "get_lineage_successors",
    // --- infrastructure ----------------------------------------------------
    "find_publishers",
    "get_latest_peer_status_for_agent",
    // `get_doorway_attestations` is the `infrastructure` role's cell probe
    // (`services::cell_probe`). Source-verified write-free: it is a
    // `TODO(Stage-F)` stub returning `Ok(Vec::new())` and reads nothing —
    // infrastructure/zomes/infrastructure/src/lib.rs:889.
    "get_doorway_attestations",
    // --- the cell probe, in four coordinators ------------------------------
    // `is_bootstrap_steward` exists in content_store, imagodei, mishpat and
    // node_registry_coordinator with the same body: `am_i_bootstrap_steward()`
    // = `dna_info()` (DNA modifiers) + `agent_info()`. No commit, no
    // `get_links`, no DHT access, no DB scan — which is exactly why it is the
    // probe. See elohim/zomes/content_store/src/bootstrap_steward.rs:90-107.
    //
    // It must be classified READ here or the probe would take the cell's write
    // lock and queue behind real writers — an unbounded wait on a call that
    // cannot be cancelled, which is the opposite of what a bounded probe is for.
    "is_bootstrap_steward",
];

/// The ONE read table: this crate's verified reads ∪ the closed-chain fence's
/// own source-verified write-free allowlist.
///
/// The union is the point. `closed_chain_fence` already answers "is this extern
/// write-free?" for a different purpose (what a SEALED chain still serves), and
/// its list is mirrored against the a2o harness rail. Two independently
/// maintained answers to one question drift — and they had: five fence entries
/// (`my_chain_activity`, `known_agents`, `export_records`, `export_held_records`,
/// `agent_activity_of`) were being serialized here for nothing, `known_agents`
/// being the live neighbour sweep in `services::lineage_bridge`. The fence stays
/// the NARROWER list (it is a security boundary and deliberately mirrors the
/// harness); this gate is the SUPERSET, and
/// `fence_read_fns_are_a_subset_of_the_gate_read_table` fails if that ever
/// inverts.
pub fn is_read_fn(fn_name: &str) -> bool {
    CRATE_READ_FNS.contains(&fn_name)
        || crate::closed_chain_fence::CLOSED_CHAIN_READ_FNS.contains(&fn_name)
}

/// Every name the table classifies, for the drift test below.
pub fn classified_read_fns() -> impl Iterator<Item = &'static str> {
    CRATE_READ_FNS
        .iter()
        .chain(crate::closed_chain_fence::CLOSED_CHAIN_READ_FNS.iter())
        .copied()
}

/// Externs verified to COMMIT. Not consulted by [`is_chain_write`] — an unknown
/// name is a write either way — but carried so the drift test can assert that
/// every name the crate calls is actually classified, and so the DNA evidence
/// for each has a home next to the reads.
pub const CRATE_WRITE_FNS: &[&str] = &[
    // --- elohim / content_store -------------------------------------------
    "compute_task",
    "create_attention_tending",
    "create_content",                        // → create_content_unchecked
    "create_feedback_signal",                //
    "create_rea_commitment",                 //
    "create_rea_economic_event",             //
    "create_rea_economic_event_from_intent", // → create_rea_economic_event
    "declare_canonical_content_head", // → declare_canonical_head_inner → create_canonical_head_link
    "declare_content_head",
    "declare_earned_canonical_head", // → declare_canonical_head_inner → create_canonical_head_link
    "get_content_by_id",             // HEALING FALLBACK COMMITS — see CRATE_READ_FNS doc
    "process_import_chunk",
    "queue_import",
    "update_content",
    "update_rea_commitment_state",
    // --- imagodei ----------------------------------------------------------
    "add_portal_host",
    "attest_collab_agreement",
    "create_agent_peer_binding",
    "create_collab_agreement",
    "create_collective",
    "create_self_revocation", // cross-DNA bridge → elohim chain
    "issue_attestation",
    "remove_portal_host",
    "submit_revocation_vote", // cross-DNA bridge → elohim chain
    "withdraw_membership_clean",
    // --- mishpat -----------------------------------------------------------
    "create_commitment",
    "create_commitment_state_link",
    // --- node-registry -----------------------------------------------------
    "carry_from", // → carry_page
    "create_shard_assignment",
    "readopt_from",
    "seal_close",
    // --- infrastructure ----------------------------------------------------
    "mark_content_server_offline",
    "record_peer_status",
    "register_content_server",
    "update_content_server_heartbeat",
];

/// Does this coordinator call commit to the source chain?
///
/// Unknown ⇒ WRITE, and the first sighting of an unknown name is logged once at
/// WARN so the table gets maintained rather than quietly out-grown. Serializing
/// an unclassified read costs throughput; treating an unclassified write as a
/// read costs the guarantee.
///
/// `zome` is accepted (and currently unused) so a future per-zome exception can
/// land without touching every call site.
pub fn is_chain_write(_zome: &str, fn_name: &str) -> bool {
    if is_read_fn(fn_name) {
        return false;
    }
    if !CRATE_WRITE_FNS.contains(&fn_name) {
        warn_unknown_extern(fn_name);
    }
    true
}

/// WARN once per process per unrecognised extern name. Bounded by the number of
/// distinct names the binary can pass, so this set cannot grow without bound.
fn warn_unknown_extern(fn_name: &str) {
    static SEEN: OnceLock<Mutex<std::collections::BTreeSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(std::collections::BTreeSet::new()));
    let mut guard = match seen.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if guard.insert(fn_name.to_string()) {
        tracing::warn!(
            fn_name = %fn_name,
            "chain_write_gate: extern is not in the verified table; serializing it as a WRITE. \
             Classify it against the DNA and add it to CRATE_READ_FNS or CRATE_WRITE_FNS."
        );
    }
}

/// Exactly the source-chain `HeadMoved` class, and nothing else.
///
/// Both spellings are real: the conductor surfaces `SourceChainError::HeadMoved`
/// from some paths and the prose "source chain head has moved …" from others.
/// The [`StorageError::Conductor`] discriminant is part of the match on purpose
/// — the same words arriving as an `InvalidInput` are a caller's payload, not a
/// race, and replaying them would be a bug.
pub fn is_source_chain_head_moved(error: &StorageError) -> bool {
    let StorageError::Conductor(message) = error else {
        return false;
    };
    message.contains("HeadMoved") || message.contains("source chain head has moved")
}

// =============================================================================
// Retry bounds
// =============================================================================

/// Total conductor attempts for one write, including the first.
const MAX_ATTEMPTS: u32 = 4;

/// Backoff before attempts 2, 3 and 4. Small, because the loser of a local race
/// is already serialized behind the winner by the time it retries — this delay
/// only exists to let an EXTERNAL writer's commit land.
const BACKOFFS: [Duration; 3] = [
    Duration::from_millis(50),
    Duration::from_millis(120),
    Duration::from_millis(280),
];

/// Ceiling on the whole retry sequence, measured from the first lock attempt.
/// Flat: nothing in this crate hands a write its own deadline today (see
/// [`write_serialized`]). Backoffs sum to ~450 ms, so this bounds the WAITING
/// as well — four attempts that each queue behind a slow conductor stop here
/// rather than at attempt four.
const RETRY_BUDGET: Duration = Duration::from_millis(2_000);

/// Deterministic ±20% jitter, so two writers that lose the same race do not
/// re-offer in lockstep forever.
///
/// Process-local counter rather than `rand`: `rand` is behind the `p2p-iroh`
/// feature in this crate, and a jitter source that vanishes in a default build
/// is worse than one that is merely predictable. Nothing security-bearing reads
/// this.
fn jittered(base: Duration) -> Duration {
    static TICK: AtomicU64 = AtomicU64::new(0x9E3779B97F4A7C15);
    let mut x = TICK.fetch_add(0x9E3779B97F4A7C15, Ordering::Relaxed);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51AFD7ED558CCD);
    x ^= x >> 33;
    // 0..=40 percent, centred by subtracting 20.
    let pct = (x % 41) as i64 - 20;
    let millis = base.as_millis() as i64;
    let adjusted = millis + (millis * pct / 100);
    Duration::from_millis(adjusted.max(1) as u64)
}

// =============================================================================
// The per-chain lock registry
// =============================================================================

/// The process-wide registry of per-chain write locks.
///
/// Keyed by a chain key (`<dna>:<agent>`) rather than by `CellId` so the same
/// gate is shared by every [`crate::hc_client::HcClient`] in the per-role
/// registry that happens to target the same cell — two clients, one chain, one
/// lock. The outer `std::sync::Mutex` guards only the map lookup and is never
/// held across an await.
#[derive(Default)]
struct ChainWriteGate {
    locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl ChainWriteGate {
    fn lock_for(&self, chain_key: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = match self.locks.lock() {
            Ok(guard) => guard,
            // A poisoned registry means a previous holder panicked WHILE holding
            // the map lock — which this code never does across an await. Recover
            // rather than propagate: refusing every write because a map insert
            // once panicked would be a worse outage than the one it reports.
            Err(poisoned) => poisoned.into_inner(),
        };
        locks
            .entry(chain_key.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

fn gate() -> &'static ChainWriteGate {
    static GATE: OnceLock<ChainWriteGate> = OnceLock::new();
    GATE.get_or_init(ChainWriteGate::default)
}

/// The chain key for a cell — one source chain per `(dna, agent)` pair.
pub fn chain_key_of(cell_id: &CellId) -> String {
    format!("{}:{}", cell_id.dna_hash(), cell_id.agent_pubkey())
}

// =============================================================================
// The choke point
// =============================================================================

/// Serialize a NON-zome chain write against this cell's zome writers.
///
/// `AdminWebsocket::authorize_signing_credentials` mints a keypair and then
/// `grant_zome_call_capability` — which commits a `CapGrant` **on the cell's
/// source chain**, over the ADMIN socket, nowhere near [`dispatch`]. It is
/// bounded (the closed-chain fence reuses persisted credentials and mints at
/// most once per cell per process) but "bounded" is not "cannot race": a mint
/// during a running sweep moves the head under a gated writer, and the writer
/// then reports an EXTERNAL co-author it does not have.
///
/// So it takes the same per-cell lock. No retry: a losing mint surfaces at
/// connect time, where the reconnect path already owns the recovery, and
/// replaying a keypair mint is not the same kind of safe as replaying an
/// encoded zome payload.
pub async fn grant_capability_serialized<F, Fut, T, E>(cell_id: &CellId, grant: F) -> Result<T, E>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let lock = gate().lock_for(&chain_key_of(cell_id));
    let _guard = lock.lock().await;
    grant().await
}

/// THE choke point. Route one conductor call: reads straight through, writes
/// through the per-chain lock with a bounded `HeadMoved` retry.
///
/// Returns the answer and the duration of the ATTEMPT that produced it — not
/// the serialized wall-clock — so a caller's RTT signal keeps meaning "what the
/// conductor took", with queueing reported separately as
/// `elohim_chain_write_serialized_wait_ms`.
///
/// `call` is re-invoked per attempt and must therefore be replay-safe. That is
/// sound for every write in this crate: a `HeadMoved` refusal commits NOTHING
/// (the conductor rejects the whole bundle before flush), and no in-crate write
/// performs a non-idempotent side effect between the gate and the websocket —
/// the payload is encoded by the caller before it arrives here, and the only
/// per-attempt work is acquiring an admission permit, signing, and the call.
///
/// # `call` OWNS the admission permit
///
/// The closure must acquire its conductor-capacity permit itself, inside its own
/// body, and let it drop when the call returns. That inversion is load-bearing,
/// not stylistic — see the module doc's "Capacity is acquired INSIDE the lock".
pub async fn dispatch<F, Fut, T>(
    chain_key: &str,
    zome: &str,
    fn_name: &str,
    call: F,
) -> Result<(T, Duration), StorageError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, StorageError>>,
{
    if !is_chain_write(zome, fn_name) {
        let mut call = call;
        let started = Instant::now();
        mark_dispatched();
        let value = call().await?;
        return Ok((value, started.elapsed()));
    }
    write_serialized(chain_key, fn_name, call).await
}

/// The write arm of [`dispatch`], exposed for callers that have already
/// classified (and for tests).
pub async fn write_serialized<F, Fut, T>(
    chain_key: &str,
    fn_name: &str,
    mut call: F,
) -> Result<(T, Duration), StorageError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, StorageError>>,
{
    let writer = current_writer_kind();
    // From here on an outer timeout can distinguish "parked" from "offered".
    mark_queued();
    let lock = gate().lock_for(chain_key);

    // A FLAT budget, stated plainly. No caller in this crate owns a deadline
    // around a write today (the ~2 s budgets in `http.rs` fence the candidate
    // /election READ path, not the declare), so a "we never extend the caller's
    // deadline" claim would be about nothing. If a write ever acquires a real
    // budget, shorten to `min(caller_deadline, start + RETRY_BUDGET)` here — and
    // only shorten.
    let started = Instant::now();
    let budget_end = started + RETRY_BUDGET;

    let mut queued_total = Duration::ZERO;
    let mut last_error: Option<StorageError> = None;

    for attempt in 1..=MAX_ATTEMPTS {
        let queue_started = Instant::now();
        let guard = lock.lock().await;
        let waited = queue_started.elapsed();
        queued_total += waited;

        let dispatched_at = Instant::now();
        // Past this point the call is offered: the closure takes its admission
        // permit and crosses the websocket, so an outer timeout can no longer
        // claim nothing happened.
        mark_dispatched();
        let outcome = call().await;
        let rtt = dispatched_at.elapsed();
        // The lock models exclusive access to the chain HEAD, which the
        // conductor has finished contending for the instant the call returns.
        drop(guard);

        match outcome {
            Ok(value) => {
                crate::metrics::observe_chain_write_serialized(writer.label(), queued_total);
                return Ok((value, rtt));
            }
            Err(error) if is_source_chain_head_moved(&error) => {
                let backoff = BACKOFFS
                    .get(attempt as usize - 1)
                    .copied()
                    .map(jittered)
                    .unwrap_or(Duration::ZERO);
                let out_of_attempts = attempt == MAX_ATTEMPTS;
                // A budget that cannot fit the backoff PLUS another attempt of
                // roughly the size we just measured is a budget that has ended.
                let out_of_budget = Instant::now() + backoff + rtt >= budget_end;
                last_error = Some(error);
                if out_of_attempts || out_of_budget {
                    crate::metrics::observe_chain_write_serialized(writer.label(), queued_total);
                    crate::metrics::inc_head_moved_exhausted(writer.label());
                    tracing::info!(
                        writer = writer.label(),
                        fn_name = %fn_name,
                        attempts = attempt,
                        reason = if out_of_attempts { "attempts" } else { "budget" },
                        "chain write exhausted its source-chain head-moved retries"
                    );
                    break;
                }
                crate::metrics::inc_head_moved_retried(writer.label());
                tokio::time::sleep(backoff).await;
            }
            // Every other failure is a verdict, not a race. Return it untouched.
            Err(error) => {
                crate::metrics::observe_chain_write_serialized(writer.label(), queued_total);
                return Err(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        StorageError::Internal("chain write gate exhausted with no recorded error".into())
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    fn head_moved() -> StorageError {
        StorageError::Conductor("Zome call failed: SourceChainError::HeadMoved".into())
    }

    fn head_moved_prose() -> StorageError {
        StorageError::Conductor(
            "Zome call failed: Source chain error: Attempted to commit a bundle to the source \
             chain, but the source chain head has moved since the bundle began"
                .into(),
        )
    }

    /// A conductor stand-in that refuses with the EXACT production error the
    /// moment two calls are inside it at once — so "serialized" is proved by the
    /// absence of that refusal, not by a timing assertion.
    #[derive(Default)]
    struct OverlapDetectingChain {
        in_flight: AtomicUsize,
        max_in_flight: AtomicUsize,
        calls: AtomicUsize,
    }

    impl OverlapDetectingChain {
        async fn call(&self, work: Duration) -> Result<Vec<u8>, StorageError> {
            self.calls.fetch_add(1, AtomicOrdering::SeqCst);
            let now = self.in_flight.fetch_add(1, AtomicOrdering::SeqCst) + 1;
            self.max_in_flight.fetch_max(now, AtomicOrdering::SeqCst);
            tokio::time::sleep(work).await;
            self.in_flight.fetch_sub(1, AtomicOrdering::SeqCst);
            if now > 1 {
                return Err(head_moved_prose());
            }
            Ok(b"committed".to_vec())
        }
    }

    #[test]
    fn classifier_follows_the_dna_not_the_english() {
        // Verified commit-free in the DNA — including two that a prefix rule
        // gets wrong in the SAFE direction (needlessly serialized before):
        // `verify_carried_election` (no commit in its own body; its earlier
        // "creates a link" reading came from a mis-sliced line range) and the
        // closed-chain fence's `known_agents` / `my_chain_activity`.
        for read in [
            "get_content",
            "get_record_for_action",
            "list_memberships_for_collective_cid",
            "query_my_source_chain",
            "resolve_content_head",
            "resolve_content_head_local",
            "resolve_canonical_election",
            "get_canonical_election_evidence",
            "validate_carried_head_record",
            "verify_carried_election",
            "verify_carried_head_evidence",
            "find_publishers",
            "sign_for_agent",
            "known_agents",
            "my_chain_activity",
            "export_records",
            "export_held_records",
            "agent_activity_of",
        ] {
            assert!(!is_chain_write("content_store", read), "{read}");
        }

        // The defect this table exists for: a name that reads like a read and
        // commits. `content_store::get_content_by_id` heals a v2 miss with
        // `create_entry` + `create_id_to_content_link` (lib.rs:7283-7284).
        assert!(
            is_chain_write("content_store", "get_content_by_id"),
            "get_content_by_id COMMITS via its healing fallback — classifying it \
             read is the bug this table replaced"
        );
        // Cross-DNA bridges commit on the OTHER cell's chain.
        assert!(is_chain_write("imagodei", "create_self_revocation"));
        assert!(is_chain_write("imagodei", "submit_revocation_vote"));

        for write in [
            "create_content",
            "declare_content_head",
            "declare_canonical_content_head",
            "update_content",
            "update_rea_commitment_state",
            "create_rea_economic_event",
            "issue_attestation",
            "record_peer_status",
            "queue_import",
            "process_import_chunk",
            "seal_close",
            // Unknown names default to write — the safe direction.
            "some_future_extern",
        ] {
            assert!(is_chain_write("content_store", write), "{write}");
        }
    }

    #[test]
    fn the_write_table_and_the_read_table_are_disjoint() {
        for w in CRATE_WRITE_FNS {
            assert!(
                !is_read_fn(w),
                "{w} is in both tables — one of the two classifications is wrong"
            );
        }
        // And every classified read really answers read.
        for r in classified_read_fns() {
            assert!(!is_chain_write("any", r), "{r}");
        }
    }

    #[test]
    fn head_moved_matcher_is_exact() {
        assert!(is_source_chain_head_moved(&head_moved()));
        assert!(is_source_chain_head_moved(&head_moved_prose()));
        assert!(!is_source_chain_head_moved(&StorageError::Conductor(
            "Zome call failed: HTTP 503 Service Unavailable".into()
        )));
        // Same words, wrong discriminant — a caller's payload, not a race.
        assert!(!is_source_chain_head_moved(&StorageError::InvalidInput(
            "HeadMoved is not an input".into()
        )));
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_writers_on_one_cell_never_overlap_and_all_succeed() {
        let chain = Arc::new(OverlapDetectingChain::default());
        let key = "chain:serialize-me";

        let mut handles = Vec::new();
        for _ in 0..8 {
            let chain = chain.clone();
            handles.push(tokio::spawn(async move {
                write_serialized(key, "create_content", || {
                    let chain = chain.clone();
                    async move { chain.call(Duration::from_millis(10)).await }
                })
                .await
            }));
        }

        for handle in handles {
            let (bytes, _rtt) = handle.await.unwrap().expect("every writer commits");
            assert_eq!(bytes, b"committed");
        }
        assert_eq!(
            chain.max_in_flight.load(AtomicOrdering::SeqCst),
            1,
            "two writers were inside the conductor call at once"
        );
        assert_eq!(
            chain.calls.load(AtomicOrdering::SeqCst),
            8,
            "serialization should cost no extra attempts"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn reads_are_not_serialized() {
        let chain = Arc::new(OverlapDetectingChain::default());
        let key = "chain:reads";

        let mut handles = Vec::new();
        for _ in 0..4 {
            let chain = chain.clone();
            handles.push(tokio::spawn(async move {
                dispatch(key, "content_store", "get_content", || {
                    let chain = chain.clone();
                    // A read that overlaps returns the stand-in's refusal; we
                    // assert on the OVERLAP COUNTER, not on the result.
                    async move {
                        let _ = chain.call(Duration::from_millis(10)).await;
                        Ok::<_, StorageError>(())
                    }
                })
                .await
            }));
        }
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        assert!(
            chain.max_in_flight.load(AtomicOrdering::SeqCst) > 1,
            "reads must stay concurrent — the gate is for writes only"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn an_external_writers_head_move_is_retried_and_succeeds() {
        let calls = AtomicUsize::new(0);
        let (bytes, _rtt) = write_serialized("chain:external", "declare_content_head", || {
            let attempt = calls.fetch_add(1, AtomicOrdering::SeqCst);
            async move {
                if attempt == 0 {
                    Err(head_moved_prose())
                } else {
                    Ok(b"declared".to_vec())
                }
            }
        })
        .await
        .expect("the replay commits");

        assert_eq!(bytes, b"declared");
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn a_non_head_moved_error_is_not_retried() {
        let calls = AtomicUsize::new(0);
        let error = write_serialized("chain:verdict", "create_content", || {
            calls.fetch_add(1, AtomicOrdering::SeqCst);
            async {
                Err::<(), _>(StorageError::Conductor(
                    "Zome call failed: HTTP 503 Service Unavailable".into(),
                ))
            }
        })
        .await
        .unwrap_err();

        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
        assert!(error.to_string().contains("503 Service Unavailable"));
    }

    #[tokio::test(start_paused = true)]
    async fn exhaustion_returns_the_original_error_within_the_attempt_bound() {
        let calls = AtomicUsize::new(0);
        let error = write_serialized("chain:exhaust", "declare_content_head", || {
            calls.fetch_add(1, AtomicOrdering::SeqCst);
            async { Err::<(), _>(head_moved()) }
        })
        .await
        .unwrap_err();

        assert_eq!(calls.load(AtomicOrdering::SeqCst), MAX_ATTEMPTS as usize);
        assert!(is_source_chain_head_moved(&error), "{error}");
    }

    #[tokio::test(start_paused = true)]
    async fn the_retry_budget_stops_before_the_attempt_bound_when_calls_are_slow() {
        let calls = AtomicUsize::new(0);
        let started = Instant::now();
        // Each attempt burns most of the flat budget, so the gate must stop on
        // BUDGET rather than spending all four attempts.
        let error = write_serialized("chain:budget", "declare_content_head", || {
            calls.fetch_add(1, AtomicOrdering::SeqCst);
            async {
                tokio::time::sleep(Duration::from_millis(900)).await;
                Err::<(), _>(head_moved())
            }
        })
        .await
        .unwrap_err();

        assert!(
            calls.load(AtomicOrdering::SeqCst) < MAX_ATTEMPTS as usize,
            "budget did not bind before the attempt count did"
        );
        assert!(is_source_chain_head_moved(&error));
        assert!(
            started.elapsed() < RETRY_BUDGET + Duration::from_millis(900),
            "gate spent past its budget plus one in-flight attempt"
        );
    }

    /// B2's regression test. A queued writer must hold NO conductor capacity:
    /// the permit is taken inside the lock, so the reader always gets one.
    ///
    /// The fake here mirrors `HcClient`'s shape — permit inside the gated
    /// closure — and a `capacity`-sized semaphore stands in for the conductor's
    /// read pool. Before the inversion the writers took their permits first and
    /// this assertion failed.
    #[tokio::test(start_paused = true)]
    async fn queued_writers_hold_no_admission_capacity_so_a_read_is_never_shed() {
        use tokio::sync::Semaphore;

        const CAPACITY: usize = 5;
        let pool = Arc::new(Semaphore::new(CAPACITY));
        let key = "chain:capacity";

        // Eight concurrent writers on ONE cell — the reconcile sweep's fan-out.
        let mut writers = Vec::new();
        for _ in 0..8 {
            let pool = pool.clone();
            writers.push(tokio::spawn(async move {
                write_serialized(key, "declare_content_head", || {
                    let pool = pool.clone();
                    async move {
                        let permit = pool.acquire().await.expect("pool open");
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        drop(permit);
                        Ok::<_, StorageError>(())
                    }
                })
                .await
            }));
        }

        // Let every writer reach its parked state.
        tokio::time::sleep(Duration::from_millis(10)).await;

        // An interactive READ asks for capacity. At most ONE writer can be
        // inside the pool (the lock holder), so 4 permits must be free.
        assert_eq!(
            pool.available_permits(),
            CAPACITY - 1,
            "queued writers are sitting on conductor capacity"
        );
        let read = tokio::time::timeout(Duration::from_millis(1), pool.acquire()).await;
        assert!(
            read.is_ok(),
            "an interactive read was shed behind queued writers"
        );

        drop(read);
        for w in writers {
            w.await.unwrap().unwrap();
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_timeout_while_queued_reports_not_dispatched() {
        let key = "chain:witness";
        let lock_held = Arc::new(tokio::sync::Notify::new());

        // Occupy the chain lock for longer than the waiter's timeout.
        let hog = {
            let lock_held = lock_held.clone();
            tokio::spawn(async move {
                let _ = write_serialized(key, "declare_content_head", || {
                    let lock_held = lock_held.clone();
                    async move {
                        lock_held.notify_one();
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        Ok::<_, StorageError>(())
                    }
                })
                .await;
            })
        };
        lock_held.notified().await;

        let witness = DispatchWitness::new();
        let dispatched_flag = AtomicBool::new(false);
        let timed_out = tokio::time::timeout(
            Duration::from_millis(50),
            witnessed(&witness, async {
                write_serialized(key, "declare_content_head", || {
                    dispatched_flag.store(true, AtomicOrdering::SeqCst);
                    async { Ok::<_, StorageError>(()) }
                })
                .await
            }),
        )
        .await;

        assert!(timed_out.is_err(), "the waiter should have timed out");
        assert!(
            !witness.dispatched(),
            "a call that never left the queue must not report as dispatched"
        );
        assert!(
            witness.parked_without_dispatch(),
            "the gate parked this call, so it must say so — nothing committed"
        );
        assert!(!dispatched_flag.load(AtomicOrdering::SeqCst));
        hog.await.unwrap();
    }

    /// The conservative half of the witness contract: an operation that never
    /// reaches the gate keeps the caller's maybe-committed reading, because this
    /// module observed nothing about it.
    #[tokio::test(start_paused = true)]
    async fn an_operation_that_never_reaches_the_gate_is_not_reported_as_parked() {
        let witness = DispatchWitness::new();
        let timed_out = tokio::time::timeout(
            Duration::from_millis(50),
            witnessed(&witness, async {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }),
        )
        .await;

        assert!(timed_out.is_err());
        assert!(!witness.dispatched());
        assert!(
            !witness.parked_without_dispatch(),
            "the gate never saw this call and must not claim it was parked"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_timeout_after_dispatch_reports_dispatched() {
        let witness = DispatchWitness::new();
        let timed_out = tokio::time::timeout(
            Duration::from_millis(50),
            witnessed(&witness, async {
                write_serialized("chain:witness2", "declare_content_head", || async {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    Ok::<_, StorageError>(())
                })
                .await
            }),
        )
        .await;

        assert!(timed_out.is_err());
        assert!(
            witness.dispatched(),
            "an offered call must report as dispatched — it may still commit"
        );
    }

    #[test]
    fn fence_read_fns_are_a_subset_of_the_gate_read_table() {
        for fence_read in crate::closed_chain_fence::CLOSED_CHAIN_READ_FNS {
            assert!(
                !is_chain_write("any", fence_read),
                "{fence_read} is write-free per the closed-chain fence but the gate serializes it                  — the two classifiers have drifted"
            );
        }
    }

    /// The drift rail. Scans this crate's own source for every `fn_name`
    /// literal reaching a `call_zome*` method and fails if one is absent from
    /// BOTH halves of the table — so a newly added call site cannot quietly
    /// inherit the unknown-defaults-to-write fallback without a human having
    /// read its extern in the DNA.
    #[test]
    fn every_zome_fn_literal_in_this_crate_is_classified() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut unclassified: Vec<String> = Vec::new();
        let mut files = vec![src];
        while let Some(path) = files.pop() {
            let Ok(entries) = std::fs::read_dir(&path) else {
                continue;
            };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    files.push(p);
                    continue;
                }
                if p.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&p) else {
                    continue;
                };
                for name in scan_zome_fn_literals(&strip_line_comments(&text)) {
                    if is_read_fn(&name) || CRATE_WRITE_FNS.contains(&name.as_str()) {
                        continue;
                    }
                    unclassified.push(format!("{}: {name}", p.display()));
                }
            }
        }
        unclassified.sort();
        unclassified.dedup();
        assert!(
            unclassified.is_empty(),
            "zome fn names reached from this crate but absent from the chain-write table.\n\
             Read each extern (and its transitive helpers) in elohim/holochain/dna/*/zomes/ and \
             add it to CRATE_READ_FNS (verified commit-free) or CRATE_WRITE_FNS:\n  {}",
            unclassified.join("\n  ")
        );
    }

    /// Drop `//`-comment tails so a doc example (`client.call_zome(cell, "my_zome",
    /// "my_fn", payload)`) is not read as a live call site. Crude but adequate:
    /// a `//` inside a string literal would over-trim, which can only ever HIDE
    /// a name, and the four names this rail found on first run were all prose.
    fn strip_line_comments(text: &str) -> String {
        text.lines()
            .map(|line| match line.find("//") {
                Some(idx) => &line[..idx],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Pull the `fn_name` argument out of every `.call_zome*(…)` in one file.
    ///
    /// Parses the argument list by paren matching and splits on top-level
    /// commas, then picks the fn slot BY ARITY — which is unambiguous across the
    /// three call shapes in this crate:
    ///
    /// * 3 args `(zome, fn, payload)` — `HcClient::call_zome{,_imagodei,_mishpat}`
    /// * 4 args `(zome, fn, payload, class)` — `HcClient::call_zome_timed`, and
    ///   `(target|cell, zome, fn, payload)` — the raw `app_ws` / legacy client
    ///
    /// Only a BARE string literal in that slot is reported: a `const` or a
    /// generic forwarder is invisible here by design (those were enumerated by
    /// hand when the table was built). The rail's job is to catch the shape a
    /// NEW call site is written in, not to re-derive the whole table.
    fn scan_zome_fn_literals(text: &str) -> Vec<String> {
        const MARKERS: [&str; 4] = [
            ".call_zome(",
            ".call_zome_timed(",
            ".call_zome_imagodei(",
            ".call_zome_mishpat(",
        ];
        let bytes = text.as_bytes();
        let mut out = Vec::new();
        for marker in MARKERS {
            let mut from = 0usize;
            while let Some(idx) = text[from..].find(marker) {
                let open = from + idx + marker.len() - 1; // at '('
                from = open + 1;
                let Some(args) = balanced_args(bytes, open) else {
                    continue;
                };
                let parts = split_top_level(&text[args.0..args.1]);
                let slot = match parts.len() {
                    3 => 1,
                    4 => 2,
                    _ => continue,
                };
                let candidate = parts[slot].trim();
                // Bare `"name"` only — `zome_name.into()` and `SOME_CONST` are
                // deliberately not guessed at.
                if candidate.len() > 2
                    && candidate.starts_with('"')
                    && candidate.ends_with('"')
                    && candidate[1..candidate.len() - 1]
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                {
                    out.push(candidate[1..candidate.len() - 1].to_string());
                }
            }
        }
        out
    }

    /// Byte range strictly inside the parens opening at `open`.
    fn balanced_args(bytes: &[u8], open: usize) -> Option<(usize, usize)> {
        let mut depth = 0usize;
        let mut in_str = false;
        let mut esc = false;
        for (i, &c) in bytes.iter().enumerate().skip(open) {
            if in_str {
                if esc {
                    esc = false;
                } else if c == b'\\' {
                    esc = true;
                } else if c == b'"' {
                    in_str = false;
                }
                continue;
            }
            match c {
                b'"' => in_str = true,
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((open + 1, i));
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Split an argument list on commas that are not nested and not in a string.
    fn split_top_level(args: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut depth = 0i32;
        let mut in_str = false;
        let mut esc = false;
        let mut start = 0usize;
        for (i, c) in args.char_indices() {
            if in_str {
                if esc {
                    esc = false;
                } else if c == '\\' {
                    esc = true;
                } else if c == '"' {
                    in_str = false;
                }
                continue;
            }
            match c {
                '"' => in_str = true,
                '(' | '[' | '{' | '<' => depth += 1,
                ')' | ']' | '}' | '>' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(&args[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        if !args[start..].trim().is_empty() {
            parts.push(&args[start..]);
        }
        parts
    }

    #[tokio::test(start_paused = true)]
    async fn an_interactive_writer_is_not_starved_by_a_background_batch() {
        let key = "chain:fairness";
        let batch_calls = 20;
        let per_call = Duration::from_millis(10);

        let batch = tokio::spawn(async move {
            as_writer(WriterKind::SweepAdopt, async move {
                for _ in 0..batch_calls {
                    let _ = write_serialized(key, "declare_content_head", || async {
                        tokio::time::sleep(per_call).await;
                        Ok::<_, StorageError>(())
                    })
                    .await;
                }
            })
            .await;
        });

        // Let the batch get a few calls in, then arrive as the person waiting.
        tokio::time::sleep(Duration::from_millis(25)).await;
        let arrived = Instant::now();
        as_writer(WriterKind::HttpAuthor, async {
            write_serialized(key, "update_content", || async {
                Ok::<_, StorageError>(())
            })
            .await
            .unwrap();
        })
        .await;
        let waited = arrived.elapsed();

        // FIFO fairness: at most the ONE in-flight call, because the batch holds
        // the lock per call, not per batch. Generous ceiling so the assertion is
        // about the SHAPE (constant, not proportional to the batch) — the whole
        // batch would be ~200ms.
        assert!(
            waited < per_call * 4,
            "interactive waited {waited:?} — that is batch-shaped, not call-shaped"
        );
        batch.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn writer_kind_defaults_to_other_and_scopes_cleanly() {
        assert_eq!(current_writer_kind(), WriterKind::Other);
        as_writer(WriterKind::Reanchor, async {
            assert_eq!(current_writer_kind(), WriterKind::Reanchor);
        })
        .await;
        assert_eq!(current_writer_kind(), WriterKind::Other);
    }

    #[test]
    fn jitter_stays_within_twenty_percent_and_never_reaches_zero() {
        for _ in 0..200 {
            let j = jittered(Duration::from_millis(100));
            assert!(j >= Duration::from_millis(80), "{j:?}");
            assert!(j <= Duration::from_millis(120), "{j:?}");
        }
        assert!(jittered(Duration::from_millis(1)) >= Duration::from_millis(1));
    }
}
