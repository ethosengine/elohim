//! D.4 — eligibility comes first, takes no parameters, and is never ranked away.
//!
//! Rail R-P5: ranking sees only eligible candidates, because an [`Eligible`] can be built only by
//! [`admit`]. The [`Eligibility`] governor face runs the same stages for the elevate sink, but its
//! `Effect` is the [`Charge`], so no face of it, `render` called alone included, yields an
//! `Eligible`:
//!
//! ```compile_fail
//! use elohim_compute::Governor;
//! use placement_core::{Candidacy, Capacity, Eligibility, Eligible, PlacementKind};
//! fn bypass<'a>(kind: &dyn PlacementKind, req: &Candidacy<'a>, ctx: &(Capacity, u64)) -> Eligible<'a> {
//!     Eligibility::new(kind).render(req, ctx).unwrap()
//! }
//! ```
//!
//! while the same call typed as the charge compiles:
//!
//! ```
//! use elohim_compute::Governor;
//! use placement_core::{Candidacy, Capacity, Charge, Eligibility, PlacementKind};
//! fn charge<'a>(kind: &dyn PlacementKind, req: &Candidacy<'a>, ctx: &(Capacity, u64)) -> Option<Charge> {
//!     Eligibility::new(kind).render(req, ctx).ok()
//! }
//! ```
//!
//! Outside the crate a struct literal does not compile:
//!
//! ```compile_fail
//! use placement_core::{Candidate, Charge, Eligible, OfferTerms};
//! fn forge<'a>(candidate: &'a Candidate, offer: &'a OfferTerms) -> Eligible<'a> {
//!     Eligible { candidate, offer, charge: Charge::default() }
//! }
//! ```
//!
//! and reading the accessors does:
//!
//! ```
//! use placement_core::Eligible;
//! fn charged_bytes(e: &Eligible<'_>) -> u64 { e.charge().bytes }
//! ```

use core::marker::PhantomData;

use elohim_compute::{Governor, LimitOwner, Refusal};
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use seam_contracts::{Answer, ComeBack, Meanwhile, ReasonLabel, Unanswered};
use serde::Serialize;

use crate::kind::{Charge, PlacementKind};
use crate::need::{AgentRefKey, Need, NeedKey, NeedTerms};
use crate::offer::{
    Candidate, Capacity, CidString, Consent, EpochSecs, EprScope, FailureDomain, OfferTerms,
};

/// A candidate admitted for one need. The fields are private (R-P5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eligible<'a> {
    candidate: &'a Candidate,
    offer: &'a OfferTerms,
    charge: Charge,
}

impl<'a> Eligible<'a> {
    pub fn candidate(&self) -> &'a Candidate {
        self.candidate
    }
    pub fn agent(&self) -> &'a AgentRef {
        &self.candidate.posture.carrier
    }
    pub fn offer(&self) -> &'a OfferTerms {
        self.offer
    }
    pub fn charge(&self) -> Charge {
        self.charge
    }
}

/// A refusal, with R7's three parts (D6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ineligible {
    pub agent: AgentRef,
    pub why: Unanswered<IneligibleReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "reason"
)]
#[non_exhaustive]
pub enum IneligibleReason {
    NoPosture,
    /// A fact the floor does not hold (spec §7.2).
    Unknown {
        input: &'static str,
    },
    OutsideReach {
        need: Reach,
        admitted: Reach,
    },
    OverCapacity,
    NoConsent,
    ConsentExpired,
    /// Only for a kind whose receiver may not carry.
    IsReceiver,
    /// Outside `Requirements::carriers`.
    NotNamed,
    /// D13, until per-space cells exist.
    CrossHouseholdBelowPublic,
    /// The kind's own refusal.
    Kind {
        label: &'static str,
    },
}

/// The `Copy` face for [`ReasonLabel`] (dashboards and the habit register enumerate the same labels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IneligibleKind {
    NoPosture,
    Unknown,
    OutsideReach,
    OverCapacity,
    NoConsent,
    ConsentExpired,
    IsReceiver,
    NotNamed,
    CrossHouseholdBelowPublic,
    Kind,
}

impl ReasonLabel for IneligibleKind {
    const ALL: &'static [Self] = &[
        IneligibleKind::NoPosture,
        IneligibleKind::Unknown,
        IneligibleKind::OutsideReach,
        IneligibleKind::OverCapacity,
        IneligibleKind::NoConsent,
        IneligibleKind::ConsentExpired,
        IneligibleKind::IsReceiver,
        IneligibleKind::NotNamed,
        IneligibleKind::CrossHouseholdBelowPublic,
        IneligibleKind::Kind,
    ];

    fn label(&self) -> &'static str {
        match self {
            IneligibleKind::NoPosture => "no-posture",
            IneligibleKind::Unknown => "unknown",
            IneligibleKind::OutsideReach => "outside-reach",
            IneligibleKind::OverCapacity => "over-capacity",
            IneligibleKind::NoConsent => "no-consent",
            IneligibleKind::ConsentExpired => "consent-expired",
            IneligibleKind::IsReceiver => "is-receiver",
            IneligibleKind::NotNamed => "not-named",
            IneligibleKind::CrossHouseholdBelowPublic => "cross-household-below-public",
            IneligibleKind::Kind => "kind",
        }
    }
}

impl IneligibleReason {
    pub fn kind(&self) -> IneligibleKind {
        match self {
            IneligibleReason::NoPosture => IneligibleKind::NoPosture,
            IneligibleReason::Unknown { .. } => IneligibleKind::Unknown,
            IneligibleReason::OutsideReach { .. } => IneligibleKind::OutsideReach,
            IneligibleReason::OverCapacity => IneligibleKind::OverCapacity,
            IneligibleReason::NoConsent => IneligibleKind::NoConsent,
            IneligibleReason::ConsentExpired => IneligibleKind::ConsentExpired,
            IneligibleReason::IsReceiver => IneligibleKind::IsReceiver,
            IneligibleReason::NotNamed => IneligibleKind::NotNamed,
            IneligibleReason::CrossHouseholdBelowPublic => {
                IneligibleKind::CrossHouseholdBelowPublic
            }
            IneligibleReason::Kind { .. } => IneligibleKind::Kind,
        }
    }

    /// Code `GateRefused(label)`; the limit owner named per reason. `OutsideReach` is the space's
    /// membrane, so it is `LimitOwner::Charter`. `CrossHouseholdBelowPublic` stays `Operator`: it
    /// holds "until per-space cells exist" (D13), a runtime limit, not a charter's rule.
    pub fn refusal(&self) -> Refusal {
        let label = self.kind().label();
        let (owner, elevate) = match self {
            IneligibleReason::NoConsent | IneligibleReason::ConsentExpired => (
                LimitOwner::Commitment,
                "no grant in force covers this need".to_string(),
            ),
            IneligibleReason::NoPosture => (
                LimitOwner::SelfLimit,
                "the carrier's own posture does not offer this kind".to_string(),
            ),
            IneligibleReason::OverCapacity => (
                LimitOwner::SelfLimit,
                "the carrier's offered capacity is spent".to_string(),
            ),
            IneligibleReason::NotNamed => (
                LimitOwner::Commitment,
                "the need names its carriers and this is not one".to_string(),
            ),
            IneligibleReason::OutsideReach { need, admitted } => (
                LimitOwner::Charter,
                format!(
                    "the space admits this carrier at {admitted:?}, wider than the need's {need:?}"
                ),
            ),
            IneligibleReason::Unknown { input } => (
                LimitOwner::Operator,
                format!("the floor holds no {input} fact"),
            ),
            IneligibleReason::IsReceiver => (
                LimitOwner::Operator,
                "a receiver's own copy is not a second holder".to_string(),
            ),
            IneligibleReason::CrossHouseholdBelowPublic => (
                LimitOwner::Operator,
                "no transport yet writes into another household's plane".to_string(),
            ),
            // The floor's own checks of the carrier's offer, carried as `Kind { label }` because the
            // view schema names no variant for them: each is the carrier's own posture (an
            // own-household-only offer, the scopes it will spend, the isolation it runs at), so each
            // is the carrier's line, never an operator veto (elohim-compute actuation's invariant).
            IneligibleReason::Kind { label } if OWN_OFFER_LABELS.contains(label) => (
                LimitOwner::SelfLimit,
                format!("the carrier's own offer does not extend to this need: {label}"),
            ),
            IneligibleReason::Kind { label } => {
                (LimitOwner::Operator, format!("the kind refused: {label}"))
            }
        };
        Refusal::gate(owner, label, elevate)
    }

    /// The come-back event and the meanwhile for each reason (posture-changed, grant-issued,
    /// capacity-freed; "run here" or "nothing").
    pub fn unanswered(self, _now: EpochSecs) -> Unanswered<IneligibleReason> {
        let on = |event: &str| ComeBack::OnEvent {
            event: event.to_string(),
        };
        let nothing = |because: &str| Meanwhile::Nothing {
            because: because.to_string(),
        };
        let (come_back, meanwhile) = match &self {
            IneligibleReason::NoPosture => (
                on("posture-changed"),
                nothing("no posture offers this kind"),
            ),
            IneligibleReason::Unknown { input } => {
                let event = match *input {
                    "consent" => "grant-issued",
                    "reach" | "household" => "membership-changed",
                    _ => "posture-changed",
                };
                (on(event), nothing("the fact is not held"))
            }
            IneligibleReason::OutsideReach { .. } => (
                on("membership-changed"),
                nothing("no carrier inside the need's reach"),
            ),
            IneligibleReason::OverCapacity => (on("capacity-freed"), nothing("the offer is spent")),
            IneligibleReason::NoConsent | IneligibleReason::ConsentExpired => {
                (on("grant-issued"), Meanwhile::RunHere)
            }
            IneligibleReason::IsReceiver => (
                on("posture-changed"),
                Meanwhile::UseHeld {
                    what: "the receiver's own copy".to_string(),
                },
            ),
            IneligibleReason::NotNamed => (
                on("carriers-changed"),
                nothing("the need names other carriers"),
            ),
            IneligibleReason::CrossHouseholdBelowPublic => (
                on("household-records-transport"),
                nothing("below public, placement stays in the receiver's household"),
            ),
            IneligibleReason::Kind { .. } => (on("posture-changed"), nothing("the kind refused")),
        };
        Unanswered::new(self, come_back, meanwhile)
    }
}

/// The floor-checked `Kind` labels that are the carrier's own offer's limits (see `refusal`).
/// `household-only` is the person's own `DeviceFacts::posture` choice (D11); `scope` and
/// `isolation` are what the offer declares it will spend and run at (F6).
pub const OWN_OFFER_LABELS: &[&str] = &[HOUSEHOLD_ONLY, SCOPE, ISOLATION];
/// An own-household-only offer meeting another household's need (D11).
pub const HOUSEHOLD_ONLY: &str = "household-only";
/// The offer's `scopes` lack the scope the need consumes, or the need declares a scope other than
/// the one its kind derives (F1, F6).
pub const SCOPE: &str = "scope";
/// The offer's isolation is unknown or weaker than the need accepts (F6).
pub const ISOLATION: &str = "isolation";

/// The first stage, consent for this receiver (scope, recipient, window, epr_scope). An
/// `Unreachable` grant refuses, as `Unknown { input: "consent" }`.
///
/// `NotNeeded` answers a kind that needs a grant only for the receiver's own carrier (the
/// requester's own envelope, §F5): for any other carrier it is `NoConsent`, so a snapshot builder's
/// consent map cannot stand in for the grant's scope, recipient, window and `epr_scope`.
fn stage_consent(
    kind: &dyn PlacementKind,
    need: &Need,
    carrier: &AgentRef,
    consent: &Answer<Consent>,
    now: EpochSecs,
) -> Result<(), IneligibleReason> {
    let required = kind.scope(need);
    // A need's own `terms.scope` is the requester's declaration, never authority (F1): the scope a
    // grant must name is the kind's derivation, and a need declaring another is refused for every
    // carrier, so a requester cannot spend a narrower grant on a wider task.
    if let NeedTerms::Task {
        scope: declared, ..
    } = &need.terms
    {
        if required != Some(*declared) {
            return Err(IneligibleReason::Kind { label: SCOPE });
        }
    }
    let Some(required) = required else {
        return Ok(()); // a declared posture suffices (R3)
    };
    match consent {
        Answer::Present(Consent::NotNeeded) if carrier == &need.receiver => Ok(()),
        Answer::Present(Consent::NotNeeded) => Err(IneligibleReason::NoConsent),
        Answer::Present(Consent::Granted {
            scope,
            recipient,
            valid_from,
            valid_until,
            epr_scope,
            ..
        }) => {
            if *scope != required || recipient != &AgentRefKey::of(&need.receiver) {
                return Err(IneligibleReason::NoConsent);
            }
            if now < *valid_from || now >= *valid_until {
                return Err(IneligibleReason::ConsentExpired);
            }
            match (epr_scope, &need.key) {
                (EprScope::All, _) => Ok(()),
                (EprScope::Tasks { cids }, NeedKey::Task(c))
                    if cids.contains(&CidString::from(c)) =>
                {
                    Ok(())
                }
                (EprScope::Tasks { .. }, _) => Err(IneligibleReason::NoConsent),
            }
        }
        Answer::Absent => Err(IneligibleReason::NoConsent),
        Answer::Unreachable => Err(IneligibleReason::Unknown { input: "consent" }),
    }
}

/// The middle stages, in order: posture offers the kind and is live → every input known → named
/// carriers → receiver rule → reach for this receiver. Returns the offer.
fn stage_gate<'a>(
    kind: &dyn PlacementKind,
    need: &Need,
    candidate: &'a Candidate,
    receiver_domain: Option<&FailureDomain>,
) -> Result<&'a OfferTerms, IneligibleReason> {
    let offer = candidate
        .posture
        .offers
        .get(&kind.id())
        .ok_or(IneligibleReason::NoPosture)?;
    if candidate.live != Answer::Present(true) {
        return Err(IneligibleReason::Unknown { input: "liveness" });
    }
    let receiver = AgentRefKey::of(&need.receiver);
    let admitted = match candidate.admitted.get(&receiver) {
        Some(Answer::Present(r)) => Some(*r),
        Some(Answer::Absent) => None,
        Some(Answer::Unreachable) | None => {
            return Err(IneligibleReason::Unknown { input: "reach" })
        }
    };
    let below_public = need.reach.openness() < Reach::Public.openness();
    let is_receiver = candidate.posture.carrier == need.receiver;
    if (below_public || offer.household_only) && !is_receiver && receiver_domain.is_none() {
        return Err(IneligibleReason::Unknown { input: "household" });
    }
    if let Some(named) = &need.requires.carriers {
        if !named.contains(&AgentRefKey::of(&candidate.posture.carrier)) {
            return Err(IneligibleReason::NotNamed);
        }
    }
    if is_receiver && !kind.receiver_may_carry() {
        return Err(IneligibleReason::IsReceiver);
    }
    let Some(admitted) = admitted else {
        return Err(IneligibleReason::OutsideReach {
            need: need.reach,
            admitted: Reach::Commons,
        });
    };
    if admitted.openness() > need.reach.openness() {
        return Err(IneligibleReason::OutsideReach {
            need: need.reach,
            admitted,
        });
    }
    if let Some(ceiling) = offer.reach_ceiling {
        if need.reach.openness() > ceiling.openness() {
            return Err(IneligibleReason::OutsideReach {
                need: need.reach,
                admitted: ceiling,
            });
        }
    }
    let same_household = is_receiver || receiver_domain == Some(&candidate.domain);
    if !same_household {
        if below_public {
            return Err(IneligibleReason::CrossHouseholdBelowPublic);
        }
        if offer.household_only {
            return Err(IneligibleReason::Kind {
                label: HOUSEHOLD_ONLY,
            });
        }
    }
    // F6, universal: a kind that spends compute is carried only by an offer that will spend that
    // scope, and a need's isolation floor only by an offer known to run at or above it.
    if let Some(required) = kind.scope(need) {
        if !offer.scopes.contains(&required) {
            return Err(IneligibleReason::Kind { label: SCOPE });
        }
    }
    if let Some(floor) = need.requires.isolation {
        if offer.isolation.is_none_or(|i| i < floor) {
            return Err(IneligibleReason::Kind { label: ISOLATION });
        }
    }
    Ok(offer)
}

/// The universal capacity stage, before `kind.fits`: every component the need requires is charged
/// against `remaining`, the capacity this kind's offer has left (`decide` passes the offer for the
/// need's own kind, net of everything charged to the carrier this pass). A component the need
/// requires and the offer does not state is `Unknown { input: "capacity" }`, never unbounded (spec
/// §7.2); one past what is left is `OverCapacity`.
fn stage_capacity(need: &Need, remaining: &Capacity) -> Result<(), IneligibleReason> {
    fn fits<T: PartialOrd>(asked: Option<T>, left: Option<T>) -> Result<(), IneligibleReason> {
        match (asked, left) {
            (None, _) => Ok(()),
            (Some(_), None) => Err(IneligibleReason::Unknown { input: "capacity" }),
            (Some(a), Some(l)) if a > l => Err(IneligibleReason::OverCapacity),
            _ => Ok(()),
        }
    }
    let r = &need.requires;
    fits(r.bytes, remaining.bytes)?;
    fits(r.memory_bytes, remaining.memory_bytes)?;
    fits(r.cpu_millis, remaining.cpu_millis)?;
    fits(r.pids, remaining.pids)
}

/// The charge a pass deducts, after `kind.fits`: never less than the need requires in any component
/// (a kind cannot widen the floor by under-charging, §D.1), and never more than `remaining` holds
/// (§D.7). A component charged where the offer states no figure is `Unknown { input: "capacity" }`,
/// as in [`stage_capacity`]. Codex review 1, finding 1.
fn floor_charge(
    need: &Need,
    charge: Charge,
    remaining: &Capacity,
) -> Result<Charge, IneligibleReason> {
    fn bound<T: Ord + Copy + Default>(
        asked: Option<T>,
        charged: T,
        left: Option<T>,
    ) -> Result<T, IneligibleReason> {
        let total = asked.map_or(charged, |a| a.max(charged));
        match left {
            _ if total == T::default() => Ok(total),
            None => Err(IneligibleReason::Unknown { input: "capacity" }),
            Some(l) if total > l => Err(IneligibleReason::OverCapacity),
            Some(_) => Ok(total),
        }
    }
    let r = &need.requires;
    Ok(Charge {
        bytes: bound(r.bytes, charge.bytes, remaining.bytes)?,
        memory_bytes: bound(r.memory_bytes, charge.memory_bytes, remaining.memory_bytes)?,
        cpu_millis: bound(r.cpu_millis, charge.cpu_millis, remaining.cpu_millis)?,
        pids: bound(r.pids, charge.pids, remaining.pids)?,
    })
}

fn admit_with<'a>(
    kind: &dyn PlacementKind,
    need: &Need,
    candidate: &'a Candidate,
    consent: &Answer<Consent>,
    receiver_domain: Option<&FailureDomain>,
    remaining: &Capacity,
    now: EpochSecs,
) -> Result<Eligible<'a>, IneligibleReason> {
    stage_consent(kind, need, &candidate.posture.carrier, consent, now)?;
    let offer = stage_gate(kind, need, candidate, receiver_domain)?;
    stage_capacity(need, remaining)?;
    let charge = floor_charge(
        need,
        kind.fits(need, candidate, offer, remaining)?,
        remaining,
    )?;
    Ok(Eligible {
        candidate,
        offer,
        charge,
    })
}

fn consent_for(need: &Need, candidate: &Candidate) -> Answer<Consent> {
    candidate
        .consent
        .get(&AgentRefKey::of(&need.receiver))
        .cloned()
        .unwrap_or(Answer::Unreachable)
}

/// The typed face. Order is fixed and matches `Governor::decide`'s authorize → gate → render (D17):
/// consent for this receiver (scope, recipient, window, epr_scope; an `Unreachable` grant refuses)
/// → posture offers the kind, and is live → every input known → named carriers → receiver rule →
/// reach for this receiver (`admitted.openness() <= need.reach.openness()`) → household rule →
/// scope and isolation (F6) → capacity, every required component against `remaining` → `kind.fits`,
/// which only narrows. No parameter set is an argument: the ceiling cannot reach it.
///
/// `remaining` is what the offer for `need.kind` has left: `decide` passes
/// `Capacity::of_offer(offer)` net of everything already charged to this carrier in the pass.
///
/// `receiver_domain` revises the spec's signature: a cross-household carrier of a below-public need
/// is refused `CrossHouseholdBelowPublic` (D13), and `household_only` offers are own-household only
/// (D11), and neither is decidable from the candidate alone. `decide` resolves it from the
/// receiver's own posture in the snapshot.
pub fn admit<'a>(
    kind: &dyn PlacementKind,
    need: &Need,
    candidate: &'a Candidate,
    receiver_domain: Option<&FailureDomain>,
    remaining: &Capacity,
    now: EpochSecs,
) -> Result<Eligible<'a>, Ineligible> {
    admit_with(
        kind,
        need,
        candidate,
        &consent_for(need, candidate),
        receiver_domain,
        remaining,
        now,
    )
    .map_err(|why| Ineligible {
        agent: candidate.posture.carrier.clone(),
        why: why.unanswered(now),
    })
}

/// The elevate face over the same stage functions, so a refusal reaches the elevate sink in the
/// shape every other `Governor` uses (`elohim-compute/src/actuation.rs`). The default spine
/// (authorize → gate → render) runs the stages in [`admit`]'s order, so both faces yield the same
/// first refusal. Its `Effect` is the [`Charge`], never an [`Eligible`]: `render` is public and
/// skips `authorize`, so an `Eligible` from it would be a carrier admitted without its grant
/// (R-P5, fix round 2; the spec gave this face `Effect = Eligible`).
pub struct Eligibility<'k, 'a> {
    pub kind: &'k dyn PlacementKind,
    _for: PhantomData<&'a ()>,
}

impl<'k, 'a> Eligibility<'k, 'a> {
    pub fn new(kind: &'k dyn PlacementKind) -> Self {
        Self {
            kind,
            _for: PhantomData,
        }
    }
}

/// One candidate for one need. `receiver_domain` as in [`admit`].
pub struct Candidacy<'a> {
    pub need: &'a Need,
    pub candidate: &'a Candidate,
    pub receiver_domain: Option<&'a FailureDomain>,
}

impl<'k, 'a> Governor for Eligibility<'k, 'a> {
    type Request = Candidacy<'a>;
    type Grant = Answer<Consent>;
    type Context = (Capacity, EpochSecs);
    type Effect = Charge;

    fn authorize(
        &self,
        req: &Candidacy<'a>,
        grant: &Answer<Consent>,
        now_epoch_s: u64,
    ) -> Result<(), Refusal> {
        stage_consent(
            self.kind,
            req.need,
            &req.candidate.posture.carrier,
            grant,
            now_epoch_s,
        )
        .map_err(|r| r.refusal())
    }

    fn gate(&self, req: &Candidacy<'a>, _ctx: &(Capacity, EpochSecs)) -> Result<(), Refusal> {
        stage_gate(self.kind, req.need, req.candidate, req.receiver_domain)
            .map(|_| ())
            .map_err(|r| r.refusal())
    }

    fn render(&self, req: &Candidacy<'a>, ctx: &(Capacity, EpochSecs)) -> Result<Charge, Refusal> {
        let offer = stage_gate(self.kind, req.need, req.candidate, req.receiver_domain)
            .map_err(|r| r.refusal())?;
        stage_capacity(req.need, &ctx.0).map_err(|r| r.refusal())?;
        self.kind
            .fits(req.need, req.candidate, offer, &ctx.0)
            .and_then(|charge| floor_charge(req.need, charge, &ctx.0))
            .map_err(|r| r.refusal())
    }
}

/// Crate-internal: an `Eligible` for the ranking contract's synthetic inputs, which admit nothing.
#[cfg(any(test, feature = "testing"))]
pub(crate) fn eligible_for_contract<'a>(
    candidate: &'a Candidate,
    offer: &'a OfferTerms,
) -> Eligible<'a> {
    Eligible {
        candidate,
        offer,
        charge: Charge::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind::testing::BytesKind;
    use crate::kind::{GrantScope, KindId};
    use crate::need::{Demand, Expresser, ExpresserKind, NeedClass, NeedTerms, Requirements};
    use crate::offer::Posture;
    use elohim_compute::RefusalCode;
    use std::collections::BTreeMap;

    fn agent(s: &str) -> AgentRef {
        AgentRef(s.to_string())
    }

    fn need(reach: Reach) -> Need {
        Need {
            kind: KindId::CustodyRoot,
            key: NeedKey::TreeView(elohim_epr::cid::compute_cid(b"tree")),
            demand: Demand {
                holders: 2,
                domains: 1,
            },
            requires: Requirements {
                bytes: Some(10),
                ..Requirements::default()
            },
            terms: NeedTerms::None,
            reach,
            receiver: agent("uhCAk-r"),
            expressed_by: Expresser {
                by: ExpresserKind::Person,
                id: "uhCAk-r".into(),
            },
            class: NeedClass::Member,
            in_scope_of: elohim_epr::cid::compute_cid(b"space"),
        }
    }

    fn candidate(name: &str, household: &str, admitted: Answer<Reach>) -> Candidate {
        let carrier = agent(name);
        let mut offers = BTreeMap::new();
        offers.insert(
            KindId::CustodyRoot,
            OfferTerms {
                spare_bytes: Some(100),
                ..OfferTerms::default()
            },
        );
        Candidate {
            domain: FailureDomain::of(Some(household), &carrier),
            posture: Posture {
                carrier,
                offers,
                reach: Reach::Intimate,
                written_at: 0,
            },
            live: Answer::Present(true),
            admitted: [(AgentRefKey::of(&agent("uhCAk-r")), admitted)]
                .into_iter()
                .collect(),
            consent: BTreeMap::new(),
        }
    }

    fn hh(s: &str) -> FailureDomain {
        FailureDomain::of(Some(s), &agent("x"))
    }

    fn room() -> Capacity {
        Capacity {
            bytes: Some(100),
            ..Capacity::default()
        }
    }

    fn first_refusal(
        n: &Need,
        c: &Candidate,
        domain: Option<&FailureDomain>,
        remaining: &Capacity,
    ) -> Option<IneligibleKind> {
        admit(
            &BytesKind::custody(KindId::CustodyRoot),
            n,
            c,
            domain,
            remaining,
            5,
        )
        .err()
        .map(|i| i.why.why.kind())
    }

    #[test]
    fn an_admitted_candidate_carries_its_charge() {
        let n = need(Reach::Intimate);
        let c = candidate("uhCAk-a", "A", Answer::Present(Reach::Intimate));
        let e = admit(
            &BytesKind::custody(KindId::CustodyRoot),
            &n,
            &c,
            Some(&hh("A")),
            &room(),
            5,
        )
        .unwrap();
        assert_eq!(e.charge().bytes, 10);
        assert_eq!(e.agent(), &agent("uhCAk-a"));
    }

    #[test]
    fn the_three_answer_states_are_three_readings() {
        let n = need(Reach::Intimate);
        let mut c = candidate("uhCAk-a", "A", Answer::Present(Reach::Intimate));
        for live in [Answer::Present(false), Answer::Absent, Answer::Unreachable] {
            c.live = live;
            assert_eq!(
                first_refusal(&n, &c, Some(&hh("A")), &room()),
                Some(IneligibleKind::Unknown)
            );
        }
        let c = candidate("uhCAk-a", "A", Answer::Absent);
        assert_eq!(
            first_refusal(&n, &c, Some(&hh("A")), &room()),
            Some(IneligibleKind::OutsideReach)
        );
        let c = candidate("uhCAk-a", "A", Answer::Unreachable);
        assert_eq!(
            first_refusal(&n, &c, Some(&hh("A")), &room()),
            Some(IneligibleKind::Unknown)
        );
    }

    #[test]
    fn reach_is_compared_by_openness() {
        let n = need(Reach::Intimate);
        let c = candidate("uhCAk-a", "A", Answer::Present(Reach::Community));
        let refusal = admit(
            &BytesKind::custody(KindId::CustodyRoot),
            &n,
            &c,
            Some(&hh("A")),
            &room(),
            0,
        )
        .unwrap_err();
        assert_eq!(
            refusal.why.why,
            IneligibleReason::OutsideReach {
                need: Reach::Intimate,
                admitted: Reach::Community
            }
        );
    }

    #[test]
    fn below_public_stays_in_the_receivers_household() {
        let c = candidate("uhCAk-b", "B", Answer::Present(Reach::Intimate));
        assert_eq!(
            first_refusal(&need(Reach::Intimate), &c, Some(&hh("A")), &room()),
            Some(IneligibleKind::CrossHouseholdBelowPublic)
        );
        let c = candidate("uhCAk-b", "B", Answer::Present(Reach::Public));
        assert_eq!(
            first_refusal(&need(Reach::Public), &c, Some(&hh("A")), &room()),
            None
        );
        // An unknown receiver household is an unknown input, never a default.
        assert_eq!(
            first_refusal(&need(Reach::Intimate), &c, None, &room()),
            Some(IneligibleKind::Unknown)
        );
    }

    #[test]
    fn the_receiver_is_not_a_second_custody_holder() {
        let c = candidate("uhCAk-r", "A", Answer::Present(Reach::SelfScope));
        assert_eq!(
            first_refusal(&need(Reach::Intimate), &c, Some(&hh("A")), &room()),
            Some(IneligibleKind::IsReceiver)
        );
    }

    #[test]
    fn named_carriers_narrow_admit() {
        let mut n = need(Reach::Intimate);
        n.requires.carriers = Some([AgentRefKey::of(&agent("uhCAk-j"))].into_iter().collect());
        let c = candidate("uhCAk-a", "A", Answer::Present(Reach::Intimate));
        assert_eq!(
            first_refusal(&n, &c, Some(&hh("A")), &room()),
            Some(IneligibleKind::NotNamed)
        );
    }

    /// A task offer that spends `measure-stage` with stated capacity.
    fn task_offer() -> OfferTerms {
        OfferTerms {
            scopes: [GrantScope::MeasureStage].into_iter().collect(),
            ..OfferTerms::default()
        }
    }

    /// A kind that admits anything, so only the floor's universal stages can refuse.
    struct Lenient(KindId, Option<GrantScope>);
    impl PlacementKind for Lenient {
        fn id(&self) -> KindId {
            self.0
        }
        fn action(&self, _: &Need) -> crate::kind::MishpatAction {
            crate::kind::MishpatAction::ReplicatesContent
        }
        fn scope(&self, _: &Need) -> Option<GrantScope> {
            self.1
        }
        fn receiver_may_carry(&self) -> bool {
            true
        }
        fn fits(
            &self,
            _: &Need,
            _: &Candidate,
            _: &OfferTerms,
            _: &Capacity,
        ) -> Result<Charge, IneligibleReason> {
            Ok(Charge::default())
        }
        fn default_method(&self) -> crate::rank::MethodId {
            crate::params::InForce::builtin(self.0).set().method
        }
    }

    #[test]
    fn capacity_is_the_floors_refusal_before_the_kinds() {
        // The lenient kind would admit anything: the refusals below are the floor's own.
        let lenient = Lenient(KindId::CustodyRoot, None);
        let n = need(Reach::Intimate);
        let c = candidate("uhCAk-a", "A", Answer::Present(Reach::Intimate));
        let run = |remaining: &Capacity| {
            admit(&lenient, &n, &c, Some(&hh("A")), remaining, 5)
                .err()
                .map(|i| i.why.why)
        };
        let tight = Capacity {
            bytes: Some(3),
            ..Capacity::default()
        };
        assert_eq!(run(&tight), Some(IneligibleReason::OverCapacity));
        // An offer that states no bytes is an unknown, never an unbounded, capacity (spec §7.2).
        assert_eq!(
            run(&Capacity::default()),
            Some(IneligibleReason::Unknown { input: "capacity" })
        );
        assert_eq!(run(&room()), None);
        // Every required component is charged, not only bytes.
        let mut pids = need(Reach::Intimate);
        pids.requires.pids = Some(4);
        let some_pids = Capacity {
            pids: Some(3),
            ..room()
        };
        assert_eq!(
            admit(&lenient, &pids, &c, Some(&hh("A")), &some_pids, 5)
                .err()
                .map(|i| i.why.why),
            Some(IneligibleReason::OverCapacity)
        );
        assert_eq!(
            admit(&lenient, &pids, &c, Some(&hh("A")), &room(), 5)
                .err()
                .map(|i| i.why.why),
            Some(IneligibleReason::Unknown { input: "capacity" })
        );
    }

    /// Codex review 1, finding 1: a kind that charges less than the need requires cannot widen the
    /// floor. The charge `admit` records is at least `need.requires`, and one past what is left is
    /// refused, whatever the kind returned.
    #[test]
    fn a_kind_cannot_under_charge_or_over_charge_a_need() {
        let lenient = Lenient(KindId::CustodyRoot, None);
        let n = need(Reach::Intimate);
        let c = candidate("uhCAk-a", "A", Answer::Present(Reach::Intimate));
        let e = admit(&lenient, &n, &c, Some(&hh("A")), &room(), 5).unwrap();
        assert_eq!(
            e.charge().bytes,
            10,
            "the floor charges what the need requires"
        );
        // The elevate face records the same charge.
        let face = Eligibility::new(&lenient);
        let req = Candidacy {
            need: &n,
            candidate: &c,
            receiver_domain: Some(&hh("A")),
        };
        assert_eq!(face.render(&req, &(room(), 5)).unwrap().bytes, 10);
        // A kind charging past what is left, or into a component the offer does not state, is refused.
        let over = Charge {
            bytes: 101,
            ..Charge::default()
        };
        assert_eq!(
            floor_charge(&n, over, &room()),
            Err(IneligibleReason::OverCapacity)
        );
        let unstated = Charge {
            pids: 1,
            ..Charge::default()
        };
        assert_eq!(
            floor_charge(&n, unstated, &room()),
            Err(IneligibleReason::Unknown { input: "capacity" })
        );
    }

    /// Codex review 1, finding 1, across needs: with the lenient kind (which charges nothing), two
    /// 100-byte needs never both land on one 100-byte offer in one pass.
    #[test]
    fn an_under_charging_kind_cannot_place_two_needs_on_one_offer() {
        use crate::decide::{decide, Charter, Snapshot};
        use crate::kind::{KindRegistry, MethodRegistry};
        use crate::offer::CidString;
        use crate::params::RatificationRule;
        use crate::rank::Scratch;
        let charter = Charter {
            id: None,
            adopted_at: 0,
            demand: BTreeMap::new(),
            serve_order: Vec::new(),
            holding_floor: Demand {
                holders: 1,
                domains: 1,
            },
            forwarding: None,
            genesis_params: BTreeMap::new(),
            tuning_rule: RatificationRule::TouchedConsent,
            moves_per_event: (1, 4),
            freshness_horizon_secs: 600,
        };
        let mut needs = Vec::new();
        for tag in [b"one".as_slice(), b"two"] {
            let mut n = need(Reach::Intimate);
            n.key = NeedKey::TreeView(elohim_epr::cid::compute_cid(tag));
            n.requires.bytes = Some(100);
            n.demand = Demand {
                holders: 1,
                domains: 1,
            };
            needs.push(n);
        }
        let mut s = Snapshot::empty(agent("uhCAk-a"));
        s.now = 5;
        let scope = CidString::from(&needs[0].in_scope_of);
        s.charters.insert(scope.clone(), charter);
        s.needs_read.insert(scope, Answer::Present(()));
        s.needs = needs;
        // The receiver is in household A and offers nothing, so it only places the household.
        let mut receiver = candidate("uhCAk-r", "A", Answer::Present(Reach::Intimate));
        receiver.posture.offers.clear();
        s.candidates = vec![
            candidate("uhCAk-a", "A", Answer::Present(Reach::Intimate)),
            receiver,
        ];
        let kinds = KindRegistry::new().with(Box::new(Lenient(KindId::CustodyRoot, None)));
        let plan = decide(
            &s,
            &kinds,
            &MethodRegistry::builtin(),
            &mut Scratch::default(),
        );
        assert_eq!(
            plan.adopt.len(),
            1,
            "one 100-byte offer carries one 100-byte need"
        );
        assert_eq!(plan.unserved.len(), 1);
    }

    #[test]
    fn an_offer_must_spend_the_scope_and_meet_the_isolation() {
        use crate::need::Isolation;
        // The lenient kind checks nothing itself: scope and isolation are the floor's.
        let lenient = Lenient(KindId::Task, Some(GrantScope::MeasureStage));
        let task_cid = elohim_epr::cid::compute_cid(b"task");
        let mut n = need(Reach::Public);
        n.kind = KindId::Task;
        n.key = NeedKey::Task(task_cid);
        n.terms = NeedTerms::Task {
            input_root: CidString::from(&task_cid),
            scope: GrantScope::MeasureStage,
            base_root: None,
        };
        // The receiver's own envelope needs no grant, so only the offer is tested.
        let with = |offer: OfferTerms| {
            let mut c = candidate("uhCAk-r", "A", Answer::Present(Reach::Public));
            c.posture.offers.insert(KindId::Task, offer);
            c.consent.insert(
                AgentRefKey::of(&n.receiver),
                Answer::Present(Consent::NotNeeded),
            );
            c
        };
        let run = |n: &Need, c: &Candidate| {
            admit(&lenient, n, c, Some(&hh("A")), &room(), 5)
                .err()
                .map(|i| i.why.why)
        };
        let scope_refused = Some(IneligibleReason::Kind { label: SCOPE });
        let isolation_refused = Some(IneligibleReason::Kind { label: ISOLATION });
        assert_eq!(run(&n, &with(OfferTerms::default())), scope_refused);
        let other_scope = OfferTerms {
            scopes: [GrantScope::HostedCell].into_iter().collect(),
            ..OfferTerms::default()
        };
        assert_eq!(run(&n, &with(other_scope)), scope_refused);
        assert_eq!(run(&n, &with(task_offer())), None);

        n.requires.isolation = Some(Isolation::Bounded);
        assert_eq!(
            run(&n, &with(task_offer())),
            isolation_refused,
            "an offer that states no isolation does not meet a floor"
        );
        let weaker = OfferTerms {
            isolation: Some(Isolation::Delegated),
            ..task_offer()
        };
        assert_eq!(run(&n, &with(weaker)), isolation_refused);
        for ok in [Isolation::Bounded, Isolation::Enforced] {
            let offer = OfferTerms {
                isolation: Some(ok),
                ..task_offer()
            };
            assert_eq!(run(&n, &with(offer)), None, "{ok:?}");
        }
    }

    #[test]
    fn a_need_declaring_another_scope_than_its_kind_derives_is_refused() {
        // The kind derives measure-stage (from the task's project, F1). A requester holding only a
        // sweettest-feedback grant writes that scope on the need: every carrier refuses it.
        let lenient = Lenient(KindId::Task, Some(GrantScope::MeasureStage));
        let task_cid = elohim_epr::cid::compute_cid(b"task");
        let mut n = need(Reach::Public);
        n.kind = KindId::Task;
        n.key = NeedKey::Task(task_cid);
        n.terms = NeedTerms::Task {
            input_root: CidString::from(&task_cid),
            scope: GrantScope::SweettestFeedback,
            base_root: None,
        };
        let mut c = candidate("uhCAk-a", "A", Answer::Present(Reach::Public));
        c.posture.offers.insert(
            KindId::Task,
            OfferTerms {
                scopes: GrantScope::ALL.iter().copied().collect(),
                ..OfferTerms::default()
            },
        );
        c.consent.insert(
            AgentRefKey::of(&n.receiver),
            Answer::Present(Consent::Granted {
                grant: "g".into(),
                scope: GrantScope::SweettestFeedback,
                recipient: AgentRefKey::of(&n.receiver),
                valid_from: 0,
                valid_until: 100,
                epr_scope: EprScope::All,
            }),
        );
        assert_eq!(
            admit(&lenient, &n, &c, Some(&hh("A")), &room(), 5)
                .err()
                .map(|i| i.why.why),
            Some(IneligibleReason::Kind { label: SCOPE })
        );
        // The same grant cannot be spent on the honest need either: it names the wrong scope.
        n.terms = NeedTerms::Task {
            input_root: CidString::from(&task_cid),
            scope: GrantScope::MeasureStage,
            base_root: None,
        };
        assert_eq!(
            admit(&lenient, &n, &c, Some(&hh("A")), &room(), 5)
                .err()
                .map(|i| i.why.why),
            Some(IneligibleReason::NoConsent)
        );
    }

    #[test]
    fn consent_bounds_are_checked_for_a_scoped_kind() {
        let task = BytesKind::task();
        let mut n = need(Reach::Public);
        n.kind = KindId::Task;
        let task_cid = elohim_epr::cid::compute_cid(b"task");
        n.key = NeedKey::Task(task_cid);
        n.terms = NeedTerms::Task {
            input_root: CidString::from(&task_cid),
            scope: GrantScope::MeasureStage,
            base_root: None,
        };
        let mut c = candidate("uhCAk-a", "A", Answer::Present(Reach::Public));
        c.posture.offers.insert(KindId::Task, task_offer());
        let receiver = AgentRefKey::of(&n.receiver);
        let grant = |from, until, scope: EprScope| Consent::Granted {
            grant: "g".into(),
            scope: GrantScope::MeasureStage,
            recipient: receiver.clone(),
            valid_from: from,
            valid_until: until,
            epr_scope: scope,
        };
        let run = |c: &Candidate| {
            admit(&task, &n, c, Some(&hh("A")), &room(), 50)
                .err()
                .map(|i| i.why.why.kind())
        };
        assert_eq!(
            run(&c),
            Some(IneligibleKind::Unknown),
            "no consent entry is unknown"
        );
        c.consent.insert(receiver.clone(), Answer::Absent);
        assert_eq!(run(&c), Some(IneligibleKind::NoConsent));
        c.consent.insert(
            receiver.clone(),
            Answer::Present(grant(0, 40, EprScope::All)),
        );
        assert_eq!(run(&c), Some(IneligibleKind::ConsentExpired));
        c.consent.insert(
            receiver.clone(),
            Answer::Present(grant(
                0,
                100,
                EprScope::Tasks {
                    cids: Default::default(),
                },
            )),
        );
        assert_eq!(run(&c), Some(IneligibleKind::NoConsent));
        c.consent.insert(
            receiver.clone(),
            Answer::Present(grant(
                0,
                100,
                EprScope::Tasks {
                    cids: [CidString::from(&task_cid)].into_iter().collect(),
                },
            )),
        );
        assert_eq!(run(&c), None);
    }

    #[test]
    fn not_needed_stands_only_for_the_receivers_own_carrier() {
        let task = BytesKind::task();
        let mut n = need(Reach::Public);
        n.kind = KindId::Task;
        let task_cid = elohim_epr::cid::compute_cid(b"task");
        n.key = NeedKey::Task(task_cid);
        n.terms = NeedTerms::Task {
            input_root: CidString::from(&task_cid),
            scope: GrantScope::MeasureStage,
            base_root: None,
        };
        let receiver = AgentRefKey::of(&n.receiver);
        let with_not_needed = |name: &str| {
            let mut c = candidate(name, "A", Answer::Present(Reach::Public));
            c.posture.offers.insert(KindId::Task, task_offer());
            c.consent
                .insert(receiver.clone(), Answer::Present(Consent::NotNeeded));
            c
        };
        let run = |c: &Candidate| {
            admit(&task, &n, c, Some(&hh("A")), &room(), 50)
                .err()
                .map(|i| i.why.why)
        };
        assert_eq!(
            run(&with_not_needed("uhCAk-a")),
            Some(IneligibleReason::NoConsent),
            "NotNeeded never stands in for another carrier's grant"
        );
        assert_eq!(
            run(&with_not_needed("uhCAk-r")),
            None,
            "the requester's own envelope needs no grant"
        );
        // A kind that needs no grant ignores the consent map entirely.
        let custody = BytesKind::custody(KindId::CustodyRoot);
        let plain = need(Reach::Public);
        let mut c = candidate("uhCAk-a", "A", Answer::Present(Reach::Public));
        c.consent
            .insert(receiver.clone(), Answer::Present(Consent::NotNeeded));
        assert!(admit(&custody, &plain, &c, Some(&hh("A")), &room(), 50).is_ok());
    }

    #[test]
    fn both_faces_yield_the_same_first_refusal() {
        let kind = BytesKind::custody(KindId::CustodyRoot);
        let n = need(Reach::Intimate);
        let cases = [
            (
                candidate("uhCAk-a", "A", Answer::Present(Reach::Intimate)),
                Some(hh("A")),
                room(),
            ),
            (
                candidate("uhCAk-b", "B", Answer::Present(Reach::Intimate)),
                Some(hh("A")),
                room(),
            ),
            (
                candidate("uhCAk-r", "A", Answer::Present(Reach::Intimate)),
                Some(hh("A")),
                room(),
            ),
            (
                candidate("uhCAk-a", "A", Answer::Absent),
                Some(hh("A")),
                room(),
            ),
            (
                candidate("uhCAk-a", "A", Answer::Present(Reach::Intimate)),
                Some(hh("A")),
                Capacity {
                    bytes: Some(1),
                    ..Capacity::default()
                },
            ),
        ];
        for (c, domain, remaining) in &cases {
            let typed = admit(&kind, &n, c, domain.as_ref(), remaining, 9)
                .err()
                .map(|i| i.why.why.kind());
            let face = Eligibility::new(&kind);
            let req = Candidacy {
                need: &n,
                candidate: c,
                receiver_domain: domain.as_ref(),
            };
            let elevated = face
                .decide(&req, &consent_for(&n, c), &(*remaining, 9), 9)
                .err()
                .map(|r| match r.code {
                    RefusalCode::GateRefused(label) => label,
                    other => panic!("{other:?}"),
                });
            assert_eq!(typed.map(|k| k.label().to_string()), elevated);
        }
    }

    #[test]
    fn every_refusal_carries_r7s_three_parts() {
        seam_contracts::assert_reason_labels_conformant::<IneligibleKind>();
        for r in [
            IneligibleReason::NoPosture,
            IneligibleReason::Unknown { input: "liveness" },
            IneligibleReason::OverCapacity,
            IneligibleReason::NoConsent,
            IneligibleReason::IsReceiver,
            IneligibleReason::CrossHouseholdBelowPublic,
        ] {
            let u = r.clone().unanswered(0);
            assert_eq!(u.why, r);
            assert!(matches!(u.come_back, ComeBack::OnEvent { .. }));
            assert!(u.escalation.is_none());
        }
    }

    #[test]
    fn refusals_name_whose_line_they_honored() {
        assert_eq!(
            IneligibleReason::NoConsent.refusal().limit_owner,
            LimitOwner::Commitment
        );
        assert_eq!(
            IneligibleReason::NoPosture.refusal().limit_owner,
            LimitOwner::SelfLimit
        );
        assert_eq!(
            IneligibleReason::OutsideReach {
                need: Reach::Intimate,
                admitted: Reach::Public
            }
            .refusal()
            .limit_owner,
            LimitOwner::Charter,
            "a space's membrane is never an operator veto"
        );
        // A person's own household-only offer is their own line, never an operator veto.
        for label in OWN_OFFER_LABELS {
            assert_eq!(
                IneligibleReason::Kind { label }.refusal().limit_owner,
                LimitOwner::SelfLimit,
                "{label}"
            );
        }
        let away = candidate("uhCAk-b", "B", Answer::Present(Reach::Public));
        let mut away = away;
        away.posture
            .offers
            .get_mut(&KindId::CustodyRoot)
            .unwrap()
            .household_only = true;
        let refusal = admit(
            &BytesKind::custody(KindId::CustodyRoot),
            &need(Reach::Public),
            &away,
            Some(&hh("A")),
            &room(),
            5,
        )
        .unwrap_err();
        assert_eq!(
            refusal.why.why,
            IneligibleReason::Kind {
                label: HOUSEHOLD_ONLY
            }
        );
        assert_eq!(refusal.why.why.refusal().limit_owner, LimitOwner::SelfLimit);
        assert_eq!(
            IneligibleReason::Kind { label: "other" }
                .refusal()
                .limit_owner,
            LimitOwner::Operator
        );
        assert!(matches!(
            IneligibleReason::OverCapacity.refusal().code,
            RefusalCode::GateRefused(ref l) if l == "over-capacity"
        ));
    }
}
