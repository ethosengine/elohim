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
//! as a first-class answer: log it once at INFO naming the peer, return
//! [`Answer::Unreachable`], and let the caller fall through to the author path.
//! Never retry-loop (the sweep IS the retry, at its own cadence), never panic.
//!
//! ## The two absences (C4)
//!
//! Since 2026-08-02 (plan task P2.3) this client answers with
//! [`seam_contracts::Answer`] rather than `Option`, and the mapping is the whole
//! point:
//!
//! - **`Unreachable`** — unparseable peer id, transport error, a peer too old to
//!   decode the request, an undecodable payload. We never learned anything.
//! - **`Absent`** — the peer answered and reports no head for this id
//!   (`head_action_hash: None`). An OBSERVED absence, and the only arm entitled
//!   to claim one.
//! - **`Present`** — a head was served. `record: None` inside it is still a
//!   present answer: a peer that holds a head it cannot resolve answers
//!   hash-only, which is the dominant shape when the advertising peer is itself
//!   conductor-missing.

use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};

use seam_contracts::Answer;

use super::P2PHandle;
use crate::services::head_adoption::{CarriedHeadRecord, HeadRecordFetcher};
use crate::views::{ContentHeadRecordPayload, ViewFederationRequest, ViewKind};

/// One-shot ask; the reconcile sweep is the retry loop.
///
/// Public so the responder's own budget
/// ([`crate::p2p::view_federation::HEAD_RECORD_CONDUCTOR_TIMEOUT`]) can be
/// asserted strictly below it — the responder must always be the side that gives
/// up first, or a slow conductor turns into a transport timeout the requester
/// cannot tell apart from an offline peer.
pub const HEAD_RECORD_TIMEOUT: Duration = Duration::from_secs(10);

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
    async fn fetch(&self, peer_id: &str, content_id: &str) -> Answer<CarriedHeadRecord> {
        let peer = match peer_id.parse::<libp2p::PeerId>() {
            Ok(p) => p,
            Err(e) => {
                // Unreachable, not Absent: we never asked anyone anything.
                tracing::warn!(
                    peer = %peer_id, error = %e,
                    "head-record fetch: unparseable peer id; skipping"
                );
                let answer = Answer::Unreachable;
                crate::metrics::inc_head_record_fetch(answer.state());
                return answer;
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
                let answer = Answer::Unreachable;
                crate::metrics::inc_head_record_fetch(answer.state());
                return answer;
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
                    let answer = Answer::Unreachable;
                    crate::metrics::inc_head_record_fetch(answer.state());
                    return answer;
                }
            };

        let answer = classify_payload(payload, peer_id, content_id);
        crate::metrics::inc_head_record_fetch(answer.state());
        answer
    }
}

/// Turn a decoded `ContentHeadRecordPayload` into the answer it actually is.
///
/// **Concerns:** C4 (honest absence — this is the ONE place this client may
/// claim an observed absence), C8 (the malformed-base64 arm degrades loudly
/// rather than silently).
///
/// **Contract test:** [`tests::fetch_failures_are_unreachable_not_absent`] and
/// its siblings.
///
/// Pure and separate from [`PeerHeadRecordFetcher::fetch`] on purpose: the
/// transport half needs a live swarm to exercise, the classification half is the
/// part that can get C4 wrong, and only the second one is worth a test. The
/// `peer_id` / `content_id` arguments are for the degradation log line only —
/// nothing about the classification depends on them.
fn classify_payload(
    payload: ContentHeadRecordPayload,
    peer_id: &str,
    content_id: &str,
) -> Answer<CarriedHeadRecord> {
    // HONEST ABSENCE, the one place this client may claim one: the peer
    // ANSWERED and reports no head for this id. `Answer::observed_absence`
    // names that provenance at the call site (there is deliberately no
    // blanket `From<Option<T>>`, precisely so this choice stays visible).
    let head_action_hash = match Answer::observed_absence(payload.head_action_hash) {
        Answer::Present(h) => h,
        Answer::Absent => return Answer::Absent,
        // `observed_absence` never yields this arm; matched rather than
        // unwrapped so a future refactor cannot turn it into a panic.
        Answer::Unreachable => return Answer::Unreachable,
    };
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

    Answer::Present(CarriedHeadRecord {
        head_action_hash,
        record,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use seam_contracts::AnswerState;

    /// Constructed field-by-field (no `..Default::default()`): a new field on the
    /// wire payload should fail HERE, loudly, rather than default silently — the
    /// C10 contract-evolution rule applied to a fixture.
    fn payload(head: Option<&str>, record: Option<&str>) -> ContentHeadRecordPayload {
        ContentHeadRecordPayload {
            content_id: "c1".to_string(),
            head_action_hash: head.map(str::to_string),
            declared_at: None,
            record: record.map(str::to_string),
        }
    }

    /// The C4 line this client exists to hold: a peer that ANSWERED with no head
    /// is `Absent`; a peer we could not reach is `Unreachable`. The transport
    /// arms return `Unreachable` directly (unparseable peer id, view-federate
    /// error, undecodable payload) and are unreachable from a pure test — what
    /// IS testable, and what actually got re-merged historically, is that the
    /// answered-with-nothing case never widens into the unreachable bucket, nor
    /// the reverse.
    #[test]
    fn fetch_failures_are_unreachable_not_absent() {
        // Peer answered, holds no head → OBSERVED absence.
        let answered_empty = classify_payload(payload(None, None), "peer", "c1");
        assert_eq!(answered_empty.state(), AnswerState::Absent);
        assert_ne!(
            answered_empty.state(),
            AnswerState::Unreachable,
            "an answered 'no head' must not degrade into 'we never heard'"
        );

        // Peer answered with a head and bytes → Present, bytes carried.
        let served = classify_payload(payload(Some("uhCkk-head"), Some("aGk=")), "peer", "c1");
        assert_eq!(served.state(), AnswerState::Present);
        let carried = served.into_option().unwrap();
        assert_eq!(carried.head_action_hash, "uhCkk-head");
        assert_eq!(carried.record.as_deref(), Some(b"hi".as_slice()));
    }

    /// A hash-only answer is PRESENT, not absent — the peer holds a head it
    /// cannot resolve. Counting it as an absence is the `carried_present`
    /// mislabel (`da8975176`), which pointed the dashboard at the wrong remedy.
    #[test]
    fn a_hash_only_answer_is_present_not_absent() {
        let hash_only = classify_payload(payload(Some("uhCkk-head"), None), "peer", "c1");
        assert_eq!(hash_only.state(), AnswerState::Present);
        assert!(hash_only.into_option().unwrap().record.is_none());
    }

    /// Malformed carried bytes drop the RECORD, never the answer: the declare
    /// may still succeed if our own conductor can retrieve the target.
    #[test]
    fn malformed_base64_degrades_to_a_record_less_present_answer() {
        let malformed = classify_payload(payload(Some("uhCkk-head"), Some("!!!not-b64")), "p", "c");
        assert_eq!(malformed.state(), AnswerState::Present);
        assert!(malformed.into_option().unwrap().record.is_none());
    }

    /// Whitespace-only bytes are not bytes.
    #[test]
    fn blank_record_is_no_record() {
        let blank = classify_payload(payload(Some("uhCkk-head"), Some("   ")), "p", "c");
        assert!(blank.into_option().unwrap().record.is_none());
    }
}
