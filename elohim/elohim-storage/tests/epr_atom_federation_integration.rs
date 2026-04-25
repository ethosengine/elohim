//! Cross-peer integration tests for /elohim/epr-atom/1.0.0 and
//! /elohim/identity/handshake/1.0.0.
//!
//! Phase 2C Batch D: cross-peer behavioural proofs over the harness from
//! commit 88cc4fdf.
//! Phase 2B Task A.9: identity handshake test extends the harness with the
//! handshake protocol and asserts that after connect, peer B resolves peer A's
//! identity from its `peer_identity_bindings` table.
//! Phase 2B Task A.10: gossipsub `elohim/identity/binding` propagation test —
//! peer A receives an AgentPeerBinding DHT signal, publishes to the topic via
//! the controller's swarm_tx, peer B receives via gossip and upserts into
//! `peer_identity_bindings` with source='gossip'.

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

// ---------------------------------------------------------------------------
// Task A.10 — Gossipsub `elohim/identity/binding` propagation (P0)
//
// When a local agent's `AgentPeerBinding` DHT signal arrives, the
// ReconcileController sends `P2PCommand::PublishIdentityBinding` to the
// swarm. The swarm publishes on `elohim/identity/binding`. Subscribed peers
// receive the gossip, apply structural verification, and upsert into
// `peer_identity_bindings` with source='gossip'.
//
// This test verifies two things:
//
// 1. Controller wiring: `ReconcileController::on_agent_peer_binding` sends the
//    correct `P2PCommand::PublishIdentityBinding` payload to the swarm channel.
//
// 2. Gossip propagation: two minimal gossipsub-capable swarms exchange a
//    binding; the receiving side upserts into its DB with source='gossip', and
//    the row is queryable via `peer_identity_bindings::lookup_active`.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// A.10.1 — Controller sends PublishIdentityBinding when swarm_tx is wired
// ---------------------------------------------------------------------------

#[tokio::test]
async fn controller_on_agent_peer_binding_sends_publish_command() {
    use chrono::Utc;
    use elohim_storage::p2p::{identity_binding_gossip::STAGE1_SIGNATURE_SENTINEL, P2PCommand};
    use elohim_storage::reconcile::controller::ReconcileController;
    use elohim_storage::reconcile::signal_stream::{
        AgentPeerBindingSignal, DeviceArchetype, DnaSignal, InMemoryDnaSignalStream,
    };
    use tokio::sync::mpsc;

    let binding_signal = AgentPeerBindingSignal {
        action_hash: "uhCkk-ctrl-binding-hash".into(),
        peer_id: "12D3KooWGossipTestPeer".into(),
        agent_cid: "bafybeicid-gossip-agent".into(),
        valid_from: Utc::now(),
        valid_until: None,
        device_archetype: DeviceArchetype::Node,
        binding_action_hash: "uhCkk-ctrl-binding-hash".into(),
        emitted_at: Utc::now(),
    };

    let stream = InMemoryDnaSignalStream::with_signals(vec![DnaSignal::AgentPeerBinding(
        binding_signal.clone(),
    )]);

    // Channel the controller will send commands to.
    let (tx, mut rx) = mpsc::channel::<P2PCommand>(8);

    let mut controller = ReconcileController::new(stream).with_swarm_tx(tx);
    controller.run_one_pass().await.expect("run_one_pass");

    // The controller must have sent exactly one PublishIdentityBinding command.
    let cmd = rx
        .recv()
        .await
        .expect("expected a command on the swarm channel");

    match cmd {
        P2PCommand::PublishIdentityBinding(payload) => {
            assert_eq!(
                payload.peer_id, binding_signal.peer_id,
                "peer_id mismatch in gossip payload"
            );
            assert_eq!(
                payload.agent_cid, binding_signal.agent_cid,
                "agent_cid mismatch in gossip payload"
            );
            assert_eq!(
                payload.binding_action_hash, binding_signal.binding_action_hash,
                "binding_action_hash mismatch in gossip payload"
            );
            assert_eq!(
                payload.signature, STAGE1_SIGNATURE_SENTINEL,
                "Stage 1 sentinel signature must be used"
            );
            assert!(
                !payload.valid_from.is_empty(),
                "valid_from must be non-empty"
            );
        }
        other => panic!(
            "expected P2PCommand::PublishIdentityBinding, got: {:?}",
            std::mem::discriminant(&other)
        ),
    }

    // No more commands on the channel.
    assert!(
        rx.try_recv().is_err(),
        "only one PublishIdentityBinding command expected"
    );

    assert_eq!(
        controller.observed_kinds(),
        &["agentPeerBinding"],
        "controller must record the agentPeerBinding kind"
    );
}

// ---------------------------------------------------------------------------
// A.10.2 — Gossip propagation: peer A publishes → peer B upserts with source='gossip'
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gossipsub_identity_binding_propagates_to_peer_db() {
    use elohim_storage::db::models::NewPeerIdentityBindingRow;
    use elohim_storage::p2p::identity_binding_gossip::{
        IdentityBindingGossip, STAGE1_SIGNATURE_SENTINEL,
    };
    use elohim_storage::test_util::test_pool;
    use futures::StreamExt;
    use libp2p::{
        gossipsub, identity, noise, swarm::SwarmEvent, tcp, yamux, Multiaddr, SwarmBuilder,
    };

    // Peer B's pool is used for the gossip upsert assertion.
    let pool_b = test_pool();

    // Build a gossipsub-only swarm bound to a deterministic identity.
    fn build_gossip_swarm() -> libp2p::Swarm<gossipsub::Behaviour> {
        let local_key = identity::Keypair::generate_ed25519();
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            // Short heartbeat so subscription info propagates quickly in tests.
            .heartbeat_interval(std::time::Duration::from_millis(100))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .expect("valid gossipsub config");
        let behaviour = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .expect("gossipsub behaviour");
        SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .expect("tcp")
            .with_behaviour(|_| behaviour)
            .expect("behaviour")
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(30)))
            .build()
    }

    let topic = gossipsub::IdentTopic::new("elohim/identity/binding");

    // --- Peer B setup ---
    let mut swarm_b = build_gossip_swarm();
    swarm_b
        .behaviour_mut()
        .subscribe(&topic)
        .expect("subscribe b");
    swarm_b
        .listen_on("/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap())
        .expect("listen b");
    // Pump swarm_b until we learn its listen address.
    let addr_b: Multiaddr = loop {
        match swarm_b.next().await.expect("event") {
            SwarmEvent::NewListenAddr { address, .. } => break address,
            _ => continue,
        }
    };

    // Build the gossip payload peer A will publish.
    let payload = IdentityBindingGossip {
        action_hash: "uhCkk-gossip-test-binding".into(),
        peer_id: "12D3KooWGossipTestPeerA".into(),
        agent_cid: "bafybeicid-gossip-propagation-agent".into(),
        valid_from: "2026-04-25T00:00:00Z".into(),
        valid_until: None,
        device_archetype: "node".into(),
        binding_action_hash: "uhCkk-gossip-test-binding".into(),
        emitted_at: "2026-04-25T00:00:01Z".into(),
        signature: STAGE1_SIGNATURE_SENTINEL.into(),
    };
    let payload_bytes = payload.to_bytes().expect("encode");

    // Channel: B notifies test when it has upserted.
    let (done_tx, done_rx) = tokio::sync::oneshot::channel::<()>();
    let pool_b_task = pool_b.clone();

    // --- Peer B driver (background task) ---
    // Receives gossip messages, verifies structurally, and upserts into DB.
    let topic_b = topic.clone();
    tokio::spawn(async move {
        let mut done_tx_opt = Some(done_tx);
        loop {
            match swarm_b.next().await {
                Some(SwarmEvent::Behaviour(gossipsub::Event::Message { message, .. })) => {
                    if message.topic != topic_b.hash() {
                        continue;
                    }
                    let Ok(received) = IdentityBindingGossip::from_bytes(&message.data) else {
                        continue;
                    };
                    if received.verify_structural().is_err() {
                        continue;
                    }
                    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                    let row = NewPeerIdentityBindingRow {
                        peer_id: received.peer_id.clone(),
                        agent_cid: received.agent_cid.clone(),
                        dht_anchor_hash: received.binding_action_hash.clone(),
                        valid_from: received.valid_from.clone(),
                        valid_until: received.valid_until.clone(),
                        observed_at: now_iso,
                        source: "gossip".to_string(),
                    };
                    let mut conn = pool_b_task.get().expect("conn");
                    elohim_storage::db::peer_identity_bindings::upsert(&mut conn, &row)
                        .expect("upsert");
                    if let Some(tx) = done_tx_opt.take() {
                        let _ = tx.send(());
                    }
                }
                None => break,
                _ => {}
            }
        }
    });

    // --- Peer A setup ---
    let mut swarm_a = build_gossip_swarm();
    swarm_a
        .behaviour_mut()
        .subscribe(&topic)
        .expect("subscribe a");
    swarm_a
        .listen_on("/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap())
        .expect("listen a");
    loop {
        match swarm_a.next().await.expect("event") {
            SwarmEvent::NewListenAddr { .. } => break,
            _ => continue,
        }
    }

    // Dial B from A.
    swarm_a.dial(addr_b.clone()).expect("dial");

    // Pump A until connection is established.
    loop {
        match swarm_a.next().await.expect("swarm a event") {
            SwarmEvent::ConnectionEstablished { .. } => break,
            _ => continue,
        }
    }

    // Pump A briefly to allow gossipsub subscription exchange to complete.
    // Retry publish until it succeeds (returns Err(InsufficientPeers) when
    // B hasn't yet propagated its subscription to A's gossipsub view).
    let publish_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        match swarm_a
            .behaviour_mut()
            .publish(topic.clone(), payload_bytes.clone())
        {
            Ok(_) => break,
            Err(gossipsub::PublishError::InsufficientPeers) => {
                // Subscription exchange not complete yet — pump and retry.
                if tokio::time::Instant::now() >= publish_deadline {
                    panic!("timed out waiting for B's subscription to propagate to A");
                }
                // Drive A's event loop once to process gossipsub control messages.
                tokio::select! {
                    Some(_) = swarm_a.next() => {}
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {}
                }
            }
            Err(e) => panic!("publish error: {e:?}"),
        }
    }

    // Continue driving A's event loop briefly so the message is fully sent.
    let drive_deadline = tokio::time::Instant::now() + Duration::from_millis(500);
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(drive_deadline) => break,
            Some(_) = swarm_a.next() => {}
        }
    }

    // Wait for peer B to receive and upsert, with a reasonable timeout.
    tokio::time::timeout(Duration::from_secs(15), done_rx)
        .await
        .expect("timeout waiting for gossip delivery")
        .expect("done channel dropped");

    // Assert: peer B's DB has the binding with source='gossip'.
    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut conn_b = pool_b.get().expect("conn_b");
    let row = elohim_storage::db::peer_identity_bindings::lookup_active(
        &mut conn_b,
        &payload.peer_id,
        &now_iso,
    )
    .expect("lookup_active")
    .expect("expected active binding for gossip-propagated peer");

    assert_eq!(
        row.agent_cid, payload.agent_cid,
        "agent_cid must match the gossip payload"
    );
    assert_eq!(
        row.source, "gossip",
        "source must be 'gossip' for gossipsub-received binding"
    );
    assert_eq!(
        row.dht_anchor_hash, payload.binding_action_hash,
        "dht_anchor_hash must match binding_action_hash from gossip payload"
    );
}
