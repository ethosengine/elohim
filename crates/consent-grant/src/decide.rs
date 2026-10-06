//! Who decides an ask: the decision seam.
//!
//! An ask waiting on an approving node is decided by one of interchangeable
//! deciders, tried in order until one decides. Approving a device is an act
//! for the person's identity, so every decider here decides for the person
//! the node speaks for, never for whoever operates the machine:
//!
//! - [`NoElohimAttending`]: an elohim attending the person. On a mature
//!   network an elohim decides with no person present. None attends yet, so
//!   the default defers to the others, as the witness beat's default passes a
//!   consent through. A decider that cannot decide defers; it never fails the
//!   ask.
//! - [`ByDeclaration`]: what the person wrote down in advance. It agrees only
//!   for a node with no identity of its own whose key the person declared,
//!   asking no more than the declared acts.
//! - [`ByAnswer`]: an answer given at the approving node, from its terminal or
//!   its portal, taken as the person's. A signed-in device acts with the
//!   person's authority, as in OAuth; the person is not asked for proof on
//!   each act. Noticing that something is off, and asking the person to sign
//!   in again, is a witness's job at the witnessed moment
//!   ([`crate::witness`]).
//!
//! A decision is not a signature. Agreeing still happens on the deciding
//! node's own machine, through the agree path and its rules.
//!
//! A node that already has an identity of its own is never decided by
//! declaration here. Joining it keeps its key and leaves what it made as it
//! was; that is the only choice offered, and it goes to whoever answers, with
//! the reason.

use serde::{Deserialize, Serialize};

use crate::act::RequestedAct;
use crate::declaration::Declaration;
use crate::pending::{NodeState, PendingAsk};

/// Who decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DecidedBy {
    Elohim,
    Declaration,
    Answer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Agree {
        acts: Vec<RequestedAct>,
        by: DecidedBy,
    },
    Decline {
        by: DecidedBy,
    },
    /// This decider leaves it to the next, and says why.
    Defer {
        reason: String,
    },
}

/// One way an ask can be decided.
pub trait Decider: Send + Sync {
    fn decide(&self, ask: &PendingAsk) -> Decision;
}

/// No elohim attends this node. Always defers.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoElohimAttending;

impl Decider for NoElohimAttending {
    fn decide(&self, _ask: &PendingAsk) -> Decision {
        Decision::Defer {
            reason: "no elohim attends this node".into(),
        }
    }
}

/// What the person declared in advance.
pub struct ByDeclaration<'a>(pub &'a Declaration);

/// Why a node with an identity of its own goes to whoever answers.
pub const OWN_IDENTITY_REASON: &str =
    "the asking device already has an identity of its own, so it is not \
    approved by declaration; joining it keeps its key and leaves what it made as it was";

impl Decider for ByDeclaration<'_> {
    fn decide(&self, ask: &PendingAsk) -> Decision {
        let Some((_, acts)) = self.0.agreed_for(&ask.ask.request) else {
            return Decision::Defer {
                reason: "this device is not declared for what it asks".into(),
            };
        };
        match ask.ask.state {
            NodeState::Unassigned => Decision::Agree {
                acts,
                by: DecidedBy::Declaration,
            },
            NodeState::OwnIdentity { .. } => Decision::Defer {
                reason: OWN_IDENTITY_REASON.into(),
            },
            NodeState::Joined => Decision::Defer {
                reason: "the asking device already speaks for someone, so it is not approved by \
                         declaration"
                    .into(),
            },
        }
    }
}

/// An answer given at the approving node, taken as the person's: the acts
/// agreed, none meaning decline.
pub struct ByAnswer(pub Vec<RequestedAct>);

impl Decider for ByAnswer {
    fn decide(&self, ask: &PendingAsk) -> Decision {
        if self.0.is_empty() {
            return Decision::Decline {
                by: DecidedBy::Answer,
            };
        }
        if !self.0.iter().all(|a| ask.ask.request.acts.contains(a)) {
            return Decision::Defer {
                reason: "the answer agrees to something the device did not ask".into(),
            };
        }
        Decision::Agree {
            acts: self.0.clone(),
            by: DecidedBy::Answer,
        }
    }
}

/// Try `deciders` in order. The first that decides wins; if all defer, the
/// reasons say why, in order.
pub fn decide(deciders: &[&dyn Decider], ask: &PendingAsk) -> Result<Decision, Vec<String>> {
    let mut reasons = Vec::new();
    for decider in deciders {
        match decider.decide(ask) {
            Decision::Defer { reason } => reasons.push(reason),
            decided => return Ok(decided),
        }
    }
    Err(reasons)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declaration::DeclaredDevice;
    use crate::pending::tests::{ask, speaks};
    use crate::pending::{Made, PendingAsks};
    use crate::request::tests::policy;

    fn pending(state: NodeState) -> PendingAsk {
        let list = PendingAsks::new();
        let mut a = ask(1);
        a.state = state;
        list.admit(a, "peer-a", None, &speaks(), &policy(), 0)
            .unwrap();
        list.list(0).remove(0)
    }

    fn declared() -> Declaration {
        Declaration {
            identity: None,
            devices: vec![DeclaredDevice {
                label: "home".into(),
                device_key: ask(1).request.device_key,
                acts: vec![RequestedAct::EnrollDevice],
            }],
            asking: None,
        }
    }

    #[test]
    fn by_default_no_elohim_attends_and_the_others_decide() {
        let d = declared();
        let chain: [&dyn Decider; 2] = [&NoElohimAttending, &ByDeclaration(&d)];
        assert_eq!(
            decide(&chain, &pending(NodeState::Unassigned)),
            Ok(Decision::Agree {
                acts: vec![RequestedAct::EnrollDevice],
                by: DecidedBy::Declaration
            })
        );
        assert!(matches!(
            NoElohimAttending.decide(&pending(NodeState::Unassigned)),
            Decision::Defer { .. }
        ));
    }

    #[test]
    fn a_node_with_an_identity_of_its_own_goes_to_whoever_answers() {
        let d = declared();
        let chain: [&dyn Decider; 2] = [&NoElohimAttending, &ByDeclaration(&d)];
        for state in [
            NodeState::OwnIdentity {
                made: Made::Unknown,
            },
            NodeState::OwnIdentity { made: Made::Things },
            NodeState::Joined,
        ] {
            let reasons = decide(&chain, &pending(state)).unwrap_err();
            assert_eq!(reasons.len(), 2, "{state:?}");
            assert!(
                reasons[1].contains("not approved by declaration"),
                "{reasons:?}"
            );
        }
        let answer = ByAnswer(vec![RequestedAct::EnrollDevice]);
        let chain: [&dyn Decider; 3] = [&NoElohimAttending, &ByDeclaration(&d), &answer];
        assert!(matches!(
            decide(
                &chain,
                &pending(NodeState::OwnIdentity {
                    made: Made::Unknown
                })
            ),
            Ok(Decision::Agree {
                by: DecidedBy::Answer,
                ..
            })
        ));
    }

    #[test]
    fn an_undeclared_device_waits_for_an_answer_and_an_empty_answer_declines() {
        let empty = Declaration::default();
        let chain: [&dyn Decider; 2] = [&NoElohimAttending, &ByDeclaration(&empty)];
        assert!(decide(&chain, &pending(NodeState::Unassigned)).is_err());
        assert_eq!(
            ByAnswer(vec![]).decide(&pending(NodeState::Unassigned)),
            Decision::Decline {
                by: DecidedBy::Answer
            }
        );
        assert!(matches!(
            ByAnswer(vec![RequestedAct::BindDeviceRoot]).decide(&pending(NodeState::Unassigned)),
            Decision::Defer { .. }
        ));
    }
}
