//! Peer identity mapping — stubbed for Phase 2c, replaced in Phase 2b.
//!
//! Maps a libp2p PeerId to an agent pubkey string for reach enforcement.
//! Without a real mapping, Collective/Steward/Private atoms cannot be served
//! cross-peer. That's deliberate: this phase exercises the code path;
//! Phase 2b wires real identity.
//!
//! Source of truth: operational — in-memory only. No persistence.
//! Reconstructable from libp2p session state.

use libp2p::PeerId;
use std::collections::HashMap;
use std::sync::RwLock;

/// Identity of the remote caller for reach enforcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallerIdentity {
    /// No identity established — serves only Commons/Public.
    Anonymous,
    /// Identity established; the string is a stable agent pubkey reference.
    Agent(String),
}

/// Trait allowing the swarm node to resolve a PeerId into a CallerIdentity.
/// Phase 2b replaces the stub with a real libp2p-identity-backed mapping.
pub trait PeerIdentityMap: Send + Sync + 'static {
    fn lookup(&self, peer: &PeerId) -> CallerIdentity;
}

/// In-memory stub — anonymous for all peers unless explicitly registered.
#[derive(Default, Debug)]
pub struct StubIdentityMap {
    inner: RwLock<HashMap<PeerId, String>>,
}

impl StubIdentityMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Test-only: register a peer → agent pubkey mapping.
    pub fn register(&self, peer: PeerId, agent_pubkey: impl Into<String>) {
        self.inner
            .write()
            .unwrap()
            .insert(peer, agent_pubkey.into());
    }
}

impl PeerIdentityMap for StubIdentityMap {
    fn lookup(&self, peer: &PeerId) -> CallerIdentity {
        self.inner
            .read()
            .unwrap()
            .get(peer)
            .cloned()
            .map(CallerIdentity::Agent)
            .unwrap_or(CallerIdentity::Anonymous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_anonymous_by_default() {
        let peer = PeerId::random();
        let map = StubIdentityMap::new();
        assert_eq!(map.lookup(&peer), CallerIdentity::Anonymous);
    }

    #[test]
    fn stub_returns_registered_agent() {
        let peer = PeerId::random();
        let map = StubIdentityMap::new();
        map.register(peer, "agent-pubkey-123");
        assert_eq!(
            map.lookup(&peer),
            CallerIdentity::Agent("agent-pubkey-123".to_string())
        );
    }
}
