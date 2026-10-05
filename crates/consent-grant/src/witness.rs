//! The witness beat: the one moment in the ceremony where someone other than
//! the person and the device may attend.
//!
//! It falls after the controller has signed and before the code is issued. A
//! peer attending the person (an elohim) will one day pause here, look at what
//! is being agreed, add what it knows to the record's provenance and sign as a
//! witness. Today nobody attends and the ceremony passes straight through.
//!
//! A witness strengthens a consent and never gates it. So the beat cannot
//! fail the ceremony, cannot change what was agreed, and cannot remove or
//! replace anyone else's signature: [`attend`] keeps only the witness
//! signatures a beat adds.

use crate::consent::{SignedConsent, SignerRole};
use crate::request::AdmittedRequest;

/// Something that may witness a consent as it is given.
///
/// Implementations receive the admitted request and the controller-signed
/// consent, and return the consent with any [`SignerRole::Witness`] signatures
/// they add. They must not block the person: a witness that cannot attend
/// returns the consent as it came.
pub trait WitnessBeat: Send + Sync {
    fn attend(&self, admitted: &AdmittedRequest, consent: SignedConsent) -> SignedConsent;
}

/// Nobody attends. The consent passes through unchanged, at once.
#[derive(Debug, Default, Clone, Copy)]
pub struct Unattended;

impl WitnessBeat for Unattended {
    fn attend(&self, _admitted: &AdmittedRequest, consent: SignedConsent) -> SignedConsent {
        consent
    }
}

/// Run the beat, keeping from what it returns only new witness signatures on
/// the same record. Anything else it returned is discarded, so a beat that
/// misbehaves costs the consent nothing.
pub fn attend(
    beat: &dyn WitnessBeat,
    admitted: &AdmittedRequest,
    consent: SignedConsent,
) -> SignedConsent {
    let returned = beat.attend(admitted, consent.clone());
    if returned.cid != consent.cid || returned.record != consent.record {
        return consent;
    }
    let added: Vec<_> = returned
        .signatures
        .into_iter()
        .filter(|s| s.role == SignerRole::Witness && !consent.signatures.contains(s))
        .collect();
    added
        .into_iter()
        .fold(consent, SignedConsent::with_signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consent::tests::peer_record;
    use crate::consent::ConsentSignature;
    use crate::hash_shape::sample_key;
    use crate::request::admit_request;
    use crate::request::tests::{peer_request, policy};
    use crate::RequestedAct;

    fn admitted() -> AdmittedRequest {
        admit_request(&peer_request(), &policy()).unwrap()
    }

    fn signature(role: SignerRole, who: u8) -> ConsentSignature {
        ConsentSignature {
            role,
            signer: sample_key(who),
            signature: format!("{role:?}-{who}"),
        }
    }

    fn signed() -> SignedConsent {
        SignedConsent::new(peer_record())
            .unwrap()
            .with_signature(signature(SignerRole::Controller, 9))
    }

    struct Beat<F: Fn(SignedConsent) -> SignedConsent + Send + Sync>(F);
    impl<F: Fn(SignedConsent) -> SignedConsent + Send + Sync> WitnessBeat for Beat<F> {
        fn attend(&self, _: &AdmittedRequest, consent: SignedConsent) -> SignedConsent {
            (self.0)(consent)
        }
    }

    #[test]
    fn by_default_nobody_attends_and_nothing_is_added() {
        let out = attend(&Unattended, &admitted(), signed());
        assert_eq!(out, signed());
        assert_eq!(out.signed_in(SignerRole::Witness), 0);
    }

    #[test]
    fn a_witness_adds_its_signature_after_the_controllers() {
        let beat = Beat(|c: SignedConsent| c.with_signature(signature(SignerRole::Witness, 5)));
        let out = attend(&beat, &admitted(), signed());
        assert_eq!(out.signatures.len(), 2);
        assert_eq!(out.signatures[0].role, SignerRole::Controller);
        assert_eq!(out.signatures[1], signature(SignerRole::Witness, 5));
        assert!(out.address_holds());
    }

    #[test]
    fn a_beat_cannot_change_the_record_or_anyone_elses_signature() {
        let rewrites = Beat(|mut c: SignedConsent| {
            c.record.agreed_acts = vec![RequestedAct::EnrollDevice];
            c.with_signature(signature(SignerRole::Witness, 5))
        });
        assert_eq!(attend(&rewrites, &admitted(), signed()), signed());

        let strips = Beat(|mut c: SignedConsent| {
            c.signatures.clear();
            c.with_signature(signature(SignerRole::Controller, 6))
                .with_signature(signature(SignerRole::Device, 7))
        });
        assert_eq!(attend(&strips, &admitted(), signed()), signed());
    }
}
