//! The steward lease view: whether a lease on a steward-scoped resource holds, read from this
//! device's own berth records, those folded from its other devices, and their acknowledgements
//! (spec §3.4).
//!
//! A lease is replayed from berth's `dev-berth-events@1` records (T1), each device's in the order
//! that device wrote them: by the record counter berth stamps on each (`dseq`; older records by
//! record time, first) — since a fold appends a head's events in its tree's (CID) order, and two
//! berth processes on one device can reach its store in the other order from the one they
//! recorded in: a `claim` opens it, a
//! `renew` extends it, a `release` (its own, or a forced one naming it) and an `unmoor` close it,
//! and a claim that took over from another session closes that session's lease. A lease with a
//! ttl stops binding `ttl_s` after it was claimed or last renewed. Leases folded from a device the
//! roster no longer admits are ignored entirely (spec §3.1): a revoked device cannot keep a
//! resource pinned, and it is not a device to wait for.
//!
//! Per resource, this device's own live lease is held unless another device's live lease
//! precedes it. A claim *saw* another lease when it was written with that lease already known:
//! this device knows that exactly for its own claims (the other's claim sits earlier in its own
//! store), and for another device's claim from the acknowledgement vector the claim carries
//! (`seen`: the highest head of each device its writer had folded, so a claim whose `seen` reaches
//! the head that carried ours was written after its writer folded ours). A lease another claim saw
//! precedes it. Two claims neither saw are concurrent, and the earlier `claimed_at`, then the
//! lexically lower device `did:key`, then the session, wins — values each claim's own device
//! stamped, so both sides of a concurrent pair compare the same values. When the two sides know
//! different things (a `seen` read before its writer's last fold landed), the side that knows less
//! can only treat the pair as concurrent, so both may yield; they never both hold. berth refuses a
//! claim at once, writing nothing, when the view already shows another device's lease, so a
//! written pair overlaps only when neither had folded the other's.
//!
//! Another device's lease that is not preceded is [`LeaseVerdict::HeldBy`], whether it is pending
//! on its device or not: a pending lease counts as live, and an absent device is never free.
//!
//! This device's lease is [`LeaseVerdict::Effective`] once every other admitted device has
//! acknowledged the first own head that carried its claim and none nacks its claim CID; until then
//! it is [`LeaseVerdict::Pending`], and [`LeaseVerdict::Unconfirmed`] once `confirm_wait_s` has passed
//! since the claim was written. The head that carried a claim is read from
//! [`HeadTable::own_log`] by store position, not by time: a claim written as the store's `n`th
//! flow event is in the first own head projected from at least `n` events — counted over this
//! same store, which each entry proves by naming the CID of the last event it counted. (A head
//! announced after a claim's stamp can still have been projected before the claim reached the
//! store; a time comparison would take that head's acknowledgement for the claim's.) A claim older
//! than the oldest logged head takes that head, conservatively. Even after confirmation, a bounded
//! lease remains effective only while every other admitted device has supplied a head within
//! `ttl_s - fence_margin_s` and that head acknowledges the first own head carrying the latest
//! renew. A no-ttl daemon lease has no time fence, but its latest renew still needs coverage.
//!
//! An offline-ok override, berth's `override` record naming a silent device, becomes effective only
//! `ttl_s + max_skew_s + fence_margin_s` after this device last folded that device (or after the
//! claim when it never did). The named silent device is the override's one exception to confirmation
//! and the holder fence; every other admitted device must still cover the latest renew and remain
//! fresh. A no-ttl lease can never become effective by override. On other devices an override is an
//! ordinary lease and binds them.
//!
//! Clocks: a ttl, confirmation, holder fence and override wait compare times. Own waits and folds use
//! this device's clock. A folded lease's ttl counts from its holder's stamp but is read against this
//! device's clock; the override wait therefore budgets the rulebook's explicit `max_skew_s` before a
//! second holder can become effective. The silent holder self-demotes by `TTL - margin` after last
//! hearing the claimant; the claimant waits `TTL + skew + margin` after last hearing the holder, so
//! at most one is effective during a partition, not merely after it.

use std::collections::{BTreeMap, BTreeSet};

use cid::Cid;
use elohim_epr_rea::{FlowEvent, FlowStore};
use serde::Serialize;

use crate::admission::Admission;
use crate::allow::{single_slot, slots, via_device, KIND_SLOT, MEASURE_SLOT};
use crate::error::PeerError;
use crate::head_table::{HeadTable, OwnAnnounce};
use crate::steward_scope::Settlement;

/// The measure berth records are folded under (T1).
pub const BERTH_EVENTS_MEASURE: &str = "dev-berth-events@1";

/// The prefix of a berth record's own fields in `classifiedAs`: `env:<k>=<v>`.
const ENV_SLOT: &str = "env:";

/// A berth resource name: `dev-integration`, `lane:<id>`, ….
pub type Resource = String;

/// The berth record kinds that open, extend, mark or close a lease, and the mooring that a
/// session's leases live under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseKind {
    /// A session moored (or refreshed its mooring); it names no resource.
    Moor,
    Claim,
    Renew,
    Release,
    Override,
    Unmoor,
}

impl LeaseKind {
    /// Within one millisecond of one device's records, the order they can only have been
    /// written in: a session moors before it claims, and a lease is claimed before it is renewed,
    /// overridden or released.
    fn order(self) -> u8 {
        match self {
            LeaseKind::Moor => 0,
            LeaseKind::Claim => 1,
            LeaseKind::Renew => 2,
            LeaseKind::Override => 3,
            LeaseKind::Release => 4,
            LeaseKind::Unmoor => 5,
        }
    }
}

/// One lease-bearing berth record, read from a flow event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseEvent {
    /// The record CID in this store. For an own claim this is what peers nack.
    pub cid: Cid,
    pub kind: LeaseKind,
    /// The device it was folded from (`via:<did>`); `None` for this device's own.
    pub via: Option<String>,
    /// `None` only for a `moor`, and for an `unmoor`, which closes every lease of its session.
    pub resource: Option<Resource>,
    pub session: String,
    /// When the record was written, its device's clock, unix milliseconds.
    pub at: u64,
    /// Its device's record counter (`env:dseq`): strictly increasing per device, never reused,
    /// so it orders a device's records even across a step of its clock. `None` for records
    /// written before berth counted them.
    pub dseq: Option<u64>,
    /// The lease's claim time as its device stamped it, unix milliseconds.
    pub claimed_at: Option<u64>,
    pub ttl_s: Option<u64>,
    /// Its 1-based position among the flow events of the store it was read from.
    pub written: u64,
    /// A claim that took the resource over from a lapsed session names it.
    pub took_over_from: Option<String>,
    /// A forced release names the holders it gave the resource back for.
    pub forced_from: Vec<String>,
    /// An offline-ok override names the silent device it overrode.
    pub overrode: Option<String>,
    /// A claim's acknowledgement vector when it was written: for each other device, the highest
    /// of its heads the claimant had folded (`env:seen`, compact JSON).
    pub seen: BTreeMap<String, u64>,
    /// This mooring's author understands the nack leaf protocol.
    pub reads_nacks: bool,
}

impl LeaseEvent {
    /// The lease record `event` is, read as the `written`th flow event of its store; `None` when
    /// it is not a berth record of a lease-bearing kind or lacks a field the kind needs.
    pub fn from_event(event: &FlowEvent, written: u64) -> Option<LeaseEvent> {
        if single_slot(event, MEASURE_SLOT)? != BERTH_EVENTS_MEASURE {
            return None;
        }
        let kind = match single_slot(event, KIND_SLOT)? {
            "moor" => LeaseKind::Moor,
            "claim" => LeaseKind::Claim,
            "renew" => LeaseKind::Renew,
            "release" => LeaseKind::Release,
            "override" => LeaseKind::Override,
            "unmoor" => LeaseKind::Unmoor,
            _ => return None,
        };
        let mut env = BTreeMap::new();
        for slot in slots(event) {
            if let Some((key, value)) = slot
                .strip_prefix(ENV_SLOT)
                .and_then(|field| field.split_once('='))
            {
                env.entry(key).or_insert(value);
            }
        }
        let resource = env.get("resource").map(|r| (*r).to_string());
        if resource.is_none() && !matches!(kind, LeaseKind::Moor | LeaseKind::Unmoor) {
            return None;
        }
        let names = |key: &str| {
            env.get(key)
                .map(|list| {
                    list.split(',')
                        .filter(|name| !name.is_empty())
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        };
        Some(LeaseEvent {
            cid: atom_cid_without_via(event)?,
            kind,
            via: via_device(event).map(str::to_string),
            resource,
            session: (*env.get("session")?).to_string(),
            at: unix_ms(env.get("at")?)?,
            dseq: env.get("dseq").and_then(|n| n.parse().ok()),
            claimed_at: env.get("claimed_at").and_then(|at| unix_ms(at)),
            ttl_s: env.get("ttl_s").and_then(|ttl| ttl.parse().ok()),
            written,
            took_over_from: env.get("took_over_from").map(|s| (*s).to_string()),
            forced_from: names("forced_from"),
            overrode: env.get("overrode").map(|s| (*s).to_string()),
            seen: env
                .get("seen")
                .and_then(|seen| serde_json::from_str(seen).ok())
                .unwrap_or_default(),
            reads_nacks: env.get("reads").is_some_and(|reads| *reads == "nacks/1"),
        })
    }
}

fn atom_cid_without_via(event: &FlowEvent) -> Option<Cid> {
    let mut original = event.clone();
    original
        .classified_as
        .retain(|slot| !slot.starts_with(crate::allow::VIA_SLOT));
    elohim_epr_rea::atom_cid(&original).ok()
}

/// Unix seconds as berth writes them (`1792769518.961`) in milliseconds.
fn unix_ms(seconds: &str) -> Option<u64> {
    let seconds: f64 = seconds.parse().ok()?;
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    // Rounded, never truncated: both devices read the same string to the same millisecond.
    Some((seconds * 1000.0).round() as u64)
}

/// A device's channel store as the lease view reads it: its lease records, split into this
/// device's own and those folded from others, each with its position among the store's flow
/// events, and every flow event's CID in store order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LeaseStore {
    pub own: Vec<LeaseEvent>,
    pub folded: Vec<LeaseEvent>,
    pub events: Vec<Cid>,
}

impl LeaseStore {
    /// A store's flow events, in append order.
    pub fn from_events(events: impl IntoIterator<Item = (Cid, FlowEvent)>) -> LeaseStore {
        let mut store = LeaseStore::default();
        for (written, (cid, event)) in (1u64..).zip(events) {
            store.events.push(cid);
            if let Some(lease) = LeaseEvent::from_event(&event, written) {
                if lease.via.is_some() {
                    store.folded.push(lease);
                } else {
                    store.own.push(lease);
                }
            }
        }
        store
    }

    /// Whether `logged` counted this store: the store holds the event it names at the position
    /// it counted to.
    fn counted(&self, logged: &OwnAnnounce) -> bool {
        match (logged.events, &logged.last) {
            (0, _) => true,
            (events, Some(last)) => usize::try_from(events - 1)
                .ok()
                .and_then(|i| self.events.get(i))
                .is_some_and(|cid| cid.to_string() == *last),
            (_, None) => false,
        }
    }
}

/// `store`'s lease records and event CIDs.
///
/// # Errors
///
/// [`PeerError::FlowStore`] when the store cannot be read.
pub fn lease_events<S>(store: &S) -> Result<LeaseStore, PeerError>
where
    S: FlowStore + ?Sized,
{
    Ok(LeaseStore::from_events(
        store.events().map_err(PeerError::FlowStore)?,
    ))
}

/// Where a lease on a steward-scoped resource stands for this device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum LeaseVerdict {
    /// This device's `session` holds it, and every other admitted device has seen the claim.
    Effective { session: String },
    /// This device's `session` claimed it; `waiting_on` have not acknowledged the claim yet.
    Pending {
        session: String,
        waiting_on: Vec<String>,
    },
    /// This device's `session` claimed it and `device` did not acknowledge within the wait.
    /// `last_seen` is when this device last folded a head from it (its clock, unix ms).
    Unconfirmed {
        session: String,
        device: String,
        last_seen: Option<u64>,
        /// True when a formerly confirmed holder demoted itself at the holder fence.
        fenced: bool,
        /// When an offline override becomes eligible, unix milliseconds.
        override_at: Option<u64>,
    },
    /// A peer acknowledged the carrying head but explicitly refused this claim CID.
    RefusedBy { session: String, device: String },
    /// Another device's lease wins: `device`/`session`, claimed at `since` (that device's clock,
    /// unix ms); `override` when it was taken over a silent device with offline-ok.
    HeldBy {
        device: String,
        session: String,
        since: u64,
        #[serde(rename = "override")]
        overriding: bool,
    },
}

/// One live lease.
#[derive(Debug, Clone)]
struct Lease {
    device: String,
    session: String,
    claimed_at: u64,
    /// Claimed or last renewed; a ttl counts from here.
    since: u64,
    ttl_s: Option<u64>,
    /// The claim record's position in its store and its write time.
    written: u64,
    /// The latest renew record's position, or the claim position before the first renew.
    renewed_written: u64,
    claim_at: u64,
    overrode: Option<String>,
    seen: BTreeMap<String, u64>,
    claim_cid: Cid,
}

impl Lease {
    fn live(&self, now: u64) -> bool {
        self.ttl_s
            .is_none_or(|ttl| self.since.saturating_add(ttl.saturating_mul(1000)) > now)
    }

    /// The tie-break: earlier claim, then lower device did:key, then session.
    fn precedence(&self) -> (u64, &str, &str) {
        (self.claimed_at, &self.device, &self.session)
    }
}

/// Replay one device's lease records, in the order it wrote them, into its open leases by
/// `(resource, session)`.
fn replay<'e>(
    device: &str,
    events: impl IntoIterator<Item = &'e LeaseEvent>,
) -> BTreeMap<(Resource, String), Lease> {
    let mut open: BTreeMap<(Resource, String), Lease> = BTreeMap::new();
    for event in events {
        let Some(resource) = event.resource.clone() else {
            if event.kind == LeaseKind::Unmoor {
                open.retain(|(_, session), _| *session != event.session);
            }
            continue;
        };
        let key = (resource.clone(), event.session.clone());
        let opened = |event: &LeaseEvent| {
            let claimed_at = event.claimed_at.unwrap_or(event.at);
            Lease {
                device: device.to_string(),
                session: event.session.clone(),
                claimed_at,
                since: if event.kind == LeaseKind::Claim {
                    claimed_at
                } else {
                    event.at
                },
                ttl_s: event.ttl_s,
                written: event.written,
                renewed_written: event.written,
                claim_at: event.at,
                overrode: None,
                seen: event.seen.clone(),
                claim_cid: event.cid,
            }
        };
        match event.kind {
            LeaseKind::Claim => {
                if let Some(prior) = &event.took_over_from {
                    open.remove(&(resource, prior.clone()));
                }
                open.insert(key, opened(event));
            }
            LeaseKind::Renew => match open.get_mut(&key) {
                Some(lease) => {
                    lease.since = event.at;
                    lease.ttl_s = event.ttl_s;
                    lease.renewed_written = event.written;
                }
                None => {
                    open.insert(key, opened(event));
                }
            },
            LeaseKind::Release => {
                open.remove(&key);
                for holder in &event.forced_from {
                    open.remove(&(resource.clone(), holder.clone()));
                }
            }
            LeaseKind::Moor => {}
            LeaseKind::Override => {
                if let Some(lease) = open.get_mut(&key) {
                    if event.overrode.is_some()
                        && event.claimed_at.is_none_or(|at| at == lease.claimed_at)
                    {
                        lease.overrode.clone_from(&event.overrode);
                    }
                }
            }
            LeaseKind::Unmoor => {
                open.retain(|(_, session), _| *session != event.session);
            }
        }
    }
    open
}

/// The steward lease view on device `me`: for every resource some live lease names, its verdict.
///
/// `store` is this device's channel store (its own lease records, those folded from other
/// devices, and its event CIDs); `heads` is this device's head table; `admitted` its roster's
/// admission now. `now` is this device's clock in unix milliseconds. Pure: nothing is asked over
/// the network, and the same inputs give the same view.
///
/// # Errors
///
/// [`PeerError::NotAdmitted`] when `admitted` does not admit `me`: a device with no standing on
/// the channel has no view of it.
pub fn steward_lease_view(
    me: &str,
    store: &LeaseStore,
    heads: &HeadTable,
    admitted: &Admission,
    now: u64,
    settlement: Settlement,
) -> Result<BTreeMap<Resource, LeaseVerdict>, PeerError> {
    if !admitted.admits_device(me) {
        return Err(PeerError::NotAdmitted {
            device: me.to_string(),
        });
    }
    let others: BTreeSet<&str> = admitted
        .devices()
        .into_iter()
        .filter(|device| *device != me)
        .collect();
    let live = |leases: BTreeMap<(Resource, String), Lease>| {
        leases
            .into_iter()
            .filter(|(_, lease)| lease.live(now))
            .map(|((resource, _), lease)| (resource, lease))
            .collect::<Vec<_>>()
    };
    // This device's own records replay by its record counter too: two berth processes can flush
    // their folds in the other order from the one they recorded in (a release can reach the store
    // before the claim it closes). Each lease keeps its store position (`written`) for `carried`.
    let mut mine: Vec<&LeaseEvent> = store
        .own
        .iter()
        .filter(|event| event.via.is_none())
        .collect();
    mine.sort_by_key(|event| device_order(event));
    let mut own: BTreeMap<Resource, Lease> = BTreeMap::new();
    for (resource, lease) in live(replay(me, mine)) {
        match own.get(&resource) {
            Some(held) if held.precedence() <= lease.precedence() => {}
            _ => {
                own.insert(resource, lease);
            }
        }
    }
    let mut by_device: BTreeMap<&str, Vec<&LeaseEvent>> = BTreeMap::new();
    for event in &store.folded {
        // A contested or unknown device's leases are ignored entirely.
        if let Some(device) = event.via.as_deref().filter(|d| others.contains(d)) {
            by_device.entry(device).or_default().push(event);
        }
    }
    let mut folded: BTreeMap<Resource, Vec<Lease>> = BTreeMap::new();
    for (device, mut events) in by_device {
        events.sort_by_key(|event| device_order(event));
        for (resource, lease) in live(replay(device, events)) {
            folded.entry(resource).or_default().push(lease);
        }
    }

    let usable: Vec<&OwnAnnounce> = heads
        .own_log
        .iter()
        .filter(|logged| store.counted(logged))
        .collect();
    let mut view = BTreeMap::new();
    for resource in own
        .keys()
        .chain(folded.keys())
        .cloned()
        .collect::<BTreeSet<_>>()
    {
        let theirs = folded.remove(&resource).unwrap_or_default();
        let verdict = match own.remove(&resource) {
            Some(lease) => {
                // The first own head that carried the claim, over this store.
                let carried = usable
                    .iter()
                    .filter(|logged| logged.events >= lease.written)
                    .map(|logged| logged.seq)
                    .min();
                // The first own head that carried the claim or its latest renew.
                let renewed_carried = usable
                    .iter()
                    .filter(|logged| logged.events >= lease.renewed_written)
                    .map(|logged| logged.seq)
                    .min();
                let preceding = theirs
                    .into_iter()
                    .filter(|other| precedes(other, &lease, carried, me))
                    .min_by(|x, y| x.precedence().cmp(&y.precedence()));
                match preceding {
                    Some(other) => held_by(other),
                    None => own_verdict(
                        lease,
                        Carried {
                            claim: carried,
                            renew: renewed_carried,
                        },
                        me,
                        &others,
                        heads,
                        now,
                        settlement,
                    ),
                }
            }
            None => match theirs
                .into_iter()
                .min_by(|x, y| x.precedence().cmp(&y.precedence()))
            {
                Some(other) => held_by(other),
                None => continue,
            },
        };
        view.insert(resource, verdict);
    }
    Ok(view)
}

/// The order a device wrote its records in. A fold appends a head's events in its channel
/// tree's order (by CID), not the order its device wrote them: replay them by the device's record
/// counter, which a clock step cannot reorder; records from before the counter by record time,
/// first.
fn device_order(event: &LeaseEvent) -> (bool, Option<u64>, u64, u8, u64) {
    (
        event.dseq.is_some(),
        event.dseq,
        event.at,
        event.kind.order(),
        event.written,
    )
}

/// Another admitted device as this device knows it from its own fold: the last head folded from
/// it and the sessions moored there.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OtherDevice {
    /// The seq of the last head folded from it; `None` when none was.
    pub seq: Option<u64>,
    /// When that head was folded, this device's clock, unix milliseconds.
    pub last_seen: Option<u64>,
    /// Its sessions moored and not unmoored, each with its latest moor (its clock, unix ms).
    pub moorings: BTreeMap<String, u64>,
}

/// Every other device `admitted` admits now, read from this device's fold (`store`, `heads`): its
/// last head and its open moorings, replayed in the order that device wrote them, as the lease view
/// replays its leases. A contested device is not listed. Pure, like [`steward_lease_view`].
///
/// # Errors
///
/// [`PeerError::NotAdmitted`] when `admitted` does not admit `me`.
pub fn other_devices(
    me: &str,
    store: &LeaseStore,
    heads: &HeadTable,
    admitted: &Admission,
) -> Result<BTreeMap<String, OtherDevice>, PeerError> {
    if !admitted.admits_device(me) {
        return Err(PeerError::NotAdmitted {
            device: me.to_string(),
        });
    }
    let mut others: BTreeMap<String, OtherDevice> = admitted
        .devices()
        .into_iter()
        .filter(|device| *device != me)
        .map(|device| {
            let head = heads.devices.get(device);
            let known = OtherDevice {
                seq: head.map(|head| head.seq),
                last_seen: head.map(|head| head.folded_at),
                moorings: BTreeMap::new(),
            };
            (device.to_string(), known)
        })
        .collect();
    let mut by_device: BTreeMap<&str, Vec<&LeaseEvent>> = BTreeMap::new();
    for event in &store.folded {
        if let Some(device) = event.via.as_deref().filter(|d| others.contains_key(*d)) {
            by_device.entry(device).or_default().push(event);
        }
    }
    for (device, mut events) in by_device {
        events.sort_by_key(|event| device_order(event));
        let moorings = &mut others.entry(device.to_string()).or_default().moorings;
        for event in events {
            match event.kind {
                LeaseKind::Moor => {
                    // The latest moor: a re-moor refreshes it, so its age says whether it is live.
                    moorings.insert(event.session.clone(), event.at);
                }
                LeaseKind::Unmoor => {
                    moorings.remove(&event.session);
                }
                _ => {}
            }
        }
    }
    Ok(others)
}

/// Whether `other`'s lease (folded from another device) precedes `mine`, whose claim was first
/// carried by own head `carried`: `mine` was written after `other` reached this store, or neither
/// claim saw the other and `other` wins the tie-break. When `other` saw `mine` (its `seen` reaches
/// `carried`), it never precedes, whatever the clocks say.
fn precedes(other: &Lease, mine: &Lease, carried: Option<u64>, me: &str) -> bool {
    let mine_saw_other = other.written < mine.written;
    let other_saw_mine = carried.is_some_and(|seq| other.seen.get(me).is_some_and(|&k| k >= seq));
    mine_saw_other || (!other_saw_mine && other.precedence() < mine.precedence())
}

fn held_by(lease: Lease) -> LeaseVerdict {
    LeaseVerdict::HeldBy {
        device: lease.device,
        session: lease.session,
        since: lease.claimed_at,
        overriding: lease.overrode.is_some(),
    }
}

/// The first own heads that carried a lease's claim and its latest renew (the claim's, before any
/// renew), over this store.
#[derive(Debug, Clone, Copy)]
struct Carried {
    claim: Option<u64>,
    renew: Option<u64>,
}

/// This device's lease, preceded by no other: effective once every other admitted device
/// acknowledged the head that carried its claim, and its holder fence holds.
fn own_verdict(
    lease: Lease,
    carried: Carried,
    me: &str,
    others: &BTreeSet<&str>,
    heads: &HeadTable,
    now: u64,
    settlement: Settlement,
) -> LeaseVerdict {
    let claim_cid = lease.claim_cid.to_string();
    if let Some(device) = others.iter().find(|device| {
        heads
            .devices
            .get(**device)
            .is_some_and(|head| head.nack.contains(&claim_cid))
    }) {
        return LeaseVerdict::RefusedBy {
            session: lease.session,
            device: (*device).to_string(),
        };
    }
    if others.is_empty() {
        return LeaseVerdict::Effective {
            session: lease.session,
        };
    }
    if let Some(device) = others.iter().find(|device| {
        heads
            .devices
            .get(**device)
            .is_some_and(|head| head.nack.overflowed())
    }) {
        return LeaseVerdict::Unconfirmed {
            session: lease.session,
            last_seen: heads.devices.get(*device).map(|head| head.folded_at),
            device: (*device).to_string(),
            fenced: false,
            override_at: None,
        };
    }
    let override_at = lease.overrode.as_ref().and_then(|silent| {
        lease.ttl_s.map(|ttl| {
            let last_seen = heads
                .devices
                .get(silent)
                .map_or(lease.claim_at, |head| head.folded_at);
            let wait_s = ttl
                .saturating_add(settlement.max_skew_s)
                .saturating_add(settlement.fence_margin_s);
            last_seen.saturating_add(wait_s.saturating_mul(1000))
        })
    });
    let override_effective = override_at.is_some_and(|at| now >= at);
    if let Some(silent) = lease.overrode.clone() {
        if !override_effective {
            return LeaseVerdict::Unconfirmed {
                session: lease.session,
                last_seen: heads.devices.get(&silent).map(|head| head.folded_at),
                device: silent,
                fenced: false,
                override_at,
            };
        }
    }
    let waiting_on: Vec<String> = others
        .iter()
        .filter(|device| lease.overrode.as_deref() != Some(**device) || !override_effective)
        .filter(|device| {
            let head = heads.devices.get(**device);
            let acked = head.and_then(|head| head.folded.get(me));
            !matches!((carried.claim, acked), (Some(seq), Some(&acked)) if acked >= seq)
        })
        .map(|device| (*device).to_string())
        .collect();
    let Some(silent) = waiting_on.first().cloned() else {
        let fence_after_s = lease
            .ttl_s
            .map(|ttl| ttl.saturating_sub(settlement.fence_margin_s));
        let stale = others.iter().find(|device| {
            if lease.overrode.as_deref() == Some(**device) && override_effective {
                return false;
            }
            let Some(head) = heads.devices.get(**device) else {
                return true;
            };
            let fresh = fence_after_s.is_none_or(|window| {
                now.saturating_sub(head.folded_at) <= window.saturating_mul(1000)
            });
            let covered = matches!(
                (carried.renew, head.folded.get(me)),
                (Some(seq), Some(&acked)) if acked >= seq
            );
            !fresh || !covered
        });
        return match stale {
            Some(device) => LeaseVerdict::Unconfirmed {
                session: lease.session,
                last_seen: heads.devices.get(*device).map(|head| head.folded_at),
                device: (*device).to_string(),
                fenced: true,
                override_at: None,
            },
            None => LeaseVerdict::Effective {
                session: lease.session,
            },
        };
    };
    let deadline = lease
        .claim_at
        .saturating_add(settlement.confirm_wait_s.saturating_mul(1000));
    if now < deadline {
        LeaseVerdict::Pending {
            session: lease.session,
            waiting_on,
        }
    } else {
        LeaseVerdict::Unconfirmed {
            session: lease.session,
            last_seen: heads.devices.get(&silent).map(|head| head.folded_at),
            device: silent,
            fenced: false,
            override_at: None,
        }
    }
}
