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
//!   holding it, and releases when its native response finishes. If an outer
//!   observer times out after the call is offered, that response keeps the
//!   scheduling lease; uncancellable WASM must not race the next writer.
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
//!   by `content_server::ContentServerBridge`. DEAD in the binary (re-verified
//!   2026-10-03): nothing constructs a `ContentServerBridge`, and its one
//!   consumer, `BlobStore::store_and_announce`, has no caller. Its three
//!   infrastructure writes (`register_content_server`,
//!   `update_content_server_heartbeat`, `mark_content_server_offline`) would
//!   bypass this gate AND the plane guard if it were ever wired up — and the
//!   heartbeat among them is a machine write on a timer. Route it through
//!   [`dispatch`] on that day, where it is refused unless it is added to
//!   [`PLANE_DEBT`] on purpose.
//! * **`import_handler::ImportHandler`** — LIVE (started from `main` unless
//!   `--no-import`). It answers an `ImportBatchQueued` signal by sending
//!   `process_import_chunk` over its OWN app websocket, hand-encoded, so those
//!   chain writes take neither this cell's lock nor the plane guard. (The HTTP
//!   import path, `import_api`, does go through [`dispatch`].)
//! * **Two credential mints outside the lock.** `signing::ConductorSigningClient`
//!   and `reconcile::holochain_app_signal::HolochainAppSignalStream` each call
//!   `authorize_signing_credentials` through the closed-chain fence but not
//!   through [`grant_capability_serialized`]. The fence reuses persisted
//!   credentials, so a mint is rare — but when one happens it is a `CapGrant`
//!   committed with no lock held and no plane declared.
//! * **Cross-process races.** Two storage processes on one cell (which the
//!   protocol does not do) still race, and land in the bounded retry exactly as
//!   an external writer does.
//!
//! Not a timeout: nothing dispatched is ever abandoned here.
//!
//! ## The plane rule — who is speaking, and why now
//!
//! Storage signs as the steward's own agent, so every chain write from this
//! crate is a permanent statement in the PERSON's voice. The lock above answers
//! "one at a time"; it never asked "who is speaking, and why now?", and the
//! answer cost the fleet about 50 hours of conductor call time a day
//! (`record_peer_status`, a machine write on a 60 s timer) and an idle
//! three-peer mesh 72 actions an hour on each lamad chain.
//!
//! So every write that reaches [`dispatch`] has a [`Voice`] and a [`Trigger`],
//! derived in ONE place ([`plane_of`]) from two things: the writer's identity,
//! and — for a writer whose loop runs on a clock — the evidence that THIS write
//! answers a change. Before the call is queued, let alone offered:
//!
//! * `Machine` + `Timer` is refused with [`StorageError::PlaneViolation`];
//! * an undeclared writer ([`WriterKind::Other`]) is refused in debug and test
//!   builds — a new write site cannot ship unnamed — and counted and logged
//!   once per `(zome, fn)` in release, where a label must never take a running
//!   node down;
//! * a clock-driven writer that still writes without showing a change is
//!   carried as DECLARED DEBT in [`PLANE_DEBT`]: its unwitnessed writes are let
//!   through and counted, each entry naming the work that retires it.
//!
//! ### A transition is shown, not said
//!
//! A writer says only who it is and what wakes it ([`WriterKind::standing`]).
//! A person or delegate acts; a machine woken by an event writes on that
//! event's transition. A machine woken by a CLOCK — a sweep, a tick, a retry
//! pass — cannot assert that anything changed by the fact of having run, so
//! "transition" is not something it can declare: it must hold a
//! [`ChangeWitness`], a value that can only be built from an observed
//! difference (the declared head differs from the one being declared; the
//! record was looked for and is absent; a de-dup ledger accepted the pair for
//! the first time; a period ended and has no record) and scope it around the
//! one write it justifies ([`with_change`]). A loop that would re-author an
//! unchanged row cannot produce one, and its write is a machine on a timer.
//!
//! What a witness does NOT prove: that the comparison it was built from was the
//! right one. `ChangeWitness::newly_claimed` trusts an in-process ledger that a
//! restart empties; `first_observation` is per process too. The witness moves
//! the claim from a word to a reviewed comparison at the write site — it does
//! not replace reading `elohim_chain_writes_by_plane_total` on an idle node.
//!
//! ### Series identity moved with the labels
//!
//! Naming every write site changed which `writer` series some writes land on,
//! on all of this module's series (`…serialized_wait_ms`,
//! `…head_moved_retried_total`, `…head_moved_exhausted_total`):
//!
//! * `http_author` → `reanchor` / `sweep_reauthor`: the declare inside
//!   `ContentService::update_via_conductor`, when the re-anchor backfill or the
//!   ghost-witness sweep (not an operator's PATCH) is the caller;
//! * `rea_signal` → `http_author` / `custody_rotation` / `custody_intent` /
//!   `provide`: REA commitment and economic-event writes now carry their
//!   caller's label (`rea_signal` keeps only the state-link drain);
//! * `other` → a named writer, for every site that used to be unnamed
//!   (`http_author`, `import`, `compute_delegate`, `heartbeat`, `release_soak`,
//!   `release_adoption`, `custody_intent`, `shard_registration`,
//!   `binding_mint`, `sweep_reauthor`, `reanchor`, `provide`).
//!
//! A dashboard or alert keyed on the old values reads a step change at deploy.
//!
//! Ruling: `genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md`
//! §1 (the rule), §12 row 1 (this guard).

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use holochain_client::CellId;
use tokio::time::Instant;
use tracing::Instrument;

use crate::error::StorageError;

// =============================================================================
// Writer vocabulary
// =============================================================================

/// The CLOSED vocabulary of in-process chain writers, used as the single metric
/// label on this module's series and as the ONE identity a write's voice and
/// trigger are derived from ([`WriterKind::plane`]).
///
/// Closed on purpose: a metric label that can take a content id (or any
/// unbounded value) is a cardinality bomb in a Prometheus scrape. A writer with
/// no entry here reports as [`WriterKind::Other`] — which is a GAP, not an
/// answer: `Other` declares neither voice nor trigger, so the plane guard
/// refuses its chain writes outside release builds.
///
/// Adding a variant: name it here, add it to [`WriterKind::ALL`], give it a
/// label, and declare its plane in [`WriterKind::plane`]. If that plane is
/// machine + timer the write is refused unless it is added to [`PLANE_DEBT`],
/// which a test pins — so new debt is a reviewed act, never a side effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterKind {
    /// The reconcile sweep's adopt-before-author declare leg.
    SweepAdopt,
    /// The reconcile sweep re-publishing a row whose anchor its own conductor
    /// cannot resolve (the ghost-witness heal), and the declare that completes
    /// its author-then-adopt.
    SweepReauthor,
    /// The sync-triggered head-adoption worker
    /// ([`crate::services::head_adoption_trigger`]).
    TriggerAdopt,
    /// [`crate::services::reanchor_backfill`] re-authoring a dead/NULL anchor.
    Reanchor,
    /// Provide-reconcile's commons author and its stranded-row revoke.
    Provide,
    /// The ordered-signal author path: a post-commit signal landed and the
    /// node answers it (the `CommitmentByState` link drain). NOT the REA
    /// commitment service — its writes speak in their caller's voice.
    ReaSignal,
    /// An operator-driven HTTP write (PATCH /db/content → declare, and friends).
    HttpAuthor,
    /// An operator-submitted bulk import: the batch entry and its chunks.
    Import,
    /// The node-local compute facing acting under a verified performer's
    /// capability: a delegated grant, its consent and its activation.
    ComputeDelegate,
    /// The lifecycle heartbeat (`infrastructure::record_peer_status`), every
    /// 60 s whether or not anything changed. Machine + timer: [`PLANE_DEBT`].
    Heartbeat,
    /// A release's soak attestation, authored once when its soak window closes.
    ReleaseSoak,
    /// Release adoption's own chain work: carrying, re-adopting and sealing
    /// across a DNA cutover, and the read that heals a missing id link.
    ReleaseAdoption,
    /// A `custody-blob` placement intent authored when this node selects itself
    /// as a holder (self-custody on a verified placement, salvage, and the
    /// custody-spool expansion).
    CustodyIntent,
    /// The custody rotation tick: a successor pledge authored when the blob a
    /// custody commitment names is no longer the blob its content points at.
    CustodyRotation,
    /// The background shard-assignment registration with the node registry.
    ShardRegistration,
    /// Minting this node's agent↔transport binding.
    BindingMint,
    /// The signing-credential `CapGrant`, committed over the admin socket when
    /// a cell connects without persisted credentials.
    CapGrant,
    /// The identity-declaration reconcile
    /// ([`crate::services::identity_declaration`]): the node begins the
    /// identity its person declared in a file on the node's own disk, when the
    /// file changes and only while no identity exists. Writing that file is the
    /// person's act at their own machine, exactly as `epr identity begin` there
    /// would be, so the write speaks in the person's voice.
    IdentityDeclaration,
    /// Device affirmation ([`crate::services::device_affirmation`]): this
    /// device records, on its own chain, that it saw another device of its
    /// person join. Its pass runs on a clock, so each affirmation must carry
    /// the observed absence of this device's affirmation on that joining
    /// record, or it is a machine on a timer and refused.
    DeviceAffirm,
    /// Anything that did not name itself.
    Other,
}

impl WriterKind {
    /// Every variant, for metric pre-touch.
    pub const ALL: [WriterKind; 20] = [
        WriterKind::SweepAdopt,
        WriterKind::SweepReauthor,
        WriterKind::TriggerAdopt,
        WriterKind::Reanchor,
        WriterKind::Provide,
        WriterKind::ReaSignal,
        WriterKind::HttpAuthor,
        WriterKind::Import,
        WriterKind::ComputeDelegate,
        WriterKind::Heartbeat,
        WriterKind::ReleaseSoak,
        WriterKind::ReleaseAdoption,
        WriterKind::CustodyIntent,
        WriterKind::CustodyRotation,
        WriterKind::ShardRegistration,
        WriterKind::BindingMint,
        WriterKind::CapGrant,
        WriterKind::IdentityDeclaration,
        WriterKind::DeviceAffirm,
        WriterKind::Other,
    ];

    /// The metric label. Stable wire vocabulary — do not rename without moving
    /// the dashboards that read it.
    pub fn label(self) -> &'static str {
        match self {
            WriterKind::SweepAdopt => "sweep_adopt",
            WriterKind::SweepReauthor => "sweep_reauthor",
            WriterKind::TriggerAdopt => "trigger_adopt",
            WriterKind::Reanchor => "reanchor",
            WriterKind::Provide => "provide",
            WriterKind::ReaSignal => "rea_signal",
            WriterKind::HttpAuthor => "http_author",
            WriterKind::Import => "import",
            WriterKind::ComputeDelegate => "compute_delegate",
            WriterKind::Heartbeat => "heartbeat",
            WriterKind::ReleaseSoak => "release_soak",
            WriterKind::ReleaseAdoption => "release_adoption",
            WriterKind::CustodyIntent => "custody_intent",
            WriterKind::CustodyRotation => "custody_rotation",
            WriterKind::ShardRegistration => "shard_registration",
            WriterKind::BindingMint => "binding_mint",
            WriterKind::CapGrant => "cap_grant",
            WriterKind::IdentityDeclaration => "identity_declaration",
            WriterKind::DeviceAffirm => "device_affirm",
            WriterKind::Other => "other",
        }
    }

    /// Whose voice this writer speaks in, and what DRIVES the code that calls
    /// it. `None` is the undeclared writer.
    ///
    /// This is the only thing a writer may say about itself. It is a fact about
    /// the code's shape — is there a person at a door, an event that arrived, or
    /// a clock — and it deliberately stops short of the trigger: a clock-driven
    /// writer cannot declare "transition" here. It has to show one
    /// ([`ChangeWitness`]).
    pub const fn standing(self) -> Option<(Voice, Drive)> {
        use Drive::{Clock, Event};
        use Voice::{Delegate, Machine, Person};
        Some(match self {
            // A person did something, at an HTTP door they were authenticated at,
            // or in a declaration file on their own node's disk.
            WriterKind::HttpAuthor | WriterKind::Import | WriterKind::IdentityDeclaration => {
                (Person, Event)
            }
            // A performer acting under a capability the person scoped.
            WriterKind::ComputeDelegate => (Delegate, Event),
            // The node, woken BY the change itself: a peer's head hint arrived,
            // a post-commit signal landed, a blob PUT queued a registration, a
            // boot found no binding, a connect found no credentials. No clock
            // can fire these twice for one event.
            WriterKind::TriggerAdopt
            | WriterKind::ReaSignal
            | WriterKind::ShardRegistration
            | WriterKind::BindingMint
            | WriterKind::CapGrant => (Machine, Event),
            // The node, woken by a CLOCK: a sweep, a tick, a retry pass. Whether
            // anything changed is exactly what such a loop cannot assert by
            // being run, so each write must carry the evidence or be a timer.
            WriterKind::SweepAdopt
            | WriterKind::SweepReauthor
            | WriterKind::Reanchor
            | WriterKind::Provide
            | WriterKind::CustodyIntent
            | WriterKind::CustodyRotation
            | WriterKind::ReleaseAdoption
            | WriterKind::ReleaseSoak
            | WriterKind::DeviceAffirm
            | WriterKind::Heartbeat => (Machine, Clock),
            WriterKind::Other => return None,
        })
    }

    /// This writer's plane with NO evidence of change in hand: what a
    /// clock-driven writer is until it shows otherwise.
    pub const fn plane(self) -> Option<Plane> {
        plane_of(self, None)
    }
}

/// THE one place a chain write's voice and trigger are derived.
///
/// * a person or a delegate acts;
/// * an event-driven machine writes on the transition that woke it;
/// * a clock-driven machine writes on a transition or a period close ONLY when
///   it holds a [`ChangeWitness`] for this write — otherwise the clock is the
///   only reason it is writing, and the trigger is `Timer`.
pub const fn plane_of(writer: WriterKind, change: Option<ChangeWitness>) -> Option<Plane> {
    let Some((voice, drive)) = writer.standing() else {
        return None;
    };
    let trigger = match (voice, drive, change) {
        (Voice::Person | Voice::Delegate, _, _) => Trigger::Act,
        (Voice::Machine, Drive::Event, _) => Trigger::Transition,
        (Voice::Machine, Drive::Clock, Some(witness)) => witness.trigger,
        (Voice::Machine, Drive::Clock, None) => Trigger::Timer,
    };
    Some(Plane { voice, trigger })
}

// =============================================================================
// The plane rule
// =============================================================================

/// Whose statement a chain write is. Closed vocabulary — see [`WriterKind`] for
/// why every label on this module's series is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voice {
    /// A human act: the steward or an operator did this, deliberately.
    Person,
    /// Acting under a person's scoped grant.
    Delegate,
    /// The node observing, repairing, obeying or nominating on its own account.
    Machine,
}

impl Voice {
    pub const fn label(self) -> &'static str {
        match self {
            Voice::Person => "person",
            Voice::Delegate => "delegate",
            Voice::Machine => "machine",
        }
    }
}

/// What wakes the code that calls a writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    /// A request, a signal, a queued job: something happened and this is the
    /// answer to it.
    Event,
    /// A sweep, a tick, a retry pass: a clock fired.
    Clock,
}

/// Why a chain write is happening NOW. Closed vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// Someone did something.
    Act,
    /// A state changed.
    Transition,
    /// A declared period ended.
    PeriodClose,
    /// A clock fired and nothing changed.
    Timer,
}

impl Trigger {
    pub const fn label(self) -> &'static str {
        match self {
            Trigger::Act => "act",
            Trigger::Transition => "transition",
            Trigger::PeriodClose => "period_close",
            Trigger::Timer => "timer",
        }
    }
}

/// A chain write's declaration: who is speaking, and why now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plane {
    pub voice: Voice,
    pub trigger: Trigger,
}

impl Plane {
    /// The anti-pattern the guard exists for: a machine writing on a timer.
    pub const fn is_machine_timer(self) -> bool {
        matches!((self.voice, self.trigger), (Voice::Machine, Trigger::Timer))
    }
}

/// Label used for the voice and the trigger of a writer that declared neither.
pub const UNDECLARED: &str = "undeclared";

// =============================================================================
// Evidence of change
// =============================================================================

/// Evidence that THIS write answers a change — the only thing that makes a
/// clock-driven machine's write a transition (or a period close) and not a
/// timer.
///
/// The fields are private and there is no "it changed, trust me" constructor.
/// Every constructor takes the two things it compares, or performs the ledger
/// insert itself, and returns `None` when they do not differ. A loop that would
/// re-author an unchanged row therefore cannot obtain one; its write reaches
/// the gate without a witness and is a machine on a timer.
///
/// Attach it to the ONE write it justifies with [`with_change`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeWitness {
    trigger: Trigger,
    evidence: &'static str,
}

/// Process-wide cap on [`ChangeWitness::first_observation`]'s ledger. Past it a
/// NEW key is not witnessed (the write falls to `Timer` and is counted) — the
/// ledger never clears itself into re-witnessing what it already saw.
const OBSERVATION_LEDGER_CAP: usize = 65_536;

fn observation_ledger() -> &'static Mutex<std::collections::HashSet<(&'static str, String)>> {
    static LEDGER: OnceLock<Mutex<std::collections::HashSet<(&'static str, String)>>> =
        OnceLock::new();
    LEDGER.get_or_init(|| Mutex::new(std::collections::HashSet::new()))
}

impl ChangeWitness {
    /// What was compared, for logs.
    pub const fn evidence(self) -> &'static str {
        self.evidence
    }

    /// The value this write sets differs from the value OBSERVED to stand.
    /// `observed = None` means the read succeeded and found nothing standing; a
    /// read that failed is not an observation, and its caller has no witness.
    pub fn value_moves(
        evidence: &'static str,
        observed: Option<&str>,
        written: &str,
    ) -> Option<Self> {
        match observed {
            Some(current) if current.trim() == written.trim() => None,
            _ => Some(ChangeWitness {
                trigger: Trigger::Transition,
                evidence,
            }),
        }
    }

    /// The record this write creates was looked for and is not there.
    pub fn absent<T>(evidence: &'static str, observed: Option<&T>) -> Option<Self> {
        match observed {
            Some(_) => None,
            None => Some(ChangeWitness {
                trigger: Trigger::Transition,
                evidence,
            }),
        }
    }

    /// A de-duplication ledger just accepted this exact `(subject, target)` for
    /// the first time in its window. `claimed` MUST be the return value of the
    /// ledger's own insert (`head_adoption::claim_candidacy` and its sibling),
    /// never a literal.
    pub fn newly_claimed(evidence: &'static str, claimed: bool) -> Option<Self> {
        claimed.then_some(ChangeWitness {
            trigger: Trigger::Transition,
            evidence,
        })
    }

    /// This process has not already written against this same observation.
    ///
    /// For a repair loop whose candidate test stays true until the repair's own
    /// success is recorded (a NULL or dead anchor, a row the conductor answers
    /// empty for). The FIRST write against `(evidence, key)` answers a change;
    /// a second one against the unchanged observation is the loop re-authoring
    /// what it already authored, and gets no witness. Call
    /// [`ChangeWitness::settle_observation`] when the repair is recorded, so a
    /// later, genuinely new occurrence is witnessed again.
    pub fn first_observation(evidence: &'static str, key: &str) -> Option<Self> {
        let mut ledger = match observation_ledger().lock() {
            Ok(guard) => guard,
            // A poisoned ledger cannot show anything; no witness.
            Err(_) => return None,
        };
        if ledger.len() >= OBSERVATION_LEDGER_CAP {
            return None;
        }
        ledger
            .insert((evidence, key.to_string()))
            .then_some(ChangeWitness {
                trigger: Trigger::Transition,
                evidence,
            })
    }

    /// The repair against `(evidence, key)` was recorded; forget the
    /// observation.
    pub fn settle_observation(evidence: &'static str, key: &str) {
        if let Ok(mut ledger) = observation_ledger().lock() {
            ledger.remove(&(evidence, key.to_string()));
        }
    }

    /// A declared period has ended and no record closing it exists yet.
    /// Timestamps are microseconds; `record_exists` is the result of a read.
    pub fn period_closed(
        evidence: &'static str,
        period_end_micros: i64,
        now_micros: i64,
        record_exists: bool,
    ) -> Option<Self> {
        (now_micros >= period_end_micros && !record_exists).then_some(ChangeWitness {
            trigger: Trigger::PeriodClose,
            evidence,
        })
    }
}

// =============================================================================
// Declared debt
// =============================================================================

/// One clock-driven writer whose unwitnessed writes the gate tolerates, and
/// what retires it.
#[derive(Debug, Clone, Copy)]
pub struct PlaneDebt {
    pub writer: WriterKind,
    /// The spec section, and the concrete work, whose landing removes this entry.
    pub retired_by: &'static str,
}

/// DECLARED DEBT: clock-driven writers that still write without showing a
/// change.
///
/// A listed writer's WITNESSED writes pass as transitions like anyone's. Its
/// UNWITNESSED writes — the ones only the clock explains — are let through and
/// counted (`elohim_chain_write_plane_violation_tolerated_total{writer}`), so
/// the debt is a number on a scrape, per writer, on an idle node. A clock-driven
/// writer that is NOT listed is refused the moment it writes without a witness.
///
/// An entry leaves when the work it names lands; `the_plane_debt_allowlist_is_pinned`
/// fails if one is added without that test being edited.
pub const PLANE_DEBT: &[PlaneDebt] = &[
    // `infrastructure::record_peer_status`, every 60 s (`heartbeat.rs`). It IS
    // the anti-pattern, and it cannot be refused yet: shard placement
    // (`services/peer_selection.rs`), the resilience card
    // (`services/household_resilience.rs`, `api/resilience.rs`) and network
    // posture (`api/network_posture.rs`) all read its timestamp as liveness
    // through `services/peer_status_fanout.rs`. Plane-separation §5.5 rules the
    // order: arm substrate liveness on iroh, move those three readers, and only
    // then change the writer — refusing it first puts every healthy peer out of
    // placement after 900 s. §11 retires the timer as a liveness carrier.
    PlaneDebt {
        writer: WriterKind::Heartbeat,
        retired_by: "plane-separation §5.5 (the heartbeat splits: posture on change, liveness \
                     on substrate presence, consumers moved first); retired per §11",
    },
    // `reanchor_backfill::run_once` — reached from the boot/paced re-anchor loop
    // and from `projection_reconcile::witness_bootstrap` on every heal sweep. It
    // re-authors every row whose anchor is NULL or dead, and the row stays a
    // candidate until the stamp recording the repair lands. When that stamp
    // fails (`database is locked`) the SAME rows are re-authored on the next
    // admitted cycle: `idle-is-free` measured 46 device-health attestations
    // re-authored six times over (276 action hashes) on a ~315 s period. The
    // first re-author against an observed NULL/dead anchor carries a witness;
    // every repeat against the unchanged observation is counted here.
    PlaneDebt {
        writer: WriterKind::Reanchor,
        retired_by: "plane-separation §6.1 (attestations leave re-authoring: the 46 rows are \
                     attestations and must never enter this loop) and §7 (storage `database \
                     is locked`: the BEGIN IMMEDIATE design, so the stamp that ends the loop \
                     lands)",
    },
    // The ghost-witness sweep (`projection_reconcile`): re-publishes a row its
    // own conductor answered EMPTY for. It preserves the existing declaration,
    // so a successful re-author does not change that answer, and the candidate
    // test stays true until the miss ledger's dormancy expires. First re-author
    // per observation is witnessed; repeats are counted here.
    PlaneDebt {
        writer: WriterKind::SweepReauthor,
        retired_by: "plane-separation §6.1 (one kind-to-lane function: an immutable claim never \
                     enters re-authoring) and §6.4 (a machine caller that would re-assert a \
                     superseded head is refused)",
    },
    // Every sweep declare in `head_adoption` carries a witness EXCEPT
    // `declare_peer_head` when the read of the row's declared head is
    // unavailable (no pooled connection, or the read errors): the declare-storm
    // gate is skipped and the node may re-declare a head it already declares.
    // Refusing there would change behaviour, so the unwitnessed declare is
    // counted instead.
    PlaneDebt {
        writer: WriterKind::SweepAdopt,
        retired_by: "plane-separation §6.4 (the declare guard keys on the standing election, \
                     read from the conductor, so the guard no longer depends on a storage \
                     read that can be unavailable)",
    },
    // `provide_reconcile` → `conductor_commitment_author`: authors a
    // `replicates-commons` commitment per desired pin with no live row, and a
    // `revokes-commitment` per stranded row. The diff IS an observed
    // difference, but it is computed two modules above the write and no witness
    // is built from it yet.
    PlaneDebt {
        writer: WriterKind::Provide,
        retired_by: "plane-separation §12 row 1, follow-up: build `ChangeWitness::absent` from \
                     the reconciler's no-live-row test (author) and `value_moves` from the \
                     stranded row's `revoked_at` (revoke), and carry them on the request",
    },
    // The salvage pass and the custody-spool expansion each author a
    // `custody-blob` intent from a sweep. (Self-custody on a verified placement
    // is witnessed.) Their selection is an observed absence that is not yet
    // turned into a witness at the write.
    PlaneDebt {
        writer: WriterKind::CustodyIntent,
        retired_by: "plane-separation §12 row 1, follow-up: `ChangeWitness::absent` from the \
                     salvage pass's and the spool pass's own commitment lookup for the \
                     deterministic custody id",
    },
    // Release adoption's tick: one carry / re-adopt page per tick while a
    // cutover is open, the seal, and the `get_content_by_id` read whose healing
    // fallback commits. A page with nothing left to carry should write nothing.
    PlaneDebt {
        writer: WriterKind::ReleaseAdoption,
        retired_by: "plane-separation §12 row 1, follow-up: `ChangeWitness::value_moves` from \
                     the carry receipt's cursor (a page is offered only when the cursor \
                     advanced), and §6.1 for the attestation read that heals by committing",
    },
    // The soak attestation is authored when a soak window ends, which is a
    // period close — but nothing reads whether THIS release already has this
    // peer's soak record before authoring another.
    PlaneDebt {
        writer: WriterKind::ReleaseSoak,
        retired_by: "plane-separation §5.2 and §11 (a first-class soak attestation kind at the \
                     recast): `ChangeWitness::period_closed` from the soak window's end and a \
                     `get_attestations_for_subject` read for this issuer",
    },
];

/// What the gate does with a chain write from a writer that never named itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndeclaredPosture {
    /// Debug and test builds: refuse, so a new write site cannot ship unnamed.
    Refuse,
    /// Release builds: count and log, and let the write through — a missing
    /// label must never break a running node.
    Count,
}

/// The operator override for [`UndeclaredPosture`].
pub const UNDECLARED_POSTURE_ENV: &str = "ELOHIM_PLANE_UNDECLARED_WRITER";

impl UndeclaredPosture {
    /// The posture a build defaults to: [`Self::Refuse`] with debug assertions,
    /// [`Self::Count`] without.
    pub const fn for_build() -> Self {
        if cfg!(debug_assertions) {
            UndeclaredPosture::Refuse
        } else {
            UndeclaredPosture::Count
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            UndeclaredPosture::Refuse => "refuse",
            UndeclaredPosture::Count => "count",
        }
    }

    /// Parse the override: `refuse` or `count`, any case, surrounding
    /// whitespace ignored. `Ok(None)` is "not set"; `Err` carries a value that
    /// was set and is not one of the two words, so the caller can say so
    /// instead of silently running the build default.
    pub fn parse_override(value: Option<&str>) -> Result<Option<Self>, String> {
        let Some(raw) = value else {
            return Ok(None);
        };
        let word = raw.trim();
        if word.eq_ignore_ascii_case("refuse") {
            Ok(Some(UndeclaredPosture::Refuse))
        } else if word.eq_ignore_ascii_case("count") {
            Ok(Some(UndeclaredPosture::Count))
        } else {
            Err(raw.to_string())
        }
    }

    /// Resolve the posture from an override value, reporting what happened.
    fn resolve(value: Option<&str>) -> (Self, PostureSource) {
        match Self::parse_override(value) {
            Ok(Some(posture)) => (posture, PostureSource::Override),
            Ok(None) => (Self::for_build(), PostureSource::BuildDefault),
            Err(_) => (Self::for_build(), PostureSource::UnrecognisedOverride),
        }
    }
}

/// Where the running posture came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostureSource {
    BuildDefault,
    Override,
    /// The override was set to something that is not `refuse` or `count`; the
    /// build default is in force and a warning was logged.
    UnrecognisedOverride,
}

/// The rule as data, so the decision can be exercised without building release.
#[derive(Debug, Clone, Copy)]
pub struct PlanePolicy {
    pub debt: &'static [PlaneDebt],
    pub undeclared: UndeclaredPosture,
}

impl PlanePolicy {
    /// The policy this process runs: [`PLANE_DEBT`], and the undeclared posture
    /// resolved ONCE (an operator override must not be re-read per write, and a
    /// test must not be able to flip it under a running one).
    ///
    /// The first call — `metrics::register_all` makes it at boot — logs the
    /// resolved posture, warns if the override was set to something
    /// unrecognised, and publishes it on
    /// `elohim_chain_write_undeclared_posture{posture}`.
    pub fn current() -> Self {
        static UNDECLARED_POSTURE: OnceLock<UndeclaredPosture> = OnceLock::new();
        let undeclared = *UNDECLARED_POSTURE.get_or_init(|| {
            let raw = std::env::var(UNDECLARED_POSTURE_ENV).ok();
            let (posture, source) = UndeclaredPosture::resolve(raw.as_deref());
            if source == PostureSource::UnrecognisedOverride {
                tracing::warn!(
                    env = UNDECLARED_POSTURE_ENV,
                    value = raw.as_deref().unwrap_or_default(),
                    in_force = posture.label(),
                    "chain_write_gate: UNRECOGNISED undeclared-writer posture override — it \
                     accepts `refuse` or `count` and nothing else. The build default is in \
                     force; the override did NOT take effect."
                );
            }
            tracing::info!(
                posture = posture.label(),
                source = ?source,
                debt_writers = PLANE_DEBT.len(),
                "chain_write_gate: plane guard armed — an undeclared writer's chain write is \
                 {} in this process",
                match posture {
                    UndeclaredPosture::Refuse => "REFUSED",
                    UndeclaredPosture::Count => "counted and let through",
                }
            );
            crate::metrics::set_chain_write_undeclared_posture(posture.label());
            posture
        });
        PlanePolicy {
            debt: PLANE_DEBT,
            undeclared,
        }
    }
}

/// The gate's answer for one chain write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaneVerdict {
    /// Declared, and not a machine on a timer.
    Pass(Plane),
    /// A machine on a timer that [`PLANE_DEBT`] names: let through, counted.
    ToleratedDebt(Plane),
    /// A machine on a timer nobody declared as debt: refused.
    RefusedMachineTimer(Plane),
    /// Undeclared, release posture: let through, counted, logged once.
    UndeclaredCounted,
    /// Undeclared, debug/test posture: refused.
    UndeclaredRefused,
}

impl PlaneVerdict {
    /// Is this write kept from the conductor?
    pub const fn is_refused(self) -> bool {
        matches!(
            self,
            PlaneVerdict::RefusedMachineTimer(_) | PlaneVerdict::UndeclaredRefused
        )
    }
}

/// THE decision. Pure: no metric, no log, no environment.
pub fn plane_verdict(
    writer: WriterKind,
    change: Option<ChangeWitness>,
    policy: &PlanePolicy,
) -> PlaneVerdict {
    let Some(plane) = plane_of(writer, change) else {
        return match policy.undeclared {
            UndeclaredPosture::Refuse => PlaneVerdict::UndeclaredRefused,
            UndeclaredPosture::Count => PlaneVerdict::UndeclaredCounted,
        };
    };
    if !plane.is_machine_timer() {
        return PlaneVerdict::Pass(plane);
    }
    if policy.debt.iter().any(|debt| debt.writer == writer) {
        PlaneVerdict::ToleratedDebt(plane)
    } else {
        PlaneVerdict::RefusedMachineTimer(plane)
    }
}

/// The rule, in full, for the LOG line of a refusal. The error a caller
/// receives stays short and stable (`StorageError::PlaneViolation`'s `Display`)
/// and does not carry a repository path into an HTTP response body.
pub const PLANE_RULE: &str = "A person's source chain speaks in the person's voice. A machine \
     write on a timer is refused, and every chain write must name its writer \
     (`chain_write_gate::as_writer(WriterKind::…)`) so its voice (person, delegate, machine) and \
     trigger (act, transition, period close, timer) are declared; a clock-driven writer shows \
     its transition with a `ChangeWitness`. See \
     genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md §1 and §12.";

/// The bounded `reason` label on the refused series.
pub const REFUSED_MACHINE_TIMER: &str = "machine_timer";
pub const REFUSED_UNDECLARED: &str = "undeclared";

fn plane_violation(
    writer: WriterKind,
    plane: Option<Plane>,
    zome: &str,
    fn_name: &str,
) -> StorageError {
    StorageError::PlaneViolation {
        writer: writer.label().to_string(),
        voice: plane.map_or(UNDECLARED, |p| p.voice.label()).to_string(),
        trigger: plane.map_or(UNDECLARED, |p| p.trigger.label()).to_string(),
        zome: zome.to_string(),
        fn_name: fn_name.to_string(),
    }
}

/// Enact [`plane_verdict`] for one chain write by `writer`: count it, log what
/// needs logging, and refuse what the rule refuses. Called BEFORE the write is
/// queued, so a refusal never holds the chain lock and never reaches the
/// conductor.
fn enforce_plane(
    writer: WriterKind,
    change: Option<ChangeWitness>,
    zome: &str,
    fn_name: &str,
    policy: &PlanePolicy,
) -> Result<(), StorageError> {
    match plane_verdict(writer, change, policy) {
        PlaneVerdict::Pass(plane) => {
            crate::metrics::inc_chain_write_by_plane(
                writer.label(),
                plane.voice.label(),
                plane.trigger.label(),
            );
            Ok(())
        }
        PlaneVerdict::ToleratedDebt(plane) => {
            crate::metrics::inc_chain_write_by_plane(
                writer.label(),
                plane.voice.label(),
                plane.trigger.label(),
            );
            crate::metrics::inc_plane_violation_tolerated(writer.label());
            Ok(())
        }
        PlaneVerdict::RefusedMachineTimer(plane) => {
            crate::metrics::inc_plane_violation_refused(writer.label(), REFUSED_MACHINE_TIMER);
            if refusal_log_due(zome, fn_name) {
                tracing::error!(
                    writer = writer.label(),
                    voice = plane.voice.label(),
                    trigger = plane.trigger.label(),
                    zome = %zome,
                    fn_name = %fn_name,
                    rule = PLANE_RULE,
                    "chain_write_gate: PLANE VIOLATION — a machine write on a timer was refused \
                     before it reached the conductor (logged at most once a minute per extern; \
                     every refusal is counted)"
                );
            }
            Err(plane_violation(writer, Some(plane), zome, fn_name))
        }
        PlaneVerdict::UndeclaredCounted => {
            crate::metrics::inc_chain_write_by_plane(writer.label(), UNDECLARED, UNDECLARED);
            crate::metrics::inc_chain_write_undeclared_writer(fn_name);
            warn_undeclared_writer(zome, fn_name);
            Ok(())
        }
        PlaneVerdict::UndeclaredRefused => {
            crate::metrics::inc_plane_violation_refused(writer.label(), REFUSED_UNDECLARED);
            if refusal_log_due(zome, fn_name) {
                tracing::error!(
                    zome = %zome,
                    fn_name = %fn_name,
                    rule = PLANE_RULE,
                    "chain_write_gate: PLANE VIOLATION — an undeclared writer's chain write was \
                     refused. Wrap the write site in `chain_write_gate::as_writer(WriterKind::…)` \
                     (logged at most once a minute per extern; every refusal is counted)"
                );
            }
            Err(plane_violation(writer, None, zome, fn_name))
        }
    }
}

/// How often one `(zome, fn)` may log a refusal. A missed write site inside a
/// per-row sweep refuses once per row; the counter carries the volume, the log
/// carries the name.
const REFUSAL_LOG_INTERVAL: Duration = Duration::from_secs(60);

/// Should this refusal be logged? True for the first refusal of `(zome, fn)` and
/// then at most once per [`REFUSAL_LOG_INTERVAL`]. Bounded by the number of
/// distinct externs the binary can name.
fn refusal_log_due(zome: &str, fn_name: &str) -> bool {
    static LAST: OnceLock<Mutex<HashMap<(String, String), std::time::Instant>>> = OnceLock::new();
    refusal_log_due_at(
        LAST.get_or_init(|| Mutex::new(HashMap::new())),
        zome,
        fn_name,
        std::time::Instant::now(),
    )
}

fn refusal_log_due_at(
    last: &Mutex<HashMap<(String, String), std::time::Instant>>,
    zome: &str,
    fn_name: &str,
    now: std::time::Instant,
) -> bool {
    let mut guard = match last.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let key = (zome.to_string(), fn_name.to_string());
    match guard.get(&key) {
        Some(at) if now.saturating_duration_since(*at) < REFUSAL_LOG_INTERVAL => false,
        _ => {
            guard.insert(key, now);
            true
        }
    }
}

/// WARN once per process per `(zome, fn)` an undeclared writer reached. Bounded
/// by the number of distinct externs the binary can name.
fn warn_undeclared_writer(zome: &str, fn_name: &str) {
    static SEEN: OnceLock<Mutex<std::collections::BTreeSet<(String, String)>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(std::collections::BTreeSet::new()));
    let mut guard = match seen.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if guard.insert((zome.to_string(), fn_name.to_string())) {
        tracing::warn!(
            zome = %zome,
            fn_name = %fn_name,
            "chain_write_gate: an UNDECLARED writer committed to a person's source chain. Wrap \
             the write site in `chain_write_gate::as_writer(WriterKind::…)` — debug and test \
             builds refuse this write (plane-separation §12)"
        );
    }
}

tokio::task_local! {
    /// Which writer the CURRENT task is acting as.
    ///
    /// A task-local rather than a parameter because the alternative is threading
    /// a label through a dozen `conductor_writes` helpers and their callers — a
    /// crate-wide signature change for an observability field. It does NOT cross
    /// `tokio::spawn`, so a writer that spawns must set it inside the spawned
    /// future. The default is [`WriterKind::Other`], which declares neither
    /// voice nor trigger: forgetting is refused by the plane guard in debug and
    /// test builds, and counted in release.
    static WRITER_KIND: WriterKind;

    /// The evidence of change for the ONE write in scope — see
    /// [`ChangeWitness`] and [`with_change`]. Absent by default: a clock-driven
    /// writer with nothing here is a machine on a timer.
    static CHANGE: Option<ChangeWitness>;

    /// `(reached the gate's queue, offered to the conductor)` — set by
    /// [`witnessed`] so an OUTER timeout can ask whether the call it abandoned
    /// was merely parked or actually offered.
    static DISPATCH_FLAGS: (Arc<AtomicBool>, Arc<AtomicBool>);

    /// Scheduling ownership before the call reaches the websocket. A cancelled
    /// admission waiter drops it; an offered call transfers it to its response.
    static WRITE_LEASE: RefCell<Option<WriteLease>>;
}

struct WriteLease {
    _local: tokio::sync::OwnedMutexGuard<()>,
    _external: Option<File>,
}

/// A conductor call that has passed local admission, boxed. See
/// [`finish_offered_call`] for why it is never carried by value.
type OfferedCall<T> = Pin<Box<dyn Future<Output = Result<T, StorageError>> + Send>>;

/// Await the offered native response. Called after local admission, so a
/// cancelled queued request still never reaches the conductor.
///
/// # The call is boxed before any future captures it
///
/// Every HTTP request that reaches the conductor awaits this future inline, so
/// whatever it holds by value is held again by every ancestor future up to
/// `HttpServer::handle_request`, and an unoptimized build copies each ancestor's
/// child future onto that ancestor's poll frame. As an `async fn` taking the call
/// by value, this future stored the call twice, the `handle_request` future
/// measured 46,072 bytes (12,960 with the call boxed), and the debug build's
/// deepest request path needed ~2.2 MB of a 2 MiB worker stack: matthew's
/// storage aborted with "thread 'http-server' has overflowed its stack"
/// (2026-10-02). This is a plain `fn` so the box is taken at the call site,
/// before the returned future exists.
pub(crate) fn finish_offered_call<Fut, T>(
    call: Fut,
) -> impl Future<Output = Result<T, StorageError>> + Send + 'static
where
    Fut: Future<Output = Result<T, StorageError>> + Send + 'static,
    T: Send + 'static,
{
    let call: OfferedCall<T> = Box::pin(call);
    async move {
        let lease = WRITE_LEASE
            .try_with(|slot| slot.borrow_mut().take())
            .ok()
            .flatten();
        let Some(lease) = lease else {
            // Reads retain their original cancellation behavior.
            return call.await;
        };
        let response = tokio::spawn(
            as_writer(current_writer_kind(), async move {
                // Dropping the caller's JoinHandle detaches this one offered call.
                // Its lock and capacity remain occupied until the native response;
                // no retry is dispatched after the original caller disappears.
                let _lease = lease;
                call.await
            })
            .instrument(tracing::Span::current()),
        );
        response.await.map_err(|error| {
            StorageError::Internal(format!("offered native chain write task failed: {error}"))
        })?
    }
}

/// Run `fut` labelled as `kind`.
///
/// # A plain `fn`, not an `async fn` — and that is load-bearing
///
/// An `async fn` taking `fut` by value stores it once as its argument and once
/// more in the `scope` future it awaits, so the wrapper is TWICE the size of
/// the future it wraps, and an unoptimized build copies each of those onto the
/// poll frame of every ancestor. This is the shape that overflowed the
/// `http-server` worker's 2 MiB stack on 2026-10-02 (see
/// [`finish_offered_call`]), and `as_writer` now wraps the request path's
/// largest futures. Returning the `scope` future directly adds the task-local's
/// slot and nothing else; `as_writer_adds_only_its_label_to_the_future_it_wraps`
/// pins it.
pub fn as_writer<F: Future>(kind: WriterKind, fut: F) -> impl Future<Output = F::Output> {
    WRITER_KIND.scope(kind, fut)
}

/// The current task's writer kind, or [`WriterKind::Other`].
pub fn current_writer_kind() -> WriterKind {
    WRITER_KIND.try_with(|k| *k).unwrap_or(WriterKind::Other)
}

/// Run `fut` — ONE chain write — carrying the evidence that it answers a
/// change. `None` is allowed and means what it says: this site looked and
/// cannot show one, so a clock-driven writer's write here is a timer's.
///
/// Scope it as tightly as the write itself. A witness left around a loop would
/// vouch for every write in it, which is the declaration-by-word this type
/// exists to end. Plain `fn` for the reason [`as_writer`] is.
pub fn with_change<F: Future>(
    change: Option<ChangeWitness>,
    fut: F,
) -> impl Future<Output = F::Output> {
    CHANGE.scope(change, fut)
}

/// The evidence of change in scope for the current write, if any.
pub fn current_change() -> Option<ChangeWitness> {
    CHANGE.try_with(|c| *c).ok().flatten()
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
    // One action's receipt rows + one local `get_details`; commits nothing.
    // The extern (content_store/src/record_witness.rs) is a coordinator-only
    // addition still to land — see genesis/data/timeline/backlog/
    // 2026-10-29-record-witness-coordinator-extern.md; until it does, the call
    // fails and the content chrome's witness reads amber.
    "get_record_witness_facts",
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
    // Native device resolution traverses existing Human/binding records and
    // verifies controller proofs across imagodei and mishpat; none of these
    // read paths register a device, repair links, or author a commitment.
    "get_human_by_agent_key",
    "get_my_human", // → get_human_by_agent_key for the calling agent
    "get_human_root_evidence",
    "resolve_device_identity",
    "get_membership_by_action",
    "get_my_household_collective_cids",
    "list_memberships_for_collective_cid",
    "query_my_source_chain",
    "query_my_source_chain_links",
    "sign_for_agent",
    // Signs a purpose-bound account-bind challenge with `sign_raw`; commits nothing.
    "sign_account_bind_challenge",
    // --- mishpat -----------------------------------------------------------
    "get_commitment",
    "get_commitment_authority_links",
    "get_commitment_record",
    "get_commitment_state_links",
    "get_lineage_successors",
    // The device consent ceremony. `my_consent_standing` reads the Human (a
    // bridge read through `get_my_human`), the deterministic bootstrap entry and
    // its authority history; `sign_device_approval` signs with `sign_raw` and
    // commits nothing. The CapGrant that authorizes it is a separate admin-socket
    // write, serialized by `grant_capability_serialized`.
    "my_consent_standing",
    "sign_device_approval",
    // Signs possession of an enrollment intent with `sign_raw`; commits nothing.
    "sign_device_enrollment",
    // Reads the devices that speak for a person (links, verifying walks).
    "identity_devices",
    // Signs one carrier statement with `sign_raw`, under the carrier's own
    // domain; commits nothing.
    "sign_carrier_statement",
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
    // content_store::statement_contract (statement_contract.rs): returns the
    // coordinator's compiled-in statement table — "Pure constant: no host
    // calls." No commit, no link or DHT read. Asked once per distinct
    // coordinator wasm hash by the bundle proof before hosted apps receive it.
    "statement_contract",
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
    "create_human",           // Human entry + IdToHuman + AgentKeyToHuman links
    "create_self_revocation", // cross-DNA bridge → elohim chain
    "issue_attestation",
    "remove_portal_host",
    "submit_revocation_vote", // cross-DNA bridge → elohim chain
    "withdraw_membership_clean",
    // --- mishpat -----------------------------------------------------------
    "create_commitment",
    "create_commitment_state_link",
    "bootstrap_device_identity", // → notarize: one binds-identity Commitment
    "enroll_identity_device",    // → notarize: one binds-identity Commitment
    "affirm_identity_device",    // → notarize: one affirms-device Commitment + its link
    "revoke_identity_device",    // → notarize: one revokes-commitment Commitment + its links
    "register_device_identity",  // → one AgentKeyToHuman link
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

/// Optional local coordination with an independently signed ceremony process.
/// The kernel releases this lock when its owning file/process closes. It is
/// scheduling only: every conductor call still exercises its native capability.
async fn external_writer_lock(chain_key: &str) -> Result<Option<File>, StorageError> {
    let Some(directory) = std::env::var_os("ELOHIM_CHAIN_WRITE_LOCK_DIR") else {
        return Ok(None);
    };
    Ok(Some(lock_file(Path::new(&directory), chain_key).await?))
}

async fn lock_file(directory: &Path, chain_key: &str) -> Result<File, StorageError> {
    // Keys are the conductor's public DNA/agent addresses, never caller paths.
    if chain_key.is_empty()
        || !chain_key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b':' | b'-' | b'_'))
    {
        return Err(StorageError::Conductor(
            "invalid chain write lock key".into(),
        ));
    }
    std::fs::create_dir_all(directory)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join(format!("{chain_key}.lock")))?;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
    }
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
pub async fn grant_capability_serialized<F, Fut, T>(
    cell_id: &CellId,
    grant: F,
) -> Result<T, StorageError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, StorageError>>,
{
    // The grant is one writer by construction, so it names itself here rather
    // than at each connect site: a `CapGrant` is committed because a cell was
    // found without usable credentials (machine, transition). It is still put
    // to the plane guard, so it is counted with every other chain write.
    enforce_plane(
        WriterKind::CapGrant,
        None,
        ADMIN_GRANT_ZOME,
        ADMIN_GRANT_FN,
        &PlanePolicy::current(),
    )?;
    let lock = gate().lock_for(&chain_key_of(cell_id));
    let guard = lock.lock_owned().await;
    let external = external_writer_lock(&chain_key_of(cell_id)).await?;
    as_writer(
        WriterKind::CapGrant,
        WRITE_LEASE.scope(
            RefCell::new(Some(WriteLease {
                _local: guard,
                _external: external,
            })),
            grant(),
        ),
    )
    .await
}

/// The admin-socket grant is not a zome call; these name it on the plane
/// guard's error and series.
const ADMIN_GRANT_ZOME: &str = "admin";
const ADMIN_GRANT_FN: &str = "authorize_signing_credentials";

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
    dispatch_under(&PlanePolicy::current(), chain_key, zome, fn_name, call).await
}

/// [`dispatch`] with the plane policy as an argument, so the rule can be
/// exercised against a policy this build does not run (an empty debt list, the
/// release posture) without building release.
async fn dispatch_under<F, Fut, T>(
    policy: &PlanePolicy,
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
    // WHO IS SPEAKING, AND WHY NOW — asked before the write is queued. A refusal
    // takes no lock, marks no witness and never invokes `call`, so nothing is
    // offered to the conductor.
    enforce_plane(
        current_writer_kind(),
        current_change(),
        zome,
        fn_name,
        policy,
    )?;
    write_serialized(chain_key, fn_name, call).await
}

/// The write arm of [`dispatch`]: the serializer and the `HeadMoved` retry, and
/// nothing else.
///
/// Crate-private on purpose. The plane guard runs in [`dispatch`], before this
/// is reached, so a caller that came straight here would commit without ever
/// declaring who is speaking. Go through [`dispatch`].
pub(crate) async fn write_serialized<F, Fut, T>(
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

    // This is the retry budget, not a native execution timeout. A caller may
    // bound its observation independently (projection reconciliation does).
    // Once offered, that one native call retains its lease until its response;
    // abandoning the observer neither releases the chain nor starts a retry.
    let started = Instant::now();
    let budget_end = started + RETRY_BUDGET;

    let mut queued_total = Duration::ZERO;
    let mut last_error: Option<StorageError> = None;

    for attempt in 1..=MAX_ATTEMPTS {
        let queue_started = Instant::now();
        let guard = lock.clone().lock_owned().await;
        let external = external_writer_lock(chain_key).await?;
        let waited = queue_started.elapsed();
        queued_total += waited;

        let dispatched_at = Instant::now();
        // Past this point the call is offered: the closure takes its admission
        // permit and crosses the websocket, so an outer timeout can no longer
        // claim nothing happened.
        mark_dispatched();
        let outcome = WRITE_LEASE
            .scope(
                RefCell::new(Some(WriteLease {
                    _local: guard,
                    _external: external,
                })),
                call(),
            )
            .await;
        let rtt = dispatched_at.elapsed();
        // The native response owns the lease once offered. A normal return has
        // released it; an abandoned observer leaves it with that response.

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
                        voice = writer.plane().map_or(UNDECLARED, |p| p.voice.label()),
                        trigger = writer.plane().map_or(UNDECLARED, |p| p.trigger.label()),
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

    #[cfg(unix)]
    #[tokio::test]
    async fn external_ceremony_lock_blocks_only_its_cell_and_releases_on_process_exit() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("same-cell.lock");
        let mut external = std::process::Command::new("flock")
            .args(["--exclusive", "--"])
            .arg(&path)
            .args(["sh", "-c", "printf 'locked\\n'; read release"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::BufRead;
        let mut ready = String::new();
        std::io::BufReader::new(external.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        assert_eq!(ready, "locked\n");
        assert!(tokio::time::timeout(
            Duration::from_millis(50),
            lock_file(directory.path(), "same-cell")
        )
        .await
        .is_err());
        let other = tokio::time::timeout(
            Duration::from_secs(1),
            lock_file(directory.path(), "other-cell"),
        )
        .await
        .unwrap()
        .unwrap();
        drop(other);
        drop(external.stdin.take());
        external.wait().unwrap();
        let acquired = tokio::time::timeout(
            Duration::from_secs(1),
            lock_file(directory.path(), "same-cell"),
        )
        .await
        .unwrap()
        .unwrap();
        drop(acquired);
    }

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
            "sign_account_bind_challenge",
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
    /// commas, then picks the fn slot by METHOD and arity — arity alone is
    /// ambiguous, because two of the shapes in this crate take four arguments:
    ///
    /// * 3 args `(zome, fn, payload)` — `HcClient::call_zome{,_imagodei,_mishpat}`
    /// * 4 args `(zome, fn, payload, class)` — `HcClient::call_zome_timed` (fn in
    ///   slot 1)
    /// * 4 args `(target|cell, zome, fn, payload)` — the raw `app_ws` / legacy
    ///   client (fn in slot 2)
    ///
    /// Only a BARE string literal in that slot is reported: a `const` or a
    /// generic forwarder is invisible here by design (those were enumerated by
    /// hand when the table was built). The rail's job is to catch the shape a
    /// NEW call site is written in, not to re-derive the whole table.
    fn scan_zome_fn_literals(text: &str) -> Vec<String> {
        scan_zome_fn_literal_sites(text)
            .into_iter()
            .map(|(_, name)| name)
            .collect()
    }

    /// [`scan_zome_fn_literals`] with the byte offset of each call's opening
    /// paren, so a caller can ask WHERE in the file the call sits.
    fn scan_zome_fn_literal_sites(text: &str) -> Vec<(usize, String)> {
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
                // `call_zome_timed(zome, fn, payload, class)` is ALSO four
                // arguments, with the fn in slot 1. Picking by arity alone read
                // its `payload` as the fn and left every timed call invisible
                // to both rails.
                let slot = match (marker, parts.len()) {
                    (_, 3) | (".call_zome_timed(", 4) => 1,
                    (_, 4) => 2,
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
                    out.push((open, candidate[1..candidate.len() - 1].to_string()));
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

    #[tokio::test]
    async fn a_caller_timeout_keeps_an_offered_native_write_exclusive_until_its_reply() {
        let key = "chain:offered-cancellation";
        let started = Arc::new(tokio::sync::Notify::new());
        let finish = Arc::new(tokio::sync::Notify::new());
        let first_started = started.clone();
        let first_finish = finish.clone();
        let first = tokio::spawn(async move {
            tokio::time::timeout(
                Duration::from_millis(50),
                write_serialized(key, "create_content", || {
                    let started = first_started.clone();
                    let finish = first_finish.clone();
                    async move {
                        let (reply, response) = tokio::sync::oneshot::channel();
                        // Like conductor WASM, this operation survives the HTTP
                        // caller abandoning its response future.
                        tokio::spawn(async move {
                            started.notify_one();
                            finish.notified().await;
                            let _ = reply.send(());
                        });
                        finish_offered_call(async move {
                            response
                                .await
                                .map_err(|error| StorageError::Internal(error.to_string()))
                        })
                        .await
                    }
                }),
            )
            .await
        });
        started.notified().await;
        assert!(
            first.await.unwrap().is_err(),
            "caller deadline was extended"
        );

        let second = tokio::spawn(async move {
            write_serialized(key, "update_content", || async {
                Ok::<_, StorageError>(())
            })
            .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        let raced = second.is_finished();
        finish.notify_one();
        second.await.unwrap().unwrap();
        assert!(
            !raced,
            "a later writer entered while the offered native call was still running"
        );
    }

    #[tokio::test]
    async fn a_cancelled_admission_waiter_never_offers_a_call_or_leaks_its_chain_lock() {
        let key = "chain:cancelled-admission";
        let admission = Arc::new(tokio::sync::Notify::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let timed_out = tokio::time::timeout(
            Duration::from_millis(20),
            write_serialized(key, "create_content", || {
                let admission = admission.clone();
                let calls = calls.clone();
                async move {
                    admission.notified().await;
                    calls.fetch_add(1, AtomicOrdering::SeqCst);
                    finish_offered_call(async { Ok::<_, StorageError>(()) }).await
                }
            }),
        )
        .await;
        assert!(timed_out.is_err());
        admission.notify_one();
        tokio::time::timeout(
            Duration::from_millis(100),
            write_serialized(key, "update_content", || async {
                Ok::<_, StorageError>(())
            }),
        )
        .await
        .expect("cancelled admission retained the chain lock")
        .unwrap();
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 0);
    }

    #[tokio::test]
    async fn an_abandoned_offered_refusal_is_not_retried() {
        let key = "chain:abandoned-refusal";
        let calls = Arc::new(AtomicUsize::new(0));
        let timed_out = tokio::time::timeout(
            Duration::from_millis(20),
            write_serialized(key, "create_content", || {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, AtomicOrdering::SeqCst);
                    finish_offered_call(async {
                        tokio::time::sleep(Duration::from_millis(60)).await;
                        Err::<(), _>(head_moved())
                    })
                    .await
                }
            }),
        )
        .await;
        assert!(timed_out.is_err());
        write_serialized(key, "update_content", || async {
            Ok::<_, StorageError>(())
        })
        .await
        .unwrap();
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
    }

    /// Size of the conductor-call future the ballast tests offer. Far larger
    /// than a real `call_zome` future, so the by-value cost is unmistakable.
    const BALLAST: usize = 128 * 1024;

    async fn ballast_call() -> Result<usize, StorageError> {
        // Held across an await, so it lives in the future's state.
        let ballast = [7u8; BALLAST];
        tokio::task::yield_now().await;
        Ok(ballast.iter().map(|byte| usize::from(*byte)).sum())
    }

    /// The offered call is boxed before any future in the gate captures it.
    ///
    /// Every HTTP write that reaches the conductor awaits this future inline,
    /// so whatever it carries by value is carried by every ancestor future up
    /// to `HttpServer::handle_request` — and an unoptimized build copies each
    /// ancestor's child future onto that ancestor's poll frame. Carrying the
    /// call by value is what took `matthew`'s 2 MiB `http-server` worker over
    /// its stack on 2026-10-02.
    #[test]
    fn the_offered_call_is_boxed_not_carried_by_value() {
        let call = ballast_call();
        assert!(std::mem::size_of_val(&call) >= BALLAST);
        let offered = finish_offered_call(call);
        let size = std::mem::size_of_val(&offered);
        assert!(
            size < 1024,
            "finish_offered_call carries its {BALLAST}-byte call inline ({size} bytes); \
             every ancestor future up to the HTTP dispatcher grows by that much"
        );
    }

    /// The spawn path of an offered write, run on a thread with tokio's default
    /// 2 MiB worker stack. Before the call was boxed, a large call future was
    /// moved by value through the write gate and `tokio::spawn`'s generic
    /// frames, each holding a full copy in an unoptimized build, and this thread
    /// aborted with "has overflowed its stack" — the production failure class.
    /// Runs in a child process because a stack overflow aborts the process.
    #[test]
    fn an_offered_write_spawns_within_the_default_worker_stack() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "chain_write_gate::tests::offered_write_stack_probe_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("ELOHIM_OFFERED_WRITE_STACK_PROBE", "1")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success() && stdout.contains("offered-write-stack-probe: ok"),
            "offered write did not complete on a 2 MiB stack ({}):\n{stderr}",
            output.status
        );
    }

    /// Child half of the probe above; inert unless that test launches it.
    #[test]
    fn offered_write_stack_probe_child() {
        if std::env::var_os("ELOHIM_OFFERED_WRITE_STACK_PROBE").is_none() {
            return;
        }
        let worker = std::thread::Builder::new()
            .name("http-server".into())
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async {
                        write_serialized("chain:offered-stack-probe", "create_content", || {
                            finish_offered_call(ballast_call())
                        })
                        .await
                    })
            })
            .unwrap();
        let (total, _) = worker.join().unwrap().unwrap();
        assert_eq!(total, 7 * BALLAST);
        println!("offered-write-stack-probe: ok");
    }

    // =========================================================================
    // The plane rule
    // =========================================================================

    /// The policy this build runs by default, minus the operator override, so
    /// these tests do not depend on the environment they are launched from.
    fn build_policy() -> PlanePolicy {
        PlanePolicy {
            debt: PLANE_DEBT,
            undeclared: UndeclaredPosture::for_build(),
        }
    }

    /// The same rule with NO declared debt: what the gate does to a
    /// machine-timer writer nobody allowlisted.
    fn policy_without_debt() -> PlanePolicy {
        PlanePolicy {
            debt: &[],
            undeclared: UndeclaredPosture::Refuse,
        }
    }

    /// A stubbed conductor: counts the calls that were actually offered.
    async fn offer(
        policy: &PlanePolicy,
        zome: &str,
        fn_name: &str,
        calls: &AtomicUsize,
    ) -> Result<(), StorageError> {
        dispatch_under(policy, "chain:plane", zome, fn_name, || {
            calls.fetch_add(1, AtomicOrdering::SeqCst);
            async { Ok::<_, StorageError>(()) }
        })
        .await
        .map(|_| ())
    }

    fn tolerated(writer: WriterKind) -> u64 {
        crate::metrics::CHAIN_WRITE_PLANE_VIOLATION_TOLERATED
            .with_label_values(&[writer.label()])
            .get()
    }

    fn refused(writer: WriterKind, reason: &str) -> u64 {
        crate::metrics::CHAIN_WRITE_PLANE_VIOLATION_REFUSED
            .with_label_values(&[writer.label(), reason])
            .get()
    }

    fn let_through_undeclared(fn_name: &str) -> u64 {
        crate::metrics::CHAIN_WRITE_UNDECLARED_WRITER
            .with_label_values(&[fn_name])
            .get()
    }

    fn by_plane(writer: WriterKind, voice: &str, trigger: &str) -> u64 {
        crate::metrics::CHAIN_WRITES_BY_PLANE
            .with_label_values(&[writer.label(), voice, trigger])
            .get()
    }

    /// A witness for tests that need SOME evidence of change in hand.
    fn a_change() -> Option<ChangeWitness> {
        ChangeWitness::value_moves("test: declared head", Some("uhCkk-old"), "uhCkk-new")
    }

    #[tokio::test(start_paused = true)]
    async fn a_machine_timer_write_outside_the_debt_list_is_refused_before_the_conductor() {
        let calls = AtomicUsize::new(0);
        let before = refused(WriterKind::Heartbeat, REFUSED_MACHINE_TIMER);
        let witness = DispatchWitness::new();

        // With the debt list emptied the heartbeat is exactly "a machine on a
        // timer that nobody allowlisted".
        let error = as_writer(
            WriterKind::Heartbeat,
            witnessed(&witness, async {
                offer(
                    &policy_without_debt(),
                    "infrastructure",
                    "record_peer_status",
                    &calls,
                )
                .await
            }),
        )
        .await
        .unwrap_err();

        match &error {
            StorageError::PlaneViolation {
                writer,
                voice,
                trigger,
                zome,
                fn_name,
            } => {
                assert_eq!(writer, "heartbeat");
                assert_eq!(voice, "machine");
                assert_eq!(trigger, "timer");
                assert_eq!(zome, "infrastructure");
                assert_eq!(fn_name, "record_peer_status");
            }
            other => panic!("expected the typed plane refusal, got {other:?}"),
        }
        assert_eq!(
            calls.load(AtomicOrdering::SeqCst),
            0,
            "a refused write was offered to the conductor"
        );
        assert!(
            !witness.dispatched() && !witness.parked_without_dispatch(),
            "a refused write must neither queue nor dispatch"
        );
        assert!(refused(WriterKind::Heartbeat, REFUSED_MACHINE_TIMER) > before);
        // A refusal is a verdict, not a race: it is not the HeadMoved class.
        assert!(!is_source_chain_head_moved(&error));
    }

    /// What a caller (and an HTTP body) receives is short and stable; the rule
    /// and its spec path live in the gate's log line.
    #[test]
    fn the_refusal_a_caller_sees_is_a_stable_code_and_the_rule_stays_in_the_log() {
        let error = plane_violation(
            WriterKind::Heartbeat,
            WriterKind::Heartbeat.plane(),
            "infrastructure",
            "record_peer_status",
        );
        let shown = error.to_string();
        assert!(shown.starts_with("plane_violation:"), "{shown}");
        assert!(
            !shown.contains("genesis/") && !shown.contains(".md"),
            "the caller-facing text must not carry a repository path: {shown}"
        );

        let body = crate::services::response::error_response(error);
        assert_eq!(body.status(), hyper::StatusCode::INTERNAL_SERVER_ERROR);

        assert!(PLANE_RULE.contains("A machine write on a timer is refused"));
        assert!(PLANE_RULE
            .contains("genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md"));
    }

    #[tokio::test(start_paused = true)]
    async fn declared_writers_that_are_not_a_machine_on_a_timer_pass() {
        let period_closed = ChangeWitness::period_closed("test: soak window", 1_000, 2_000, false);
        for (writer, change, voice, trigger) in [
            (WriterKind::HttpAuthor, None, Voice::Person, Trigger::Act),
            (
                WriterKind::ComputeDelegate,
                None,
                Voice::Delegate,
                Trigger::Act,
            ),
            // Event-driven: the event that woke it is the transition.
            (
                WriterKind::TriggerAdopt,
                None,
                Voice::Machine,
                Trigger::Transition,
            ),
            // Clock-driven: a transition only because it SHOWS one.
            (
                WriterKind::SweepAdopt,
                a_change(),
                Voice::Machine,
                Trigger::Transition,
            ),
            (
                WriterKind::ReleaseSoak,
                period_closed,
                Voice::Machine,
                Trigger::PeriodClose,
            ),
        ] {
            assert_eq!(
                plane_of(writer, change),
                Some(Plane { voice, trigger }),
                "{writer:?} no longer derives the plane this test pins"
            );
            let calls = AtomicUsize::new(0);
            // Refused-if-undeclared AND no tolerated debt: the strictest policy.
            as_writer(
                writer,
                with_change(
                    change,
                    offer(
                        &policy_without_debt(),
                        "content_store",
                        "create_content",
                        &calls,
                    ),
                ),
            )
            .await
            .unwrap_or_else(|e| panic!("{writer:?} ({voice:?}, {trigger:?}) was refused: {e}"));
            assert_eq!(calls.load(AtomicOrdering::SeqCst), 1, "{writer:?}");
        }
    }

    /// The point of the witness: "transition" is not a word a clock-driven
    /// writer can say. Without evidence in scope its write is a timer's, and a
    /// writer that is not declared debt is refused — under the PRODUCTION
    /// policy, not a test one.
    #[tokio::test(start_paused = true)]
    async fn a_clock_driven_writer_cannot_declare_a_transition_it_cannot_show() {
        assert_eq!(
            WriterKind::CustodyRotation.standing(),
            Some((Voice::Machine, Drive::Clock))
        );
        assert!(
            !PLANE_DEBT
                .iter()
                .any(|d| d.writer == WriterKind::CustodyRotation),
            "this test needs a clock-driven writer that is not declared debt"
        );
        let calls = AtomicUsize::new(0);
        let before = refused(WriterKind::CustodyRotation, REFUSED_MACHINE_TIMER);

        // No witness in scope: refused.
        let error = as_writer(
            WriterKind::CustodyRotation,
            offer(
                &build_policy(),
                "content_store",
                "create_rea_commitment",
                &calls,
            ),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(&error, StorageError::PlaneViolation { trigger, .. } if trigger == "timer"),
            "{error:?}"
        );
        // A witness that could not be built is the same as none.
        let unchanged = ChangeWitness::value_moves("test: state", Some("active"), "active");
        assert!(unchanged.is_none());
        as_writer(
            WriterKind::CustodyRotation,
            with_change(
                unchanged,
                offer(
                    &build_policy(),
                    "content_store",
                    "update_rea_commitment_state",
                    &calls,
                ),
            ),
        )
        .await
        .unwrap_err();
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 0);
        // Lower bounds: a sibling test refuses this writer too, in parallel.
        assert!(refused(WriterKind::CustodyRotation, REFUSED_MACHINE_TIMER) >= before + 2);

        // The same write with the change shown: passes, as a transition.
        let transitions_before = by_plane(WriterKind::CustodyRotation, "machine", "transition");
        as_writer(
            WriterKind::CustodyRotation,
            with_change(
                ChangeWitness::value_moves("test: state", Some("proposed"), "active"),
                offer(
                    &build_policy(),
                    "content_store",
                    "update_rea_commitment_state",
                    &calls,
                ),
            ),
        )
        .await
        .expect("a shown transition passes");
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
        assert!(
            by_plane(WriterKind::CustodyRotation, "machine", "transition") > transitions_before
        );
    }

    /// A witness does not leak: it vouches for the write it is scoped around
    /// and nothing after it.
    #[tokio::test(start_paused = true)]
    async fn a_change_witness_covers_only_the_write_it_is_scoped_around() {
        let calls = AtomicUsize::new(0);
        as_writer(WriterKind::CustodyRotation, async {
            with_change(
                a_change(),
                offer(
                    &build_policy(),
                    "content_store",
                    "create_rea_commitment",
                    &calls,
                ),
            )
            .await
            .expect("witnessed");
            assert_eq!(current_change(), None);
            offer(
                &build_policy(),
                "content_store",
                "create_rea_commitment",
                &calls,
            )
            .await
            .expect_err("the next write has no witness of its own");
            // An inner scope that could not show a change overrides an outer one.
            with_change(a_change(), async {
                with_change(
                    None,
                    offer(
                        &build_policy(),
                        "content_store",
                        "create_rea_commitment",
                        &calls,
                    ),
                )
                .await
                .expect_err("an inner `None` is not rescued by an outer witness");
            })
            .await;
        })
        .await;
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
    }

    #[test]
    fn a_change_witness_cannot_be_built_from_an_unchanged_observation() {
        // The value already stands.
        assert!(ChangeWitness::value_moves("t", Some("h1"), "h1").is_none());
        assert!(ChangeWitness::value_moves("t", Some(" h1 "), "h1").is_none());
        assert!(ChangeWitness::value_moves("t", Some("h1"), "h2").is_some());
        assert!(ChangeWitness::value_moves("t", None, "h2").is_some());
        // The record is already there.
        assert!(ChangeWitness::absent("t", Some(&())).is_none());
        assert!(ChangeWitness::absent::<()>("t", None).is_some());
        // The ledger had already seen it.
        assert!(ChangeWitness::newly_claimed("t", false).is_none());
        assert!(ChangeWitness::newly_claimed("t", true).is_some());
        // The period is still open, or its record exists.
        assert!(ChangeWitness::period_closed("t", 2_000, 1_000, false).is_none());
        assert!(ChangeWitness::period_closed("t", 1_000, 2_000, true).is_none());
        let closed = ChangeWitness::period_closed("t", 1_000, 2_000, false).unwrap();
        assert_eq!(
            plane_of(WriterKind::ReleaseSoak, Some(closed)).map(|p| p.trigger),
            Some(Trigger::PeriodClose)
        );

        // A repair loop: the first write against an observation is witnessed,
        // the repeat against the SAME observation is not, and settling the
        // observation makes a later occurrence a new change.
        let evidence = "test: row has no anchor";
        let key = "plane-test-row-first-observation";
        assert!(ChangeWitness::first_observation(evidence, key).is_some());
        assert!(ChangeWitness::first_observation(evidence, key).is_none());
        assert!(ChangeWitness::first_observation(evidence, key).is_none());
        ChangeWitness::settle_observation(evidence, key);
        let again = ChangeWitness::first_observation(evidence, key).unwrap();
        assert_eq!(again.evidence(), evidence);
        ChangeWitness::settle_observation(evidence, key);
    }

    /// The 46-rows-on-a-cycle shape, at the gate: a debt-listed repair loop's
    /// first write is a transition and is NOT counted as debt; every repeat
    /// against the unchanged observation is a timer and IS.
    #[tokio::test(start_paused = true)]
    async fn a_repair_loop_re_authoring_an_unchanged_row_is_counted_as_debt() {
        let calls = AtomicUsize::new(0);
        let evidence = "test: content row's anchor is dead";
        let key = "plane-test-row-repeat";
        let debt_before = tolerated(WriterKind::Reanchor);
        let timers_before = by_plane(WriterKind::Reanchor, "machine", "timer");
        let transitions_before = by_plane(WriterKind::Reanchor, "machine", "transition");

        for _sweep in 0..6 {
            as_writer(
                WriterKind::Reanchor,
                with_change(
                    ChangeWitness::first_observation(evidence, key),
                    offer(&build_policy(), "content_store", "create_content", &calls),
                ),
            )
            .await
            .expect("re-anchor is declared debt: never refused, always counted");
        }
        ChangeWitness::settle_observation(evidence, key);

        assert_eq!(calls.load(AtomicOrdering::SeqCst), 6);
        assert_eq!(
            by_plane(WriterKind::Reanchor, "machine", "transition"),
            transitions_before + 1,
            "only the first re-author answers a change"
        );
        assert_eq!(
            by_plane(WriterKind::Reanchor, "machine", "timer"),
            timers_before + 5
        );
        assert_eq!(
            tolerated(WriterKind::Reanchor),
            debt_before + 5,
            "five of six re-authors were the loop repeating itself"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn an_undeclared_writers_chain_write_is_refused_in_debug_and_test_builds() {
        // The posture is a property of the BUILD: tests compile with debug
        // assertions, so this is the arm a new unnamed write site hits here and
        // on a debug mesh binary.
        assert_eq!(
            UndeclaredPosture::for_build(),
            if cfg!(debug_assertions) {
                UndeclaredPosture::Refuse
            } else {
                UndeclaredPosture::Count
            }
        );
        let policy = PlanePolicy {
            debt: PLANE_DEBT,
            undeclared: UndeclaredPosture::Refuse,
        };
        assert_eq!(
            plane_verdict(WriterKind::Other, None, &policy),
            PlaneVerdict::UndeclaredRefused
        );
        // A witness does not name a writer.
        assert_eq!(
            plane_verdict(WriterKind::Other, a_change(), &policy),
            PlaneVerdict::UndeclaredRefused
        );

        let calls = AtomicUsize::new(0);
        let refused_before = refused(WriterKind::Other, REFUSED_UNDECLARED);
        let let_through_before = let_through_undeclared("plane_test_refused_extern");
        // No `as_writer`: the task-local default is `Other`.
        let error = offer(
            &policy,
            "content_store",
            "plane_test_refused_extern",
            &calls,
        )
        .await
        .unwrap_err();

        match &error {
            StorageError::PlaneViolation {
                writer,
                voice,
                trigger,
                ..
            } => {
                assert_eq!(writer, "other");
                assert_eq!(voice, UNDECLARED);
                assert_eq!(trigger, UNDECLARED);
            }
            other => panic!("expected the typed plane refusal, got {other:?}"),
        }
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 0);
        assert!(
            refused(WriterKind::Other, REFUSED_UNDECLARED) > refused_before,
            "an undeclared refusal is counted on the refused series, reason=undeclared"
        );
        assert_eq!(
            let_through_undeclared("plane_test_refused_extern"),
            let_through_before,
            "a refused write was not let through and must not read as one"
        );
    }

    /// The release arm, exercised through the function that decides rather than
    /// by building release: an unnamed writer is counted and let through.
    #[tokio::test(start_paused = true)]
    async fn an_undeclared_writers_chain_write_is_counted_and_let_through_under_the_release_posture(
    ) {
        let policy = PlanePolicy {
            debt: PLANE_DEBT,
            undeclared: UndeclaredPosture::Count,
        };
        assert_eq!(
            plane_verdict(WriterKind::Other, None, &policy),
            PlaneVerdict::UndeclaredCounted
        );
        assert!(!plane_verdict(WriterKind::Other, None, &policy).is_refused());

        let calls = AtomicUsize::new(0);
        let before = let_through_undeclared("plane_test_counted_extern");
        for _ in 0..3 {
            offer(
                &policy,
                "content_store",
                "plane_test_counted_extern",
                &calls,
            )
            .await
            .expect("release must not break a running node on a label");
        }
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 3);
        assert_eq!(
            let_through_undeclared("plane_test_counted_extern"),
            before + 3,
            "every undeclared write is counted, even though it is logged once"
        );
    }

    #[test]
    fn the_undeclared_posture_override_is_case_insensitive_and_never_fails_silently() {
        for word in ["refuse", "REFUSE", " Refuse "] {
            assert_eq!(
                UndeclaredPosture::resolve(Some(word)),
                (UndeclaredPosture::Refuse, PostureSource::Override),
                "{word:?}"
            );
        }
        for word in ["count", "Count", "COUNT\n"] {
            assert_eq!(
                UndeclaredPosture::resolve(Some(word)),
                (UndeclaredPosture::Count, PostureSource::Override),
                "{word:?}"
            );
        }
        assert_eq!(
            UndeclaredPosture::resolve(None),
            (UndeclaredPosture::for_build(), PostureSource::BuildDefault)
        );
        // Set, but not one of the two words: the build default stays in force
        // AND the resolution says so, which is what `current()` warns on.
        for word in ["", "off", "true", "refused", "counts"] {
            assert_eq!(
                UndeclaredPosture::resolve(Some(word)),
                (
                    UndeclaredPosture::for_build(),
                    PostureSource::UnrecognisedOverride
                ),
                "{word:?}"
            );
            assert_eq!(
                UndeclaredPosture::parse_override(Some(word)),
                Err(word.to_string())
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn the_allowlisted_heartbeat_passes_and_is_counted_as_tolerated_debt() {
        let calls = AtomicUsize::new(0);
        let before = tolerated(WriterKind::Heartbeat);

        as_writer(
            WriterKind::Heartbeat,
            offer(
                &build_policy(),
                "infrastructure",
                "record_peer_status",
                &calls,
            ),
        )
        .await
        .expect("the live heartbeat must not be refused before its readers move (§5.5)");

        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(
            tolerated(WriterKind::Heartbeat),
            before + 1,
            "tolerated debt must be counted on every write"
        );
        assert_eq!(
            plane_verdict(WriterKind::Heartbeat, None, &build_policy()),
            PlaneVerdict::ToleratedDebt(Plane {
                voice: Voice::Machine,
                trigger: Trigger::Timer,
            })
        );
    }

    /// THE PIN. New debt is a deliberate, reviewed act: adding an entry to
    /// [`PLANE_DEBT`] fails here until this list is edited to match.
    ///
    /// Before you edit it: a machine write on a timer is the anti-pattern
    /// plane-separation §1 rules out. The answer is almost always to show the
    /// change at the write site (a [`ChangeWitness`]), to close a period, or to
    /// move the fact off the chain — not to add a row here. Removing a row when
    /// its work lands is the edit this test wants.
    #[test]
    fn the_plane_debt_allowlist_is_pinned() {
        let pinned: [&str; 8] = [
            "heartbeat",
            "reanchor",
            "sweep_reauthor",
            "sweep_adopt",
            "provide",
            "custody_intent",
            "release_adoption",
            "release_soak",
        ];

        let actual: Vec<&str> = PLANE_DEBT.iter().map(|d| d.writer.label()).collect();
        assert_eq!(
            actual, pinned,
            "PLANE_DEBT changed. Tolerated machine-timer debt may only grow by a reviewed edit \
             to this test (plane-separation §12)."
        );
        for debt in PLANE_DEBT {
            assert_eq!(
                debt.writer.standing(),
                Some((Voice::Machine, Drive::Clock)),
                "{:?} is listed as debt but is not a clock-driven machine writer — only those \
                 can be a machine on a timer, so the entry is dead",
                debt.writer
            );
            assert!(
                debt.retired_by.contains('§'),
                "{:?}: every debt entry names the spec section that retires it",
                debt.writer
            );
        }
        // And the converse, also pinned: the clock-driven writers that are NOT
        // debt, i.e. the ones the gate REFUSES when they cannot show a change.
        // A new clock-driven kind must land in one list or the other on purpose.
        let refused_without_a_witness: Vec<&str> = WriterKind::ALL
            .iter()
            .filter(|w| matches!(w.standing(), Some((_, Drive::Clock))))
            .filter(|w| !PLANE_DEBT.iter().any(|d| d.writer == **w))
            .map(|w| w.label())
            .collect();
        assert_eq!(
            refused_without_a_witness,
            ["custody_rotation", "device_affirm"]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn reads_are_unaffected_by_the_plane_guard() {
        // Undeclared, under the strictest policy: a read never asks who speaks.
        let calls = AtomicUsize::new(0);
        let before = let_through_undeclared("get_content");
        offer(
            &policy_without_debt(),
            "content_store",
            "get_content",
            &calls,
        )
        .await
        .expect("an undeclared READ must pass");
        // Even the refused plane reads freely — the rule is about writes.
        as_writer(
            WriterKind::Heartbeat,
            offer(
                &policy_without_debt(),
                "infrastructure",
                "get_latest_peer_status_for_agent",
                &calls,
            ),
        )
        .await
        .expect("a machine-timer READ must pass");
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 2);
        assert_eq!(
            let_through_undeclared("get_content"),
            before,
            "a read must not be counted as an undeclared write"
        );
    }

    #[test]
    fn every_writer_but_other_declares_a_standing_and_the_vocabulary_is_closed() {
        let mut labels: Vec<&str> = WriterKind::ALL.iter().map(|w| w.label()).collect();
        labels.sort_unstable();
        let before = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), before, "two writers share a metric label");

        for writer in WriterKind::ALL {
            let Some((voice, drive)) = writer.standing() else {
                assert_eq!(writer, WriterKind::Other, "{writer:?} declares no standing");
                assert_eq!(plane_of(writer, a_change()), None);
                continue;
            };
            assert_ne!(writer, WriterKind::Other);
            // A person or a delegate is at a door; only a machine runs on a clock.
            if voice != Voice::Machine {
                assert_eq!(drive, Drive::Event, "{writer:?}");
            }
            for change in [None, a_change()] {
                let plane = plane_of(writer, change).unwrap();
                assert!(["person", "delegate", "machine"].contains(&plane.voice.label()));
                assert!(
                    ["act", "transition", "period_close", "timer"].contains(&plane.trigger.label())
                );
                // Only a person or a delegate acts; a machine never does.
                assert_eq!(
                    plane.trigger == Trigger::Act,
                    voice != Voice::Machine,
                    "{writer:?} derives {plane:?}"
                );
                // A timer is exactly a clock-driven machine with nothing to show.
                assert_eq!(
                    plane.is_machine_timer(),
                    drive == Drive::Clock && change.is_none(),
                    "{writer:?} with {change:?} derives {plane:?}"
                );
            }
        }
    }

    #[tokio::test]
    async fn the_capability_grant_names_itself_and_is_counted() {
        use holochain_types::prelude::{AgentPubKey, DnaHash};
        let cell = CellId::new(
            DnaHash::from_raw_36(vec![0xC1; 36]),
            AgentPubKey::from_raw_36(vec![0xC2; 36]),
        );
        let seen = grant_capability_serialized(&cell, || async {
            Ok::<_, StorageError>(current_writer_kind())
        })
        .await
        .expect("a credential grant is a machine transition, never refused");
        assert_eq!(seen, WriterKind::CapGrant);
        assert_eq!(
            plane_verdict(WriterKind::CapGrant, None, &build_policy()),
            PlaneVerdict::Pass(Plane {
                voice: Voice::Machine,
                trigger: Trigger::Transition,
            })
        );
    }

    #[test]
    fn the_plane_series_are_on_the_scrape_before_the_first_write() {
        crate::metrics::register_all();
        let text = crate::metrics::gather_text();
        for series in [
            // The debt, per listed writer.
            "elohim_chain_write_plane_violation_tolerated_total{writer=\"heartbeat\"}",
            "elohim_chain_write_plane_violation_tolerated_total{writer=\"reanchor\"}",
            // Both planes of a clock-driven writer.
            "elohim_chain_writes_by_plane_total{trigger=\"timer\",voice=\"machine\",writer=\"heartbeat\"}",
            "elohim_chain_writes_by_plane_total{trigger=\"transition\",voice=\"machine\",writer=\"reanchor\"}",
            // Refusals: a measured zero on an idle node, by reason.
            "elohim_chain_write_plane_violation_refused_total{reason=\"machine_timer\",writer=\"custody_rotation\"}",
            "elohim_chain_write_plane_violation_refused_total{reason=\"undeclared\",writer=\"other\"}",
            // Undeclared writes let through, per extern this crate writes.
            "elohim_chain_write_undeclared_writer_total{fn=\"record_peer_status\"}",
            "elohim_chain_write_undeclared_writer_total{fn=\"create_content\"}",
            // The posture in force, as a gauge.
            "elohim_chain_write_undeclared_posture{posture=\"refuse\"}",
            "elohim_chain_write_undeclared_posture{posture=\"count\"}",
        ] {
            assert!(
                text.contains(series),
                "{series} is not on the scrape at boot:\n{text}"
            );
        }
        let in_force = PlanePolicy::current().undeclared.label();
        assert!(
            text.contains(&format!(
                "elohim_chain_write_undeclared_posture{{posture=\"{in_force}\"}} 1"
            )),
            "the posture in force must read 1"
        );
    }

    #[test]
    fn a_refusal_is_logged_at_most_once_per_extern_per_interval() {
        let last = Mutex::new(HashMap::new());
        let t0 = std::time::Instant::now();
        // A missed site inside a per-row sweep: 500 refusals in one second.
        let logged = (0..500)
            .filter(|i| {
                refusal_log_due_at(
                    &last,
                    "content_store",
                    "create_content",
                    t0 + Duration::from_millis(*i * 2),
                )
            })
            .count();
        assert_eq!(logged, 1, "one error line, not one per row");
        // A different extern is its own line.
        assert!(refusal_log_due_at(
            &last,
            "content_store",
            "update_content",
            t0
        ));
        // And the first extern speaks again once the interval has passed.
        assert!(!refusal_log_due_at(
            &last,
            "content_store",
            "create_content",
            t0 + REFUSAL_LOG_INTERVAL - Duration::from_millis(1)
        ));
        assert!(refusal_log_due_at(
            &last,
            "content_store",
            "create_content",
            t0 + REFUSAL_LOG_INTERVAL
        ));
    }

    /// `as_writer` and `with_change` now wrap the largest futures on the HTTP
    /// request path. Each must add its task-local slot and NOTHING else — an
    /// `async fn` taking the future by value carried it twice, the shape that
    /// overflowed the `http-server` worker stack on 2026-10-02.
    #[test]
    fn as_writer_adds_only_its_label_to_the_future_it_wraps() {
        let bare = std::mem::size_of_val(&ballast_call());
        assert!(bare >= BALLAST);

        let labelled = std::mem::size_of_val(&as_writer(WriterKind::HttpAuthor, ballast_call()));
        println!("as_writer: wraps {bare} bytes in {labelled} bytes");
        assert!(
            labelled < bare + 64,
            "as_writer carries its {bare}-byte future in {labelled} bytes; it must add only \
             the label"
        );

        let witnessed = std::mem::size_of_val(&as_writer(
            WriterKind::CustodyRotation,
            with_change(a_change(), ballast_call()),
        ));
        println!("as_writer + with_change: wraps {bare} bytes in {witnessed} bytes");
        assert!(
            witnessed < bare + 128,
            "as_writer + with_change carries its {bare}-byte future in {witnessed} bytes"
        );
    }

    /// THE STATIC RAIL. A chain-writing extern named by a bare literal at a
    /// `.call_zome*(` site must sit inside an `as_writer(…)` call — so a new
    /// write site fails HERE, in a unit test, and not only at runtime on a debug
    /// mesh.
    ///
    /// What it cannot see, by construction: a name passed as a `const` or a
    /// variable (`release_adoption`'s `CARRY_FN`, `api::account`'s forwarder),
    /// and the shared helpers in `services/conductor_writes.rs`, whose callers
    /// carry the label (listed below so that list cannot grow unnoticed). Those
    /// remain covered by the runtime refusal.
    #[test]
    fn every_write_literal_sits_inside_a_writer_scope() {
        /// Shared helpers whose CALLERS name the writer. Each is one extern.
        const LABELLED_BY_CALLER: &[(&str, &str)] = &[
            ("services/conductor_writes.rs", "create_rea_commitment"),
            ("services/conductor_writes.rs", "create_rea_economic_event"),
            (
                "services/conductor_writes.rs",
                "update_rea_commitment_state",
            ),
            ("services/conductor_writes.rs", "create_content"),
            ("services/conductor_writes.rs", "update_content"),
            ("services/conductor_writes.rs", "get_content_by_id"),
            (
                "services/conductor_writes.rs",
                "declare_earned_canonical_head",
            ),
            ("services/conductor_writes.rs", "declare_content_head"),
            (
                "services/conductor_writes.rs",
                "declare_canonical_content_head",
            ),
            ("services/conductor_writes.rs", "create_commitment"),
            (
                "services/conductor_writes.rs",
                "create_commitment_state_link",
            ),
            // The device-consent cell's writes speak in their caller's voice:
            // a person's own-node route (`http_author`) or the identity
            // declaration reconcile (`identity_declaration`).
            ("services/device_consent_cell.rs", "create_human"),
            (
                "services/device_consent_cell.rs",
                "register_device_identity",
            ),
        ];

        /// Writes that do not reach this gate at all — the dead
        /// `ContentServerBridge` (module doc, "What this does NOT close"). Named
        /// here so the rail says so out loud; wiring the bridge up means
        /// routing these through `dispatch` and deleting these rows.
        const BYPASSES_THE_GATE: &[(&str, &str)] = &[
            ("content_server.rs", "register_content_server"),
            ("content_server.rs", "update_content_server_heartbeat"),
            ("content_server.rs", "mark_content_server_offline"),
        ];

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut unscoped: Vec<String> = Vec::new();
        let mut seen_exceptions: Vec<(String, String)> = Vec::new();
        let mut files = vec![root.clone()];
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
                let Ok(raw) = std::fs::read_to_string(&p) else {
                    continue;
                };
                let text = strip_line_comments(&raw);
                let bytes = text.as_bytes();
                let relative = p
                    .strip_prefix(&root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .replace('\\', "/");

                // Every `as_writer( … )` argument span in this file.
                let mut scopes: Vec<(usize, usize)> = Vec::new();
                let mut from = 0usize;
                while let Some(idx) = text[from..].find("as_writer(") {
                    let open = from + idx + "as_writer".len();
                    from = open + 1;
                    if let Some(span) = balanced_args(bytes, open) {
                        scopes.push(span);
                    }
                }

                for (at, name) in scan_zome_fn_literal_sites(&text) {
                    if is_read_fn(&name) {
                        continue;
                    }
                    if scopes.iter().any(|(start, end)| *start <= at && at < *end) {
                        continue;
                    }
                    if LABELLED_BY_CALLER
                        .iter()
                        .chain(BYPASSES_THE_GATE)
                        .any(|(file, extern_name)| *file == relative && *extern_name == name)
                    {
                        seen_exceptions.push((relative.clone(), name));
                        continue;
                    }
                    unscoped.push(format!("{relative}: {name}"));
                }
            }
        }
        unscoped.sort();
        unscoped.dedup();
        assert!(
            unscoped.is_empty(),
            "chain-writing externs called outside any `as_writer(…)` scope. Name the writer at \
             the call site (`chain_write_gate::as_writer(WriterKind::…, …)`), or — for a shared \
             helper whose callers name it — add it to LABELLED_BY_CALLER:\n  {}",
            unscoped.join("\n  ")
        );
        // The exception list holds nothing that is no longer true.
        for (file, extern_name) in LABELLED_BY_CALLER.iter().chain(BYPASSES_THE_GATE) {
            assert!(
                seen_exceptions
                    .iter()
                    .any(|(f, n)| f == file && n == extern_name),
                "LABELLED_BY_CALLER lists {file}: {extern_name}, but no unscoped literal call of \
                 it exists there any more — remove the entry"
            );
        }
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
