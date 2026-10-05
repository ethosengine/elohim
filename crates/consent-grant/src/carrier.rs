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

use serde::{Deserialize, Serialize};

use crate::ceremony::Delivered;
use crate::delivery::Redemption;
use crate::pending::Ask;

/// What one node asks of another over a carrier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CarryRequest {
    Ask {
        ask: Ask,
    },
    /// The approving node hands back the code for the request with `state`.
    Code {
        state: String,
        code: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        approver: Option<String>,
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
}
