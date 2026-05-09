//! Phase 11 — transport-neutral trust handshake service.
//!
//! Today the libp2p inline trust handler builds a stub
//! [`crate::trust_verification::VerifiedTrustContext`] (full conductor
//! integration is a Stage 3 / future-sprint concern) and stores it
//! against the libp2p `PeerId` in
//! [`crate::p2p::trust_cache::PeerTrustCache`]. This service exposes
//! the response-building half of that flow so both transports return
//! identical wire bytes for the same input.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the trust plane is dual-stack permanent. Trust attestations are
//! DHT-notarized; the wire just carries them. Both transports
//! preserve DHT-derived integrity equally.
//!
//! ## Trust cache (libp2p-keyed today)
//!
//! [`PeerTrustCache`](crate::p2p::trust_cache::PeerTrustCache) keys on
//! libp2p `PeerId`. The libp2p path inserts after responding; the iroh
//! path skips cache insertion in iroh mode (mode-exclusive at runtime,
//! cross-stack peer-map graduates the cache to be transport-agnostic
//! in Phase 12). Iroh-mode reach-auth fast path therefore always
//! falls through to the slow-path DB lookup — correct, just not
//! ambient-cached. This is documented behavior, not a regression.

use crate::p2p::trust_protocol::{TrustHandshake, TrustResponse};

/// Minimal transport-neutral trust handshake service. Today this is
/// just a response-builder around the stub [`VerifiedTrustContext`]
/// shape the libp2p path constructs; when the conductor integration
/// fills in real attestation verification, that work moves here so
/// both transports share it.
#[derive(Clone, Default)]
pub struct TrustService;

impl std::fmt::Debug for TrustService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrustService").finish()
    }
}

impl TrustService {
    pub fn new() -> Self {
        Self
    }

    /// Build a [`TrustResponse`] for an inbound [`TrustHandshake`].
    /// Mirrors the libp2p inline handler's response-build half
    /// exactly so wire bytes match for the same input.
    pub fn handle(&self, _request: TrustHandshake) -> TrustResponse {
        TrustResponse::Verified {
            reach_ceiling: "public".to_string(),
            ttl_seconds: 3600,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> TrustHandshake {
        TrustHandshake {
            agent_pubkey: "agent-stub".to_string(),
            membership_cids: vec![],
            relationship_cids: vec![],
            attestation_cids: vec![],
            stewardship_cids: vec![],
        }
    }

    #[test]
    fn handle_returns_public_ceiling_with_one_hour_ttl() {
        let svc = TrustService::new();
        match svc.handle(sample_request()) {
            TrustResponse::Verified {
                reach_ceiling,
                ttl_seconds,
            } => {
                assert_eq!(reach_ceiling, "public");
                assert_eq!(ttl_seconds, 3600);
            }
            other => panic!("expected Verified, got {other:?}"),
        }
    }
}
