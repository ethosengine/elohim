//! ## Source of Truth
//!
//! This module is **Operational (Category C)** per the p2p-design-gate output in
//! `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`.
//! It composes a read-projection from notarized DHT state. The DHT remains canonical.
//! No SQLite table here is authoritative.
//!
//! Federation aggregator for `/elohim/view-federation/1.0.0`. Fans out a single
//! (view_kind, agent_cid) query to all [`PeerIdentityBindingRow`] entries for that
//! agent and collects per-peer [`ViewSlice`] responses with bounded latency.
//!
//! ## Stage 1 structural verification only
//!
//! Per `project_bootstrap_to_elohim_security_gradient`, F-T21 verifies only that
//! the returned signature is non-empty + base64-decodable + plausibly sized.
//! Stage 2 adds ed25519 cryptographic verification once resolver-backed
//! `peer_id → libp2p::identity::PublicKey` lookup exists. See
//! [`verify_slice_structural`].

use std::time::Duration;

use libp2p::PeerId;

use crate::db::models::PeerIdentityBindingRow;
use crate::p2p::{FederationError, P2PHandle};
use crate::views::{Freshness, FreshnessState, ViewFederationRequest, ViewKind, ViewSlice};

/// Bounds on a structurally-valid signature in base64-decoded form.
/// ed25519 produces 64-byte signatures; the 32–128 range covers that and any
/// future Stage 2 algorithm with similar size class.
const MIN_SIG_BYTES: usize = 32;
const MAX_SIG_BYTES: usize = 128;

/// Per-peer result from [`Federator::query`].
///
/// `slice` is `None` on signature rejection or when the peer is unreachable.
/// `freshness.state` discriminates the outcome:
/// - [`FreshnessState::Live`] / [`FreshnessState::Stale`] — slice arrived and
///   passed Stage 1 structural verification.
/// - [`FreshnessState::Offline`] — peer was unreachable (transport error or timeout).
/// - [`FreshnessState::Unverifiable`] — slice arrived but failed structural
///   verification, OR the binding row's `peer_id` was unparseable.
#[derive(Debug, Clone)]
pub struct FederationResult {
    /// The libp2p PeerId string from the binding row.
    pub peer_id: String,
    /// Freshness classification for this peer's contribution.
    pub freshness: Freshness,
    /// The verified slice, or `None` when the peer could not contribute.
    pub slice: Option<ViewSlice>,
}

/// Requester-side aggregator for the `/elohim/view-federation/1.0.0` protocol.
///
/// `Federator::query` fans out a single `(view_kind, agent_cid)` query to every
/// binding row supplied by the caller and collects per-peer results via
/// [`futures::future::join_all`], all gated by `per_peer_timeout`.
pub struct Federator {
    p2p: P2PHandle,
}

impl Federator {
    /// Construct a `Federator` backed by the given `P2PHandle`.
    pub fn new(p2p: P2PHandle) -> Self {
        Self { p2p }
    }

    /// Fan out `(view_kind, agent_cid)` to every binding in `bindings` and
    /// return one [`FederationResult`] per entry.
    ///
    /// Results are returned in the same order as `bindings`. Futures run
    /// concurrently via [`futures::future::join_all`]; the per-peer deadline
    /// is enforced inside [`P2PHandle::view_federate`].
    pub async fn query(
        &self,
        view_kind: ViewKind,
        agent_cid: &str,
        bindings: &[PeerIdentityBindingRow],
        per_peer_timeout: Duration,
    ) -> Vec<FederationResult> {
        let futures = bindings.iter().map(|binding| {
            let peer_id_str = binding.peer_id.clone();
            let agent_cid_owned = agent_cid.to_string();
            let vk = view_kind.clone();
            let p2p = self.p2p.clone();
            let request_id = uuid::Uuid::new_v4().to_string();

            async move {
                // Parse the multibase peer_id string. An unparseable row is a
                // projection corruption; return Unverifiable rather than panic.
                let peer_id = match peer_id_str.parse::<PeerId>() {
                    Ok(p) => p,
                    Err(_) => {
                        return FederationResult {
                            peer_id: peer_id_str,
                            freshness: Freshness {
                                state: FreshnessState::Unverifiable,
                                stale_since_ms: None,
                            },
                            slice: None,
                        }
                    }
                };

                let request = ViewFederationRequest {
                    view_kind: vk,
                    agent_cid: agent_cid_owned,
                    request_id,
                };

                match p2p.view_federate(peer_id, request, per_peer_timeout).await {
                    Ok(response) => {
                        // Stage 1 structural verification — see verify_slice_structural.
                        if !verify_slice_structural(&response.slice) {
                            return FederationResult {
                                peer_id: peer_id_str,
                                freshness: Freshness {
                                    state: FreshnessState::Unverifiable,
                                    stale_since_ms: None,
                                },
                                slice: None,
                            };
                        }
                        // Slice passed — propagate the freshness state the peer reported.
                        FederationResult {
                            peer_id: peer_id_str,
                            freshness: response.slice.freshness.clone(),
                            slice: Some(response.slice),
                        }
                    }
                    // All transport-level failures (Timeout, SwarmGone, TransportError)
                    // are classified as Offline from the aggregator's perspective.
                    Err(FederationError::Timeout)
                    | Err(FederationError::SwarmGone)
                    | Err(FederationError::TransportError) => FederationResult {
                        peer_id: peer_id_str,
                        freshness: Freshness {
                            state: FreshnessState::Offline,
                            stale_since_ms: None,
                        },
                        slice: None,
                    },
                }
            }
        });
        futures::future::join_all(futures).await
    }
}

/// Stage 1 structural verification of a federated [`ViewSlice`]'s signature field.
///
/// Returns `true` if:
/// 1. The signature string is non-empty.
/// 2. It is decodable as standard base64.
/// 3. The decoded byte slice is within [`MIN_SIG_BYTES`]..=[`MAX_SIG_BYTES`].
///
/// This is the structural floor that admits a slice for use in the aggregated
/// result. Cryptographic correctness (ed25519 verify against the peer's public
/// key) is deferred to Stage 2.
///
/// ## Stage 2 extension point
///
/// Replace the body of this function with:
/// ```ignore
/// let canonical = slice.canonical_bytes_for_signing();
/// let sig_bytes = base64::engine::general_purpose::STANDARD
///     .decode(&slice.signature)
///     .ok()?;
/// let public_key = peer_pubkey_resolver.lookup(slice.peer_id)?;
/// public_key.verify(&canonical, &sig_bytes)
/// ```
/// (The `peer_pubkey_resolver` is not yet implemented — see
/// `identity_binding_gossip::verify_structural` for the parallel Stage 2 deferral.)
fn verify_slice_structural(slice: &ViewSlice) -> bool {
    use base64::Engine as _;
    if slice.signature.is_empty() {
        return false;
    }
    base64::engine::general_purpose::STANDARD
        .decode(&slice.signature)
        .is_ok_and(|bytes| (MIN_SIG_BYTES..=MAX_SIG_BYTES).contains(&bytes.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::{Freshness, FreshnessState, JsonVal, ViewKind, ViewSlice};
    use base64::Engine as _;

    fn make_slice(state: FreshnessState, sig_bytes: &[u8]) -> ViewSlice {
        ViewSlice {
            peer_id: "12D3KooWTest".to_string(),
            view_kind: ViewKind::Cluster,
            freshness: Freshness {
                state,
                stale_since_ms: None,
            },
            payload: JsonVal(serde_json::Value::Null),
            signature: base64::engine::general_purpose::STANDARD.encode(sig_bytes),
        }
    }

    #[test]
    fn structural_verify_accepts_64_byte_signature() {
        let slice = make_slice(FreshnessState::Live, &[0u8; 64]);
        assert!(verify_slice_structural(&slice));
    }

    #[test]
    fn structural_verify_rejects_empty_signature() {
        let mut slice = make_slice(FreshnessState::Live, &[0u8; 64]);
        slice.signature = String::new();
        assert!(!verify_slice_structural(&slice));
    }

    #[test]
    fn structural_verify_rejects_undersized_signature() {
        // 16 bytes < MIN_SIG_BYTES (32)
        let slice = make_slice(FreshnessState::Live, &[0u8; 16]);
        assert!(!verify_slice_structural(&slice));
    }

    #[test]
    fn structural_verify_rejects_oversized_signature() {
        // 256 bytes > MAX_SIG_BYTES (128)
        let slice = make_slice(FreshnessState::Live, &[0u8; 256]);
        assert!(!verify_slice_structural(&slice));
    }

    #[test]
    fn structural_verify_rejects_non_base64_signature() {
        let mut slice = make_slice(FreshnessState::Live, &[0u8; 64]);
        slice.signature = "not-valid-base64!!!".to_string();
        assert!(!verify_slice_structural(&slice));
    }
}
