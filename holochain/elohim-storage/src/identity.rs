//! Node Identity - Links libp2p PeerId to Holochain AgentPubKey
//!
//! This module provides the identity bridge between elohim-storage's P2P
//! network (rust-libp2p) and Holochain's agent identity system.
//!
//! ## Why Two Identities?
//!
//! | Layer | Identity | Purpose |
//! |-------|----------|---------|
//! | Holochain | AgentPubKey | Provenance, permissions, DHT |
//! | P2P Storage | libp2p PeerId | Shard transfer, routing |
//!
//! Both are ed25519 keys, but serve different protocol layers.
//! The ContentServer zome links them in the Holochain DHT.

#[cfg(feature = "p2p")]
use libp2p::{identity::Keypair, PeerId};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::StorageError;

/// Node capabilities for P2P participation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Can store blobs locally
    pub storage: bool,
    /// Always-on (home node) vs intermittent (laptop)
    pub always_on: bool,
    /// Maximum storage capacity in bytes
    pub max_storage_bytes: u64,
    /// Willing to serve family members
    pub serve_family: bool,
    /// Willing to serve anyone (network mode)
    pub serve_public: bool,
}

impl Default for NodeCapabilities {
    fn default() -> Self {
        Self {
            storage: true,
            always_on: false,
            max_storage_bytes: 10 * 1024 * 1024 * 1024, // 10 GB default
            serve_family: false,
            serve_public: false,
        }
    }
}

impl NodeCapabilities {
    /// Capabilities for a laptop (intermittent, personal storage)
    pub fn laptop() -> Self {
        Self {
            storage: true,
            always_on: false,
            max_storage_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            serve_family: false,
            serve_public: false,
        }
    }

    /// Capabilities for a home node (always-on, serves family)
    pub fn home_node() -> Self {
        Self {
            storage: true,
            always_on: true,
            max_storage_bytes: 100 * 1024 * 1024 * 1024, // 100 GB
            serve_family: true,
            serve_public: false,
        }
    }

    /// Capabilities for a network node (public service)
    pub fn network_node() -> Self {
        Self {
            storage: true,
            always_on: true,
            max_storage_bytes: 500 * 1024 * 1024 * 1024, // 500 GB
            serve_family: true,
            serve_public: true,
        }
    }

    /// Encode capabilities as JSON for ContentServer registration
    pub fn to_json(&self) -> Result<String, StorageError> {
        serde_json::to_string(self).map_err(StorageError::Json)
    }
}

/// Node identity linking libp2p PeerId to Holochain AgentPubKey
#[cfg(feature = "p2p")]
#[derive(Clone)]
pub struct NodeIdentity {
    /// libp2p ed25519 keypair for P2P operations
    keypair: Keypair,
    /// libp2p peer identifier (derived from keypair)
    peer_id: PeerId,
    /// Holochain agent public key (base64)
    agent_pubkey: String,
    /// Node capabilities
    capabilities: NodeCapabilities,
    /// Geographic region for latency-based routing
    region: Option<String>,
}

#[cfg(feature = "p2p")]
impl NodeIdentity {
    /// Create a new identity with a fresh keypair
    pub fn generate(agent_pubkey: String) -> Result<Self, StorageError> {
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        Ok(Self {
            keypair,
            peer_id,
            agent_pubkey,
            capabilities: NodeCapabilities::default(),
            region: None,
        })
    }

    /// Create identity from an existing keypair
    pub fn from_keypair(keypair: Keypair, agent_pubkey: String) -> Result<Self, StorageError> {
        let peer_id = PeerId::from(keypair.public());

        Ok(Self {
            keypair,
            peer_id,
            agent_pubkey,
            capabilities: NodeCapabilities::default(),
            region: None,
        })
    }

    /// Load identity from a file (or generate if not exists)
    pub fn load_or_generate(
        path: &Path,
        agent_pubkey: String,
    ) -> Result<Self, StorageError> {
        if path.exists() {
            Self::load(path, agent_pubkey)
        } else {
            let identity = Self::generate(agent_pubkey)?;
            identity.save(path)?;
            Ok(identity)
        }
    }

    /// Load identity keypair from file
    pub fn load(path: &Path, agent_pubkey: String) -> Result<Self, StorageError> {
        let bytes = std::fs::read(path)?;

        // Decode the keypair from protobuf format
        let keypair = Keypair::from_protobuf_encoding(&bytes)
            .map_err(|e| StorageError::Identity(format!("Failed to decode keypair: {}", e)))?;

        Self::from_keypair(keypair, agent_pubkey)
    }

    /// Save identity keypair to file
    pub fn save(&self, path: &Path) -> Result<(), StorageError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Encode keypair to protobuf format
        let bytes = self.keypair.to_protobuf_encoding()
            .map_err(|e| StorageError::Identity(format!("Failed to encode keypair: {}", e)))?;

        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Get the libp2p keypair
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Get the libp2p PeerId
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    /// Get the PeerId as a string
    pub fn peer_id_string(&self) -> String {
        self.peer_id.to_string()
    }

    /// Get the Holochain agent public key
    pub fn agent_pubkey(&self) -> &str {
        &self.agent_pubkey
    }

    /// Get node capabilities
    pub fn capabilities(&self) -> &NodeCapabilities {
        &self.capabilities
    }

    /// Set node capabilities
    pub fn set_capabilities(&mut self, capabilities: NodeCapabilities) {
        self.capabilities = capabilities;
    }

    /// Get the region
    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    /// Set the region
    pub fn set_region(&mut self, region: Option<String>) {
        self.region = region;
    }

    /// Check if this node should serve a given agent
    pub fn should_serve(&self, requester_agent: &str) -> bool {
        if self.capabilities.serve_public {
            return true;
        }

        // TODO: Check if requester is in family list
        // For now, serve_family means serve anyone (will be refined in Phase 3)
        self.capabilities.serve_family
    }
}

/// Serializable identity info (for ContentServer registration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentityInfo {
    /// libp2p PeerId (base58)
    pub peer_id: String,
    /// Holochain agent public key
    pub agent_pubkey: String,
    /// Node capabilities as JSON
    pub capabilities_json: String,
    /// Geographic region
    pub region: Option<String>,
}

#[cfg(feature = "p2p")]
impl From<&NodeIdentity> for NodeIdentityInfo {
    fn from(identity: &NodeIdentity) -> Self {
        Self {
            peer_id: identity.peer_id_string(),
            agent_pubkey: identity.agent_pubkey.clone(),
            capabilities_json: identity.capabilities.to_json().unwrap_or_default(),
            region: identity.region.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_presets() {
        let laptop = NodeCapabilities::laptop();
        assert!(!laptop.always_on);
        assert!(!laptop.serve_family);
        assert!(!laptop.serve_public);

        let home = NodeCapabilities::home_node();
        assert!(home.always_on);
        assert!(home.serve_family);
        assert!(!home.serve_public);

        let network = NodeCapabilities::network_node();
        assert!(network.always_on);
        assert!(network.serve_family);
        assert!(network.serve_public);
    }

    #[test]
    fn test_capabilities_json() {
        let caps = NodeCapabilities::laptop();
        let json = caps.to_json().unwrap();
        assert!(json.contains("\"storage\":true"));
        assert!(json.contains("\"always_on\":false"));
    }

    #[cfg(feature = "p2p")]
    #[test]
    fn test_identity_generation() {
        let identity = NodeIdentity::generate("uhCAk_test_agent".to_string()).unwrap();

        // PeerId should be valid
        assert!(!identity.peer_id_string().is_empty());

        // Agent pubkey should match
        assert_eq!(identity.agent_pubkey(), "uhCAk_test_agent");
    }

    #[cfg(feature = "p2p")]
    #[test]
    fn test_identity_save_load() {
        let temp_dir = std::env::temp_dir();
        let key_path = temp_dir.join("test_identity.key");

        // Generate and save
        let identity1 = NodeIdentity::generate("uhCAk_test_save".to_string()).unwrap();
        identity1.save(&key_path).unwrap();

        // Load
        let identity2 = NodeIdentity::load(&key_path, "uhCAk_test_save".to_string()).unwrap();

        // Same PeerId
        assert_eq!(identity1.peer_id_string(), identity2.peer_id_string());

        // Cleanup
        std::fs::remove_file(key_path).ok();
    }
}
