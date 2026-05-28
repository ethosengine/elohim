//! Conductor agent-info substrate gossip — propagates Holochain
//! `AgentInfoSigned` JSON strings across the libp2p mesh so every embedded
//! conductor's peer cache survives the Phase 1 doorway-A / doorway-B signal
//! partition, even when only one signal server is reachable from any given
//! pod's perspective.
//!
//! ## Design classification (per p2p-design-gate)
//!
//! Category C — operational. In-flight gossip envelope, never stored.
//! Receivers decode, verify, inject into the conductor's existing peer cache
//! via admin RPC, then drop. Lost messages are reconstructed by the next 60s
//! heartbeat. No persistence beyond the conductor's own internal store.
//!
//! Source of truth: the publishing peer's embedded conductor's
//! `admin_ws.agent_info(None)` admin RPC. The substrate gossip is purely a
//! transport mechanism; the conductor remains authoritative for signature
//! verification + dedup on `admin_ws.add_agent_info`.
//!
//! ## Spec
//!
//! `genesis/docs/superpowers/specs/2026-05-28-conductor-agent-info-substrate-gossip-design.md`

use serde::{Deserialize, Serialize};

/// Wire payload for the `elohim/conductor/agent-info/v1` gossipsub topic.
///
/// Carries an opaque kitsune2 v2 agent_info JSON string. Receiver passes
/// `agent_info_json` directly to `admin_ws.add_agent_info(vec![json])`
/// without inspecting its internals — the conductor itself does signature
/// verification + dedup. Edge handler does only cheap structural checks
/// (`verify_structural`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConductorAgentInfo {
    /// Opaque kitsune2 v2 agent_info JSON string. Publisher reads via
    /// `admin_ws.agent_info(None)`, receiver passes to
    /// `admin_ws.add_agent_info`. The substrate never cracks open the JSON.
    pub agent_info_json: String,
    /// Microsecond unix timestamp at publish. Subscriber uses this for
    /// last-seen dedup (drop messages older than the most-recent seen for the
    /// same peer key) and operators use it for observability (how stale is
    /// any given entry in the cache).
    pub published_at: i64,
}

impl ConductorAgentInfo {
    /// MessagePack encode (named fields — forward-compat for future fields).
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    /// MessagePack decode from gossipsub-received bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }

    /// Cheap structural check. Called on the gossip edge before try_sending
    /// into the bounded mpsc — drops obviously-malformed payloads before they
    /// reach the worker queue. Full validation happens in the conductor on
    /// `add_agent_info`.
    pub fn verify_structural(&self) -> Result<(), &'static str> {
        if self.agent_info_json.is_empty() {
            return Err("agent_info_json is empty");
        }
        if self.published_at <= 0 {
            return Err("published_at must be a positive microsecond timestamp");
        }
        Ok(())
    }
}

/// Gossipsub topic name. Use this constant at all publish/subscribe sites to
/// prevent compile-time typo drift.
pub const CONDUCTOR_AGENT_INFO_TOPIC: &str = "elohim/conductor/agent-info/v1";

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ConductorAgentInfo {
        ConductorAgentInfo {
            agent_info_json: r#"{"agent":"uhCAk_sample_pubkey","space":"uhC0k_sample_space","urls":["wss://signal.elohim.host/uhCAk_sample_pubkey"],"expires_at":1234567890}"#.to_string(),
            published_at: 1_700_000_000_000_000,
        }
    }

    #[test]
    fn roundtrip_preserves_payload() {
        let original = sample();
        let bytes = original.to_bytes().expect("to_bytes");
        let decoded = ConductorAgentInfo::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(original, decoded);
    }

    #[test]
    fn wire_bytes_are_small() {
        let bytes = sample().to_bytes().expect("to_bytes");
        // ~150-byte JSON payload + ~30 bytes envelope ≈ 180 bytes for this fixture.
        // Bound of 512 covers real kitsune2 agent_info (~400-600 bytes typical) with
        // some slack for unusual cases — but tight enough to catch runaway field
        // additions in future revisions.
        assert!(bytes.len() < 512, "payload should fit in 512B; got {} bytes", bytes.len());
    }

    #[test]
    fn verify_structural_passes_valid_payload() {
        assert_eq!(sample().verify_structural(), Ok(()));
    }

    #[test]
    fn verify_structural_rejects_empty_json() {
        let mut bad = sample();
        bad.agent_info_json = String::new();
        assert!(bad.verify_structural().is_err());
    }

    #[test]
    fn verify_structural_rejects_zero_timestamp() {
        let mut bad = sample();
        bad.published_at = 0;
        assert!(bad.verify_structural().is_err());
    }

    #[test]
    fn verify_structural_rejects_negative_timestamp() {
        let mut bad = sample();
        bad.published_at = -1;
        assert!(bad.verify_structural().is_err());
    }
}
