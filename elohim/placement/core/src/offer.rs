//! D.3 — postures, offers and candidates. An offer is an intent to provide (spec §7.1). Device facts
//! never leave the device; only their effect crosses, as a narrowed, deferred or withdrawn offer.
//!
//! Rail R-P4: [`DeviceFacts`] does not serialize, so its only way off the device is
//! [`DeviceFacts::posture`]:
//!
//! ```compile_fail
//! use placement_core::DeviceFacts;
//! let facts = DeviceFacts {
//!     on_battery: true, metered: false, always_on: false,
//!     can_steward: true, can_infer: false, budget_bytes: 0,
//! };
//! let _ = serde_json::to_string(&facts);
//! ```
//!
//! The same call over the posture it produces compiles:
//!
//! ```
//! use placement_core::{DeviceFacts, PersonChoices};
//! use elohim_epr::Reach;
//! use elohim_epr_rea::AgentRef;
//! let facts = DeviceFacts {
//!     on_battery: true, metered: false, always_on: false,
//!     can_steward: true, can_infer: false, budget_bytes: 0,
//! };
//! let choices = PersonChoices { will_carry: Default::default(), lift_battery_default: false, budget_bytes: None };
//! let posture = facts.posture(AgentRef("uhCAk-me".into()), &choices, Reach::Intimate, 0);
//! let _ = serde_json::to_string(&posture).unwrap();
//! ```

use std::collections::{BTreeMap, BTreeSet};

use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::AgentRef;
use seam_contracts::Answer;
use serde::{Deserialize, Serialize};

use crate::kind::{GrantScope, KindId};
use crate::need::{AgentRefKey, Isolation};
use crate::params::PlaceError;

pub type EpochSecs = u64;

/// A carrier's public offer. Written when it changes, with hysteresis, never on a timer (spec
/// §7.1). Its liveness is read by the observer, never assumed from the posture (D3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Posture {
    pub carrier: AgentRef,
    pub offers: BTreeMap<KindId, OfferTerms>,
    /// How far the posture is visible.
    pub reach: Reach,
    pub written_at: EpochSecs,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferTerms {
    pub spare_bytes: Option<u64>,
    pub compute: Option<Capacity>,
    /// Roots kept at depth 3, each listed only at a reach no narrower than the posture's (F8).
    pub holds: Vec<CidString>,
    pub reach_ceiling: Option<Reach>,
    /// F6: what a task offer will spend.
    pub scopes: BTreeSet<GrantScope>,
    /// F6.
    pub isolation: Option<Isolation>,
    /// Own-household only and deferred to a charging or unmetered window (D11): eligible for its
    /// own household's needs, charged as a lag, never as an ineligibility.
    pub deferred: bool,
    pub household_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capacity {
    pub bytes: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub cpu_millis: Option<u32>,
    pub pids: Option<u32>,
}

impl Capacity {
    /// What an offer holds out: `spare_bytes` (or the compute block's bytes) and the compute block.
    pub fn of_offer(offer: &OfferTerms) -> Self {
        let compute = offer.compute.unwrap_or_default();
        Self {
            bytes: offer.spare_bytes.or(compute.bytes),
            memory_bytes: compute.memory_bytes,
            cpu_millis: compute.cpu_millis,
            pids: compute.pids,
        }
    }

    /// What remains after a charge; an unknown (`None`) component stays unknown.
    pub(crate) fn charged(self, c: &crate::kind::Charge) -> Self {
        Self {
            bytes: self.bytes.map(|b| b.saturating_sub(c.bytes)),
            memory_bytes: self.memory_bytes.map(|b| b.saturating_sub(c.memory_bytes)),
            cpu_millis: self.cpu_millis.map(|b| b.saturating_sub(c.cpu_millis)),
            pids: self.pids.map(|b| b.saturating_sub(c.pids)),
        }
    }

    /// What an offer has with a holding's own footprint given back: a posture's spare already nets
    /// out what its carrier holds, so retaining a holding is measured against spare plus that
    /// holding, never against spare for another copy. An unknown component stays unknown.
    pub(crate) fn credited(self, c: &crate::kind::Charge) -> Self {
        Self {
            bytes: self.bytes.map(|b| b.saturating_add(c.bytes)),
            memory_bytes: self.memory_bytes.map(|b| b.saturating_add(c.memory_bytes)),
            cpu_millis: self.cpu_millis.map(|b| b.saturating_add(c.cpu_millis)),
            pids: self.pids.map(|b| b.saturating_add(c.pids)),
        }
    }
}

/// The device's own facts. NOT `Serialize` (rail R-P4). Its only way off the device is
/// [`DeviceFacts::posture`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceFacts {
    pub on_battery: bool,
    pub metered: bool,
    pub always_on: bool,
    pub can_steward: bool,
    pub can_infer: bool,
    pub budget_bytes: u64,
}

/// The person's own choices about what their device carries. Not `Serialize`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonChoices {
    pub will_carry: BTreeSet<KindId>,
    pub lift_battery_default: bool,
    pub budget_bytes: Option<u64>,
}

impl DeviceFacts {
    /// Carrying for another household or space is off by default on battery, on a metered link or
    /// when not always on; the person's own choice may lift it (spec §7.2). Holding within the
    /// device's own household is not withdrawn for battery: the custody offer stays, `household_only`
    /// and `deferred`, so a two-phone household with no hub still holds its own bytes on both phones
    /// (spec §7.2, D11). Peers see only the presence, deferral or absence of an offer.
    pub fn posture(
        &self,
        me: AgentRef,
        choices: &PersonChoices,
        reach: Reach,
        now: EpochSecs,
    ) -> Posture {
        let constrained = self.on_battery || self.metered || !self.always_on;
        let limited = constrained && !choices.lift_battery_default;
        let budget = match choices.budget_bytes {
            Some(chosen) => chosen.min(self.budget_bytes),
            None => self.budget_bytes,
        };
        let mut offers = BTreeMap::new();
        for &kind in &choices.will_carry {
            let custody = matches!(kind, KindId::CustodyBlob | KindId::CustodyRoot);
            if custody && !self.can_steward {
                continue;
            }
            if kind == KindId::Inference && !self.can_infer {
                continue;
            }
            if limited && !custody {
                continue; // withdrawn: only the absence crosses
            }
            offers.insert(
                kind,
                OfferTerms {
                    spare_bytes: custody.then_some(budget),
                    household_only: limited,
                    deferred: limited,
                    ..OfferTerms::default()
                },
            );
        }
        Posture {
            carrier: me,
            offers,
            reach,
            written_at: now,
        }
    }
}

/// What an OBSERVER resolves about a posture from records it holds, never self-report (the
/// `build_salvage_candidates` household join generalized). An unknown fact is `Unreachable`, not a
/// default. Admission and consent are per receiver, because one snapshot holds needs from many
/// receivers (D2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub posture: Posture,
    pub domain: FailureDomain,
    /// Presence or inventory freshness within the floor's freshness horizon (D3); anything but
    /// `Present(true)` is ineligible (`Unknown { input: "liveness" }`).
    pub live: Answer<bool>,
    /// receiver → narrowest circle admitted, relative to it.
    pub admitted: BTreeMap<AgentRefKey, Answer<Reach>>,
    /// receiver → R3: a grant is consent and eligibility.
    pub consent: BTreeMap<AgentRefKey, Answer<Consent>>,
}

impl Candidate {
    pub fn agent(&self) -> &AgentRef {
        &self.posture.carrier
    }
}

/// A failure domain: `hh:<household>` when the household is known, else `agent:<key>`, so an
/// unknown-household candidate is its own domain and never collapses into one bucket.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FailureDomain(String);

impl FailureDomain {
    pub fn of(household: Option<&str>, agent: &AgentRef) -> Self {
        match household {
            Some(h) => Self(format!("hh:{h}")),
            None => Self(format!("agent:{}", agent.0)),
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// The household id, when this domain is a household.
    pub fn household(&self) -> Option<&str> {
        self.0.strip_prefix("hh:")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "consent"
)]
pub enum Consent {
    NotNeeded,
    /// The grant's own bounds, as `compute_grants.rs` enforces them (D2): recipient, the validity
    /// window (`valid_from <= now < valid_until`) and its `epr_scope`, checked against `NeedKey::Task`.
    Granted {
        grant: String,
        scope: GrantScope,
        recipient: AgentRefKey,
        valid_from: EpochSecs,
        valid_until: EpochSecs,
        epr_scope: EprScope,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "scope"
)]
pub enum EprScope {
    All,
    Tasks { cids: BTreeSet<CidString> },
}

/// A CID in its string form, for set members, map keys and fields inside internally tagged enums
/// (G1). A string in every format.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CidString(String);

impl From<&Cid> for CidString {
    fn from(cid: &Cid) -> Self {
        Self(cid.to_string())
    }
}

impl CidString {
    pub fn to_cid(&self) -> Result<Cid, PlaceError> {
        Cid::try_from(self.0.as_str()).map_err(|e| PlaceError(format!("cid {:?}: {e}", self.0)))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(on_battery: bool) -> DeviceFacts {
        DeviceFacts {
            on_battery,
            metered: false,
            always_on: !on_battery,
            can_steward: true,
            can_infer: false,
            budget_bytes: 1_000,
        }
    }

    fn choices() -> PersonChoices {
        PersonChoices {
            will_carry: [KindId::CustodyRoot, KindId::Task].into_iter().collect(),
            lift_battery_default: false,
            budget_bytes: Some(400),
        }
    }

    #[test]
    fn battery_keeps_own_household_custody_deferred_and_withdraws_the_rest() {
        let p = facts(true).posture(
            AgentRef("uhCAk-phone".into()),
            &choices(),
            Reach::Intimate,
            7,
        );
        let custody = &p.offers[&KindId::CustodyRoot];
        assert!(custody.household_only && custody.deferred);
        assert_eq!(custody.spare_bytes, Some(400));
        assert!(
            !p.offers.contains_key(&KindId::Task),
            "a task offer is withdrawn on battery"
        );
        assert_eq!(p.written_at, 7);
    }

    #[test]
    fn mains_power_offers_everything_chosen() {
        let p = facts(false).posture(AgentRef("uhCAk-hub".into()), &choices(), Reach::Intimate, 0);
        assert!(!p.offers[&KindId::CustodyRoot].deferred);
        assert!(p.offers.contains_key(&KindId::Task));
    }

    #[test]
    fn the_person_may_lift_the_battery_default() {
        let mut c = choices();
        c.lift_battery_default = true;
        let p = facts(true).posture(AgentRef("uhCAk-phone".into()), &c, Reach::Intimate, 0);
        assert!(p.offers.contains_key(&KindId::Task));
        assert!(!p.offers[&KindId::CustodyRoot].household_only);
    }

    #[test]
    fn failure_domain_forms() {
        let a = AgentRef("uhCAk-a".into());
        assert_eq!(FailureDomain::of(Some("A"), &a).as_str(), "hh:A");
        assert_eq!(FailureDomain::of(Some("A"), &a).household(), Some("A"));
        assert_eq!(FailureDomain::of(None, &a).as_str(), "agent:uhCAk-a");
        assert_eq!(FailureDomain::of(None, &a).household(), None);
    }

    #[test]
    fn cid_string_round_trips() {
        let c = elohim_epr::cid::compute_cid(b"x");
        let s = CidString::from(&c);
        assert_eq!(s.to_cid().unwrap(), c);
        assert_eq!(serde_json::to_string(&s).unwrap(), format!("\"{c}\""));
        assert!(CidString("not-a-cid".into()).to_cid().is_err());
    }
}
