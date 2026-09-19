//! Regression guard — habit `identity-cross-signed`.
//!
//! # What was broken
//!
//! An `AgentPeerBinding` asserts a `(agent_cid, transport_id)` pair — "agent X
//! owns transport endpoint Y". Today that assertion is **self-asserted**:
//!
//! - the gossip publisher writes `STAGE1_SIGNATURE_SENTINEL`, a placeholder
//!   (`reconcile/controller.rs`, `p2p/identity_binding_gossip.rs`);
//! - `imagodei_integrity`'s `validate_create` checks only that the signature
//!   field is **non-empty** — which a sentinel satisfies;
//! - the libp2p transport keypair and the Holochain agent key are generated
//!   independently, and nothing makes either sign over the other.
//!
//! So a gossiped spoof `(agent_cid = attacker, transport_id = victim)` passes
//! validation, and any economic attribution that joins through such a binding
//! credits the wrong peer. `elohim/elohim-storage/CLAUDE.md` states the rule
//! plainly: do NOT consume bindings for economic attribution until a cross-signed
//! control proof lands.
//!
//! # Why this file exists
//!
//! It was the STANDING RED for this habit: the signature ALGEBRA existed and was
//! sound — `p2p::binding_cross_signature` (slice C2-S1), pure and red-team
//! reviewed — but was `pub mod`-declared and referenced **nowhere else in
//! `src/`**. What was missing was the admissibility DECISION between a binding
//! and an attribution join; the only check was `!signature.is_empty()`.
//!
//! C2-S4 supplied it (`p2p::binding_proof_wire`), so these two assertions now
//! PASS and stand as the regression guard. They deliberately do not run
//! `#[ignore]`d any more: an ignored test is a CI no-op, and this property is
//! exactly the kind that decays silently.
//!
//! Both halves are asserted deliberately. A red that only refused sentinels could
//! be "cured" by refusing everything, which would be equally useless — the positive
//! case pins that real cross-signed bindings must be ADMITTED.
//!
//! Backlog: `genesis/data/timeline/backlog/agent-peer-binding-cross-signed-proof.md`
//! Spec: `genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md`
//!
//! Not yet in scope here (later C2 slices, deliberately NOT conflated with
//! admissibility): notarized-timestamp freshness (S3), lineage currency (S6).
//! Verifying two signatures is necessary, never sufficient.
//!
//! C2-S2 raised the fixture's own bar: the ids below are now DERIVED from the
//! signing keys (`transport_id` from the transport key, `agent_cid` from the
//! agent key), and a third assertion pins that the same fixture survives the
//! projection chokepoint — not just the pure algebra. A fixture naming
//! arbitrary ids can no longer stand in for a real binding.

use ed25519_dalek::{Signer, SigningKey};
use elohim_storage::p2p::binding_cross_signature::{
    canonical_bytes, BindingCore, CrossSignatureProof, AGENT_DOMAIN, SCHEME_VERSION,
    TRANSPORT_DOMAIN, TRANSPORT_KIND_LIBP2P,
};
use elohim_storage::p2p::binding_proof_wire::{
    agent_cid_from_agent_pubkey, classify_binding_signature, encode_proof,
    libp2p_peer_id_from_ed25519_pubkey,
};
use elohim_storage::p2p::identity_binding_gossip::STAGE1_SIGNATURE_SENTINEL;

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn sample_core() -> BindingCore {
    BindingCore {
        // C2-S2: both ids DERIVE from the keys that sign for them.
        agent_cid: agent_cid_from_agent_pubkey(&signing_key(2).verifying_key().to_bytes()),
        transport_id: libp2p_peer_id_from_ed25519_pubkey(
            &signing_key(1).verifying_key().to_bytes(),
        )
        .expect("fixture transport key derives a PeerId"),
        transport_kind: TRANSPORT_KIND_LIBP2P,
        valid_from: "2026-07-25T00:00:00Z".to_string(),
        // Bounded: the chokepoint refuses open-ended credentials, so a fixture
        // that means to be admissible end-to-end must expire.
        valid_until: Some("2026-08-14T00:00:00Z".to_string()),
        nonce: "dGVzdC1ub25jZQ==".to_string(),
        issued_at: "2026-07-25T00:00:00Z".to_string(),
    }
}

/// A genuinely cross-signed proof: both halves sign their own domain-separated
/// preimage, so neither identity is unilaterally claimable.
fn cross_signed_proof(core: &BindingCore) -> CrossSignatureProof {
    let transport_sk = signing_key(1);
    let agent_sk = signing_key(2);
    CrossSignatureProof {
        scheme_version: SCHEME_VERSION,
        transport_kind: core.transport_kind,
        transport_pubkey: transport_sk.verifying_key().to_bytes(),
        transport_signature: transport_sk
            .sign(&canonical_bytes(TRANSPORT_DOMAIN, core))
            .to_bytes(),
        agent_pubkey: agent_sk.verifying_key().to_bytes(),
        agent_signature: agent_sk
            .sign(&canonical_bytes(AGENT_DOMAIN, core))
            .to_bytes(),
        nonce: core.nonce.clone(),
        issued_at: core.issued_at.clone(),
    }
}

#[test]
fn a_sentinel_binding_is_inadmissible_for_economic_attribution() {
    // There is no admissibility predicate in the tree at all — that absence IS
    // the finding. The signature field a live binding carries today is this
    // sentinel, and the only gate on it is `!is_empty()`.
    let admissible = elohim_storage::p2p::binding_admissible_for_attribution(
        STAGE1_SIGNATURE_SENTINEL,
        /* dev_mode */ false,
    );
    assert!(
        !admissible,
        "a self-asserted sentinel binding must never back economic attribution — \
         a gossiped spoof (agent_cid = attacker, transport_id = victim) would \
         credit the attacker for the victim's work"
    );
}

#[test]
fn a_genuinely_cross_signed_binding_is_admissible() {
    // The half that keeps the red honest: a cure may not simply refuse
    // everything. Uses the landed C2-S1 algebra, so this can only pass against
    // real Ed25519 verification of both domain-separated halves.
    let core = sample_core();
    let proof = cross_signed_proof(&core);

    // Sanity: the algebra itself accepts this proof, so a failure below is about
    // the missing admissibility decision, not a malformed fixture.
    elohim_storage::p2p::binding_cross_signature::verify_binding_signatures(&core, &proof)
        .expect("fixture must be a valid cross-signature");

    let admissible = elohim_storage::p2p::binding_admissible_for_attribution_proof(
        &core, &proof, /* dev_mode */ false,
    );
    assert!(
        admissible,
        "a binding whose transport key and agent key each signed over the other \
         must be admitted — refusing everything is not a cure"
    );
}

/// C2-S2: admissibility in the pure algebra must survive the PROJECTION
/// chokepoint too — the layer that additionally requires each id to derive from
/// the key that signed for it. A proof that only the algebra accepts is not a
/// binding any writer may mark `cross_signed`.
#[test]
fn a_derived_cross_signed_binding_survives_the_projection_chokepoint() {
    let core = sample_core();
    let signature = encode_proof(&cross_signed_proof(&core));

    assert!(
        classify_binding_signature(
            &core.agent_cid,
            &core.transport_id,
            core.transport_kind,
            &core.valid_from,
            core.valid_until.as_deref(),
            &signature,
        )
        .is_cross_signed(),
        "the chokepoint must admit a proof whose ids derive from its keys — \
         otherwise no peer could ever mint one and the habit can never flip"
    );

    // ...and must refuse the SAME proof presented against an endpoint the
    // transport key does not derive. This is the hole C2-S1's module doc
    // assigned to S2: the algebra alone cannot tell these two apart.
    let foreign = libp2p_peer_id_from_ed25519_pubkey(&signing_key(9).verifying_key().to_bytes())
        .expect("derive");
    let mut lifted = core.clone();
    lifted.transport_id = foreign.clone();
    assert!(
        !classify_binding_signature(
            &core.agent_cid,
            &foreign,
            core.transport_kind,
            &core.valid_from,
            core.valid_until.as_deref(),
            &encode_proof(&cross_signed_proof(&lifted)),
        )
        .is_cross_signed(),
        "a key that is not this endpoint's key must not certify this endpoint"
    );
}
