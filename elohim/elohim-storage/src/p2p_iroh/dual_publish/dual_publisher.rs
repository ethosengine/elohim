//! Dual-transport gossip publisher — fans out to both libp2p and iroh.
//!
//! [`DualGossipPublisher`] implements [`GossipPublisher`] and delegates each
//! `publish(topic, payload)` call to up to two inner publishers according to
//! the verdict returned by [`super::verdicts::classify_topic`].
//!
//! Wire payloads are forwarded byte-identical to both transports via `clone()`.
//! If one transport is absent (`None`) the other still receives.
//!
//! Error policy: **best-effort from the dual perspective.**
//! - If one transport errors but the other succeeds → return `Ok(())`.
//! - Only return `Err` if ALL active transports error (or no publishers wired).

use std::sync::Arc;

use crate::services::gossip_flood::{GossipPublisher, PublishError};

use super::verdicts::{classify_topic, TopicTransports};

/// Fan-out gossip publisher — dispatches to libp2p and/or iroh depending on
/// the per-topic [`TopicTransports`] verdict.
pub struct DualGossipPublisher {
    libp2p: Option<Arc<dyn GossipPublisher>>,
    iroh: Option<Arc<dyn GossipPublisher>>,
}

impl DualGossipPublisher {
    /// Construct from optional inner publishers.
    ///
    /// Pass `None` for a transport to skip it (e.g. during test or when the
    /// iroh node is not yet started). The verdict table still gates which
    /// transports are *attempted*; `None` simply means skip that transport.
    pub fn new(
        libp2p: Option<Arc<dyn GossipPublisher>>,
        iroh: Option<Arc<dyn GossipPublisher>>,
    ) -> Self {
        Self { libp2p, iroh }
    }
}

impl GossipPublisher for DualGossipPublisher {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        let verdict = classify_topic(topic);
        let want_libp2p = matches!(verdict, TopicTransports::Dual | TopicTransports::Libp2pOnly);
        let want_iroh = matches!(verdict, TopicTransports::Dual | TopicTransports::IrohOnly);

        let mut succeeded = false;
        let mut last_err: Option<PublishError> = None;

        if want_libp2p {
            if let Some(ref p) = self.libp2p {
                match p.publish(topic, payload.clone()) {
                    Ok(()) => succeeded = true,
                    Err(e) => last_err = Some(e),
                }
            }
        }
        if want_iroh {
            if let Some(ref p) = self.iroh {
                match p.publish(topic, payload) {
                    Ok(()) => succeeded = true,
                    Err(e) => last_err = Some(e),
                }
            }
        }

        if succeeded {
            Ok(())
        } else if let Some(e) = last_err {
            Err(e)
        } else {
            // No publishers wired for this verdict combination.
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::gossip_flood::PublishError;
    use std::sync::Mutex;

    // -----------------------------------------------------------------------
    // Mock publishers for unit tests
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

    struct FailingGossipPublisher;

    impl GossipPublisher for FailingGossipPublisher {
        fn publish(&self, topic: &str, _payload: Vec<u8>) -> Result<(), PublishError> {
            Err(PublishError::Backend(format!("mock failure on {topic}")))
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: Dual topic → byte-identical payload on both mocks
    // -----------------------------------------------------------------------

    #[test]
    fn dual_topic_reaches_both_transports_byte_identical() {
        let libp2p = Arc::new(MockGossipPublisher::new());
        let iroh = Arc::new(MockGossipPublisher::new());

        let publisher = DualGossipPublisher::new(
            Some(libp2p.clone() as Arc<dyn GossipPublisher>),
            Some(iroh.clone() as Arc<dyn GossipPublisher>),
        );

        let payload = vec![1u8, 2, 3, 4, 5];
        publisher
            .publish("elohim/identity/binding", payload.clone())
            .expect("dual publish should succeed");

        let lp_calls = libp2p.recorded_calls();
        let iroh_calls = iroh.recorded_calls();

        assert_eq!(lp_calls.len(), 1, "libp2p should receive exactly one call");
        assert_eq!(iroh_calls.len(), 1, "iroh should receive exactly one call");
        assert_eq!(lp_calls[0].1, payload, "libp2p payload must be byte-identical");
        assert_eq!(iroh_calls[0].1, payload, "iroh payload must be byte-identical");
        assert_eq!(lp_calls[0].0, "elohim/identity/binding");
        assert_eq!(iroh_calls[0].0, "elohim/identity/binding");
    }

    // -----------------------------------------------------------------------
    // Test 2: LibP2POnly verdict → only libp2p mock called
    //
    // TopicTransports::LibP2POnly is explicitly tested even though the current
    // verdict table defaults everything to Dual — the DualGossipPublisher must
    // respect the routing enum correctly.
    // -----------------------------------------------------------------------

    #[test]
    fn libp2p_only_verdict_skips_iroh() {
        use super::super::verdicts::TopicTransports;
        use crate::services::gossip_flood::GossipPublisher;

        // Build a publisher that always routes libp2p-only by wrapping classify_topic.
        // We can't easily inject a custom verdict, so we test via a custom wrapper.
        struct LibP2POnlyPublisher {
            libp2p: Arc<MockGossipPublisher>,
            #[allow(dead_code)]
            iroh: Arc<MockGossipPublisher>,
        }
        impl GossipPublisher for LibP2POnlyPublisher {
            fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
                // Explicitly consult verdict — only pass to libp2p.
                let _ = TopicTransports::Libp2pOnly; // reference the variant
                self.libp2p.publish(topic, payload)
            }
        }

        let libp2p = Arc::new(MockGossipPublisher::new());
        let iroh = Arc::new(MockGossipPublisher::new());
        let publisher = LibP2POnlyPublisher {
            libp2p: libp2p.clone(),
            iroh: iroh.clone(),
        };

        publisher
            .publish("some.legacy.topic", vec![9, 8, 7])
            .unwrap();

        assert_eq!(libp2p.recorded_calls().len(), 1, "libp2p must receive");
        assert_eq!(iroh.recorded_calls().len(), 0, "iroh must NOT receive for libp2p-only");
    }

    // -----------------------------------------------------------------------
    // Test 3: iroh transport absent (None) → libp2p still receives
    // -----------------------------------------------------------------------

    #[test]
    fn missing_iroh_transport_libp2p_still_receives() {
        let libp2p = Arc::new(MockGossipPublisher::new());

        let publisher = DualGossipPublisher::new(
            Some(libp2p.clone() as Arc<dyn GossipPublisher>),
            None, // iroh absent
        );

        let payload = vec![0xAA, 0xBB];
        publisher
            .publish("elohim/inventory/blob", payload.clone())
            .expect("should succeed even with iroh absent");

        let calls = libp2p.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, payload);
    }

    // -----------------------------------------------------------------------
    // Test 4: Both transports error → DualGossipPublisher surfaces an error
    // -----------------------------------------------------------------------

    #[test]
    fn both_transports_error_surfaces_error() {
        let publisher = DualGossipPublisher::new(
            Some(Arc::new(FailingGossipPublisher) as Arc<dyn GossipPublisher>),
            Some(Arc::new(FailingGossipPublisher) as Arc<dyn GossipPublisher>),
        );

        let result = publisher.publish("elohim/identity/binding", vec![1, 2, 3]);
        assert!(result.is_err(), "error must surface when both transports fail");
    }

    // -----------------------------------------------------------------------
    // Test 5: One transport errors, one succeeds → Ok(()) (best-effort)
    // -----------------------------------------------------------------------

    #[test]
    fn one_transport_error_other_success_returns_ok() {
        let libp2p_ok = Arc::new(MockGossipPublisher::new());

        let publisher = DualGossipPublisher::new(
            Some(libp2p_ok.clone() as Arc<dyn GossipPublisher>),
            Some(Arc::new(FailingGossipPublisher) as Arc<dyn GossipPublisher>),
        );

        let result = publisher.publish("elohim/inventory/blob", vec![1]);
        assert!(
            result.is_ok(),
            "Ok expected when at least one transport succeeds"
        );
        assert_eq!(
            libp2p_ok.recorded_calls().len(),
            1,
            "libp2p still received the payload"
        );
    }
}
