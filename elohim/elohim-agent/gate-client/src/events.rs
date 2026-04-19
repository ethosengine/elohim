//! RelationalImpactEvent — the closed enum of event kinds the gate recognizes.
//!
//! All type definitions live in `gate-types::events`; this module re-exports
//! them for backward compatibility with any code importing
//! `gate_client::events::RelationalImpactEvent`.
//!
//! Tests for space-type inference remain here because they exercise
//! `gate_client::space::detect_from_event`, which is gate-client logic.

pub use gate_types::events::RelationalImpactEvent;

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Helper constructors ──────────────────────────────────────────────────

    fn content_publish() -> RelationalImpactEvent {
        RelationalImpactEvent::ContentPublish {
            content_cid: "bafkreiabc".to_string(),
            declared_reach: "public".to_string(),
            author: "agent-alice".to_string(),
        }
    }

    fn attestation_write() -> RelationalImpactEvent {
        RelationalImpactEvent::AttestationWrite {
            subject_hash: "uhCkk-abc".to_string(),
            claim_kind: "brit".to_string(),
            issuer: "agent-bob".to_string(),
        }
    }

    fn economic_event_emit() -> RelationalImpactEvent {
        RelationalImpactEvent::EconomicEventEmit {
            event_kind: "transfer".to_string(),
            provider: "agent-carol".to_string(),
            receiver: "agent-dave".to_string(),
            quantity: "10".to_string(),
        }
    }

    fn peer_message() -> RelationalImpactEvent {
        RelationalImpactEvent::PeerMessage {
            recipient: "agent-eve".to_string(),
            payload_kind: "handshake".to_string(),
        }
    }

    fn sync_to_peers() -> RelationalImpactEvent {
        RelationalImpactEvent::SyncToPeers {
            manifest_cid: "bafkreidef".to_string(),
            item_count: 42,
        }
    }

    fn advice_sought() -> RelationalImpactEvent {
        RelationalImpactEvent::AdviceSought {
            requester: "agent-frank".to_string(),
            summary_cid: "bafkreighi".to_string(),
            topic: "conflict-resolution".to_string(),
        }
    }

    fn capability_invoke() -> RelationalImpactEvent {
        RelationalImpactEvent::CapabilityInvoke {
            capability: "translate".to_string(),
            requester: "agent-grace".to_string(),
            request_id: "req-001".to_string(),
        }
    }

    fn private_to_public_crossing() -> RelationalImpactEvent {
        RelationalImpactEvent::PrivateToPublicCrossing {
            source_space: "private-draft".to_string(),
            artifact_ref: "bafkreijkl".to_string(),
        }
    }

    // ─── kind() string tests — one per variant ────────────────────────────────

    #[test]
    fn content_publish_kind() {
        assert_eq!(content_publish().kind(), "content-publish");
    }

    #[test]
    fn attestation_write_kind() {
        assert_eq!(attestation_write().kind(), "attestation-write");
    }

    #[test]
    fn economic_event_emit_kind() {
        assert_eq!(economic_event_emit().kind(), "economic-event-emit");
    }

    #[test]
    fn peer_message_kind() {
        assert_eq!(peer_message().kind(), "peer-message");
    }

    #[test]
    fn sync_to_peers_kind() {
        assert_eq!(sync_to_peers().kind(), "sync-to-peers");
    }

    #[test]
    fn advice_sought_kind() {
        assert_eq!(advice_sought().kind(), "advice-sought");
    }

    #[test]
    fn capability_invoke_kind() {
        assert_eq!(capability_invoke().kind(), "capability-invoke");
    }

    #[test]
    fn private_to_public_crossing_kind() {
        assert_eq!(
            private_to_public_crossing().kind(),
            "private-to-public-crossing"
        );
    }

    // ─── Serde round-trip tests — one per variant ─────────────────────────────
    //
    // These tests confirm: (a) the event serializes without panic, (b) the
    // serde `tag = "kind"` discriminant survives the round-trip, and (c) the
    // deserialized value's kind() matches the original.

    fn round_trip(event: &RelationalImpactEvent) -> RelationalImpactEvent {
        let json = serde_json::to_string(event).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    #[test]
    fn content_publish_round_trip() {
        let orig = content_publish();
        let rt = round_trip(&orig);
        assert_eq!(rt.kind(), orig.kind());
        // Confirm the tag discriminant is present in the JSON.
        let json = serde_json::to_string(&orig).unwrap();
        assert!(json.contains("\"kind\":\"content-publish\""), "got: {json}");
    }

    #[test]
    fn attestation_write_round_trip() {
        let orig = attestation_write();
        let rt = round_trip(&orig);
        assert_eq!(rt.kind(), orig.kind());
        let json = serde_json::to_string(&orig).unwrap();
        assert!(
            json.contains("\"kind\":\"attestation-write\""),
            "got: {json}"
        );
    }

    #[test]
    fn economic_event_emit_round_trip() {
        let orig = economic_event_emit();
        let rt = round_trip(&orig);
        assert_eq!(rt.kind(), orig.kind());
        let json = serde_json::to_string(&orig).unwrap();
        assert!(
            json.contains("\"kind\":\"economic-event-emit\""),
            "got: {json}"
        );
    }

    #[test]
    fn peer_message_round_trip() {
        let orig = peer_message();
        let rt = round_trip(&orig);
        assert_eq!(rt.kind(), orig.kind());
        let json = serde_json::to_string(&orig).unwrap();
        assert!(json.contains("\"kind\":\"peer-message\""), "got: {json}");
    }

    #[test]
    fn sync_to_peers_round_trip() {
        let orig = sync_to_peers();
        let rt = round_trip(&orig);
        assert_eq!(rt.kind(), orig.kind());
        let json = serde_json::to_string(&orig).unwrap();
        assert!(json.contains("\"kind\":\"sync-to-peers\""), "got: {json}");
    }

    #[test]
    fn advice_sought_round_trip() {
        let orig = advice_sought();
        let rt = round_trip(&orig);
        assert_eq!(rt.kind(), orig.kind());
        let json = serde_json::to_string(&orig).unwrap();
        assert!(json.contains("\"kind\":\"advice-sought\""), "got: {json}");
    }

    #[test]
    fn capability_invoke_round_trip() {
        let orig = capability_invoke();
        let rt = round_trip(&orig);
        assert_eq!(rt.kind(), orig.kind());
        let json = serde_json::to_string(&orig).unwrap();
        assert!(
            json.contains("\"kind\":\"capability-invoke\""),
            "got: {json}"
        );
    }

    #[test]
    fn private_to_public_crossing_round_trip() {
        let orig = private_to_public_crossing();
        let rt = round_trip(&orig);
        assert_eq!(rt.kind(), orig.kind());
        let json = serde_json::to_string(&orig).unwrap();
        assert!(
            json.contains("\"kind\":\"private-to-public-crossing\""),
            "got: {json}"
        );
    }

    // ─── Space-type inference per event variant ───────────────────────────────

    #[test]
    fn content_publish_infers_public_space() {
        let ctx = crate::space::detect_from_event(&content_publish());
        assert_eq!(ctx.space_type, crate::space::SpaceType::Public);
        assert!(!ctx.is_exempt());
    }

    #[test]
    fn attestation_write_infers_public_space() {
        let ctx = crate::space::detect_from_event(&attestation_write());
        assert_eq!(ctx.space_type, crate::space::SpaceType::Public);
        assert!(!ctx.is_exempt());
    }

    #[test]
    fn economic_event_emit_infers_public_space() {
        let ctx = crate::space::detect_from_event(&economic_event_emit());
        assert_eq!(ctx.space_type, crate::space::SpaceType::Public);
        assert!(!ctx.is_exempt());
    }

    #[test]
    fn peer_message_infers_public_space() {
        let ctx = crate::space::detect_from_event(&peer_message());
        assert_eq!(ctx.space_type, crate::space::SpaceType::Public);
        assert!(!ctx.is_exempt());
    }

    #[test]
    fn sync_to_peers_infers_sync_after_offline() {
        let ctx = crate::space::detect_from_event(&sync_to_peers());
        assert_eq!(ctx.space_type, crate::space::SpaceType::SyncAfterOffline);
        assert!(!ctx.is_exempt());
        assert!(ctx.space_type.is_boundary_crossing());
    }

    #[test]
    fn advice_sought_infers_advice_seeking_boundary_crossing() {
        let ctx = crate::space::detect_from_event(&advice_sought());
        assert_eq!(ctx.space_type, crate::space::SpaceType::AdviceSeeking);
        assert!(!ctx.is_exempt());
        assert!(ctx.space_type.is_boundary_crossing());
    }

    #[test]
    fn capability_invoke_infers_public_space() {
        let ctx = crate::space::detect_from_event(&capability_invoke());
        assert_eq!(ctx.space_type, crate::space::SpaceType::Public);
        assert!(!ctx.is_exempt());
    }

    #[test]
    fn private_to_public_crossing_infers_private_drafting_crossing() {
        let ctx = crate::space::detect_from_event(&private_to_public_crossing());
        assert_eq!(
            ctx.space_type,
            crate::space::SpaceType::PrivateDraftingCrossing
        );
        assert!(!ctx.is_exempt());
        assert!(ctx.space_type.is_boundary_crossing());
    }
}
