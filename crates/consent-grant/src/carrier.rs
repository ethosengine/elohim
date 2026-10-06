//! The messages a carrier moves between an asking device and an approving node.
//!
//! A carrier never stores them and never causes a signature. Three exchanges:
//!
//! - the device's node offers its [`Ask`]; the approving node lists it or says why
//!   not;
//! - once someone has decided and the approving node has agreed, the approving node hands
//!   the code back for the request's state;
//! - the device's node redeems the code with the terminal's PKCE verifier, and
//!   the approving node answers with what [`crate::redeem`] would give.
//!
//! The code is bound to the device's own request and verifier exactly as on
//! the link-and-paste path, so a carrier that copies a message gains nothing.
//! The messages travel as JSON so optional fields keep their meaning under any
//! outer encoding.
//!
//! Who says what is proven, not taken from the transport. Every node on the
//! private network can send anything; the transport id it arrives from is
//! authenticated by the transport, the agent key it claims is not. So each
//! message that a node acts on carries a [`CarrierProof`]: the claimed agent
//! key's signature over what the message says, which transport id says it, and
//! what kind of message it is ([`statement`]):
//!
//! - an [`crate::pending::Ask`] is signed by the device key the request names,
//!   over its state, its PKCE challenge and the device's transport id, so no
//!   one else can list, or replace, an ask in that device's name;
//! - a `Listed` is signed by the approving node's key over its own transport
//!   id, so a device knows which key listed it and from where;
//! - a `Code` and a `Declined` are signed by that same key over the request's
//!   state and its transport id, so only the node that listed an ask can hand
//!   back its code or decline it, and a device that declared its approver
//!   takes them only from that key.
//!
//! A replayed proof does not travel: it names the transport id it was made
//! for, and the receiver checks it against the one the message came from.

use serde::{Deserialize, Serialize};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use crate::ceremony::Delivered;
use crate::delivery::Redemption;
use crate::hash_shape::agent_public_key;
use crate::pending::Ask;

/// The domain every carrier statement is signed under. Nothing else signed by
/// an agent key starts with it.
pub const CARRIER_DOMAIN: &str = "elohim:device-carrier:v1:";

/// What a carrier statement says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CarrierKind {
    Ask,
    Listed,
    Code,
    Declined,
}

impl CarrierKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Listed => "listed",
            Self::Code => "code",
            Self::Declined => "declined",
        }
    }
}

/// An agent key's signature over a carrier statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarrierProof {
    /// The agent key that signed.
    pub signer: String,
    /// Its ed25519 signature, base64url.
    pub signature: String,
}

/// The longest field a statement carries: states and challenges are short
/// tokens, transport ids a few dozen characters.
pub const MAX_STATEMENT_FIELD: usize = 128;

/// The bytes a carrier statement is signed as: the domain, then the kind,
/// the request's state, the speaker's transport id and the PKCE challenge,
/// one per line (empty when a kind carries none). `None` when a field is
/// too long or holds a line break, so no two statements share bytes.
pub fn statement(
    kind: CarrierKind,
    state: &str,
    transport_id: &str,
    challenge: &str,
) -> Option<Vec<u8>> {
    let fits = |f: &str| f.len() <= MAX_STATEMENT_FIELD && !f.contains(['\n', '\r']);
    if !(fits(state) && fits(transport_id) && fits(challenge)) || transport_id.is_empty() {
        return None;
    }
    Some(
        format!(
            "{CARRIER_DOMAIN}{}\n{state}\n{transport_id}\n{challenge}",
            kind.as_str()
        )
        .into_bytes(),
    )
}

/// Whether `proof` is `signer`'s signature over the statement, and `signer`
/// is who it says.
pub fn proof_holds(
    proof: &CarrierProof,
    signer: &str,
    kind: CarrierKind,
    state: &str,
    transport_id: &str,
    challenge: &str,
) -> bool {
    if proof.signer != signer {
        return false;
    }
    let (Some(key), Ok(raw), Some(message)) = (
        agent_public_key(signer),
        URL_SAFE_NO_PAD.decode(&proof.signature),
        statement(kind, state, transport_id, challenge),
    ) else {
        return false;
    };
    elohim_epr::proof::verify(&key, &message, &raw)
}

/// What one node asks of another over a carrier.
// One message is built per send and dropped after it; the ask's size is
// not worth an indirection on the wire type.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CarryRequest {
    Ask {
        ask: Ask,
    },
    /// The approving node hands back the code for the request with `state`.
    /// `proof` is `approver`'s over (code, state, its transport id).
    Code {
        state: String,
        code: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        approver: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        proof: Option<CarrierProof>,
    },
    Redeem {
        redemption: Redemption,
    },
    /// The approving node declined the request with `state`: nothing was
    /// signed, and the device's terminal stops waiting and says so.
    Declined {
        state: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        approver: Option<String>,
        /// `approver`'s over (declined, state, its transport id).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        proof: Option<CarrierProof>,
    },
}

/// What the other node answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CarryResponse {
    /// The ask waits on this node under `number`.
    Listed {
        number: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        approver: Option<String>,
        /// `approver`'s over (listed, its transport id).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        proof: Option<CarrierProof>,
    },
    /// The ask was not listed, or the code not taken, and why.
    Refused { code: String },
    /// The device's node holds the code for its terminal.
    CodeTaken,
    /// The device's node took the decline; its terminal stops waiting.
    DeclineTaken,
    /// What the code redeemed.
    Delivered { delivered: Box<Delivered> },
}

impl CarryRequest {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("carry_unreadable: {e}"))
    }
}

impl CarryResponse {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes).map_err(|e| format!("carry_unreadable: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pending::tests::ask;

    #[test]
    fn every_message_round_trips_through_its_bytes() {
        for request in [
            CarryRequest::Ask { ask: ask(1) },
            CarryRequest::Code {
                state: "s".repeat(32),
                code: "c".repeat(32),
                approver: None,
                proof: None,
            },
        ] {
            assert_eq!(
                CarryRequest::from_bytes(&request.to_bytes()).unwrap(),
                request
            );
        }
        for response in [
            CarryResponse::Listed {
                number: 3,
                approver: Some("uhCAk".into()),
                proof: None,
            },
            CarryResponse::Refused {
                code: "ask_list_full".into(),
            },
            CarryResponse::CodeTaken,
        ] {
            assert_eq!(
                CarryResponse::from_bytes(&response.to_bytes()).unwrap(),
                response
            );
        }
        assert!(CarryRequest::from_bytes(b"{\"kind\":\"Grant\"}").is_err());
    }

    #[test]
    fn a_proof_holds_only_for_its_own_signer_kind_state_and_transport() {
        let (key, agent) = crate::pending::tests::keypair(7);
        let sign = |kind, state: &str, tid: &str, ch: &str| CarrierProof {
            signer: agent.clone(),
            signature: URL_SAFE_NO_PAD.encode(elohim_epr::proof::sign(
                &key,
                &statement(kind, state, tid, ch).unwrap(),
            )),
        };
        let p = sign(CarrierKind::Code, "state-1", "peer-a", "");
        assert!(proof_holds(
            &p,
            &agent,
            CarrierKind::Code,
            "state-1",
            "peer-a",
            ""
        ));
        // Replayed from another transport id, for another ask, as another
        // kind, or claimed for another key: nothing.
        assert!(!proof_holds(
            &p,
            &agent,
            CarrierKind::Code,
            "state-1",
            "peer-b",
            ""
        ));
        assert!(!proof_holds(
            &p,
            &agent,
            CarrierKind::Code,
            "state-2",
            "peer-a",
            ""
        ));
        assert!(!proof_holds(
            &p,
            &agent,
            CarrierKind::Declined,
            "state-1",
            "peer-a",
            ""
        ));
        let (_, other) = crate::pending::tests::keypair(8);
        assert!(!proof_holds(
            &p,
            &other,
            CarrierKind::Code,
            "state-1",
            "peer-a",
            ""
        ));
        // A field cannot smuggle a second line.
        assert!(statement(CarrierKind::Ask, "a\nb", "peer-a", "").is_none());
        assert!(statement(CarrierKind::Ask, "a", "", "").is_none());
    }
}
