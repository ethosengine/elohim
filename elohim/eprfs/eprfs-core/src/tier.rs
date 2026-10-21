//! Tiers, absences, the order of asking, and the tier chain (interfaces spec §A.2, §A.4).
//!
//! A requester reads through one [`TierChain`]: its own store first, then the outer tiers its
//! records and doorways named, in a pure order, each asked only when the injected [`AskPolicy`]
//! admits it for the root and only within the injected [`ForwardGate`]'s budget. Every byte is
//! verified against its address at every hop, and every absence is named with ruling 7's three
//! parts. The chain makes no reach, ranking or budget decision of its own: those are the injected
//! policies' (spec §12.3).
//!
//! Nothing here does I/O, reads a clock or spawns: the chain only awaits the `EprfsStorage` ports
//! it was given (rail R-E2).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::Bytes;
use seam_contracts::{ComeBack, Meanwhile, Unanswered};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::address::{BlobCid, EprRef};
use crate::attestation::AttestationDraft;
use crate::error::{EprfsError, Result};
use crate::reach::{AskPolicy, ReachLevel, ReachRefusal, Requester};
use crate::storage::{
    group_by_cid, Awaiting, BatchAnswer, BatchItem, BatchOptions, BatchReport, BlobHandle,
    BlobPresence, BlobSink, CollectSink, EprRecord, EprfsStorage, FetchPolicy, SinkControl,
};

/// The event every come-back names when only a tier coming back can change the answer.
pub const EVENT_TIER_REACHABLE: &str = "tier-reachable";
/// The event a refusal names: only a change of the requester's standing changes it.
pub const EVENT_STANDING_CHANGED: &str = "standing-changed";

/// Who answered. `own` is indexical: it means "this store" to whoever holds it, so it is never
/// serialized (its `Serialize` refuses) and every serving adapter stamps its own id instead; a
/// requester-side chain rewrites any `own` in a remote answer to the dialled record's id (A2).
///
/// An agent key (`uhCAk…`) when a cross-signed binding resolves one; otherwise the transport id
/// the tier was dialled by. Never read as attribution: it decides where to dial and what to count,
/// never whom to credit (A11).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TierId(TierIdForm);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum TierIdForm {
    Own,
    Agent(String),
    Transport(String),
}

const AGENT_PREFIX: &str = "agent:";
const TRANSPORT_PREFIX: &str = "transport:";

impl TierId {
    pub fn own() -> Self {
        Self(TierIdForm::Own)
    }

    pub fn agent(key: impl Into<String>) -> Self {
        Self(TierIdForm::Agent(key.into()))
    }

    pub fn transport(peer_id: impl Into<String>) -> Self {
        Self(TierIdForm::Transport(peer_id.into()))
    }

    pub fn is_own(&self) -> bool {
        matches!(self.0, TierIdForm::Own)
    }

    /// The key or transport id without its form prefix; `"own"` for the indexical id.
    pub fn as_str(&self) -> &str {
        match &self.0 {
            TierIdForm::Own => "own",
            TierIdForm::Agent(key) => key,
            TierIdForm::Transport(peer) => peer,
        }
    }
}

impl std::fmt::Display for TierId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            TierIdForm::Own => f.write_str("own"),
            TierIdForm::Agent(key) => write!(f, "{AGENT_PREFIX}{key}"),
            TierIdForm::Transport(peer) => write!(f, "{TRANSPORT_PREFIX}{peer}"),
        }
    }
}

/// `own` returns a serde error the caller handles, never a panic: serving adapters stamp their id
/// before they serialize, and the endpoint contract asserts no answer carries `own`.
impl Serialize for TierId {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        if self.is_own() {
            return Err(serde::ser::Error::custom(
                "tier id `own` is indexical and never serialized",
            ));
        }
        serializer.collect_str(self)
    }
}

/// Refuses `"own"` and any string without one of the two prefixes, or with an empty id after it.
impl<'de> Deserialize<'de> for TierId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct TierIdVisitor;
        impl Visitor<'_> for TierIdVisitor {
            type Value = TierId;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("\"agent:<key>\" or \"transport:<peer id>\"")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<TierId, E> {
                let parsed = if let Some(key) = value.strip_prefix(AGENT_PREFIX) {
                    (!key.is_empty()).then(|| TierId::agent(key))
                } else if let Some(peer) = value.strip_prefix(TRANSPORT_PREFIX) {
                    (!peer.is_empty()).then(|| TierId::transport(peer))
                } else {
                    None
                };
                parsed.ok_or_else(|| E::invalid_value(de::Unexpected::Str(value), &self))
            }
        }
        deserializer.deserialize_str(TierIdVisitor)
    }
}

/// The kind of carrier a tier is. Ordered narrowest first: a class conflict between two records of
/// one tier keeps the lower class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TierClass {
    Own,
    PersonDevice,
    Household,
    Hub,
    HolonHub,
    Pool,
    DoorwayPeer,
    Doorway,
}

/// Where the requester learned of a tier. Both sources are legitimate (R6a): held records (roster,
/// commitments, the person's devices, spaces) and the doorways the requester ingresses through,
/// which are plural participants. The record keeps every source that named it, so the probe can
/// count distinct doorways.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "source"
)]
pub enum TierSource {
    HeldRecord { record: String },
    Doorway { doorway: TierId },
}

/// A dial hint, never identity: a base URL or a multiaddr.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "form"
)]
pub enum TierAddress {
    Url { base: String },
    Multiaddr { addr: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierRecord {
    pub id: TierId,
    pub class: TierClass,
    /// Non-empty by construction (A14).
    pub sources: Vec<TierSource>,
    /// Where to dial (A11).
    pub endpoint: Option<TierAddress>,
    /// The widest reach this tier is admitted to read, as `Reach::openness()`, from a held record
    /// or the doorway entry; `None` is unknown (A5, H7).
    pub standing: Option<ReachLevel>,
    pub proximity: Option<Proximity>,
}

impl TierRecord {
    /// Merges a duplicate by id: sources are unioned; a class conflict keeps the narrower class
    /// (the lower `TierClass`); a standing conflict keeps the narrower standing; a missing
    /// endpoint or proximity is filled from `other`. The chain counts class conflicts.
    pub fn merge(&mut self, other: TierRecord) {
        debug_assert_eq!(self.id, other.id, "merge joins records of one tier");
        for source in other.sources {
            if !self.sources.contains(&source) {
                self.sources.push(source);
            }
        }
        self.class = self.class.min(other.class);
        self.standing = match (self.standing, other.standing) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        if self.endpoint.is_none() {
            self.endpoint = other.endpoint;
        }
        if self.proximity.is_none() {
            self.proximity = other.proximity;
        }
    }
}

/// No `Ord` derive: [`ProximityOrder`]'s comparator is explicit (A15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proximity {
    /// lan 0 · wan 1 · relay 2.
    pub network_rank: u8,
    /// Roots this tier declared warm (per content, not per tier).
    pub warm_roots: Vec<BlobCid>,
    pub serves_extracted: bool,
    pub rtt_ms: Option<u32>,
    pub last_seen_ms: Option<u64>,
}

/// Pure and stable. The own store is always first and is not an input: the chain holds it apart.
pub trait TierOrder: Send + Sync {
    fn order(&self, root: &BlobCid, records: &mut Vec<TierRecord>);
}

/// The default order. Compares, in order: class; warm for `root` first; network rank; measured
/// rtt ascending, unknown last; more recent `last_seen` first, unknown last; id. It carries the
/// service worker's scoring factors as an ordering, not an additive score: a warm WAN peer does
/// not outrank a cold LAN peer of a narrower class (A15, H17).
#[derive(Debug, Clone, Copy, Default)]
pub struct ProximityOrder;

impl TierOrder for ProximityOrder {
    fn order(&self, root: &BlobCid, records: &mut Vec<TierRecord>) {
        records.sort_by(|a, b| {
            let warm = |r: &TierRecord| {
                !r.proximity
                    .as_ref()
                    .is_some_and(|p| p.warm_roots.contains(root))
            };
            let rank = |r: &TierRecord| r.proximity.as_ref().map_or(u8::MAX, |p| p.network_rank);
            let rtt = |r: &TierRecord| {
                r.proximity
                    .as_ref()
                    .and_then(|p| p.rtt_ms)
                    .map_or((1, 0), |ms| (0, ms))
            };
            let seen = |r: &TierRecord| {
                r.proximity
                    .as_ref()
                    .and_then(|p| p.last_seen_ms)
                    .map_or((1, 0), |ms| (0, u64::MAX - ms))
            };
            a.class
                .cmp(&b.class)
                .then_with(|| warm(a).cmp(&warm(b)))
                .then_with(|| rank(a).cmp(&rank(b)))
                .then_with(|| rtt(a).cmp(&rtt(b)))
                .then_with(|| seen(a).cmp(&seen(b)))
                .then_with(|| a.id.cmp(&b.id))
        });
    }
}

/// The typed reason inside `Unanswered<ItemAbsence>`. The come-back and the meanwhile ride beside
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
#[non_exhaustive]
pub enum ItemAbsence {
    /// Observed absent at every tier listed.
    NotHeld { tried: Vec<TierId> },
    /// Reach said no, by name.
    Refused { by: TierId, refusal: ReachRefusal },
    /// Never queued silently; the retry time is the come-back.
    OverBudget { by: TierId, rule: String },
    /// No answer: absence NOT established.
    Unreachable { tier: TierId },
    /// The tier could not establish the path's membership: NOT final, the chain advances (A3).
    MembershipUnknown { by: TierId },
    /// The bytes failed their address; never stored.
    Integrity { from: TierId },
}

impl ItemAbsence {
    /// Observed absent at every tier in `tried` (empty for a leaf store, B2).
    pub fn not_held(tried: Vec<TierId>) -> Unanswered<ItemAbsence> {
        let because = if tried.is_empty() {
            "the object is not held here and no tier was asked"
        } else {
            "no tier asked holds it"
        };
        Unanswered::new(
            ItemAbsence::NotHeld { tried },
            ComeBack::OnEvent {
                event: EVENT_TIER_REACHABLE.into(),
            },
            Meanwhile::Nothing {
                because: because.into(),
            },
        )
    }

    /// Refused by `by`'s reach gate, with the refusal it named.
    pub fn refused(by: &TierId, refusal: ReachRefusal) -> Unanswered<ItemAbsence> {
        Unanswered::new(
            ItemAbsence::Refused {
                by: by.clone(),
                refusal,
            },
            ComeBack::OnEvent {
                event: EVENT_STANDING_CHANGED.into(),
            },
            Meanwhile::Nothing {
                because: "the requester lacks the standing this root needs".into(),
            },
        )
    }

    /// Past `by`'s forwarding budget: come back at the time the gate gave.
    pub fn over_budget(by: &TierId, budget: &OverBudget) -> Unanswered<ItemAbsence> {
        Unanswered::new(
            ItemAbsence::OverBudget {
                by: by.clone(),
                rule: budget.rule.clone(),
            },
            ComeBack::At {
                epoch_secs: budget.retry_at_epoch_secs,
            },
            Meanwhile::Nothing {
                because: "the forwarding budget is spent until the retry time".into(),
            },
        )
    }

    /// `tier` gave no answer: absence not established.
    pub fn unreachable(tier: &TierId) -> Unanswered<ItemAbsence> {
        Unanswered::new(
            ItemAbsence::Unreachable { tier: tier.clone() },
            ComeBack::OnEvent {
                event: EVENT_TIER_REACHABLE.into(),
            },
            Meanwhile::Nothing {
                because: "the tier did not answer".into(),
            },
        )
    }

    /// `by` could not establish the path's membership under the root.
    pub fn membership_unknown(by: &TierId) -> Unanswered<ItemAbsence> {
        Unanswered::new(
            ItemAbsence::MembershipUnknown { by: by.clone() },
            ComeBack::OnEvent {
                event: EVENT_TIER_REACHABLE.into(),
            },
            Meanwhile::Nothing {
                because: "the tier does not hold the path's spine".into(),
            },
        )
    }

    /// Bytes from `from` failed their address.
    pub fn integrity(from: &TierId) -> Unanswered<ItemAbsence> {
        Unanswered::new(
            ItemAbsence::Integrity { from: from.clone() },
            ComeBack::OnEvent {
                event: EVENT_TIER_REACHABLE.into(),
            },
            Meanwhile::Nothing {
                because: "the bytes offered failed their address".into(),
            },
        )
    }

    /// The full mapping (A17): `EprfsError::Unanswered { why }` → `why`, unchanged; a corrupt
    /// object (`EprfsError::Corrupt`) → `Integrity`; `Io` and every other storage failure →
    /// `Unreachable { at }` (absence not established); only `BlobNotFound` and `BlobNotLocal` →
    /// `NotHeld { [] }`.
    ///
    /// A leaf store's own miss names no tier in `tried`, because `tried` lists the tiers a
    /// requester dialled (B2), and `at` is the store's `TierId`, which a requester-side chain
    /// rewrites to the dialled id when it is `own`. Only the typed variant is integrity: no error
    /// message is read.
    pub fn from_error(e: &EprfsError, at: &TierId) -> Unanswered<ItemAbsence> {
        match e {
            EprfsError::Unanswered { why, .. } => why.as_ref().clone(),
            EprfsError::BlobNotFound(_) | EprfsError::BlobNotLocal(_) => {
                ItemAbsence::not_held(Vec::new())
            }
            EprfsError::Corrupt(_) => ItemAbsence::integrity(at),
            _ => ItemAbsence::unreachable(at),
        }
    }

    /// The wire tag of this variant, from an exhaustive match: what a fieldless label face maps.
    pub fn kind(&self) -> &'static str {
        match self {
            ItemAbsence::NotHeld { .. } => "not-held",
            ItemAbsence::Refused { .. } => "refused",
            ItemAbsence::OverBudget { .. } => "over-budget",
            ItemAbsence::Unreachable { .. } => "unreachable",
            ItemAbsence::MembershipUnknown { .. } => "membership-unknown",
            ItemAbsence::Integrity { .. } => "integrity",
        }
    }

    /// One value of every variant, so a crate outside eprfs-core (which cannot match this
    /// non-exhaustive enum exhaustively) can test that its mapping covers each one.
    /// [`ItemAbsence::kind`]'s exhaustive match is what keeps this list complete.
    pub fn one_of_each() -> Vec<ItemAbsence> {
        let by = TierId::agent("uhCAkexample");
        vec![
            ItemAbsence::NotHeld {
                tried: vec![by.clone()],
            },
            ItemAbsence::Refused {
                by: by.clone(),
                refusal: ReachRefusal::new(crate::reach::ReachReason::OutsideStanding, None),
            },
            ItemAbsence::OverBudget {
                by: by.clone(),
                rule: "example".into(),
            },
            ItemAbsence::Unreachable { tier: by.clone() },
            ItemAbsence::MembershipUnknown { by: by.clone() },
            ItemAbsence::Integrity { from: by },
        ]
    }

    /// True when the absence is observed (`NotHeld`): every other kind leaves absence unestablished.
    pub fn is_observed(&self) -> bool {
        matches!(self, ItemAbsence::NotHeld { .. })
    }

    /// True for the two answers every walk reads as an error, never as an absence (B4): bytes
    /// that failed their address, and a failure of the own store. Only the own store can name the
    /// indexical `own` (it never crosses the wire, and a chain rewrites any remote `own` to the
    /// id it dialled), so `Unreachable { own }` means exactly that.
    pub fn is_walk_error(&self) -> bool {
        match self {
            ItemAbsence::Integrity { .. } => true,
            ItemAbsence::Unreachable { tier } => tier.is_own(),
            _ => false,
        }
    }

    /// Replace every indexical `own` in a remote answer with the id the requester dialled (A2).
    pub fn rewrite_own(&mut self, dialled: &TierId) {
        let fix = |id: &mut TierId| {
            if id.is_own() {
                *id = dialled.clone();
            }
        };
        match self {
            ItemAbsence::NotHeld { tried } => {
                tried.iter_mut().for_each(fix);
                let mut seen = Vec::with_capacity(tried.len());
                tried.retain(|id| {
                    if seen.contains(id) {
                        false
                    } else {
                        seen.push(id.clone());
                        true
                    }
                });
            }
            ItemAbsence::Refused { by, .. }
            | ItemAbsence::OverBudget { by, .. }
            | ItemAbsence::MembershipUnknown { by } => fix(by),
            ItemAbsence::Unreachable { tier } => fix(tier),
            ItemAbsence::Integrity { from } => fix(from),
        }
    }
}

/// Bounded upstream (spec §12.3 rule 3). A synchronous try-acquire, so the core needs no runtime.
/// On a person's own device the budget is the person's; on a shared tier the adapter reads it from
/// the space's charter, per requester (`on_behalf`, A4).
pub trait ForwardGate: Send + Sync {
    fn admit(
        &self,
        on_behalf: &Requester,
        to: &TierId,
        bytes_hint: Option<u64>,
    ) -> std::result::Result<ForwardPermit, OverBudget>;
}

/// Dropping it releases the slot it holds.
pub struct ForwardPermit {
    _release: Box<dyn Send + Sync>,
}

impl ForwardPermit {
    pub fn new(release: impl Send + Sync + 'static) -> Self {
        Self {
            _release: Box::new(release),
        }
    }
}

impl std::fmt::Debug for ForwardPermit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ForwardPermit")
    }
}

/// A gate's refusal. The gate has a clock and the core does not (R-E2), so the gate gives the
/// absolute come-back time as well as the relative one.
///
/// Revision of interfaces spec §A.2: the spec's `OverBudget { retry_after_ms, rule }` cannot give
/// R7's `ComeBack::At { epoch_secs }` without a clock in the core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverBudget {
    pub retry_after_ms: u64,
    pub retry_at_epoch_secs: u64,
    pub rule: String,
}

/// Builds a tier from a record, using `record.endpoint`. TypeScript has the same trait (§H.1).
pub trait TierConnector: Send + Sync {
    fn connect(&self, record: &TierRecord) -> Option<Arc<dyn EprfsStorage>>;
}

/// Per-tier counters, keyed by the id the chain dialled. Never attribution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TierCounts {
    /// Batches sent to this tier.
    pub asked: u64,
    /// Items it delivered verified bytes for.
    pub hits: u64,
    /// Items it answered absent.
    pub misses: u64,
    /// Items whose bytes failed their address.
    pub integrity: u64,
    /// Verified items whose write-back to the own store failed (the read still succeeded).
    pub write_back_failures: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChainStatsSnapshot {
    pub own_hits: u64,
    pub own_misses: u64,
    /// Duplicate records of one tier that disagreed on class.
    pub class_conflicts: u64,
    pub per_tier: BTreeMap<TierId, TierCounts>,
}

#[derive(Debug, Default)]
struct ChainStats(Mutex<ChainStatsSnapshot>);

impl ChainStats {
    fn update(&self, f: impl FnOnce(&mut ChainStatsSnapshot)) {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut guard);
    }

    fn tier(&self, id: &TierId, f: impl FnOnce(&mut TierCounts)) {
        self.update(|s| f(s.per_tier.entry(id.clone()).or_default()));
    }

    fn snapshot(&self) -> ChainStatsSnapshot {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// How much one outer ask may carry and keep (interfaces spec §4's bounded memory).
///
/// `max_request_items` is the per-request bound the chain and every serving tier share: a
/// [`TierChain`] never sends an outer tier a batch of more CIDs than this, and a serving endpoint
/// refuses a larger one (its `TreeLimits::max_pending_entries`, whose default this is). A chain
/// whose bound exceeds a tier's is that tier not answering, so the two stay at one default.
///
/// `max_write_back_bytes` bounds the verified bytes an ask holds for write-back: once a tier has
/// delivered that much, the chain tells it `Stop`, stores what it holds, and asks the same tier
/// again for what it had not yet answered. The chain's peak is therefore this bound plus one
/// object, whatever the batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainLimits {
    pub max_request_items: usize,
    pub max_write_back_bytes: u64,
}

impl Default for ChainLimits {
    fn default() -> Self {
        Self {
            max_request_items: crate::TreeLimits::default().max_pending_entries,
            max_write_back_bytes: 64 * 1024 * 1024,
        }
    }
}

/// The requester's composition of its own store and its outer tiers.
pub struct TierChain {
    own: Arc<dyn EprfsStorage>,
    /// Merged records, in no order: warmth is per root, so the chain orders per request.
    outer: Vec<(TierRecord, Arc<dyn EprfsStorage>)>,
    order: Arc<dyn TierOrder>,
    ask: Arc<dyn AskPolicy>,
    gate: Arc<dyn ForwardGate>,
    /// The requester itself, for its own reads.
    who: Requester,
    limits: ChainLimits,
    stats: ChainStats,
}

/// One CID still being sought, with every item that carries it.
struct Open<'i> {
    cid: &'i BlobCid,
    items: Vec<&'i BatchItem>,
    /// Tiers observed not holding it, in the order asked (and as forwarders reported).
    tried: Vec<TierId>,
    /// A final answer: refused or over budget.
    last: Option<Unanswered<ItemAbsence>>,
    /// An answer that leaves absence unestablished: unreachable, membership unknown, integrity.
    soft: Option<Unanswered<ItemAbsence>>,
    done: bool,
}

enum Delivered {
    Continue,
    Stopped,
}

impl TierChain {
    /// Merge the records by id, drop any record of the own store, and connect each; a record
    /// `connect` cannot dial is dropped. `order` is kept and applied per request, for the root
    /// asked: a long-lived chain (a serving tier's upstream) serves every root, and a tier warm
    /// for one root is not warm for another.
    ///
    /// Revision of interfaces spec §A.4: the constructor took a `root_hint` and ordered once.
    pub fn new(
        own: Arc<dyn EprfsStorage>,
        records: Vec<TierRecord>,
        order: Arc<dyn TierOrder>,
        connect: &dyn TierConnector,
        ask: Arc<dyn AskPolicy>,
        gate: Arc<dyn ForwardGate>,
        who: Requester,
    ) -> Self {
        let stats = ChainStats::default();
        let mut merged: Vec<TierRecord> = Vec::new();
        for record in records {
            if record.id.is_own() {
                continue;
            }
            match merged.iter_mut().find(|m| m.id == record.id) {
                Some(existing) => {
                    if existing.class != record.class {
                        stats.update(|s| s.class_conflicts += 1);
                    }
                    existing.merge(record);
                }
                None => merged.push(record),
            }
        }
        let outer = merged
            .into_iter()
            .filter_map(|record| connect.connect(&record).map(|tier| (record, tier)))
            .collect();
        Self {
            own,
            outer,
            order,
            ask,
            gate,
            who,
            limits: ChainLimits::default(),
            stats,
        }
    }

    /// Replace the default [`ChainLimits`]. A zero bound is raised to one.
    pub fn with_limits(mut self, limits: ChainLimits) -> Self {
        self.limits = ChainLimits {
            max_request_items: limits.max_request_items.max(1),
            max_write_back_bytes: limits.max_write_back_bytes.max(1),
        };
        self
    }

    pub fn limits(&self) -> ChainLimits {
        self.limits
    }

    /// The tiers this call may ask, in order: every connected tier in the policy's order for
    /// `root`, or, for a forwarded call, `restrict` in its own order with repeats and unknown ids
    /// dropped, so no call asks a tier twice and the list is never longer than the outer set,
    /// whatever the wire sent.
    fn candidates(
        &self,
        root: &BlobCid,
        restrict: Option<&[TierId]>,
    ) -> Vec<&(TierRecord, Arc<dyn EprfsStorage>)> {
        match restrict {
            None => {
                let mut records: Vec<TierRecord> = self
                    .outer
                    .iter()
                    .map(|(record, _)| record.clone())
                    .collect();
                self.order.order(root, &mut records);
                records
                    .iter()
                    .filter_map(|r| self.outer.iter().find(|(record, _)| record.id == r.id))
                    .collect()
            }
            Some(via) => {
                let mut picked: Vec<&(TierRecord, Arc<dyn EprfsStorage>)> = Vec::new();
                for id in via {
                    if picked.len() == self.outer.len() {
                        break;
                    }
                    if picked.iter().any(|(record, _)| &record.id == id) {
                        continue;
                    }
                    if let Some(found) = self.outer.iter().find(|(record, _)| &record.id == id) {
                        picked.push(found);
                    }
                }
                picked
            }
        }
    }

    pub fn own(&self) -> &Arc<dyn EprfsStorage> {
        &self.own
    }

    pub fn tiers(&self) -> impl Iterator<Item = &TierRecord> + '_ {
        self.outer.iter().map(|(record, _)| record)
    }

    pub fn stats(&self) -> ChainStatsSnapshot {
        self.stats.snapshot()
    }

    /// The chain's own identity, for the absences it names itself: the requester's agent key when
    /// it has one, else the own store's id.
    fn self_id(&self) -> TierId {
        self.who
            .agent()
            .map(TierId::agent)
            .unwrap_or_else(|| self.own.tier_id())
    }

    /// The serving-side entry (A4): forward a miss on behalf of an inbound requester. The gate is
    /// asked with `on_behalf` (a per-requester budget on a shared tier), and the outer list is
    /// intersected with `opts.via` in `via`'s order. The next tier sees this tier's own identity,
    /// so it judges reach for the forwarder; the endpoint has already judged the requester.
    ///
    /// The root's reach rides in `opts.root_reach` alone, the one channel the chain's own reads
    /// use too; a serving tier fills it from its own records ([`crate::ReachGate::root_level`]),
    /// never from the wire. (Revision of interfaces spec §A.2, which gave it a second channel as a
    /// parameter.)
    pub async fn forward_blobs(
        &self,
        on_behalf: &Requester,
        root: &BlobCid,
        items: &[BatchItem],
        opts: &BatchOptions,
        sink: &mut (dyn BlobSink + Send),
    ) -> Result<BatchReport> {
        self.answer(on_behalf, root, items, opts, Some(&opts.via), sink)
            .await
    }

    /// The chain as a store bound to one inbound requester (A4): every fetch through it is gated
    /// with `on_behalf`'s budget and asks only the tiers in `opts.via`, in that order, under
    /// `opts.root_reach`. A serving tier reads through it, so its membership spine fetches and
    /// its depth-0 queries (read, list, diff) are budgeted to the requester and restricted to the
    /// requester's tiers, exactly as its batch forwarding is, never to the tier's own standing.
    pub fn on_behalf(&self, on_behalf: &Requester, opts: &BatchOptions) -> OnBehalf<'_> {
        OnBehalf {
            chain: self,
            who: on_behalf.clone(),
            via: opts.via.clone(),
            root_reach: opts.root_reach,
        }
    }

    /// Deliver verified bytes to every item of one CID.
    fn deliver(
        report: &mut BatchReport,
        items: &[&BatchItem],
        bytes: &Bytes,
        from: &TierId,
        sink: &mut (dyn BlobSink + Send),
    ) -> Delivered {
        for item in items {
            report.arrived += 1;
            report.bytes += bytes.len() as u64;
            if sink.arrived(item, bytes.clone(), from) == SinkControl::Stop {
                report.stopped = true;
                return Delivered::Stopped;
            }
        }
        Delivered::Continue
    }

    /// The contract of interfaces spec §A.4: own store first without `has_blob`; `LocalOnly`
    /// stays local; ask only admitted tiers, strictly in order, within the gate; verify at every
    /// hop; store before return; every absence named with its three parts.
    async fn answer(
        &self,
        who: &Requester,
        root: &BlobCid,
        items: &[BatchItem],
        opts: &BatchOptions,
        restrict: Option<&[TierId]>,
        sink: &mut (dyn BlobSink + Send),
    ) -> Result<BatchReport> {
        let mut report = BatchReport::default();
        let own_id = self.own.tier_id();
        let local = BatchOptions {
            policy: FetchPolicy::LocalOnly,
            via: Vec::new(),
            root_reach: opts.root_reach,
        };

        // 1. The own store, without `has_blob` (contract 1).
        let mut open: Vec<Open<'_>> = Vec::new();
        for (cid, group) in group_by_cid(items) {
            let soft = match self.own.fetch_at(root, group[0], &local).await {
                Ok(handle) if handle.cid == *cid && cid.verifies(&handle.bytes) => {
                    self.stats.update(|s| s.own_hits += group.len() as u64);
                    if let Delivered::Stopped =
                        Self::deliver(&mut report, &group, &handle.bytes, &own_id, sink)
                    {
                        return Ok(report);
                    }
                    continue;
                }
                Ok(_) => Some(ItemAbsence::integrity(&own_id)),
                Err(error) => {
                    let why = ItemAbsence::from_error(&error, &own_id);
                    (!why.why.is_observed()).then_some(why)
                }
            };
            self.stats.update(|s| s.own_misses += group.len() as u64);
            open.push(Open {
                cid,
                items: group,
                tried: Vec::new(),
                last: None,
                soft,
                done: false,
            });
        }
        if open.is_empty() {
            return Ok(report);
        }

        // 2. The outer tiers, only when the policy allows leaving the device (contracts 2, 3).
        //    Each tier is asked in rounds of at most `max_request_items` CIDs (the bound a serving
        //    tier enforces), and a round ends early once it holds `max_write_back_bytes` for
        //    write-back; what a round did not reach is asked again of the same tier.
        if opts.policy == FetchPolicy::FetchIfMissing {
            let admitted: Vec<_> = self
                .candidates(root, restrict)
                .into_iter()
                .filter(|(record, _)| self.ask.may_ask(record, root, opts.root_reach))
                .collect();

            'tiers: for (position, (record, tier)) in admitted.iter().enumerate() {
                let tier_opts = BatchOptions {
                    policy: opts.policy,
                    via: admitted[position + 1..]
                        .iter()
                        .map(|(r, _)| r.id.clone())
                        .collect(),
                    root_reach: opts.root_reach,
                };
                // The items this tier is still to be asked, each once in this call whatever it
                // answers (a tier that answers soft is not asked the same item twice); an item a
                // paused round did not reach goes back to the front.
                let mut queue: std::collections::VecDeque<usize> = (0..open.len())
                    .filter(|&i| {
                        let o = &open[i];
                        !o.done && o.last.is_none() && !o.tried.contains(&record.id)
                    })
                    .collect();
                loop {
                    let mut wanting: Vec<usize> = Vec::new();
                    while wanting.len() < self.limits.max_request_items {
                        let Some(i) = queue.pop_front() else { break };
                        // A forwarder's report may have struck this tier since it was queued.
                        let o = &open[i];
                        if !o.done && o.last.is_none() && !o.tried.contains(&record.id) {
                            wanting.push(i);
                        }
                    }
                    if wanting.is_empty() {
                        break;
                    }
                    // No item size is known before it arrives, so no byte hint is given; the
                    // chain bounds what it holds itself (`max_write_back_bytes`).
                    let permit = match self.gate.admit(who, &record.id, None) {
                        Ok(permit) => permit,
                        Err(budget) => {
                            // Over budget is answered with its come-back time, never routed around.
                            let refusal = ItemAbsence::over_budget(&self.self_id(), &budget);
                            for o in open.iter_mut().filter(|o| !o.done && o.last.is_none()) {
                                o.last = Some(refusal.clone());
                            }
                            break 'tiers;
                        }
                    };
                    let requested: Vec<BatchItem> =
                        wanting.iter().map(|&i| open[i].items[0].clone()).collect();
                    self.stats.tier(&record.id, |c| c.asked += 1);
                    // The tier's answers stream through `OuterAsk`: each is filtered, verified and
                    // delivered on arrival, and the caller's `Stop` reaches the tier at once.
                    // Verified bytes wait for write-back, up to `max_write_back_bytes`.
                    let mut ask = OuterAsk {
                        tier: &record.id,
                        delegated: &tier_opts.via,
                        stats: &self.stats,
                        open: &mut open,
                        wanting: &wanting,
                        awaiting: Awaiting::new(requested.iter().map(|r| &r.cid)),
                        report: &mut report,
                        sink: &mut *sink,
                        write_back: Vec::new(),
                        held_bytes: 0,
                        max_held_bytes: self.limits.max_write_back_bytes,
                        paused: false,
                        stopped: false,
                    };
                    // A tier failing as a whole after some answers keeps what it verifiably gave;
                    // what it left unanswered is named below.
                    let _ = tier
                        .fetch_blobs(root, &requested, &tier_opts, &mut ask)
                        .await;
                    drop(permit);
                    let OuterAsk {
                        awaiting,
                        write_back,
                        paused,
                        stopped,
                        ..
                    } = ask;
                    // Store before return; a failed write-back still delivered (contract 7).
                    for (cid, bytes) in write_back {
                        if self.own.put_blob_verified(&cid, bytes).await.is_err() {
                            self.stats.tier(&record.id, |c| c.write_back_failures += 1);
                        }
                    }
                    if stopped {
                        return Ok(report);
                    }
                    let mut again: Vec<usize> = Vec::new();
                    for k in awaiting.remaining() {
                        if paused {
                            // The chain paused this round: ask the tier again. Every pause
                            // follows a delivery, so each round makes progress.
                            again.push(wanting[k]);
                        } else {
                            // A tier that failed as a whole, or left an item unanswered, did not
                            // answer it.
                            keep_soft(
                                &mut open[wanting[k]].soft,
                                ItemAbsence::unreachable(&record.id),
                            );
                        }
                    }
                    again.sort_unstable();
                    for i in again.into_iter().rev() {
                        queue.push_front(i);
                    }
                }
            }
        }

        // 3. Name what is still absent: a final refusal, else an unestablished absence, else an
        //    observed one listing every tier asked (contracts 9, 12, and B's contract 33).
        //    An answer every walk reads as an error (bytes this side verified and found bad, the
        //    own store failing) outranks a later final refusal: a refusal or the gate's over budget
        //    never degrades integrity to an absence (B4).
        for o in open.into_iter().filter(|o| !o.done) {
            let why = match o.soft {
                Some(soft) if soft.why.is_walk_error() => soft,
                soft => o
                    .last
                    .or(soft)
                    .unwrap_or_else(|| ItemAbsence::not_held(o.tried)),
            };
            for item in &o.items {
                report.absent += 1;
                sink.absent(item, why.clone());
            }
        }
        Ok(report)
    }
}

/// One outer tier's ask, as the sink its `fetch_blobs` streams into (interfaces spec §4: no batch
/// is buffered whole). Every answer is matched to the open item it answers through
/// [`Awaiting`]'s index, verified on arrival, and delivered to the caller's sink at once, so the
/// caller's `Stop` stops the tier. An answer for an unrequested CID, a second answer for one, or
/// bytes that fail their address are never kept (contracts 6, 10).
struct OuterAsk<'a, 'i, 's> {
    tier: &'a TierId,
    /// The tiers this ask handed `tier` as its `via`.
    delegated: &'a [TierId],
    stats: &'a ChainStats,
    open: &'a mut Vec<Open<'i>>,
    /// Requested position to index into `open`.
    wanting: &'a [usize],
    awaiting: Awaiting,
    report: &'a mut BatchReport,
    sink: &'a mut (dyn BlobSink + Send + 's),
    /// Verified objects to store once the tier returns: at most one per requested CID.
    write_back: Vec<(BlobCid, Bytes)>,
    /// The bytes `write_back` holds, against `max_held_bytes`.
    held_bytes: u64,
    max_held_bytes: u64,
    /// The chain told the tier `Stop` to write back what it holds; the round is asked again.
    paused: bool,
    /// The caller's sink said `Stop`.
    stopped: bool,
}

impl BlobSink for OuterAsk<'_, '_, '_> {
    fn arrived(&mut self, item: &BatchItem, bytes: Bytes, _from: &TierId) -> SinkControl {
        // After a pause nothing more is taken: an answer past it is asked again next round.
        if self.stopped || self.paused {
            return SinkControl::Stop;
        }
        let Some(k) = self.awaiting.claim(&item.cid) else {
            return self.awaiting.control();
        };
        let o = &mut self.open[self.wanting[k]];
        if o.cid.verifies(&bytes) {
            self.stats.tier(self.tier, |c| c.hits += 1);
            o.done = true;
            self.held_bytes += bytes.len() as u64;
            self.write_back.push((o.cid.clone(), bytes.clone()));
            if let Delivered::Stopped =
                TierChain::deliver(self.report, &o.items, &bytes, self.tier, self.sink)
            {
                self.stopped = true;
                return SinkControl::Stop;
            }
            if self.held_bytes >= self.max_held_bytes
                && self.awaiting.control() == SinkControl::Continue
            {
                self.paused = true;
                return SinkControl::Stop;
            }
        } else {
            self.stats.tier(self.tier, |c| c.integrity += 1);
            keep_soft(&mut o.soft, ItemAbsence::integrity(self.tier));
        }
        self.awaiting.control()
    }

    fn absent(&mut self, item: &BatchItem, mut why: Unanswered<ItemAbsence>) {
        if self.stopped || self.paused {
            return;
        }
        let Some(k) = self.awaiting.claim(&item.cid) else {
            return;
        };
        let o = &mut self.open[self.wanting[k]];
        self.stats.tier(self.tier, |c| c.misses += 1);
        why.why.rewrite_own(self.tier);
        match &why.why {
            ItemAbsence::NotHeld { tried } => {
                if !o.tried.contains(self.tier) {
                    o.tried.push(self.tier.clone());
                }
                // Only the tiers this chain handed the tier as its `via` count as asked by it
                // (contract 5): a tier cannot strike from the chain a tier it was never given, and
                // `tried` stays bounded by the chain's own outer set.
                for id in tried {
                    if self.delegated.contains(id) && !o.tried.contains(id) {
                        o.tried.push(id.clone());
                    }
                }
            }
            ItemAbsence::Refused { .. } | ItemAbsence::OverBudget { .. } => {
                o.last = Some(why);
            }
            // A tier's *report* that bytes failed their address is not this side's observation:
            // no bytes reached the requester, so it is absence not established and the chain
            // advances. Only bytes this side verified and found bad (`arrived` above) are the
            // walk error B4 names; otherwise any tier could abort a requester's walk by saying so.
            ItemAbsence::Integrity { .. } => keep_soft(&mut o.soft, reported_integrity(self.tier)),
            ItemAbsence::Unreachable { .. } | ItemAbsence::MembershipUnknown { .. } => {
                keep_soft(&mut o.soft, why)
            }
        }
    }
}

/// What a remote tier's `Integrity` statement becomes on the requester's side: the tier gave no
/// verifiable answer, so `Unreachable { tier }`, with the statement kept in the meanwhile's text.
fn reported_integrity(tier: &TierId) -> Unanswered<ItemAbsence> {
    Unanswered::new(
        ItemAbsence::Unreachable { tier: tier.clone() },
        ComeBack::OnEvent {
            event: EVENT_TIER_REACHABLE.into(),
        },
        Meanwhile::Nothing {
            because: "the tier reported that its copy failed its address; no bytes reached here"
                .into(),
        },
    )
}

/// A later tier's unestablished answer replaces an earlier one, except that an answer every walk
/// reads as an error (bytes that failed their address, the own store failing) is never degraded
/// to a mere absence by a later tier's silence (B4).
fn keep_soft(slot: &mut Option<Unanswered<ItemAbsence>>, why: Unanswered<ItemAbsence>) {
    if !slot.as_ref().is_some_and(|held| held.why.is_walk_error()) {
        *slot = Some(why);
    }
}

/// [`TierChain::on_behalf`]: the chain bound to one inbound requester's budget, tiers and root
/// reach. Pathless reads and writes go to the chain as they are.
pub struct OnBehalf<'c> {
    chain: &'c TierChain,
    who: Requester,
    via: Vec<TierId>,
    root_reach: Option<ReachLevel>,
}

#[async_trait]
impl EprfsStorage for OnBehalf<'_> {
    async fn resolve_epr(&self, reference: &EprRef) -> Result<EprRecord> {
        self.chain.resolve_epr(reference).await
    }

    async fn has_blob(&self, cid: &BlobCid) -> Result<BlobPresence> {
        self.chain.has_blob(cid).await
    }

    async fn fetch_blob(&self, cid: &BlobCid, policy: FetchPolicy) -> Result<BlobHandle> {
        self.chain.fetch_blob(cid, policy).await
    }

    async fn put_blob(&self, bytes: Bytes) -> Result<BlobCid> {
        self.chain.put_blob(bytes).await
    }

    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> Result<()> {
        self.chain.put_blob_verified(cid, bytes).await
    }

    async fn publish_attestation(&self, draft: AttestationDraft) -> Result<EprRef> {
        self.chain.publish_attestation(draft).await
    }

    fn tier_id(&self) -> TierId {
        self.chain.tier_id()
    }

    async fn fetch_at(
        &self,
        root: &BlobCid,
        item: &BatchItem,
        opts: &BatchOptions,
    ) -> Result<BlobHandle> {
        let mut collected = CollectSink::default();
        self.fetch_blobs(root, std::slice::from_ref(item), opts, &mut collected)
            .await?;
        one_answer(item, collected, &self.chain.self_id())
    }

    /// The walk's options give the policy; the requester's `via` and the root's reach are the
    /// binding's, whatever the walk passed.
    async fn fetch_blobs(
        &self,
        root: &BlobCid,
        items: &[BatchItem],
        opts: &BatchOptions,
        sink: &mut (dyn BlobSink + Send),
    ) -> Result<BatchReport> {
        let bound = BatchOptions {
            policy: opts.policy,
            via: self.via.clone(),
            root_reach: self.root_reach,
        };
        self.chain
            .answer(&self.who, root, items, &bound, Some(&self.via), sink)
            .await
    }
}

#[async_trait]
impl EprfsStorage for TierChain {
    async fn resolve_epr(&self, reference: &EprRef) -> Result<EprRecord> {
        self.own.resolve_epr(reference).await
    }

    /// The own store's presence; when it misses and outer tiers exist, `Unknown`, never `Missing`
    /// (A18): a pathless question never leaves the device.
    async fn has_blob(&self, cid: &BlobCid) -> Result<BlobPresence> {
        match self.own.has_blob(cid).await? {
            BlobPresence::Missing if !self.outer.is_empty() => Ok(BlobPresence::Unknown),
            presence => Ok(presence),
        }
    }

    /// Own only. Under `FetchIfMissing` with outer tiers, an own miss is `RootRequired`, never a
    /// silent local-only answer and never `BlobNotFound` (A18).
    async fn fetch_blob(&self, cid: &BlobCid, policy: FetchPolicy) -> Result<BlobHandle> {
        match self.own.fetch_blob(cid, FetchPolicy::LocalOnly).await {
            Err(EprfsError::BlobNotFound(_) | EprfsError::BlobNotLocal(_))
                if policy == FetchPolicy::FetchIfMissing && !self.outer.is_empty() =>
            {
                Err(EprfsError::RootRequired(cid.clone()))
            }
            other => other,
        }
    }

    async fn put_blob(&self, bytes: Bytes) -> Result<BlobCid> {
        self.own.put_blob(bytes).await
    }

    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> Result<()> {
        self.own.put_blob_verified(cid, bytes).await
    }

    async fn publish_attestation(&self, draft: AttestationDraft) -> Result<EprRef> {
        self.own.publish_attestation(draft).await
    }

    fn tier_id(&self) -> TierId {
        self.own.tier_id()
    }

    /// A one-item [`EprfsStorage::fetch_blobs`] (A8).
    async fn fetch_at(
        &self,
        root: &BlobCid,
        item: &BatchItem,
        opts: &BatchOptions,
    ) -> Result<BlobHandle> {
        let mut collected = CollectSink::default();
        self.fetch_blobs(root, std::slice::from_ref(item), opts, &mut collected)
            .await?;
        one_answer(item, collected, &self.self_id())
    }

    async fn fetch_blobs(
        &self,
        root: &BlobCid,
        items: &[BatchItem],
        opts: &BatchOptions,
        sink: &mut (dyn BlobSink + Send),
    ) -> Result<BatchReport> {
        self.answer(&self.who, root, items, opts, None, sink).await
    }
}

/// One item's batch answer as a one-object fetch: its bytes, or its absence as
/// `EprfsError::Unanswered`. A batch that left the item unanswered is `Unreachable { by }`.
fn one_answer(item: &BatchItem, mut collected: CollectSink, by: &TierId) -> Result<BlobHandle> {
    match collected.answers.pop() {
        Some((_, BatchAnswer::Arrived { bytes, .. })) => Ok(BlobHandle {
            cid: item.cid.clone(),
            bytes,
        }),
        Some((_, BatchAnswer::Absent(why))) => Err(EprfsError::Unanswered {
            cid: item.cid.clone(),
            why: Box::new(why),
        }),
        None => Err(EprfsError::Unanswered {
            cid: item.cid.clone(),
            why: Box::new(ItemAbsence::unreachable(by)),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reach::ReachReason;

    fn record(id: TierId, class: TierClass) -> TierRecord {
        TierRecord {
            id,
            class,
            sources: vec![TierSource::HeldRecord {
                record: "roster".into(),
            }],
            endpoint: None,
            standing: None,
            proximity: None,
        }
    }

    #[test]
    fn own_tier_id_never_serializes_and_prefixed_ids_round_trip() {
        assert!(serde_json::to_string(&TierId::own()).is_err());
        let agent = TierId::agent("uhCAkabc");
        assert_eq!(serde_json::to_string(&agent).unwrap(), "\"agent:uhCAkabc\"");
        assert_eq!(
            serde_json::from_str::<TierId>("\"transport:12D3Koo\"").unwrap(),
            TierId::transport("12D3Koo")
        );
        for bad in ["\"own\"", "\"uhCAkabc\"", "\"agent:\"", "\"transport:\""] {
            assert!(serde_json::from_str::<TierId>(bad).is_err(), "{bad}");
        }
        assert_eq!(agent.as_str(), "uhCAkabc");
    }

    #[test]
    fn item_absence_wire_form_is_kind_tagged_camel_case() {
        let absence = ItemAbsence::Refused {
            by: TierId::agent("uhCAkhub"),
            refusal: ReachRefusal::new(ReachReason::OutsideStanding, None),
        };
        assert_eq!(
            serde_json::to_value(&absence).unwrap(),
            serde_json::json!({
                "kind": "refused",
                "by": "agent:uhCAkhub",
                "refusal": {"reason": "outside-standing", "needsStanding": null}
            })
        );
        for absence in ItemAbsence::one_of_each() {
            let json = serde_json::to_value(&absence).unwrap();
            assert_eq!(json["kind"], absence.kind());
            assert_eq!(
                serde_json::from_value::<ItemAbsence>(json).unwrap(),
                absence
            );
        }
    }

    #[test]
    fn from_error_maps_only_not_found_to_not_held() {
        let at = TierId::agent("uhCAkme");
        let cid = BlobCid::compute_raw(b"x");
        let kind = |e: EprfsError| ItemAbsence::from_error(&e, &at).why;
        assert_eq!(
            kind(EprfsError::BlobNotFound(cid.clone())),
            ItemAbsence::NotHeld { tried: vec![] }
        );
        assert_eq!(
            kind(EprfsError::BlobNotLocal(cid.clone())),
            ItemAbsence::NotHeld { tried: vec![] }
        );
        assert_eq!(
            kind(EprfsError::Corrupt(cid.clone())),
            ItemAbsence::Integrity { from: at.clone() }
        );
        // No message is read: a storage error that says "corrupt" is not integrity.
        assert_eq!(
            kind(EprfsError::Storage(format!(
                "existing content object is corrupt: {cid}"
            ))),
            ItemAbsence::Unreachable { tier: at.clone() }
        );
        assert_eq!(
            kind(EprfsError::Storage("disk on fire".into())),
            ItemAbsence::Unreachable { tier: at.clone() }
        );
        let inner = ItemAbsence::membership_unknown(&TierId::agent("uhCAkhub"));
        assert_eq!(
            ItemAbsence::from_error(
                &EprfsError::Unanswered {
                    cid,
                    why: Box::new(inner.clone())
                },
                &at
            ),
            inner
        );
    }

    #[test]
    fn every_absence_constructor_carries_its_three_parts() {
        let by = TierId::agent("uhCAkhub");
        let budget = OverBudget {
            retry_after_ms: 2000,
            retry_at_epoch_secs: 1_700_000_002,
            rule: "household-forward".into(),
        };
        let over = ItemAbsence::over_budget(&by, &budget);
        assert_eq!(
            over.come_back,
            ComeBack::At {
                epoch_secs: 1_700_000_002
            }
        );
        for u in [
            ItemAbsence::not_held(vec![by.clone()]),
            ItemAbsence::refused(&by, ReachRefusal::new(ReachReason::NotUnderRoot, None)),
            over,
            ItemAbsence::unreachable(&by),
            ItemAbsence::membership_unknown(&by),
            ItemAbsence::integrity(&by),
        ] {
            assert!(u.escalation.is_none());
            match &u.meanwhile {
                Meanwhile::Nothing { because } => assert!(!because.is_empty()),
                other => panic!("unexpected meanwhile {other:?}"),
            }
        }
    }

    #[test]
    fn rewrite_own_replaces_the_indexical_id_and_dedupes() {
        let dialled = TierId::agent("uhCAklaptop");
        let mut absence = ItemAbsence::NotHeld {
            tried: vec![TierId::own(), dialled.clone()],
        };
        absence.rewrite_own(&dialled);
        assert_eq!(
            absence,
            ItemAbsence::NotHeld {
                tried: vec![dialled.clone()]
            }
        );
        let mut refused = ItemAbsence::Integrity {
            from: TierId::own(),
        };
        refused.rewrite_own(&dialled);
        assert_eq!(refused, ItemAbsence::Integrity { from: dialled });
    }

    #[test]
    fn merge_unions_sources_and_keeps_the_narrower_class() {
        let id = TierId::agent("uhCAkpeer");
        let mut a = record(id.clone(), TierClass::DoorwayPeer);
        let mut b = record(id, TierClass::Household);
        b.sources = vec![TierSource::Doorway {
            doorway: TierId::agent("uhCAkdoorway"),
        }];
        b.endpoint = Some(TierAddress::Url {
            base: "https://peer".into(),
        });
        b.standing = Some(ReachLevel(2));
        a.merge(b);
        assert_eq!(a.class, TierClass::Household);
        assert_eq!(a.sources.len(), 2);
        assert!(a.endpoint.is_some());
        assert_eq!(a.standing, Some(ReachLevel(2)));
    }

    #[test]
    fn proximity_order_ranks_class_then_warmth_then_network_then_rtt() {
        let root = BlobCid::compute(b"root");
        let prox = |rank: u8, warm: bool, rtt: Option<u32>| Proximity {
            network_rank: rank,
            warm_roots: if warm { vec![root.clone()] } else { vec![] },
            serves_extracted: true,
            rtt_ms: rtt,
            last_seen_ms: None,
        };
        let mut cold_lan = record(TierId::agent("a"), TierClass::DoorwayPeer);
        cold_lan.proximity = Some(prox(0, false, Some(5)));
        let mut warm_wan = record(TierId::agent("b"), TierClass::DoorwayPeer);
        warm_wan.proximity = Some(prox(1, true, Some(50)));
        let mut hub = record(TierId::agent("c"), TierClass::Hub);
        hub.proximity = Some(prox(2, false, None));
        let mut unknown_rtt = record(TierId::agent("d"), TierClass::DoorwayPeer);
        unknown_rtt.proximity = Some(prox(0, false, None));
        let mut records = vec![unknown_rtt, cold_lan, warm_wan, hub];
        ProximityOrder.order(&root, &mut records);
        let ids: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["c", "b", "a", "d"]);
    }

    #[test]
    fn tier_record_wire_form_matches_the_typescript_shape() {
        let mut r = record(TierId::agent("uhCAkhub"), TierClass::Hub);
        r.endpoint = Some(TierAddress::Url {
            base: "https://hub.example".into(),
        });
        r.standing = Some(ReachLevel(3));
        r.proximity = Some(Proximity {
            network_rank: 0,
            warm_roots: vec![],
            serves_extracted: false,
            rtt_ms: Some(4),
            last_seen_ms: None,
        });
        assert_eq!(
            serde_json::to_value(&r).unwrap(),
            serde_json::json!({
                "id": "agent:uhCAkhub",
                "class": "hub",
                "sources": [{"source": "held-record", "record": "roster"}],
                "endpoint": {"form": "url", "base": "https://hub.example"},
                "standing": 3,
                "proximity": {
                    "networkRank": 0, "warmRoots": [], "servesExtracted": false,
                    "rttMs": 4, "lastSeenMs": null
                }
            })
        );
    }
}

/// Seam B's contract items 30–33 (interfaces spec §B): the walk over a tier chain.
#[cfg(test)]
mod walk_over_chain {
    use super::*;
    use crate::reach::{KeyCustody, ReachReason};
    use crate::test_support::{
        household_record, ten_thousand, tp, AllowAll, Answering, Connect, CountingStorage,
        OpenGate, Unverified,
    };
    use crate::walk::{walk_collect, AbsentReason, TreePath, View, WalkItem};
    use crate::{level_walk::walk_batched, TreeLimits};

    fn chain(own: CountingStorage, outer: Vec<(TierId, Arc<dyn EprfsStorage>)>) -> TierChain {
        let records = outer.iter().map(|(id, _)| household_record(id)).collect();
        let connect = Connect(outer.into_iter().collect());
        TierChain::new(
            Arc::new(own),
            records,
            Arc::new(ProximityOrder),
            &connect,
            Arc::new(AllowAll),
            Arc::new(OpenGate),
            Requester::authenticated("uhCAkphone", KeyCustody::DeviceHeld),
        )
    }

    fn opts(policy: FetchPolicy) -> BatchOptions {
        policy.into()
    }

    /// (30) A walk over a chain whose own store holds only the root fetches every other node from
    /// the outer tier exactly once under `FetchIfMissing`, and writes each back.
    #[tokio::test]
    async fn walk_over_chain_fetches_each_node_from_the_outer_tier_once() {
        let mut laptop = CountingStorage::new().with_id(TierId::agent("uhCAklaptop"));
        let (root, nodes) = ten_thousand(&mut laptop, false);
        let mut own = CountingStorage::new();
        own.blobs.insert(root.clone(), laptop.blobs[&root].clone());
        let laptop = Arc::new(laptop);
        let laptop_id = TierId::agent("uhCAklaptop");
        let c = chain(
            own,
            vec![(laptop_id.clone(), laptop.clone() as Arc<dyn EprfsStorage>)],
        );
        let view = View::from_tree_paths([tp("a1/b2/f003"), tp("a4/b4/f099"), tp("a9/b0")]);
        let walked = walk_collect(
            &c,
            &root,
            &view,
            &opts(FetchPolicy::FetchIfMissing),
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(walked.nodes.len(), 7);
        let mut fetched = laptop.fetched();
        fetched.sort();
        let mut expected: Vec<BlobCid> = ["a1", "a1/b2", "a4", "a4/b4", "a9", "a9/b0"]
            .iter()
            .map(|p| nodes[&tp(p)].clone())
            .collect();
        expected.sort();
        assert_eq!(
            fetched, expected,
            "each non-root node from the laptop, once"
        );
        let stats = c.stats();
        assert_eq!(stats.own_hits, 1);
        assert_eq!(stats.per_tier[&laptop_id].hits, 6);
        assert_eq!(stats.per_tier[&laptop_id].write_back_failures, 0);

        // A second walk is an own hit throughout: store before return.
        laptop.reset();
        walk_collect(
            &c,
            &root,
            &view,
            &opts(FetchPolicy::FetchIfMissing),
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        assert!(laptop.fetched().is_empty());

        // The level-batched walk over a cold chain asks the laptop once per level.
        let mut own = CountingStorage::new();
        own.blobs.insert(root.clone(), laptop.blobs[&root].clone());
        let cold = chain(
            own,
            vec![(laptop_id.clone(), laptop.clone() as Arc<dyn EprfsStorage>)],
        );
        let batched = walk_batched(
            &cold,
            &root,
            &view,
            &opts(FetchPolicy::FetchIfMissing),
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(batched.nodes.len(), 7);
        assert_eq!(
            cold.stats().per_tier[&laptop_id].asked,
            2,
            "levels a* and a*/b*"
        );
    }

    /// (31) Under `LocalOnly` the same walk yields `NotHeld` per missing subtree and makes zero
    /// outer calls.
    #[tokio::test]
    async fn local_only_walk_over_chain_names_not_held_and_never_dials() {
        let mut laptop = CountingStorage::new();
        let (root, _) = ten_thousand(&mut laptop, false);
        let mut own = CountingStorage::new();
        own.blobs.insert(root.clone(), laptop.blobs[&root].clone());
        let laptop = Arc::new(laptop);
        let c = chain(
            own,
            vec![(
                TierId::agent("uhCAklaptop"),
                laptop.clone() as Arc<dyn EprfsStorage>,
            )],
        );
        let view = View::from_tree_paths([tp("a1/b2"), tp("a4")]);
        let walked = walk_collect(
            &c,
            &root,
            &view,
            &opts(FetchPolicy::LocalOnly),
            &TreeLimits::default(),
        )
        .await
        .unwrap();
        assert!(laptop.fetched().is_empty());
        assert_eq!(walked.absent.len(), 2);
        for item in &walked.absent {
            assert!(matches!(
                item,
                WalkItem::Absent { reason: AbsentReason::NotHeld { tried, .. }, unanswered: Some(_), .. }
                    if tried.is_empty()
            ));
        }
    }

    /// (32) The depth-0 query defaults equal the free functions, including through `&dyn`.
    #[tokio::test]
    async fn depth_zero_query_defaults_equal_the_free_functions_through_dyn() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, true);
        let limits = TreeLimits::default();
        let o = opts(FetchPolicy::LocalOnly);
        let dynamic: &dyn EprfsStorage = &store;
        let path = tp("a3/b7/f042");
        assert_eq!(
            dynamic
                .query_read_path(&root, &path, &o, &limits)
                .await
                .unwrap(),
            crate::walk::read_path(&store, &root, &path, &o, &limits)
                .await
                .unwrap()
        );
        assert_eq!(
            dynamic
                .query_list_dir(&root, &tp("a3/b7"), &o, &limits)
                .await
                .unwrap(),
            crate::walk::list_dir(&store, &root, &tp("a3/b7"), &o, &limits)
                .await
                .unwrap()
        );
        assert_eq!(
            dynamic
                .query_diff(&root, &root, &View::whole(), &o, &limits)
                .await
                .unwrap(),
            Vec::new()
        );
        let walked = walk_collect(
            dynamic,
            &root,
            &View::from_tree_paths([tp("a2")]),
            &o,
            &limits,
        )
        .await
        .unwrap();
        assert_eq!(walked.nodes.len(), 12);
    }

    /// (33) A walk whose tier answers `Refused` or `Unreachable` yields `Withheld` and continues;
    /// nothing that is not observed absent reads as `NotHeld`.
    #[tokio::test]
    async fn refused_or_unreachable_tiers_yield_withheld_and_the_walk_continues() {
        let mut source = CountingStorage::new();
        let (root, _) = ten_thousand(&mut source, false);
        let hub = TierId::agent("uhCAkhub");
        for why in [
            ItemAbsence::refused(&hub, ReachRefusal::new(ReachReason::OutsideStanding, None)),
            ItemAbsence::unreachable(&hub),
        ] {
            let mut own = CountingStorage::new();
            own.blobs.insert(root.clone(), source.blobs[&root].clone());
            let answering: Arc<dyn EprfsStorage> = Arc::new(Answering {
                id: hub.clone(),
                why: why.clone(),
            });
            let c = chain(own, vec![(hub.clone(), answering)]);
            let view = View::from_tree_paths([tp("a1"), tp("a2")]);
            let walked = walk_collect(
                &c,
                &root,
                &view,
                &opts(FetchPolicy::FetchIfMissing),
                &TreeLimits::default(),
            )
            .await
            .unwrap();
            assert_eq!(
                walked.nodes.len(),
                1,
                "the root, then two withheld subtrees"
            );
            assert_eq!(walked.absent.len(), 2);
            for item in &walked.absent {
                match item {
                    WalkItem::Absent {
                        reason: AbsentReason::Withheld { why: w, .. },
                        unanswered: Some(u),
                        ..
                    } => {
                        assert_eq!(w, &why.why);
                        assert_eq!(u, &why);
                    }
                    other => panic!("expected Withheld, got {other:?}"),
                }
            }
        }
    }

    /// The four walks a chain answer reaches, gathered as error kinds: depth-first, level-batched,
    /// `read_path` and `list_dir` must agree (B.1 level equivalence over a chain, B4).
    async fn error_kinds(c: &TierChain, root: &BlobCid, view: &View) -> Vec<&'static str> {
        let kind = |e: &crate::TreeError| match e {
            crate::TreeError::Integrity(_) => "integrity",
            crate::TreeError::Storage { .. } => "storage",
            other => panic!("unexpected error {other:?}"),
        };
        let fetch = opts(FetchPolicy::FetchIfMissing);
        let limits = TreeLimits::default();
        vec![
            kind(
                &walk_collect(c, root, view, &fetch, &limits)
                    .await
                    .unwrap_err(),
            ),
            kind(
                &walk_batched(c, root, view, &fetch, &limits)
                    .await
                    .unwrap_err(),
            ),
            kind(
                &crate::list_dir(c, root, &tp("a1/b2"), &fetch, &limits)
                    .await
                    .unwrap_err(),
            ),
            kind(
                &crate::resolve_links(c, root, &[tp("a1/b2")], &fetch, &limits)
                    .await
                    .unwrap()
                    .pop()
                    .unwrap()
                    .unwrap_err(),
            ),
        ]
    }

    /// (34) An outer tier serving bytes that fail their address is an integrity error in every
    /// walk over the chain, never a `Withheld` absence, and a later tier's silence does not
    /// degrade it.
    #[tokio::test]
    async fn a_tampering_outer_tier_is_an_integrity_error_in_every_walk() {
        let mut laptop = CountingStorage::new().with_id(TierId::agent("uhCAka-laptop"));
        let (root, nodes) = ten_thousand(&mut laptop, false);
        let mut own = CountingStorage::new();
        own.blobs.insert(root.clone(), laptop.blobs[&root].clone());
        laptop.tamper(&nodes[&tp("a1")], Bytes::from_static(b"not the node"));
        let silent: Arc<dyn EprfsStorage> = Arc::new(Answering {
            id: TierId::agent("uhCAkb-hub"),
            why: ItemAbsence::unreachable(&TierId::agent("uhCAkb-hub")),
        });
        let c = chain(
            own,
            vec![
                (TierId::agent("uhCAka-laptop"), Arc::new(Unverified(laptop))),
                (TierId::agent("uhCAkb-hub"), silent),
            ],
        );
        let view = View::from_tree_paths([tp("a1/b2")]);
        assert_eq!(
            error_kinds(&c, &root, &view).await,
            vec!["integrity"; 4],
            "depth-first, level-batched, list and resolve agree"
        );
    }

    /// (34, continued) Integrity this side verified outranks every later answer: a later tier's
    /// `Refused`, and the gate refusing the next dial (`OverBudget`), are final for the item but
    /// never degrade the integrity failure to a `Withheld` absence.
    #[tokio::test]
    async fn integrity_outranks_a_later_refusal_and_the_gate_refusing_the_next_dial() {
        struct RefuseDialTo(TierId);
        impl ForwardGate for RefuseDialTo {
            fn admit(
                &self,
                _: &Requester,
                to: &TierId,
                _: Option<u64>,
            ) -> std::result::Result<ForwardPermit, OverBudget> {
                if to == &self.0 {
                    Err(OverBudget {
                        retry_after_ms: 1000,
                        retry_at_epoch_secs: 1_700_000_001,
                        rule: "per-tier".into(),
                    })
                } else {
                    Ok(ForwardPermit::new(()))
                }
            }
        }
        let laptop_id = TierId::agent("uhCAka-laptop");
        let hub = TierId::agent("uhCAkb-hub");
        let tampered = || {
            let mut laptop = CountingStorage::new().with_id(laptop_id.clone());
            let (root, nodes) = ten_thousand(&mut laptop, false);
            let mut own = CountingStorage::new();
            own.blobs.insert(root.clone(), laptop.blobs[&root].clone());
            laptop.tamper(&nodes[&tp("a1")], Bytes::from_static(b"not the node"));
            (root, own, laptop)
        };
        let view = View::from_tree_paths([tp("a1/b2")]);

        // Tamper, then a refusal.
        let (root, own, laptop) = tampered();
        let refusing: Arc<dyn EprfsStorage> = Arc::new(Answering {
            id: hub.clone(),
            why: ItemAbsence::refused(&hub, ReachRefusal::new(ReachReason::OutsideStanding, None)),
        });
        let c = chain(
            own,
            vec![
                (laptop_id.clone(), Arc::new(Unverified(laptop))),
                (hub.clone(), refusing),
            ],
        );
        assert_eq!(error_kinds(&c, &root, &view).await, vec!["integrity"; 4]);

        // Tamper, then the gate refuses the dial to the next tier.
        let (root, own, laptop) = tampered();
        let never: Arc<dyn EprfsStorage> = Arc::new(CountingStorage::new().with_id(hub.clone()));
        let tiers: Vec<(TierId, Arc<dyn EprfsStorage>)> = vec![
            (laptop_id.clone(), Arc::new(Unverified(laptop))),
            (hub.clone(), never),
        ];
        let records = tiers.iter().map(|(id, _)| household_record(id)).collect();
        let c = TierChain::new(
            Arc::new(own),
            records,
            Arc::new(ProximityOrder),
            &Connect(tiers.into_iter().collect()),
            Arc::new(AllowAll),
            Arc::new(RefuseDialTo(hub.clone())),
            Requester::authenticated("uhCAkphone", KeyCustody::DeviceHeld),
        );
        assert_eq!(error_kinds(&c, &root, &view).await, vec!["integrity"; 4]);
    }

    /// (40) A tier's *statement* that its copy failed its address is not an integrity failure this
    /// side observed: no bytes arrived, so the walk names it `Withheld { Unreachable { tier } }`
    /// and continues, and a later tier that holds the object still answers it.
    #[tokio::test]
    async fn a_remote_integrity_statement_is_withheld_not_a_walk_error() {
        let mut source = CountingStorage::new();
        let (root, _) = ten_thousand(&mut source, false);
        let hub = TierId::agent("uhCAkhub");
        let claiming = || -> Arc<dyn EprfsStorage> {
            Arc::new(Answering {
                id: hub.clone(),
                why: ItemAbsence::integrity(&hub),
            })
        };
        let mut own = CountingStorage::new();
        own.blobs.insert(root.clone(), source.blobs[&root].clone());
        let c = chain(own, vec![(hub.clone(), claiming())]);
        let view = View::from_tree_paths([tp("a1"), tp("a2")]);
        let fetch = opts(FetchPolicy::FetchIfMissing);
        let limits = TreeLimits::default();
        for walked in [
            walk_collect(&c, &root, &view, &fetch, &limits)
                .await
                .unwrap(),
            walk_batched(&c, &root, &view, &fetch, &limits)
                .await
                .unwrap(),
        ] {
            assert_eq!(walked.nodes.len(), 1);
            assert_eq!(walked.absent.len(), 2);
            for item in &walked.absent {
                assert!(
                    matches!(
                        item,
                        WalkItem::Absent {
                            reason: AbsentReason::Withheld {
                                why: ItemAbsence::Unreachable { tier },
                                ..
                            },
                            ..
                        } if tier == &hub
                    ),
                    "expected Withheld Unreachable, got {item:?}"
                );
            }
        }

        // A later tier holding the object answers it.
        let mut own = CountingStorage::new();
        own.blobs.insert(root.clone(), source.blobs[&root].clone());
        let laptop_id = TierId::agent("uhCAklaptop");
        let laptop = Arc::new(source.with_id(laptop_id.clone()));
        let c = chain(
            own,
            vec![
                (hub.clone(), claiming()),
                (laptop_id, laptop as Arc<dyn EprfsStorage>),
            ],
        );
        let walked = walk_collect(&c, &root, &view, &fetch, &limits)
            .await
            .unwrap();
        assert!(walked.absent.is_empty());
    }

    /// (35) The own store failing a read (not missing the object) is a storage error in every walk
    /// over the chain, whatever the outer tiers then say.
    #[tokio::test]
    async fn the_own_store_failing_a_read_is_a_storage_error_in_every_walk() {
        let mut source = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut source, false);
        let mut own = CountingStorage::new();
        own.blobs.insert(root.clone(), source.blobs[&root].clone());
        own.fail = Some(nodes[&tp("a1")].clone());
        let hub = TierId::agent("uhCAkhub");
        let not_held: Arc<dyn EprfsStorage> = Arc::new(Answering {
            id: hub.clone(),
            why: ItemAbsence::not_held(Vec::new()),
        });
        let c = chain(own, vec![(hub, not_held)]);
        let view = View::from_tree_paths([tp("a1/b2")]);
        assert_eq!(error_kinds(&c, &root, &view).await, vec!["storage"; 4]);
    }

    /// (36) A forwarder's `NotHeld.tried` strikes from the chain only the tiers the chain handed
    /// it as `via` (contract 5); an id it was never given is dropped, so it can neither grow the
    /// list nor stop the chain asking a tier it did not delegate.
    #[tokio::test]
    async fn forwarded_tried_counts_only_the_tiers_the_chain_delegated() {
        let mut source = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut source, false);
        let a = TierId::agent("uhCAka");
        let b = TierId::agent("uhCAkb");
        let z = TierId::agent("uhCAkz-never-dialled");
        let liar: Arc<dyn EprfsStorage> = Arc::new(Answering {
            id: a.clone(),
            why: ItemAbsence::not_held(vec![z.clone()]),
        });
        let honest = Arc::new(CountingStorage::new().with_id(b.clone()));
        let c = chain(
            CountingStorage::new(),
            vec![(a.clone(), liar), (b.clone(), honest.clone())],
        );
        let item = BatchItem::new(nodes[&tp("a1")].clone(), tp("a1"));
        let mut sink = CollectSink::default();
        c.fetch_blobs(
            &root,
            std::slice::from_ref(&item),
            &opts(FetchPolicy::FetchIfMissing),
            &mut sink,
        )
        .await
        .unwrap();
        match &sink.answers[..] {
            [(_, BatchAnswer::Absent(why))] => {
                assert_eq!(why.why, ItemAbsence::NotHeld { tried: vec![a, b] })
            }
            other => panic!("expected one NotHeld, got {other:?}"),
        }
        assert_eq!(
            honest.fetched(),
            vec![item.cid.clone()],
            "b was still asked"
        );

        // Naming a tier it WAS handed strikes it: strict tiering, the forwarder asked it.
        let liar: Arc<dyn EprfsStorage> = Arc::new(Answering {
            id: TierId::agent("uhCAka"),
            why: ItemAbsence::not_held(vec![TierId::agent("uhCAkb")]),
        });
        let honest = Arc::new(CountingStorage::new().with_id(TierId::agent("uhCAkb")));
        let c = chain(
            CountingStorage::new(),
            vec![
                (TierId::agent("uhCAka"), liar),
                (TierId::agent("uhCAkb"), honest.clone()),
            ],
        );
        let mut sink = CollectSink::default();
        c.fetch_blobs(
            &root,
            std::slice::from_ref(&item),
            &opts(FetchPolicy::FetchIfMissing),
            &mut sink,
        )
        .await
        .unwrap();
        assert!(honest.fetched().is_empty());
    }

    /// (37) `on_behalf` binds the chain to the inbound requester: its gate is asked for that
    /// requester, and only the requester's `via` is dialled, whatever a walk's options say.
    #[tokio::test]
    async fn on_behalf_gates_for_the_requester_and_dials_only_its_via() {
        struct OnlyFor(&'static str);
        impl ForwardGate for OnlyFor {
            fn admit(
                &self,
                on_behalf: &Requester,
                _: &TierId,
                _: Option<u64>,
            ) -> std::result::Result<ForwardPermit, OverBudget> {
                if on_behalf.agent() == Some(self.0) {
                    Ok(ForwardPermit::new(()))
                } else {
                    Err(OverBudget {
                        retry_after_ms: 1000,
                        retry_at_epoch_secs: 1_700_000_001,
                        rule: "per-requester".into(),
                    })
                }
            }
        }
        let mut laptop = CountingStorage::new().with_id(TierId::agent("uhCAklaptop"));
        let (root, _) = ten_thousand(&mut laptop, false);
        let laptop = Arc::new(laptop);
        let laptop_id = TierId::agent("uhCAklaptop");
        let records = vec![household_record(&laptop_id)];
        let connect = Connect(
            [(laptop_id.clone(), laptop.clone() as Arc<dyn EprfsStorage>)]
                .into_iter()
                .collect(),
        );
        let c = TierChain::new(
            Arc::new(CountingStorage::new()),
            records,
            Arc::new(ProximityOrder),
            &connect,
            Arc::new(AllowAll),
            Arc::new(OnlyFor("uhCAkmatthew")),
            Requester::authenticated("uhCAkhub", KeyCustody::HubHeld),
        );
        let fetch = opts(FetchPolicy::FetchIfMissing);
        let limits = TreeLimits::default();
        // The chain's own requester is past the gate's budget for this requester rule.
        assert!(
            crate::list_dir(&c, &root, &TreePath::root(), &fetch, &limits)
                .await
                .is_err()
        );
        let matthew = Requester::authenticated("uhCAkmatthew", KeyCustody::DeviceHeld);
        let bound = BatchOptions {
            via: vec![laptop_id.clone()],
            ..fetch.clone()
        };
        let listing = crate::list_dir(
            &c.on_behalf(&matthew, &bound),
            &root,
            &TreePath::root(),
            &fetch,
            &limits,
        )
        .await
        .unwrap();
        assert_eq!(listing.entries.len(), 10);
        // An empty `via` dials no tier at all (the root is now an own hit; `a1` is not held).
        laptop.reset();
        let unbound = c.on_behalf(&matthew, &BatchOptions::default());
        assert!(crate::list_dir(&unbound, &root, &tp("a1"), &fetch, &limits)
            .await
            .is_err());
        assert!(laptop.fetched().is_empty());
    }

    /// (38) A tier that floods unrequested and repeated answers is cut off once it has sent more
    /// unwanted answers than it was asked for: nothing unrequested is stored or delivered, what
    /// it gave verifiably stands, and what it never reached is `Unreachable { it }`. A tier that
    /// has answered everything is told to stop at once.
    #[tokio::test]
    async fn a_flooding_outer_tier_is_cut_off_and_nothing_unrequested_is_kept() {
        use crate::test_support::Flood;
        let flood_id = TierId::agent("uhCAkflood");
        let (flood, cids) = Flood::new(flood_id.clone(), &[b"one", b"two", b"three"], 1, 1);
        let flood = Arc::new(flood);
        let c = chain(
            CountingStorage::new(),
            vec![(flood_id.clone(), flood.clone() as Arc<dyn EprfsStorage>)],
        );
        let root = BlobCid::compute(b"root");
        let items: Vec<BatchItem> = cids
            .iter()
            .zip(["a", "b", "c"])
            .map(|(cid, path)| BatchItem::new(cid.clone(), tp(path)))
            .collect();
        let mut sink = CollectSink::default();
        c.fetch_blobs(&root, &items, &opts(FetchPolicy::FetchIfMissing), &mut sink)
            .await
            .unwrap();
        // junk, one, one again, junk, two, two again: the sixth call overran the batch of three.
        assert!(flood.saw_stop(), "the flooding tier was never told to stop");
        assert_eq!(flood.calls(), 6);
        assert_eq!(sink.answers.len(), 3, "one answer per item");
        for (item, answer) in &sink.answers[..2] {
            match answer {
                BatchAnswer::Arrived { bytes, from } => {
                    assert!(item.cid.verifies(bytes));
                    assert_eq!(from, &flood_id);
                }
                other => panic!("expected {item:?} to arrive, got {other:?}"),
            }
        }
        assert_eq!(sink.answers[2].0.cid, cids[2]);
        assert!(matches!(
            &sink.answers[2].1,
            BatchAnswer::Absent(u) if u.why == ItemAbsence::Unreachable { tier: flood_id.clone() }
        ));
        for (cid, kept) in cids.iter().zip([true, true, false]) {
            assert_eq!(
                c.own().has_blob(cid).await.unwrap() == BlobPresence::Local,
                kept,
                "write-back of {cid}"
            );
        }

        // Answered in full, the tier is stopped before its repeats.
        let (flood, cids) = Flood::new(flood_id.clone(), &[b"only"], 0, 3);
        let flood = Arc::new(flood);
        let c = chain(
            CountingStorage::new(),
            vec![(flood_id.clone(), flood.clone() as Arc<dyn EprfsStorage>)],
        );
        let mut sink = CollectSink::default();
        c.fetch_blobs(
            &root,
            &[BatchItem::new(cids[0].clone(), tp("a"))],
            &opts(FetchPolicy::FetchIfMissing),
            &mut sink,
        )
        .await
        .unwrap();
        assert_eq!(flood.calls(), 1);
        assert_eq!(sink.answers.len(), 1);
    }

    /// (39) The caller's `Stop` reaches the outer tier on the arrival that prompted it: the chain
    /// streams, so the tier sends nothing more, and what was delivered was written back.
    #[tokio::test]
    async fn the_callers_stop_reaches_the_outer_tier_at_once() {
        use crate::test_support::{Flood, StopAfter};
        let flood_id = TierId::agent("uhCAkstreaming");
        let (tier, cids) = Flood::new(flood_id.clone(), &[b"one", b"two", b"three"], 0, 0);
        let tier = Arc::new(tier);
        let c = chain(
            CountingStorage::new(),
            vec![(flood_id.clone(), tier.clone() as Arc<dyn EprfsStorage>)],
        );
        let items: Vec<BatchItem> = cids
            .iter()
            .zip(["a", "b", "c"])
            .map(|(cid, path)| BatchItem::new(cid.clone(), tp(path)))
            .collect();
        let mut sink = StopAfter {
            after: 1,
            inner: CollectSink::default(),
        };
        let report = c
            .fetch_blobs(
                &BlobCid::compute(b"root"),
                &items,
                &opts(FetchPolicy::FetchIfMissing),
                &mut sink,
            )
            .await
            .unwrap();
        assert!(report.stopped);
        assert_eq!(
            tier.calls(),
            1,
            "the tier kept sending after the caller's Stop"
        );
        assert!(tier.saw_stop());
        assert_eq!(sink.inner.answers.len(), 1);
        assert_eq!(
            c.own().has_blob(&cids[0]).await.unwrap(),
            BlobPresence::Local,
            "the delivered object was written back"
        );
        assert_eq!(
            c.own().has_blob(&cids[1]).await.unwrap(),
            BlobPresence::Missing
        );
    }
}
