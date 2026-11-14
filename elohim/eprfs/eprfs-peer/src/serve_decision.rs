//! Whether one artifact object may be served to one requester, read from the record's own reach
//! claim and the device's `serves` policy (host-compatibility plan item K12, packet 1).
//!
//! Pure: no lock, clock, store or network. The gate establishes who is asking and which record
//! was selected; this module only decides, and says how a refusal is answered on the wire
//! ([`wire_refusal`]) and recorded locally ([`deny_reason`]). The gate calls it on every artifact
//! answer (packet 2): `blob_mount::answer`, `DeviceScope` and `StewardDevicesGate`.
//!
//! A record's claim is the `env:reach=<level>` slot on this device's own custody `Produce`
//! events for its manifest ([`reach_claims`]). A record no own event gives a slot is
//! [`ReachClaim::Slotless`] and keeps today's rule: the method's `serves` row must name the
//! steward audience, and a method or input object also needs `discloseMethod`. A slotted record
//! is served by reach: only `self` serves, to an admitted device of the person, unless the
//! method's row narrows it. Every other value refuses, including every level above `self`, which
//! is refused until the ladder above `self` is built (rulings 2026-11-12, condition 5).

use std::collections::BTreeMap;

use elohim_epr_rea::FlowEvent;
use eprfs_core::BlobCid;

use crate::allow::slots;
use crate::blob_mount::custody_manifest;
use crate::serves::ArtifactServeList;

/// The classification slot prefix carrying a record's reach.
pub const REACH_SLOT: &str = "env:reach=";

/// Any slot whose key starts with this, in any case, is a reach slot; one not spelled exactly
/// [`REACH_SLOT`] is unreadable rather than absent, so a mistyped `private` is never slotless.
const REACH_KEY: &str = "env:reach";

/// The reach vocabulary the gate reads, with its openness (1 = most restrictive). The gate keeps
/// its own closed list so the daemon does not depend on `elohim-epr`; a test pins it to
/// `elohim_epr::Reach`.
pub const REACH_LEVELS: [(&str, u8); 8] = [
    ("private", 1),
    ("self", 2),
    ("intimate", 3),
    ("trusted", 4),
    ("familiar", 5),
    ("community", 6),
    ("public", 7),
    ("commons", 8),
];

const SELF_OPENNESS: u8 = 2;

/// Who is asking, as the gate established it. `None` is every requester the roster does not
/// admit as a device-held key, whatever else is true of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relationship {
    None,
    OwnDevice,
}

/// What this device's own custody events say about one manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachClaim {
    /// No own custody event for the manifest carries a reach slot.
    Slotless,
    /// The narrowest slot among the own custody events that carry one.
    Declared(DeclaredReach),
}

/// A reach slot's value as the gate reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredReach {
    Private,
    OwnDevices,
    /// A level above `self`, by openness (3..=8).
    Wider(u8),
    /// A value outside the vocabulary, an empty value, or more than one slot in one event.
    Unreadable,
}

impl DeclaredReach {
    /// Openness for choosing the narrowest; an unreadable slot sits below `private`, so it is
    /// never widened by another event and keeps its own reason.
    fn rank(self) -> u8 {
        match self {
            DeclaredReach::Unreadable => 0,
            DeclaredReach::Private => 1,
            DeclaredReach::OwnDevices => SELF_OPENNESS,
            DeclaredReach::Wider(openness) => openness,
        }
    }

    fn narrowest(self, other: DeclaredReach) -> DeclaredReach {
        if other.rank() < self.rank() {
            other
        } else {
            self
        }
    }
}

/// The loaded `berth.artifacts` policy, or the fact that none is loaded. A policy that failed to
/// parse is `Unavailable`, never an empty list: an empty list narrows nothing, so it would serve
/// every `self` record.
///
/// Callers hold the shared policy as `Option<ArtifactServeList>` (`None` for an unreadable file,
/// a daemon with no policy and a poisoned lock) and build this with [`ServePolicy::from`], never
/// by wrapping a defaulted [`ArtifactServeList`] in `Loaded`.
#[derive(Debug, Clone, Copy)]
pub enum ServePolicy<'a> {
    Unavailable,
    Loaded(&'a ArtifactServeList),
}

impl<'a> From<Option<&'a ArtifactServeList>> for ServePolicy<'a> {
    fn from(policy: Option<&'a ArtifactServeList>) -> Self {
        policy.map_or(ServePolicy::Unavailable, ServePolicy::Loaded)
    }
}

/// Which object of a record is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    ManifestOrChunk,
    MethodOrInput,
}

/// Why an object is served.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// A slotless record, served by its method's `serves` row, as before K12.
    MethodRow,
    /// A record declaring `self`, served to an admitted device of the person.
    Reach,
}

/// The slot a serve's `Use` event carries when it was served by reach.
pub const GATE_REACH_SLOT: &str = "gate:reach";

/// The slot a serve's `Use` event carries when it was served by its method's `serves` row.
pub const GATE_ROW_SLOT: &str = "gate:row";

impl Basis {
    /// The `gate:` slot a serve's `Use` event names this basis with.
    pub fn slot(self) -> &'static str {
        match self {
            Basis::MethodRow => GATE_ROW_SLOT,
            Basis::Reach => GATE_REACH_SLOT,
        }
    }
}

/// The standing an admitted device is told it lacks for a record declared above `self`.
pub const LADDER_UNBUILT: &str = "serve-reach:ladder-unbuilt";

/// How a refusal is answered on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireRefusal {
    /// Exactly what this device answers the same request for an object it does not hold: the
    /// requester cannot tell a refused record from a missing one.
    AsAbsent,
    /// A typed refusal naming a standing of the person's own policy the requester lacks.
    Standing(String),
}

/// What the requester is told for `deny`, as a function of who it is, the record's claim and its
/// method only.
///
/// - A requester that is not an admitted device of the person learns nothing, whatever the claim
///   or the reason (including an unavailable policy).
/// - A record declared `private`, or with a slot the gate cannot read, answers as absent even to
///   the person's own device, whatever the reason (including an unavailable policy): a sibling is
///   outside a `private` record's reach, and must not learn that it exists.
/// - Otherwise the standing labels of the method-row gate are kept: `serve-blob:<method>` for an
///   unavailable policy, a missing or non-steward row, or a row that narrows a `self` record;
///   `disclose-method:<method>` for an undisclosed slotless method; and
///   [`LADDER_UNBUILT`] for a record declared above `self`.
pub fn wire_refusal(
    deny: Deny,
    who: Relationship,
    claim: &ReachClaim,
    method_name: &str,
) -> WireRefusal {
    let private = matches!(
        claim,
        ReachClaim::Declared(DeclaredReach::Private | DeclaredReach::Unreadable)
    );
    if who == Relationship::None || private {
        return WireRefusal::AsAbsent;
    }
    match deny {
        Deny::NotAdmitted | Deny::Private => WireRefusal::AsAbsent,
        Deny::PolicyUnavailable | Deny::NoMethodRow | Deny::Narrowed => {
            WireRefusal::Standing(crate::gate::serve_blob_standing(method_name))
        }
        Deny::MethodNotDisclosed => WireRefusal::Standing(format!("disclose-method:{method_name}")),
        Deny::LadderUnbuilt => WireRefusal::Standing(LADDER_UNBUILT.to_string()),
    }
}

/// The true reason a refusal of an admitted device is recorded with on this device (the
/// `reason:` slot of its `draw:refused` event), which may say more than the wire does. A
/// `private` claim and an unreadable one share [`Deny::Private`]; which it was is read from the
/// claim.
pub fn deny_reason(deny: Deny, claim: &ReachClaim, method_name: &str) -> String {
    match deny {
        Deny::PolicyUnavailable => "serve-policy:unavailable".to_string(),
        Deny::NotAdmitted => "outside-standing".to_string(),
        Deny::NoMethodRow | Deny::Narrowed => crate::gate::serve_blob_standing(method_name),
        Deny::MethodNotDisclosed => format!("disclose-method:{method_name}"),
        Deny::Private => match claim {
            ReachClaim::Declared(DeclaredReach::Unreadable) => "serve-reach:unreadable",
            _ => "serve-reach:private",
        }
        .to_string(),
        Deny::LadderUnbuilt => LADDER_UNBUILT.to_string(),
    }
}

/// Why an object is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deny {
    /// No policy loaded, or it could not be read.
    PolicyUnavailable,
    /// The requester is not an admitted device of the person.
    NotAdmitted,
    /// Slotless, and no row for the method names the steward audience.
    NoMethodRow,
    /// Slotless method or input object whose method is not in `discloseMethod`.
    MethodNotDisclosed,
    /// Declared `private`, or a slot the gate cannot read. The two answer alike; a caller that
    /// records the local refusal reason reads it from the claim, not from this value.
    Private,
    /// Declared above `self`; refused until the ladder above `self` is built.
    LadderUnbuilt,
    /// Declared `self`, and the method's row exists without the steward audience.
    Narrowed,
}

/// Parse one slot value against the closed vocabulary; anything else is unreadable.
pub fn parse_reach(value: &str) -> DeclaredReach {
    match REACH_LEVELS.iter().find(|(name, _)| *name == value) {
        Some((_, 1)) => DeclaredReach::Private,
        Some((_, SELF_OPENNESS)) => DeclaredReach::OwnDevices,
        Some((_, openness)) => DeclaredReach::Wider(*openness),
        None => DeclaredReach::Unreadable,
    }
}

/// The reach one event declares: `None` without a slot, unreadable with more than one.
pub fn event_reach(event: &FlowEvent) -> Option<DeclaredReach> {
    let mut reach = slots(event).filter(|slot| {
        slot.get(..REACH_KEY.len())
            .is_some_and(|key| key.eq_ignore_ascii_case(REACH_KEY))
    });
    let first = reach.next()?;
    Some(match (reach.next(), first.strip_prefix(REACH_SLOT)) {
        (None, Some(value)) => parse_reach(value),
        _ => DeclaredReach::Unreadable,
    })
}

/// The claim of every manifest named by `custodian`'s own custody events, folded in one pass.
///
/// Only events that are this device's own custody count (a sibling's crossed or folded
/// `Produce` names no authority here). Once any own event of a manifest carries a slot, the
/// slotless ones are ignored, and the narrowest slot decides, so the order in which binaries of
/// different ages ran cannot widen a claim. A manifest absent from the map has no own custody
/// record and is refused before any decision: a caller must never default a missing entry to
/// [`ReachClaim::Slotless`], which would send a sibling-only manifest down the method-row path.
pub fn reach_claims<'e>(
    events: impl IntoIterator<Item = &'e FlowEvent>,
    custodian: &str,
) -> BTreeMap<BlobCid, ReachClaim> {
    let mut claims = BTreeMap::new();
    for event in events {
        let Some(manifest) = custody_manifest(event, custodian) else {
            continue;
        };
        let claim = claims.entry(manifest).or_insert(ReachClaim::Slotless);
        if let Some(declared) = event_reach(event) {
            *claim = match *claim {
                ReachClaim::Slotless => ReachClaim::Declared(declared),
                ReachClaim::Declared(current) => ReachClaim::Declared(current.narrowest(declared)),
            };
        }
    }
    claims
}

/// Whether `object` of a record whose claim is `claim` and whose method is `method_name` may be
/// served to `who` under `policy`. First match wins; every combination not listed denies.
///
/// # Errors
///
/// The [`Deny`] that refuses it.
pub fn serve_decision(
    who: Relationship,
    claim: &ReachClaim,
    method_name: &str,
    object: ObjectKind,
    policy: &ServePolicy<'_>,
) -> Result<Basis, Deny> {
    let ServePolicy::Loaded(serves) = policy else {
        return Err(Deny::PolicyUnavailable);
    };
    if who == Relationship::None {
        return Err(Deny::NotAdmitted);
    }
    match claim {
        ReachClaim::Slotless => {
            if !serves.row_grants_steward(method_name) {
                return Err(Deny::NoMethodRow);
            }
            if object == ObjectKind::MethodOrInput && !serves.discloses_method(method_name) {
                return Err(Deny::MethodNotDisclosed);
            }
            Ok(Basis::MethodRow)
        }
        ReachClaim::Declared(DeclaredReach::Private | DeclaredReach::Unreadable) => {
            Err(Deny::Private)
        }
        ReachClaim::Declared(DeclaredReach::Wider(_)) => Err(Deny::LadderUnbuilt),
        ReachClaim::Declared(DeclaredReach::OwnDevices) => {
            if serves.narrows(method_name) {
                Err(Deny::Narrowed)
            } else {
                Ok(Basis::Reach)
            }
        }
    }
}
