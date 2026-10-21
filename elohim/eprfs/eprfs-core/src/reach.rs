//! Reach on both sides of a tier, as injected policy (interfaces spec §A.3).
//!
//! eprfs-core never names a reach value. A serving tier asks a [`ReachGate`] whether a requester
//! may read under a root; a requesting chain asks an [`AskPolicy`] whether a tier may be asked at
//! all. Both are implemented in hosts, which hold the records reach is judged from. The core sees
//! only their verdicts, and carries a root's reach as an opaque ordinal, [`ReachLevel`].

use serde::{Deserialize, Serialize};

use crate::address::BlobCid;
use crate::tier::{TierClass, TierRecord};

/// Who asks, as the serving transport authenticated them. Built only at a transport adapter; the
/// seam registry pins the construction sites. Device facts never appear here (R6).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Requester {
    agent: Option<String>,
    custody: KeyCustody,
}

impl Requester {
    /// A requester whose agent key the transport authenticated, with how that key is held.
    pub fn authenticated(agent_key: impl Into<String>, custody: KeyCustody) -> Self {
        Self {
            agent: Some(agent_key.into()),
            custody,
        }
    }

    /// A requester the transport could not authenticate.
    pub fn anonymous() -> Self {
        Self {
            agent: None,
            custody: KeyCustody::Anonymous,
        }
    }

    pub fn agent(&self) -> Option<&str> {
        self.agent.as_deref()
    }

    pub fn custody(&self) -> KeyCustody {
        self.custody
    }
}

/// Spec §10.11: the actor's key-custody tag rides with every reach check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyCustody {
    DeviceHeld,
    HubHeld,
    DoorwayHeld,
    SponsoredAgent,
    Anonymous,
}

/// Every non-admission is a refusal; there is no admit-on-error path (spec §18). To a requester
/// outside reach, a missing object and a private one get the same refusal (spec §12.3 rule 2).
/// This is the local gate's diagnostic shape. Serving endpoints normalize it to `OutsideStanding`
/// with no standing label before replying, so record existence and private group names stay local.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReachRefusal {
    pub reason: ReachReason,
    /// The standing the requester lacks, opaque to eprfs.
    pub needs_standing: Option<String>,
}

impl ReachRefusal {
    pub fn new(reason: ReachReason, needs_standing: Option<String>) -> Self {
        Self {
            reason,
            needs_standing,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReachReason {
    OutsideStanding,
    NoRecord,
    NotUnderRoot,
    IdentityRequired,
    AuthorizerUnavailable,
}

/// Serving side. Tree nodes inherit their root's reach (spec §12.3 rule 2); an object outside
/// every root is judged by its own records. Synchronous: reach is read from records the tier
/// already holds, and an unknown record refuses (`NoRecord`) rather than fetching.
pub trait ReachGate: Send + Sync {
    fn admit_root(&self, who: &Requester, root: &BlobCid) -> Result<(), ReachRefusal>;
    fn admit_object(&self, who: &Requester, cid: &BlobCid) -> Result<(), ReachRefusal>;
    /// The root's reach as this tier's own records give it, so a serving tier forwarding a miss
    /// hands its `AskPolicy` the reach it judged, never one read from the wire. `None` (the
    /// default) is unknown, which an `AskPolicy` treats narrowly. (Revision of interfaces spec
    /// §A.3: the serving side had no source for `BatchOptions::root_reach`.)
    fn root_level(&self, root: &BlobCid) -> Option<ReachLevel> {
        let _ = root;
        None
    }
}

/// Requester side: may this tier be ASKED for objects under this root? The read pattern is itself
/// a signal, so a private root's miss never leaves the tiers its reach admits (spec §12.3 rule 2).
/// The root's reach is the host's; eprfs passes it opaquely.
pub trait AskPolicy: Send + Sync {
    fn may_ask(&self, tier: &TierRecord, root: &BlobCid, root_reach: Option<ReachLevel>) -> bool;
}

/// `Reach::openness()` as an opaque ordinal, so eprfs-core names no reach value. Larger is more
/// open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReachLevel(pub u8);

impl ReachLevel {
    /// The reach of an answer drawn from two roots (a diff): the narrower of the two, or unknown
    /// when either is unknown, which an [`AskPolicy`] treats narrowly.
    pub fn narrower(a: Option<ReachLevel>, b: Option<ReachLevel>) -> Option<ReachLevel> {
        match (a, b) {
            (Some(a), Some(b)) => Some(a.min(b)),
            _ => None,
        }
    }
}

/// The requester-side rule a host's `ReachStandingAsk` applies (interfaces spec §A.6), once, so
/// the browser, the native hosts and the test harnesses cannot drift on the rule that keeps a
/// private root's read pattern on the tiers its reach admits. Parameterized by the two openness
/// levels it defaults to, so eprfs-core still names no reach value (R-E1, R-E2): ask a tier only if
/// the root's openness is at least the tier's standing; a tier of unknown standing defaults to
/// `household` for the close classes (`Own`, `PersonDevice`, `Household`) and to `public`
/// otherwise; with the root's reach unknown, only the close classes are asked. The TypeScript twin
/// is `standingPolicy`; the shared vectors' `ask` section pins both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpennessAsk {
    pub household: ReachLevel,
    pub public: ReachLevel,
}

impl OpennessAsk {
    fn close(class: TierClass) -> bool {
        matches!(
            class,
            TierClass::Own | TierClass::PersonDevice | TierClass::Household
        )
    }
}

impl AskPolicy for OpennessAsk {
    fn may_ask(&self, tier: &TierRecord, _root: &BlobCid, root_reach: Option<ReachLevel>) -> bool {
        match root_reach {
            None => Self::close(tier.class),
            Some(root) => {
                let standing = tier.standing.unwrap_or(if Self::close(tier.class) {
                    self.household
                } else {
                    self.public
                });
                root >= standing
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anonymous_requester_has_no_agent_and_anonymous_custody() {
        let anon = Requester::anonymous();
        assert_eq!(anon.agent(), None);
        assert_eq!(anon.custody(), KeyCustody::Anonymous);
        let me = Requester::authenticated("uhCAkme", KeyCustody::DeviceHeld);
        assert_eq!(me.agent(), Some("uhCAkme"));
        assert_eq!(me.custody(), KeyCustody::DeviceHeld);
    }

    #[test]
    fn refusal_wire_form_is_camel_case_with_kebab_reason() {
        let refusal = ReachRefusal::new(ReachReason::OutsideStanding, Some("household".into()));
        assert_eq!(
            serde_json::to_value(&refusal).unwrap(),
            serde_json::json!({"reason": "outside-standing", "needsStanding": "household"})
        );
        assert_eq!(
            serde_json::to_value(KeyCustody::SponsoredAgent).unwrap(),
            serde_json::json!("sponsored-agent")
        );
    }
}
