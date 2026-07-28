//! P2P client for [`ViewKind::ContentHeadRecord`] — the transport half of
//! declare-carries-Record, and the twin of `GET /db/content/{id}/head-record`.
//!
//! ## Why a peer has to serve this at all
//!
//! On a full-arc fleet every `get` is local-only: the retrieval cascade
//! short-circuits at authority, so a conductor that has not gossiped in a
//! foreign root's action cannot fetch it — absence is TERMINAL, not slow. A peer
//! adopting another peer's canonical head therefore cannot ask its own conductor
//! for the target `Record`; it has to carry the bytes from the peer that holds
//! them and let its own conductor re-verify them in wasm.
//!
//! ## Explicit degradation on an old peer
//!
//! `ViewKind` is an externally-tagged enum with no `#[serde(other)]` escape, so
//! a pre-cure peer FAILS to decode this request and the transport surfaces a
//! codec/inbound error. That is expected during a rolling deploy and is treated
//! as a first-class answer: log it once at INFO naming the peer, return `None`,
//! and let the caller fall through to the author path. Never retry-loop (the
//! sweep IS the retry, at its own cadence), never panic.

use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};

use super::P2PHandle;
use crate::services::head_adoption::{CarriedHeadRecord, HeadRecordFetcher};
use crate::views::{ContentHeadRecordPayload, ViewFederationRequest, ViewKind};

/// One-shot ask; the reconcile sweep is the retry loop.
const HEAD_RECORD_TIMEOUT: Duration = Duration::from_secs(10);

/// Fetches a peer's head `Record` over the libp2p view-federation plane.
pub struct PeerHeadRecordFetcher {
    p2p: P2PHandle,
}

impl PeerHeadRecordFetcher {
    pub fn new(p2p: P2PHandle) -> Self {
        Self { p2p }
    }
}

#[async_trait::async_trait]
impl HeadRecordFetcher for PeerHeadRecordFetcher {
    async fn fetch(&self, peer_id: &str, content_id: &str) -> Option<CarriedHeadRecord> {
        let peer = match peer_id.parse::<libp2p::PeerId>() {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!(
                    peer = %peer_id, error = %e,
                    "head-record fetch: unparseable peer id; skipping"
                );
                return None;
            }
        };

        let request = ViewFederationRequest {
            view_kind: ViewKind::ContentHeadRecord {
                content_id: content_id.to_string(),
            },
            agent_cid: self.p2p.agent_pubkey().to_string(),
            request_id: uuid::Uuid::new_v4().to_string(),
            inventory_offset: None,
        };

        let resp = match self
            .p2p
            .view_federate(peer, request, HEAD_RECORD_TIMEOUT)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                // Covers BOTH "peer is offline/slow" and "peer is too old to
                // decode this ViewKind". They are indistinguishable at this
                // layer and have the same correct response: degrade to the
                // author path and try again next sweep.
                tracing::info!(
                    target: "elohim_storage::head_adoption",
                    peer = %peer_id,
                    content_id = %content_id,
                    error = %e,
                    "head-record fetch failed (peer offline, or pre-cure and unable to \
                     decode ContentHeadRecord) — falling back to the author path"
                );
                return None;
            }
        };

        let payload: ContentHeadRecordPayload =
            match serde_json::from_value(resp.slice.payload.0.clone()) {
                Ok(p) => p,
                Err(e) => {
                    tracing::info!(
                        target: "elohim_storage::head_adoption",
                        peer = %peer_id,
                        content_id = %content_id,
                        error = %e,
                        "head-record payload undecodable — falling back to the author path"
                    );
                    return None;
                }
            };

        // Honest absence: the peer answered, but has no head (or cannot retrieve
        // its record). Not an error — just nothing to carry.
        let head_action_hash = payload.head_action_hash?;
        let record = match payload
            .record
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(b64) => match STANDARD.decode(b64) {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    // A malformed record is worse than none: declaring with it
                    // would be refused. Drop the bytes, keep the hash — the
                    // declare may still succeed if our own conductor can
                    // retrieve the target.
                    tracing::warn!(
                        target: "elohim_storage::head_adoption",
                        peer = %peer_id,
                        content_id = %content_id,
                        error = %e,
                        "head-record served malformed base64 — declaring without a carried record"
                    );
                    None
                }
            },
            None => None,
        };

        Some(CarriedHeadRecord {
            head_action_hash,
            record,
        })
    }
}
