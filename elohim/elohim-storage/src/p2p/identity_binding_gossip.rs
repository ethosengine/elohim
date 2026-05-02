//! Identity binding gossip wire contract — the payload published on the
//! `elohim/identity/binding` gossipsub topic when a local agent's
//! `AgentPeerBinding` DHT entry is committed.
//!
//! ## Design classification
//!
//! This payload is **Category C — operational**. It is a libp2p session-local
//! projection that wraps a Category-A notarized `AgentPeerBinding` DHT entry.
//! The gossip payload propagates DHT-truth to peers' `peer_identity_bindings`
//! operational table; it is NOT itself a new source of persistent truth.
//!
//! Source of truth: the Holochain `AgentPeerBinding` DHT entry referenced by
//! `binding_action_hash`. Peers receiving this payload should verify against
//! the DHT entry (Stage 2 / 3 concern — see signature field below).
//!
//! ## Stage 1 — structural-only verification
//!
//! Per memory pin `project_bootstrap_to_elohim_security_gradient`, Stage 1
//! verification is structural only:
//!   - `agent_cid` must be non-empty
//!   - `valid_from` must be a non-empty ISO-8601 string
//!   - `binding_action_hash` must be non-empty
//!   - `signature` must be non-empty (length not verified until Stage 2)
//!
//! Full Ed25519 verification against the agent's public key is deferred to
//! Stage 2 / 3 when the signer-CID resolver is available.
//!
//! ## Encoding
//!
//! MessagePack via `rmp-serde`. Matches the EPR-2B/C codec convention
//! (see `epr_protocol.rs`). Payloads are small (~200 bytes); no length
//! prefix is needed — gossipsub frames the message.

use serde::{Deserialize, Serialize};

/// Wire payload for the `elohim/identity/binding` gossipsub topic.
///
/// Carries the key fields from `AgentPeerBindingSignal` (see
/// `reconcile::signal_stream`), plus a `signature` field for future
/// Ed25519 verification (Stage 2 / 3). For Stage 1 the signature is a
/// non-empty sentinel value.
///
/// Note: the `action_hash` field present in `AgentPeerBindingSignal` is NOT
/// included here. `binding_action_hash` is the canonical DHT provenance field
/// for the gossip wire; the "uniform stream deserialization" justification only
/// applies to the conductor-side signal stream, not to P2P gossip payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityBindingGossip {
    /// multibase-encoded libp2p PeerId of the bound peer.
    pub peer_id: String,
    /// CID of the agent to which this peer is bound. Must be non-empty.
    pub agent_cid: String,
    /// ISO-8601 timestamp from which this binding is valid.
    /// Used as `valid_from` in the DB row. Must be non-empty.
    pub valid_from: String,
    /// ISO-8601 timestamp at which this binding expires. `None` = no expiry.
    /// Used as `valid_until` in the DB row.
    pub valid_until: Option<String>,
    /// Hardware/deployment archetype string (e.g. "node", "desktop", "mobile").
    pub device_archetype: String,
    /// Holochain ActionHash (base64) of the `AgentPeerBinding` DHT entry.
    /// Used as the `dht_anchor_hash` in the DB row.
    pub binding_action_hash: String,
    /// ISO-8601 timestamp at which the signal was emitted from the conductor.
    pub emitted_at: String,
    /// Ed25519 signature over the binding payload, base64-encoded.
    ///
    /// ## Stage 1 (current)
    /// Structural-only verification: checked non-empty only. The sentinel value
    /// "c3RhZ2UtMS1zaWduYXR1cmU=" (base64("stage-1-signature")) is used by the
    /// ReconcileController when constructing the gossip payload.
    ///
    /// ## Stage 2 (TODO — A.13 or later)
    /// Replace sentinel with a real Ed25519 signature over the canonical
    /// payload bytes. Verify at receive time using the agent's public key
    /// resolved from the signer-CID resolver.
    pub signature: String,
}

impl IdentityBindingGossip {
    /// Encode to MessagePack bytes for gossipsub publish.
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }

    /// Decode from gossipsub-received bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }

    /// Stage 1 structural verification.
    ///
    /// Returns `Ok(())` if the payload passes structural checks, or `Err(reason)`
    /// if a required field is empty. This mirrors A.9's structural-only pattern.
    ///
    /// ## Stage 2 extension point
    /// When Stage 2 Ed25519 verification lands, add resolver-backed pubkey
    /// lookup and `ed25519_dalek::VerifyingKey::verify_strict` here. Until
    /// then, only non-empty checks are enforced (Bootstrap Social, Stage 1).
    pub fn verify_structural(&self) -> Result<(), &'static str> {
        if self.agent_cid.is_empty() {
            return Err("agent_cid is empty");
        }
        if self.valid_from.is_empty() {
            return Err("valid_from is empty");
        }
        if self.binding_action_hash.is_empty() {
            return Err("binding_action_hash is empty");
        }
        if self.signature.is_empty() {
            return Err("signature is empty (Stage 1: must be non-empty sentinel)");
        }
        Ok(())
    }
}

/// Gossipsub topic name for identity binding propagation.
///
/// Published by `ReconcileController::on_agent_peer_binding` when a local
/// `AgentPeerBinding` DHT signal arrives. Subscribed in `behaviour.rs`.
/// Use this constant at all publish/subscribe/receive call sites to prevent
/// compile-time typo drift.
pub const IDENTITY_BINDING_TOPIC: &str = "elohim/identity/binding";

/// Sentinel signature value used by Stage 1 gossip publish.
///
/// base64("stage-1-signature") — non-empty, passes structural check,
/// clearly not a real signature. Replaced by a real Ed25519 signature in Stage 2.
pub const STAGE1_SIGNATURE_SENTINEL: &str = "c3RhZ2UtMS1zaWduYXR1cmU=";

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> IdentityBindingGossip {
        IdentityBindingGossip {
            peer_id: "12D3KooWExamplePeerId".into(),
            agent_cid: "bafybeicid-agent-gossip-test".into(),
            valid_from: "2026-04-25T00:00:00Z".into(),
            valid_until: None,
            device_archetype: "node".into(),
            binding_action_hash: "uhCkk-binding-action-hash".into(),
            emitted_at: "2026-04-25T00:00:01Z".into(),
            signature: STAGE1_SIGNATURE_SENTINEL.into(),
        }
    }

    #[test]
    fn roundtrip() {
        let payload = sample();
        let bytes = payload.to_bytes().unwrap();
        let decoded = IdentityBindingGossip::from_bytes(&bytes).unwrap();
        assert_eq!(payload, decoded);
    }

    #[test]
    fn roundtrip_with_valid_until() {
        let mut payload = sample();
        payload.valid_until = Some("2027-01-01T00:00:00Z".into());
        let bytes = payload.to_bytes().unwrap();
        let decoded = IdentityBindingGossip::from_bytes(&bytes).unwrap();
        assert_eq!(payload, decoded);
        assert_eq!(decoded.valid_until, Some("2027-01-01T00:00:00Z".into()));
    }

    #[test]
    fn wire_bytes_are_small() {
        // Heuristic: MessagePack should keep this under 350 bytes for a typical payload.
        let payload = sample();
        let bytes = payload.to_bytes().unwrap();
        assert!(
            bytes.len() < 350,
            "unexpected wire size: {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn verify_structural_passes_valid_payload() {
        let payload = sample();
        assert!(payload.verify_structural().is_ok());
    }

    #[test]
    fn verify_structural_rejects_empty_agent_cid() {
        let mut payload = sample();
        payload.agent_cid = String::new();
        assert!(payload.verify_structural().is_err());
    }

    #[test]
    fn verify_structural_rejects_empty_valid_from() {
        let mut payload = sample();
        payload.valid_from = String::new();
        assert!(payload.verify_structural().is_err());
    }

    #[test]
    fn verify_structural_rejects_empty_binding_action_hash() {
        let mut payload = sample();
        payload.binding_action_hash = String::new();
        assert!(payload.verify_structural().is_err());
    }

    #[test]
    fn verify_structural_rejects_empty_signature() {
        let mut payload = sample();
        payload.signature = String::new();
        assert!(payload.verify_structural().is_err());
    }

    /// T03b: gossip writer carries `device_archetype` from the wire payload.
    /// `superseded_by` stays `None` because the gossip wire format does not
    /// carry supersession state — only the DHT-arrival path is authoritative.
    #[test]
    fn gossip_writer_propagates_device_archetype() {
        // Decode a gossip payload announcing a "mobile" peer.
        let mut payload = sample();
        payload.device_archetype = "mobile".to_string();
        let bytes = payload.to_bytes().expect("encode");
        let decoded = IdentityBindingGossip::from_bytes(&bytes).expect("decode");
        assert!(decoded.verify_structural().is_ok());

        // Replicate the row construction performed by p2p/mod.rs at gossip
        // receive time. This codifies the field-flow contract that T03b wires:
        // device_archetype propagated; superseded_by always None at gossip time.
        let row = crate::db::models::NewPeerIdentityBindingRow {
            peer_id: decoded.peer_id.clone(),
            agent_cid: decoded.agent_cid.clone(),
            dht_anchor_hash: decoded.binding_action_hash.clone(),
            valid_from: decoded.valid_from.clone(),
            valid_until: decoded.valid_until.clone(),
            observed_at: "2026-04-25T12:00:00Z".to_string(),
            source: "gossip".to_string(),
            device_archetype: decoded.device_archetype.clone(),
            superseded_by: None,
        };
        assert_eq!(row.device_archetype, "mobile");
        assert!(row.superseded_by.is_none());
        assert_eq!(row.source, "gossip");
    }
}
