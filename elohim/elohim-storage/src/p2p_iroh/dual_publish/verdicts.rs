//! Per-topic transport routing table for dual-publish fan-out.
//!
//! The gossip plane is permanent dual-stack per the complementarity spec
//! line 278. Each topic classifies into a [`TopicTransports`] verdict that
//! [`super::dual_publisher::DualGossipPublisher`] uses to decide which
//! transports receive the publish call.
//!
//! Verdicts are purely compile-time routing decisions — no persistence,
//! no DHT entries, no wire frames. Category C (operational, in-process only).

/// Which transports a given topic should publish on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopicTransports {
    /// Both transports MUST publish (permanent dual-stack per spec line 278).
    Dual,
    /// Only libp2p (legacy / consumer-grade only — none currently classified as this).
    Libp2pOnly,
    /// Only iroh (would require all subscribers to be iroh-capable; none currently).
    IrohOnly,
}

/// Returns [`TopicTransports`] for a given topic name.
///
/// Reach-scoped EPR-announce topics (prefix `elohim/<pillar>/`) all classify
/// as `Dual`. The default for any unrecognised topic is conservatively `Dual`
/// so new topics are covered without an explicit table update.
pub fn classify_topic(topic_name: &str) -> TopicTransports {
    match topic_name {
        // Integrity + identity — permanent dual per spec line 278.
        "elohim/inventory/blob" => TopicTransports::Dual,
        "elohim/identity/binding" => TopicTransports::Dual,
        "recovery.invitation" => TopicTransports::Dual,
        "recovery.revocation" => TopicTransports::Dual,
        "elohim/integrity/revocation" => TopicTransports::Dual,
        // Feedback-signal topics (reach-scoped, transition dual).
        t if t.starts_with("/elohim/feedback-signal/") => TopicTransports::Dual,
        // EPR reach-scoped topics and any other elohim/ prefixed topics (D.3).
        t if t.starts_with("elohim/") => TopicTransports::Dual,
        // Default conservative: dual. New topics must explicitly opt out.
        _ => TopicTransports::Dual,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_topic_is_dual() {
        assert_eq!(
            classify_topic("elohim/inventory/blob"),
            TopicTransports::Dual
        );
    }

    #[test]
    fn identity_binding_topic_is_dual() {
        assert_eq!(
            classify_topic("elohim/identity/binding"),
            TopicTransports::Dual
        );
    }

    #[test]
    fn recovery_invitation_is_dual() {
        assert_eq!(classify_topic("recovery.invitation"), TopicTransports::Dual);
    }

    #[test]
    fn recovery_revocation_is_dual() {
        assert_eq!(classify_topic("recovery.revocation"), TopicTransports::Dual);
    }

    #[test]
    fn integrity_revocation_is_dual() {
        assert_eq!(
            classify_topic("elohim/integrity/revocation"),
            TopicTransports::Dual
        );
    }

    #[test]
    fn feedback_signal_prefix_is_dual() {
        assert_eq!(
            classify_topic("/elohim/feedback-signal/bafyreiabcdef0001"),
            TopicTransports::Dual
        );
    }

    #[test]
    fn reach_scoped_epr_topic_is_dual() {
        // D.3 reach-scoped EPR announce topics
        assert_eq!(classify_topic("elohim/lamad/public"), TopicTransports::Dual);
        assert_eq!(
            classify_topic("elohim/shefa/trusted/coll-42"),
            TopicTransports::Dual
        );
    }

    #[test]
    fn unknown_topic_defaults_to_dual() {
        // Conservative default: unknown topics get dual coverage.
        assert_eq!(classify_topic("some.unknown.topic"), TopicTransports::Dual);
    }
}
