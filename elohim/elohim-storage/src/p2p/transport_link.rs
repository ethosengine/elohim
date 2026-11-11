//! The cross-signed link between a node's two transport keys: its libp2p identity
//! key (the one its `AgentPeerBinding` cross-signs) and its iroh node key.
//!
//! A cross-signed `AgentPeerBinding` (`p2p::binding_mint`) binds an agent to a
//! libp2p PeerId only: its transport half is signed with `identity.key`. The iroh
//! node key (`iroh.key`) is a separate key, so no binding names an iroh NodeId, and
//! a claim signed with the iroh key (custody announcements and salvage ads on a dual
//! node) had no authenticated path to an agent — only `peer_transport_manifest`,
//! whose `agent_cid` is a self-asserted routing hint.
//!
//! A [`TransportLink`] closes that gap without a second authority: BOTH keys sign
//! the same body over the elohim-epr transcript
//!
//! ```text
//! u16be(len(domain)) ‖ b"elohim:transport-link:v1:" ‖ u16be(epoch=0) ‖ u16be(scheme)
//!   ‖ u16be(len(key)) ‖ key ‖ dag-cbor({epoch, iroh_node_id, issued_at_ms, libp2p_peer_id})
//! ```
//!
//! so neither side can be claimed unilaterally: a libp2p key cannot link someone
//! else's iroh NodeId, and an iroh key cannot link someone else's PeerId. The agent
//! is then reached only through the libp2p PeerId's own cross-signed binding
//! (`p2p::transport_authority`): iroh NodeId → (this link) → PeerId → (cross-signed
//! `AgentPeerBinding`) → agent. Remove either step and the iroh author resolves to no
//! agent at all.
//!
//! The link rides the transport manifest as an additive field outside the manifest's
//! v1 signing bytes (`TransportManifestAnnouncement::transport_link`), so old readers
//! ignore it and keep verifying the routing payload.

use elohim_epr::proof::AgentKeypair;
use elohim_epr::signature::{SignatureCheck, SignatureEnvelope};
use serde::{Deserialize, Serialize};

use crate::p2p::custody_announce::transport_ids_of_key;

/// Signature domain of a transport link.
pub const TRANSPORT_LINK_DOMAIN: &[u8] = b"elohim:transport-link:v1:";

/// The link epoch this build reads and writes.
pub const TRANSPORT_LINK_EPOCH: u16 = 0;

/// How far ahead of the receiver's clock an `issued_at_ms` may be (clock skew).
pub const TRANSPORT_LINK_MAX_SKEW_MS: i64 = 5 * 60 * 1000;

/// Both transport keys of one node, each attesting the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportLink {
    /// iroh NodeId (64 lowercase hex: the ed25519 public key).
    pub iroh_node_id: String,
    /// libp2p PeerId (base58) of the node's identity key.
    pub libp2p_peer_id: String,
    /// When the link was signed (epoch milliseconds). Newest wins at the receiver.
    pub issued_at_ms: i64,
    /// Envelope epoch ([`TRANSPORT_LINK_EPOCH`]); also framed into the transcript.
    pub epoch: u16,
    /// Signature by the libp2p identity key over [`TransportLink::signed_body`].
    pub libp2p_sig: SignatureEnvelope,
    /// Signature by the iroh node key over the same body.
    pub iroh_sig: SignatureEnvelope,
}

/// The signed portion, in dag-cbor canonical key order (length, then bytes).
#[derive(Serialize)]
struct SignedLink<'a> {
    epoch: u16,
    iroh_node_id: &'a str,
    issued_at_ms: i64,
    libp2p_peer_id: &'a str,
}

/// Why a link does not verify.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LinkError {
    #[error("link epoch {0} is not one this build reads")]
    UnknownEpoch(u16),
    #[error("the libp2p half does not verify")]
    Libp2pSignature,
    #[error("the iroh half does not verify")]
    IrohSignature,
    #[error("the libp2p half is not signed by the key {0} names")]
    Libp2pKeyMismatch(String),
    #[error("the iroh half is not signed by the key {0} names")]
    IrohKeyMismatch(String),
    #[error("link issued at {0} ms is ahead of this clock beyond the skew bound")]
    FromTheFuture(i64),
}

impl TransportLink {
    /// The bytes both halves sign (after the transcript prefix).
    pub fn signed_body(
        epoch: u16,
        iroh_node_id: &str,
        libp2p_peer_id: &str,
        issued_at_ms: i64,
    ) -> Vec<u8> {
        let signed = SignedLink {
            epoch,
            iroh_node_id,
            issued_at_ms,
            libp2p_peer_id,
        };
        // Plain data (strings and integers): encoding cannot fail.
        serde_ipld_dagcbor::to_vec(&signed).unwrap_or_default()
    }

    /// Link this node's two keys. Returns `None` if the libp2p key does not spell a
    /// PeerId (it is always ed25519 in this crate, so this does not happen).
    pub fn sign(
        libp2p_key: &AgentKeypair,
        iroh_key: &AgentKeypair,
        issued_at_ms: i64,
    ) -> Option<Self> {
        let libp2p_public = libp2p_key.public_key_bytes();
        let libp2p_peer_id = libp2p::identity::ed25519::PublicKey::try_from_bytes(&libp2p_public)
            .ok()
            .map(|pk| {
                libp2p::identity::PublicKey::from(pk)
                    .to_peer_id()
                    .to_string()
            })?;
        let iroh_node_id = hex::encode(iroh_key.public_key_bytes());
        let body = Self::signed_body(
            TRANSPORT_LINK_EPOCH,
            &iroh_node_id,
            &libp2p_peer_id,
            issued_at_ms,
        );
        Some(Self {
            libp2p_sig: SignatureEnvelope::sign_ed25519(
                libp2p_key,
                TRANSPORT_LINK_DOMAIN,
                TRANSPORT_LINK_EPOCH,
                &body,
            ),
            iroh_sig: SignatureEnvelope::sign_ed25519(
                iroh_key,
                TRANSPORT_LINK_DOMAIN,
                TRANSPORT_LINK_EPOCH,
                &body,
            ),
            iroh_node_id,
            libp2p_peer_id,
            issued_at_ms,
            epoch: TRANSPORT_LINK_EPOCH,
        })
    }

    /// Both halves verify over the same body, each under the key its id names, and
    /// the link is not dated in the future. `now_ms` is the receiver's clock.
    pub fn verify(&self, now_ms: i64) -> Result<(), LinkError> {
        if self.epoch != TRANSPORT_LINK_EPOCH {
            return Err(LinkError::UnknownEpoch(self.epoch));
        }
        if self.issued_at_ms > now_ms.saturating_add(TRANSPORT_LINK_MAX_SKEW_MS) {
            return Err(LinkError::FromTheFuture(self.issued_at_ms));
        }
        let body = Self::signed_body(
            self.epoch,
            &self.iroh_node_id,
            &self.libp2p_peer_id,
            self.issued_at_ms,
        );
        // Each half's key must BE the id it speaks for.
        let libp2p_key = self
            .libp2p_sig
            .multikey()
            .map_err(|_| LinkError::Libp2pSignature)?;
        if transport_ids_of_key(&libp2p_key).first() != Some(&self.libp2p_peer_id) {
            return Err(LinkError::Libp2pKeyMismatch(self.libp2p_peer_id.clone()));
        }
        let iroh_key = self
            .iroh_sig
            .multikey()
            .map_err(|_| LinkError::IrohSignature)?;
        if iroh_key.ed25519_key().map(hex::encode).as_deref() != Some(self.iroh_node_id.as_str()) {
            return Err(LinkError::IrohKeyMismatch(self.iroh_node_id.clone()));
        }
        if self
            .libp2p_sig
            .check(TRANSPORT_LINK_DOMAIN, self.epoch, &body)
            != SignatureCheck::Verified
        {
            return Err(LinkError::Libp2pSignature);
        }
        if self
            .iroh_sig
            .check(TRANSPORT_LINK_DOMAIN, self.epoch, &body)
            != SignatureCheck::Verified
        {
            return Err(LinkError::IrohSignature);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;

    fn keys(a: u8, b: u8) -> (AgentKeypair, AgentKeypair) {
        (
            AgentKeypair::from_secret(&[a; 32]).unwrap(),
            AgentKeypair::from_secret(&[b; 32]).unwrap(),
        )
    }

    #[test]
    fn an_honest_link_verifies_and_names_both_keys() {
        let (libp2p, iroh) = keys(1, 2);
        let link = TransportLink::sign(&libp2p, &iroh, NOW).unwrap();
        assert_eq!(link.verify(NOW), Ok(()));
        assert_eq!(link.iroh_node_id, hex::encode(iroh.public_key_bytes()));
        assert!(link.libp2p_peer_id.starts_with("12D3Koo"));
    }

    #[test]
    fn neither_side_can_be_claimed_unilaterally() {
        let (libp2p, iroh) = keys(1, 2);
        let (_, victim_iroh) = keys(3, 4);
        // A libp2p key naming someone else's iroh NodeId: the iroh half is not the
        // victim's key.
        let mut forged = TransportLink::sign(&libp2p, &iroh, NOW).unwrap();
        forged.iroh_node_id = hex::encode(victim_iroh.public_key_bytes());
        assert!(forged.verify(NOW).is_err());
        // An iroh key naming someone else's PeerId.
        let (victim_libp2p, _) = keys(5, 6);
        let mut forged = TransportLink::sign(&libp2p, &iroh, NOW).unwrap();
        forged.libp2p_peer_id = TransportLink::sign(&victim_libp2p, &iroh, NOW)
            .unwrap()
            .libp2p_peer_id;
        assert!(forged.verify(NOW).is_err());
        // A half swapped in from another link.
        let other = TransportLink::sign(&victim_libp2p, &iroh, NOW).unwrap();
        let mut forged = TransportLink::sign(&libp2p, &iroh, NOW).unwrap();
        forged.libp2p_sig = other.libp2p_sig;
        assert!(forged.verify(NOW).is_err());
        // A re-dated link.
        let mut forged = TransportLink::sign(&libp2p, &iroh, NOW).unwrap();
        forged.issued_at_ms += 1;
        assert!(forged.verify(NOW).is_err());
    }

    #[test]
    fn a_future_dated_or_unknown_epoch_link_does_not_verify() {
        let (libp2p, iroh) = keys(1, 2);
        let future =
            TransportLink::sign(&libp2p, &iroh, NOW + TRANSPORT_LINK_MAX_SKEW_MS + 1).unwrap();
        assert!(matches!(
            future.verify(NOW),
            Err(LinkError::FromTheFuture(_))
        ));
        let mut epoch = TransportLink::sign(&libp2p, &iroh, NOW).unwrap();
        epoch.epoch = 1;
        assert!(matches!(epoch.verify(NOW), Err(LinkError::UnknownEpoch(1))));
    }
}
