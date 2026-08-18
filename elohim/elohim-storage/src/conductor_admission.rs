//! CONDUCTOR ADMISSION — the capacity contract over the conductor's DB read pool.
//!
//! ## Why this exists
//!
//! Every other pacing knob in this crate sits on the OFFERING side of the
//! conductor seam: batch size ([`crate::p2p::reconcile_rails::AdaptiveBatchBudget`]),
//! per-leg budgets, fan-out ceilings, circuit thresholds, CPU limits. The thing
//! that is actually SCARCE lives on the other side — the conductor's DB read
//! permit pool, sized `calculate_default_db_max_readers = max(2*cpus, 8)`. We
//! computed that number at boot and exported it as
//! `elohim_node_db_max_readers`, and then bounded nothing with it.
//!
//! The consequence was structural, not incidental. Four reconcile arms and a
//! dozen HTTP paths each dispatched into the conductor with no knowledge of one
//! another, so the pool's limit could only ever be discovered by TIMING OUT
//! against it — by which point the cost has already been paid.
//!
//! ## This is admission, NOT a caller-side timeout on `call_zome`
//!
//! The distinction is the whole design, and the T13 capability spike
//! (`genesis/docs/superpowers/specs/2026-08-08-conductor-call-deadline-capability-spike.md`
//! §2) is precise about why. Bolting a timeout onto an in-flight
//! [`crate::hc_client::HcClient::call_zome`] is NOT equivalent to refusing the
//! call, in either direction:
//!
//! - Most of a slow call's wall-clock IS cancellable — permit acquisition inside
//!   the conductor is a plain tokio `Semaphore::acquire`, so abandoning the
//!   future does give that queue slot back. But an already-running **WASM body
//!   runs to completion regardless** (`spawn_blocking`), so a caller timeout can
//!   never mean "it did not happen" — only "I stopped waiting". The work, and
//!   the answer, are lost while the cost is not.
//! - Either way the call must be re-offered, so a timeout paces nothing. It
//!   reacts to saturation by adding a retry to it.
//!
//! This gate never abandons a dispatched call. Its bound is on ACQUIRING LOCAL
//! CAPACITY, before anything crosses the websocket. Past the gate, the call is
//! unbounded on our side exactly as it is today — bounded on the far side by the
//! extern's own in-wasm deadline, which is where a conductor-touching surface is
//! supposed to bound its work (see
//! [`crate::services::head_batch_resolver::BATCH_EXTERN_BUDGET`] and the
//! `unattempted` / `stop_reason` contract). Admission and in-wasm budgeting are
//! complementary: one bounds how much we OFFER, the other bounds what one offer
//! COSTS. Neither substitutes for the other, and this module adds no timeout to
//! any call that has been dispatched.
//!
//! A shed therefore costs the conductor nothing at all, and — unlike a timeout —
//! it is legible: the caller knows the work was never started.
//!
//! ## What it makes measurable
//!
//! Little's Law needs `L = λW`. The pre-existing instrumentation had capacity
//! (`elohim_node_db_max_readers`) and arrival (`elohim_head_batch_queue_wait_ms`),
//! and never occupancy or hold-time — so `L`, the number that says "the pool is
//! full", was not computable from anything we emitted. Gating every call through
//! one semaphore makes both fall out for free:
//!
//! - `elohim_conductor_admission_in_flight` — **L**, occupancy. THE number.
//! - `elohim_conductor_admission_hold_ms` — **W**, how long a call holds capacity.
//! - `elohim_conductor_admission_wait_ms` — queueing delay AT the gate, which
//!   (unlike `RTT − in-wasm elapsed_ms`) is monotonic in the offered load, and is
//!   therefore a legitimate error signal for a controller.
//!
//! ## Sizing — borrowed from the conductor, not guessed
//!
//! The conductor already answers "how many concurrent requests should be
//! admitted against this read pool?" for the analogous path: network authority
//! responses are throttled by `incoming_request_concurrency_limit`, whose
//! default is **`db_max_readers − 3`** (T13 spike §2, citing the fork's
//! `config/conductor.rs:142-167`). It reserves three permits so the pool's own
//! keeper is never starved by its callers.
//!
//! The same spike names the gap this module closes: *"There is no equivalent
//! ceiling for app-interface zome calls; `CONCURRENCY_COUNT = 128` is a
//! per-connection constant, not a conductor policy, and a client can raise its
//! own effective limit by opening more websockets."* We are that client, and
//! opening more websockets is precisely what a registry of per-role clients
//! does. So the default here mirrors the conductor's own policy rather than
//! inventing a fraction — the reserve is a borrowed constant with a citation,
//! not a hedge.
//!
//! Two caveats are honest to state and neither is fixable from here: one zome
//! call is not exactly one read permit (an extern may take several in sequence),
//! and the conductor spends permits on its own gossip/validation/publish work
//! that this gate cannot see. So the gate is an ADMISSION approximation, not an
//! accounting of the pool — it bounds our contribution to the pool's load, which
//! is the only half we own. Both knobs are env-overridable so a live node can be
//! retuned without a rebuild.
//!
//! ## The sizing test — and the half of it this doc originally got wrong
//!
//! This module used to say: if `in_flight` sits pinned at capacity with real
//! demand queued behind it, THAT is the evidence-backed case for raising
//! `db_max_readers`. Measured against a live conductor (2026-08-18, local island
//! peer, `app/elohim-app/scripts/hc-admission-probe.py`), that inference is
//! **necessary but not sufficient**, and acting on it alone makes things worse.
//!
//! Occupancy pinned at capacity only says the GATE is the binding constraint. It
//! cannot distinguish "the pool has headroom we are refusing to use" from "the
//! pool is already past its knee". The sufficient test is a second run at a
//! larger [`ENV_PERMITS`], asking whether ARRIVAL RATE rises:
//!
//! ```text
//!   capacity 17 -> 34, same offered load:  lambda 924/s -> 897/s   (flat)
//!                                          W      18.2ms -> 37.6ms (x2.06)
//! ```
//!
//! Throughput did not move; hold-time scaled with the permits. That is a
//! saturated downstream absorbing concurrency as queueing — the pool was
//! OVERSUBSCRIBED at 17, not undersized, and doubling it bought only latency.
//! So: `in_flight == capacity` is where the investigation starts, and
//! `d(lambda)/d(capacity) > 0` is what has to hold before anyone raises
//! `db_max_readers` in the conductor fork. Little's Law itself held throughout
//! (|L - lambda*W| under 1% across 15 runs spanning capacities 4..34), which is
//! what makes the comparison trustworthy in the first place.
//!
//! ## A shed has to be legible OUTSIDE this process too
//!
//! [`is_admission_shed`] is what lets a caller route a shed as backpressure
//! rather than as a verdict, and the reconcile arms use it. At the HTTP egress
//! that classification was missing until 2026-08-18: handlers collapsed the shed
//! into a generic "conductor unavailable" (destroying [`ADMISSION_SHED_MARKER`]),
//! and the last error seam then answered a bare `500`. A person on the other end
//! could not tell "the conductor never saw this" from "the conductor broke". The
//! shed now leaves as the catching-up `503` + `Retry-After` that storage's own
//! request-admission ceiling already emits — see
//! [`crate::services::response::admission_shed_backpressure`], which is the one
//! home for that wire shape.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::StorageError;

/// Override the permit-pool size. Set this only with occupancy evidence in hand.
pub const ENV_PERMITS: &str = "ELOHIM_CONDUCTOR_PERMITS";
/// Override how long an INTERACTIVE call waits at the gate before being shed.
pub const ENV_WAIT_MS: &str = "ELOHIM_CONDUCTOR_ADMISSION_WAIT_MS";
/// Override how long a BACKGROUND call waits at the gate before deferring.
pub const ENV_BACKGROUND_WAIT_MS: &str = "ELOHIM_CONDUCTOR_ADMISSION_BACKGROUND_WAIT_MS";

/// The conductor's own floor in `calculate_default_db_max_readers`.
pub const PERMIT_FLOOR: u32 = 8;

/// Permits left to the conductor's own keeper, mirroring the default of its
/// `incoming_request_concurrency_limit` (`db_max_readers − 3`) — the ceiling it
/// already applies to network authority responses against the same pool.
/// Borrowed rather than invented; see the module doc.
pub const CONDUCTOR_RESERVE: u32 = 3;

/// An interactive caller (an HTTP request) waits this long for capacity.
///
/// Kept at half the conductor's own 10s `ACQUIRE_TIMEOUT_MS`: waiting longer
/// than the resource's internal timeout buys nothing but a later failure.
pub const DEFAULT_INTERACTIVE_WAIT: Duration = Duration::from_secs(5);

/// A background sweep waits this long, then defers.
///
/// An order of magnitude shorter than the interactive bound, and that asymmetry
/// is the point: a reconcile arm that cannot get capacity has a correct, cheap
/// alternative — return the ids to `pending` and retry next sweep — whereas an
/// HTTP caller has only an error. Under contention the arms yield to people.
pub const DEFAULT_BACKGROUND_WAIT: Duration = Duration::from_secs(1);

/// Seconds a shed caller is told to wait before re-offering the call.
///
/// Matches storage's own request-admission ceiling (`STORAGE_SHED_RETRY_AFTER_SECS`)
/// on purpose: a caller neither can nor should tell WHICH pool shed it, so one
/// client retry rule has to cover both.
pub const SHED_RETRY_AFTER_SECS: u64 = 2;

/// Substring stamped into every shed error, so the class survives the trip
/// through [`StorageError`]'s `String` payloads and can be recognised by
/// callers that route backpressure differently from failure.
pub const ADMISSION_SHED_MARKER: &str = "conductor admission: shed";

/// Which side of the gate a caller stands on.
///
/// This is NOT a priority ordering — the semaphore is FIFO-fair and both classes
/// draw from the same pool. It selects the WAIT BOUND, and labels the metrics so
/// the interactive/background split is visible rather than inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionClass {
    /// A person is waiting on this: an HTTP handler, a signing call, an import.
    Interactive,
    /// A sweep, reconciler, or heartbeat — work with a cheap deferral path.
    Background,
}

impl AdmissionClass {
    /// Closed metric-label vocabulary.
    pub fn label(self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::Background => "background",
        }
    }
}

/// Per-class wait bounds, resolved ONCE.
///
/// Read from the environment at gate construction rather than per acquire: a
/// `std::env::var` lookup on the hot path of every zome call buys nothing (the
/// value cannot meaningfully change mid-process) and makes the bound harder to
/// reason about than a field you can print.
#[derive(Debug, Clone, Copy)]
pub struct WaitBounds {
    pub interactive: Duration,
    pub background: Duration,
}

impl WaitBounds {
    /// Defaults with any env overrides applied.
    pub fn from_env() -> Self {
        Self {
            interactive: env_duration_ms(ENV_WAIT_MS, DEFAULT_INTERACTIVE_WAIT),
            background: env_duration_ms(ENV_BACKGROUND_WAIT_MS, DEFAULT_BACKGROUND_WAIT),
        }
    }

    /// The bound for one class.
    pub fn for_class(self, class: AdmissionClass) -> Duration {
        match class {
            AdmissionClass::Interactive => self.interactive,
            AdmissionClass::Background => self.background,
        }
    }
}

impl Default for WaitBounds {
    fn default() -> Self {
        Self {
            interactive: DEFAULT_INTERACTIVE_WAIT,
            background: DEFAULT_BACKGROUND_WAIT,
        }
    }
}

/// The conductor's default read-pool size for a given CPU count —
/// `calculate_default_db_max_readers = max(2*cpus, 8)`, mirrored.
///
/// Pure and total so the sizing rule is testable without a conductor, and so
/// `main.rs` and this gate cannot drift apart on what "capacity" means.
///
/// **Contract test:** [`tests::capacity_mirrors_the_conductor_default`].
pub fn conductor_db_max_readers(cpus: u32) -> u32 {
    cpus.saturating_mul(2).max(PERMIT_FLOOR)
}

/// How many of `db_max_readers` this process will admit against —
/// `db_max_readers − CONDUCTOR_RESERVE`, floored at one permit.
///
/// Mirrors the conductor's `incoming_request_concurrency_limit` default rather
/// than inventing a fraction of the pool.
///
/// **Contract test:** [`tests::admissible_mirrors_the_conductors_own_ceiling`].
pub fn admissible_permits(db_max_readers: u32) -> u32 {
    db_max_readers.saturating_sub(CONDUCTOR_RESERVE).max(1)
}

/// The capacity this process will admit against: [`ENV_PERMITS`] when set and
/// parseable to a non-zero value, else [`admissible_permits`] of this host's
/// [`conductor_db_max_readers`].
pub fn default_capacity() -> u32 {
    if let Some(n) = env_u32(ENV_PERMITS) {
        return n;
    }
    admissible_permits(conductor_db_max_readers(
        crate::services::system_metrics::cpu_count().unwrap_or(0),
    ))
}

fn env_u32(key: &str) -> Option<u32> {
    std::env::var(key)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|n| *n > 0)
}

fn env_duration_ms(key: &str, default: Duration) -> Duration {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|ms| *ms > 0)
        .map_or(default, Duration::from_millis)
}

/// One process-wide gate in front of the conductor.
#[derive(Debug)]
pub struct ConductorAdmission {
    sem: Arc<Semaphore>,
    capacity: u32,
    bounds: WaitBounds,
    /// Monotonic count of calls shed at the gate. Cheap to read from a test or a
    /// log line without scraping Prometheus.
    shed: AtomicU64,
}

impl ConductorAdmission {
    /// Build a gate of exactly `capacity` permits with env-resolved wait bounds
    /// (clamped to at least one permit — a zero-permit gate would shed every call
    /// forever, which is never what a misconfigured env var should mean).
    pub fn with_capacity(capacity: u32) -> Self {
        Self::new(capacity, WaitBounds::from_env())
    }

    /// Build a gate with explicit bounds — the test-facing constructor, so a
    /// test never depends on ambient environment.
    pub fn new(capacity: u32, bounds: WaitBounds) -> Self {
        let capacity = capacity.max(1);
        Self {
            sem: Arc::new(Semaphore::new(capacity as usize)),
            capacity,
            bounds,
            shed: AtomicU64::new(0),
        }
    }

    /// Total permits.
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// The per-class wait bounds this gate was built with.
    pub fn bounds(&self) -> WaitBounds {
        self.bounds
    }

    /// **L** — permits currently held. Read from the semaphore rather than a
    /// separate counter so it cannot drift from the truth it describes.
    pub fn in_flight(&self) -> u32 {
        self.capacity
            .saturating_sub(u32::try_from(self.sem.available_permits()).unwrap_or(self.capacity))
    }

    /// Calls shed at the gate over this gate's lifetime.
    pub fn shed_count(&self) -> u64 {
        self.shed.load(Ordering::Relaxed)
    }

    /// Acquire capacity, or shed.
    ///
    /// On `Ok`, the returned [`AdmissionPermit`] MUST be held across the whole
    /// conductor call — dropping it early releases capacity the conductor is
    /// still spending, which is exactly the accounting error this gate exists to
    /// remove.
    ///
    /// On `Err`, nothing was dispatched: the conductor never saw this call and
    /// spent nothing on it. That is the whole reason the gate sits BEFORE the
    /// call rather than after it.
    pub async fn acquire(
        &self,
        class: AdmissionClass,
        zome: &str,
    ) -> Result<AdmissionPermit, StorageError> {
        let started = Instant::now();
        // Sampled BEFORE queueing: "did this call arrive to a full pool?" is a
        // different question from "was it eventually made to wait", and only the
        // first one distinguishes a saturated gate from a merely busy one.
        let arrived_saturated = self.sem.available_permits() == 0;

        let permit = match tokio::time::timeout(
            self.bounds.for_class(class),
            Arc::clone(&self.sem).acquire_owned(),
        )
        .await
        {
            Ok(Ok(permit)) => permit,
            // The semaphore is never closed in this process; treat it as a
            // conductor-unavailable condition rather than panicking a handler.
            Ok(Err(_closed)) => {
                return Err(StorageError::Conductor(
                    "conductor admission: permit pool closed".to_string(),
                ))
            }
            Err(_elapsed) => {
                self.shed.fetch_add(1, Ordering::Relaxed);
                crate::metrics::inc_admission_shed(class.label(), zome);
                return Err(self.shed_error(class, zome));
            }
        };

        let wait = started.elapsed();
        crate::metrics::observe_admission_acquire(class.label(), wait, arrived_saturated);

        Ok(AdmissionPermit {
            _permit: permit,
            zome: zome.to_string(),
            wait,
            held_since: Instant::now(),
        })
    }

    fn shed_error(&self, class: AdmissionClass, zome: &str) -> StorageError {
        StorageError::Timeout(format!(
            "{ADMISSION_SHED_MARKER}: no conductor permit for {zome} within {}ms \
             (class={}, capacity={}, in_flight={}) — nothing was dispatched",
            self.bounds.for_class(class).as_millis(),
            class.label(),
            self.capacity,
            self.in_flight(),
        ))
    }
}

/// TRUE when this error is a gate shed rather than a conductor answer.
///
/// A shed establishes NOTHING about the work — the conductor never saw it — so a
/// caller must route it exactly like backpressure (return to pending, retry with
/// backoff), never like a failure verdict.
pub fn is_admission_shed(err: &StorageError) -> bool {
    matches!(err, StorageError::Timeout(msg) if msg.contains(ADMISSION_SHED_MARKER))
}

/// Map a zome-call failure onto a handler's "unavailable" error WITHOUT
/// laundering a shed.
///
/// A conductor-admission shed passes through UNCHANGED: it carries the marker
/// [`is_admission_shed`] keys on, and collapsing it into the generic
/// "unavailable" message destroys the only signal saying the conductor never
/// saw the call — which is how a shed reached the wire as a bare 500 (measured
/// live 2026-08-18). Every handler that wraps zome-call errors must route them
/// through here (or preserve the marker itself); classification happens at the
/// egress, so a single `map_err` rewrite anywhere on the path is enough to
/// break it.
pub fn zome_unavailable_error(op: &str, e: StorageError) -> StorageError {
    if is_admission_shed(&e) {
        return e;
    }
    StorageError::Conductor(format!("{op}: conductor unavailable"))
}

/// Held capacity. Releases on drop, recording **W** (hold time) as it goes.
#[derive(Debug)]
pub struct AdmissionPermit {
    _permit: OwnedSemaphorePermit,
    zome: String,
    wait: Duration,
    held_since: Instant,
}

impl AdmissionPermit {
    /// How long this call queued at the gate.
    ///
    /// This is the signal a batch-size controller wants: it is monotonic in the
    /// offered load (bigger batches hold permits longer, so occupancy rises, so
    /// queueing rises), which `RTT − in-wasm elapsed_ms` is not — that quantity
    /// subtracts precisely the time an extern spends blocked on a read permit,
    /// so a conductor stalling INSIDE the extern reads as zero queue wait and
    /// invites the controller to ask for more.
    pub fn wait(&self) -> Duration {
        self.wait
    }

    /// How long capacity has been held so far.
    pub fn held(&self) -> Duration {
        self.held_since.elapsed()
    }
}

impl Drop for AdmissionPermit {
    fn drop(&mut self) {
        crate::metrics::observe_admission_release(&self.zome, self.held_since.elapsed());
    }
}

// ---------------------------------------------------------------------------
// Process-wide gate
// ---------------------------------------------------------------------------

static ADMISSION: OnceLock<ConductorAdmission> = OnceLock::new();

/// Size the process-wide gate. Idempotent: the FIRST caller wins, so a later
/// re-configure cannot silently resize a pool that already has permits out.
/// Returns the live capacity, which may differ from `capacity` if the gate was
/// already built (by an earlier `configure`, or lazily by [`admission`]).
pub fn configure(capacity: u32) -> u32 {
    let gate = ADMISSION.get_or_init(|| ConductorAdmission::with_capacity(capacity));
    crate::metrics::set_admission_capacity(gate.capacity());
    gate.capacity()
}

/// The process-wide gate, built at [`default_capacity`] if `configure` has not
/// run yet. Lazy on purpose: a code path that reaches the conductor before boot
/// wiring completes must still be gated, not silently ungated.
pub fn admission() -> &'static ConductorAdmission {
    ADMISSION.get_or_init(|| {
        let gate = ConductorAdmission::with_capacity(default_capacity());
        crate::metrics::set_admission_capacity(gate.capacity());
        gate
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_mirrors_the_conductor_default() {
        // max(2*cpus, 8) — the conductor's own rule, floor included.
        assert_eq!(conductor_db_max_readers(0), 8);
        assert_eq!(conductor_db_max_readers(1), 8);
        assert_eq!(conductor_db_max_readers(4), 8);
        assert_eq!(conductor_db_max_readers(5), 10);
        assert_eq!(conductor_db_max_readers(8), 16);
        // No overflow panic at the top of the range.
        assert_eq!(
            conductor_db_max_readers(u32::MAX),
            u32::MAX.saturating_mul(2).max(8)
        );
    }

    #[test]
    fn admissible_mirrors_the_conductors_own_ceiling() {
        // `incoming_request_concurrency_limit` default = db_max_readers − 3.
        assert_eq!(admissible_permits(8), 5);
        assert_eq!(admissible_permits(16), 13);
        assert_eq!(admissible_permits(32), 29);
        // The reserve never drives the gate to zero, however small the pool.
        assert_eq!(admissible_permits(3), 1);
        assert_eq!(admissible_permits(0), 1);
    }

    /// Gate with the documented default bounds, independent of ambient env.
    fn gate(capacity: u32) -> ConductorAdmission {
        ConductorAdmission::new(capacity, WaitBounds::default())
    }

    #[test]
    fn a_zero_capacity_gate_still_admits_one() {
        // A misconfigured env var must not mean "shed everything forever".
        assert_eq!(gate(0).capacity(), 1);
    }

    #[tokio::test]
    async fn occupancy_rises_with_held_permits_and_falls_on_drop() {
        let gate = gate(3);
        assert_eq!(gate.in_flight(), 0);

        let a = gate
            .acquire(AdmissionClass::Interactive, "content_store")
            .await
            .unwrap();
        assert_eq!(gate.in_flight(), 1);
        let b = gate
            .acquire(AdmissionClass::Interactive, "content_store")
            .await
            .unwrap();
        assert_eq!(gate.in_flight(), 2);

        drop(a);
        assert_eq!(gate.in_flight(), 1);
        drop(b);
        assert_eq!(gate.in_flight(), 0);
    }

    #[tokio::test]
    async fn the_gate_bounds_concurrency_at_capacity() {
        let gate = Arc::new(gate(2));
        let _a = gate
            .acquire(AdmissionClass::Interactive, "z")
            .await
            .unwrap();
        let _b = gate
            .acquire(AdmissionClass::Interactive, "z")
            .await
            .unwrap();
        assert_eq!(gate.in_flight(), 2);

        // A third caller cannot proceed while both permits are held. Probed
        // without a wall-clock sleep: `try_acquire` is the same admission
        // question asked non-blockingly.
        assert!(gate.sem.try_acquire().is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn a_full_gate_sheds_rather_than_dispatching() {
        let gate = gate(1);
        let _held = gate
            .acquire(AdmissionClass::Background, "content_store")
            .await
            .unwrap();

        // Virtual time: the background bound elapses without a real 1s wait.
        let err = gate
            .acquire(AdmissionClass::Background, "content_store")
            .await
            .expect_err("a full gate must shed, not queue forever");

        assert!(is_admission_shed(&err), "shed must be recognisable: {err}");
        assert_eq!(gate.shed_count(), 1);
        // The shed says what it did NOT do — the conductor never saw this call.
        assert!(err.to_string().contains("nothing was dispatched"));
    }

    #[tokio::test]
    async fn a_released_permit_lets_the_next_caller_in() {
        let gate = gate(1);
        {
            let _held = gate
                .acquire(AdmissionClass::Interactive, "z")
                .await
                .unwrap();
        }
        // Not shed: capacity came back when the first permit dropped.
        let second = gate.acquire(AdmissionClass::Interactive, "z").await;
        assert!(second.is_ok());
    }

    #[test]
    fn only_a_gate_shed_is_classified_as_one() {
        let shed = StorageError::Timeout(format!("{ADMISSION_SHED_MARKER}: ..."));
        assert!(is_admission_shed(&shed));
        // An ordinary conductor timeout is NOT a shed — it was dispatched, and
        // the conductor is still executing it.
        assert!(!is_admission_shed(&StorageError::Timeout(
            "Websocket error: Timeout".to_string()
        )));
        assert!(!is_admission_shed(&StorageError::Conductor(
            "Zome call failed".to_string()
        )));
    }

    #[tokio::test(start_paused = true)]
    async fn background_yields_to_interactive_under_contention() {
        // Same pool, different bounds: the class that has a cheap deferral path
        // gives up first, so the class that has a person waiting gets the permit.
        let gate = Arc::new(gate(1));
        let held = gate
            .acquire(AdmissionClass::Interactive, "z")
            .await
            .unwrap();

        let bg_gate = Arc::clone(&gate);
        let bg = tokio::spawn(async move {
            bg_gate
                .acquire(AdmissionClass::Background, "z")
                .await
                .map(|_| ())
        });

        // Past the background bound (1s), inside the interactive bound (5s).
        tokio::time::sleep(Duration::from_millis(1_500)).await;
        let bg_result = bg.await.expect("task");
        assert!(bg_result.is_err_and(|e| is_admission_shed(&e)));

        drop(held);
        assert!(gate.acquire(AdmissionClass::Interactive, "z").await.is_ok());
    }
}
