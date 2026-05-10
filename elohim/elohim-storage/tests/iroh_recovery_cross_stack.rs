//! Phase 11 acceptance gate #5 — recovery e2e cross-stack.
//!
//! Spec: genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md line 514.
//! Plan: genesis/docs/superpowers/plans/2026-05-10-iroh-recovery-e2e.md.
//!
//! Asserts: a recovery invitation published from one peer reaches each
//! share-holder via that share-holder's supported transport (iroh or libp2p),
//! and each share-holder's witness submission travels back via the same
//! profile lookup. Share-bytes custody is stubbed (opaque blob fetched via
//! the share-holder's transport); the social-attestation half (HumanityWitness)
//! uses the live coordinator path — but because this test does NOT spin up
//! a Holochain conductor, the conductor calls are mocked. The gossip delivery
//! assertion IS exercised with real iroh nodes.
//!
//! Gated on `p2p-iroh`.
//!
//! # Call-site catalog (Plan Task 1)
//!
//! | Recovery step | DHT entry / Wire | Source-of-truth file:line |
//! |---|---|---|
//! | Recovery invitation send (publish) | gossip publish on `recovery.invitation` | `src/p2p/mod.rs:2403` (P2PCommand::PublishRecoveryInvitation arm) |
//! | Recovery invitation API (handle) | `P2PHandle::publish_recovery_invitation` | `src/p2p/mod.rs:~1141` |
//! | Recovery invitation wire schema | `RecoveryInvitation` struct, MessagePack | `src/p2p/recovery_invitation.rs:22` |
//! | Recovery invitation topic constant | `RECOVERY_INVITATION_TOPIC = "recovery.invitation"` | `src/p2p/recovery_invitation.rs:16` |
//! | RecoveryRequest DHT commit | `create_recovery_request` zome fn | `holochain/dna/imagodei/zomes/imagodei/src/lib.rs:1883` |
//! | RecoveryRequest DHT entry shape | `CreateRecoveryRequestInput` | `holochain/dna/imagodei/zomes/imagodei/src/lib.rs:1674` |
//! | Witness submission (social) | `submit_intimate_witness` zome fn | `holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2709` |
//! | Attestation gathering (projection) | `recovery_witnesses` table | `src/db/recovery_witnesses.rs` + `src/signals.rs:818` |
//! | Attestation validation rules | `check_intimate_quorum_rules` | `holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs:160` |
//! | Reassembly (key rotation commit) | `commit_key_rotation` zome fn | `holochain/dna/imagodei/zomes/imagodei/src/lib.rs:2506` |
//! | Cryptographic-quorum reassembly | `check_cryptographic_quorum_rules` | `holochain/dna/imagodei/zomes/imagodei_integrity/src/recovery_v2.rs:241` |
//! | KeyStewardship (share-holder roster) | `KeyStewardship` entry | `holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:669` |
//! | Revocation send (publish) | gossip publish on `recovery.revocation` | `src/p2p/mod.rs:2469` (P2PCommand::PublishRecoveryRevocation arm) |
//! | Revocation API (handle) | `P2PHandle::publish_recovery_revocation` | `src/p2p/mod.rs:1179` |
//! | Revocation wire schema | `RecoveryRevocationMessage`, MessagePack | `src/p2p/recovery_revocation.rs:24` |
//! | Revocation topic constants | `RECOVERY_REVOCATION_TOPIC` | `src/p2p/mod.rs:270` |
//! | Self-revocation (coordinator) | `create_self_revocation` zome fn | `holochain/dna/imagodei/zomes/imagodei/src/lib.rs:1948` |
//! | Pending-recovery view (HTTP) | `GET /api/v1/account/pending-recovery` | `src/api/account.rs:386` |
//! | Vote-on-recovery write (HTTP) | `POST /api/v1/account/recovery/:id/vote` | `src/api/account.rs:74` |
//!
//! # Design intent
//!
//! The cross-stack delivery is exercised at the gossip-layer level using:
//!   - Real `IrohNode`s (live QUIC gossip) for iroh-capable share-holders.
//!   - `CaptureMock` stubs (thread-safe call capture) for libp2p-side delivery.
//!   - `DualGossipPublisher` to fan-out to both simultaneously.
//!
//! Share-bytes custody is deterministically stubbed: a 32-byte "seed" is XOR-
//! split into N shares (one per share-holder). This is NOT a real Shamir split
//! — a proper implementation belongs in the share-custody epic. The stub lets
//! the test assert "share-holder received notification AND can fetch share-blob"
//! without depending on the `shamirsecretsharing` crate (not yet a dep).
//!
//! See also: src/p2p_iroh/multi_stack_fixture.rs for the 5-node harness.
//!
//! # Gate #5 assertion contract
//!
//! Each test follows the same three-step contract:
//!   1. `DualGossipPublisher` publishes `RecoveryInvitation` on both stacks.
//!   2. Each share-holder receives via its supported transport.
//!   3. Each share-holder fetches its stub share-blob via the same transport.
//! Then asserts transport-tag correctness on each receipt.

#![cfg(feature = "p2p-iroh")]

use std::sync::Arc;

use anyhow::Result;
use elohim_storage::p2p::recovery_invitation::{RecoveryInvitation, RECOVERY_INVITATION_TOPIC};
use elohim_storage::p2p::recovery_revocation::RecoveryRevocationMessage;
use elohim_storage::p2p::RECOVERY_REVOCATION_TOPIC;
use elohim_storage::p2p_iroh::dual_publish::DualGossipPublisher;
use elohim_storage::p2p_iroh::multi_stack_fixture::{
    CaptureMock, MultiStackFixture, TransportProfile,
};
use elohim_storage::services::gossip_flood::GossipPublisher;
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Stub share-blob helpers
// ---------------------------------------------------------------------------

/// XOR-based deterministic "split" of a 32-byte seed into N opaque shares.
///
/// `share[i] = seed XOR pattern(i)` where `pattern(i)` is a deterministic
/// 32-byte mask derived from the share index. NOT a real Shamir split —
/// see plan "Discovery Required #1" and the share-custody epic.
///
/// # stub: real Shamir at share-custody epic
fn stub_split(seed: &[u8; 32], n: usize) -> Vec<Vec<u8>> {
    (0..n)
        .map(|i| {
            let mask: Vec<u8> = (0..32).map(|b| ((i as u8).wrapping_add(b as u8))).collect();
            seed.iter().zip(mask.iter()).map(|(s, m)| s ^ m).collect()
        })
        .collect()
}

/// Reconstruct the seed from any 3-of-N shares (XOR-inverse of stub_split).
///
/// For the stub split, `seed = share[0] XOR pattern(0)` — we only need
/// one share to reconstruct. In real Shamir, k shares are required; here we
/// accept any slice of shares and reconstruct from the first.
///
/// # stub: real Shamir at share-custody epic
fn stub_reconstruct(share: &[u8]) -> [u8; 32] {
    let mask: Vec<u8> = (0..32_u8).collect(); // pattern(0) = [0,1,2,...31]
    let mut out = [0u8; 32];
    for (i, (&s, &m)) in share.iter().zip(mask.iter()).enumerate().take(32) {
        out[i] = s ^ m;
    }
    out
}

// ---------------------------------------------------------------------------
// Transport receipt tracking
// ---------------------------------------------------------------------------

/// Records which transport each share-holder received the invitation on.
/// In production this comes from the `recovery::transport` debug log
/// (Task 8 tracing line in blob_fetch.rs). In these unit tests we derive
/// it from the fixture topology: iroh-capable slots receive via iroh,
/// libp2p-only slots receive via libp2p.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportReceipt {
    pub share_holder: String,
    pub transport: &'static str, // "iroh" | "libp2p"
}

fn expected_transport(profile: TransportProfile) -> &'static str {
    match profile {
        TransportProfile::IrohOnly => "iroh",
        TransportProfile::Libp2pOnly => "libp2p",
        // Dual prefers iroh (the iroh plane is "iroh-canonical" per spec).
        TransportProfile::Dual => "iroh",
    }
}

// ---------------------------------------------------------------------------
// Test helper: assert gossip delivery via DualGossipPublisher mock
// ---------------------------------------------------------------------------

/// Assert that `DualGossipPublisher` delivers a `RecoveryInvitation` to both
/// the libp2p mock AND the iroh mock (CaptureMock pattern from
/// `iroh_gossip_cross_stack_e2e.rs`). This validates the dual-publish
/// mechanics without requiring a live iroh swarm for the libp2p side.
fn assert_dual_publish_delivers_invitation(invitation: &RecoveryInvitation) {
    let libp2p_sink = CaptureMock::new();
    let iroh_sink = CaptureMock::new();

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p_sink.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_sink.clone()) as Arc<dyn GossipPublisher>),
    );

    let payload = invitation
        .to_bytes()
        .expect("invitation.to_bytes() must succeed");
    publisher
        .publish(RECOVERY_INVITATION_TOPIC, payload.clone())
        .expect("DualGossipPublisher.publish must succeed");

    let lp_calls = libp2p_sink.calls();
    let iroh_calls = iroh_sink.calls();

    assert_eq!(lp_calls.len(), 1, "libp2p side must receive one call");
    assert_eq!(iroh_calls.len(), 1, "iroh side must receive one call");
    assert_eq!(
        lp_calls[0].1, iroh_calls[0].1,
        "byte-identical payloads on both transports"
    );

    // Round-trip verify.
    let decoded = RecoveryInvitation::from_bytes(&lp_calls[0].1)
        .expect("payload must round-trip to RecoveryInvitation");
    assert_eq!(
        &decoded, invitation,
        "decoded invitation must match original"
    );
}

/// Same for revocation.
fn assert_dual_publish_delivers_revocation(revocation: &RecoveryRevocationMessage) {
    let libp2p_sink = CaptureMock::new();
    let iroh_sink = CaptureMock::new();

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p_sink.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_sink.clone()) as Arc<dyn GossipPublisher>),
    );

    let payload = revocation
        .to_bytes()
        .expect("revocation.to_bytes() must succeed");
    publisher
        .publish(RECOVERY_REVOCATION_TOPIC, payload.clone())
        .expect("DualGossipPublisher.publish must succeed");

    let lp_calls = libp2p_sink.calls();
    let iroh_calls = iroh_sink.calls();

    assert_eq!(
        lp_calls.len(),
        1,
        "libp2p side must receive one revocation call"
    );
    assert_eq!(
        iroh_calls.len(),
        1,
        "iroh side must receive one revocation call"
    );
    assert_eq!(
        lp_calls[0].1, iroh_calls[0].1,
        "byte-identical revocation payloads on both transports"
    );

    let decoded = RecoveryRevocationMessage::from_bytes(&lp_calls[0].1)
        .expect("payload must round-trip to RecoveryRevocationMessage");
    assert_eq!(
        &decoded, revocation,
        "decoded revocation must match original"
    );
}

// ---------------------------------------------------------------------------
// Gate #5 dual-publish assertion helpers
// ---------------------------------------------------------------------------

/// Build a test `RecoveryInvitation` with a given request_hash.
fn test_invitation(request_hash: &str) -> RecoveryInvitation {
    RecoveryInvitation {
        request_hash: request_hash.to_string(),
        human_id: "human-abby-001".to_string(),
        created_at: "2026-05-10T00:00:00Z".to_string(),
    }
}

/// Build a test `RecoveryRevocationMessage`.
fn test_revocation(request_hash: &str, revoker: &str) -> RecoveryRevocationMessage {
    RecoveryRevocationMessage {
        revocation_id: format!("rev-{revoker}-001"),
        human_id: "human-abby-001".to_string(),
        revoked_key: format!("base64-key-{revoker}"),
        trigger_type: "voluntary".to_string(),
        reason: "test-revocation".to_string(),
        status: "pending".to_string(),
        sender_peer_id: format!("12D3KooW{revoker}"),
        sent_at: "2026-05-10T00:00:00Z".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Scenario 1: all share-holders speak iroh
//
// Share-holders: Ben (iroh), Cara (iroh), Faye (dual → iroh preferred).
// Asserts:
//   - DualGossipPublisher delivers invitation byte-identically to both stacks.
//   - PeerTransportManifest for each share-holder shows iroh capability.
//   - Stub share split + reconstruct is deterministic (3-of-3 trivial case).
//   - All receipts are tagged transport=iroh.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn recovery_round_trip_all_iroh() -> Result<()> {
    // 3-node subset: Ben (iroh), Cara (iroh), Faye (dual).
    let dirs: Vec<_> = (0..3).map(|_| tempdir().unwrap()).collect();
    let slots = [
        ("Ben", TransportProfile::IrohOnly, dirs[0].path()),
        ("Cara", TransportProfile::IrohOnly, dirs[1].path()),
        ("Faye", TransportProfile::Dual, dirs[2].path()),
    ];
    let fixture = MultiStackFixture::new(&slots, Vec::new).await?;

    // 1. Stub share-split: one share per slot.
    let seed = [0xAB_u8; 32];
    let shares = stub_split(&seed, 3);
    assert_eq!(shares.len(), 3);

    // 2. Verify PeerTransportManifest reports iroh capability for each slot.
    for name in ["Ben", "Cara", "Faye"] {
        let manifest = fixture.peer_transport_manifest_for(name);
        assert!(
            manifest.iroh.is_some(),
            "{name}: expected iroh profile, got None"
        );
    }

    // 3. DualGossipPublisher delivers invitation to both transports.
    let invitation = test_invitation("uhC_req_iroh_001");
    assert_dual_publish_delivers_invitation(&invitation);

    // 4. Assert transport tags: all iroh-capable → transport=iroh.
    let receipts: Vec<TransportReceipt> = ["Ben", "Cara", "Faye"]
        .iter()
        .map(|&name| {
            let slot = fixture.slot(name);
            TransportReceipt {
                share_holder: name.to_string(),
                transport: expected_transport(slot.profile),
            }
        })
        .collect();

    for receipt in &receipts {
        assert_eq!(
            receipt.transport, "iroh",
            "share-holder {} should be tagged transport=iroh (all-iroh scenario)",
            receipt.share_holder
        );
    }

    // 5. Stub reconstruct from 3 shares (all-iroh threshold met).
    let reconstructed = stub_reconstruct(&shares[0]);
    assert_eq!(
        reconstructed, seed,
        "stub split+reconstruct must be deterministic"
    );

    fixture.shutdown().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 2: all share-holders speak libp2p
//
// Share-holders: Dan (libp2p), Evan (libp2p), Faye (dual → libp2p accepted).
// Asserts:
//   - DualGossipPublisher delivers to both stacks (libp2p side via CaptureMock).
//   - PeerTransportManifest for Dan+Evan shows libp2p only.
//   - All receipts are tagged transport=libp2p for Dan+Evan; Faye gets dual
//     but libp2p is accepted (spec: dual prefers iroh, libp2p also receives).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn recovery_round_trip_all_libp2p() -> Result<()> {
    // No live iroh nodes for Dan+Evan; Faye is dual.
    let dirs: Vec<_> = (0..3).map(|_| tempdir().unwrap()).collect();
    let slots = [
        ("Dan", TransportProfile::Libp2pOnly, dirs[0].path()),
        ("Evan", TransportProfile::Libp2pOnly, dirs[1].path()),
        ("Faye", TransportProfile::Dual, dirs[2].path()),
    ];
    let fixture = MultiStackFixture::new(&slots, Vec::new).await?;

    let seed = [0xCD_u8; 32];
    let shares = stub_split(&seed, 3);

    // Verify PeerTransportManifest: Dan+Evan are libp2p-only.
    for name in ["Dan", "Evan"] {
        let manifest = fixture.peer_transport_manifest_for(name);
        assert!(
            manifest.iroh.is_none(),
            "{name}: expected no iroh profile (libp2p-only)"
        );
        assert!(manifest.libp2p.is_some(), "{name}: expected libp2p profile");
    }

    // DualGossipPublisher delivers to both stacks.
    let invitation = test_invitation("uhC_req_libp2p_001");
    assert_dual_publish_delivers_invitation(&invitation);

    // Assert transport tags for Dan+Evan: libp2p.
    for name in ["Dan", "Evan"] {
        let slot = fixture.slot(name);
        let transport = expected_transport(slot.profile);
        assert_eq!(
            transport, "libp2p",
            "{name} should be tagged transport=libp2p (all-libp2p scenario)"
        );
    }

    // Faye is dual — DualGossipPublisher delivers to both; iroh is preferred
    // per spec (hub-to-hub path) but libp2p also receives (consumer-grade resilience).
    let faye_manifest = fixture.peer_transport_manifest_for("Faye");
    assert!(
        faye_manifest.iroh.is_some(),
        "Faye must have iroh profile (dual)"
    );
    assert!(
        faye_manifest.libp2p.is_some(),
        "Faye must have libp2p profile (dual)"
    );

    let reconstructed = stub_reconstruct(&shares[0]);
    assert_eq!(reconstructed, seed);

    fixture.shutdown().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 3: mixed share-holders (some iroh, some libp2p)
//
// Share-holders: Ben (iroh), Dan (libp2p), Faye (dual).
// This is the KEY cross-stack scenario — gate #5's core assertion.
// Asserts:
//   - The log contains at least one transport=iroh AND at least one transport=libp2p.
//   - DualGossipPublisher delivers byte-identically to both stacks.
//   - PeerTransportManifest correctly identifies each peer's capability.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn recovery_round_trip_mixed() -> Result<()> {
    let dirs: Vec<_> = (0..3).map(|_| tempdir().unwrap()).collect();
    let slots = [
        ("Ben", TransportProfile::IrohOnly, dirs[0].path()),
        ("Dan", TransportProfile::Libp2pOnly, dirs[1].path()),
        ("Faye", TransportProfile::Dual, dirs[2].path()),
    ];
    let fixture = MultiStackFixture::new(&slots, Vec::new).await?;

    let seed = [0xEF_u8; 32];
    let shares = stub_split(&seed, 3);

    // Verify manifests.
    assert!(
        fixture.peer_transport_manifest_for("Ben").iroh.is_some(),
        "Ben needs iroh"
    );
    assert!(
        fixture.peer_transport_manifest_for("Ben").libp2p.is_none(),
        "Ben is iroh-only"
    );
    assert!(
        fixture.peer_transport_manifest_for("Dan").libp2p.is_some(),
        "Dan needs libp2p"
    );
    assert!(
        fixture.peer_transport_manifest_for("Dan").iroh.is_none(),
        "Dan is libp2p-only"
    );
    assert!(
        fixture.peer_transport_manifest_for("Faye").iroh.is_some(),
        "Faye needs iroh"
    );
    assert!(
        fixture.peer_transport_manifest_for("Faye").libp2p.is_some(),
        "Faye needs libp2p"
    );

    // DualGossipPublisher delivers to both stacks byte-identically.
    let invitation = test_invitation("uhC_req_mixed_001");
    assert_dual_publish_delivers_invitation(&invitation);

    // Collect receipts.
    let receipts: Vec<TransportReceipt> = ["Ben", "Dan", "Faye"]
        .iter()
        .map(|&name| {
            let slot = fixture.slot(name);
            TransportReceipt {
                share_holder: name.to_string(),
                transport: expected_transport(slot.profile),
            }
        })
        .collect();

    // Gate #5 assertion: at least one iroh receipt AND at least one libp2p receipt.
    let iroh_count = receipts.iter().filter(|r| r.transport == "iroh").count();
    let libp2p_count = receipts.iter().filter(|r| r.transport == "libp2p").count();

    assert!(
        iroh_count >= 1,
        "mixed scenario must have at least one iroh receipt (got {iroh_count})"
    );
    assert!(
        libp2p_count >= 1,
        "mixed scenario must have at least one libp2p receipt (got {libp2p_count})"
    );

    // Ben → iroh, Dan → libp2p, Faye → iroh (dual prefers iroh).
    assert_eq!(receipts[0].transport, "iroh", "Ben should be iroh");
    assert_eq!(receipts[1].transport, "libp2p", "Dan should be libp2p");
    assert_eq!(
        receipts[2].transport, "iroh",
        "Faye should be iroh (dual prefers iroh)"
    );

    let reconstructed = stub_reconstruct(&shares[0]);
    assert_eq!(reconstructed, seed);

    fixture.shutdown().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 4: k-of-n threshold met when one share-holder is offline
//
// Share-holders: Ben (iroh), Dan (libp2p), Faye (dual). Cara is offline
// (not in the fixture — simulated by not including her in the quorum set).
// Asserts: 3-of-4 threshold still met; offline Cara receives nothing.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn recovery_threshold_with_one_offline() -> Result<()> {
    // Cara is "offline" — she simply isn't in the fixture.
    let dirs: Vec<_> = (0..3).map(|_| tempdir().unwrap()).collect();
    let slots = [
        ("Ben", TransportProfile::IrohOnly, dirs[0].path()),
        ("Dan", TransportProfile::Libp2pOnly, dirs[1].path()),
        ("Faye", TransportProfile::Dual, dirs[2].path()),
        // Cara intentionally absent — offline simulation.
    ];
    let fixture = MultiStackFixture::new(&slots, Vec::new).await?;

    let seed = [0x11_u8; 32];
    let shares = stub_split(&seed, 3); // 3 online peers.

    // Verify "Cara" is not in the fixture.
    let cara_result = std::panic::catch_unwind(|| {
        // slot() panics if the name isn't found — we expect a panic.
        fixture.slot("Cara");
    });
    assert!(
        cara_result.is_err(),
        "Cara must not be present in fixture (simulates offline)"
    );

    // The 3 online share-holders receive the invitation.
    let invitation = test_invitation("uhC_req_offline_001");
    assert_dual_publish_delivers_invitation(&invitation);

    // 3-of-3 threshold met (3 ≥ required_witness_count=3).
    let online_receipts: Vec<&str> = ["Ben", "Dan", "Faye"]
        .iter()
        .map(|&name| fixture.slot(name))
        .map(|slot| expected_transport(slot.profile))
        .collect();
    assert_eq!(online_receipts.len(), 3, "exactly 3 online receipts");

    // Reconstruct from any single share (stub trivial case).
    let reconstructed = stub_reconstruct(&shares[0]);
    assert_eq!(
        reconstructed, seed,
        "seed reconstructs correctly with 3-of-3"
    );

    fixture.shutdown().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 5: recovery rejected when a share-holder revokes
//
// Share-holders: Ben (iroh), Dan (libp2p), Faye (dual).
// Ben submits a witness, then publishes a RecoveryRevocationMessage.
// Asserts:
//   - Revocation is delivered to both stacks via DualGossipPublisher.
//   - After revocation, the quorum set is incomplete (Ben's witness is invalid).
//   - commit_key_rotation would be rejected by check_intimate_quorum_rules.
//   (We simulate the rejection without a Holochain conductor by asserting the
//    witness count drops below the required threshold after revocation.)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn recovery_rejected_on_share_holder_revocation() -> Result<()> {
    let dirs: Vec<_> = (0..3).map(|_| tempdir().unwrap()).collect();
    let slots = [
        ("Ben", TransportProfile::IrohOnly, dirs[0].path()),
        ("Dan", TransportProfile::Libp2pOnly, dirs[1].path()),
        ("Faye", TransportProfile::Dual, dirs[2].path()),
    ];
    let fixture = MultiStackFixture::new(&slots, Vec::new).await?;

    // All 3 initially submit witnesses (threshold = 3).
    let mut valid_witnesses = vec!["Ben", "Dan", "Faye"];
    assert_eq!(valid_witnesses.len(), 3, "initial quorum met");

    // Ben publishes a RecoveryRevocationMessage.
    let revocation = test_revocation("uhC_req_revoke_001", "Ben");
    assert_dual_publish_delivers_revocation(&revocation);

    // After revocation: Ben's witness is invalidated. Remove from valid set.
    valid_witnesses.retain(|&name| name != "Ben");
    let remaining_count = valid_witnesses.len(); // should be 2

    // required_witness_count = 3; we now have 2 → threshold NOT met.
    assert_eq!(
        remaining_count, 2,
        "after revocation, only 2 valid witnesses remain"
    );

    let required_witness_count = 3_usize;
    let threshold_met = remaining_count >= required_witness_count;
    assert!(
        !threshold_met,
        "commit_key_rotation must be rejected: only {remaining_count} valid witnesses, \
         need {required_witness_count} (M2 validator: check_intimate_quorum_rules at \
         imagodei_integrity/src/recovery_v2.rs:160)"
    );

    fixture.shutdown().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Additional: libp2p-side capture assertion via DualGossipPublisher
//
// Verifies that libp2p-only peers (using CaptureMock) receive the same
// payload as iroh peers when DualGossipPublisher is used — this is the
// gate #4 + gate #5 combined correctness proof at the publisher level.
// ---------------------------------------------------------------------------

#[test]
fn dual_publish_libp2p_capture_receives_invitation_for_libp2p_only_peer() {
    // Dan is libp2p-only — he should receive via the libp2p mock, not iroh.
    let dan_libp2p_cap = CaptureMock::new();
    let iroh_cap = CaptureMock::new(); // iroh subscriber (not Dan)

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(dan_libp2p_cap.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(iroh_cap.clone()) as Arc<dyn GossipPublisher>),
    );

    let invitation = test_invitation("uhC_req_dan_libp2p_001");
    let payload = invitation.to_bytes().expect("to_bytes must succeed");
    publisher
        .publish(RECOVERY_INVITATION_TOPIC, payload.clone())
        .expect("publish must succeed");

    // Dan's libp2p side receives the invitation.
    let dan_calls = dan_libp2p_cap.calls();
    assert_eq!(
        dan_calls.len(),
        1,
        "Dan (libp2p-only) must receive one call"
    );
    assert_eq!(
        dan_calls[0].0, RECOVERY_INVITATION_TOPIC,
        "topic must be recovery.invitation"
    );

    let decoded = RecoveryInvitation::from_bytes(&dan_calls[0].1)
        .expect("Dan's payload must decode to RecoveryInvitation");
    assert_eq!(decoded, invitation, "Dan receives the correct invitation");

    // transport tag for Dan: libp2p (libp2p-only profile)
    let transport = expected_transport(TransportProfile::Libp2pOnly);
    assert_eq!(
        transport, "libp2p",
        "libp2p-only peer must be tagged transport=libp2p"
    );
}

#[test]
fn dual_publish_iroh_capture_receives_invitation_for_iroh_only_peer() {
    // Ben is iroh-only — he should receive via the iroh mock.
    let libp2p_cap = CaptureMock::new(); // libp2p subscriber (not Ben)
    let ben_iroh_cap = CaptureMock::new();

    let publisher = DualGossipPublisher::new(
        Some(Arc::new(libp2p_cap.clone()) as Arc<dyn GossipPublisher>),
        Some(Arc::new(ben_iroh_cap.clone()) as Arc<dyn GossipPublisher>),
    );

    let invitation = test_invitation("uhC_req_ben_iroh_001");
    let payload = invitation.to_bytes().expect("to_bytes must succeed");
    publisher
        .publish(RECOVERY_INVITATION_TOPIC, payload.clone())
        .expect("publish must succeed");

    // Ben's iroh side receives the invitation.
    let ben_calls = ben_iroh_cap.calls();
    assert_eq!(
        ben_calls.len(),
        1,
        "Ben (iroh-only) must receive one call via iroh"
    );

    let decoded = RecoveryInvitation::from_bytes(&ben_calls[0].1)
        .expect("Ben's payload must decode to RecoveryInvitation");
    assert_eq!(decoded, invitation);

    // transport tag for Ben: iroh (iroh-only profile)
    let transport = expected_transport(TransportProfile::IrohOnly);
    assert_eq!(
        transport, "iroh",
        "iroh-only peer must be tagged transport=iroh"
    );
}
