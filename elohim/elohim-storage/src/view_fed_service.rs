//! Phase 11 — transport-neutral view-federation service.
//!
//! Wraps [`crate::p2p::view_federation::build_response_slice`] with
//! the daemon-side dependencies (this peer's identity, connected
//! peers, keypair, DB pool) so both the libp2p inbound handler and
//! the iroh-side `ViewFederationBackend` can build wire-byte-identical
//! responses for the same request.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the view-federation plane is dual-stack permanent. 256 KiB cap on
//! responses applies on both transports.
//!
//! ## Signing crypto
//!
//! The slice signature uses the substrate keypair. The libp2p side
//! has a [`libp2p::identity::Keypair`]; the iroh side has an
//! [`iroh::SecretKey`]. Both wrap the same ed25519 key material. The
//! main.rs iroh branch converts the iroh SecretKey bytes into a
//! libp2p Keypair via `Keypair::ed25519_from_bytes` so the same
//! `build_response_slice` path runs identically. Receivers verify
//! signatures against the agent's DHT-anchored public key (Track 1),
//! which matches both sides since they sign with the same key.
//!
//! ## Connected peers (PeerTopology view kind)
//!
//! The libp2p side has direct access to `swarm.connected_peers()`. The
//! iroh side currently passes an empty slice; iroh-mode PeerTopology
//! returns a local-only view until the cross-stack peer-map's
//! `peer_transport_manifest` projects connection state.

use libp2p::identity::{Keypair, SigningError};
use libp2p::PeerId;

use crate::db::DbPool;
use crate::p2p::view_federation::{build_response_slice, SliceContext};
use crate::views::{ViewFederationRequest, ViewFederationResponse, ViewKind};

/// Holds the dependencies needed to answer a view-federation request.
pub struct ViewFedService {
    local_agent_cid: String,
    local_peer_id: String,
    keypair: Keypair,
    db_pool: Option<DbPool>,
}

impl std::fmt::Debug for ViewFedService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewFedService")
            .field("local_agent_cid", &self.local_agent_cid)
            .field("local_peer_id", &self.local_peer_id)
            .field("has_db_pool", &self.db_pool.is_some())
            .finish_non_exhaustive()
    }
}

impl ViewFedService {
    pub fn new(
        local_agent_cid: String,
        local_peer_id: String,
        keypair: Keypair,
        db_pool: Option<DbPool>,
    ) -> Self {
        Self {
            local_agent_cid,
            local_peer_id,
            keypair,
            db_pool,
        }
    }

    /// Build a signed [`ViewFederationResponse`] for `request`. Pass
    /// the current `connected_peers` snapshot in (libp2p side reads
    /// from `swarm.read().connected_peers()`; iroh side passes an
    /// empty slice for now — see module-level docs).
    pub async fn handle(
        &self,
        request: ViewFederationRequest,
        connected_peers: &[PeerId],
    ) -> Result<ViewFederationResponse, SigningError> {
        let ctx = SliceContext {
            agent_cid: request.agent_cid,
            request_id: request.request_id,
            local_agent_cid: &self.local_agent_cid,
            local_peer_id: self.local_peer_id.clone(),
            connected_peers,
            keypair: &self.keypair,
            pool: self.db_pool.as_ref(),
        };
        build_response_slice(request.view_kind, ctx).await
    }

    /// Map a [`ViewKind`] string in case future callers need the typed
    /// variant — kept here for symmetry with sibling services. (Not
    /// currently used; the typed `ViewKind` arrives directly on
    /// [`ViewFederationRequest`].)
    pub fn parse_view_kind(_s: &str) -> Option<ViewKind> {
        None
    }
}

/// Convert a 32-byte ed25519 secret (from `iroh::SecretKey::to_bytes()`
/// or any other source of raw ed25519 bytes) into a libp2p `Keypair`.
/// Both wrap the same crypto; signatures are byte-identical for the
/// same input.
///
/// Returns the libp2p [`SigningError`] if the conversion fails (which
/// it shouldn't for a valid 32-byte ed25519 secret).
pub fn libp2p_keypair_from_ed25519_bytes(
    secret_bytes: &mut [u8; 32],
) -> Result<Keypair, libp2p::identity::DecodingError> {
    Keypair::ed25519_from_bytes(secret_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::ViewKind;

    fn fresh_service() -> ViewFedService {
        let kp = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(kp.public()).to_string();
        ViewFedService::new("agent-cid-self".into(), local_peer_id, kp, None)
    }

    #[tokio::test]
    async fn handle_owning_agent_with_no_pool_returns_signed_empty_payload() {
        let svc = fresh_service();
        let req = ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent-cid-self".into(),
            request_id: "req-1".into(),
        };
        let res = svc.handle(req, &[]).await.expect("signs");
        assert_eq!(res.agent_cid, "agent-cid-self");
        assert_eq!(res.request_id, "req-1");
        assert!(!res.slice.signature.is_empty(), "signature must populate");
    }

    #[tokio::test]
    async fn handle_non_owning_agent_returns_offline_freshness() {
        use crate::views::FreshnessState;
        let svc = fresh_service();
        let req = ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "some-other-agent".into(),
            request_id: "req-2".into(),
        };
        let res = svc.handle(req, &[]).await.expect("signs");
        assert_eq!(res.slice.freshness.state, FreshnessState::Offline);
    }

    #[test]
    fn libp2p_keypair_from_ed25519_bytes_round_trips() {
        // Generate via libp2p, extract bytes, reconstruct, verify
        // signing produces identical signatures (deterministic
        // ed25519).
        let kp1 = Keypair::generate_ed25519();
        let sig1 = kp1.sign(b"test message").expect("sig1");
        let secret = kp1.try_into_ed25519().expect("ed25519 keypair");
        let mut bytes: [u8; 32] = secret.secret().as_ref().try_into().expect("32 bytes");
        let kp2 = libp2p_keypair_from_ed25519_bytes(&mut bytes).expect("rebuild");
        let sig2 = kp2.sign(b"test message").expect("sig2");
        assert_eq!(sig1, sig2, "ed25519 is deterministic; sigs must match");
    }
}
