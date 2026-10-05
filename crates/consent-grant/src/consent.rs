//! The consent record: what was asked, what was agreed, for which device and
//! which person.
//!
//! The record is a content-addressed atom. Its address covers what it says and
//! nothing about how it was delivered or who has signed it, so:
//!
//! - it can be held locally and shown to anyone, and reads the same everywhere;
//! - signatures are detached and can be added later without changing it. A
//!   record signed only by the approving controller is recorded. One the device
//!   also signed is witnessed by both parties to it. A peer may add its own.
//!
//! Every party signs the same short message, [`consent_message`]: a domain tag
//! followed by the record's address. The address commits to every byte of the
//! record, and the tag keeps a consent signature from ever reading as a
//! signature on anything else a key signs.
//!
//! It is evidence that consent was given. It does not make a device recognized;
//! the binding the controller's cell signs does that, and peers verify the
//! binding. The record lets anyone see that the binding was asked for and
//! agreed to, and what was declined.
//!
//! The device's label is left out on purpose. A label is the person's private
//! word for a machine, and a record that may be shown or notarized later must
//! not carry it.

use serde::{Deserialize, Serialize};

use crate::act::{self, RequestedAct};
use crate::hash_shape;
use crate::request::AdmittedRequest;
use crate::GRANT_DOMAIN;

/// What precedes a record's address in the message its signers sign. The
/// mishpat zome's `sign_device_consent` signs exactly these bytes.
pub const CONSENT_SIGNING_DOMAIN: &str = "elohim:device-consent:v1:";

/// The bytes a party signs to put its name on the consent addressed `cid`.
pub fn consent_message(cid: &str) -> Vec<u8> {
    let mut message = Vec::with_capacity(CONSENT_SIGNING_DOMAIN.len() + cid.len());
    message.extend_from_slice(CONSENT_SIGNING_DOMAIN.as_bytes());
    message.extend_from_slice(cid.as_bytes());
    message
}

/// What the controller decided about an admitted request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agreement {
    /// The identity the device is bound to: the root of the person's identity
    /// record. The controller supplies it; the terminal does not know it.
    pub identity_root: String,
    /// The identity's current authority record, under which the controller acts.
    pub authority: String,
    /// The acts agreed to, chosen from those asked for.
    pub agreed_acts: Vec<RequestedAct>,
    pub agreed_at_micros: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentRefusal {
    IdentityMalformed,
    AgreedNothing,
    AgreedBeyondRequest,
    AgreedIncoherent,
    Encoding,
}

impl ConsentRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::IdentityMalformed => "consent_identity_malformed",
            Self::AgreedNothing => "consent_agreed_nothing",
            Self::AgreedBeyondRequest => "consent_agreed_beyond_request",
            Self::AgreedIncoherent => "consent_agreed_incoherent",
            Self::Encoding => "consent_encoding",
        }
    }
}

/// The addressed statement of a consent. Field names are the wire form; the
/// canonical encoding is dag-cbor, which orders keys itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConsentRecord {
    pub domain: String,
    pub client_id: String,
    pub identity_root: String,
    pub authority: String,
    pub device_key: String,
    /// Present exactly when binding the device root was agreed to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_root_key: Option<String>,
    pub network_dna: String,
    pub content_dna: String,
    pub asked_acts: Vec<RequestedAct>,
    pub agreed_acts: Vec<RequestedAct>,
    pub agreed_at_micros: i64,
    /// The asking terminal's PKCE challenge. It ties the record to one request
    /// without revealing anything a holder could use: redeeming needs the
    /// verifier behind it.
    pub request_binding: String,
}

impl ConsentRecord {
    /// Record a controller's agreement to an admitted request.
    ///
    /// A person may agree to less than was asked, never more, and what they
    /// agree to must still make sense by itself.
    pub fn agree(admitted: &AdmittedRequest, agreement: Agreement) -> Result<Self, ConsentRefusal> {
        use ConsentRefusal as R;

        if !hash_shape::is_action_hash(&agreement.identity_root)
            || !hash_shape::is_action_hash(&agreement.authority)
        {
            return Err(R::IdentityMalformed);
        }
        let r = admitted.request();
        let agreed = &agreement.agreed_acts;
        if agreed.is_empty() {
            return Err(R::AgreedNothing);
        }
        if !agreed.iter().all(|a| r.acts.contains(a)) {
            return Err(R::AgreedBeyondRequest);
        }
        if !act::coherent(agreed) {
            return Err(R::AgreedIncoherent);
        }
        let binds_root = agreed.contains(&RequestedAct::BindDeviceRoot);
        Ok(Self {
            domain: GRANT_DOMAIN.to_string(),
            client_id: r.client_id.clone(),
            identity_root: agreement.identity_root,
            authority: agreement.authority,
            device_key: r.device_key.clone(),
            device_root_key: binds_root.then(|| r.device_root_key.clone()).flatten(),
            network_dna: r.network_dna.clone(),
            content_dna: r.content_dna.clone(),
            asked_acts: r.acts.clone(),
            agreed_acts: agreement.agreed_acts,
            agreed_at_micros: agreement.agreed_at_micros,
            request_binding: r.code_challenge.clone(),
        })
    }

    /// The bytes that are addressed and signed.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ConsentRefusal> {
        serde_ipld_dagcbor::to_vec(self).map_err(|_| ConsentRefusal::Encoding)
    }

    /// The record's content address: CIDv1, dag-cbor, sha2-256, as every atom.
    pub fn cid(&self) -> Result<String, ConsentRefusal> {
        Ok(elohim_epr::cid::compute_cid(&self.canonical_bytes()?).to_string())
    }

    /// What was asked for and not agreed to.
    pub fn declined_acts(&self) -> Vec<RequestedAct> {
        self.asked_acts
            .iter()
            .copied()
            .filter(|a| !self.agreed_acts.contains(a))
            .collect()
    }
}

/// In what capacity a party signed. The floor today is deterministic: the
/// controller who agreed and the device that asked. A witness is anyone else
/// who was present and attests to it, which is where an elohim attending the
/// person adds its signature as that role matures. A witness strengthens a
/// record; a record never needs one to be valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignerRole {
    Controller,
    Device,
    Witness,
}

/// One party's signature on a record. Detached: it is not part of what the
/// record's address covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentSignature {
    pub role: SignerRole,
    /// The signer's key, in the form its kind is written (an agent key).
    pub signer: String,
    /// The raw ed25519 signature over [`consent_message`] of the record's
    /// address, in unpadded URL-safe base64. Not over the canonical bytes
    /// themselves: the address already commits to them.
    pub signature: String,
}

/// A record together with the signatures gathered so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedConsent {
    pub cid: String,
    pub record: ConsentRecord,
    pub signatures: Vec<ConsentSignature>,
}

impl SignedConsent {
    pub fn new(record: ConsentRecord) -> Result<Self, ConsentRefusal> {
        Ok(Self {
            cid: record.cid()?,
            record,
            signatures: Vec::new(),
        })
    }

    /// Add a signature. A signer already present in the same role is replaced,
    /// so re-signing never grows the set. The address does not change.
    pub fn with_signature(mut self, signature: ConsentSignature) -> Self {
        self.signatures
            .retain(|s| !(s.signer == signature.signer && s.role == signature.role));
        self.signatures.push(signature);
        self
    }

    /// Whether a controller has signed. This is the floor a consent must meet
    /// to be issued; a device or witness signature alone is not agreement.
    pub fn controller_signed(&self) -> bool {
        self.signatures
            .iter()
            .any(|s| s.role == SignerRole::Controller)
    }

    /// The bytes every signer of this consent signs.
    pub fn message(&self) -> Vec<u8> {
        consent_message(&self.cid)
    }

    /// How many parties have signed in `role`.
    pub fn signed_in(&self, role: SignerRole) -> usize {
        self.signatures.iter().filter(|s| s.role == role).count()
    }

    /// Whether the stated address is the record's own. Signature validity is
    /// checked by whoever holds the signers' keys; this crate holds none.
    pub fn address_holds(&self) -> bool {
        self.record.cid().is_ok_and(|cid| cid == self.cid)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hash_shape::{sample_action, sample_key};
    use crate::request::admit_request;
    use crate::request::tests::{peer_request, policy, request};

    pub(crate) const AT: i64 = 1_791_000_000_000_000;

    pub(crate) fn agreement(acts: Vec<RequestedAct>) -> Agreement {
        Agreement {
            identity_root: sample_action(1),
            authority: sample_action(2),
            agreed_acts: acts,
            agreed_at_micros: AT,
        }
    }

    pub(crate) fn peer_record() -> ConsentRecord {
        let admitted = admit_request(&peer_request(), &policy()).unwrap();
        let acts = admitted.request().acts.clone();
        ConsentRecord::agree(&admitted, agreement(acts)).unwrap()
    }

    fn agree(acts: Vec<RequestedAct>) -> Result<ConsentRecord, ConsentRefusal> {
        let admitted = admit_request(&peer_request(), &policy()).unwrap();
        ConsentRecord::agree(&admitted, agreement(acts))
    }

    #[test]
    fn a_record_says_what_was_asked_and_what_was_agreed() {
        let full = peer_record();
        assert_eq!(full.agreed_acts, full.asked_acts);
        assert!(full.device_root_key.is_some());
        assert!(full.declined_acts().is_empty());

        // The person enrolls the device and declines to bind its root.
        let narrowed = agree(vec![RequestedAct::EnrollDevice]).unwrap();
        assert_eq!(narrowed.declined_acts(), vec![RequestedAct::BindDeviceRoot]);
        assert_eq!(narrowed.device_root_key, None);
        assert_eq!(narrowed.asked_acts.len(), 2);
    }

    #[test]
    fn a_person_may_agree_to_less_never_more() {
        use ConsentRefusal as R;
        assert_eq!(agree(vec![]), Err(R::AgreedNothing));
        assert_eq!(
            agree(vec![RequestedAct::BindDeviceRoot]),
            Err(R::AgreedIncoherent)
        );
        let enroll_only = admit_request(&request(), &policy()).unwrap();
        let wider = agreement(vec![
            RequestedAct::EnrollDevice,
            RequestedAct::BindDeviceRoot,
        ]);
        assert_eq!(
            ConsentRecord::agree(&enroll_only, wider),
            Err(R::AgreedBeyondRequest)
        );
    }

    #[test]
    fn the_identity_is_named_by_notarized_records() {
        let admitted = admit_request(&request(), &policy()).unwrap();
        let mut a = agreement(vec![RequestedAct::EnrollDevice]);
        a.identity_root = sample_key(1);
        assert_eq!(
            ConsentRecord::agree(&admitted, a),
            Err(ConsentRefusal::IdentityMalformed)
        );
    }

    #[test]
    fn the_address_is_a_dag_cbor_cid_and_is_stable() {
        let cid = peer_record().cid().unwrap();
        assert!(cid.starts_with("bafyrei"), "{cid}");
        assert_eq!(cid, peer_record().cid().unwrap());
        let mut other = peer_record();
        other.agreed_at_micros += 1;
        assert_ne!(cid, other.cid().unwrap());
    }

    #[test]
    fn the_record_carries_no_label_and_no_delivery_state() {
        let json = serde_json::to_string(&peer_record()).unwrap();
        assert!(!json.contains("Matthew's workspace"));
        for absent in ["label", "state", "returnPath", "code\"", "redeemed"] {
            assert!(!json.contains(absent), "{absent}");
        }
    }

    #[test]
    fn signing_again_never_moves_the_address() {
        let signed = SignedConsent::new(peer_record()).unwrap();
        let cid = signed.cid.clone();
        let sign = |role, who: u8, sig: &str| ConsentSignature {
            role,
            signer: sample_key(who),
            signature: sig.into(),
        };
        let signed = signed
            .with_signature(sign(SignerRole::Controller, 9, "controller"))
            .with_signature(sign(SignerRole::Device, 7, "device"))
            .with_signature(sign(SignerRole::Controller, 9, "controller-again"))
            .with_signature(sign(SignerRole::Witness, 5, "elohim"));
        assert_eq!(signed.cid, cid);
        assert!(signed.address_holds());
        assert_eq!(signed.signatures.len(), 3);
        assert_eq!(signed.signatures[1].signature, "controller-again");
        assert!(signed.controller_signed());
    }

    #[test]
    fn signers_sign_the_tagged_address_not_the_record_bytes() {
        let signed = SignedConsent::new(peer_record()).unwrap();
        let message = signed.message();
        assert_eq!(
            message,
            format!("elohim:device-consent:v1:{}", signed.cid).into_bytes()
        );
        assert_ne!(message, signed.record.canonical_bytes().unwrap());
    }

    #[test]
    fn a_tampered_record_no_longer_matches_its_address() {
        let mut signed = SignedConsent::new(peer_record()).unwrap();
        signed.record.agreed_acts = vec![RequestedAct::EnrollDevice];
        assert!(!signed.address_holds());
    }

    #[test]
    fn the_record_round_trips_through_its_canonical_bytes() {
        let record = peer_record();
        let bytes = record.canonical_bytes().unwrap();
        let back: ConsentRecord = serde_ipld_dagcbor::from_slice(&bytes).unwrap();
        assert_eq!(back, record);
    }
}
