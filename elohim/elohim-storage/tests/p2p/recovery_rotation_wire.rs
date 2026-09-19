use elohim_storage::p2p::recovery_rotation::RecoveryRotationMessage;

#[test]
fn rotation_message_roundtrips_msgpack() {
    let original = RecoveryRotationMessage {
        rotation_id: "uhCAkRecoveryRequest1".into(),
        human_agent_pubkey: "uhCAkHumanAgentPubkeyBase64".into(),
        new_agent_pubkey: "uhCAkNewAgentPubkeyBase64".into(),
        superseded_agent_pubkey: "uhCAkPreviousAgentPubkeyBase64".into(),
        recovery_request_hash: "uhCAkRecoveryRequest1".into(),
        rotated_at: "2026-05-11T12:00:00Z".into(),
        sender_peer_id: "12D3KooWtest".into(),
        sent_at: "2026-05-11T12:00:01Z".into(),
    };

    let bytes = original.to_bytes().expect("encode");
    let decoded = RecoveryRotationMessage::from_bytes(&bytes).expect("decode");
    assert_eq!(decoded, original);
}

#[test]
fn rotation_wire_bytes_are_small() {
    let msg = RecoveryRotationMessage {
        rotation_id: "uhCAkRecoveryRequest1".into(),
        human_agent_pubkey: "uhCAkHumanAgentPubkeyBase64".into(),
        new_agent_pubkey: "uhCAkNewAgentPubkeyBase64".into(),
        superseded_agent_pubkey: "uhCAkPreviousAgentPubkeyBase64".into(),
        recovery_request_hash: "uhCAkRecoveryRequest1".into(),
        rotated_at: "2026-05-11T12:00:00Z".into(),
        sender_peer_id: "12D3KooWtest".into(),
        sent_at: "2026-05-11T12:00:01Z".into(),
    };
    // MessagePack should keep this well under 400 bytes for typical payloads.
    assert!(msg.to_bytes().unwrap().len() < 400);
}
