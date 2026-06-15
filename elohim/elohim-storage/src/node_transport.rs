//! Loosely-coupled transport-identity seam.
//!
//! ## Why this exists
//!
//! `self_cid` (the node's own steward identity) and the `peerId` reported on
//! `GET /p2p/status` are the JOIN KEY the resilience card depends on: the
//! provide-loop authors `replicates-content` Commitments with it as `provider`,
//! the custody sweep matches `commitment.provider == self_cid`, and the seeder
//! resolves provider/receiver from `/p2p/status .peerId`. All three must agree
//! or the snapshot's joins silently empty
//! (`project_resilience_snapshot_humans_junction`).
//!
//! Before this seam, `self_cid` was derived ONLY in libp2p mode (from
//! `NodeIdentity::peer_id_string()`), so in iroh mode it stayed `None` → the
//! provide-loop never spawned → the resilience card read all zeros. This trait
//! lets BOTH transports answer the same two questions through one interface, so
//! the (already transport-neutral) provide-loop spawns for whichever backend is
//! active, with NO branching or duplication of transport logic.
//!
//! ## Design (deliberately minimal — YAGNI)
//!
//! The trait carries exactly the two identity questions the boot path asks. It
//! does NOT model blob/gossip/sync routing — those stay in the libp2p `P2PNode`
//! and the iroh backends, which are runtime-selected, not unified here. Each
//! impl is a thin wrapper over a precomputed identity string; the actual key
//! material is loaded in the composition root (`main.rs`), which already holds
//! both the libp2p `NodeIdentity` and the iroh `SecretKey`. Keeping the impls
//! string-wrapping (rather than holding crate-specific key types) keeps this
//! module decoupled from both libp2p and iroh, and unit-testable with no
//! feature flags.
//!
//! Category C (operational): a per-process identity view, never persisted,
//! never notarized.

/// The transport-identity seam the boot path and `/p2p/status` consult.
///
/// `Send + Sync` so an `Arc<dyn NodeTransport>` can be threaded into async
/// tasks (the provide-loop) and the HTTP server state.
pub trait NodeTransport: Send + Sync {
    /// The node's stable self identity — the provider/commitment join key used
    /// to seed `config.self_cid` when `SELF_CID` is unset. `None` means the
    /// active transport could not produce a stable identity (the provide-loop
    /// then stays dormant, which is honest, not a regression).
    fn self_cid(&self) -> Option<String>;

    /// What `GET /p2p/status .peerId` reports for this transport. The seeder
    /// keys commitments to this value, so it MUST equal `self_cid` for a
    /// transport that participates in the resilience join.
    fn status_peer_id(&self) -> Option<String>;
}

/// libp2p transport identity. Wraps the libp2p peer id string
/// (`NodeIdentity::peer_id_string()`) the daemon already computes from
/// `identity.key`. Behavior-preserving: returns EXACTLY today's value.
#[derive(Debug, Clone)]
pub struct Libp2pTransport {
    peer_id: String,
}

impl Libp2pTransport {
    /// Construct from the libp2p peer id string. Pass the value of
    /// `NodeIdentity::peer_id_string()` — the identity the libp2p P2P node
    /// reports on `/p2p/status`.
    pub fn new(peer_id: impl Into<String>) -> Self {
        Self {
            peer_id: peer_id.into(),
        }
    }
}

impl NodeTransport for Libp2pTransport {
    fn self_cid(&self) -> Option<String> {
        if self.peer_id.is_empty() {
            None
        } else {
            Some(self.peer_id.clone())
        }
    }

    fn status_peer_id(&self) -> Option<String> {
        // libp2p: self_cid and the /p2p/status peerId are the same peer id.
        self.self_cid()
    }
}

/// iroh transport identity. Wraps the iroh `NodeId` string
/// (`SecretKey::public().to_string()`) loaded from `iroh.key`. This is the
/// same value the iroh view-fed service uses as the local agent CID, so the
/// resilience join is consistent across the iroh surface.
#[derive(Debug, Clone)]
pub struct IrohTransport {
    node_id: String,
}

impl IrohTransport {
    /// Construct from the iroh NodeId string. Pass
    /// `secret.public().to_string()` where `secret` is loaded via the same
    /// `load_or_generate_secret_key(iroh.key)` the iroh boot block uses
    /// (idempotent — same file ⇒ same id).
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
        }
    }
}

impl NodeTransport for IrohTransport {
    fn self_cid(&self) -> Option<String> {
        if self.node_id.is_empty() {
            None
        } else {
            Some(self.node_id.clone())
        }
    }

    fn status_peer_id(&self) -> Option<String> {
        // iroh: the NodeId is both the self identity and the /p2p/status peerId.
        self.self_cid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libp2p_transport_reports_peer_id_as_self_cid_and_status() {
        // A representative libp2p peer id (base58btc, the `12D3Koo…` shape).
        let peer_id = "12D3KooWExampleLibp2pPeerIdForTransportSeamTest";
        let t = Libp2pTransport::new(peer_id);
        // Both questions return the SAME libp2p peer id — behavior-preserving
        // (the seeder's join key == /p2p/status peerId == today's value).
        assert_eq!(t.self_cid(), Some(peer_id.to_string()));
        assert_eq!(t.status_peer_id(), Some(peer_id.to_string()));
        assert_eq!(t.self_cid(), t.status_peer_id());
    }

    #[test]
    fn iroh_transport_reports_node_id_and_self_cid_is_some() {
        // A representative iroh NodeId (hex-encoded ed25519 public key).
        let node_id = "ed25519iroh0node0id0example0for0transport0seam0test";
        let t = IrohTransport::new(node_id);
        // The precise thing that was failing before the seam: with an iroh
        // transport active, self_cid() is Some/non-empty, so the (already
        // transport-neutral) provide-loop can now spawn in iroh mode.
        let self_cid = t.self_cid();
        assert!(self_cid.is_some(), "iroh self_cid must be Some");
        assert!(
            !self_cid.as_deref().unwrap_or_default().is_empty(),
            "iroh self_cid must be non-empty so the provide-loop spawns"
        );
        assert_eq!(t.self_cid(), Some(node_id.to_string()));
        assert_eq!(t.status_peer_id(), Some(node_id.to_string()));
    }

    #[test]
    fn empty_identity_yields_none() {
        // Defensive: an empty string is treated as "no stable identity".
        assert_eq!(Libp2pTransport::new("").self_cid(), None);
        assert_eq!(IrohTransport::new("").self_cid(), None);
    }
}
