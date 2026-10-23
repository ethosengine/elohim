//! The steward lease view: whether a lease on a steward-scoped resource holds, read from this
//! device's own berth records, those folded from its other devices, and their acknowledgements
//! (spec §3.4).
//!
//! A lease is replayed from berth's `dev-berth-events@1` records (T1): a `claim` opens it, a
//! `renew` extends it, a `release` (its own, or a forced one naming it) and an `unmoor` close it,
//! and a claim that took over from another session closes that session's lease. A lease with a
//! ttl stops binding `ttl_s` after it was claimed or last renewed. Leases folded from a device the
//! roster no longer admits are ignored entirely (spec §3.1): a revoked device cannot keep a
//! resource pinned, and it is not a device to wait for.
//!
//! Per resource, the live leases of every holder compete and one wins: the earlier `claimed_at`,
//! then the lexically lower device `did:key`, then the session. Each device compares the same
//! recorded values, each stamped by its own device, so both sides of a concurrent pair pick the
//! same winner. Another device's winning lease is [`LeaseVerdict::HeldBy`] whether it is pending
//! on its device or not: a pending lease counts as live, and an absent device is never free.
//!
//! This device's winning lease is [`LeaseVerdict::Effective`] once every other admitted device
//! has acknowledged the first own head that carried its claim; until then it is
//! [`LeaseVerdict::Pending`], and [`LeaseVerdict::Unconfirmed`] once `confirm_wait_s` has passed
//! since the claim was written. The head that carried a claim is read from
//! [`HeadTable::own_log`] by position, not by time: a claim written as the store's `n`th flow
//! event is in the first own head projected from at least `n` events. (A head announced after a
//! claim's stamp can still have been projected before the claim reached the store; a time
//! comparison would take that head's acknowledgement for the claim's.) An offline-ok override,
//! berth's `override` record naming the device it overrode, makes its holder's lease effective at
//! once; on the other devices it is an ordinary lease and binds them.

use std::collections::{BTreeMap, BTreeSet};

use elohim_epr_rea::{FlowEvent, FlowStore};
use serde::Serialize;

use crate::admission::Admission;
use crate::allow::{single_slot, slots, via_device, KIND_SLOT, MEASURE_SLOT};
use crate::error::PeerError;
use crate::head_table::HeadTable;

/// The measure berth records are folded under (T1).
pub const BERTH_EVENTS_MEASURE: &str = "dev-berth-events@1";

/// The prefix of a berth record's own fields in `classifiedAs`: `env:<k>=<v>`.
const ENV_SLOT: &str = "env:";

/// A berth resource name: `dev-integration`, `lane:<id>`, ….
pub type Resource = String;

/// The berth record kinds that open, extend, mark or close a lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseKind {
    Claim,
    Renew,
    Release,
    Override,
    Unmoor,
}

/// One lease-bearing berth record, read from a flow event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseEvent {
    pub kind: LeaseKind,
    /// The device it was folded from (`via:<did>`); `None` for this device's own.
    pub via: Option<String>,
    /// `None` only for an `unmoor`, which closes every lease of its session.
    pub resource: Option<Resource>,
    pub session: String,
    /// When the record was written, its device's clock, unix milliseconds.
    pub at: u64,
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
}

impl LeaseEvent {
    /// The lease record `event` is, read as the `written`th flow event of its store; `None` when
    /// it is not a berth record of a lease-bearing kind or lacks a field the kind needs.
    pub fn from_event(event: &FlowEvent, written: u64) -> Option<LeaseEvent> {
        if single_slot(event, MEASURE_SLOT)? != BERTH_EVENTS_MEASURE {
            return None;
        }
        let kind = match single_slot(event, KIND_SLOT)? {
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
        if resource.is_none() && kind != LeaseKind::Unmoor {
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
            kind,
            via: via_device(event).map(str::to_string),
            resource,
            session: (*env.get("session")?).to_string(),
            at: unix_ms(env.get("at")?)?,
            claimed_at: env.get("claimed_at").and_then(|at| unix_ms(at)),
            ttl_s: env.get("ttl_s").and_then(|ttl| ttl.parse().ok()),
            written,
            took_over_from: env.get("took_over_from").map(|s| (*s).to_string()),
            forced_from: names("forced_from"),
            overrode: env.get("overrode").map(|s| (*s).to_string()),
        })
    }
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

/// The lease records of `store`, split into this device's own and those folded from others, each
/// with its position among the store's flow events.
///
/// # Errors
///
/// [`PeerError::FlowStore`] when the store cannot be read.
pub fn lease_events<S>(store: &S) -> Result<(Vec<LeaseEvent>, Vec<LeaseEvent>), PeerError>
where
    S: FlowStore + ?Sized,
{
    let mut own = Vec::new();
    let mut folded = Vec::new();
    for (written, (_, event)) in (1u64..).zip(store.events().map_err(PeerError::FlowStore)?) {
        if let Some(lease) = LeaseEvent::from_event(&event, written) {
            if lease.via.is_some() {
                folded.push(lease);
            } else {
                own.push(lease);
            }
        }
    }
    Ok((own, folded))
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
    },
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
    claim_at: u64,
    overriding: bool,
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

/// Replay one device's lease records, in store order, into its open leases by
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
                claim_at: event.at,
                overriding: false,
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
            LeaseKind::Override => {
                if let Some(lease) = open.get_mut(&key) {
                    if event.overrode.is_some()
                        && event.claimed_at.is_none_or(|at| at == lease.claimed_at)
                    {
                        lease.overriding = true;
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
/// `own` are this device's lease records and `folded` those folded from other devices (each with
/// its `via`), both in store order; `heads` is this device's head table; `admitted` its roster's
/// admission now. `now` is this device's clock in unix milliseconds. Pure: nothing is asked over
/// the network, and the same inputs give the same view.
pub fn steward_lease_view(
    me: &str,
    own: &[LeaseEvent],
    folded: &[LeaseEvent],
    heads: &HeadTable,
    admitted: &Admission,
    now: u64,
    confirm_wait_s: u64,
) -> BTreeMap<Resource, LeaseVerdict> {
    let others: BTreeSet<&str> = admitted
        .devices()
        .into_iter()
        .filter(|device| *device != me)
        .collect();
    let mut leases: Vec<(Resource, Lease)> =
        replay(me, own.iter().filter(|event| event.via.is_none()))
            .into_iter()
            .map(|((resource, _), lease)| (resource, lease))
            .collect();
    let mut by_device: BTreeMap<&str, Vec<&LeaseEvent>> = BTreeMap::new();
    for event in folded {
        // A contested or unknown device's leases are ignored entirely.
        if let Some(device) = event.via.as_deref().filter(|d| others.contains(d)) {
            by_device.entry(device).or_default().push(event);
        }
    }
    for (device, events) in by_device {
        leases.extend(
            replay(device, events)
                .into_iter()
                .map(|((resource, _), lease)| (resource, lease)),
        );
    }

    let mut winners: BTreeMap<Resource, Lease> = BTreeMap::new();
    for (resource, lease) in leases {
        if !lease.live(now) {
            continue;
        }
        match winners.get(&resource) {
            Some(winner) if winner.precedence() <= lease.precedence() => {}
            _ => {
                winners.insert(resource, lease);
            }
        }
    }

    winners
        .into_iter()
        .map(|(resource, lease)| {
            let verdict = if lease.device == me {
                own_verdict(lease, me, &others, heads, now, confirm_wait_s)
            } else {
                LeaseVerdict::HeldBy {
                    device: lease.device,
                    session: lease.session,
                    since: lease.claimed_at,
                    overriding: lease.overriding,
                }
            };
            (resource, verdict)
        })
        .collect()
}

/// This device's winning lease: effective once every other admitted device acknowledged the
/// first own head that carried its claim.
fn own_verdict(
    lease: Lease,
    me: &str,
    others: &BTreeSet<&str>,
    heads: &HeadTable,
    now: u64,
    confirm_wait_s: u64,
) -> LeaseVerdict {
    if lease.overriding || others.is_empty() {
        return LeaseVerdict::Effective {
            session: lease.session,
        };
    }
    // The first own head projected from a store that already held the claim.
    let carried = heads
        .own_log
        .iter()
        .filter(|announce| announce.events >= lease.written)
        .map(|announce| announce.seq)
        .min();
    let waiting_on: Vec<String> = others
        .iter()
        .filter(|device| {
            let acked = heads
                .devices
                .get(**device)
                .and_then(|head| head.folded.get(me));
            !matches!((carried, acked), (Some(seq), Some(&acked)) if acked >= seq)
        })
        .map(|device| (*device).to_string())
        .collect();
    let Some(silent) = waiting_on.first().cloned() else {
        return LeaseVerdict::Effective {
            session: lease.session,
        };
    };
    let deadline = lease
        .claim_at
        .saturating_add(confirm_wait_s.saturating_mul(1000));
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
        }
    }
}
