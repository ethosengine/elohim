//! The controller who agrees: who they are, and whether they may agree to this
//! request.
//!
//! A host reads this from the controller's own cell before it asks that cell to
//! sign. Checking it here first lets a person hear plainly why they cannot
//! agree, rather than read a refusal from the zome.
//!
//! An identity may have several controllers. By default any one of them may
//! agree: the others affirm the device later if they choose. A person who wants
//! more than one to agree before a device counts declares that in their
//! controller policy, and `required` reports it. Nothing in the ceremony waits
//! for the others.

use crate::act::RequestedAct;
use crate::consent::Agreement;
use crate::request::AdmittedRequest;

/// What the controller's cell reports about the identity it acts for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerStanding {
    /// The Human record the identity is rooted in.
    pub identity_root: String,
    /// The identity's current authority record.
    pub authority: String,
    /// The network the authority is kept on.
    pub network_dna: String,
    /// Every controller the authority names.
    pub controllers: Vec<String>,
    /// How many controllers the authority's declared policy asks to agree.
    pub required: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandingRefusal {
    /// The request names a network this identity is not kept on.
    NetworkForeign,
    /// The key that would sign is not one of the identity's controllers.
    NotAController,
}

impl StandingRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::NetworkForeign => "consent_network_foreign",
            Self::NotAController => "consent_not_a_controller",
        }
    }
}

impl ControllerStanding {
    /// Whether `signer` may agree to `admitted` for this identity.
    pub fn admits(&self, admitted: &AdmittedRequest, signer: &str) -> Result<(), StandingRefusal> {
        if admitted.request().network_dna != self.network_dna {
            return Err(StandingRefusal::NetworkForeign);
        }
        if !self.controllers.iter().any(|c| c == signer) {
            return Err(StandingRefusal::NotAController);
        }
        Ok(())
    }

    /// The agreement this controller makes, for [`crate::ConsentRecord::agree`].
    pub fn agreement(&self, agreed_acts: Vec<RequestedAct>, agreed_at_micros: i64) -> Agreement {
        Agreement {
            identity_root: self.identity_root.clone(),
            authority: self.authority.clone(),
            agreed_acts,
            agreed_at_micros,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash_shape::{sample_action, sample_key};
    use crate::request::admit_request;
    use crate::request::tests::{policy, request, CONTENT, NETWORK};

    fn standing() -> ControllerStanding {
        ControllerStanding {
            identity_root: sample_action(1),
            authority: sample_action(2),
            network_dna: NETWORK.into(),
            controllers: vec![sample_key(9), sample_key(10)],
            required: 1,
        }
    }

    #[test]
    fn any_named_controller_may_agree_on_the_identitys_network() {
        let admitted = admit_request(&request(), &policy()).unwrap();
        assert_eq!(standing().admits(&admitted, &sample_key(9)), Ok(()));
        assert_eq!(standing().admits(&admitted, &sample_key(10)), Ok(()));
        assert_eq!(
            standing().admits(&admitted, &sample_key(7)),
            Err(StandingRefusal::NotAController)
        );
        let mut elsewhere = request();
        elsewhere.network_dna = CONTENT.into();
        let elsewhere = admit_request(&elsewhere, &policy()).unwrap();
        assert_eq!(
            standing().admits(&elsewhere, &sample_key(9)),
            Err(StandingRefusal::NetworkForeign)
        );
    }

    #[test]
    fn the_agreement_names_the_standing_identity() {
        let a = standing().agreement(vec![RequestedAct::EnrollDevice], 5);
        assert_eq!(a.identity_root, sample_action(1));
        assert_eq!(a.authority, sample_action(2));
        assert_eq!(a.agreed_at_micros, 5);
    }
}
