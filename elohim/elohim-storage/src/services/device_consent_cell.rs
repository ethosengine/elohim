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
//!
//! [`ConductorDeviceCell`] is the asking device's side: it reports the node's
//! own key and networks, signs possession of an enrollment intent with
//! `mishpat::sign_device_enrollment` under a mandate, and notarizes the binding
//! with `mishpat::enroll_identity_device`.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use consent_grant::{ControllerProof, ControllerStanding, Enrollment, EnrollmentIntent};
use holochain_client::CellId;
use holochain_types::prelude::{ActionHash, AgentPubKey, DnaHash, ExternIO, Signature, Timestamp};
use serde::{Deserialize, Serialize};

use super::device_consent::{
    ApprovalProofs, ApprovalRequest, BindingReceipt, CellFailure, CellStanding, ControllerCell,
    DeviceCell, DeviceSelf, NewHuman,
};
use crate::error::StorageError;
use crate::hc_client::{HcClient, MISHPAT_ROLE};

const MISHPAT_ZOME: &str = "mishpat";
const SIGN_APPROVAL: &str = "sign_device_approval";
const BOOTSTRAP: &str = "bootstrap_device_identity";
const SIGN_ENROLLMENT: &str = "sign_device_enrollment";
const ENROLL: &str = "enroll_identity_device";
const DEVICES: &str = "identity_devices";
const AFFIRM: &str = "affirm_identity_device";
const SIGN_CARRIER: &str = "sign_carrier_statement";
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

#[derive(Serialize, Deserialize, Debug)]
struct ProofWire {
    agent: AgentPubKey,
    signature: Signature,
}

#[derive(Serialize, Debug)]
struct DeviceBindingWire {
    action: String,
    binding_kind: String,
    intent: DeviceIntentWire,
    controllers: Vec<ProofWire>,
    possession: ProofWire,
    /// Omitted when every approver is a root controller, so the zome reads
    /// the same bytes it always did.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    approved_via: Vec<ApprovedViaWire>,
}

/// A device approver and the joining record it speaks through.
#[derive(Serialize, Deserialize, Debug)]
struct ApprovedViaWire {
    agent: AgentPubKey,
    binding: ActionHash,
}

/// The imagodei coordinator's `CreateHumanInput`.
#[derive(Serialize, Debug)]
struct CreateHumanWire<'a> {
    id: &'a str,
    display_name: &'a str,
    bio: Option<String>,
    affinities: Vec<String>,
    profile_reach: &'a str,
    location: Option<String>,
}

/// The imagodei coordinator's `RegisterDeviceIdentityInput`.
#[derive(Serialize, Debug)]
struct RegisterWire {
    binding: ActionHash,
    expected_content_dna: DnaHash,
}

/// The imagodei coordinator's `HumanOutput`, as far as a declaration compares.
#[derive(Deserialize, Debug)]
struct HumanOutputWire {
    human: HumanWire,
}

#[derive(Deserialize, Debug)]
struct HumanWire {
    id: String,
    display_name: String,
    profile_reach: String,
}

#[derive(Deserialize, Debug)]
struct CommitmentOutputWire {
    action_hash: ActionHash,
    entry_hash: holochain_types::prelude::EntryHash,
}

#[derive(Deserialize, Debug)]
struct DeviceApprovalProofsWire {
    consent: ProofWire,
    enrollment: Option<ProofWire>,
    #[serde(default)]
    via: Option<ActionHash>,
}

#[derive(Deserialize, Debug)]
struct ConsentStandingWire {
    identity_root: ActionHash,
    authority: Option<ActionHash>,
    controllers: Vec<AgentPubKey>,
    required: usize,
    network_dna: DnaHash,
    #[serde(default)]
    speaks_via: Option<ActionHash>,
    #[serde(default)]
    also_speaks_for: Vec<ActionHash>,
}

/// The mishpat coordinator's `IdentityDevice`.
#[derive(Deserialize, Debug)]
struct IdentityDeviceWire {
    device_key: AgentPubKey,
    binding: ActionHash,
    content_dna: DnaHash,
    approved_by: Vec<AgentPubKey>,
    joined_at: Timestamp,
    affirmed_by: Vec<AgentPubKey>,
}

/// The mishpat coordinator's `IdentityDevices`.
#[derive(Deserialize, Debug)]
struct IdentityDevicesWire {
    #[allow(dead_code)]
    identity_root: ActionHash,
    roots: Vec<AgentPubKey>,
    devices: Vec<IdentityDeviceWire>,
    not_standing: u32,
    truncated: bool,
}

/// The mishpat coordinator's `CarrierStatement`. Field order matters: the
/// mandate carries its exact JSON.
#[derive(Serialize, Debug)]
struct CarrierStatementWire<'a> {
    kind: &'a str,
    state: &'a str,
    transport_id: &'a str,
    challenge: &'a str,
}

/// Sign a carrier statement with `cell`'s key, under a mandate for this one
/// call.
async fn sign_carrier_with(
    hc: &HcClient,
    cell: &CellId,
    kind: consent_grant::CarrierKind,
    state: &str,
    transport_id: &str,
    challenge: &str,
) -> Result<consent_grant::CarrierProof, CellFailure> {
    let wire = CarrierStatementWire {
        kind: kind.as_str(),
        state,
        transport_id,
        challenge,
    };
    let json = serde_json::to_string(&wire)
        .map_err(|e| CellFailure::Unavailable(format!("statement encoding: {e}")))?;
    let payload = ExternIO::encode(&wire)
        .map_err(|e| CellFailure::Unavailable(format!("statement encoding: {e}")))?
        .into_vec();
    let answer = hc
        .call_zome_mandated(
            cell,
            MISHPAT_ZOME,
            SIGN_CARRIER,
            payload,
            json,
            MANDATE_WINDOW,
        )
        .await
        .map_err(failure)?;
    let proof: ProofWire = ExternIO::from(answer)
        .decode()
        .map_err(|e| CellFailure::Unavailable(format!("statement proof decode: {e}")))?;
    let proof = self::proof(proof);
    Ok(consent_grant::CarrierProof {
        signer: proof.agent,
        signature: proof.signature,
    })
}

/// The qahal `VerifyDeviceInput` that `affirm_identity_device` takes. Field
/// order matters: the mandate carries its exact JSON.
#[derive(Serialize, Debug)]
struct VerifyDeviceWire {
    binding: ActionHash,
    expected_device: AgentPubKey,
    expected_content_dna: DnaHash,
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

fn intent_wire(i: &EnrollmentIntent) -> Result<DeviceIntentWire, CellFailure> {
    Ok(DeviceIntentWire {
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
}

fn approval_wire(approval: &ApprovalRequest) -> Result<DeviceApprovalWire, CellFailure> {
    let enrollment = approval.enrollment.as_ref().map(intent_wire).transpose()?;
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
                None => CellStanding::Unbootstrapped {
                    identity_root: s.identity_root.to_string(),
                },
                Some(authority) => CellStanding::Ready(ControllerStanding {
                    identity_root: s.identity_root.to_string(),
                    authority: authority.to_string(),
                    network_dna: s.network_dna.to_string(),
                    controllers: s.controllers.iter().map(ToString::to_string).collect(),
                    required: s.required,
                    speaks_via: s.speaks_via.map(|v| v.to_string()),
                    also_speaks_for: s.also_speaks_for.iter().map(ToString::to_string).collect(),
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
            via: proofs.via.map(|v| v.to_string()),
        })
    }

    async fn devices(
        &self,
        identity_root: &str,
    ) -> Result<consent_grant::DevicesRead, CellFailure> {
        let root: ActionHash = hash(identity_root, "identity")?;
        let payload = ExternIO::encode(&root)
            .map_err(|e| CellFailure::Unavailable(e.to_string()))?
            .into_vec();
        let answer = self
            .hc
            .call_zome_mishpat(MISHPAT_ZOME, DEVICES, payload)
            .await
            .map_err(failure)?;
        let read: IdentityDevicesWire = ExternIO::from(answer)
            .decode()
            .map_err(|e| CellFailure::Unavailable(format!("devices decode: {e}")))?;
        let me = self.agent();
        Ok(consent_grant::DevicesRead {
            roots: read.roots.iter().map(ToString::to_string).collect(),
            devices: read
                .devices
                .into_iter()
                .map(|d| {
                    let device_key = d.device_key.to_string();
                    consent_grant::StandingDevice {
                        device_fingerprint: consent_grant::hash_shape::fingerprint(&device_key),
                        this_device: device_key == me,
                        device_key,
                        binding: d.binding.to_string(),
                        content_dna: d.content_dna.to_string(),
                        joined_at: d.joined_at.as_millis(),
                        approved_by: d.approved_by.iter().map(ToString::to_string).collect(),
                        affirmed_count: d.affirmed_by.len(),
                        affirmed_by: d.affirmed_by.iter().map(ToString::to_string).collect(),
                    }
                })
                .collect(),
            not_standing: read.not_standing,
            truncated: read.truncated,
        })
    }

    async fn sign_carrier(
        &self,
        kind: consent_grant::CarrierKind,
        state: &str,
        transport_id: &str,
        challenge: &str,
    ) -> Result<consent_grant::CarrierProof, CellFailure> {
        sign_carrier_with(&self.hc, &self.cell, kind, state, transport_id, challenge).await
    }

    async fn affirm(&self, device: &consent_grant::StandingDevice) -> Result<(), CellFailure> {
        let wire = VerifyDeviceWire {
            binding: hash(&device.binding, "joining record")?,
            expected_device: hash(&device.device_key, "device")?,
            expected_content_dna: hash(&device.content_dna, "content network")?,
        };
        let json = serde_json::to_string(&wire)
            .map_err(|e| CellFailure::Unavailable(format!("affirmation encoding: {e}")))?;
        let payload = ExternIO::encode(&wire)
            .map_err(|e| CellFailure::Unavailable(format!("affirmation encoding: {e}")))?
            .into_vec();
        self.hc
            .call_zome_mandated(
                &self.cell,
                MISHPAT_ZOME,
                AFFIRM,
                payload,
                json,
                MANDATE_WINDOW,
            )
            .await
            .map_err(failure)?;
        Ok(())
    }

    async fn my_human(&self) -> Result<Option<consent_grant::ExistingIdentity>, CellFailure> {
        if self.hc.cell_id_for_role(MISHPAT_ROLE) != Some(&self.cell) {
            return Err(CellFailure::Unavailable(
                "this node reads a Human only for its own cell".into(),
            ));
        }
        let payload = ExternIO::encode(())
            .map_err(|e| CellFailure::Unavailable(e.to_string()))?
            .into_vec();
        let answer = self
            .hc
            .call_zome_imagodei("imagodei", "get_my_human", payload)
            .await
            .map_err(failure)?;
        let human: Option<HumanOutputWire> = ExternIO::from(answer)
            .decode()
            .map_err(|e| CellFailure::Unavailable(format!("human decode: {e}")))?;
        Ok(human.map(|h| consent_grant::ExistingIdentity {
            human_id: h.human.id,
            display_name: h.human.display_name,
            profile_reach: h.human.profile_reach,
        }))
    }

    async fn create_human(&self, human: &NewHuman) -> Result<(), CellFailure> {
        // Ordinary credentials reach the client's own imagodei cell only.
        if self.hc.cell_id_for_role(MISHPAT_ROLE) != Some(&self.cell) {
            return Err(CellFailure::Unavailable(
                "this node creates a Human only for its own cell".into(),
            ));
        }
        let payload = ExternIO::encode(CreateHumanWire {
            id: &human.id,
            display_name: &human.display_name,
            bio: None,
            affinities: Vec::new(),
            profile_reach: &human.profile_reach,
            location: None,
        })
        .map_err(|e| CellFailure::Unavailable(format!("human encoding: {e}")))?
        .into_vec();
        self.hc
            .call_zome_imagodei("imagodei", "create_human", payload)
            .await
            .map_err(failure)?;
        Ok(())
    }

    async fn bootstrap(&self, identity_root: &str) -> Result<(), CellFailure> {
        let human: ActionHash = hash(identity_root, "identity")?;
        let json = serde_json::to_string(&human)
            .map_err(|e| CellFailure::Unavailable(format!("bootstrap encoding: {e}")))?;
        let payload = ExternIO::encode(&human)
            .map_err(|e| CellFailure::Unavailable(format!("bootstrap encoding: {e}")))?
            .into_vec();
        self.hc
            .call_zome_mandated(
                &self.cell,
                MISHPAT_ZOME,
                BOOTSTRAP,
                payload,
                json,
                MANDATE_WINDOW,
            )
            .await
            .map_err(failure)?;
        Ok(())
    }
}

/// The asking device's own mishpat cell on this node's conductor.
pub struct ConductorDeviceCell {
    hc: Arc<HcClient>,
    cell: CellId,
}

impl ConductorDeviceCell {
    /// This node's own mishpat cell, when the installed hApp has one.
    pub fn own(hc: Arc<HcClient>) -> Option<Self> {
        let cell = hc.cell_id_for_role(MISHPAT_ROLE)?.clone();
        Some(Self { hc, cell })
    }
}

fn signature(text: &str) -> Result<Signature, CellFailure> {
    let raw: [u8; 64] = URL_SAFE_NO_PAD
        .decode(text)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| CellFailure::Refused("a proof is not a signature".into()))?;
    Ok(Signature(raw))
}

#[async_trait]
impl DeviceCell for ConductorDeviceCell {
    async fn sign_carrier(
        &self,
        kind: consent_grant::CarrierKind,
        state: &str,
        transport_id: &str,
        challenge: &str,
    ) -> Result<consent_grant::CarrierProof, CellFailure> {
        sign_carrier_with(&self.hc, &self.cell, kind, state, transport_id, challenge).await
    }

    async fn whoami(&self) -> Result<DeviceSelf, CellFailure> {
        Ok(DeviceSelf {
            device_key: self.cell.agent_pubkey().to_string(),
            network_dna: self.cell.dna_hash().to_string(),
            content_dna: self.hc.cell_id().dna_hash().to_string(),
        })
    }

    async fn enroll(&self, enrollment: &Enrollment) -> Result<BindingReceipt, CellFailure> {
        // Whether this node has a Human of its own. One that does joins as it
        // is: its binding is notarized but not registered as its identity, so
        // its key keeps resolving to the Human it began and what it made keeps
        // tracing there (registering would switch that resolution to the
        // person it joined; the device_enrollment sweettest shows both).
        let has_own_human = {
            let payload = ExternIO::encode(())
                .map_err(|e| CellFailure::Unavailable(e.to_string()))?
                .into_vec();
            let answer = self
                .hc
                .call_zome_imagodei("imagodei", "get_my_human", payload)
                .await
                .map_err(failure)?;
            let human: Option<HumanOutputWire> = ExternIO::from(answer)
                .decode()
                .map_err(|e| CellFailure::Unavailable(format!("human decode: {e}")))?;
            human.is_some()
        };
        let intent = intent_wire(&enrollment.intent)?;
        let json = serde_json::to_string(&intent)
            .map_err(|e| CellFailure::Unavailable(format!("intent encoding: {e}")))?;
        let payload = ExternIO::encode(&intent)
            .map_err(|e| CellFailure::Unavailable(format!("intent encoding: {e}")))?
            .into_vec();
        let answer = self
            .hc
            .call_zome_mandated(
                &self.cell,
                MISHPAT_ZOME,
                SIGN_ENROLLMENT,
                payload,
                json,
                MANDATE_WINDOW,
            )
            .await
            .map_err(failure)?;
        let possession: ProofWire = ExternIO::from(answer)
            .decode()
            .map_err(|e| CellFailure::Unavailable(format!("possession decode: {e}")))?;
        let controllers = enrollment
            .controllers
            .iter()
            .map(|p| {
                Ok(ProofWire {
                    agent: hash(&p.agent, "controller")?,
                    signature: signature(&p.signature)?,
                })
            })
            .collect::<Result<Vec<_>, CellFailure>>()?;
        let binding = DeviceBindingWire {
            action: "binds-identity".into(),
            binding_kind: "device-v1".into(),
            intent,
            controllers,
            possession,
            approved_via: enrollment
                .approved_via
                .iter()
                .map(|v| {
                    Ok(ApprovedViaWire {
                        agent: hash(&v.agent, "approver")?,
                        binding: hash(&v.binding, "approver record")?,
                    })
                })
                .collect::<Result<Vec<_>, CellFailure>>()?,
        };
        let payload = ExternIO::encode(&binding)
            .map_err(|e| CellFailure::Unavailable(format!("binding encoding: {e}")))?
            .into_vec();
        let answer = self
            .hc
            .call_zome_mishpat(MISHPAT_ZOME, ENROLL, payload)
            .await
            .map_err(failure)?;
        let receipt: CommitmentOutputWire = ExternIO::from(answer)
            .decode()
            .map_err(|e| CellFailure::Unavailable(format!("receipt decode: {e}")))?;
        if !has_own_human {
            // A node with no identity of its own now knows whose it is: its key
            // resolves to the person it joined (one AgentKeyToHuman link).
            let payload = ExternIO::encode(RegisterWire {
                binding: receipt.action_hash.clone(),
                expected_content_dna: self.hc.cell_id().dna_hash().clone(),
            })
            .map_err(|e| CellFailure::Unavailable(format!("register encoding: {e}")))?
            .into_vec();
            self.hc
                .call_zome_imagodei("imagodei", "register_device_identity", payload)
                .await
                .map_err(failure)?;
        }
        Ok(BindingReceipt {
            binding_action: receipt.action_hash.to_string(),
            binding_entry: receipt.entry_hash.to_string(),
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

    /// The bytes a proof is checked against by a terminal (`consent_grant`) are
    /// the bytes the zome signs: its `serde_json` of the same intent.
    #[test]
    fn the_terminal_checks_the_bytes_the_zome_signs() {
        let intent = approval().enrollment.unwrap();
        let zome_bytes = serde_json::to_vec(&intent_wire(&intent).unwrap()).unwrap();
        assert_eq!(intent.signed_bytes().unwrap(), zome_bytes);
    }

    #[test]
    fn a_bootstrap_payload_is_the_humans_hash() {
        let human: ActionHash = hash(IDENTITY, "identity").unwrap();
        let json = serde_json::to_string(&human).unwrap();
        assert!(json.starts_with("[132,41,36,"), "{json}");
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
