//! W2A — verify libp2p EPR Atom Announce of Content kind records the
//! sender PeerId as a predecessor; FeedbackSignal Announce does not.
//!
//! ## Design
//!
//! `P2PNode` requires a full libp2p swarm and is not constructible in a
//! lightweight integration test without binding ports and spawning tasks.
//! Per the plan note: "adapt the test to invoke the post-ingest
//! record_predecessor call directly with a synthetic EprAtomRequest::Announce
//! — the goal is to verify the kind-filter + record_predecessor wiring, not
//! to spin a swarm."
//!
//! This test exercises the _exact same logic_ that `handle_epr_atom_request`
//! applies after receiving an `EprAtomResponse::Announced { accepted: true }`:
//!   1. CBOR-decode the envelope bytes to extract (kind, cid).
//!   2. Filter to `EprKind::Content` only.
//!   3. Call `services::back_prop::record_predecessor` with the sealing keys.
//!
//! The transport-neutral ingest path uses `EprAtomService::handle` directly
//! (same function that `handle_epr_atom_request` delegates to), so the test
//! exercises real DB I/O through the real ingest + predecessor paths.

use chrono::{TimeZone, Utc};
use dryoc::classic::crypto_box::crypto_box_seed_keypair;
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use elohim_storage::p2p::dedup::DedupLru;
use elohim_storage::p2p::epr_atom_protocol::{EprAtomRequest, EprAtomResponse};
use elohim_storage::p2p::identity_map::CallerIdentity;
use elohim_storage::services::back_prop::{read_predecessors, record_predecessor, SealingPubKeys};
use elohim_storage::services::sealed_against_self::{
    ImagodeiPubKey, ImagodeiSecretKey, MishpatQuorumPubKey, MishpatQuorumSecretKey,
};
use elohim_storage::test_util::test_pool;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Keypair helpers (same deterministic seeds as existing T11/T12 tests)
// ---------------------------------------------------------------------------

fn mishpat_keypair() -> (MishpatQuorumPubKey, MishpatQuorumSecretKey) {
    let (pk, sk) = crypto_box_seed_keypair(&[0x10u8; 32]);
    (MishpatQuorumPubKey(pk), MishpatQuorumSecretKey(sk))
}

fn imagodei_keypair() -> (ImagodeiPubKey, ImagodeiSecretKey) {
    let (pk, sk) = crypto_box_seed_keypair(&[0x20u8; 32]);
    (ImagodeiPubKey(pk), ImagodeiSecretKey(sk))
}

fn sealing_pub_keys<'a>(
    mishpat_pk: &'a MishpatQuorumPubKey,
    imagodei_pk: &'a ImagodeiPubKey,
) -> SealingPubKeys<'a> {
    SealingPubKeys {
        mishpat_pk,
        imagodei_pk,
    }
}

// ---------------------------------------------------------------------------
// EPR construction helpers
// ---------------------------------------------------------------------------

fn cid_from_byte(b: u8) -> cid::Cid {
    compute_cid(&[b])
}

/// Build a CBOR-encoded Content-kind EPR for test use.
/// Uses a distinct payload so each call produces a unique CID.
fn build_content_envelope_bytes(payload: &[u8]) -> Vec<u8> {
    let kp = AgentKeypair::from_secret(&[0x42u8; 32]).unwrap();
    let agent_cid = cid_from_byte(100);

    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(cid_from_byte(1))
        .schema_key("concept")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: Some(cid_from_byte(2)),
            value: Some(cid_from_byte(3)),
            governance: Some(cid_from_byte(4)),
        })
        .claim(cid_from_byte(5))
        .issued_at(Utc.with_ymd_and_hms(2026, 5, 11, 12, 0, 0).unwrap())
        .payload(payload.to_vec())
        .sign(&kp, agent_cid)
        .expect("sign Content EPR");

    let mut buf = Vec::new();
    ciborium::ser::into_writer(&epr, &mut buf).expect("CBOR encode Content EPR");
    buf
}

/// Build a CBOR-encoded FeedbackSignal-kind EPR for test use.
/// FeedbackSignal only requires a Governance coupling leg.
fn build_feedback_signal_envelope_bytes(payload: &[u8]) -> Vec<u8> {
    let kp = AgentKeypair::from_secret(&[0x43u8; 32]).unwrap();
    let agent_cid = cid_from_byte(101);

    let epr = Epr::builder()
        .kind(EprKind::FeedbackSignal)
        .schema_ref(cid_from_byte(10))
        .schema_key("feedback-signal")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: None,
            value: None,
            governance: Some(cid_from_byte(11)),
        })
        .issued_at(Utc.with_ymd_and_hms(2026, 5, 11, 12, 1, 0).unwrap())
        .payload(payload.to_vec())
        .sign(&kp, agent_cid)
        .expect("sign FeedbackSignal EPR");

    let mut buf = Vec::new();
    ciborium::ser::into_writer(&epr, &mut buf).expect("CBOR encode FeedbackSignal EPR");
    buf
}

// ---------------------------------------------------------------------------
// Helper: apply the record_predecessor logic that handle_epr_atom_request uses.
//
// This replicates the exact production wiring so the test verifies the
// kind-filter + record_predecessor behaviour without spinning a libp2p swarm.
// ---------------------------------------------------------------------------

fn apply_predecessor_wiring(
    pool: &elohim_storage::db::DbPool,
    sealing_keys: &SealingPubKeys<'_>,
    envelope_bytes: &[u8],
    sender_peer_id: &str,
) {
    // Step 1: ingest via EprAtomService (same as P2PNode::handle_epr_atom_request delegates to).
    let dedup = Arc::new(DedupLru::new());
    let service = elohim_storage::epr_atom_service::EprAtomService::new(Some(pool.clone()), dedup);
    let request = EprAtomRequest::Announce {
        envelope_bytes: envelope_bytes.to_vec(),
    };
    let response = service.handle(sender_peer_id, CallerIdentity::Anonymous, request);

    // Step 2: after successful Announce, apply the same kind-filter + record call
    // that handle_epr_atom_request does (W2A wiring).
    if let EprAtomResponse::Announced { accepted: true, .. } = &response {
        // Decode to extract kind + cid (same lightweight decode as in p2p/mod.rs).
        if let Ok(epr) = ciborium::de::from_reader::<Epr, _>(envelope_bytes) {
            if epr.envelope.kind == EprKind::Content {
                let mut conn = pool.get().expect("pool connection");
                record_predecessor(
                    &mut conn,
                    &epr.envelope.cid.to_string(),
                    sender_peer_id,
                    sealing_keys,
                )
                .expect("record_predecessor must not fail in test");
            }
            // FeedbackSignal (and all other non-Content kinds): no record call.
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Content EPR Announce → predecessor row MUST be recorded.
#[test]
fn announce_content_epr_records_predecessor() {
    let pool = test_pool();
    let (mishpat_pk, mishpat_sk) = mishpat_keypair();
    let (imagodei_pk, imagodei_sk) = imagodei_keypair();
    let seal_keys = sealing_pub_keys(&mishpat_pk, &imagodei_pk);
    let unseal_keys = elohim_storage::services::back_prop::UnsealingKeys {
        mishpat_pk: &mishpat_pk,
        mishpat_sk: &mishpat_sk,
        imagodei_pk: &imagodei_pk,
        imagodei_sk: &imagodei_sk,
    };

    let sender_peer_id = "12D3KooWW2AContentSender0000000000000001";

    let envelope_bytes = build_content_envelope_bytes(b"w2a-content-test-payload");

    // Decode to get CID for assertion.
    let epr: Epr = ciborium::de::from_reader(envelope_bytes.as_slice()).expect("decode");
    let content_cid = epr.envelope.cid.to_string();

    apply_predecessor_wiring(&pool, &seal_keys, &envelope_bytes, sender_peer_id);

    let mut conn = pool.get().expect("conn");
    let predecessors = read_predecessors(&mut conn, &content_cid, &unseal_keys)
        .expect("read_predecessors must not fail");

    assert_eq!(
        predecessors,
        vec![sender_peer_id.to_string()],
        "Content EPR Announce must record the sender PeerId as a predecessor"
    );
}

/// FeedbackSignal EPR Announce → NO predecessor row recorded.
#[test]
fn announce_feedback_signal_epr_does_not_record_predecessor() {
    let pool = test_pool();
    let (mishpat_pk, mishpat_sk) = mishpat_keypair();
    let (imagodei_pk, imagodei_sk) = imagodei_keypair();
    let seal_keys = sealing_pub_keys(&mishpat_pk, &imagodei_pk);
    let unseal_keys = elohim_storage::services::back_prop::UnsealingKeys {
        mishpat_pk: &mishpat_pk,
        mishpat_sk: &mishpat_sk,
        imagodei_pk: &imagodei_pk,
        imagodei_sk: &imagodei_sk,
    };

    let sender_peer_id = "12D3KooWW2AFeedbackSender000000000000002";

    let envelope_bytes = build_feedback_signal_envelope_bytes(b"w2a-feedbacksignal-test-payload");

    let epr: Epr = ciborium::de::from_reader(envelope_bytes.as_slice()).expect("decode");
    let signal_cid = epr.envelope.cid.to_string();

    apply_predecessor_wiring(&pool, &seal_keys, &envelope_bytes, sender_peer_id);

    let mut conn = pool.get().expect("conn");
    let predecessors = read_predecessors(&mut conn, &signal_cid, &unseal_keys)
        .expect("read_predecessors must not fail");

    assert!(
        predecessors.is_empty(),
        "FeedbackSignal EPR Announce must NOT record a predecessor — back-prop graph is for content provenance only; got: {:?}",
        predecessors
    );
}
