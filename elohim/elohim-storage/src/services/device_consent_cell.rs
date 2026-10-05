//! The controller's cell, reached through this node's conductor, for the
//! device consent ceremony.
//!
//! [`super::device_consent`] decides nothing about how a cell is reached; it
//! asks a [`ControllerCell`]. This is the one that asks a real conductor: it
//! reads the person's standing with `mishpat::my_consent_standing` and has the
//! cell sign with `mishpat::sign_device_approval` under a mandate minted for
//! that one call ([`HcClient::call_zome_mandated`]).
//!
//! Which cell signs is the `cell` it is built with. On a person's own node that
//! is the node's mishpat cell ([`ConductorControllerCell::own`]).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use consent_grant::{ControllerProof, ControllerStanding, EnrollmentIntent};
use holochain_client::CellId;
use holochain_types::prelude::{ActionHash, AgentPubKey, DnaHash, ExternIO, Signature, Timestamp};
use serde::{Deserialize, Serialize};

use super::device_consent::{
    ApprovalProofs, ApprovalRequest, CellFailure, CellStanding, ControllerCell,
};
use crate::error::StorageError;
use crate::hc_client::{HcClient, MISHPAT_ROLE};

const MISHPAT_ZOME: &str = "mishpat";
const SIGN_APPROVAL: &str = "sign_device_approval";
/// How long the mandate for one approval stays usable. Long enough for one
/// call on a busy conductor; after it the grant authorizes nothing.
const MANDATE_WINDOW: Duration = Duration::from_secs(120);

// Mirrors of the mishpat coordinator's wire types (`device_enrollment.rs`).
// Field names and ORDER matter: the mandate carries the exact JSON the zome
// re-serializes, so `serde_json::to_string` here must match it byte for byte.
// `the_approval_payload_keeps_its_key_order` pins the same key list on both
// sides.

#[derive(Serialize, Debug)]
struct DeviceConsentWire {
    authority: ActionHash,
    identity_root: ActionHash,
    consent_cid: String,
}

#[derive(Serialize, Debug)]
struct DeviceIntentWire {
    domain: String,
    authority: ActionHash,
    identity_root: ActionHash,
    device_key: AgentPubKey,
    network_dna: DnaHash,
    content_dna: DnaHash,
    issued_at: Timestamp,
    supersedes: Option<ActionHash>,
}

#[derive(Serialize, Debug)]
struct DeviceApprovalWire {
    consent: DeviceConsentWire,
    enrollment: Option<DeviceIntentWire>,
}

#[derive(Deserialize, Debug)]
struct ProofWire {
    agent: AgentPubKey,
    signature: Signature,
}

#[derive(Deserialize, Debug)]
struct DeviceApprovalProofsWire {
    consent: ProofWire,
    enrollment: Option<ProofWire>,
}

#[derive(Deserialize, Debug)]
struct ConsentStandingWire {
    identity_root: ActionHash,
    authority: Option<ActionHash>,
    controllers: Vec<AgentPubKey>,
    required: usize,
    network_dna: DnaHash,
}

/// A `u…` hash string as the zome's type. The `u` check comes first because
/// the decoder panics on an empty string.
fn hash<T>(text: &str, what: &str) -> Result<T, CellFailure>
where
    T: for<'a> TryFrom<&'a str>,
{
    if !text.starts_with('u') {
        return Err(CellFailure::Refused(format!("{what} is not a hash")));
    }
    T::try_from(text).map_err(|_| CellFailure::Refused(format!("{what} is not a hash")))
}

fn approval_wire(approval: &ApprovalRequest) -> Result<DeviceApprovalWire, CellFailure> {
    let enrollment = approval
        .enrollment
        .as_ref()
        .map(|i: &EnrollmentIntent| {
            Ok::<_, CellFailure>(DeviceIntentWire {
                domain: i.domain.clone(),
                authority: hash(&i.authority, "authority")?,
                identity_root: hash(&i.identity_root, "identity")?,
                device_key: hash(&i.device_key, "device key")?,
                network_dna: hash(&i.network_dna, "network")?,
                content_dna: hash(&i.content_dna, "content network")?,
                issued_at: Timestamp::from_micros(i.issued_at_micros),
                supersedes: i
                    .supersedes
                    .as_deref()
                    .map(|s| hash(s, "superseded binding"))
                    .transpose()?,
            })
        })
        .transpose()?;
    Ok(DeviceApprovalWire {
        consent: DeviceConsentWire {
            authority: hash(&approval.authority, "authority")?,
            identity_root: hash(&approval.identity_root, "identity")?,
            consent_cid: approval.consent_cid.clone(),
        },
        enrollment,
    })
}

fn proof(wire: ProofWire) -> ControllerProof {
    ControllerProof {
        agent: wire.agent.to_string(),
        signature: URL_SAFE_NO_PAD.encode(wire.signature.0),
    }
}

/// Unavailable when the conductor could not be reached or did not answer;
/// refused when the zome answered with a refusal.
fn failure(error: StorageError) -> CellFailure {
    let message = error.to_string();
    match error {
        StorageError::Conductor(_)
            if !crate::conductor_bridge_health::is_transport_dead(&message)
                && message.contains("Guest") =>
        {
            CellFailure::Refused(message)
        }
        StorageError::InvalidInput(_) => CellFailure::Refused(message),
        _ => CellFailure::Unavailable(message),
    }
}

/// A controller cell on the conductor `hc` speaks to.
pub struct ConductorControllerCell {
    hc: Arc<HcClient>,
    cell: CellId,
}

impl ConductorControllerCell {
    /// The cell `cell` on `hc`'s conductor.
    pub fn new(hc: Arc<HcClient>, cell: CellId) -> Self {
        Self { hc, cell }
    }

    /// This node's own mishpat cell, when the installed hApp has one.
    pub fn own(hc: Arc<HcClient>) -> Option<Self> {
        let cell = hc.cell_id_for_role(MISHPAT_ROLE)?.clone();
        Some(Self::new(hc, cell))
    }
}

#[async_trait]
impl ControllerCell for ConductorControllerCell {
    fn agent(&self) -> String {
        self.cell.agent_pubkey().to_string()
    }

    async fn standing(&self) -> Result<CellStanding, CellFailure> {
        // Ordinary credentials read the client's own cells only. A host that
        // signs for a cell it hosts reads that cell's standing its own way.
        if self.hc.cell_id_for_role(MISHPAT_ROLE) != Some(&self.cell) {
            return Err(CellFailure::Unavailable(
                "this node reads standing only for its own cell".into(),
            ));
        }
        let payload = ExternIO::encode(())
            .map_err(|e| CellFailure::Unavailable(e.to_string()))?
            .into_vec();
        let answer = self
            .hc
            .call_zome_mishpat("mishpat", "my_consent_standing", payload)
            .await
            .map_err(failure)?;
        let standing: Option<ConsentStandingWire> = ExternIO::from(answer)
            .decode()
            .map_err(|e| CellFailure::Unavailable(format!("standing decode: {e}")))?;
        Ok(match standing {
            None => CellStanding::NoPerson,
            Some(s) => match s.authority {
                None => CellStanding::Unbootstrapped,
                Some(authority) => CellStanding::Ready(ControllerStanding {
                    identity_root: s.identity_root.to_string(),
                    authority: authority.to_string(),
                    network_dna: s.network_dna.to_string(),
                    controllers: s.controllers.iter().map(ToString::to_string).collect(),
                    required: s.required,
                }),
            },
        })
    }

    async fn sign(&self, approval: &ApprovalRequest) -> Result<ApprovalProofs, CellFailure> {
        let wire = approval_wire(approval)?;
        let json = serde_json::to_string(&wire)
            .map_err(|e| CellFailure::Unavailable(format!("approval encoding: {e}")))?;
        let payload = ExternIO::encode(&wire)
            .map_err(|e| CellFailure::Unavailable(format!("approval encoding: {e}")))?
            .into_vec();
        let answer = self
            .hc
            .call_zome_mandated(
                &self.cell,
                MISHPAT_ZOME,
                SIGN_APPROVAL,
                payload,
                json,
                MANDATE_WINDOW,
            )
            .await
            .map_err(failure)?;
        let proofs: DeviceApprovalProofsWire = ExternIO::from(answer)
            .decode()
            .map_err(|e| CellFailure::Unavailable(format!("proofs decode: {e}")))?;
        Ok(ApprovalProofs {
            consent: proof(proofs.consent),
            enrollment: proofs.enrollment.map(proof),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: &str = "uhCkkRrENFlI2RlXCelrj6C6ttNq8qTI_wSh0fFmsrSvBgdkkM39j";
    const AUTHORITY: &str = "uhCkkN_k9u6glm7eRydJ_WWbyUSSWbBYMHd_6aWO4ag-PrcWHA5_-";
    const AGENT: &str = "uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl";
    const NETWORK: &str = "uhC0kQwOEwmIBZhBT3I7vGZPz0kEL3_hqavyFW0upoO_hyEwuEglj";
    const CONTENT: &str = "uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt";

    fn approval() -> ApprovalRequest {
        ApprovalRequest {
            consent_cid: "bafyreigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".into(),
            authority: AUTHORITY.into(),
            identity_root: IDENTITY.into(),
            enrollment: Some(EnrollmentIntent {
                domain: consent_grant::ENROLLMENT_DOMAIN.into(),
                authority: AUTHORITY.into(),
                identity_root: IDENTITY.into(),
                device_key: AGENT.into(),
                network_dna: NETWORK.into(),
                content_dna: CONTENT.into(),
                issued_at_micros: 123,
                supersedes: None,
            }),
        }
    }

    fn keys_in_order(json: &str) -> Vec<String> {
        json.split('"')
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[1].starts_with(':'))
            .map(|w| w[0].to_string())
            .collect()
    }

    /// Same list as the mishpat zome's `the_approval_payload_keeps_its_key_order`.
    #[test]
    fn the_approval_payload_keeps_its_key_order() {
        let json = serde_json::to_string(&approval_wire(&approval()).unwrap()).unwrap();
        assert_eq!(
            keys_in_order(&json),
            [
                "consent",
                "authority",
                "identity_root",
                "consent_cid",
                "enrollment",
                "domain",
                "authority",
                "identity_root",
                "device_key",
                "network_dna",
                "content_dna",
                "issued_at",
                "supersedes",
            ]
        );
        assert!(json.contains("\"issued_at\":123,"));
        assert!(json.ends_with("\"supersedes\":null}}"));
    }

    #[test]
    fn hashes_travel_as_the_zome_types_and_back() {
        let wire = approval_wire(&approval()).unwrap();
        assert_eq!(wire.consent.authority.to_string(), AUTHORITY);
        let intent = wire.enrollment.unwrap();
        assert_eq!(intent.device_key.to_string(), AGENT);
        assert_eq!(intent.network_dna.to_string(), NETWORK);
    }

    #[test]
    fn a_malformed_hash_is_refused_not_decoded() {
        let mut bad = approval();
        bad.authority = String::new();
        assert!(matches!(approval_wire(&bad), Err(CellFailure::Refused(_))));
        bad.authority = "uhCkknotahash".into();
        assert!(matches!(approval_wire(&bad), Err(CellFailure::Refused(_))));
    }

    #[test]
    fn a_zome_refusal_is_told_apart_from_an_unreachable_conductor() {
        assert!(matches!(
            failure(StorageError::Conductor(
                "Zome call failed: Guest(\"only a controller of this identity may agree for it\")"
                    .into()
            )),
            CellFailure::Refused(_)
        ));
        assert!(matches!(
            failure(StorageError::Connection("App connect failed".into())),
            CellFailure::Unavailable(_)
        ));
        assert!(matches!(
            failure(StorageError::NotFound(
                "mishpat cell not provisioned".into()
            )),
            CellFailure::Unavailable(_)
        ));
    }
}
