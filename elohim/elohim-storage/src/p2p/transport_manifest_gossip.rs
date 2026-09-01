//! `elohim/transport/manifest` — how a dual-stack peer tells the mesh where
//! its iroh endpoint lives.
//!
//! **The gap this closes.** libp2p discovers peers by itself (mDNS,
//! Kademlia, bootstrap list, identify). iroh does not: its `Endpoint` only
//! dials a `NodeAddr` it has been handed, or one a discovery service
//! resolves — and the sovereign defaults register NO discovery service
//! (`p2p_iroh::config`). So on a dual-mode node the iroh plane came up with
//! every responder mounted and zero peers to dial, and stayed that way.
//!
//! **The mechanism.** Each node periodically publishes a signed
//! [`TransportManifestAnnouncement`] over the gossip fan-out (libp2p
//! gossipsub today; dual-published to iroh-gossip so once the iroh plane has
//! neighbors it carries the same message). Receivers verify the signature
//! against the announced iroh NodeId (which IS an ed25519 public key), then
//! upsert the peer into the `IrohPeerBook` and the durable
//! `peer_transport_manifest` row. The iroh side of the node watches the book
//! and does `Endpoint::add_node_addr` + gossip `join_peers` for every new
//! entry — the membership layer lights, and the round driver has targets.
//!
//! **Trust.** The v1 signature binds the dial/routing payload to the iroh key.
//! A peer can only ever advertise addresses for a NodeId it holds the secret
//! for; a third party cannot steer our dials to an address of its choosing.
//! The additive `user_agent` observation is deliberately outside those v1
//! signing bytes so old readers can ignore it and still verify the routing
//! payload. It is advisory evidence only, never authority or capability
//! negotiation. The signature also does NOT prove that `agent_cid` /
//! `libp2p_peer_id` belong to the same operator — that binding is the
//! identity-binding gossip's job (DHT-anchored). Here they are routing hints,
//! never authority.
//!
//! This is a transport-neutral wire type (sibling of `custody_announce`); it
//! depends on ed25519 only, never on the iroh crate, so it compiles with
//! `p2p` alone and the receive arm can exist on libp2p-only builds (where it
//! simply records the row and has no book to feed).

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Gossip topic name. `elohim/` prefix → `classify_topic` verdict Dual.
pub const TRANSPORT_MANIFEST_TOPIC: &str = "elohim/transport/manifest";

/// Default cadence of the self-announcement (seconds). The libp2p identify
/// exchange re-runs on every new connection; this is deliberately slower —
/// it carries addresses, which change rarely, and gossip is flood-cost.
pub const DEFAULT_ANNOUNCE_INTERVAL_SECS: u64 = 30;

/// Env override for the announce cadence.
pub const ANNOUNCE_INTERVAL_ENV: &str = "ELOHIM_TRANSPORT_MANIFEST_INTERVAL_SECS";

/// One peer's self-description of where its iroh endpoint can be reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportManifestAnnouncement {
    /// iroh NodeId — 64 lowercase hex chars of the ed25519 public key. The
    /// signature verifies against THIS.
    pub iroh_node_id: String,
    /// Direct socket addresses (`ip:port`) the endpoint is bound/reachable on.
    pub iroh_direct_addrs: Vec<String>,
    /// Home relay URL, when a relay is configured.
    pub iroh_relay_url: Option<String>,
    /// DHT-anchored agent identity (routing hint, not authority).
    pub agent_cid: Option<String>,
    /// libp2p PeerId (base58) when dual-stack (routing hint).
    pub libp2p_peer_id: Option<String>,
    /// Planes this node serves over iroh (kebab-case: `blob`, `gossip`,
    /// `sync`, `epr`, `epr-atom`, `shard`, `view-fed`, `identity-handshake`,
    /// `trust`).
    pub planes: Vec<String>,
    /// Advisory build identity (`service/version[+commit]`) observed from the
    /// running peer. This additive extension is intentionally NOT covered by
    /// the v1 signature: old readers ignore the map key and must still verify
    /// the signed routing payload. Never infer capabilities or authority from
    /// this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// Wall-clock ms at signing. Receivers keep the newest per NodeId.
    pub announced_at_ms: i64,
    /// ed25519 signature (64 bytes, hex) over [`Self::signing_bytes`].
    pub signature: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestVerifyError {
    #[error("iroh_node_id is not a 64-hex ed25519 public key")]
    BadNodeId,
    #[error("signature is not 64-byte hex")]
    BadSignatureEncoding,
    #[error("signature does not verify against iroh_node_id")]
    SignatureMismatch,
    #[error("announcement carries no reachable address (no direct addrs, no relay)")]
    Unreachable,
    #[error("direct address '{0}' is not ip:port")]
    BadDirectAddr(String),
}

impl TransportManifestAnnouncement {
    /// Build and sign an announcement with the iroh secret key bytes.
    #[allow(clippy::too_many_arguments)]
    pub fn sign(
        secret: &[u8; 32],
        iroh_direct_addrs: Vec<String>,
        iroh_relay_url: Option<String>,
        agent_cid: Option<String>,
        libp2p_peer_id: Option<String>,
        planes: Vec<String>,
        announced_at_ms: i64,
    ) -> Self {
        Self::sign_with_user_agent(
            secret,
            iroh_direct_addrs,
            iroh_relay_url,
            agent_cid,
            libp2p_peer_id,
            planes,
            None,
            announced_at_ms,
        )
    }

    /// Build and sign an announcement carrying an optional advisory build
    /// identity. `user_agent` stays outside the v1 signing bytes by contract.
    #[allow(clippy::too_many_arguments)]
    pub fn sign_with_user_agent(
        secret: &[u8; 32],
        iroh_direct_addrs: Vec<String>,
        iroh_relay_url: Option<String>,
        agent_cid: Option<String>,
        libp2p_peer_id: Option<String>,
        planes: Vec<String>,
        user_agent: Option<String>,
        announced_at_ms: i64,
    ) -> Self {
        let key = SigningKey::from_bytes(secret);
        let mut ann = Self {
            iroh_node_id: hex::encode(key.verifying_key().to_bytes()),
            iroh_direct_addrs,
            iroh_relay_url,
            agent_cid,
            libp2p_peer_id,
            planes,
            user_agent,
            announced_at_ms,
            signature: String::new(),
        };
        let sig = key.sign(&ann.signing_bytes());
        ann.signature = hex::encode(sig.to_bytes());
        ann
    }

    /// The canonical v1 bytes the signature covers. The additive advisory
    /// `user_agent` and `signature` fields are excluded so old readers ignore
    /// the extension without rejecting an otherwise valid announcement.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"elohim/transport/manifest/v1\n");
        out.extend_from_slice(self.iroh_node_id.as_bytes());
        out.push(b'\n');
        for a in &self.iroh_direct_addrs {
            out.extend_from_slice(a.as_bytes());
            out.push(b',');
        }
        out.push(b'\n');
        out.extend_from_slice(self.iroh_relay_url.as_deref().unwrap_or("").as_bytes());
        out.push(b'\n');
        out.extend_from_slice(self.agent_cid.as_deref().unwrap_or("").as_bytes());
        out.push(b'\n');
        out.extend_from_slice(self.libp2p_peer_id.as_deref().unwrap_or("").as_bytes());
        out.push(b'\n');
        for p in &self.planes {
            out.extend_from_slice(p.as_bytes());
            out.push(b',');
        }
        out.push(b'\n');
        out.extend_from_slice(self.announced_at_ms.to_string().as_bytes());
        out
    }

    /// Verify the signature against `iroh_node_id` and check the
    /// announcement is dialable (at least one well-formed direct address or a
    /// relay URL).
    pub fn verify(&self) -> Result<(), ManifestVerifyError> {
        let pk_bytes: [u8; 32] = hex::decode(&self.iroh_node_id)
            .ok()
            .and_then(|v| v.try_into().ok())
            .ok_or(ManifestVerifyError::BadNodeId)?;
        let pk = VerifyingKey::from_bytes(&pk_bytes).map_err(|_| ManifestVerifyError::BadNodeId)?;
        let sig_bytes: [u8; 64] = hex::decode(&self.signature)
            .ok()
            .and_then(|v| v.try_into().ok())
            .ok_or(ManifestVerifyError::BadSignatureEncoding)?;
        let sig = Signature::from_bytes(&sig_bytes);
        pk.verify(&self.signing_bytes(), &sig)
            .map_err(|_| ManifestVerifyError::SignatureMismatch)?;
        for a in &self.iroh_direct_addrs {
            if a.parse::<std::net::SocketAddr>().is_err() {
                return Err(ManifestVerifyError::BadDirectAddr(a.clone()));
            }
        }
        if self.iroh_direct_addrs.is_empty() && self.iroh_relay_url.is_none() {
            return Err(ManifestVerifyError::Unreachable);
        }
        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }

    /// Parsed direct addresses (only call after `verify`).
    pub fn direct_socket_addrs(&self) -> Vec<std::net::SocketAddr> {
        self.iroh_direct_addrs
            .iter()
            .filter_map(|a| a.parse().ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct LegacyTransportManifestAnnouncement {
        iroh_node_id: String,
        iroh_direct_addrs: Vec<String>,
        iroh_relay_url: Option<String>,
        agent_cid: Option<String>,
        libp2p_peer_id: Option<String>,
        planes: Vec<String>,
        announced_at_ms: i64,
        signature: String,
    }

    impl LegacyTransportManifestAnnouncement {
        fn from_current(ann: &TransportManifestAnnouncement) -> Self {
            Self {
                iroh_node_id: ann.iroh_node_id.clone(),
                iroh_direct_addrs: ann.iroh_direct_addrs.clone(),
                iroh_relay_url: ann.iroh_relay_url.clone(),
                agent_cid: ann.agent_cid.clone(),
                libp2p_peer_id: ann.libp2p_peer_id.clone(),
                planes: ann.planes.clone(),
                announced_at_ms: ann.announced_at_ms,
                signature: ann.signature.clone(),
            }
        }

        fn signing_bytes(&self) -> Vec<u8> {
            let mut out = Vec::new();
            out.extend_from_slice(b"elohim/transport/manifest/v1\n");
            out.extend_from_slice(self.iroh_node_id.as_bytes());
            out.push(b'\n');
            for addr in &self.iroh_direct_addrs {
                out.extend_from_slice(addr.as_bytes());
                out.push(b',');
            }
            out.push(b'\n');
            out.extend_from_slice(self.iroh_relay_url.as_deref().unwrap_or("").as_bytes());
            out.push(b'\n');
            out.extend_from_slice(self.agent_cid.as_deref().unwrap_or("").as_bytes());
            out.push(b'\n');
            out.extend_from_slice(self.libp2p_peer_id.as_deref().unwrap_or("").as_bytes());
            out.push(b'\n');
            for plane in &self.planes {
                out.extend_from_slice(plane.as_bytes());
                out.push(b',');
            }
            out.push(b'\n');
            out.extend_from_slice(self.announced_at_ms.to_string().as_bytes());
            out
        }

        fn verify_signature(&self) -> bool {
            let Ok(pk_bytes) = hex::decode(&self.iroh_node_id) else {
                return false;
            };
            let Ok(pk_bytes) = <[u8; 32]>::try_from(pk_bytes) else {
                return false;
            };
            let Ok(pk) = VerifyingKey::from_bytes(&pk_bytes) else {
                return false;
            };
            let Ok(sig_bytes) = hex::decode(&self.signature) else {
                return false;
            };
            let Ok(sig_bytes) = <[u8; 64]>::try_from(sig_bytes) else {
                return false;
            };
            pk.verify(&self.signing_bytes(), &Signature::from_bytes(&sig_bytes))
                .is_ok()
        }
    }

    fn secret() -> [u8; 32] {
        let mut s = [7u8; 32];
        s[0] = 1;
        s
    }

    fn sample() -> TransportManifestAnnouncement {
        TransportManifestAnnouncement::sign(
            &secret(),
            vec!["127.0.0.1:10701".into()],
            None,
            Some("agent-matthew".into()),
            Some("12D3KooWexample".into()),
            vec!["sync".into(), "blob".into()],
            1_700_000_000_000,
        )
    }

    #[test]
    fn signed_announcement_round_trips_and_verifies() {
        let ann = sample();
        let bytes = ann.to_bytes().unwrap();
        let back = TransportManifestAnnouncement::from_bytes(&bytes).unwrap();
        assert_eq!(ann, back);
        assert_eq!(back.verify(), Ok(()));
        assert_eq!(back.iroh_node_id.len(), 64);
        assert_eq!(back.user_agent, None);
    }

    #[test]
    fn old_message_decodes_on_new_reader_with_byte_identical_v1_shape() {
        let ann = sample();
        let legacy = LegacyTransportManifestAnnouncement::from_current(&ann);
        let legacy_bytes = rmp_serde::to_vec_named(&legacy).unwrap();
        let decoded = TransportManifestAnnouncement::from_bytes(&legacy_bytes).unwrap();

        assert_eq!(decoded.user_agent, None);
        assert_eq!(decoded.verify(), Ok(()));
        assert_eq!(decoded.to_bytes().unwrap(), legacy_bytes);
    }

    #[test]
    fn populated_user_agent_round_trips_and_old_reader_still_verifies() {
        let ann = TransportManifestAnnouncement::sign_with_user_agent(
            &secret(),
            vec!["127.0.0.1:10701".into()],
            None,
            Some("agent-matthew".into()),
            Some("12D3KooWexample".into()),
            vec!["sync".into(), "blob".into()],
            Some("elohim-storage/1.2.3+abc1234".into()),
            1_700_000_000_000,
        );
        let bytes = ann.to_bytes().unwrap();
        let current = TransportManifestAnnouncement::from_bytes(&bytes).unwrap();
        let legacy: LegacyTransportManifestAnnouncement = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(
            current.user_agent.as_deref(),
            Some("elohim-storage/1.2.3+abc1234")
        );
        assert_eq!(current.verify(), Ok(()));
        assert!(legacy.verify_signature());
        assert_eq!(
            current.signing_bytes(),
            LegacyTransportManifestAnnouncement::from_current(&current).signing_bytes()
        );
    }

    #[test]
    fn v1_signing_bytes_are_pinned_and_ignore_advisory_user_agent() {
        let plain = sample();
        let mut populated = plain.clone();
        populated.user_agent = Some("elohim-storage/9.9.9+different".into());
        let expected = format!(
            "elohim/transport/manifest/v1\n{}\n127.0.0.1:10701,\n\nagent-matthew\n12D3KooWexample\nsync,blob,\n1700000000000",
            plain.iroh_node_id
        );

        assert_eq!(plain.signing_bytes(), expected.as_bytes());
        assert_eq!(populated.signing_bytes(), expected.as_bytes());
        assert_eq!(populated.verify(), Ok(()));
    }

    #[test]
    fn tampered_address_fails_verification() {
        let mut ann = sample();
        ann.iroh_direct_addrs = vec!["10.0.0.9:4444".into()];
        assert_eq!(ann.verify(), Err(ManifestVerifyError::SignatureMismatch));
    }

    #[test]
    fn foreign_node_id_cannot_be_claimed() {
        // Signed by key A, but claiming node id B.
        let mut ann = sample();
        let other = SigningKey::from_bytes(&[9u8; 32]);
        ann.iroh_node_id = hex::encode(other.verifying_key().to_bytes());
        assert_eq!(ann.verify(), Err(ManifestVerifyError::SignatureMismatch));
    }

    #[test]
    fn unreachable_announcement_is_rejected() {
        let ann =
            TransportManifestAnnouncement::sign(&secret(), vec![], None, None, None, vec![], 1);
        assert_eq!(ann.verify(), Err(ManifestVerifyError::Unreachable));
    }

    #[test]
    fn malformed_direct_addr_is_rejected_even_when_signed() {
        let ann = TransportManifestAnnouncement::sign(
            &secret(),
            vec!["not-an-addr".into()],
            None,
            None,
            None,
            vec![],
            1,
        );
        assert_eq!(
            ann.verify(),
            Err(ManifestVerifyError::BadDirectAddr("not-an-addr".into()))
        );
    }

    #[test]
    fn topic_is_dual_classified_by_prefix() {
        assert!(TRANSPORT_MANIFEST_TOPIC.starts_with("elohim/"));
    }
}
