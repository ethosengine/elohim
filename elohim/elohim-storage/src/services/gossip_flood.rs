//! Gossip-flood service — Phase 3.5 Trust-Compute Gradient (Primitive 3).
//!
//! When a [`FeedbackSignal`] is published, it is also broadcast on the
//! *content's* reach gossipsub topic so that all current holders of that
//! content see the correction. This is Primitive 3: gossip-flood.
//!
//! ## Layering
//!
//! Primitive 3 is layered ON TOP OF Primitive 2 (T12 back_prop):
//!
//! - **back_prop** (T12) — walks the sealed predecessor chain hop-by-hop,
//!   propagating the signal upstream to the agents who forwarded the content.
//! - **gossip_flood** (T13) — broadcasts the signal to ALL current holders
//!   via a gossipsub topic, so peers who received the content by other means
//!   (e.g. direct fetch, bloom sync) also see the correction.
//!
//! Both primitives are complementary: back_prop sends to known forwarders;
//! gossip_flood reaches the full current-holder set.
//!
//! ## GossipPublisher trait
//!
//! The sync trait [`GossipPublisher`] seams this service from the async libp2p
//! swarm. Production wiring (an async Gossipsub handle) is deferred to T19.
//! For now the trait is defined here and tested via an in-memory mock.
//!
//! // TODO(T19): wire production [`LibP2PGossipPublisher`] by injecting a
//! // channel sender into the swarm event loop from `p2p/behaviour.rs` and
//! // registering handler in `p2p/mod.rs`. The stub placeholder is defined
//! // below but contains no live swarm logic.
//!
//! ## Receiver-side dedup
//!
//! On the receive path, callers pass the signal's CID (computed at the publish
//! site) and the shared [`DedupLru`] instance. The dedup is keyed on the
//! signal's own CID — not `target_cid` — so distinct signals for the same
//! content are each processed once.
//!
//! ## Wire format
//!
//! MessagePack via `rmp_serde`, matching the [`FeedbackSignal`] wire contract
//! in `p2p/feedback_signal.rs` and the `back_prop` encoding convention (T12).
//!
//! See also: `services/back_prop.rs` (T12), `p2p/dedup.rs`, `p2p/feedback_signal.rs`.

use crate::p2p::dedup::DedupLru;
use crate::p2p::feedback_signal::FeedbackSignal;

// ---------------------------------------------------------------------------
// GossipPublisher trait — sync seam for the async swarm
// ---------------------------------------------------------------------------

/// Error from [`GossipPublisher::publish`].
#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    /// The swarm is not currently subscribed to the given topic.
    #[error("not subscribed to topic: {0}")]
    NotSubscribed(String),

    /// An underlying transport or codec error.
    #[error("publish backend error: {0}")]
    Backend(String),
}

/// Sync abstraction over gossipsub publish.
///
/// Seams the domain service from the async libp2p Gossipsub handle.
/// Tests inject an in-memory [`MockGossipPublisher`]. Production wires a
/// thin adapter (`LibP2PGossipPublisher`) from T19.
///
/// Implementations must be `Send + Sync` so the service can be held behind
/// `Arc<dyn GossipPublisher>` in a multi-threaded Axum runtime.
pub trait GossipPublisher: Send + Sync {
    /// Publish `payload` bytes to `topic`.
    ///
    /// Returns `Ok(())` if the message was accepted by the transport.
    /// The call is best-effort at the transport level; the caller is
    /// responsible for deciding whether a `Publish` error is fatal.
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError>;
}

// ---------------------------------------------------------------------------
// Flood error
// ---------------------------------------------------------------------------

/// Errors from [`flood_feedback`].
#[derive(Debug, thiserror::Error)]
pub enum GossipFloodError {
    /// MessagePack encoding of the [`FeedbackSignal`] failed.
    #[error("gossip flood encode error: {0}")]
    Encode(String),

    /// The underlying [`GossipPublisher`] rejected the publish.
    #[error("gossip flood publish error: {0}")]
    Publish(#[from] PublishError),
}

// ---------------------------------------------------------------------------
// ReceiveDecision
// ---------------------------------------------------------------------------

/// Decision returned by [`handle_received_signal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiveDecision {
    /// The CID is new to this node; the caller should process the signal.
    Process,
    /// The CID has already been seen; the caller should drop the message.
    DropAsDuplicate,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Encode and publish a [`FeedbackSignal`] to a gossipsub topic so that all
/// current holders of the content can see the correction.
///
/// # Arguments
///
/// * `signal` — the signal to flood.
/// * `signal_cid` — the CIDv1 of this signal (computed by the caller at the
///   EPR put site; see module doc). Used only for caller logging/tracing;
///   not embedded in the published payload.
/// * `content_reach_topic` — the gossipsub topic for the content's reach set.
///   Conventionally `/elohim/reach/{target_cid}/1.0.0`.
/// * `publisher` — sync publish abstraction; test mock or T19 production adapter.
///
/// # Wire format
///
/// The payload is `rmp_serde::to_vec(signal)` — MessagePack with camelCase
/// field names, identical to the back_prop encoding (T12).
pub fn flood_feedback(
    signal: &FeedbackSignal,
    signal_cid: &str,
    content_reach_topic: &str,
    publisher: &dyn GossipPublisher,
) -> Result<(), GossipFloodError> {
    tracing::trace!(
        signal_cid = signal_cid,
        topic = content_reach_topic,
        "gossip_flood: publishing FeedbackSignal"
    );
    let payload =
        rmp_serde::to_vec_named(signal).map_err(|e| GossipFloodError::Encode(e.to_string()))?;
    publisher
        .publish(content_reach_topic, payload)
        .map_err(GossipFloodError::Publish)
}

/// Decide whether a received signal should be processed or dropped as a
/// duplicate.
///
/// # Arguments
///
/// * `signal_cid` — the CIDv1 of the arriving signal (content-addressed;
///   computed by the receive path, e.g. `blake3(rmp_serde::to_vec(signal))`).
/// * `dedup` — the shared [`DedupLru`] instance for this peer.
///
/// # Dedup semantics
///
/// The LRU is bounded (default 1024 entries). Eviction is LRU-order; once
/// the oldest entry is evicted, the same CID will be treated as `Process`
/// again. This is best-effort dedup — the bound is a compute/memory trade-off,
/// not an authoritative history. See [`DedupLru`] docs for capacity discussion.
pub fn handle_received_signal(signal_cid: &str, dedup: &DedupLru) -> ReceiveDecision {
    if dedup.insert(signal_cid) {
        ReceiveDecision::Process
    } else {
        ReceiveDecision::DropAsDuplicate
    }
}

// ---------------------------------------------------------------------------
// LibP2PGossipPublisher placeholder (T19)
// ---------------------------------------------------------------------------

// TODO(T19): Implement `LibP2PGossipPublisher` by injecting a `tokio::sync::mpsc`
// sender into the libp2p swarm event loop. The swarm task receives a
// `GossipCommand::Publish { topic, payload }` message and calls
// `Gossipsub::publish(IdentTopic::new(topic), payload)`. Register the handler
// in `p2p/behaviour.rs` and expose the sender via `p2p/mod.rs`.
//
// Stub is intentionally omitted here to avoid modifying `p2p/behaviour.rs` or
// `p2p/mod.rs` before T19.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::dedup::DedupLru;
    use crate::p2p::feedback_signal::{FeedbackSignal, SignalKind, StandingImpact};
    use std::sync::Mutex;

    // -----------------------------------------------------------------------
    // In-memory mock publisher
    // -----------------------------------------------------------------------

    struct MockGossipPublisher {
        calls: Mutex<Vec<(String, Vec<u8>)>>,
    }

    impl MockGossipPublisher {
        fn new() -> Self {
            Self {
                calls: Mutex::new(vec![]),
            }
        }

        fn recorded_calls(&self) -> Vec<(String, Vec<u8>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl GossipPublisher for MockGossipPublisher {
        fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
            self.calls
                .lock()
                .unwrap()
                .push((topic.to_string(), payload));
            Ok(())
        }
    }

    /// Publisher that always returns `NotSubscribed`.
    struct FailingGossipPublisher;

    impl GossipPublisher for FailingGossipPublisher {
        fn publish(&self, topic: &str, _payload: Vec<u8>) -> Result<(), PublishError> {
            Err(PublishError::NotSubscribed(topic.to_string()))
        }
    }

    // -----------------------------------------------------------------------
    // Helper: minimal FeedbackSignal
    // -----------------------------------------------------------------------

    fn make_signal(target_cid: &str) -> FeedbackSignal {
        FeedbackSignal {
            target_cid: target_cid.to_string(),
            signal_kind: SignalKind::Correction,
            vouch_kind: None,
            evidence_cid: Some("bafyreievidence0001".to_string()),
            standing_impact: StandingImpact::DebitSoft,
            signed_by: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
            signature: "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: flood_publishes_msgpack_payload_to_topic
    //
    // Encodes, publishes, then decodes the captured payload and asserts equality.
    // -----------------------------------------------------------------------

    #[test]
    fn flood_publishes_msgpack_payload_to_topic() {
        let signal = make_signal("bafyreiabcdef_flood1");
        let publisher = MockGossipPublisher::new();
        let topic = "/elohim/reach/bafyreiabcdef_flood1/1.0.0";

        flood_feedback(&signal, "cid-signal-001", topic, &publisher)
            .expect("flood_feedback should succeed");

        let calls = publisher.recorded_calls();
        assert_eq!(calls.len(), 1, "exactly one publish call expected");

        let (_, payload_bytes) = &calls[0];
        let decoded: FeedbackSignal =
            rmp_serde::from_slice(payload_bytes).expect("payload should decode to FeedbackSignal");
        assert_eq!(decoded, signal, "decoded signal must equal original");
    }

    // -----------------------------------------------------------------------
    // Test 2: flood_publishes_to_specified_topic
    //
    // Confirms the exact topic string passed to flood_feedback is forwarded
    // to the publisher unchanged.
    // -----------------------------------------------------------------------

    #[test]
    fn flood_publishes_to_specified_topic() {
        let signal = make_signal("bafyreiabcdef_flood2");
        let publisher = MockGossipPublisher::new();
        let expected_topic = "/elohim/reach/bafyreiabcdef_flood2/1.0.0";

        flood_feedback(&signal, "cid-signal-002", expected_topic, &publisher)
            .expect("flood_feedback should succeed");

        let calls = publisher.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0, expected_topic,
            "topic forwarded to publisher must match caller's input"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: flood_propagates_publish_error
    //
    // Publisher returns NotSubscribed; flood_feedback surfaces it as
    // GossipFloodError::Publish(PublishError::NotSubscribed(...)).
    // -----------------------------------------------------------------------

    #[test]
    fn flood_propagates_publish_error() {
        let signal = make_signal("bafyreiabcdef_flood3");
        let publisher = FailingGossipPublisher;
        let topic = "/elohim/reach/bafyreiabcdef_flood3/1.0.0";

        let err = flood_feedback(&signal, "cid-signal-003", topic, &publisher)
            .expect_err("flood_feedback should propagate the publisher error");

        match err {
            GossipFloodError::Publish(PublishError::NotSubscribed(t)) => {
                assert_eq!(t, topic, "error should carry the topic that failed");
            }
            other => panic!("expected GossipFloodError::Publish(NotSubscribed), got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Test 4: dedup_first_arrival_processes
    //
    // A fresh CID on an empty DedupLru returns Process.
    // -----------------------------------------------------------------------

    #[test]
    fn dedup_first_arrival_processes() {
        let dedup = DedupLru::new();
        let decision = handle_received_signal("cid-fresh-001", &dedup);
        assert_eq!(
            decision,
            ReceiveDecision::Process,
            "first arrival must return Process"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: dedup_repeat_drops
    //
    // Same CID twice — second call returns DropAsDuplicate.
    // -----------------------------------------------------------------------

    #[test]
    fn dedup_repeat_drops() {
        let dedup = DedupLru::new();
        let first = handle_received_signal("cid-repeat-001", &dedup);
        let second = handle_received_signal("cid-repeat-001", &dedup);

        assert_eq!(
            first,
            ReceiveDecision::Process,
            "first arrival must Process"
        );
        assert_eq!(
            second,
            ReceiveDecision::DropAsDuplicate,
            "repeat must DropAsDuplicate"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: dedup_distinct_cids_both_process
    //
    // Two different CIDs each return Process on first arrival.
    // -----------------------------------------------------------------------

    #[test]
    fn dedup_distinct_cids_both_process() {
        let dedup = DedupLru::new();
        let a = handle_received_signal("cid-distinct-A", &dedup);
        let b = handle_received_signal("cid-distinct-B", &dedup);

        assert_eq!(a, ReceiveDecision::Process, "CID A must Process");
        assert_eq!(b, ReceiveDecision::Process, "CID B must Process");
    }

    // -----------------------------------------------------------------------
    // Test 7: dedup_lru_eviction_allows_reprocess
    //
    // Fill a capacity-2 LRU past its limit, confirm the evicted CID is treated
    // as new again. Documents that the dedup bound is best-effort.
    // -----------------------------------------------------------------------

    #[test]
    fn dedup_lru_eviction_allows_reprocess() {
        // Capacity = 2: after inserting a third entry the LRU entry (cid-evict-A)
        // is evicted and should return Process on the next call.
        let dedup = DedupLru::with_capacity(2);

        // Fill to capacity: [A(LRU), B(MRU)]
        assert_eq!(
            handle_received_signal("cid-evict-A", &dedup),
            ReceiveDecision::Process
        );
        assert_eq!(
            handle_received_signal("cid-evict-B", &dedup),
            ReceiveDecision::Process
        );

        // Insert C: evicts A → [B(LRU), C(MRU)]
        assert_eq!(
            handle_received_signal("cid-evict-C", &dedup),
            ReceiveDecision::Process
        );

        // A was evicted: must Process again (best-effort dedup, not authoritative)
        assert_eq!(
            handle_received_signal("cid-evict-A", &dedup),
            ReceiveDecision::Process,
            "evicted CID must reprocess — dedup bound is best-effort, not authoritative"
        );

        // After insert(C): [B(LRU), C(MRU)]
        // insert(A) above: capacity full → evicts B (LRU) → [C(LRU), A(MRU)]
        // B was evicted by A's re-insertion; must Process again
        assert_eq!(
            handle_received_signal("cid-evict-B", &dedup),
            ReceiveDecision::Process,
            "B was evicted by A's re-insertion; must Process again"
        );
    }
}
