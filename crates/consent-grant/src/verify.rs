//! What a terminal checks before it uses what it collected.
//!
//! A terminal redeems its code at an approving node it may never have spoken to before.
//! Before it signs anything of its own over what came back, it checks that the
//! consent is the one it asked for and that every signature on it is real:
//!
//! - the record hashes to its stated address;
//! - the record was made for this terminal's request: same device, client,
//!   networks, acts asked and PKCE challenge, and nothing agreed beyond them;
//! - at least one controller signed, and every signature verifies by the key
//!   that claims it over the consent message;
//! - when enrolling was agreed, the enrollment is the one the record agreed to
//!   and every controller proof on it verifies over its intent's bytes.
//!
//! A valid signature says the key that claims it signed. Whether that key is
//! one of the identity's controllers is the network's to say: the zome checks
//! it when the device submits its binding.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use crate::act::RequestedAct;
use crate::ceremony::Delivered;
use crate::consent::{consent_message, SignerRole};
use crate::enrollment::EnrollmentIntent;
use crate::hash_shape::agent_public_key;
use crate::request::GrantRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveredRefusal {
    /// The record does not hash to its stated address.
    AddressBroken,
    /// The record was made for some other request.
    NotThisRequest,
    /// The record says more was agreed than was asked.
    AgreedBeyondRequest,
    /// No controller signed it.
    ControllerUnsigned,
    /// A signature on the consent does not verify.
    SignatureInvalid,
    /// Enrolling was agreed but no matching enrollment came with it, or one
    /// came that the record did not agree to.
    EnrollmentMismatch,
    /// A controller proof on the enrollment does not verify.
    EnrollmentSignatureInvalid,
    /// The approving node the device expected (declared, or proven over the
    /// carrier) did not sign: some other key approved.
    NotTheExpectedApprover,
}

impl DeliveredRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::AddressBroken => "delivered_address_broken",
            Self::NotThisRequest => "delivered_not_this_request",
            Self::AgreedBeyondRequest => "delivered_agreed_beyond_request",
            Self::ControllerUnsigned => "delivered_controller_unsigned",
            Self::SignatureInvalid => "delivered_signature_invalid",
            Self::EnrollmentMismatch => "delivered_enrollment_mismatch",
            Self::EnrollmentSignatureInvalid => "delivered_enrollment_signature_invalid",
            Self::NotTheExpectedApprover => "delivered_not_the_expected_approver",
        }
    }
}

fn verifies(signer: &str, message: &[u8], signature: &str) -> bool {
    let (Some(key), Ok(raw)) = (agent_public_key(signer), URL_SAFE_NO_PAD.decode(signature)) else {
        return false;
    };
    elohim_epr::proof::verify(&key, message, &raw)
}

/// Check what was collected for `asked` before anything is signed over it.
/// `approver` is the key the device expects to have approved, when it knows
/// one: the one it declared, or the one whose proof came with the code over
/// the carrier. That key must then be among the consent's controller signers
/// and the enrollment's approvers (adversarial review, high 3: a rogue node
/// on the network cannot hand back a consent of its own making).
pub fn check_delivered(
    delivered: &Delivered,
    asked: &GrantRequest,
    approver: Option<&str>,
) -> Result<(), DeliveredRefusal> {
    use DeliveredRefusal as R;

    let consent = &delivered.consent;
    if !consent.address_holds() {
        return Err(R::AddressBroken);
    }
    let r = &consent.record;
    if r.request_binding != asked.code_challenge
        || r.device_key != asked.device_key
        || r.client_id != asked.client_id
        || r.network_dna != asked.network_dna
        || r.content_dna != asked.content_dna
        || r.asked_acts != asked.acts
    {
        return Err(R::NotThisRequest);
    }
    if !r.agreed_acts.iter().all(|a| asked.acts.contains(a)) {
        return Err(R::AgreedBeyondRequest);
    }
    let binds_root = r.agreed_acts.contains(&RequestedAct::BindDeviceRoot);
    if binds_root && r.device_root_key != asked.device_root_key
        || !binds_root && r.device_root_key.is_some()
    {
        return Err(R::NotThisRequest);
    }
    if !consent.controller_signed() {
        return Err(R::ControllerUnsigned);
    }
    let message = consent_message(&consent.cid);
    if !consent
        .signatures
        .iter()
        .all(|s| verifies(&s.signer, &message, &s.signature))
    {
        return Err(R::SignatureInvalid);
    }
    if let Some(expected) = approver {
        let signed_consent = consent
            .signatures
            .iter()
            .any(|s| s.role == SignerRole::Controller && s.signer == expected);
        let signed_enrollment = delivered
            .enrollment
            .as_ref()
            .is_none_or(|e| e.controllers.iter().any(|p| p.agent == expected));
        if !signed_consent || !signed_enrollment {
            return Err(R::NotTheExpectedApprover);
        }
    }
    match (&delivered.enrollment, EnrollmentIntent::agreed_in(r)) {
        (None, None) => Ok(()),
        (Some(enrollment), Some(_)) if enrollment.is_agreed_in(r) => {
            let bytes = enrollment
                .intent
                .signed_bytes()
                .ok_or(R::EnrollmentMismatch)?;
            if enrollment
                .controllers
                .iter()
                .all(|p| verifies(&p.agent, &bytes, &p.signature))
            {
                Ok(())
            } else {
                Err(R::EnrollmentSignatureInvalid)
            }
        }
        _ => Err(R::EnrollmentMismatch),
    }
}

/// The signer roles a delivered consent carries, for a terminal to report.
pub fn signers(delivered: &Delivered, role: SignerRole) -> Vec<&str> {
    delivered
        .consent
        .signatures
        .iter()
        .filter(|s| s.role == role)
        .map(|s| s.signer.as_str())
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::consent::tests::AT;
    use crate::consent::{ConsentRecord, ConsentSignature, SignedConsent};
    use crate::enrollment::{ControllerProof, Enrollment};
    use crate::request::admit_request;
    use crate::request::tests::{peer_request, policy};
    use elohim_epr::proof::{sign, AgentKeypair};

    /// An agent key whose ed25519 key is `keypair`'s.
    pub(crate) fn agent_key(keypair: &AgentKeypair) -> String {
        let mut bytes = vec![0x84, 0x20, 0x24];
        bytes.extend_from_slice(&keypair.public_key_bytes());
        bytes.extend_from_slice(&[0; 4]);
        format!("u{}", URL_SAFE_NO_PAD.encode(bytes))
    }

    fn controller() -> AgentKeypair {
        AgentKeypair::from_secret(&[9; 32]).unwrap()
    }

    /// What an honest approving node hands back for `peer_request`.
    fn delivered() -> Delivered {
        let admitted = admit_request(&peer_request(), &policy()).unwrap();
        let acts = admitted.request().acts.clone();
        let record =
            ConsentRecord::agree(&admitted, crate::consent::tests::agreement(acts)).unwrap();
        let key = controller();
        let signer = agent_key(&key);
        let consent = SignedConsent::new(record.clone()).unwrap();
        let consent = consent.clone().with_signature(ConsentSignature {
            role: SignerRole::Controller,
            signer: signer.clone(),
            signature: URL_SAFE_NO_PAD.encode(sign(&key, &consent.message())),
        });
        let intent = EnrollmentIntent::agreed_in(&record).unwrap();
        let proof = sign(&key, &intent.signed_bytes().unwrap());
        Delivered {
            consent,
            enrollment: Some(Enrollment {
                intent,
                controllers: vec![ControllerProof {
                    agent: signer,
                    signature: URL_SAFE_NO_PAD.encode(proof),
                }],
                approved_via: vec![],
            }),
        }
    }

    #[test]
    fn an_honest_delivery_checks() {
        assert_eq!(check_delivered(&delivered(), &peer_request(), None), Ok(()));
        assert_eq!(signers(&delivered(), SignerRole::Controller).len(), 1);
        assert_eq!(delivered().consent.record.agreed_at_micros, AT);
    }

    #[test]
    fn each_way_a_delivery_can_be_wrong_is_refused_by_name() {
        use DeliveredRefusal as R;
        let refused = |change: &dyn Fn(&mut Delivered)| {
            let mut d = delivered();
            change(&mut d);
            check_delivered(&d, &peer_request(), None).unwrap_err()
        };
        assert_eq!(
            refused(&|d| d.consent.record.agreed_at_micros += 1),
            R::AddressBroken
        );
        assert_eq!(
            refused(&|d| d.consent.signatures[0].signature = "AAAA".into()),
            R::SignatureInvalid
        );
        assert_eq!(
            refused(&|d| d.consent.signatures.clear()),
            R::ControllerUnsigned
        );
        assert_eq!(refused(&|d| d.enrollment = None), R::EnrollmentMismatch);
        assert_eq!(
            refused(&|d| {
                let e = d.enrollment.as_mut().unwrap();
                e.controllers[0].signature = e.controllers[0].signature.chars().rev().collect();
            }),
            R::EnrollmentSignatureInvalid
        );

        // A consent signature cannot stand in for an enrollment proof.
        let mut swapped = delivered();
        let consent_sig = swapped.consent.signatures[0].signature.clone();
        swapped.enrollment.as_mut().unwrap().controllers[0].signature = consent_sig;
        assert_eq!(
            check_delivered(&swapped, &peer_request(), None),
            Err(R::EnrollmentSignatureInvalid)
        );

        // Adversarial review, high 3: the device expected another approver; a
        // consent this key made is not that one's.
        let signer = delivered().consent.signatures[0].signer.clone();
        assert_eq!(
            check_delivered(&delivered(), &peer_request(), Some(&signer)),
            Ok(())
        );
        let (_, rogue) = crate::pending::tests::keypair(77);
        assert_eq!(
            check_delivered(&delivered(), &peer_request(), Some(&rogue)),
            Err(R::NotTheExpectedApprover)
        );

        // Another terminal's request.
        let mut other = peer_request();
        other.code_challenge = crate::pkce::challenge(&"v".repeat(43));
        assert_eq!(
            check_delivered(&delivered(), &other, None),
            Err(R::NotThisRequest)
        );
    }
}
