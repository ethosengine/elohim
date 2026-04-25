//! Cross-peer integration tests for /elohim/epr-atom/1.0.0 and
//! /elohim/identity/handshake/1.0.0.
//!
//! Phase 2C Batch D: cross-peer behavioural proofs over the harness from
//! commit 88cc4fdf.
//! Phase 2B Task A.9: identity handshake test extends the harness with the
//! handshake protocol and asserts that after connect, peer B resolves peer A's
//! identity from its `peer_identity_bindings` table.

mod harness;
use elohim_storage::p2p::{verify_incoming_epr, CallerIdentity, MAX_BATCH_CIDS};
use harness::{spawn_test_node, BatchOutcome};
use std::time::Duration;
use tokio::time::timeout;

const DIAL_SETTLE: Duration = Duration::from_millis(100);
const CONNECT_WAIT: Duration = Duration::from_secs(10);
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::test]
async fn harness_two_nodes_connect() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;
    node_a.dial(node_b.addr()).await.expect("dial");
    node_a
        .wait_for_connection(&node_b.peer_id(), Duration::from_secs(5))
        .await;
}

// ---------------------------------------------------------------------------
// Task 15 — Round-trip integrity (P0)
//
// A authors a signed Commons atom; B fetches by CID via /elohim/epr-atom/1.0.0
// and re-verifies the envelope. Proves wire bytes are byte-preserving and the
// signature re-verifies on the far side.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn signed_atom_round_trips_and_verifies() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;

    node_a.dial(node_b.addr()).await.expect("dial");
    tokio::time::sleep(DIAL_SETTLE).await;
    node_a
        .wait_for_connection(&node_b.peer_id(), CONNECT_WAIT)
        .await;

    let epr = node_a.author_test_atom("commons", b"hello from A").await;
    let cid = epr.envelope.cid.to_string();
    node_a.ingest_local(epr.clone()).await;

    let response = timeout(
        FETCH_TIMEOUT,
        node_b.fetch_atom_from(&node_a.peer_id(), &cid),
    )
    .await
    .expect("fetch timed out")
    .unwrap_or_else(|e| panic!("fetch error: {e}"));

    let wire = response.expect("atom was None — B did not receive the atom");

    let (verified_epr, verified_cid) =
        verify_incoming_epr(&wire).expect("B should reverify the envelope");

    assert_eq!(verified_cid.to_string(), cid, "CID drifted across the wire",);
    assert_eq!(
        verified_epr.payload, epr.payload,
        "payload changed on the wire",
    );
    assert_eq!(
        verified_epr.envelope.proof.signer, epr.envelope.proof.signer,
        "signer changed on the wire",
    );
}

// ---------------------------------------------------------------------------
// Task 16 — Reach gate parity (P0)
//
// Private atoms must NEVER leak to anonymous peers. The response must be
// NotFound (not AccessDenied) to preserve the leak-free invariant.
//
// A second test documents Phase 2c's deliberate limitation: even when the
// serving peer knows the caller's agent pubkey, Private atoms are only
// released to callers whose identity matches the atom's author. Phase 2b
// will replace this with relationship/stewardship lookup.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn private_atom_not_served_to_anonymous_peer() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;

    node_a.dial(node_b.addr()).await.expect("dial");
    tokio::time::sleep(DIAL_SETTLE).await;
    node_a
        .wait_for_connection(&node_b.peer_id(), CONNECT_WAIT)
        .await;

    // A authors a Private atom. B never registers A's identity, so from A's
    // view B is Anonymous.
    let epr = node_a.author_test_atom("private", b"secret material").await;
    let cid = epr.envelope.cid.to_string();
    node_a.ingest_local(epr).await;

    let response = timeout(
        FETCH_TIMEOUT,
        node_b.fetch_atom_from(&node_a.peer_id(), &cid),
    )
    .await
    .expect("fetch timed out")
    .unwrap_or_else(|e| panic!("fetch error: {e}"));

    // Leak-free invariant: the response MUST be NotFound, not AccessDenied.
    // The harness maps NotFound to Ok(None).
    assert!(response.is_none(), "private atom leaked to anonymous peer",);
}

#[tokio::test]
async fn private_atom_not_cross_peer_servable_phase_2c() {
    // Documents Phase 2c's deliberate limitation: even with identity mapping,
    // Private atoms only release to author==caller. B is not the author, so
    // A's gate denies regardless of identity mapping. Phase 2b replaces this
    // with relationship/stewardship lookup.
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;

    // A registers B's agent identity so the lookup produces
    // CallerIdentity::Agent (not Anonymous). The gate still checks
    // caller == atom.author, which B is not.
    node_a
        .register_peer_identity(&node_b.peer_id(), &node_b.agent_pubkey())
        .await;

    node_a.dial(node_b.addr()).await.expect("dial");
    tokio::time::sleep(DIAL_SETTLE).await;
    node_a
        .wait_for_connection(&node_b.peer_id(), CONNECT_WAIT)
        .await;

    let epr = node_a.author_test_atom("private", b"steward-only").await;
    let cid = epr.envelope.cid.to_string();
    node_a.ingest_local(epr).await;

    let response = timeout(
        FETCH_TIMEOUT,
        node_b.fetch_atom_from(&node_a.peer_id(), &cid),
    )
    .await
    .expect("fetch timed out")
    .unwrap_or_else(|e| panic!("fetch error: {e}"));

    assert!(
        response.is_none(),
        "Phase 2c gate should deny private atom even to identity-mapped non-author peer",
    );
}

// ---------------------------------------------------------------------------
// Task 17 — Batch semantics (P1)
//
// FetchBatch must preserve slot order and must apply the same leak-free
// reach gate per-slot as the single Fetch path. A batch of
// [public, private_not_author, unknown] must return [Some, None, None].
//
// MAX_BATCH_CIDS enforces back-pressure: oversize requests are rejected as
// a protocol Error, not an AtomBatch of Nones.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_batch_preserves_slot_order_with_leak_free_denial() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;

    node_a.dial(node_b.addr()).await.expect("dial");
    tokio::time::sleep(DIAL_SETTLE).await;
    node_a
        .wait_for_connection(&node_b.peer_id(), CONNECT_WAIT)
        .await;

    // Slot 0: a Commons atom on A — should be served.
    let public_epr = node_a.author_test_atom("commons", b"public slot").await;
    let public_cid = public_epr.envelope.cid.to_string();
    node_a.ingest_local(public_epr).await;

    // Slot 1: a Private atom on A, caller (B) is not author — should gate to None.
    let private_epr = node_a.author_test_atom("private", b"private slot").await;
    let private_cid = private_epr.envelope.cid.to_string();
    node_a.ingest_local(private_epr).await;

    // Slot 2: a syntactically valid CID that was never ingested.
    let unknown_cid = elohim_epr::cid::compute_cid(b"never-ingested").to_string();

    let outcome = timeout(
        FETCH_TIMEOUT,
        node_b.fetch_batch_from(
            &node_a.peer_id(),
            vec![public_cid, private_cid, unknown_cid],
        ),
    )
    .await
    .expect("fetch_batch timed out")
    .unwrap_or_else(|e| panic!("fetch_batch error: {e}"));

    let atoms = match outcome {
        BatchOutcome::AtomBatch(atoms) => atoms,
        BatchOutcome::ProtocolError(msg) => {
            panic!("expected AtomBatch, got ProtocolError: {msg}")
        }
    };

    assert_eq!(atoms.len(), 3, "slot count must match request");
    assert!(atoms[0].is_some(), "slot 0 (public) must be served");
    assert!(
        atoms[1].is_none(),
        "slot 1 (private, non-author) must be None — leak-free denial",
    );
    assert!(atoms[2].is_none(), "slot 2 (unknown CID) must be None",);
}

#[tokio::test]
async fn fetch_batch_rejects_oversized_request() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;

    node_a.dial(node_b.addr()).await.expect("dial");
    tokio::time::sleep(DIAL_SETTLE).await;
    node_a
        .wait_for_connection(&node_b.peer_id(), CONNECT_WAIT)
        .await;

    // Construct MAX_BATCH_CIDS+1 CIDs. The length check runs before any parse,
    // so syntactically-valid stand-ins are sufficient.
    let stub_cid = elohim_epr::cid::compute_cid(b"oversize-stub").to_string();
    let cids: Vec<String> = (0..=MAX_BATCH_CIDS).map(|_| stub_cid.clone()).collect();

    let outcome = timeout(
        FETCH_TIMEOUT,
        node_b.fetch_batch_from(&node_a.peer_id(), cids),
    )
    .await
    .expect("fetch_batch timed out")
    .unwrap_or_else(|e| panic!("fetch_batch error: {e}"));

    match outcome {
        BatchOutcome::ProtocolError(msg) => {
            assert!(
                msg.contains("batch too large"),
                "error should describe oversize, got: {msg}",
            );
        }
        BatchOutcome::AtomBatch(_) => {
            panic!("oversize batch must be rejected as ProtocolError, not AtomBatch")
        }
    }
}

// ---------------------------------------------------------------------------
// Task 18 — Validation rejection (P1)
//
// Phase 2c's `verify_incoming_epr` is explicitly structural-only — its
// docstring calls out that full Ed25519 verification against a public key is
// deferred to a later layer (no signer-CID resolver at this point). The
// protection surface at Phase 2c is therefore:
//
//   1. CID recompute — detects tampering of any field in the canonical bytes
//      (payload and all envelope fields, including the signature).
//   2. Structural signature check — algorithm must be "ed25519", signature
//      must be 64 bytes.
//   3. Coupling validator.
//
// Three tests: payload tamper (CID mismatch path), bad signature length
// (structural path), and a byte-flipped 64-byte signature to pin the
// "signature is in canonical bytes" invariant. If that invariant ever
// changes (detached-signature refactor), Phase 2b's resolver-backed
// Ed25519 verify must land first or the third test must flip.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn announce_with_tampered_payload_is_rejected() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;

    node_a.dial(node_b.addr()).await.expect("dial");
    tokio::time::sleep(DIAL_SETTLE).await;
    node_a
        .wait_for_connection(&node_b.peer_id(), CONNECT_WAIT)
        .await;

    let mut epr = node_a.author_test_atom("commons", b"payload v1").await;
    let cid = epr.envelope.cid.to_string();

    assert!(!epr.payload.is_empty(), "test needs a non-empty payload");
    epr.payload[0] ^= 0x01;

    let wire = node_a.encode_envelope(&epr).await;

    let ack = timeout(FETCH_TIMEOUT, node_a.announce_to(&node_b.peer_id(), wire))
        .await
        .expect("announce timed out")
        .unwrap_or_else(|e| panic!("announce channel error: {e}"));

    assert!(
        !ack.accepted,
        "tampered payload must be rejected — CID recompute mismatch",
    );
    assert!(
        ack.reason.is_some(),
        "rejection must carry a reason for operator visibility",
    );

    let post_fetch = timeout(
        FETCH_TIMEOUT,
        node_a.fetch_atom_from(&node_b.peer_id(), &cid),
    )
    .await
    .expect("fetch timed out")
    .unwrap_or_else(|e| panic!("fetch error: {e}"));
    assert!(
        post_fetch.is_none(),
        "tampered atom must not be persisted in B's pool",
    );
}

#[tokio::test]
async fn announce_with_wrong_signature_length_is_rejected() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;

    node_a.dial(node_b.addr()).await.expect("dial");
    tokio::time::sleep(DIAL_SETTLE).await;
    node_a
        .wait_for_connection(&node_b.peer_id(), CONNECT_WAIT)
        .await;

    let mut epr = node_a.author_test_atom("commons", b"payload v1").await;
    let cid = epr.envelope.cid.to_string();

    epr.envelope.proof.signature.pop();
    assert_eq!(epr.envelope.proof.signature.len(), 63);

    let wire = node_a.encode_envelope(&epr).await;

    let ack = timeout(FETCH_TIMEOUT, node_a.announce_to(&node_b.peer_id(), wire))
        .await
        .expect("announce timed out")
        .unwrap_or_else(|e| panic!("announce channel error: {e}"));

    assert!(
        !ack.accepted,
        "63-byte signature must be rejected at verify_incoming_epr",
    );

    let post_fetch = timeout(
        FETCH_TIMEOUT,
        node_a.fetch_atom_from(&node_b.peer_id(), &cid),
    )
    .await
    .expect("fetch timed out")
    .unwrap_or_else(|e| panic!("fetch error: {e}"));
    assert!(
        post_fetch.is_none(),
        "structurally invalid atom must not be persisted",
    );
}

#[tokio::test]
async fn announce_with_tampered_signature_bytes_accepted_phase_2c_limitation() {
    // Documents Phase 2c's deliberate scoping. `verify_incoming_epr` is
    // structural-only AND the EPR canonicalisation is detached-signature —
    // the signature is not included in the canonical bytes the CID hashes
    // over. The result: byte flips inside a 64-byte ed25519 signature
    // (length preserved, algorithm preserved) pass every current check.
    //
    // The intent of Phase 2c is to ship federation with an honest scope —
    // structural integrity + coupling + reach gate — and land full
    // resolver-backed Ed25519 verification in Phase 2b+ where the
    // signer-CID resolver is available.
    //
    // When Phase 2b's resolver-backed verify lands, flip the final
    // assertion to `!ack.accepted` and rename this test to drop the
    // `_phase_2c_limitation` suffix.
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;

    node_a.dial(node_b.addr()).await.expect("dial");
    tokio::time::sleep(DIAL_SETTLE).await;
    node_a
        .wait_for_connection(&node_b.peer_id(), CONNECT_WAIT)
        .await;

    let mut epr = node_a.author_test_atom("commons", b"payload v1").await;
    epr.envelope.proof.signature[0] ^= 0x01;
    assert_eq!(epr.envelope.proof.signature.len(), 64);
    assert_eq!(epr.envelope.proof.algorithm, "ed25519");

    let wire = node_a.encode_envelope(&epr).await;

    let ack = timeout(FETCH_TIMEOUT, node_a.announce_to(&node_b.peer_id(), wire))
        .await
        .expect("announce timed out")
        .unwrap_or_else(|e| panic!("announce channel error: {e}"));

    assert!(
        ack.accepted,
        "Phase 2c detached-signature + structural-only verify accepts \
         byte-flipped sigs; flip to !accepted when Phase 2b Ed25519 verify \
         lands (see test docstring).",
    );
}

// ---------------------------------------------------------------------------
// Task A.9 — Identity handshake (P0)
//
// After two peers connect, the /elohim/identity/handshake/1.0.0 protocol
// fires automatically on ConnectionEstablished. Each peer sends the other
// its signed AgentPeerBinding. The receiver verifies structural integrity
// and validity window, then inserts into `peer_identity_bindings` with
// source='handshake'.
//
// This test asserts: after node_a dials node_b and they both complete the
// handshake, peer B's HolochainBackedPeerIdentityMap resolves peer A's
// PeerId to CallerIdentity::Agent(peer_a_agent_cid).
// ---------------------------------------------------------------------------

/// `HANDSHAKE_SETTLE` gives both peers time to exchange the handshake and
/// write the DB row before the assertion.
const HANDSHAKE_SETTLE: Duration = Duration::from_millis(500);

#[tokio::test]
async fn identity_handshake_populates_peer_identity_map_on_connect() {
    let node_a = spawn_test_node("a").await;
    let node_b = spawn_test_node("b").await;

    node_a.dial(node_b.addr()).await.expect("dial");
    tokio::time::sleep(DIAL_SETTLE).await;
    node_a
        .wait_for_connection(&node_b.peer_id(), CONNECT_WAIT)
        .await;

    // Wait for the handshake to complete and the DB rows to be written.
    tokio::time::sleep(HANDSHAKE_SETTLE).await;

    // Verify: node_b's identity map resolves node_a's PeerId → Agent(agent_cid_a).
    // `node_b.lookup_peer_identity(&node_a.peer_id())` reads from node_b's
    // peer_identity_bindings table via HolochainBackedPeerIdentityMap.
    let identity_a_from_b = node_b.lookup_peer_identity(&node_a.peer_id()).await;
    assert!(
        matches!(
            &identity_a_from_b,
            CallerIdentity::Agent(cid) if cid == &node_a.agent_cid()
        ),
        "peer B should resolve peer A's identity as Agent after handshake, got: {identity_a_from_b:?}",
    );

    // Symmetry: node_a's identity map resolves node_b's PeerId → Agent(agent_cid_b).
    let identity_b_from_a = node_a.lookup_peer_identity(&node_b.peer_id()).await;
    assert!(
        matches!(
            &identity_b_from_a,
            CallerIdentity::Agent(cid) if cid == &node_b.agent_cid()
        ),
        "peer A should resolve peer B's identity as Agent after handshake, got: {identity_b_from_a:?}",
    );
}
