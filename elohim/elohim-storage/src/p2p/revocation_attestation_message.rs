//! # RevocationAttestationMessage wire type
//!
//! **Source of truth:** Holochain DHT — `KeyRevocation` and `RevocationVote`
//! integrity entries (imagodei zome). This struct is a **Category C
//! operational projection** of those entries, serialized over the IntegrityNotify
//! libp2p direct-notify path. Per EPR D3 / M4 D2 (duality binding), the wire
//! shape mirrors the published `elohim/sdk/schemas/v1/dna-signals/revocation-
//! attestation.schema.json` contract exactly — no new schema, no envelope
//! inlining, no DHT round-trip per vote.
//!
//! **Lifetime:** transient. No persistence at this layer — canonical writes
//! happen via the local conductor's signal stream through the
//! `AttestationProjector` (for `attestation:revocation-vote` children landing
//! in `attestations`) and `RecoveryFlowProjector` (for `governance-action:
//! key-revocation` openers landing in `key_revocations`).
//!
//! **Producer:** Recovery M4 Stage 3 (post-commit hook).
//! **Consumer:** `epr_atom_service::handle_integrity_notify` "RevocationAttestation" arm.

use serde::{Deserialize, Serialize};

/// MessagePack wire payload for the `IntegrityNotify { kind: "RevocationAttestation" }`
/// direct-notify path.
///
/// Unifies two Recovery M4 signals:
/// - `RecoveryV2Signal::KeyRevocationRequested` (`attestation_kind = "request"`)
/// - `RecoveryV2Signal::RevocationVoteSubmitted` (`attestation_kind = "vote"`)
///
/// Field names mirror `revocation-attestation.schema.json` exactly (camelCase
/// via `serde(rename_all = "camelCase")`), so the wire format is consistent
/// across the direct-notify and signal-stream paths.
///
/// `signal_type` is always `"revocationAttestation"` per the schema `const`.
/// It is carried on the wire so a deserialiser can distinguish message types
/// without examining the `kind` field on the outer envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RevocationAttestationMessage {
    /// Always `"revocationAttestation"` — matches `signalType` const in schema.
    pub signal_type: String,
    /// Holochain ActionHash (base64) of the originating DHT entry.
    /// For `request` kind: the KeyRevocation entry action hash.
    /// For `vote` kind: the RevocationVote entry action hash.
    pub action_hash: String,
    /// Stable identifier linking this attestation to the overall KeyRevocation
    /// request. Corresponds to Recovery M4 `KeyRevocationRequested.id` /
    /// `RevocationVoteSubmitted.revocation_id`.
    pub revocation_id: String,
    /// Agent CID or pubkey of the steward submitting the attestation.
    /// For `request` kind: the initiating steward (`initiated_by` from M4).
    /// For `vote` kind: the voting steward (`steward_id` from M4).
    pub steward_id: String,
    /// Whether this attestation approves the revocation.
    /// For `request` kind this is always `true` (the request itself is an
    /// affirmative act). For `vote` kind this reflects the steward's vote.
    pub approved: bool,
    /// `"request"` or `"vote"` per schema enum.
    /// `"request"` maps from M4 `KeyRevocationRequested`.
    /// `"vote"` maps from M4 `RevocationVoteSubmitted`.
    pub attestation_kind: String,
    /// Current count of affirmative votes at the time this attestation was
    /// recorded.
    pub current_votes: u32,
    /// Vote threshold required for the revocation to become effective.
    pub required_votes: u32,
    /// Whether the vote threshold has been reached as of this attestation.
    /// When `true`, a `KeyRevocationSignal` should follow shortly after the
    /// conductor processes the threshold transition.
    pub threshold_reached: bool,
    /// UTC timestamp at which the attestation was recorded in the DNA.
    /// For `request` kind: `created_at` from M4 `KeyRevocationRequested`.
    /// For `vote` kind: `voted_at` from M4 `RevocationVoteSubmitted`.
    pub attested_at: String,
    /// UTC timestamp at which the signal was emitted from the conductor
    /// post-commit hook.
    pub emitted_at: String,
}

impl RevocationAttestationMessage {
    /// Encode to MessagePack bytes for IntegrityNotify direct-notify.
    ///
    /// Named-field MessagePack (struct-as-map) so adding/reordering fields
    /// doesn't silently break across rolling upgrades.
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// Decode from IntegrityNotify-received bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revocation_attestation_message_roundtrips_msgpack() {
        let original = RevocationAttestationMessage {
            signal_type: "revocationAttestation".into(),
            action_hash: "uhCkk...".into(),
            revocation_id: "u-revocation-1".into(),
            steward_id: "agent-cid-1".into(),
            approved: true,
            attestation_kind: "vote".into(),
            current_votes: 2,
            required_votes: 3,
            threshold_reached: false,
            attested_at: "2026-05-15T11:00:00Z".into(),
            emitted_at: "2026-05-15T11:00:01Z".into(),
        };

        let bytes = original.to_bytes().expect("encode");
        let decoded = RevocationAttestationMessage::from_bytes(&bytes).expect("decode");
        assert_eq!(decoded, original);
    }

    #[test]
    fn revocation_attestation_message_request_kind_roundtrips() {
        let original = RevocationAttestationMessage {
            signal_type: "revocationAttestation".into(),
            action_hash: "uhCkkRequest...".into(),
            revocation_id: "u-revocation-request-1".into(),
            steward_id: "initiating-steward-cid".into(),
            approved: true,
            attestation_kind: "request".into(),
            current_votes: 1,
            required_votes: 3,
            threshold_reached: false,
            attested_at: "2026-05-15T10:00:00Z".into(),
            emitted_at: "2026-05-15T10:00:01Z".into(),
        };

        let bytes = original.to_bytes().expect("encode");
        let decoded = RevocationAttestationMessage::from_bytes(&bytes).expect("decode");
        assert_eq!(decoded, original);
    }
}
