//! What a terminal sends to ask, and whether a portal should show it to a
//! person at all.

use serde::{Deserialize, Serialize};

use crate::act::{self, RequestedAct};
use crate::return_path::{is_token, ReturnPath};
use crate::{hash_shape, pkce, GRANT_DOMAIN};

/// A terminal's request for a controller's consent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantRequest {
    /// Must equal [`GRANT_DOMAIN`].
    pub domain: String,
    /// The registered relying party asking (for example the `epr` CLI).
    pub client_id: String,
    /// The agent key of the node that asks to be recognized.
    pub device_key: String,
    /// The device's cryptographic root, present exactly when the request asks
    /// to bind it (`device.bind-root`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_root_key: Option<String>,
    /// What the person will see this device called on the consent screen.
    /// Never part of the consent record: a label is the person's private word
    /// for a machine.
    pub label: String,
    /// The network on which the person's identity is kept.
    pub network_dna: String,
    /// The network on which the device will act for them.
    pub content_dna: String,
    pub acts: Vec<RequestedAct>,
    /// PKCE `S256` challenge; the terminal keeps the verifier.
    pub code_challenge: String,
    /// Opaque value the terminal checks on return, so a code that arrives is
    /// one it asked for.
    pub state: String,
    pub return_path: ReturnPath,
}

/// The portal's limits. Declared by the portal, not the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantPolicy {
    pub known_clients: Vec<String>,
    pub max_label_len: usize,
}

impl GrantPolicy {
    pub fn for_clients<I, S>(clients: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            known_clients: clients.into_iter().map(Into::into).collect(),
            max_label_len: 64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestRefusal {
    DomainUnsupported,
    ClientUnknown,
    DeviceKeyMalformed,
    NetworkMalformed,
    LabelMalformed,
    ActsEmpty,
    ActsIncoherent,
    DeviceRootMalformed,
    DeviceRootUnasked,
    DeviceRootMissing,
    ChallengeMalformed,
    StateMalformed,
    ReturnPortRestricted,
}

impl RequestRefusal {
    /// Stable code for the wire and the log.
    pub fn code(self) -> &'static str {
        match self {
            Self::DomainUnsupported => "request_domain_unsupported",
            Self::ClientUnknown => "request_client_unknown",
            Self::DeviceKeyMalformed => "request_device_key_malformed",
            Self::NetworkMalformed => "request_network_malformed",
            Self::LabelMalformed => "request_label_malformed",
            Self::ActsEmpty => "request_acts_empty",
            Self::ActsIncoherent => "request_acts_incoherent",
            Self::DeviceRootMalformed => "request_device_root_malformed",
            Self::DeviceRootUnasked => "request_device_root_unasked",
            Self::DeviceRootMissing => "request_device_root_missing",
            Self::ChallengeMalformed => "request_challenge_malformed",
            Self::StateMalformed => "request_state_malformed",
            Self::ReturnPortRestricted => "request_return_port_restricted",
        }
    }
}

/// A request that passed [`admit_request`]. Recording an agreement takes this
/// type, so a request that was never checked cannot be agreed to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedRequest(GrantRequest);

impl AdmittedRequest {
    pub fn request(&self) -> &GrantRequest {
        &self.0
    }

    /// Short device key for the consent screen.
    pub fn device_fingerprint(&self) -> String {
        hash_shape::fingerprint(&self.0.device_key)
    }
}

/// Lowest port a loopback return may name: below it are ports an unprivileged
/// terminal could not have opened.
const MIN_LOOPBACK_PORT: u16 = 1024;

/// Decide whether a request is fit to put in front of a person.
///
/// Admission says the request is well formed and from a known client. It says
/// nothing about whether the person should approve it; that is theirs.
pub fn admit_request(
    request: &GrantRequest,
    policy: &GrantPolicy,
) -> Result<AdmittedRequest, RequestRefusal> {
    use RequestRefusal as R;

    if request.domain != GRANT_DOMAIN {
        return Err(R::DomainUnsupported);
    }
    if !policy.known_clients.iter().any(|c| c == &request.client_id) {
        return Err(R::ClientUnknown);
    }
    if !hash_shape::is_agent_key(&request.device_key) {
        return Err(R::DeviceKeyMalformed);
    }
    if !hash_shape::is_dna_hash(&request.network_dna)
        || !hash_shape::is_dna_hash(&request.content_dna)
    {
        return Err(R::NetworkMalformed);
    }
    let label = request.label.trim();
    if label.is_empty()
        || label.len() != request.label.len()
        || label.chars().count() > policy.max_label_len
        || label.chars().any(char::is_control)
    {
        return Err(R::LabelMalformed);
    }
    if request.acts.is_empty() {
        return Err(R::ActsEmpty);
    }
    if !act::coherent(&request.acts) {
        return Err(R::ActsIncoherent);
    }
    // A device root travels exactly when binding it is asked for, so the
    // consent screen never shows a key nobody asked to bind, and never binds
    // one it did not show.
    let binds_root = request.acts.contains(&RequestedAct::BindDeviceRoot);
    match (&request.device_root_key, binds_root) {
        (None, false) => {}
        (Some(_), false) => return Err(R::DeviceRootUnasked),
        (None, true) => return Err(R::DeviceRootMissing),
        (Some(key), true) => {
            if !hash_shape::is_device_root_key(key) {
                return Err(R::DeviceRootMalformed);
            }
        }
    }
    if !pkce::challenge_is_well_formed(&request.code_challenge) {
        return Err(R::ChallengeMalformed);
    }
    if !is_token(&request.state, 16, 128) {
        return Err(R::StateMalformed);
    }
    if let ReturnPath::Loopback { port } = request.return_path {
        if port < MIN_LOOPBACK_PORT {
            return Err(R::ReturnPortRestricted);
        }
    }
    Ok(AdmittedRequest(request.clone()))
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::hash_shape::sample_key;

    pub(crate) const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    pub(crate) const NETWORK: &str = "uhC0kQwOEwmIBZhBT3I7vGZPz0kEL3_hqavyFW0upoO_hyEwuEglj";
    pub(crate) const CONTENT: &str = "uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt";

    pub(crate) fn policy() -> GrantPolicy {
        GrantPolicy::for_clients(["epr-cli"])
    }

    pub(crate) fn device_root() -> String {
        format!("did:key:z6Mk{}", "h".repeat(44))
    }

    pub(crate) fn request() -> GrantRequest {
        GrantRequest {
            domain: GRANT_DOMAIN.into(),
            client_id: "epr-cli".into(),
            device_key: sample_key(7),
            device_root_key: None,
            label: "Matthew's workspace".into(),
            network_dna: NETWORK.into(),
            content_dna: CONTENT.into(),
            acts: vec![RequestedAct::EnrollDevice],
            code_challenge: pkce::challenge(VERIFIER),
            state: "s".repeat(32),
            return_path: ReturnPath::Paste,
        }
    }

    /// Enroll and bind the device root: what a peer runtime asks for.
    pub(crate) fn peer_request() -> GrantRequest {
        let mut r = request();
        r.acts.push(RequestedAct::BindDeviceRoot);
        r.device_root_key = Some(device_root());
        r
    }

    fn refused(change: impl FnOnce(&mut GrantRequest)) -> RequestRefusal {
        let mut r = request();
        change(&mut r);
        admit_request(&r, &policy()).unwrap_err()
    }

    #[test]
    fn a_well_formed_request_is_admitted() {
        let admitted = admit_request(&request(), &policy()).unwrap();
        assert_eq!(admitted.request(), &request());
        assert!(admitted.device_fingerprint().contains('…'));
        assert!(admit_request(&peer_request(), &policy()).is_ok());
    }

    #[test]
    fn each_malformation_has_its_own_refusal() {
        use RequestRefusal as R;
        assert_eq!(
            refused(|r| r.domain = "elohim:consent-grant:v0".into()),
            R::DomainUnsupported
        );
        assert_eq!(
            refused(|r| r.client_id = "stranger".into()),
            R::ClientUnknown
        );
        assert_eq!(
            refused(|r| r.device_key = NETWORK.into()),
            R::DeviceKeyMalformed
        );
        assert_eq!(
            refused(|r| r.network_dna = sample_key(1)),
            R::NetworkMalformed
        );
        assert_eq!(
            refused(|r| r.content_dna = sample_key(1)),
            R::NetworkMalformed
        );
        assert_eq!(refused(|r| r.acts.clear()), R::ActsEmpty);
        assert_eq!(
            refused(|r| r.acts.push(RequestedAct::EnrollDevice)),
            R::ActsIncoherent
        );
        assert_eq!(
            refused(|r| r.code_challenge = VERIFIER[..42].into()),
            R::ChallengeMalformed
        );
        assert_eq!(refused(|r| r.state = "short".into()), R::StateMalformed);
        assert_eq!(
            refused(|r| r.state = format!("{}#", "s".repeat(31))),
            R::StateMalformed
        );
        assert_eq!(
            refused(|r| r.return_path = ReturnPath::Loopback { port: 80 }),
            R::ReturnPortRestricted
        );
    }

    #[test]
    fn a_device_root_travels_exactly_when_binding_is_asked() {
        use RequestRefusal as R;
        assert_eq!(
            refused(|r| r.device_root_key = Some(device_root())),
            R::DeviceRootUnasked
        );
        assert_eq!(
            refused(|r| r.acts.push(RequestedAct::BindDeviceRoot)),
            R::DeviceRootMissing
        );
        assert_eq!(
            refused(|r| {
                r.acts.push(RequestedAct::BindDeviceRoot);
                r.device_root_key = Some("did:key:z6Mkshort".into());
            }),
            R::DeviceRootMalformed
        );
        assert_eq!(
            refused(|r| {
                r.acts = vec![RequestedAct::BindDeviceRoot];
                r.device_root_key = Some(device_root());
            }),
            R::ActsIncoherent
        );
    }

    #[test]
    fn a_label_is_plain_bounded_text() {
        use RequestRefusal as R;
        for bad in ["", "   ", " padded ", "line\nbreak", &"x".repeat(65)] {
            assert_eq!(
                refused(|r| r.label = bad.to_string()),
                R::LabelMalformed,
                "{bad:?}"
            );
        }
    }

    #[test]
    fn refusal_codes_are_distinct() {
        use RequestRefusal as R;
        let all = [
            R::DomainUnsupported,
            R::ClientUnknown,
            R::DeviceKeyMalformed,
            R::NetworkMalformed,
            R::LabelMalformed,
            R::ActsEmpty,
            R::ActsIncoherent,
            R::DeviceRootMalformed,
            R::DeviceRootUnasked,
            R::DeviceRootMissing,
            R::ChallengeMalformed,
            R::StateMalformed,
            R::ReturnPortRestricted,
        ];
        let codes: HashSet<_> = all.iter().map(|r| r.code()).collect();
        assert_eq!(codes.len(), all.len());
    }

    #[test]
    fn the_wire_form_is_camel_case() {
        let json = serde_json::to_value(peer_request()).unwrap();
        for field in [
            "clientId",
            "deviceKey",
            "deviceRootKey",
            "networkDna",
            "contentDna",
            "codeChallenge",
            "returnPath",
        ] {
            assert!(json.get(field).is_some(), "{field}");
        }
        assert_eq!(json["acts"][0], "device.enroll");
    }
}
