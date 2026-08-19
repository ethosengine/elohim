//! The attribution cut, end to end — habit `identity-cross-signed` (C2-S4/S5).
//!
//! `binding_attribution_refuses_sentinel.rs` pins the *decision* in isolation.
//! This file pins the path a real row travels: a binding is written through the
//! projection writer, classified by the chokepoint, and then asked for by an
//! economic join. The properties under test:
//!
//! 1. A self-asserted binding (the `STAGE1_SIGNATURE_SENTINEL` every live row
//!    carries today) is classified `unverified` in the database.
//! 2. A genuinely cross-signed binding is classified `cross_signed` — the cure
//!    cannot be "refuse everything".
//! 3. Under `Enforce`, only the cross-signed row survives the attribution cut,
//!    and the refusal is COUNTED rather than silent.
//! 4. Under `Observe` (the default posture, so this is what a fleet does on the
//!    next deploy) behaviour is unchanged — both rows are served — and the
//!    self-asserted one is still counted. Deploying the cut must not blank a
//!    single economic surface before any peer can mint a proof.
//! 5. A poisoned signature field cannot panic the read path or sneak through.
//!
//! Deliberately NOT asserted here: that a `cross_signed` row is trustworthy to a
//! third party. Tier-1 verification is receiver-local until the integrity-zome
//! fold (C2-S7), and no test in this crate can claim otherwise.

use diesel::prelude::*;
use ed25519_dalek::{Signer, SigningKey};
use elohim_storage::db::models::NewPeerIdentityBindingRow;
use elohim_storage::db::peer_identity_bindings::{
    list_active_for_agent, list_attributable_for_agent_with_posture, AttributionPosture,
};
use elohim_storage::db::{run_migrations, DbPool};
use elohim_storage::p2p::binding_cross_signature::{
    canonical_bytes, BindingCore, CrossSignatureProof, AGENT_DOMAIN, SCHEME_VERSION,
    TRANSPORT_DOMAIN, TRANSPORT_KIND_LIBP2P,
};
use elohim_storage::p2p::binding_proof_wire::{
    agent_cid_from_agent_pubkey, classify_binding_signature, encode_proof,
    libp2p_peer_id_from_ed25519_pubkey, BindingProofStatus,
};
use elohim_storage::p2p::identity_binding_gossip::{
    binding_row_from_gossip, IdentityBindingGossip, STAGE1_SIGNATURE_SENTINEL,
};

// C2-S2 raised the bar on these fixtures: an id is only usable in a proof if it
// DERIVES from the key that signs for it. `AGENT` is the `uhCAk…` HoloHash the
// agent key names; `HONEST_PEER` is the libp2p PeerId the transport key derives.
// `SPOOF_PEER` deliberately derives from NEITHER — it is the attacker's claim.
fn transport_sk() -> SigningKey {
    SigningKey::from_bytes(&[3u8; 32])
}
fn agent_sk() -> SigningKey {
    SigningKey::from_bytes(&[4u8; 32])
}
fn agent() -> String {
    agent_cid_from_agent_pubkey(&agent_sk().verifying_key().to_bytes())
}
fn honest_peer() -> String {
    libp2p_peer_id_from_ed25519_pubkey(&transport_sk().verifying_key().to_bytes())
        .expect("fixture transport key derives a PeerId")
}
fn spoof_peer() -> String {
    libp2p_peer_id_from_ed25519_pubkey(
        &SigningKey::from_bytes(&[5u8; 32])
            .verifying_key()
            .to_bytes(),
    )
    .expect("derive")
}

const FROM: &str = "2026-08-01T00:00:00Z";
const UNTIL: &str = "2026-08-31T00:00:00Z";
const NOW: &str = "2026-08-10T00:00:00Z";

fn test_pool() -> DbPool {
    use diesel::r2d2::{ConnectionManager, Pool};
    let url = format!(
        "file:attr_cut_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4().as_simple()
    );
    let pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<SqliteConnection>::new(&url))
        .expect("pool");
    run_migrations(&pool).expect("migrations");
    pool
}

/// A genuine cross-signature for `(agent(), peer_id)`: the transport key attests
/// the agent, the agent key attests the transport, each over its own
/// domain-separated preimage.
fn cross_signed_signature(peer_id: &str) -> String {
    let core = BindingCore {
        agent_cid: agent(),
        transport_id: peer_id.to_string(),
        transport_kind: TRANSPORT_KIND_LIBP2P,
        valid_from: FROM.to_string(),
        valid_until: Some(UNTIL.to_string()),
        nonce: "YXR0cmlidXRpb24tY3V0".to_string(),
        issued_at: FROM.to_string(),
    };
    let transport_sk = SigningKey::from_bytes(&[3u8; 32]);
    let agent_sk = SigningKey::from_bytes(&[4u8; 32]);
    encode_proof(&CrossSignatureProof {
        scheme_version: SCHEME_VERSION,
        transport_kind: TRANSPORT_KIND_LIBP2P,
        transport_pubkey: transport_sk.verifying_key().to_bytes(),
        transport_signature: transport_sk
            .sign(&canonical_bytes(TRANSPORT_DOMAIN, &core))
            .to_bytes(),
        agent_pubkey: agent_sk.verifying_key().to_bytes(),
        agent_signature: agent_sk
            .sign(&canonical_bytes(AGENT_DOMAIN, &core))
            .to_bytes(),
        nonce: core.nonce.clone(),
        issued_at: core.issued_at.clone(),
    })
}

/// Project a binding the way the gossip receive arm does — through
/// `binding_row_from_gossip`, so the classification under test is the one
/// production uses, not a re-derivation.
fn project_gossiped_binding(pool: &DbPool, peer_id: &str, signature: &str) {
    let payload = IdentityBindingGossip {
        peer_id: peer_id.to_string(),
        agent_cid: agent(),
        valid_from: FROM.to_string(),
        valid_until: Some(UNTIL.to_string()),
        device_archetype: "node".to_string(),
        binding_action_hash: format!("uhCkk-anchor-{peer_id}"),
        emitted_at: FROM.to_string(),
        signature: signature.to_string(),
    };
    let row: NewPeerIdentityBindingRow = binding_row_from_gossip(&payload, NOW);
    let mut conn = pool.get().expect("conn");
    elohim_storage::db::peer_identity_bindings::upsert(&mut conn, &row).expect("upsert");
}

#[test]
fn a_sentinel_binding_projects_as_unverified() {
    let pool = test_pool();
    project_gossiped_binding(&pool, &spoof_peer(), STAGE1_SIGNATURE_SENTINEL);

    let mut conn = pool.get().expect("conn");
    let rows = list_active_for_agent(&mut conn, &agent(), NOW).expect("list");
    assert_eq!(rows.len(), 1);
    assert!(
        !rows[0].is_cross_signed(),
        "the signature field every live binding carries today is a placeholder — \
         it must never project as verified"
    );
    assert_eq!(
        rows[0].signature, STAGE1_SIGNATURE_SENTINEL,
        "the signature must be RETAINED, not dropped at projection time — \
         otherwise the classification can never be re-derived on rebuild"
    );
}

/// The migration's `DEFAULT 'unverified' NOT NULL` must carry every row that
/// predates these columns — and every future writer that forgets them. Written
/// as a raw INSERT naming only the pre-migration columns, which is exactly the
/// shape of an already-populated fleet database.
#[test]
fn a_row_written_without_the_proof_columns_defaults_to_unverified() {
    let pool = test_pool();
    let mut conn = pool.get().expect("conn");
    diesel::sql_query(format!(
        "INSERT INTO peer_identity_bindings \
         (peer_id, agent_cid, dht_anchor_hash, valid_from, valid_until, observed_at, source, \
          device_archetype, superseded_by) \
         VALUES ('12D3KooWLegacyRow', '{}', 'uhCkk-legacy', \
          '2026-08-01T00:00:00Z', NULL, '2026-08-10T00:00:00Z', 'dht', 'node', NULL)",
        agent()
    ))
    .execute(&mut conn)
    .expect("legacy-shaped insert must still be accepted");

    let rows = list_active_for_agent(&mut conn, &agent(), NOW).expect("list");
    assert_eq!(rows.len(), 1);
    assert!(
        !rows[0].is_cross_signed(),
        "a pre-existing row carries no proof and must read as self-asserted"
    );
    assert_eq!(rows[0].signature, "");

    let cut = list_attributable_for_agent_with_posture(
        &mut conn,
        &agent(),
        NOW,
        AttributionPosture::Enforce,
    )
    .expect("cut");
    assert!(cut.is_empty(), "and it must not be attributable");
}

#[test]
fn a_cross_signed_binding_projects_as_cross_signed() {
    let pool = test_pool();
    project_gossiped_binding(
        &pool,
        &honest_peer(),
        &cross_signed_signature(&honest_peer()),
    );

    let mut conn = pool.get().expect("conn");
    let rows = list_active_for_agent(&mut conn, &agent(), NOW).expect("list");
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0].is_cross_signed(),
        "a real bidirectional cross-signature must be admitted — refusing \
         everything is not a cure"
    );
}

#[test]
fn enforce_admits_only_the_cross_signed_binding_and_counts_the_refusal() {
    let pool = test_pool();
    project_gossiped_binding(
        &pool,
        &honest_peer(),
        &cross_signed_signature(&honest_peer()),
    );
    project_gossiped_binding(&pool, &spoof_peer(), STAGE1_SIGNATURE_SENTINEL);

    let mut conn = pool.get().expect("conn");
    let cut = list_attributable_for_agent_with_posture(
        &mut conn,
        &agent(),
        NOW,
        AttributionPosture::Enforce,
    )
    .expect("cut");

    assert_eq!(
        cut.peer_ids(),
        vec![honest_peer().as_str()],
        "an economic join may credit only the peer whose binding was proven; \
         the spoof claiming this agent's identity is refused"
    );
    assert_eq!(
        cut.unverified_seen(),
        1,
        "the refusal must be COUNTED — a cut that drops rows silently cannot be \
         operated, and this count is the habit's flip-to-green measure"
    );
}

/// The genesis seeder writes `signature: [0]` — a single zero byte, chosen to
/// satisfy the integrity validator's non-empty rule at Stage 1
/// (`genesis/seeder/src/seed-agent-bindings.ts`). Every fixture binding on every
/// seeded environment has this exact shape, so pin it directly rather than
/// trusting the general hostile-input case: it must classify unverified,
/// without panicking, and without disturbing the fixture's projection.
#[test]
fn the_genesis_seeder_fixture_signature_classifies_unverified() {
    let seeder_signature = String::from_utf8_lossy(&[0u8]).into_owned();
    assert_eq!(
        classify_binding_signature(
            &agent(),
            &spoof_peer(),
            TRANSPORT_KIND_LIBP2P,
            FROM,
            Some(UNTIL),
            &seeder_signature,
        ),
        BindingProofStatus::unverified()
    );

    let pool = test_pool();
    project_gossiped_binding(&pool, &spoof_peer(), &seeder_signature);
    let mut conn = pool.get().expect("conn");
    assert_eq!(
        list_active_for_agent(&mut conn, &agent(), NOW)
            .expect("list")
            .len(),
        1,
        "the fixture row still projects — the cut classifies bindings, it never drops them"
    );
}

#[test]
fn observe_is_behaviour_preserving_and_still_counts() {
    let pool = test_pool();
    project_gossiped_binding(
        &pool,
        &honest_peer(),
        &cross_signed_signature(&honest_peer()),
    );
    project_gossiped_binding(&pool, &spoof_peer(), STAGE1_SIGNATURE_SENTINEL);

    let mut conn = pool.get().expect("conn");
    let routing = list_active_for_agent(&mut conn, &agent(), NOW).expect("list");
    let cut = list_attributable_for_agent_with_posture(
        &mut conn,
        &agent(),
        NOW,
        AttributionPosture::Observe,
    )
    .expect("cut");

    assert_eq!(
        cut.rows().len(),
        routing.len(),
        "the DEFAULT posture must serve exactly what the pre-cut code served — \
         deploying this slice may not blank an economic surface before any peer \
         can mint a proof"
    );
    assert_eq!(
        cut.unverified_seen(),
        1,
        "observe still measures what it let through — that is the whole point of \
         the posture"
    );
}

/// One poisoned row must not panic the read path, and must not be attributable.
/// (The `EprRouter`-poisoned-row class: a single hostile DHT entry emptying a
/// whole projection is a real, shipped failure shape in this codebase.)
#[test]
fn a_poisoned_signature_neither_panics_nor_attributes() {
    let pool = test_pool();
    project_gossiped_binding(
        &pool,
        &honest_peer(),
        &cross_signed_signature(&honest_peer()),
    );
    for (i, poison) in [
        "elohim:apb:1:!!!!",
        "elohim:apb:1:AAAAAAAA",
        "elohim:apb:99:AAAA",
        "\u{0}\u{1}\u{2}",
    ]
    .iter()
    .enumerate()
    {
        project_gossiped_binding(&pool, &format!("12D3KooWPoison{i}"), poison);
    }

    let mut conn = pool.get().expect("conn");
    let cut = list_attributable_for_agent_with_posture(
        &mut conn,
        &agent(),
        NOW,
        AttributionPosture::Enforce,
    )
    .expect("poisoned rows must not error the read");

    assert_eq!(
        cut.peer_ids(),
        vec![honest_peer().as_str()],
        "hostile signature bodies are classified unverified and cut, while the \
         legitimate row is unaffected — one bad row does not empty the set"
    );
    assert_eq!(cut.unverified_seen(), 4);
}

/// A proof is bound to the pair it was minted for: lifting a valid proof from
/// one binding onto another agent/transport pair does not verify.
#[test]
fn a_lifted_proof_does_not_attach_to_another_binding() {
    let lifted = cross_signed_signature(&honest_peer());
    let status = classify_binding_signature(
        &agent(),
        &spoof_peer(), // same agent, different transport id
        TRANSPORT_KIND_LIBP2P,
        FROM,
        Some(UNTIL),
        &lifted,
    );
    assert_eq!(
        status,
        BindingProofStatus::unverified(),
        "a proof minted for one transport endpoint must not certify another"
    );

    let pool = test_pool();
    project_gossiped_binding(&pool, &spoof_peer(), &lifted);
    let mut conn = pool.get().expect("conn");
    let cut = list_attributable_for_agent_with_posture(
        &mut conn,
        &agent(),
        NOW,
        AttributionPosture::Enforce,
    )
    .expect("cut");
    assert!(
        cut.is_empty(),
        "the lift must be refused at the projection, not only in the pure algebra"
    );
}
