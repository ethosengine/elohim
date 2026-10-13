//! The first carrier: a device on the private network asks, an approving node lists
//! it, someone decides, and the code goes back the same way, with nobody
//! carrying a link.
//!
//! What carries it. The asking node offers its ask to every peer its local
//! discovery found on the private network, and to no one else: in libp2p and
//! dual modes that is libp2p mDNS (`DeliveryPeer::network == "lan"`), never
//! Kademlia, relay, gossip or a bootstrap peer. The messages ride the existing
//! `/elohim/epr/1.0.0` request/response protocol as one `DeviceCarry` variant
//! each way. In iroh-only mode the node has no local discovery (its only
//! discovery is pkarr, which is not local), so the carrier is absent there and
//! says so; the link-and-paste path always remains.
//!
//! What it is not:
//! - Not authority. An ask that arrives is evidence that a node asked. A
//!   decision is made through `consent_grant::decide`, and agreeing still runs
//!   the agree path on this node's own machine, under its loopback rule.
//! - Not stored. Pending asks, the device's announce and codes in transit are
//!   memory only; nothing is notarized, linked or written to a table. A node
//!   that restarts is asked again.
//! - Not reachable from the network beyond the one protocol message. The list,
//!   the decision and the device's announce are this-machine-only routes.
//!
//! Who lists. A node lists an ask only when it speaks for a person: its own
//! cell is a root controller of an identity's authority, or a device whose
//! joining record stands (every device of a person speaks for them). A node
//! whose record no longer stands, or with no identity, drops every ask with its
//! reason and answers the asking device nothing at all, so it never appears to
//! the asking terminal as a node that lists the ask. Whom the node speaks for is
//! read from its own cell into memory (`Carrier::speaks`), never per incoming
//! ask. A read older than `SPEAKS_MAX_AGE_MICROS` counts as unknown, and
//! unknown lists nothing: nothing is listed on evidence older than that,
//! however the read is kept.
//!
//! How the read is kept (`SpeaksRefresh`, chosen once at start by
//! `ELOHIM_CONSENT_SPEAKS_REFRESH`):
//! - `demand` (the default). A resting node reads nothing. A read is wanted
//!   by a device's signed ask, by a pending-list, decide or signed-out
//!   standing request, and by an identity write. One read runs at a time, and
//!   a caller that waited behind one is judged against it when its turn
//!   comes. A read wanted by a request is not made within
//!   `SPEAKS_REFRESH_SECS` of the last attempt, answered or not; only a
//!   device's ask may try again `SPEAKS_RETRY_SECS` after a read the cell did
//!   not answer (the re-offer that retry exists for). An identity write
//!   that changed something always reads. An ask cannot wait for a read (the
//!   transport's event loop answers it), so an ask that finds no fresh read
//!   wants one and is dropped unanswered, exactly as by a node whose standing
//!   is unknown; the asking device offers again every `ROUND_SECS` and is
//!   listed then.
//! - `timer`. One refresher reads every `SPEAKS_REFRESH_SECS` and at once when
//!   this node's identity may have changed (`identity_changed`), whether or
//!   not anyone asks. It rests when the coordinator answers that the standing
//!   function does not exist: that answer cannot change until the coordinator
//!   does, so it asks again when a coordinator update is applied by this
//!   process (`happ_manager::coordinator_updates`) or, since an update made
//!   from elsewhere is not seen here, at the first turn after
//!   `SPEAKS_REST_MAX_SECS`.
//!
//! What demand gives up against a healthy timer. This is a difference in the
//! safety direction, not only in availability, and it is inside the bound
//! above, not outside it:
//! - A new ask is listed on a read up to `SPEAKS_REFRESH_SECS` old, and no
//!   read is made on its account until that read is that old. So is the
//!   timer's, but the timer's reads fall every `SPEAKS_REFRESH_SECS` whatever
//!   happens, and a demand read falls only when something asked: a record
//!   that stops standing just after a demand read is listed on for up to
//!   `SPEAKS_REFRESH_SECS` more.
//! - The same ceremony, offered again by the same device from the same peer,
//!   keeps its place on a read up to `SPEAKS_MAX_AGE_MICROS` old while the
//!   next read is made, where a healthy timer would have read again within
//!   `SPEAKS_REFRESH_SECS`. If that next read is not answered, the ceremony
//!   stays listed until the old read reaches the bound.
//! - An ask that was waiting when a read began is listed on that read at any
//!   age inside `SPEAKS_MAX_AGE_MICROS` (below): on a cell that takes a
//!   minute to answer, on evidence a minute old.
//! - An idle node holds no evidence, so it lists nothing while its conductor
//!   cannot answer, where the timer lists for up to `SPEAKS_MAX_AGE_MICROS`
//!   into an outage.
//!
//! When a read is taken to have been made. Its evidence is stamped when its
//! turn came, before the cell was asked, so it is never counted newer than it
//! is: a read that took longer than `SPEAKS_MAX_AGE_MICROS` is unknown the
//! moment it lands. The next read is paced from when this one finished.
//!
//! A slow cell. A read that takes longer than `SPEAKS_REFRESH_SECS` lands
//! already too old for a new ask, every time. So the carrier remembers when
//! an ask its device signed first arrived and was not listed (`Waiting`), and
//! lists it on a read whose turn came at or after that arrival, inside
//! `SPEAKS_MAX_AGE_MICROS`: that read is as fresh as that ask could get. An
//! ask that arrives after a read began has no such claim on it. The memory
//! is bounded: `MAX_WAITING` asks, `MAX_WAITING_PER_SOURCE` a peer, each for
//! an ask's life; an ask there is no room to remember falls back to the
//! `SPEAKS_REFRESH_SECS` rule.
//!
//! What time is. Every age and pace here (evidence, the refresh, the retry,
//! the timer's rest) is counted on a clock nobody can set: time since boot,
//! suspend included, on Linux (`CLOCK_BOOTTIME`), and the process's
//! monotonic clock elsewhere (which may not count a suspend). The wall clock
//! is read only for an ask's own life on the list, and for logs.
//!
//! This node's listing proof is made by its own worker (`prover`), woken by a
//! read that says the node speaks for someone. A read never waits for it.
//!
//! Identity namespaces stay apart: the carrier sees a transport id, kept only
//! to answer the peer that asked; the request names the device's agent key,
//! which is what a person is shown. The two are never compared.

use std::sync::{Arc, Mutex, OnceLock};

use consent_grant::{
    proof_holds, redeem, Ask, CarrierKind, CarrierProof, CarryRequest, CarryResponse, Dropped,
    GrantRequest, Listed, MemoryStore, NodeState, PendingAsks, Speaks, ASK_LIFE_MICROS,
};

/// How often the refresher reads whom this node speaks for; on demand, the
/// least time between two reads the cell answered.
pub const SPEAKS_REFRESH_SECS: u64 = 30;
/// How old that read may be before it counts as unknown.
pub const SPEAKS_MAX_AGE_MICROS: i64 = 120 * 1_000_000;
/// How soon a read the cell did not answer may be made again on demand: one
/// re-offer of an asking device.
pub const SPEAKS_RETRY_SECS: u64 = 5;
/// How long the timer rests on a coordinator without the standing function
/// before it asks again anyway, no coordinator update having been seen here.
pub const SPEAKS_REST_MAX_SECS: u64 = 30 * 60;
/// Names how whom this node speaks for is kept: `demand` or `timer`.
pub const SPEAKS_REFRESH_ENV: &str = "ELOHIM_CONSENT_SPEAKS_REFRESH";
use tracing::{debug, info, warn};

const SPEAKS_REFRESH_MICROS: i64 = SPEAKS_REFRESH_SECS as i64 * 1_000_000;
const SPEAKS_RETRY_MICROS: i64 = SPEAKS_RETRY_SECS as i64 * 1_000_000;
const SPEAKS_REST_MAX_MICROS: i64 = SPEAKS_REST_MAX_SECS as i64 * 1_000_000;

/// The most asks whose first arrival is remembered while they wait for a
/// read, and the most from one peer (the pending list's own bounds, doubled
/// and as they are).
pub const MAX_WAITING: usize = 64;
pub const MAX_WAITING_PER_SOURCE: usize = 4;

/// An ask its device signed that wanted a read and was not listed: from
/// which peer, and when it first arrived, on the carrier's clock.
struct Waiting {
    source: String,
    ask: Ask,
    arrived: i64,
}

/// Microseconds on a clock nobody can set. On Linux, time since boot with
/// suspend counted, so what was read before a suspend is as old afterwards
/// as it is. Elsewhere, and if that clock cannot be read, the process's
/// monotonic clock.
fn steady_micros() -> i64 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let mut at = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: `at` is a valid, writable timespec for the call's duration.
        if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut at) } == 0 {
            // The field types differ by platform.
            #[allow(clippy::unnecessary_cast)]
            return (at.tv_sec as i64)
                .saturating_mul(1_000_000)
                .saturating_add(at.tv_nsec as i64 / 1_000);
        }
    }
    static STARTED: OnceLock<std::time::Instant> = OnceLock::new();
    STARTED
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_micros() as i64
}

/// How whom this node speaks for is kept (module doc, "How the read is kept").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeaksRefresh {
    /// Read when something asks; a resting node reads nothing.
    Demand,
    /// Read every `SPEAKS_REFRESH_SECS`, whether or not anyone asks.
    Timer,
}

impl SpeaksRefresh {
    /// What `SPEAKS_REFRESH_ENV` says: `timer`, or `demand` for anything else
    /// (unset included). The second value is whether the word was understood.
    pub fn parse(word: Option<&str>) -> (Self, bool) {
        match word.map(str::trim) {
            None | Some("") => (Self::Demand, true),
            Some(w) if w.eq_ignore_ascii_case("demand") => (Self::Demand, true),
            Some(w) if w.eq_ignore_ascii_case("timer") => (Self::Timer, true),
            Some(_) => (Self::Demand, false),
        }
    }

    /// Read once at start (`HttpServer::serve`), never per request.
    pub fn from_env() -> Self {
        let word = std::env::var(SPEAKS_REFRESH_ENV).ok();
        let (mode, understood) = Self::parse(word.as_deref());
        if !understood {
            warn!(
                env = SPEAKS_REFRESH_ENV,
                value = word.as_deref().unwrap_or_default(),
                "device carrier: not `demand` or `timer`; reading on demand"
            );
        }
        mode
    }

    pub fn word(self) -> &'static str {
        match self {
            Self::Demand => "demand",
            Self::Timer => "timer",
        }
    }
}

/// Why a read of whom this node speaks for is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Want {
    /// A device's signed ask wants it: read unless the last answered read
    /// finished within `SPEAKS_REFRESH_SECS`, or the last unanswered one
    /// within `SPEAKS_RETRY_SECS`.
    Ask,
    /// A request on this node's routes wants it, which anyone may send:
    /// read unless a read was tried within `SPEAKS_REFRESH_SECS`, answered
    /// or not.
    Request,
    /// The timer's turn: read, unless a read landed while this turn waited
    /// for its own.
    Timer,
    /// This node's identity was just written: read.
    Now,
}

impl Want {
    /// What a write to this node's identity over its routes wants, by the
    /// status it answered with. Created: the identity changed, read now. Any
    /// other success (it was already so, nothing was written): no more than
    /// any request. A refusal: nothing, however often it is sent.
    pub fn after_write(status: u16) -> Option<Self> {
        match status {
            201 => Some(Self::Now),
            200..=299 => Some(Self::Request),
            _ => None,
        }
    }
}

/// What the reads of whom this node speaks for have done lately.
#[derive(Default)]
struct SpeaksReads {
    /// When the last read finished and whether the cell answered it; while a
    /// read is out, when it began, unanswered.
    last: Option<(i64, bool)>,
    /// How many reads have landed, answered or not.
    landed: u64,
    /// The timer's rest on a coordinator without the standing function: the
    /// coordinator updates seen when it began, and when it ends regardless.
    rest: Option<(u64, i64)>,
    /// The read is failing and has said so once: whether for want of the
    /// function.
    failing: Option<bool>,
}

/// Everything the carrier holds, for this process.
pub struct Carrier {
    /// Asks waiting on this node as an approving node.
    pub pending: PendingAsks,
    /// Codes this node issued, shared with `/auth/consent/redeem`.
    pub deliveries: Arc<MemoryStore>,
    /// This node's controller key, once its cell is reachable.
    self_key: OnceLock<String>,
    /// This node's key's proof over (listed, its transport id), made once
    /// per key and transport id by the refresher, answered with every
    /// listing (the transport's event loop cannot wait for a signature).
    listed_proof: Mutex<Option<(String, String, CarrierProof)>>,
    /// The approver the carrier proved for a request this node redeemed,
    /// by state: what `/auth/device/enroll` checks the consent against.
    proven: Mutex<Option<(String, String)>>,
    /// This node's own announce, when it is the asking device.
    announce: Mutex<Option<Announce>>,
    /// Whom this node speaks for, as last read from its own cell, and when.
    speaks: Mutex<Option<(Speaks, i64)>>,
    /// Woken when this node's identity may have changed.
    speaks_changed: tokio::sync::Notify,
    /// How that read is kept; on demand until `set_refresh` says otherwise.
    refresh: OnceLock<SpeaksRefresh>,
    /// Woken when something wants that read and one is due.
    speaks_wanted: tokio::sync::Notify,
    /// Held across a read, so one runs at a time. Never held across the
    /// listing proof.
    reading: tokio::sync::Mutex<()>,
    reads: Mutex<SpeaksReads>,
    /// Woken when a read says this node speaks for someone.
    proof_wanted: tokio::sync::Notify,
    /// Held while the listing proof is made, so one is made at a time.
    proving: tokio::sync::Mutex<()>,
    /// Asks waiting for a read, by first arrival (module doc, "A slow cell").
    waiting: Mutex<Vec<Waiting>>,
    /// The time every age and pace is counted on, in microseconds:
    /// `steady_micros`, or a test's own clock.
    clock: Box<dyn Fn() -> i64 + Send + Sync>,
    /// The wall clock, for an ask's life on the list only.
    wall: Box<dyn Fn() -> i64 + Send + Sync>,
}

/// The device's side of one ask.
#[derive(Debug, Clone)]
pub struct Announce {
    pub request: GrantRequest,
    pub state: NodeState,
    pub for_approver: Option<String>,
    pub started_at_micros: i64,
    /// The device key's proof over this ask, as sent from this node.
    pub proof: Option<CarrierProof>,
    /// Approving nodes that listed it, each with its key proven over its
    /// transport id: (transport id, approver key, number there).
    pub listed_by: Vec<(String, Option<String>, u32)>,
    /// The code an approving node that listed it handed back, from where, and
    /// the key whose proof came with it.
    pub code: Option<(String, String)>,
    pub code_approver: Option<String>,
    /// The approving node that declined it (its key, when known), if one did.
    pub declined_by: Option<Option<String>>,
}

impl Announce {
    pub fn ask(&self) -> Ask {
        Ask {
            request: self.request.clone(),
            state: self.state,
            for_approver: self.for_approver.clone(),
            proof: self.proof.clone(),
        }
    }

    pub fn expired(&self, now_micros: i64) -> bool {
        now_micros >= self.started_at_micros.saturating_add(ASK_LIFE_MICROS)
    }
}

pub fn carrier() -> &'static Carrier {
    static CARRIER: OnceLock<Carrier> = OnceLock::new();
    CARRIER.get_or_init(Carrier::new)
}

fn now_micros() -> i64 {
    chrono::Utc::now().timestamp_micros()
}

impl Carrier {
    fn new() -> Self {
        Self {
            pending: PendingAsks::new(),
            deliveries: Arc::new(MemoryStore::new()),
            self_key: OnceLock::new(),
            listed_proof: Mutex::new(None),
            proven: Mutex::new(None),
            announce: Mutex::new(None),
            speaks: Mutex::new(None),
            speaks_changed: tokio::sync::Notify::new(),
            refresh: OnceLock::new(),
            speaks_wanted: tokio::sync::Notify::new(),
            reading: tokio::sync::Mutex::new(()),
            reads: Mutex::new(SpeaksReads::default()),
            proof_wanted: tokio::sync::Notify::new(),
            proving: tokio::sync::Mutex::new(()),
            waiting: Mutex::new(Vec::new()),
            clock: Box::new(steady_micros),
            wall: Box::new(now_micros),
        }
    }

    fn now(&self) -> i64 {
        (self.clock)()
    }

    pub fn set_self_key(&self, key: String) {
        let _ = self.self_key.set(key);
    }

    pub fn self_key(&self) -> Option<&str> {
        self.self_key.get().map(String::as_str)
    }

    /// Whom this node speaks for: the last read, or unknown when there is none
    /// or it is `SPEAKS_MAX_AGE_MICROS` old.
    pub fn speaks(&self) -> Speaks {
        self.speaks_within(self.now(), SPEAKS_MAX_AGE_MICROS)
    }

    /// The last read when it is younger than `max_age_micros` at `now_micros`
    /// (the carrier's clock), or unknown.
    fn speaks_within(&self, now_micros: i64, max_age_micros: i64) -> Speaks {
        match &*self.speaks.lock().unwrap_or_else(|e| e.into_inner()) {
            Some((speaks, at)) if now_micros < at.saturating_add(max_age_micros) => speaks.clone(),
            _ => Speaks::Unknown,
        }
    }

    /// Say how the read is kept. Once, at start; whether it was taken.
    pub fn set_refresh(&self, refresh: SpeaksRefresh) -> bool {
        self.refresh.set(refresh).is_ok()
    }

    pub fn refresh(&self) -> SpeaksRefresh {
        self.refresh.get().copied().unwrap_or(SpeaksRefresh::Demand)
    }

    fn reads_lock(&self) -> std::sync::MutexGuard<'_, SpeaksReads> {
        self.reads.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Whether a read wanted for `want` may be made at `now_micros` (`Want`).
    pub fn read_due(&self, want: Want, now_micros: i64) -> bool {
        let last = self.reads_lock().last;
        match (want, last) {
            (Want::Now | Want::Timer, _) | (_, None) => true,
            (Want::Ask, Some((at, false))) => now_micros >= at.saturating_add(SPEAKS_RETRY_MICROS),
            (Want::Ask | Want::Request, Some((at, _))) => {
                now_micros >= at.saturating_add(SPEAKS_REFRESH_MICROS)
            }
        }
    }

    /// A device's ask wants whom this node speaks for and cannot wait for it
    /// (it is answered in the transport's event loop): wake the reader when a
    /// read is due. Nothing by timer, which reads without being asked.
    pub fn want_speaks(&self, now_micros: i64) {
        if self.refresh() == SpeaksRefresh::Demand && self.read_due(Want::Ask, now_micros) {
            self.speaks_wanted.notify_one();
        }
    }

    /// Wait until an ask wants whom this node speaks for.
    pub async fn speaks_wanted(&self) {
        self.speaks_wanted.notified().await
    }

    /// What an ask from `source` is listed on. By timer, the last read. On
    /// demand an ask its device signed from `source` wants a read, and is
    /// listed on a read younger than `SPEAKS_REFRESH_SECS`. Inside
    /// `SPEAKS_MAX_AGE_MICROS`, two asks are listed on an older one: the very
    /// ask already on this node's list from the same peer (the request, what
    /// the device says of itself, its declared approver and its proof all
    /// equal), which keeps its place while the next read is made; and an ask
    /// that was waiting before that read began (`Waiting`). Anything else
    /// from a listed device is a new ask.
    fn speaks_for_ask(&self, source: &str, ask: &Ask, wall_micros: i64) -> Speaks {
        if self.refresh() == SpeaksRefresh::Timer {
            return self.speaks();
        }
        let now = self.now();
        let signed = ask.signed_from(source);
        if signed {
            self.want_speaks(now);
        }
        match self.speaks_within(now, SPEAKS_REFRESH_MICROS) {
            Speaks::Unknown => {}
            fresh => {
                self.waited(source, ask);
                return fresh;
            }
        }
        let listed = self
            .pending
            .pick(&ask.request.device_key, wall_micros)
            .is_some_and(|p| p.source == source && p.ask == *ask);
        if listed {
            return self.speaks_within(now, SPEAKS_MAX_AGE_MICROS);
        }
        if !signed {
            return Speaks::Unknown;
        }
        let read_at = self
            .speaks
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|(_, at)| *at);
        match (self.first_arrival(source, ask, now), read_at) {
            (Some(arrived), Some(at)) if at >= arrived => {
                let speaks = self.speaks_within(now, SPEAKS_MAX_AGE_MICROS);
                if speaks != Speaks::Unknown {
                    self.waited(source, ask);
                }
                speaks
            }
            _ => Speaks::Unknown,
        }
    }

    /// When this ask from `source` first arrived and was not listed,
    /// remembering now if it is not remembered. `None` when there is no room
    /// to remember it: it is then listed only on a read younger than
    /// `SPEAKS_REFRESH_SECS`.
    // bounded-work: one pass over at most MAX_WAITING asks, each kept for an
    // ask's life at most.
    fn first_arrival(&self, source: &str, ask: &Ask, now_micros: i64) -> Option<i64> {
        let mut waiting = self.waiting.lock().unwrap_or_else(|e| e.into_inner());
        waiting.retain(|w| now_micros < w.arrived.saturating_add(ASK_LIFE_MICROS));
        if let Some(w) = waiting.iter().find(|w| w.source == source && w.ask == *ask) {
            return Some(w.arrived);
        }
        let from_source = waiting.iter().filter(|w| w.source == source).count();
        if waiting.len() >= MAX_WAITING || from_source >= MAX_WAITING_PER_SOURCE {
            debug!(
                source,
                "device carrier: no room to remember when this ask arrived"
            );
            return None;
        }
        waiting.push(Waiting {
            source: source.to_string(),
            ask: ask.clone(),
            arrived: now_micros,
        });
        Some(now_micros)
    }

    /// This ask from `source` waits no longer.
    fn waited(&self, source: &str, ask: &Ask) {
        self.waiting
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|w| !(w.source == source && w.ask == *ask));
    }

    /// Whether the timer is resting on a coordinator without the standing
    /// function. The rest ends when this process has applied a coordinator
    /// update since it began, or at its cap.
    pub fn resting(&self, coordinator_updates: u64) -> bool {
        let now = self.now();
        let mut reads = self.reads_lock();
        match reads.rest {
            Some((seen, until)) if seen == coordinator_updates && now < until => true,
            Some(_) => {
                info!("device carrier: asking the coordinator for this node's standing again");
                reads.rest = None;
                false
            }
            None => false,
        }
    }

    /// Take what one read answered: its evidence as of `began_micros`, when
    /// its turn came and before the cell was asked, and the next read paced
    /// from `done_micros`. A read the cell could not answer leaves the last
    /// one to age into unknown. A failing read says so once at warn and then
    /// at debug until it answers.
    fn note_read(
        &self,
        read: Result<Speaks, super::device_consent::CellFailure>,
        began_micros: i64,
        done_micros: i64,
        coordinator_updates: u64,
    ) {
        match read {
            Ok(speaks) => {
                let person = speaks.person().is_some();
                self.set_speaks(speaks, began_micros);
                let mut reads = self.reads_lock();
                reads.last = Some((done_micros, true));
                reads.landed += 1;
                reads.rest = None;
                if reads.failing.take().is_some() {
                    info!("device carrier: whom this node speaks for can be read again");
                }
                drop(reads);
                if person {
                    self.proof_wanted.notify_one();
                }
            }
            Err(why) => {
                let missing = super::device_consent_cell::standing_function_missing(&why);
                let mut reads = self.reads_lock();
                // The function's absence is an answer, and a lasting one.
                reads.last = Some((done_micros, missing));
                reads.landed += 1;
                if missing {
                    reads.rest = Some((
                        coordinator_updates,
                        done_micros.saturating_add(SPEAKS_REST_MAX_MICROS),
                    ));
                }
                if reads.failing.replace(missing) == Some(missing) {
                    debug!(
                        ?why,
                        "device carrier: whom this node speaks for still cannot be read"
                    );
                } else if missing {
                    warn!(
                        ?why,
                        "device carrier: this node's coordinator has no standing function, so it \
                         lists nothing until its coordinator is updated"
                    );
                } else {
                    warn!(
                        ?why,
                        "device carrier: whom this node speaks for could not be read"
                    );
                }
            }
        }
    }

    /// One read of whom this node speaks for, from its own cell. One runs at
    /// a time: a second caller waits for the first, and the time it is judged
    /// at is taken when its turn comes, not when it arrived, so callers that
    /// queued behind a slow read do not each read after it. With no cell
    /// (the conductor is not reachable yet) the turn is spent as a read the
    /// cell did not answer. Whether this call read.
    // bounded-work: one standing read, and one Human read when this node is a
    // controller. `Want::Now` is wanted only by a write that changed this
    // node's identity. `Want::Request`: at most one attempt per SPEAKS_REFRESH_SECS
    // however many callers. `Want::Ask`: at most one answered read per
    // SPEAKS_REFRESH_SECS and one unanswered per SPEAKS_RETRY_SECS.
    pub async fn read_turn(
        &self,
        cell: Option<&dyn ControllerCell>,
        names: &dyn IdentifierSource,
        want: Want,
        coordinator_updates: u64,
    ) -> bool {
        let landed = self.reads_lock().landed;
        let _one = self.reading.lock().await;
        let began = self.now();
        // A timer's turn that waited behind an identity write's read does
        // not read again on its heels.
        if want == Want::Timer && self.reads_lock().landed != landed {
            return false;
        }
        if !self.read_due(want, began) {
            return false;
        }
        // Spent before the cell is asked: a caller that goes away while the
        // read is out does not leave the next one free to follow at once.
        self.reads_lock().last = Some((began, false));
        let Some(cell) = cell else {
            debug!("device carrier: no cell to read whom this node speaks for from yet");
            return false;
        };
        self.set_self_key(cell.agent());
        let read = read_speaks(cell, names).await;
        self.note_read(read, began, self.now(), coordinator_updates);
        true
    }

    /// The timer's turn: a read, unless it is resting. Whether it read.
    pub async fn timer_turn(
        &self,
        cell: &dyn ControllerCell,
        names: &dyn IdentifierSource,
        coordinator_updates: u64,
    ) -> bool {
        if self.resting(coordinator_updates) {
            return false;
        }
        self.read_turn(Some(cell), names, Want::Timer, coordinator_updates)
            .await
    }

    /// Wait until the timer's next refresh is due: `SPEAKS_REFRESH_SECS`, or
    /// sooner when this node's identity may have changed.
    pub async fn until_refresh(&self) {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(SPEAKS_REFRESH_SECS)) => {}
            _ = self.speaks_changed.notified() => {}
        }
    }

    /// Wait until a read says this node speaks for someone.
    pub async fn proof_wanted(&self) {
        self.proof_wanted.notified().await
    }

    /// Make this node's listing proof when it speaks for someone and holds
    /// none for its key at its transport id: one mandate grant, once per key
    /// and transport id (adversarial review, high 3). One is made at a time,
    /// and a caller that finds one being made leaves it to that one. The
    /// signature waits on this node's chain with no deadline of its own, so
    /// this is never called with a read out (`prover`).
    pub async fn ensure_listed_proof(&self, cell: &dyn ControllerCell, link: &dyn CarrierLink) {
        let Ok(_one) = self.proving.try_lock() else {
            return;
        };
        let Some(tid) = link.self_transport() else {
            return;
        };
        let key = cell.agent();
        if self.speaks().person().is_none() || self.has_listed_proof(&key, &tid) {
            return;
        }
        match cell.sign_carrier(CarrierKind::Listed, "", &tid, "").await {
            Ok(proof) => {
                info!("device carrier: this node's key is proven over its transport id");
                self.set_listed_proof(&key, &tid, proof);
            }
            Err(why) => warn!(?why, "device carrier: the listing proof could not be made"),
        }
    }

    /// Record a read of whom this node speaks for, as of `now_micros` on the
    /// carrier's clock.
    pub fn set_speaks(&self, speaks: Speaks, now_micros: i64) {
        let mut slot = self.speaks.lock().unwrap_or_else(|e| e.into_inner());
        if slot.as_ref().map(|(s, _)| s) != Some(&speaks) {
            info!(speaks = %speaks.words(), "device carrier: whom this node speaks for");
        }
        *slot = Some((speaks, now_micros));
    }

    /// This node's identity may have changed (begun, bootstrapped, enrolled,
    /// declared): read whom it speaks for again now.
    pub fn identity_changed(&self) {
        self.speaks_changed.notify_one();
    }

    fn announce_lock(&self) -> std::sync::MutexGuard<'_, Option<Announce>> {
        self.announce.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Begin announcing `request`, replacing any earlier announce.
    pub fn start_announce(&self, announce: Announce) {
        info!(
            device = %consent_grant::hash_shape::fingerprint(&announce.request.device_key),
            state = ?announce.state,
            "device carrier: announcing an ask on the private network"
        );
        *self.announce_lock() = Some(announce);
    }

    pub fn stop_announce(&self) {
        *self.announce_lock() = None;
    }

    /// The current announce, dropped once its five minutes are up.
    pub fn announce(&self) -> Option<Announce> {
        let mut slot = self.announce_lock();
        if slot.as_ref().is_some_and(|a| a.expired(now_micros())) {
            info!("device carrier: the announce ended unanswered after five minutes");
            *slot = None;
        }
        slot.clone()
    }

    /// Keep this node's listing proof for `key` at `transport_id`.
    pub fn set_listed_proof(&self, key: &str, transport_id: &str, proof: CarrierProof) {
        *self.listed_proof.lock().unwrap_or_else(|e| e.into_inner()) =
            Some((key.to_string(), transport_id.to_string(), proof));
    }

    /// Whether this node holds a listing proof for `key` at `transport_id`.
    pub fn has_listed_proof(&self, key: &str, transport_id: &str) -> bool {
        self.listed_proof
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|(k, t, _)| k == key && t == transport_id)
    }

    fn listed_proof_for(&self, key: Option<&str>) -> Option<CarrierProof> {
        let slot = self.listed_proof.lock().unwrap_or_else(|e| e.into_inner());
        slot.as_ref()
            .filter(|(k, _, _)| Some(k.as_str()) == key)
            .map(|(_, _, p)| p.clone())
    }

    /// The approver the carrier proved for the request with `state`, once
    /// this node redeemed its code.
    pub fn proven_approver(&self, state: &str) -> Option<String> {
        self.proven
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .filter(|(s, _)| s == state)
            .map(|(_, k)| k.clone())
    }

    fn set_proven(&self, state: &str, approver: &str) {
        *self.proven.lock().unwrap_or_else(|e| e.into_inner()) =
            Some((state.to_string(), approver.to_string()));
    }

    /// Record that the approving node at `source` listed this node's ask,
    /// when its key is proven over `source` (adversarial review, high 3) and
    /// is the declared approver, if one was declared. Whether it was taken.
    pub fn note_listed(
        &self,
        source: &str,
        approver: Option<String>,
        proof: Option<&CarrierProof>,
        number: u32,
    ) -> bool {
        let mut slot = self.announce_lock();
        let Some(a) = slot.as_mut() else {
            return false;
        };
        let Some(key) = approver.filter(|k| {
            proof.is_some_and(|p| proof_holds(p, k, CarrierKind::Listed, "", source, ""))
        }) else {
            warn!(
                source,
                "device carrier: a listing came without its key's proof; not taken"
            );
            return false;
        };
        if a.for_approver.as_ref().is_some_and(|wanted| wanted != &key) {
            warn!(
                source,
                "device carrier: listed by a node other than the declared approver"
            );
            return false;
        }
        a.listed_by.retain(|(s, _, _)| s != source);
        a.listed_by.push((source.to_string(), Some(key), number));
        true
    }

    /// Answer one carrier request from the peer `source`. `None` is silence:
    /// the asking device is told nothing.
    pub fn on_request(&self, source: &str, request: CarryRequest) -> Option<CarryResponse> {
        Some(match request {
            CarryRequest::Ask { ask } => return self.on_ask(source, ask),
            CarryRequest::Code {
                state,
                code,
                approver,
                proof,
            } => self.on_code(source, &state, code, approver, proof),
            CarryRequest::Declined {
                state,
                approver,
                proof,
            } => self.on_declined(source, &state, approver, proof),
            CarryRequest::Redeem { redemption } => {
                // bounded-work: one sweep over deliveries that each live minutes.
                self.deliveries.sweep(now_micros());
                match redeem(&self.deliveries, &redemption, now_micros()) {
                    Ok(delivered) => {
                        info!(
                            source,
                            "device carrier: a code was redeemed over the carrier"
                        );
                        CarryResponse::Delivered {
                            delivered: Box::new(delivered),
                        }
                    }
                    Err(r) => {
                        warn!(
                            source,
                            code = r.code(),
                            "device carrier: redemption refused"
                        );
                        CarryResponse::Refused {
                            code: r.code().to_string(),
                        }
                    }
                }
            }
        })
    }

    fn on_ask(&self, source: &str, ask: Ask) -> Option<CarryResponse> {
        let device = consent_grant::hash_shape::fingerprint(&ask.request.device_key);
        let now = (self.wall)();
        let speaks = self.speaks_for_ask(source, &ask, now);
        match self.pending.admit(
            ask,
            source,
            self.self_key(),
            &speaks,
            &super::device_consent::policy(),
            now,
        ) {
            Ok(listed) => {
                let (number, how) = match listed {
                    Listed::New(n) => (n, "listed"),
                    Listed::Replaced(n) => (n, "replaced its earlier ask"),
                };
                info!(
                    source,
                    device, number, "device carrier: ask received and {how}"
                );
                Some(CarryResponse::Listed {
                    number,
                    approver: self.self_key().map(str::to_string),
                    proof: self.listed_proof_for(self.self_key()),
                })
            }
            Err(dropped) => {
                let code = dropped.code();
                match dropped {
                    // Not an approving node for anyone: dropped, and the
                    // asking device is told nothing.
                    Dropped::SpeaksForNobody | Dropped::StandingUnknown => {
                        info!(
                            source,
                            device, code, "device carrier: ask received and dropped unanswered"
                        );
                        return None;
                    }
                    Dropped::Malformed(_)
                    | Dropped::NotForThisApprover
                    | Dropped::AlreadyDecided
                    | Dropped::Unsigned
                    | Dropped::ListedFromElsewhere => {
                        info!(
                            source,
                            device, code, "device carrier: ask received and dropped"
                        )
                    }
                    Dropped::Full | Dropped::SourceOverLimit => {
                        warn!(
                            source,
                            device, code, "device carrier: ask dropped, list bounded"
                        )
                    }
                }
                Some(CarryResponse::Refused {
                    code: code.to_string(),
                })
            }
        }
    }

    /// Whether a code or decline from `source` for `state` is the listing
    /// node's own: that node listed the ask with its key proven, the message
    /// names that key, and its proof holds over (kind, state, `source`).
    /// Answers with the key, or the refusal code.
    fn proven_from(
        a: &Announce,
        source: &str,
        state: &str,
        kind: CarrierKind,
        approver: Option<&str>,
        proof: Option<&CarrierProof>,
    ) -> Result<String, &'static str> {
        if a.request.state != state {
            return Err("carry_not_this_ask");
        }
        let Some(listed) = a
            .listed_by
            .iter()
            .find(|(s, _, _)| s == source)
            .and_then(|(_, k, _)| k.clone())
        else {
            return Err("carry_from_unlisting_peer");
        };
        if approver != Some(listed.as_str()) {
            return Err("carry_from_another_key");
        }
        if !proof.is_some_and(|p| proof_holds(p, &listed, kind, state, source, "")) {
            return Err("carry_unproven");
        }
        if a.for_approver.as_ref().is_some_and(|w| w != &listed) {
            return Err("carry_not_the_declared_approver");
        }
        Ok(listed)
    }

    /// A code for this node's own ask. Taken only from the approving node
    /// that listed it, signed by the key it listed with (adversarial review,
    /// high 3), so another peer cannot preempt the real code with its own.
    fn on_code(
        &self,
        source: &str,
        state: &str,
        code: String,
        approver: Option<String>,
        proof: Option<CarrierProof>,
    ) -> CarryResponse {
        let mut slot = self.announce_lock();
        let refused = |code: &str| CarryResponse::Refused { code: code.into() };
        let Some(a) = slot.as_mut() else {
            return refused("carry_no_announce");
        };
        match Self::proven_from(
            a,
            source,
            state,
            CarrierKind::Code,
            approver.as_deref(),
            proof.as_ref(),
        ) {
            Ok(key) => {
                info!(
                    source,
                    approver = %consent_grant::hash_shape::fingerprint(&key),
                    "device carrier: the approving node handed the code back"
                );
                a.code = Some((code, source.to_string()));
                a.code_approver = Some(key);
                CarryResponse::CodeTaken
            }
            Err(why) => {
                warn!(source, why, "device carrier: a code was refused");
                refused(why)
            }
        }
    }

    /// A decline for this node's own ask, taken only as a code is. The
    /// announce stops; nothing was signed.
    fn on_declined(
        &self,
        source: &str,
        state: &str,
        approver: Option<String>,
        proof: Option<CarrierProof>,
    ) -> CarryResponse {
        let mut slot = self.announce_lock();
        let refused = |code: &str| CarryResponse::Refused { code: code.into() };
        let Some(a) = slot.as_mut() else {
            return refused("carry_no_announce");
        };
        match Self::proven_from(
            a,
            source,
            state,
            CarrierKind::Declined,
            approver.as_deref(),
            proof.as_ref(),
        ) {
            Ok(key) => {
                info!(
                    source,
                    "device carrier: the approving node declined the ask"
                );
                a.declined_by = Some(Some(key));
                CarryResponse::DeclineTaken
            }
            Err(why) => {
                warn!(source, why, "device carrier: a decline was refused");
                refused(why)
            }
        }
    }
}

// =============================================================================
// The routes: this machine only
// =============================================================================

use async_trait::async_trait;
use consent_grant::{
    decide, ByAnswer, ByDeclaration, Decider, Decision, Declaration, NoElohimAttending,
    PendingView, Redemption, RequestedAct, ReturnPath, ReturnTargetView, WitnessBeat,
};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use serde::Deserialize;

use super::device_consent::{agree_request, refusal, ControllerCell, IdentifierSource, Refused};
use super::response;

/// How the carrier reaches peers. The libp2p handle in production; a fake in
/// tests.
#[async_trait]
pub trait CarrierLink: Send + Sync {
    /// The peers local discovery found on the private network.
    fn peers(&self) -> Vec<String>;
    /// This node's own transport id, as its peers see it: what its carrier
    /// statements name.
    fn self_transport(&self) -> Option<String>;
    async fn send(&self, peer: &str, request: CarryRequest) -> Result<CarryResponse, String>;
}

#[async_trait]
impl CarrierLink for crate::p2p::P2PHandle {
    fn peers(&self) -> Vec<String> {
        self.private_network_peers()
    }
    fn self_transport(&self) -> Option<String> {
        Some(self.local_peer_id()).filter(|id| !id.is_empty())
    }
    async fn send(&self, peer: &str, request: CarryRequest) -> Result<CarryResponse, String> {
        self.device_carry(peer, request).await
    }
}

fn carrier_absent() -> Response<Full<Bytes>> {
    refusal(
        StatusCode::CONFLICT,
        "this node has no local discovery in its transport mode, so nothing on the private \
         network can carry an ask; use the link instead",
        "carrier_absent",
    )
}

/// `GET /auth/consent/pending`: what is asking this node, and whose
/// identity an approval here would be for. A node that speaks for nobody
/// lists nothing and says so.
pub fn pending_list(link: Option<&dyn CarrierLink>, speaks: &Speaks) -> Response<Full<Bytes>> {
    let now = now_micros();
    let asks: Vec<PendingView> = match speaks {
        Speaks::Person(_) => carrier()
            .pending
            .list(now)
            .iter()
            .map(|p| PendingView::of(p, now, speaks))
            .collect(),
        Speaks::Nobody | Speaks::Unknown => Vec::new(),
    };
    response::ok(&serde_json::json!({
        "carrier": if link.is_some() { "private-network" } else { "absent" },
        "approver": carrier().self_key(),
        "speaksFor": speaks,
        "speaksForWords": speaks.words(),
        "asks": asks,
    }))
}

/// Whom this node speaks for, from its own cell: one standing read, and one
/// read of its Human only when it is a controller, which `names` names by the
/// person's sign-in word (a claim, shown only). The cell's failure when it
/// cannot answer.
pub async fn read_speaks(
    cell: &dyn ControllerCell,
    names: &dyn IdentifierSource,
) -> Result<Speaks, super::device_consent::CellFailure> {
    use super::device_consent::CellStanding;
    let me = cell.agent();
    Ok(match cell.standing().await? {
        CellStanding::Ready(standing) => {
            let person = if standing.controllers.contains(&me) {
                cell.my_human().await.ok().flatten()
            } else {
                None
            };
            let identifier = person
                .as_ref()
                .and_then(|p| super::device_consent::identifier_of(names, p, &me));
            Speaks::of(Some(&standing), &me, person.as_ref(), identifier)
        }
        CellStanding::NoPerson | CellStanding::Unbootstrapped { .. } => Speaks::Nobody,
    })
}

/// This node's key's proof over (kind, state, its transport id), for a code
/// or a decline it hands back. `None` (logged) when it cannot be made: the
/// device then refuses what it is handed.
async fn handback_proof(
    cell: Option<&dyn ControllerCell>,
    link: &dyn CarrierLink,
    kind: CarrierKind,
    state: &str,
) -> Option<CarrierProof> {
    let (Some(cell), Some(tid)) = (cell, link.self_transport()) else {
        warn!("device carrier: no cell or transport id to prove a hand-back with");
        return None;
    };
    match cell.sign_carrier(kind, state, &tid, "").await {
        Ok(proof) => Some(proof),
        Err(why) => {
            warn!(?why, "device carrier: a hand-back proof could not be made");
            None
        }
    }
}

/// Run one of the carrier's workers for the life of the process: if it
/// panics, say so at error and start it again after `WORKER_RESTART_SECS`. A
/// worker that is gone is a node that never lists again, or never proves
/// its listing, with nothing to show for it.
// bounded-work: one restart per WORKER_RESTART_SECS at most.
pub async fn supervised<F, Fut>(what: &'static str, run: F)
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    loop {
        match tokio::spawn(run()).await {
            Err(gone) if gone.is_panic() => {
                tracing::error!(
                    worker = what,
                    "device carrier: a worker panicked; starting it again"
                );
                tokio::time::sleep(std::time::Duration::from_secs(WORKER_RESTART_SECS)).await;
            }
            // Returned, or the runtime is shutting down.
            _ => return,
        }
    }
}

/// How long after a carrier worker panics it is started again.
pub const WORKER_RESTART_SECS: u64 = 5;

/// The demand reader: each time an ask wants whom this node speaks for, one
/// turn. `cell` is this node's own cell when its conductor is reachable.
// bounded-work: nothing while no ask wants a read; then `Carrier::read_turn`.
pub async fn demand_reader<C, F>(
    carrier: &Carrier,
    cell: F,
    names: &dyn IdentifierSource,
    coordinator_updates: fn() -> u64,
) where
    C: ControllerCell,
    F: Fn() -> Option<C> + Send + Sync,
{
    loop {
        carrier.speaks_wanted().await;
        let cell = cell();
        carrier
            .read_turn(
                cell.as_ref().map(|c| c as &dyn ControllerCell),
                names,
                Want::Ask,
                coordinator_updates(),
            )
            .await;
    }
}

/// The timer: a turn every `SPEAKS_REFRESH_SECS`, or sooner when this node's
/// identity may have changed, whether or not anyone asks.
// bounded-work: one standing read (and one Human read when this node is a
// controller) per SPEAKS_REFRESH_SECS; never per incoming ask, and none while
// resting on a coordinator without the function.
pub async fn timer_reader<C, F>(
    carrier: &Carrier,
    cell: F,
    names: &dyn IdentifierSource,
    coordinator_updates: fn() -> u64,
) where
    C: ControllerCell,
    F: Fn() -> Option<C> + Send + Sync,
{
    loop {
        if let Some(cell) = cell() {
            carrier
                .timer_turn(&cell, names, coordinator_updates())
                .await;
        }
        carrier.until_refresh().await;
    }
}

/// The listing proof's own worker, in either mode: woken by a read that says
/// this node speaks for someone, apart from the reads so that a signature
/// waiting on this node's chain holds up no read.
// bounded-work: one mandated signature per key and transport id for the
// process's life; tried at most once per answered read until it is made.
pub async fn prover<C, F>(carrier: &Carrier, cell: F, link: &dyn CarrierLink)
where
    C: ControllerCell,
    F: Fn() -> Option<C> + Send + Sync,
{
    loop {
        carrier.proof_wanted().await;
        if let Some(cell) = cell() {
            carrier.ensure_listed_proof(&cell, link).await;
        }
    }
}

/// The body of `POST /auth/consent/pending/decide`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DecideInput {
    /// The ask's number, its device key, or that key's fingerprint.
    ask: String,
    /// An answer given on this machine, taken as the person's: the acts agreed,
    /// none meaning decline. Absent, the other deciders are asked.
    #[serde(default)]
    answer: Option<AnswerInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnswerInput {
    agreed_acts: Vec<RequestedAct>,
}

/// `POST /auth/consent/pending/decide`: decide one waiting ask, and when it
/// is agreed, agree through the agree path and hand the code back over the
/// carrier to the peer that brought the ask.
#[allow(clippy::too_many_arguments)]
pub async fn decide_pending(
    store: &MemoryStore,
    cell: Option<&dyn ControllerCell>,
    beat: &dyn WitnessBeat,
    link: Option<&dyn CarrierLink>,
    declaration: &Declaration,
    speaks: &Speaks,
    signed_in: bool,
    body: &[u8],
) -> Response<Full<Bytes>> {
    match decide_inner(
        store,
        cell,
        beat,
        link,
        declaration,
        speaks,
        signed_in,
        body,
    )
    .await
    {
        Ok(answer) => answer,
        Err(refused) => *refused,
    }
}

#[allow(clippy::too_many_arguments)]
async fn decide_inner(
    store: &MemoryStore,
    cell: Option<&dyn ControllerCell>,
    beat: &dyn WitnessBeat,
    link: Option<&dyn CarrierLink>,
    declaration: &Declaration,
    speaks: &Speaks,
    signed_in: bool,
    body: &[u8],
) -> Result<Response<Full<Bytes>>, Refused> {
    if speaks.person().is_none() {
        return Err(Box::new(refusal(
            StatusCode::CONFLICT,
            &speaks.words(),
            "node_speaks_for_nobody",
        )));
    }
    if !signed_in {
        return Err(Box::new(refusal(
            StatusCode::UNAUTHORIZED,
            "sign in on this node before deciding an ask",
            "consent_not_signed_in",
        )));
    }
    let input: DecideInput = serde_json::from_slice(body)
        .map_err(|e| Box::new(response::bad_request(&format!("Invalid JSON: {e}"))))?;
    let now = now_micros();
    let pending = carrier().pending.pick(&input.ask, now).ok_or_else(|| {
        Box::new(refusal(
            StatusCode::NOT_FOUND,
            "nothing is asking under that number or key; it may have expired",
            "pending_unknown",
        ))
    })?;
    let by_answer = input.answer.map(|a| ByAnswer(a.agreed_acts));
    let by_declaration = ByDeclaration(declaration);
    let mut chain: Vec<&dyn Decider> = vec![&NoElohimAttending];
    match &by_answer {
        // Whoever answers here is present and answering; their answer stands.
        Some(answer) => chain.push(answer),
        None => chain.push(&by_declaration),
    }
    let decision = match decide(&chain, &pending) {
        Ok(decision) => decision,
        Err(reasons) => {
            info!(
                number = pending.number,
                ?reasons,
                "device carrier: ask left for an answer"
            );
            return Ok(response::json_response(
                StatusCode::CONFLICT,
                &serde_json::json!({
                    "error": reasons.last().cloned().unwrap_or_default(),
                    "code": "pending_needs_answer",
                    "reasons": reasons,
                    "ask": PendingView::of(&pending, now, speaks),
                }),
            ));
        }
    };
    let (acts, by) = match decision {
        Decision::Decline { by } => {
            // Nothing is signed. The ask leaves the list and is remembered,
            // and the asking device is told, so its terminal stops waiting.
            carrier().pending.decided(pending.number, now);
            info!(number = pending.number, ?by, "device carrier: ask declined");
            let told = match link {
                Some(link) => {
                    let state = pending.ask.request.state.clone();
                    let proof = handback_proof(cell, link, CarrierKind::Declined, &state).await;
                    link.send(
                        &pending.source,
                        CarryRequest::Declined {
                            state,
                            approver: carrier().self_key().map(str::to_string),
                            proof,
                        },
                    )
                    .await
                }
                None => Err("this node has no carrier".into()),
            };
            let told = match told {
                Ok(CarryResponse::DeclineTaken) => serde_json::json!({ "taken": true }),
                Ok(other) => serde_json::json!({ "taken": false, "answer": other }),
                Err(why) => serde_json::json!({ "taken": false, "error": why }),
            };
            info!(number = pending.number, told = %told, "device carrier: decline handed back");
            return Ok(response::ok(&serde_json::json!({
                "number": pending.number, "decidedBy": by, "declined": true, "handedBack": told,
            })));
        }
        Decision::Agree { acts, by } => (acts, by),
        Decision::Defer { .. } => unreachable!("decide never returns a deferral"),
    };
    if pending.ask.request.return_path != ReturnPath::Paste {
        return Err(Box::new(refusal(
            StatusCode::BAD_REQUEST,
            "an ask carried here must take its code back the same way",
            "carry_needs_paste_return",
        )));
    }
    let agreed = agree_request(
        store,
        cell,
        beat,
        &pending.ask.request,
        Some(pending.ask.state),
        acts,
        now,
    )
    .await?;
    carrier().pending.decided(pending.number, now);
    info!(
        number = pending.number,
        ?by,
        "device carrier: ask decided and agreed"
    );
    let ReturnTargetView::Display { value } = &agreed.return_target else {
        return Err(Box::new(response::internal_error(
            "an announced ask got no code",
        )));
    };
    let code = value.split('#').next().unwrap_or_default().to_string();
    let handed_back = match link {
        Some(link) => {
            let state = pending.ask.request.state.clone();
            let proof = handback_proof(cell, link, CarrierKind::Code, &state).await;
            link.send(
                &pending.source,
                CarryRequest::Code {
                    state,
                    code,
                    approver: carrier().self_key().map(str::to_string),
                    proof,
                },
            )
            .await
        }
        None => Err("this node has no carrier".into()),
    };
    let handed_back = match handed_back {
        Ok(CarryResponse::CodeTaken) => serde_json::json!({ "taken": true }),
        Ok(other) => serde_json::json!({ "taken": false, "answer": other }),
        Err(why) => serde_json::json!({ "taken": false, "error": why }),
    };
    info!(number = pending.number, handed_back = %handed_back, "device carrier: code handed back");
    Ok(response::ok(&serde_json::json!({
        "number": pending.number,
        "decidedBy": by,
        "agreed": agreed,
        "handedBack": handed_back,
    })))
}

/// The body of `POST /auth/device/announce`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnnounceInput {
    request: GrantRequest,
    #[serde(default)]
    approver_key: Option<String>,
}

/// `POST /auth/device/announce`: begin offering this node's ask to the
/// approving nodes local discovery finds. `state` is what this node says about itself.
pub async fn start_announce(
    link: Option<&dyn CarrierLink>,
    cell: Option<&dyn super::device_consent::DeviceCell>,
    state: NodeState,
    body: &[u8],
) -> Result<(Response<Full<Bytes>>, bool), Refused> {
    let Some(link) = link else {
        return Err(Box::new(carrier_absent()));
    };
    let input: AnnounceInput = serde_json::from_slice(body)
        .map_err(|e| Box::new(response::bad_request(&format!("Invalid JSON: {e}"))))?;
    consent_grant::admit_request(&input.request, &super::device_consent::policy()).map_err(
        |r| {
            Box::new(refusal(
                StatusCode::BAD_REQUEST,
                "the request would be refused by any approving node",
                r.code(),
            ))
        },
    )?;
    if input.request.return_path != ReturnPath::Paste {
        return Err(Box::new(refusal(
            StatusCode::BAD_REQUEST,
            "an announced ask takes its code back over the carrier, so it asks for paste",
            "carry_needs_paste_return",
        )));
    }
    // The ask is this device's own, from here (adversarial review, high 4):
    // its key signs (ask, state, its transport id, the PKCE challenge).
    let (Some(cell), Some(tid)) = (cell, link.self_transport()) else {
        return Err(Box::new(refusal(
            StatusCode::SERVICE_UNAVAILABLE,
            "this node cannot sign its ask yet (no cell or no transport id); try again shortly",
            "carry_cannot_sign_ask",
        )));
    };
    let proof = cell
        .sign_carrier(
            CarrierKind::Ask,
            &input.request.state,
            &tid,
            &input.request.code_challenge,
        )
        .await
        .map_err(|why| {
            warn!(?why, "device carrier: the ask could not be signed");
            Box::new(refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                "this node's key could not sign its ask; try again shortly",
                "carry_cannot_sign_ask",
            ))
        })?;
    carrier().start_announce(Announce {
        request: input.request,
        state,
        for_approver: input.approver_key,
        started_at_micros: now_micros(),
        proof: Some(proof),
        listed_by: Vec::new(),
        code: None,
        code_approver: None,
        declined_by: None,
    });
    Ok((
        response::json_response(
            StatusCode::ACCEPTED,
            &serde_json::json!({ "state": state, "peers": link.peers().len() }),
        ),
        true,
    ))
}

/// The most peers one announce round offers its ask to.
const MAX_PEERS_PER_ROUND: usize = 16;
/// How often an announce offers its ask again.
const ROUND_SECS: u64 = 5;

/// Offer the current announce to every private-network peer, every few
/// seconds, until a code arrives, the five minutes are up, or it is stopped.
pub async fn run_announce(link: std::sync::Arc<dyn CarrierLink>) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static RUNNING: AtomicBool = AtomicBool::new(false);
    // One loop serves whatever announce is current; a second start reuses it.
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    announce_rounds(link).await;
    RUNNING.store(false, Ordering::SeqCst);
}

async fn announce_rounds(link: std::sync::Arc<dyn CarrierLink>) {
    // bounded-work: at most MAX_PEERS_PER_ROUND messages per ROUND_SECS, for
    // at most ASK_LIFE_MICROS; each message waits at most DEVICE_CARRY_TIMEOUT.
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(ROUND_SECS));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        ticker.tick().await;
        let Some(announce) = carrier().announce() else {
            return;
        };
        if announce.code.is_some() || announce.declined_by.is_some() {
            return;
        }
        for peer in link.peers().into_iter().take(MAX_PEERS_PER_ROUND) {
            match link
                .send(
                    &peer,
                    CarryRequest::Ask {
                        ask: announce.ask(),
                    },
                )
                .await
            {
                Ok(CarryResponse::Listed {
                    number,
                    approver,
                    proof,
                }) => {
                    carrier().note_listed(&peer, approver, proof.as_ref(), number);
                }
                Ok(CarryResponse::Refused { code }) => {
                    tracing::debug!(peer, code, "device carrier: a peer did not list the ask")
                }
                Ok(_) => {}
                Err(why) => tracing::debug!(peer, why, "device carrier: a peer did not answer"),
            }
        }
    }
}

/// `GET /auth/device/announce`: how this node's ask is doing.
pub fn announce_status() -> Response<Full<Bytes>> {
    let Some(a) = carrier().announce() else {
        return response::ok(&serde_json::json!({ "status": "none" }));
    };
    let left = ((a.started_at_micros + ASK_LIFE_MICROS - now_micros()) / 1_000_000).max(0);
    let listed_by: Vec<_> = a
        .listed_by
        .iter()
        .map(|(_, approver, number)| {
            serde_json::json!({
                "approver": approver,
                "approverFingerprint": approver.as_deref().map(consent_grant::hash_shape::fingerprint),
                "number": number,
            })
        })
        .collect();
    let status = match (&a.code, &a.declined_by, a.listed_by.is_empty()) {
        (Some(_), _, _) => "code",
        (None, Some(_), _) => "declined",
        (None, None, false) => "listed",
        (None, None, true) => "announcing",
    };
    let declined_by = a.declined_by.as_ref().map(|approver| {
        serde_json::json!({
            "approver": approver,
            "approverFingerprint": approver.as_deref().map(consent_grant::hash_shape::fingerprint),
        })
    });
    response::ok(&serde_json::json!({
        "status": status,
        "secondsLeft": left,
        "state": a.state,
        "listedBy": listed_by,
        "code": a.code.as_ref().map(|(c, _)| format!("{c}#{}", a.request.state)),
        "codeApprover": a.code_approver,
        "codeApproverFingerprint": a.code_approver.as_deref().map(consent_grant::hash_shape::fingerprint),
        "declinedBy": declined_by,
    }))
}

/// The body of `POST /auth/device/announce/redeem`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RedeemInput {
    redemption: Redemption,
}

/// `POST /auth/device/announce/redeem`: redeem the code at the approving node that
/// handed it back, over the carrier, with the terminal's verifier.
pub async fn announce_redeem(link: Option<&dyn CarrierLink>, body: &[u8]) -> Response<Full<Bytes>> {
    let Some(link) = link else {
        return carrier_absent();
    };
    let input: RedeemInput = match serde_json::from_slice(body) {
        Ok(i) => i,
        Err(e) => return response::bad_request(&format!("Invalid JSON: {e}")),
    };
    let Some((state, (_, source), approver)) = carrier()
        .announce()
        .and_then(|a| Some((a.request.state.clone(), a.code?, a.code_approver)))
    else {
        return refusal(
            StatusCode::CONFLICT,
            "no approving node has handed a code back yet",
            "carry_no_code",
        );
    };
    match link
        .send(
            &source,
            CarryRequest::Redeem {
                redemption: input.redemption,
            },
        )
        .await
    {
        Ok(CarryResponse::Delivered { delivered }) => {
            // What enrolling checks the consent against: the key whose proof
            // came with the code.
            if let Some(approver) = &approver {
                carrier().set_proven(&state, approver);
            }
            carrier().stop_announce();
            response::ok(&*delivered)
        }
        Ok(CarryResponse::Refused { code }) => refusal(
            StatusCode::BAD_REQUEST,
            "the approving node would not hand over the consent",
            &code,
        ),
        Ok(_) => refusal(
            StatusCode::BAD_GATEWAY,
            "the approving node answered something else",
            "carry_unexpected_answer",
        ),
        Err(why) => refusal(
            StatusCode::BAD_GATEWAY,
            &format!("the approving node could not be reached: {why}"),
            "carry_unreachable",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use consent_grant::{pkce, GRANT_DOMAIN};

    fn request() -> GrantRequest {
        GrantRequest {
            domain: GRANT_DOMAIN.into(),
            client_id: "epr-cli".into(),
            device_key: "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl".into(),
            device_root_key: None,
            label: "home".into(),
            network_dna: "uhC0kQwOEwmIBZhBT3I7vGZPz0kEL3_hqavyFW0upoO_hyEwuEglj".into(),
            content_dna: "uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt".into(),
            acts: vec![consent_grant::RequestedAct::EnrollDevice],
            code_challenge: pkce::challenge(&"v".repeat(43)),
            state: "s".repeat(32),
            return_path: ReturnPath::Paste,
        }
    }

    fn fresh() -> Carrier {
        Carrier::new()
    }

    /// A real key `n`, and its agent key text.
    fn real(n: u8) -> (elohim_epr::proof::AgentKeypair, String) {
        let kp = elohim_epr::proof::AgentKeypair::from_secret(&[n; 32]).unwrap();
        let agent =
            holochain_types::prelude::AgentPubKey::from_raw_32(kp.public_key_bytes().to_vec())
                .to_string();
        (kp, agent)
    }

    /// Key `n`'s proof over a carrier statement.
    fn proof_by(n: u8, kind: CarrierKind, state: &str, tid: &str, challenge: &str) -> CarrierProof {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let (kp, agent) = real(n);
        let message = consent_grant::statement(kind, state, tid, challenge).unwrap();
        CarrierProof {
            signer: agent,
            signature: URL_SAFE_NO_PAD.encode(elohim_epr::proof::sign(&kp, &message)),
        }
    }

    /// `request` asked by its device (key `n`), signed as sent from `tid`.
    fn signed_ask(n: u8, mut request: GrantRequest, tid: &str) -> Ask {
        request.device_key = real(n).1;
        let proof = proof_by(
            n,
            CarrierKind::Ask,
            &request.state,
            tid,
            &request.code_challenge,
        );
        Ask {
            request,
            state: NodeState::Unassigned,
            for_approver: None,
            proof: Some(proof),
        }
    }

    /// An announce of `request()` with nothing yet listed.
    fn announcing() -> Announce {
        Announce {
            request: request(),
            state: NodeState::Unassigned,
            for_approver: None,
            started_at_micros: now_micros(),
            proof: None,
            listed_by: vec![],
            code: None,
            code_approver: None,
            declined_by: None,
        }
    }

    /// A node that speaks for the fake cell's person.
    fn speaking() -> Speaks {
        let ConsentStanding::Ready(standing) = ready() else {
            unreachable!()
        };
        Speaks::of(
            Some(&standing),
            crate::services::device_consent::tests::CONTROLLER,
            None,
            None,
        )
    }

    #[test]
    fn an_approving_node_lists_an_ask_and_says_which_it_is() {
        let c = fresh();
        c.set_self_key("uhCAkApprover".into());
        c.set_speaks(speaking(), c.now());
        let ask = signed_ask(31, request(), "peer-a");
        assert_eq!(
            c.on_request("peer-a", CarryRequest::Ask { ask: ask.clone() }),
            Some(CarryResponse::Listed {
                number: 1,
                approver: Some("uhCAkApprover".into()),
                proof: None,
            })
        );
        let mut elsewhere = signed_ask(32, request(), "peer-a");
        elsewhere.for_approver = Some("uhCAkOther".into());
        assert_eq!(
            c.on_request("peer-a", CarryRequest::Ask { ask: elsewhere }),
            Some(CarryResponse::Refused {
                code: "ask_for_another_approver".into()
            })
        );
        assert_eq!(c.pending.len(), 1);
    }

    #[test]
    fn a_node_that_speaks_for_nobody_drops_asks_and_answers_nothing() {
        let c = fresh();
        c.set_self_key(key(60));
        let ask = || CarryRequest::Ask {
            ask: signed_ask(33, request(), "peer-a"),
        };
        // Never read: unknown, so nothing is listed and nothing is said.
        assert_eq!(c.speaks(), Speaks::Unknown);
        assert_eq!(c.on_request("peer-a", ask()), None);
        // A node that speaks for nobody.
        c.set_speaks(Speaks::Nobody, c.now());
        assert_eq!(c.on_request("peer-a", ask()), None);
        assert!(c.pending.is_empty());
        // A read too old counts as unknown.
        c.set_speaks(speaking(), c.now() - SPEAKS_MAX_AGE_MICROS);
        assert_eq!(c.speaks(), Speaks::Unknown);
        assert_eq!(c.on_request("peer-a", ask()), None);
        // Fresh, and a controller: listed.
        c.set_speaks(speaking(), c.now());
        assert!(matches!(
            c.on_request("peer-a", ask()),
            Some(CarryResponse::Listed { number: 1, .. })
        ));
    }

    /// What a node recorded when its identity was begun: `matthew`.
    struct Names;
    impl IdentifierSource for Names {
        fn recorded(&self, human_id: &str, _: &str) -> Option<String> {
            (human_id == "h-1").then(|| "matthew".to_string())
        }
        fn declared(&self) -> Option<consent_grant::DeclaredIdentity> {
            None
        }
    }

    #[tokio::test]
    async fn whom_a_node_speaks_for_is_read_from_its_own_cell() {
        use crate::services::device_consent::tests::{CONTROLLER, IDENTITY};
        let ready_cell = FakeCell::new(Ok(ready()));
        // No Human to name: nobody's word is read.
        let read = read_speaks(&ready_cell, &Names).await.unwrap();
        assert_eq!(read.person().unwrap().identity_root, IDENTITY);
        assert_eq!(read.person().unwrap().identifier, None);
        // With the node's own Human, the word it recorded names the person.
        ready_cell
            .humans
            .lock()
            .unwrap()
            .push(crate::services::device_consent::NewHuman {
                id: "h-1".into(),
                display_name: "Matthew".into(),
                profile_reach: "private".into(),
            });
        let named = read_speaks(&ready_cell, &Names).await.unwrap();
        let person = named.person().unwrap();
        assert_eq!(person.identifier.as_deref(), Some("matthew"));
        assert!(named
            .words()
            .starts_with("An approval here is for matthew (Matthew),"));
        // The cell's key resolves to an identity it does not speak for: neither
        // a root controller nor a device whose joining record stands.
        let joined = FakeCell::new(Ok(ConsentStanding::Ready(
            consent_grant::ControllerStanding {
                identity_root: IDENTITY.into(),
                authority: "a".into(),
                network_dna: "n".into(),
                controllers: vec![key(61)],
                required: 1,
                speaks_via: None,
                also_speaks_for: vec![],
            },
        )));
        assert_ne!(key(61), CONTROLLER);
        assert_eq!(read_speaks(&joined, &Names).await, Ok(Speaks::Nobody));
        let none = FakeCell::new(Ok(ConsentStanding::NoPerson));
        assert_eq!(read_speaks(&none, &Names).await, Ok(Speaks::Nobody));
        let down = FakeCell::new(Err(
            crate::services::device_consent::CellFailure::Unavailable("down".into()),
        ));
        assert!(read_speaks(&down, &Names).await.is_err());
    }

    #[tokio::test]
    async fn the_list_says_whom_the_node_speaks_for_and_a_node_for_nobody_lists_nothing() {
        let bytes = |r: Response<Full<Bytes>>| async move {
            let b = r.into_body().collect().await.unwrap().to_bytes();
            serde_json::from_slice::<serde_json::Value>(&b).unwrap()
        };
        listed(45, NodeState::Unassigned);
        let nobody = bytes(pending_list(None, &Speaks::Nobody)).await;
        assert_eq!(nobody["speaksFor"], serde_json::json!({"kind": "nobody"}));
        assert_eq!(nobody["asks"], serde_json::json!([]));
        assert!(nobody["speaksForWords"]
            .as_str()
            .unwrap()
            .contains("does not speak for anyone"));
        let person = bytes(pending_list(None, &speaking())).await;
        assert_eq!(person["speaksFor"]["kind"], "person");
        let asks = person["asks"].as_array().unwrap();
        assert!(!asks.is_empty());
        for a in asks {
            assert_eq!(
                a["forIdentity"]["identityRoot"],
                crate::services::device_consent::tests::IDENTITY
            );
        }
    }

    #[tokio::test]
    async fn a_node_that_speaks_for_nobody_decides_nothing() {
        let device = listed(46, NodeState::Unassigned);
        let cell = FakeCell::new(Ok(ready()));
        let answer = decide_pending(
            &MemoryStore::new(),
            Some(&cell),
            &consent_grant::Unattended,
            None,
            &declaring(46),
            &Speaks::Nobody,
            true,
            &serde_json::to_vec(&serde_json::json!({ "ask": device })).unwrap(),
        )
        .await;
        assert_eq!(answer.status(), StatusCode::CONFLICT);
        let b = answer.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&b).unwrap();
        assert_eq!(v["code"], "node_speaks_for_nobody");
    }

    #[test]
    fn a_device_takes_a_decline_only_for_its_ask_from_a_node_that_listed_it() {
        let c = fresh();
        let state = "s".repeat(32);
        let (_, approver) = real(50);
        let decline = |source: &str, state: &str, proof: Option<CarrierProof>| {
            c.on_request(
                source,
                CarryRequest::Declined {
                    state: state.into(),
                    approver: Some(approver.clone()),
                    proof,
                },
            )
            .unwrap()
        };
        c.start_announce(announcing());
        let proven = || Some(proof_by(50, CarrierKind::Declined, &state, "peer-a", ""));
        assert_eq!(
            decline("peer-a", &state, proven()),
            CarryResponse::Refused {
                code: "carry_from_unlisting_peer".into()
            }
        );
        let listing = proof_by(50, CarrierKind::Listed, "", "peer-a", "");
        assert!(c.note_listed("peer-a", Some(approver.clone()), Some(&listing), 1));
        // Unsigned: refused, and the announce keeps waiting.
        assert_eq!(
            decline("peer-a", &state, None),
            CarryResponse::Refused {
                code: "carry_unproven".into()
            }
        );
        assert_eq!(
            decline("peer-a", &state, proven()),
            CarryResponse::DeclineTaken
        );
        assert_eq!(c.announce().unwrap().declined_by, Some(Some(approver)));
    }

    #[test]
    fn a_device_takes_a_code_only_for_its_ask_from_a_node_that_listed_it() {
        let c = fresh();
        let state = "s".repeat(32);
        let (_, approver) = real(50);
        let code = |source: &str, state: &str, key: &str, proof: Option<CarrierProof>| {
            c.on_request(
                source,
                CarryRequest::Code {
                    state: state.into(),
                    code: "c".repeat(32),
                    approver: Some(key.to_string()),
                    proof,
                },
            )
            .unwrap()
        };
        let proven = |state: &str| Some(proof_by(50, CarrierKind::Code, state, "peer-a", ""));
        assert_eq!(
            code("peer-a", &state, &approver, proven(&state)),
            CarryResponse::Refused {
                code: "carry_no_announce".into()
            }
        );
        c.start_announce(announcing());
        assert_eq!(
            code("peer-a", &state, &approver, proven(&state)),
            CarryResponse::Refused {
                code: "carry_from_unlisting_peer".into()
            }
        );
        let listing = proof_by(50, CarrierKind::Listed, "", "peer-a", "");
        assert!(c.note_listed("peer-a", Some(approver.clone()), Some(&listing), 1));
        assert_eq!(
            code(
                "peer-a",
                &"x".repeat(32),
                &approver,
                proven(&"x".repeat(32))
            ),
            CarryResponse::Refused {
                code: "carry_not_this_ask".into()
            }
        );
        assert_eq!(
            code("peer-a", &state, &approver, proven(&state)),
            CarryResponse::CodeTaken
        );
        let a = c.announce().unwrap();
        assert_eq!(a.code, Some(("c".repeat(32), "peer-a".into())));
        assert_eq!(a.code_approver, Some(approver));
    }

    #[test]
    fn a_rogue_node_on_the_network_cannot_capture_the_asking_device() {
        // Adversarial review, high 3. Before the fix: the rogue answered
        // Listed with any key, then sent a Code; on_code checked only that
        // the source had listed the ask and discarded the approver, and the
        // last code won.
        let c = fresh();
        let state = "s".repeat(32);
        let (_, honest) = real(50);
        let (_, rogue) = real(66);
        c.start_announce(announcing());
        // A listing without its key's proof, or with a proof made for another
        // transport id (replayed from the honest node), is not taken.
        assert!(!c.note_listed("rogue-peer", Some(honest.clone()), None, 9));
        let replayed = proof_by(50, CarrierKind::Listed, "", "honest-peer", "");
        assert!(!c.note_listed("rogue-peer", Some(honest.clone()), Some(&replayed), 9));
        // The rogue may list under its own proven key ...
        let own = proof_by(66, CarrierKind::Listed, "", "rogue-peer", "");
        assert!(c.note_listed("rogue-peer", Some(rogue.clone()), Some(&own), 9));
        // ... but cannot hand a code back in the honest node's name, or
        // without its own key's proof over this ask.
        let as_honest = Some(proof_by(50, CarrierKind::Code, &state, "rogue-peer", ""));
        let reply = |key: &str, proof: Option<CarrierProof>| {
            c.on_request(
                "rogue-peer",
                CarryRequest::Code {
                    state: state.clone(),
                    code: "r".repeat(32),
                    approver: Some(key.to_string()),
                    proof,
                },
            )
            .unwrap()
        };
        assert_eq!(
            reply(&honest, as_honest),
            CarryResponse::Refused {
                code: "carry_from_another_key".into()
            }
        );
        assert_eq!(
            reply(&rogue, None),
            CarryResponse::Refused {
                code: "carry_unproven".into()
            }
        );
        // A device that declared its approver takes nothing from another key,
        // even proven.
        let mut declared = announcing();
        declared.for_approver = Some(honest.clone());
        c.start_announce(declared);
        assert!(!c.note_listed("rogue-peer", Some(rogue.clone()), Some(&own), 9));
        assert_eq!(
            reply(
                &rogue,
                Some(proof_by(66, CarrierKind::Code, &state, "rogue-peer", ""))
            ),
            CarryResponse::Refused {
                code: "carry_from_unlisting_peer".into()
            }
        );
        assert!(c.announce().unwrap().code.is_none());
        // With no approver declared, a proven rogue's code is taken under
        // its own key, so the terminal names that key and asks before it
        // enrolls, and enrolling checks the consent against it.
        let mut open = announcing();
        open.state = NodeState::Unassigned;
        c.start_announce(open);
        assert!(c.note_listed("rogue-peer", Some(rogue.clone()), Some(&own), 9));
        assert_eq!(
            reply(
                &rogue,
                Some(proof_by(66, CarrierKind::Code, &state, "rogue-peer", ""))
            ),
            CarryResponse::CodeTaken
        );
        assert_eq!(c.announce().unwrap().code_approver, Some(rogue));
    }

    use crate::services::device_consent::tests::{ready, FakeCell};
    use crate::services::device_consent::CellStanding as ConsentStanding;
    use http_body_util::BodyExt;

    /// A carrier link that answers every message with `CodeTaken` and keeps
    /// what it was asked to carry.
    struct FakeLink(std::sync::Mutex<Vec<(String, CarryRequest)>>);

    #[async_trait]
    impl CarrierLink for FakeLink {
        fn peers(&self) -> Vec<String> {
            vec!["peer-approver".into()]
        }
        fn self_transport(&self) -> Option<String> {
            Some("peer-self".into())
        }
        async fn send(&self, peer: &str, request: CarryRequest) -> Result<CarryResponse, String> {
            self.0.lock().unwrap().push((peer.to_string(), request));
            Ok(CarryResponse::CodeTaken)
        }
    }

    fn key(n: u8) -> String {
        real(n).1
    }

    fn state_of(n: u8) -> String {
        format!("state{n:0>27}")
    }

    /// List an ask from a device with key `key(n)` on the global carrier.
    fn listed(n: u8, state: NodeState) -> String {
        let mut r = request();
        // Each ask its own state token: a decided one is remembered by it.
        r.state = state_of(n);
        let mut ask = signed_ask(n, r, &format!("peer-device-{n}"));
        ask.state = state;
        carrier()
            .pending
            .admit(
                ask,
                &format!("peer-device-{n}"),
                None,
                &speaking(),
                &crate::services::device_consent::policy(),
                now_micros(),
            )
            .unwrap();
        key(n)
    }

    fn declaring(n: u8) -> Declaration {
        Declaration {
            identity: None,
            devices: vec![consent_grant::DeclaredDevice {
                label: "home".into(),
                device_key: key(n),
                acts: vec![RequestedAct::EnrollDevice],
            }],
            asking: None,
        }
    }

    async fn decided(
        declaration: &Declaration,
        body: serde_json::Value,
        link: &FakeLink,
    ) -> (StatusCode, serde_json::Value) {
        let cell = FakeCell::new(Ok(ready()));
        let store = MemoryStore::new();
        let answer = decide_pending(
            &store,
            Some(&cell),
            &consent_grant::Unattended,
            Some(link),
            declaration,
            &speaking(),
            true,
            &serde_json::to_vec(&body).unwrap(),
        )
        .await;
        let status = answer.status();
        let bytes = answer.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn a_declared_unassigned_device_is_agreed_by_declaration_and_its_code_goes_back() {
        let device = listed(41, NodeState::Unassigned);
        let link = FakeLink(Default::default());
        let (status, answer) =
            decided(&declaring(41), serde_json::json!({ "ask": device }), &link).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        assert_eq!(answer["decidedBy"], "declaration");
        assert_eq!(answer["handedBack"]["taken"], true);
        let sent = link.0.lock().unwrap().clone();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "peer-device-41");
        assert!(matches!(&sent[0].1, CarryRequest::Code { state, .. } if *state == state_of(41)));
        assert!(carrier().pending.pick(&device, now_micros()).is_none());
    }

    #[tokio::test]
    async fn an_undeclared_device_waits_for_an_answer_and_the_answer_decides() {
        let device = listed(42, NodeState::Unassigned);
        let link = FakeLink(Default::default());
        let (status, answer) = decided(
            &Declaration::default(),
            serde_json::json!({ "ask": device }),
            &link,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(answer["code"], "pending_needs_answer");
        assert_eq!(answer["ask"]["deviceKey"], device);
        assert!(link.0.lock().unwrap().is_empty());
        let (status, answer) = decided(
            &Declaration::default(),
            serde_json::json!({ "ask": device, "answer": { "agreedActs": ["device.enroll"] } }),
            &link,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        assert_eq!(answer["decidedBy"], "answer");
    }

    #[tokio::test]
    async fn a_declared_node_with_an_identity_of_its_own_still_goes_to_an_answer() {
        let device = listed(
            43,
            NodeState::OwnIdentity {
                made: consent_grant::Made::Unknown,
            },
        );
        let link = FakeLink(Default::default());
        let (status, answer) =
            decided(&declaring(43), serde_json::json!({ "ask": device }), &link).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(answer["error"]
            .as_str()
            .unwrap()
            .contains("already has an identity of its own"));
        assert!(link.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_empty_answer_declines_signs_nothing_and_tells_the_device() {
        let device = listed(44, NodeState::Unassigned);
        let link = FakeLink(Default::default());
        let (status, answer) = decided(
            &declaring(44),
            serde_json::json!({ "ask": device, "answer": { "agreedActs": [] } }),
            &link,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(answer["declined"], true);
        // Nothing signed; the asking device is told it was declined.
        let sent = link.0.lock().unwrap().clone();
        assert_eq!(sent.len(), 1);
        assert!(
            matches!(&sent[0].1, CarryRequest::Declined { state, .. } if *state == state_of(44))
        );
        assert!(carrier().pending.pick(&device, now_micros()).is_none());
    }

    #[tokio::test]
    async fn nobody_signed_in_decides_nothing() {
        let cell = FakeCell::new(Ok(ready()));
        let answer = decide_pending(
            &MemoryStore::new(),
            Some(&cell),
            &consent_grant::Unattended,
            None,
            &Declaration::default(),
            &speaking(),
            false,
            b"{\"ask\":\"1\"}",
        )
        .await;
        assert_eq!(answer.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn with_no_local_discovery_the_carrier_is_absent_and_says_so() {
        let refused = start_announce(None, None, NodeState::Unassigned, b"{}")
            .await
            .unwrap_err();
        assert_eq!(refused.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn an_unknown_code_redeems_nothing_over_the_carrier() {
        let c = fresh();
        let refused = c.on_request(
            "peer-a",
            CarryRequest::Redeem {
                redemption: consent_grant::Redemption {
                    code: "c".repeat(32),
                    code_verifier: "v".repeat(43),
                    client_id: "epr-cli".into(),
                    device_key: request().device_key,
                },
            },
        );
        let refused = refused.unwrap();
        assert_eq!(
            refused,
            CarryResponse::Refused {
                code: "redemption_code_unknown".into()
            }
        );
    }

    // --- how whom this node speaks for is kept: on demand, or by timer ---

    use crate::services::device_consent::CellFailure;
    use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering::SeqCst};

    const SECOND: i64 = 1_000_000;
    const MISSING: &str = "Conductor error: Attempted to call a zome function that doesn't \
                           exist: Zome: mishpat Fn my_consent_standing";

    /// A clock a test moves by hand.
    #[derive(Clone)]
    struct Clock(Arc<AtomicI64>);

    impl Clock {
        fn new() -> Self {
            Self(Arc::new(AtomicI64::new(1_000_000 * SECOND)))
        }
        fn at(&self) -> i64 {
            self.0.load(SeqCst)
        }
        fn set(&self, micros: i64) {
            self.0.store(micros, SeqCst)
        }
        fn pass(&self, micros: i64) {
            self.0.fetch_add(micros, SeqCst);
        }
    }

    /// A carrier that reads its time from `clock`.
    fn carrier_at(clock: &Clock) -> Carrier {
        let clock = clock.clone();
        Carrier {
            clock: Box::new(move || clock.at()),
            ..Carrier::new()
        }
    }

    struct Counted {
        standing: Mutex<Result<ConsentStanding, CellFailure>>,
        reads: AtomicUsize,
        out: AtomicUsize,
        most_out: AtomicUsize,
        hold_reads: AtomicBool,
        read_gate: tokio::sync::Semaphore,
        hold_proofs: AtomicBool,
        proof_gate: tokio::sync::Semaphore,
        proofs: AtomicUsize,
    }

    /// A cell that counts its standing reads and how many were out at once,
    /// and holds each read, or each proof, until it is let go when told to.
    #[derive(Clone)]
    struct CountedCell(Arc<Counted>);

    impl CountedCell {
        fn new(standing: Result<ConsentStanding, CellFailure>) -> Self {
            Self(Arc::new(Counted {
                standing: Mutex::new(standing),
                reads: AtomicUsize::new(0),
                out: AtomicUsize::new(0),
                most_out: AtomicUsize::new(0),
                hold_reads: AtomicBool::new(false),
                read_gate: tokio::sync::Semaphore::new(0),
                hold_proofs: AtomicBool::new(false),
                proof_gate: tokio::sync::Semaphore::new(0),
                proofs: AtomicUsize::new(0),
            }))
        }
        fn reads(&self) -> usize {
            self.0.reads.load(SeqCst)
        }
        fn most_out(&self) -> usize {
            self.0.most_out.load(SeqCst)
        }
        fn proofs(&self) -> usize {
            self.0.proofs.load(SeqCst)
        }
        fn answers(&self, standing: Result<ConsentStanding, CellFailure>) {
            *self.0.standing.lock().unwrap() = standing;
        }
        fn hold_reads(&self, hold: bool) {
            self.0.hold_reads.store(hold, SeqCst);
        }
        fn let_reads_go(&self, n: usize) {
            self.0.read_gate.add_permits(n);
        }
        fn hold_proofs(&self) {
            self.0.hold_proofs.store(true, SeqCst);
        }
    }

    fn without_the_function() -> CellFailure {
        CellFailure::Unavailable(MISSING.into())
    }

    fn down() -> CellFailure {
        CellFailure::Unavailable("down".into())
    }

    #[async_trait]
    impl ControllerCell for CountedCell {
        fn agent(&self) -> String {
            crate::services::device_consent::tests::CONTROLLER.into()
        }
        async fn standing(&self) -> Result<ConsentStanding, CellFailure> {
            self.0.reads.fetch_add(1, SeqCst);
            let out = self.0.out.fetch_add(1, SeqCst) + 1;
            self.0.most_out.fetch_max(out, SeqCst);
            if self.0.hold_reads.load(SeqCst) {
                self.0.read_gate.acquire().await.unwrap().forget();
            }
            self.0.out.fetch_sub(1, SeqCst);
            self.0.standing.lock().unwrap().clone()
        }
        async fn sign(
            &self,
            _: &crate::services::device_consent::ApprovalRequest,
        ) -> Result<crate::services::device_consent::ApprovalProofs, CellFailure> {
            Err(down())
        }
        async fn bootstrap(&self, _: &str) -> Result<(), CellFailure> {
            Ok(())
        }
        async fn create_human(
            &self,
            _: &crate::services::device_consent::NewHuman,
        ) -> Result<(), CellFailure> {
            Ok(())
        }
        async fn my_human(&self) -> Result<Option<consent_grant::ExistingIdentity>, CellFailure> {
            Ok(None)
        }
        async fn sign_carrier(
            &self,
            kind: CarrierKind,
            state: &str,
            transport_id: &str,
            challenge: &str,
        ) -> Result<CarrierProof, CellFailure> {
            self.0.proofs.fetch_add(1, SeqCst);
            if self.0.hold_proofs.load(SeqCst) {
                self.0.proof_gate.acquire().await.unwrap().forget();
            }
            Ok(proof_by(61, kind, state, transport_id, challenge))
        }
    }

    /// The ask of device `n`, offered from `peer`.
    fn offer(n: u8, peer: &str) -> CarryRequest {
        CarryRequest::Ask {
            ask: signed_ask(n, request(), peer),
        }
    }

    /// Whether an ask has wanted a read since the last want was taken.
    async fn wanted(c: &Carrier) -> bool {
        tokio::time::timeout(std::time::Duration::from_millis(20), c.speaks_wanted())
            .await
            .is_ok()
    }

    /// Let every spawned task run as far as it can.
    async fn settle() {
        for _ in 0..64 {
            tokio::task::yield_now().await;
        }
    }

    /// One turn in a task of its own.
    fn turn(c: &Arc<Carrier>, cell: &CountedCell, want: Want) -> tokio::task::JoinHandle<bool> {
        let (c, cell) = (c.clone(), cell.clone());
        tokio::spawn(async move { c.read_turn(Some(&cell), &Names, want, 0).await })
    }

    #[test]
    fn the_refresh_word_is_demand_unless_it_says_timer() {
        assert_eq!(SpeaksRefresh::parse(None), (SpeaksRefresh::Demand, true));
        assert_eq!(
            SpeaksRefresh::parse(Some("")),
            (SpeaksRefresh::Demand, true)
        );
        assert_eq!(
            SpeaksRefresh::parse(Some("demand")),
            (SpeaksRefresh::Demand, true)
        );
        assert_eq!(
            SpeaksRefresh::parse(Some(" Timer ")),
            (SpeaksRefresh::Timer, true)
        );
        assert_eq!(
            SpeaksRefresh::parse(Some("poll")),
            (SpeaksRefresh::Demand, false)
        );
        // A carrier nobody told is on demand, and is told once.
        let c = fresh();
        assert_eq!(c.refresh(), SpeaksRefresh::Demand);
        assert!(c.set_refresh(SpeaksRefresh::Timer));
        assert!(!c.set_refresh(SpeaksRefresh::Demand));
        assert_eq!(c.refresh(), SpeaksRefresh::Timer);
    }

    /// The production demand reader and prover, run as `HttpServer::serve`
    /// runs them: an ask wakes the reader, the reader reads, the read wakes
    /// the prover, and the device's re-offer is listed.
    #[tokio::test]
    async fn on_demand_a_resting_node_reads_nothing_and_the_readers_own_loop_lists_the_re_offer() {
        let clock = Clock::new();
        let c = Arc::new(carrier_at(&clock));
        let cell = CountedCell::new(Ok(ready()));
        let reader = {
            let (c, cell) = (c.clone(), cell.clone());
            tokio::spawn(async move {
                demand_reader(&c, move || Some(cell.clone()), &Names, || 0).await
            })
        };
        let proofs = {
            let (c, cell) = (c.clone(), cell.clone());
            tokio::spawn(async move {
                let link = FakeLink(std::sync::Mutex::new(vec![]));
                prover(&c, move || Some(cell.clone()), &link).await
            })
        };
        // Nothing asks, for an hour: the cell is not read.
        settle().await;
        clock.pass(3600 * SECOND);
        settle().await;
        assert_eq!(cell.reads(), 0);
        assert_eq!(cell.proofs(), 0);
        // An ask cannot wait for the read: it wakes the reader and is not
        // answered.
        assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
        assert!(c.pending.is_empty());
        settle().await;
        assert_eq!(cell.reads(), 1);
        assert_eq!(cell.proofs(), 1);
        // The device offers again five seconds on and is listed, with this
        // node's proof.
        clock.pass(5 * SECOND);
        let Some(CarryResponse::Listed { number, proof, .. }) =
            c.on_request("peer-a", offer(31, "peer-a"))
        else {
            panic!("listed on the re-offer");
        };
        assert_eq!(number, 1);
        assert!(proof.is_some());
        // Its offers for the rest of the thirty seconds read nothing more.
        for _ in 0..4 {
            clock.pass(5 * SECOND);
            assert!(c.on_request("peer-a", offer(31, "peer-a")).is_some());
            settle().await;
        }
        assert_eq!(cell.reads(), 1);
        // Thirty seconds after the read, the next offer is listed on it and
        // has the reader read again. The proof is made once.
        clock.pass(5 * SECOND);
        assert!(c.on_request("peer-a", offer(31, "peer-a")).is_some());
        settle().await;
        assert_eq!(cell.reads(), 2);
        assert_eq!(cell.proofs(), 1);
        reader.abort();
        proofs.abort();
    }

    #[tokio::test]
    async fn the_reader_with_no_cell_spends_the_wake_and_the_next_offer_tries_again() {
        let clock = Clock::new();
        let c = Arc::new(carrier_at(&clock));
        let cell = CountedCell::new(Ok(ready()));
        let reachable = Arc::new(AtomicBool::new(false));
        let reader = {
            let (c, cell, reachable) = (c.clone(), cell.clone(), reachable.clone());
            tokio::spawn(async move {
                let cell = move || reachable.load(SeqCst).then(|| cell.clone());
                demand_reader(&c, cell, &Names, || 0).await
            })
        };
        // The conductor is not reachable yet: the wake is spent as a read
        // the cell did not answer.
        assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
        settle().await;
        assert_eq!(cell.reads(), 0);
        assert!(!c.read_due(Want::Ask, clock.at() + 5 * SECOND - 1));
        assert!(c.read_due(Want::Ask, clock.at() + 5 * SECOND));
        assert!(!c.read_due(Want::Request, clock.at() + 30 * SECOND - 1));
        // The cell comes back, and nothing reads until something asks.
        reachable.store(true, SeqCst);
        settle().await;
        assert_eq!(cell.reads(), 0);
        // The device's next offer wakes the reader again; the one after is
        // listed.
        clock.pass(5 * SECOND);
        assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
        settle().await;
        assert_eq!(cell.reads(), 1);
        clock.pass(5 * SECOND);
        assert!(c.on_request("peer-a", offer(31, "peer-a")).is_some());
        reader.abort();
    }

    #[tokio::test]
    async fn callers_that_queued_behind_a_slow_read_are_judged_when_their_turn_comes() {
        let clock = Clock::new();
        let c = Arc::new(carrier_at(&clock));
        let cell = CountedCell::new(Ok(ready()));
        cell.hold_reads(true);
        let began = clock.at();
        let first = turn(&c, &cell, Want::Request);
        settle().await;
        assert_eq!(cell.reads(), 1);
        // Callers arrive at thirty, sixty and ninety seconds while it is out.
        let mut queued = Vec::new();
        for secs in [30, 60, 90] {
            clock.set(began + secs * SECOND);
            queued.push(turn(&c, &cell, Want::Request));
            queued.push(turn(&c, &cell, Want::Ask));
            settle().await;
        }
        assert_eq!(cell.reads(), 1);
        // It lands at a hundred seconds.
        clock.set(began + 100 * SECOND);
        cell.hold_reads(false);
        cell.let_reads_go(1);
        assert!(first.await.unwrap());
        for waited in queued {
            assert!(!waited.await.unwrap());
        }
        assert_eq!(cell.reads(), 1);
        assert_eq!(cell.most_out(), 1);
        // Its evidence is as of when its turn came; the next read is paced
        // from when it landed.
        assert_eq!(c.speaks.lock().unwrap().as_ref().unwrap().1, began);
        assert_eq!(c.reads_lock().last, Some((began + 100 * SECOND, true)));
        clock.set(began + 130 * SECOND - 1);
        assert!(!turn(&c, &cell, Want::Request).await.unwrap());
        assert!(!turn(&c, &cell, Want::Ask).await.unwrap());
        clock.set(began + 130 * SECOND);
        assert!(turn(&c, &cell, Want::Request).await.unwrap());
        assert_eq!(cell.reads(), 2);
        // Identity writes always read, and still one at a time.
        cell.hold_reads(true);
        let writes = [
            turn(&c, &cell, Want::Now),
            turn(&c, &cell, Want::Now),
            turn(&c, &cell, Want::Now),
        ];
        settle().await;
        assert_eq!(cell.reads(), 3);
        cell.let_reads_go(3);
        for write in writes {
            assert!(write.await.unwrap());
        }
        assert_eq!(cell.reads(), 5);
        assert_eq!(cell.most_out(), 1);
    }

    #[tokio::test]
    async fn a_burst_of_anonymous_requests_is_one_attempt_in_thirty_seconds_answered_or_not() {
        let clock = Clock::new();
        let c = Arc::new(carrier_at(&clock));
        let cell = CountedCell::new(Err(down()));
        cell.hold_reads(true);
        let began = clock.at();
        // Forty callers at once, and forty more over the seconds the slow
        // read is out.
        let mut callers: Vec<_> = (0..40).map(|_| turn(&c, &cell, Want::Request)).collect();
        settle().await;
        for _ in 0..40 {
            clock.pass(SECOND / 2);
            callers.push(turn(&c, &cell, Want::Request));
            settle().await;
        }
        assert_eq!(cell.reads(), 1);
        cell.hold_reads(false);
        cell.let_reads_go(1);
        let mut read = 0;
        for caller in callers {
            read += usize::from(caller.await.unwrap());
        }
        assert_eq!(read, 1);
        assert_eq!(cell.reads(), 1);
        assert_eq!(cell.most_out(), 1);
        // The cell did not answer. No request has it tried again for thirty
        // seconds from when that attempt ended...
        let ended = began + 20 * SECOND;
        assert_eq!(c.reads_lock().last, Some((ended, false)));
        for secs in [1, 5, 6, 29] {
            clock.set(ended + secs * SECOND);
            for caller in (0..10).map(|_| turn(&c, &cell, Want::Request)) {
                assert!(!caller.await.unwrap());
            }
        }
        clock.set(ended + 30 * SECOND - 1);
        assert!(!turn(&c, &cell, Want::Request).await.unwrap());
        assert_eq!(cell.reads(), 1);
        clock.set(ended + 30 * SECOND);
        assert!(turn(&c, &cell, Want::Request).await.unwrap());
        assert_eq!(cell.reads(), 2);
        // ...and only a device's ask tries again after five.
        let ended = clock.at();
        clock.set(ended + 5 * SECOND - 1);
        assert!(!turn(&c, &cell, Want::Ask).await.unwrap());
        clock.set(ended + 5 * SECOND);
        assert!(!turn(&c, &cell, Want::Request).await.unwrap());
        assert!(turn(&c, &cell, Want::Ask).await.unwrap());
        assert_eq!(cell.reads(), 3);
        // An ask nobody signed wakes no reader.
        let fresh_carrier = carrier_at(&clock);
        let mut unsigned = signed_ask(31, request(), "peer-a");
        unsigned.proof = None;
        assert_eq!(
            fresh_carrier.on_request("peer-a", CarryRequest::Ask { ask: unsigned }),
            None
        );
        assert!(!wanted(&fresh_carrier).await);
        assert_eq!(
            fresh_carrier.on_request("peer-b", offer(31, "peer-a")),
            None
        );
        assert!(!wanted(&fresh_carrier).await);
        assert_eq!(
            fresh_carrier.on_request("peer-a", offer(31, "peer-a")),
            None
        );
        assert!(wanted(&fresh_carrier).await);
    }

    #[tokio::test]
    async fn a_read_that_took_longer_than_the_bound_lists_nothing() {
        for refresh in [SpeaksRefresh::Demand, SpeaksRefresh::Timer] {
            let clock = Clock::new();
            let c = Arc::new(carrier_at(&clock));
            c.set_refresh(refresh);
            let cell = CountedCell::new(Ok(ready()));
            cell.hold_reads(true);
            let slow = turn(&c, &cell, Want::Now);
            settle().await;
            clock.pass(SPEAKS_MAX_AGE_MICROS);
            cell.let_reads_go(1);
            assert!(slow.await.unwrap());
            // It answered that the node speaks for someone, 120 s ago.
            assert_eq!(c.speaks(), Speaks::Unknown, "{refresh:?}");
            assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
            assert!(c.pending.is_empty());
        }
    }

    #[tokio::test]
    async fn a_proof_that_cannot_be_made_holds_up_no_read() {
        let clock = Clock::new();
        let c = Arc::new(carrier_at(&clock));
        let cell = CountedCell::new(Ok(ready()));
        cell.hold_proofs();
        let proofs = {
            let (c, cell) = (c.clone(), cell.clone());
            tokio::spawn(async move {
                let link = FakeLink(std::sync::Mutex::new(vec![]));
                prover(&c, move || Some(cell.clone()), &link).await
            })
        };
        // The first read wakes the prover, whose signature never comes back.
        assert!(c.read_turn(Some(&cell), &Names, Want::Now, 0).await);
        settle().await;
        assert_eq!(cell.proofs(), 1);
        assert!(!c.has_listed_proof(&cell.agent(), "peer-self"));
        // Every kind of read still runs, and the ask is still listed.
        let second = std::time::Duration::from_secs(1);
        for want in [Want::Now, Want::Now] {
            let read = tokio::time::timeout(second, c.read_turn(Some(&cell), &Names, want, 0));
            assert!(read.await.expect("the read did not wait for the proof"));
        }
        clock.pass(30 * SECOND);
        for want in [Want::Request, Want::Ask] {
            let read = tokio::time::timeout(second, c.read_turn(Some(&cell), &Names, want, 0));
            read.await.expect("the read did not wait for the proof");
        }
        assert_eq!(cell.reads(), 4);
        assert!(matches!(
            c.on_request("peer-a", offer(31, "peer-a")),
            Some(CarryResponse::Listed { proof: None, .. })
        ));
        // A second proof is not started beside the first.
        let link = FakeLink(std::sync::Mutex::new(vec![]));
        tokio::time::timeout(second, c.ensure_listed_proof(&cell, &link))
            .await
            .expect("left to the one being made");
        settle().await;
        assert_eq!(cell.proofs(), 1);
        proofs.abort();
    }

    #[tokio::test]
    async fn nothing_is_listed_on_a_read_as_old_as_the_bound_however_it_is_kept() {
        for refresh in [SpeaksRefresh::Demand, SpeaksRefresh::Timer] {
            let clock = Clock::new();
            let c = carrier_at(&clock);
            c.set_refresh(refresh);
            let cell = CountedCell::new(Ok(ready()));
            let read_at = clock.at();
            c.read_turn(Some(&cell), &Names, Want::Now, 0).await;
            assert!(matches!(
                c.on_request("peer-a", offer(31, "peer-a")),
                Some(CarryResponse::Listed { number: 1, .. })
            ));
            // One microsecond inside the bound the same ceremony keeps its
            // place.
            clock.set(read_at + SPEAKS_MAX_AGE_MICROS - 1);
            assert!(matches!(
                c.on_request("peer-a", offer(31, "peer-a")),
                Some(CarryResponse::Listed { number: 1, .. })
            ));
            // At the bound nothing is listed: not that ceremony, not a new
            // device.
            clock.set(read_at + SPEAKS_MAX_AGE_MICROS);
            assert_eq!(c.speaks(), Speaks::Unknown, "{refresh:?}");
            assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
            assert_eq!(c.on_request("peer-b", offer(32, "peer-b")), None);
            assert_eq!(c.pending.len(), 1, "{refresh:?}");
        }
    }

    #[tokio::test]
    async fn on_demand_a_new_ask_is_listed_only_on_a_read_younger_than_the_refresh() {
        let clock = Clock::new();
        let c = carrier_at(&clock);
        let cell = CountedCell::new(Ok(ready()));
        let read_at = clock.at();
        c.read_turn(Some(&cell), &Names, Want::Now, 0).await;
        assert!(c.on_request("peer-a", offer(31, "peer-a")).is_some());
        // One microsecond inside thirty seconds: listed, and no read wanted.
        clock.set(read_at + 30 * SECOND - 1);
        assert!(matches!(
            c.on_request("peer-b", offer(32, "peer-b")),
            Some(CarryResponse::Listed { number: 2, .. })
        ));
        assert!(!wanted(&c).await);
        // At thirty seconds a new device is not listed; it wants the read.
        clock.set(read_at + 30 * SECOND);
        assert_eq!(c.on_request("peer-c", offer(33, "peer-c")), None);
        assert!(wanted(&c).await);
        assert_eq!(c.pending.len(), 2);
        // By timer the same new device is listed on that read.
        let t = carrier_at(&clock);
        t.set_refresh(SpeaksRefresh::Timer);
        clock.set(read_at);
        t.read_turn(Some(&cell), &Names, Want::Now, 0).await;
        clock.set(read_at + 30 * SECOND);
        assert!(t.on_request("peer-c", offer(33, "peer-c")).is_some());
    }

    #[tokio::test]
    async fn on_demand_only_the_same_ceremony_keeps_its_place_on_an_older_read() {
        let clock = Clock::new();
        let c = carrier_at(&clock);
        let cell = CountedCell::new(Ok(ready()));
        c.read_turn(Some(&cell), &Names, Want::Now, 0).await;
        assert!(c.on_request("peer-a", offer(31, "peer-a")).is_some());
        clock.pass(60 * SECOND);
        // The same ceremony, offered again from the same peer: its place.
        assert!(matches!(
            c.on_request("peer-a", offer(31, "peer-a")),
            Some(CarryResponse::Listed { number: 1, .. })
        ));
        // The listed device begins another ceremony (a new state, or a new
        // challenge), signed by it from the same peer: a new ask, not listed
        // on the old read, and the first is not replaced.
        let mut restated = request();
        restated.state = "t".repeat(32);
        let mut rechallenged = request();
        rechallenged.code_challenge = pkce::challenge(&"w".repeat(43));
        for other in [restated, rechallenged] {
            let ask = signed_ask(31, other, "peer-a");
            assert_eq!(c.on_request("peer-a", CarryRequest::Ask { ask }), None);
        }
        // The same ceremony from another peer is not the listed one either.
        assert_eq!(c.on_request("peer-b", offer(31, "peer-b")), None);
        let listed = c.pending.list(now_micros());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].ask.request, request_of(31));
        // On a fresh read the new ceremony replaces the old, as before.
        clock.pass(SECOND);
        c.read_turn(Some(&cell), &Names, Want::Now, 0).await;
        let mut restated = request();
        restated.state = "t".repeat(32);
        let ask = signed_ask(31, restated, "peer-a");
        assert!(matches!(
            c.on_request("peer-a", CarryRequest::Ask { ask }),
            Some(CarryResponse::Listed { number: 1, .. })
        ));
    }

    /// `request()` as device `n` asks it.
    fn request_of(n: u8) -> GrantRequest {
        signed_ask(n, request(), "peer-a").request
    }

    #[tokio::test]
    async fn on_demand_a_node_whose_record_stopped_standing_stops_listing_at_the_next_ask() {
        let clock = Clock::new();
        let c = carrier_at(&clock);
        let cell = CountedCell::new(Ok(ready()));
        c.read_turn(Some(&cell), &Names, Want::Now, 0).await;
        assert!(c.on_request("peer-a", offer(31, "peer-a")).is_some());
        // The record stops standing while the node rests.
        cell.answers(Ok(ConsentStanding::NoPerson));
        clock.pass(60 * SECOND);
        assert_eq!(cell.reads(), 1);
        // A new device is not listed on what was read before.
        assert_eq!(c.on_request("peer-b", offer(32, "peer-b")), None);
        // The ceremony already listed keeps its place for this offer, on a
        // read sixty seconds old where a timer would have read again; the
        // offer wants the read.
        assert!(matches!(
            c.on_request("peer-a", offer(31, "peer-a")),
            Some(CarryResponse::Listed { number: 1, .. })
        ));
        assert!(wanted(&c).await);
        assert!(c.read_turn(Some(&cell), &Names, Want::Ask, 0).await);
        assert_eq!(c.speaks(), Speaks::Nobody);
        // The next ask, from anyone, is dropped unanswered.
        clock.pass(5 * SECOND);
        assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
        assert_eq!(c.on_request("peer-b", offer(32, "peer-b")), None);
    }

    #[tokio::test]
    async fn by_timer_every_turn_reads_and_an_ask_wants_nothing() {
        let clock = Clock::new();
        let c = carrier_at(&clock);
        c.set_refresh(SpeaksRefresh::Timer);
        let cell = CountedCell::new(Ok(ready()));
        // An ask before the first turn is dropped unanswered and wakes nobody.
        assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
        assert!(!wanted(&c).await);
        assert_eq!(cell.reads(), 0);
        // Every turn reads, asked or not, however lately it read.
        assert!(c.timer_turn(&cell, &Names, 0).await);
        assert!(c.timer_turn(&cell, &Names, 0).await);
        assert_eq!(cell.reads(), 2);
        // A new ask is listed on the last turn's read at any age inside the
        // bound, and wants nothing.
        clock.pass(100 * SECOND);
        assert!(matches!(
            c.on_request("peer-a", offer(31, "peer-a")),
            Some(CarryResponse::Listed { number: 1, .. })
        ));
        assert!(!wanted(&c).await);
        // A cell that stops answering leaves that read to age; the turns go on.
        cell.answers(Err(down()));
        assert!(c.timer_turn(&cell, &Names, 0).await);
        assert!(c.timer_turn(&cell, &Names, 0).await);
        assert_eq!(cell.reads(), 4);
        assert!(c.speaks().person().is_some());
        // The record stops standing: the next turn's read stops the listing.
        cell.answers(Ok(ConsentStanding::NoPerson));
        assert!(c.timer_turn(&cell, &Names, 0).await);
        assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
    }

    /// The production timer loop, on tokio's own (paused) time.
    #[tokio::test(start_paused = true)]
    async fn the_timers_own_loop_reads_every_thirty_seconds_and_when_identity_changes() {
        let c = Arc::new(fresh());
        c.set_refresh(SpeaksRefresh::Timer);
        let cell = CountedCell::new(Ok(ready()));
        let timer = {
            let (c, cell) = (c.clone(), cell.clone());
            tokio::spawn(
                async move { timer_reader(&c, move || Some(cell.clone()), &Names, || 0).await },
            )
        };
        settle().await;
        assert_eq!(cell.reads(), 1);
        let secs = std::time::Duration::from_secs;
        tokio::time::advance(secs(SPEAKS_REFRESH_SECS) - std::time::Duration::from_millis(1)).await;
        settle().await;
        assert_eq!(cell.reads(), 1);
        tokio::time::advance(std::time::Duration::from_millis(1)).await;
        settle().await;
        assert_eq!(cell.reads(), 2);
        for _ in 0..8 {
            tokio::time::advance(secs(SPEAKS_REFRESH_SECS)).await;
            settle().await;
        }
        assert_eq!(cell.reads(), 10);
        c.identity_changed();
        settle().await;
        assert_eq!(cell.reads(), 11);
        timer.abort();
    }

    #[tokio::test]
    async fn the_timer_rests_on_a_coordinator_without_the_function_until_it_is_updated() {
        let clock = Clock::new();
        let c = carrier_at(&clock);
        c.set_refresh(SpeaksRefresh::Timer);
        let cell = CountedCell::new(Err(without_the_function()));
        let at = clock.at();
        assert!(c.timer_turn(&cell, &Names, 7).await);
        // The answer cannot change: no turn asks again while it rests.
        for turn in 1..=59 {
            clock.set(at + turn * 30 * SECOND);
            assert!(c.resting(7));
            assert!(!c.timer_turn(&cell, &Names, 7).await);
        }
        assert_eq!(cell.reads(), 1);
        // With no update seen here, it asks again at the cap, and rests again.
        let cap = at + SPEAKS_REST_MAX_SECS as i64 * SECOND;
        clock.set(cap - 1);
        assert!(!c.timer_turn(&cell, &Names, 7).await);
        clock.set(cap);
        assert!(c.timer_turn(&cell, &Names, 7).await);
        clock.set(cap + 30 * SECOND);
        assert!(!c.timer_turn(&cell, &Names, 7).await);
        assert_eq!(cell.reads(), 2);
        // A coordinator update applied here ends the rest at the next turn.
        clock.set(cap + 60 * SECOND);
        assert!(c.timer_turn(&cell, &Names, 8).await);
        clock.set(cap + 90 * SECOND);
        assert!(!c.timer_turn(&cell, &Names, 8).await);
        assert_eq!(cell.reads(), 3);
        // An identity write reads through the rest.
        assert!(c.read_turn(Some(&cell), &Names, Want::Now, 8).await);
        assert_eq!(cell.reads(), 4);
        // The updated coordinator answers: the rest is over.
        cell.answers(Ok(ready()));
        clock.set(cap + 120 * SECOND);
        assert!(c.timer_turn(&cell, &Names, 9).await);
        assert!(!c.resting(9));
        assert!(c.timer_turn(&cell, &Names, 9).await);
        assert!(c.speaks().person().is_some());
        // A cell that is only unreachable is no reason to rest, and neither
        // is another function's absence.
        for why in [
            down(),
            CellFailure::Unavailable(MISSING.replace("my_consent_standing", "get_my_human")),
        ] {
            cell.answers(Err(why));
            assert!(c.timer_turn(&cell, &Names, 9).await);
            assert!(!c.resting(9));
        }
    }

    #[tokio::test]
    async fn on_demand_the_functions_absence_is_an_answer_not_asked_again_for_thirty_seconds() {
        let clock = Clock::new();
        let c = carrier_at(&clock);
        let cell = CountedCell::new(Err(without_the_function()));
        let at = clock.at();
        assert!(c.read_turn(Some(&cell), &Names, Want::Ask, 0).await);
        assert_eq!(c.speaks(), Speaks::Unknown);
        // Not the five seconds of a cell that did not answer.
        clock.set(at + 30 * SECOND - 1);
        assert!(!c.read_turn(Some(&cell), &Names, Want::Ask, 0).await);
        assert_eq!(cell.reads(), 1);
        // And asked again then, on demand, without the timer's rest.
        clock.set(at + 30 * SECOND);
        assert!(c.read_turn(Some(&cell), &Names, Want::Ask, 0).await);
        clock.set(at + 60 * SECOND);
        assert!(c.read_turn(Some(&cell), &Names, Want::Ask, 0).await);
        assert_eq!(cell.reads(), 3);
    }

    // --- round two: writes, the whole ask, a slow cell, and what time is ---

    /// A carrier on `clock`, with a wall clock a test steps by hand.
    fn carrier_with_wall(clock: &Clock, wall: &Clock) -> Carrier {
        let (clock, wall) = (clock.clone(), wall.clone());
        Carrier {
            clock: Box::new(move || clock.at()),
            wall: Box::new(move || wall.at()),
            ..Carrier::new()
        }
    }

    #[tokio::test]
    async fn a_refused_write_reads_nothing_however_often_it_is_sent() {
        for refused in [400, 401, 403, 404, 409, 500, 503] {
            assert_eq!(Want::after_write(refused), None);
        }
        assert_eq!(Want::after_write(201), Some(Want::Now));
        assert_eq!(Want::after_write(200), Some(Want::Request));
        let clock = Clock::new();
        let c = carrier_at(&clock);
        let cell = CountedCell::new(Ok(ready()));
        // What the routes do with a write's answer.
        let after = |status: u16| {
            let (c, cell) = (&c, &cell);
            async move {
                if let Some(want) = Want::after_write(status) {
                    c.read_turn(Some(cell), &Names, want, 0).await;
                }
            }
        };
        // A hundred refused writes: no read at all.
        for status in [401, 403, 400, 409].into_iter().cycle().take(100) {
            after(status).await;
            clock.pass(SECOND / 10);
        }
        assert_eq!(cell.reads(), 0);
        // A hundred writes that succeeded without writing: what a hundred
        // requests get, one attempt in thirty seconds.
        for _ in 0..100 {
            after(200).await;
            clock.pass(SECOND / 10);
        }
        assert_eq!(cell.reads(), 1);
        clock.pass(20 * SECOND);
        after(200).await;
        assert_eq!(cell.reads(), 2);
        // A write that created something reads at once, each time.
        after(201).await;
        after(201).await;
        assert_eq!(cell.reads(), 4);
    }

    #[tokio::test]
    async fn on_demand_an_ask_that_differs_only_in_what_nobody_signed_is_a_new_ask() {
        let clock = Clock::new();
        let c = carrier_at(&clock);
        let cell = CountedCell::new(Ok(ready()));
        c.read_turn(Some(&cell), &Names, Want::Now, 0).await;
        let listed = signed_ask(31, request(), "peer-a");
        assert!(c
            .on_request(
                "peer-a",
                CarryRequest::Ask {
                    ask: listed.clone()
                }
            )
            .is_some());
        clock.pass(60 * SECOND);
        // The same request and the same proof, replayed from the same peer
        // with only what the device says of itself changed, or only its
        // declared approver: not the listed ask, so not listed on a read a
        // minute old, and the listed one is not replaced.
        let mut restated = listed.clone();
        restated.state = NodeState::OwnIdentity {
            made: consent_grant::Made::Unknown,
        };
        let mut redirected = listed.clone();
        redirected.for_approver = Some(cell.agent());
        for replay in [restated.clone(), redirected] {
            assert!(replay.signed_from("peer-a"));
            assert_eq!(
                c.on_request("peer-a", CarryRequest::Ask { ask: replay }),
                None
            );
        }
        assert_eq!(c.pending.list(now_micros())[0].ask, listed);
        // The exact ask keeps its place.
        assert!(matches!(
            c.on_request("peer-a", CarryRequest::Ask { ask: listed }),
            Some(CarryResponse::Listed { number: 1, .. })
        ));
        // The replay waits for the next read like any new ask, and is
        // listed on it.
        assert!(c.read_turn(Some(&cell), &Names, Want::Ask, 0).await);
        assert!(matches!(
            c.on_request("peer-a", CarryRequest::Ask { ask: restated }),
            Some(CarryResponse::Listed { number: 1, .. })
        ));
    }

    #[tokio::test]
    async fn on_demand_an_ask_that_waited_through_a_slow_read_is_listed_on_it_inside_the_bound() {
        for (took, listed) in [
            (20, true),
            (31, true),
            (60, true),
            (119, true),
            (121, false),
        ] {
            let clock = Clock::new();
            let c = Arc::new(carrier_at(&clock));
            let cell = CountedCell::new(Ok(ready()));
            cell.hold_reads(true);
            // The ask arrives, wakes the reader, and is not answered.
            assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
            let read = turn(&c, &cell, Want::Ask);
            settle().await;
            assert_eq!(cell.reads(), 1);
            // Ten seconds into the read another device asks.
            clock.pass(10 * SECOND);
            assert_eq!(c.on_request("peer-b", offer(32, "peer-b")), None);
            // The read lands `took` seconds after it began.
            clock.pass((took - 10) * SECOND);
            cell.let_reads_go(1);
            assert!(read.await.unwrap());
            // The device that was waiting before the read began is listed
            // on it when it offers again, while it is inside the bound.
            let waited = c.on_request("peer-a", offer(31, "peer-a"));
            assert_eq!(waited.is_some(), listed, "a read of {took} s");
            // The device that arrived after the read began is listed on it
            // only while it is younger than the refresh.
            let late = c.on_request("peer-b", offer(32, "peer-b"));
            assert_eq!(late.is_some(), took < 30, "a read of {took} s");
            // Once listed, the wait is forgotten.
            if listed {
                assert!(c
                    .waiting
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|w| w.source != "peer-a"));
            }
        }
    }

    #[test]
    fn the_memory_of_waiting_asks_is_bounded_and_an_ask_past_it_is_not_remembered() {
        let clock = Clock::new();
        let c = carrier_at(&clock);
        let ask = |n: u8, peer: &str| signed_ask(n, request(), peer);
        let at = clock.at();
        // One peer: MAX_WAITING_PER_SOURCE asks, and no more.
        for n in 0..MAX_WAITING_PER_SOURCE as u8 {
            assert_eq!(
                c.first_arrival("peer-a", &ask(n + 1, "peer-a"), at),
                Some(at)
            );
        }
        assert_eq!(c.first_arrival("peer-a", &ask(99, "peer-a"), at), None);
        // Remembered once: a later offer does not move its arrival.
        clock.pass(5 * SECOND);
        assert_eq!(
            c.first_arrival("peer-a", &ask(1, "peer-a"), clock.at()),
            Some(at)
        );
        // All peers together: MAX_WAITING asks, and no more.
        let mut peer = 0;
        while c.waiting.lock().unwrap().len() < MAX_WAITING {
            peer += 1;
            let name = format!("peer-{peer}");
            assert!(c.first_arrival(&name, &ask(7, &name), clock.at()).is_some());
        }
        assert_eq!(
            c.first_arrival("peer-z", &ask(7, "peer-z"), clock.at()),
            None
        );
        assert_eq!(c.waiting.lock().unwrap().len(), MAX_WAITING);
        // An ask past the bound is listed by the thirty-second rule only.
        let cell_time = clock.at();
        c.set_speaks(speaking(), cell_time);
        clock.pass(31 * SECOND);
        assert_eq!(c.on_request("peer-z", offer(7, "peer-z")), None);
        // Each is forgotten after an ask's life, and there is room again.
        clock.set(at + 5 * SECOND + ASK_LIFE_MICROS);
        assert!(c
            .first_arrival("peer-z", &ask(7, "peer-z"), clock.at())
            .is_some());
        assert_eq!(c.waiting.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn no_age_or_pace_moves_when_the_wall_clock_is_stepped() {
        let (clock, wall) = (Clock::new(), Clock::new());
        let c = carrier_with_wall(&clock, &wall);
        let cell = CountedCell::new(Ok(ready()));
        let read_at = clock.at();
        c.read_turn(Some(&cell), &Names, Want::Now, 0).await;
        assert!(c.on_request("peer-a", offer(31, "peer-a")).is_some());
        // The wall clock is set a hundred seconds back. A hundred and
        // eighty real seconds on, the wall says eighty; the read is not
        // listed on.
        wall.pass(-100 * SECOND);
        clock.set(read_at + SPEAKS_MAX_AGE_MICROS - 1);
        wall.pass(SPEAKS_MAX_AGE_MICROS - 1);
        assert!(c.speaks().person().is_some());
        clock.set(read_at + SPEAKS_MAX_AGE_MICROS);
        wall.pass(1);
        assert_eq!(c.speaks(), Speaks::Unknown);
        assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
        clock.set(read_at + 180 * SECOND);
        wall.pass(60 * SECOND);
        assert_eq!(c.speaks(), Speaks::Unknown);
        assert_eq!(c.on_request("peer-a", offer(31, "peer-a")), None);
        // Nor does the step hold a retry back: five real seconds after a
        // read the cell did not answer, a device's ask reads again.
        cell.answers(Err(down()));
        assert!(c.read_turn(Some(&cell), &Names, Want::Ask, 0).await);
        let failed_at = clock.at();
        wall.pass(-100 * SECOND);
        clock.set(failed_at + 5 * SECOND - 1);
        assert!(!c.read_turn(Some(&cell), &Names, Want::Ask, 0).await);
        clock.set(failed_at + 5 * SECOND);
        assert!(c.read_turn(Some(&cell), &Names, Want::Ask, 0).await);
        // The wall clock is set an hour ahead: a read ten real seconds old
        // is still listed on, no request reads early, and the timer's rest
        // does not end.
        cell.answers(Ok(ready()));
        assert!(c.read_turn(Some(&cell), &Names, Want::Now, 0).await);
        wall.pass(3600 * SECOND);
        clock.pass(10 * SECOND);
        assert!(c.speaks().person().is_some());
        assert!(c.on_request("peer-b", offer(32, "peer-b")).is_some());
        assert!(!c.read_turn(Some(&cell), &Names, Want::Request, 0).await);
        cell.answers(Err(without_the_function()));
        assert!(c.read_turn(Some(&cell), &Names, Want::Now, 3).await);
        wall.pass(24 * 3600 * SECOND);
        clock.pass(60 * SECOND);
        assert!(c.resting(3));
    }

    #[tokio::test]
    async fn the_carriers_own_clock_is_the_steady_one_and_keeps_real_time() {
        let c = fresh();
        let before = steady_micros();
        let carriers = c.now();
        assert!((carriers - before).abs() < SECOND);
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        let passed = c.now() - carriers;
        assert!((40_000..5 * SECOND).contains(&passed), "{passed}");
        // Not the wall clock: time since boot is not time since 1970.
        assert!((now_micros() - c.now()).abs() > 365 * 24 * 3600 * SECOND);
    }

    #[tokio::test]
    async fn a_timer_turn_that_waited_behind_an_identity_write_does_not_read_again() {
        let clock = Clock::new();
        let c = Arc::new(carrier_at(&clock));
        c.set_refresh(SpeaksRefresh::Timer);
        let cell = CountedCell::new(Ok(ready()));
        cell.hold_reads(true);
        let write = turn(&c, &cell, Want::Now);
        settle().await;
        let timer = {
            let (c, cell) = (c.clone(), cell.clone());
            tokio::spawn(async move { c.timer_turn(&cell, &Names, 0).await })
        };
        settle().await;
        cell.hold_reads(false);
        cell.let_reads_go(1);
        assert!(write.await.unwrap());
        assert!(!timer.await.unwrap());
        assert_eq!(cell.reads(), 1);
        // The next turn, with nothing in its way, reads.
        assert!(c.timer_turn(&cell, &Names, 0).await);
        assert_eq!(cell.reads(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn a_worker_that_panics_is_started_again() {
        let starts = Arc::new(AtomicUsize::new(0));
        let worker = {
            let starts = starts.clone();
            tokio::spawn(supervised("test worker", move || {
                let starts = starts.clone();
                async move {
                    if starts.fetch_add(1, SeqCst) < 2 {
                        panic!("a worker's panic, on purpose");
                    }
                    std::future::pending::<()>().await
                }
            }))
        };
        settle().await;
        assert_eq!(starts.load(SeqCst), 1);
        let restart = std::time::Duration::from_secs(WORKER_RESTART_SECS);
        tokio::time::advance(restart - std::time::Duration::from_millis(1)).await;
        settle().await;
        assert_eq!(starts.load(SeqCst), 1);
        tokio::time::advance(std::time::Duration::from_millis(1)).await;
        settle().await;
        assert_eq!(starts.load(SeqCst), 2);
        tokio::time::advance(restart).await;
        settle().await;
        assert_eq!(starts.load(SeqCst), 3);
        // The third start does not panic, and is left running.
        tokio::time::advance(10 * restart).await;
        settle().await;
        assert_eq!(starts.load(SeqCst), 3);
        worker.abort();
    }
}
