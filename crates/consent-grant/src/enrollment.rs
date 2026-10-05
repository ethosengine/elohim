//! The enrollment a consent agrees to, carried back to the terminal that asked.
//!
//! A consent record is evidence. What makes a device recognized is a binding
//! signed by the person's controller and by the device, notarized on the
//! device's own cell. When the person agrees to enroll a device, their node
//! signs the enrollment intent there and then; the terminal collects the intent
//! with that proof, adds its own possession proof over the same intent, and
//! submits the binding itself. The portal never needs the device's key.
//!
//! The intent is derived from the consent record, never written separately, so
//! the two cannot say different things about which device and which person.

use serde::{Deserialize, Serialize};

use crate::act::RequestedAct;
use crate::consent::ConsentRecord;

/// The domain every enrollment intent names. The mishpat zome refuses any other.
pub const ENROLLMENT_DOMAIN: &str = "elohim:device-enrollment:v1";

/// What the controller and the device both sign to bind the device to the
/// person. Field for field the mishpat zome's `DeviceIntent`, with hashes in
/// their `u…` text form and the time in microseconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnrollmentIntent {
    pub domain: String,
    /// The authority record the controller signed under.
    pub authority: String,
    /// The Human record the device is bound to.
    pub identity_root: String,
    pub device_key: String,
    pub network_dna: String,
    pub content_dna: String,
    pub issued_at_micros: i64,
    /// The revoked binding a reenrollment replaces. Always absent from an
    /// intent derived here: this ceremony enrolls a device anew.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
}

impl EnrollmentIntent {
    /// The intent `record` agreed to, or `None` when enrolling was not agreed.
    pub fn agreed_in(record: &ConsentRecord) -> Option<Self> {
        record
            .agreed_acts
            .contains(&RequestedAct::EnrollDevice)
            .then(|| Self {
                domain: ENROLLMENT_DOMAIN.to_string(),
                authority: record.authority.clone(),
                identity_root: record.identity_root.clone(),
                device_key: record.device_key.clone(),
                network_dna: record.network_dna.clone(),
                content_dna: record.content_dna.clone(),
                issued_at_micros: record.agreed_at_micros,
                supersedes: None,
            })
    }
}

impl EnrollmentIntent {
    /// The exact bytes every proof on this intent signs: the mishpat zome's
    /// `serde_json` encoding of its `DeviceIntent`, in which each hash is the
    /// array of its 39 raw bytes and the time is its microseconds. `None` when
    /// the domain is not [`ENROLLMENT_DOMAIN`] or a hash is malformed, since no
    /// zome would sign such an intent.
    pub fn signed_bytes(&self) -> Option<Vec<u8>> {
        use crate::hash_shape::{is_action_hash, is_agent_key, is_dna_hash, raw_39};
        if self.domain != ENROLLMENT_DOMAIN
            || !is_action_hash(&self.authority)
            || !is_action_hash(&self.identity_root)
            || !is_agent_key(&self.device_key)
            || !is_dna_hash(&self.network_dna)
            || !is_dna_hash(&self.content_dna)
            || self
                .supersedes
                .as_deref()
                .is_some_and(|s| !is_action_hash(s))
        {
            return None;
        }
        let array = |text: &str| -> Option<String> {
            let bytes = raw_39(text)?;
            let items: Vec<String> = bytes.iter().map(u8::to_string).collect();
            Some(format!("[{}]", items.join(",")))
        };
        let supersedes = match &self.supersedes {
            Some(s) => array(s)?,
            None => "null".to_string(),
        };
        Some(
            format!(
                concat!(
                    r#"{{"domain":"{}","authority":{},"identity_root":{},"device_key":{},"#,
                    r#""network_dna":{},"content_dna":{},"issued_at":{},"supersedes":{}}}"#
                ),
                ENROLLMENT_DOMAIN,
                array(&self.authority)?,
                array(&self.identity_root)?,
                array(&self.device_key)?,
                array(&self.network_dna)?,
                array(&self.content_dna)?,
                self.issued_at_micros,
                supersedes,
            )
            .into_bytes(),
        )
    }
}

/// One controller's signature on an enrollment intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerProof {
    /// The controller's agent key.
    pub agent: String,
    /// The raw ed25519 signature, in unpadded URL-safe base64.
    pub signature: String,
}

/// The intent a consent agreed to, and the controller proofs gathered on it so
/// far. One proof is enough to collect it; a person's other controllers may
/// affirm the device later, and nothing here waits for them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Enrollment {
    pub intent: EnrollmentIntent,
    pub controllers: Vec<ControllerProof>,
    /// For each approver that is a joined device rather than a root
    /// controller, the joining record it speaks through: the device names it
    /// in its own joining record so any peer can walk it back. Absent when
    /// every approver is a root controller.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approved_via: Vec<ApprovedVia>,
}

/// A device approver and the joining record it speaks through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovedVia {
    pub agent: String,
    pub binding: String,
}

impl Enrollment {
    /// Whether this is the enrollment `record` agreed to, signed by at least one
    /// controller.
    pub fn is_agreed_in(&self, record: &ConsentRecord) -> bool {
        EnrollmentIntent::agreed_in(record).as_ref() == Some(&self.intent)
            && !self.controllers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consent::tests::{agreement, peer_record};
    use crate::request::admit_request;
    use crate::request::tests::{peer_request, policy};

    #[test]
    fn the_intent_says_what_the_record_says() {
        let record = peer_record();
        let intent = EnrollmentIntent::agreed_in(&record).unwrap();
        assert_eq!(intent.domain, ENROLLMENT_DOMAIN);
        assert_eq!(intent.authority, record.authority);
        assert_eq!(intent.identity_root, record.identity_root);
        assert_eq!(intent.device_key, record.device_key);
        assert_eq!(intent.network_dna, record.network_dna);
        assert_eq!(intent.content_dna, record.content_dna);
        assert_eq!(intent.issued_at_micros, record.agreed_at_micros);
        assert_eq!(intent.supersedes, None);
    }

    #[test]
    fn no_intent_without_agreement_to_enroll() {
        let mut record = peer_record();
        record.agreed_acts.clear();
        assert_eq!(EnrollmentIntent::agreed_in(&record), None);
    }

    #[test]
    fn an_enrollment_matches_only_its_own_record_and_needs_a_proof() {
        let record = peer_record();
        let proof = ControllerProof {
            agent: "controller".into(),
            signature: "sig".into(),
        };
        let enrollment = Enrollment {
            intent: EnrollmentIntent::agreed_in(&record).unwrap(),
            controllers: vec![proof],
            approved_via: vec![],
        };
        assert!(enrollment.is_agreed_in(&record));

        let unsigned = Enrollment {
            controllers: vec![],
            ..enrollment.clone()
        };
        assert!(!unsigned.is_agreed_in(&record));

        let admitted = admit_request(&peer_request(), &policy()).unwrap();
        let mut later = agreement(admitted.request().acts.clone());
        later.agreed_at_micros += 1;
        let other = ConsentRecord::agree(&admitted, later).unwrap();
        assert!(!enrollment.is_agreed_in(&other));
    }

    #[test]
    fn the_signed_bytes_are_the_zomes_json_of_the_intent() {
        let intent = EnrollmentIntent::agreed_in(&peer_record()).unwrap();
        let bytes = String::from_utf8(intent.signed_bytes().unwrap()).unwrap();
        assert!(bytes
            .starts_with(r#"{"domain":"elohim:device-enrollment:v1","authority":[132,41,36,2,2,"#));
        assert!(bytes.ends_with(&format!(
            r#""issued_at":{},"supersedes":null}}"#,
            intent.issued_at_micros
        )));
        let mut foreign = intent.clone();
        foreign.domain = "elohim:device-enrollment:v0".into();
        assert_eq!(foreign.signed_bytes(), None);
        let mut broken = intent;
        broken.device_key = "uhCAk".into();
        assert_eq!(broken.signed_bytes(), None);
    }

    #[test]
    fn the_wire_form_is_camel_case() {
        let intent = EnrollmentIntent::agreed_in(&peer_record()).unwrap();
        let json = serde_json::to_value(&intent).unwrap();
        for field in [
            "identityRoot",
            "deviceKey",
            "networkDna",
            "contentDna",
            "issuedAtMicros",
        ] {
            assert!(json.get(field).is_some(), "{field}");
        }
        assert!(json.get("supersedes").is_none());
    }
}
