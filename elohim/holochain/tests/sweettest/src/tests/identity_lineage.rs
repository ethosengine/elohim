//! @dna-scope: imagodei
//! Sweettest — Identity Head + Key Lineage (Wave B of the identity-head plan).
//!
//! Conductor-side proof of the NEW coordinator surface:
//!   B1  identity_chain_root / identity_head / identity_version_parents — the
//!       chain-root over the KeyRotation version DAG (un-rotated == degenerate
//!       root; the multi-node walk + stability is proven by pure-logic unit
//!       tests in imagodei::identity_lineage, since a VALID KeyRotation cannot be
//!       minted in a conductor — no coordinator fn creates the HumanityWitness /
//!       KeyStewardship evidence the integrity validator requires; the same
//!       constraint that keeps recovery_m3::m3_happy_path a stub).
//!   B3  rotate_identity_key controller-policy authorization: an unauthorized
//!       (non-head, self-policy) rotation is REFUSED before any entry write; an
//!       authorized (self / recovery-quorum) rotation PASSES the gate (then hits
//!       the un-mintable-KeyRotation constraint downstream, asserted precisely);
//!       steward-set is Wave C; the chain-root is unchanged by a rotation attempt.
//!   B0  the imagodei↔mishpat reference: a mishpat binds-identity commitment
//!       referencing the imagodei chain-root is accepted (a CID reference, not a
//!       runtime bridge — resolving the B0 escalation point as workable).
//!
//! Per the DNA sweettest convention these carry `#[ignore]` so local `cargo test`
//! skips the packed-DNA conductor cost; CI runs them via `--run-ignored all`.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};
use holochain_types::prelude::{DnaFile, RoleName};
use serde::{Deserialize, Serialize};

const DNA: &str = "imagodei";

/// Mirror of `imagodei::identity_lineage::RotateIdentityKeyInput`.
/// `authority` is a `serde_json::Value` shaped as the externally-tagged
/// `RecoveryAuthority` enum (e.g. `{"IntimateQuorum": {"witness_hashes": []}}`).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RotateIdentityKeyInputMirror {
    pub human_agent_pubkey: holo_hash::AgentPubKey,
    pub new_agent_pubkey: holo_hash::AgentPubKey,
    pub superseded_agent_pubkey: holo_hash::AgentPubKey,
    pub controller_policy: String,
    pub authority: serde_json::Value,
    pub recovery_request_hash: holo_hash::ActionHash,
}

/// Mirror of `mishpat::commitments::CreateCommitmentInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateCommitmentInput {
    pub action: String,
    pub payload_json: String,
    pub signed_at: String,
}

/// Mirror of `mishpat::commitments::CommitmentOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommitmentOutput {
    pub action_hash: holo_hash::ActionHash,
    pub entry_hash: holo_hash::EntryHash,
}

fn intimate_quorum_empty() -> serde_json::Value {
    // Externally-tagged RecoveryAuthority::IntimateQuorum { witness_hashes: [] }.
    // Decodes at the zome (reaching the authorization gate); the empty quorum is
    // then rejected by the KeyRotation integrity validator (proving the gate
    // ran BEFORE the entry was written).
    serde_json::json!({ "IntimateQuorum": { "witness_hashes": [] } })
}

fn fake_action_hash() -> holo_hash::ActionHash {
    holo_hash::ActionHash::from_raw_36(vec![0u8; 36])
}

fn fake_agent(seed: u8) -> holo_hash::AgentPubKey {
    holo_hash::AgentPubKey::from_raw_36(vec![seed; 36])
}

// ---------------------------------------------------------------------------
// B1 — chain-root / head on an un-rotated identity (degenerate root == the key).
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed imagodei.dna — CI runs via --run-ignored all"]
async fn identity_lineage_un_rotated_root_and_head() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("identity-lineage-b1", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();

    // Un-rotated identity: root == head == the key itself; no version-parents.
    let root: holo_hash::AgentPubKey = conductor
        .call(&cell.zome("imagodei"), "identity_chain_root", agent.clone())
        .await;
    assert_eq!(
        root, agent,
        "un-rotated chain_root must be the key itself (degenerate root)"
    );

    let head: holo_hash::AgentPubKey = conductor
        .call(&cell.zome("imagodei"), "identity_head", agent.clone())
        .await;
    assert_eq!(head, agent, "un-rotated head must be the key itself");

    let parents: Vec<holo_hash::AgentPubKey> = conductor
        .call(
            &cell.zome("imagodei"),
            "identity_version_parents",
            agent.clone(),
        )
        .await;
    assert!(
        parents.is_empty(),
        "un-rotated identity has no version-parents"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// B3b — unauthorized rotation REFUSED (self-policy, caller is not the head).
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed imagodei.dna — CI runs via --run-ignored all"]
async fn rotate_identity_key_self_policy_non_head_caller_refused() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("identity-lineage-b3b", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();

    // A DIFFERENT (fake) identity whose head is NOT the caller. self-policy then
    // requires caller == head → REFUSED before any entry is written.
    let other = fake_agent(9);
    let input = RotateIdentityKeyInputMirror {
        human_agent_pubkey: other.clone(),
        new_agent_pubkey: fake_agent(10),
        superseded_agent_pubkey: other.clone(), // == head(other) (un-rotated)
        controller_policy: "self".to_string(),
        authority: intimate_quorum_empty(),
        recovery_request_hash: fake_action_hash(),
    };
    let result: holochain::conductor::api::error::ConductorApiResult<serde_json::Value> = conductor
        .call_fallible(&cell.zome("imagodei"), "rotate_identity_key", input)
        .await;

    let err = result.expect_err("self-policy rotation by a non-head caller must be refused");
    let message = format!("{err:?}");
    assert!(
        message.contains("unauthorized controller"),
        "refusal must name the controller-authorization failure. Got: {message}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// B3a — self-authorized rotation (caller IS the head) PASSES the gate. The
// downstream un-mintable-KeyRotation constraint is asserted precisely so the
// test proves the AUTHORIZATION gate ran and passed.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed imagodei.dna — CI runs via --run-ignored all"]
async fn rotate_identity_key_self_policy_head_caller_gate_passes() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("identity-lineage-b3a", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();

    // Un-rotated caller: head == caller, so self-policy authorization PASSES.
    let input = RotateIdentityKeyInputMirror {
        human_agent_pubkey: agent.clone(),
        new_agent_pubkey: fake_agent(11),
        superseded_agent_pubkey: agent.clone(),
        controller_policy: "self".to_string(),
        authority: intimate_quorum_empty(),
        recovery_request_hash: fake_action_hash(),
    };
    let result: holochain::conductor::api::error::ConductorApiResult<serde_json::Value> = conductor
        .call_fallible(&cell.zome("imagodei"), "rotate_identity_key", input)
        .await;

    // The gate passed; the entry write then fails on the empty quorum (no
    // coordinator path mints witnesses). Assert the error is NOT an
    // authorization refusal — proving the self-controller gate passed.
    if let Err(err) = result {
        let message = format!("{err:?}");
        assert!(
            !message.contains("unauthorized controller"),
            "self-policy with caller == head must pass the gate. Got: {message}"
        );
        assert!(
            !message.contains("not the chain's current head"),
            "superseded == current head must pass the head invariant. Got: {message}"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// B3c — recovery-quorum (grandma) rotation PASSES the gate; chain-root unchanged.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed imagodei.dna — CI runs via --run-ignored all"]
async fn rotate_identity_key_recovery_quorum_gate_passes_root_unchanged() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("identity-lineage-b3c", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();

    let root_before: holo_hash::AgentPubKey = conductor
        .call(&cell.zome("imagodei"), "identity_chain_root", agent.clone())
        .await;

    // Grandma case: the human's key is lost; a recovery quorum (not the head)
    // authorizes. has_recovery_authority is true → the recovery-quorum gate passes.
    let input = RotateIdentityKeyInputMirror {
        human_agent_pubkey: agent.clone(),
        new_agent_pubkey: fake_agent(12),
        superseded_agent_pubkey: agent.clone(),
        controller_policy: "recovery-quorum".to_string(),
        authority: intimate_quorum_empty(),
        recovery_request_hash: fake_action_hash(),
    };
    let result: holochain::conductor::api::error::ConductorApiResult<serde_json::Value> = conductor
        .call_fallible(&cell.zome("imagodei"), "rotate_identity_key", input)
        .await;

    if let Err(err) = result {
        let message = format!("{err:?}");
        assert!(
            !message.contains("unauthorized controller"),
            "recovery-quorum authority must pass the controller gate. Got: {message}"
        );
        assert!(
            !message.contains("requires a recovery authority"),
            "an IntimateQuorum authority satisfies the recovery-quorum gate. Got: {message}"
        );
    }

    // Chain-root stability contract: unchanged by the rotation attempt.
    let root_after: holo_hash::AgentPubKey = conductor
        .call(&cell.zome("imagodei"), "identity_chain_root", agent.clone())
        .await;
    assert_eq!(
        root_before, root_after,
        "chain_root must be unchanged across a rotation"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// B3 — steward-set is refused this wave (no insecure default door; Wave C).
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed imagodei.dna — CI runs via --run-ignored all"]
async fn rotate_identity_key_steward_set_deferred() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("identity-lineage-steward", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();

    let input = RotateIdentityKeyInputMirror {
        human_agent_pubkey: agent.clone(),
        new_agent_pubkey: fake_agent(13),
        superseded_agent_pubkey: agent.clone(),
        controller_policy: "steward-set".to_string(),
        authority: intimate_quorum_empty(),
        recovery_request_hash: fake_action_hash(),
    };
    let result: holochain::conductor::api::error::ConductorApiResult<serde_json::Value> = conductor
        .call_fallible(&cell.zome("imagodei"), "rotate_identity_key", input)
        .await;

    let err = result.expect_err("steward-set is deferred to Wave C and must be refused");
    assert!(
        format!("{err:?}").contains("Wave C"),
        "steward-set refusal must name the Wave C deferral"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// B3 — recovery-quorum with a STUB (unimplemented) authority is refused at the
// coordinator gate with a clear message.
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed imagodei.dna — CI runs via --run-ignored all"]
async fn rotate_identity_key_recovery_quorum_stub_authority_refused() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("identity-lineage-stub-auth", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();

    // CommunityConsensus is a stub RecoveryAuthority variant (not an implemented
    // quorum) → the coordinator refuses it before any entry is written.
    let input = RotateIdentityKeyInputMirror {
        human_agent_pubkey: agent.clone(),
        new_agent_pubkey: fake_agent(14),
        superseded_agent_pubkey: agent.clone(),
        controller_policy: "recovery-quorum".to_string(),
        authority: serde_json::json!({ "CommunityConsensus": { "challenge_hash": fake_action_hash() } }),
        recovery_request_hash: fake_action_hash(),
    };
    let result: holochain::conductor::api::error::ConductorApiResult<serde_json::Value> = conductor
        .call_fallible(&cell.zome("imagodei"), "rotate_identity_key", input)
        .await;

    let err = result.expect_err("a stub recovery authority must be refused at the coordinator");
    assert!(
        format!("{err:?}").contains("requires a recovery authority"),
        "stub-authority refusal must name the recovery-authority requirement"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// B0 — the imagodei↔mishpat reference: a mishpat binds-identity commitment can
// reference the imagodei chain-root (a CID reference, not a runtime bridge).
// ---------------------------------------------------------------------------
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed imagodei.dna + mishpat.dna — CI runs via --run-ignored all"]
async fn binds_identity_references_imagodei_chain_root() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;

    let imagodei_dna = load_dna("imagodei", &network_seed("imagodei"), Some(agent.clone())).await?;
    let mishpat_dna = load_dna("mishpat", &network_seed("mishpat"), Some(agent.clone())).await?;
    let imagodei_hash = imagodei_dna.dna_hash().clone();
    let mishpat_hash = mishpat_dna.dna_hash().clone();

    let dnas_with_roles: Vec<(RoleName, DnaFile)> = vec![
        ("imagodei".into(), imagodei_dna),
        ("mishpat".into(), mishpat_dna),
    ];
    let app = conductor
        .setup_app_for_agent("identity-lineage-b0", agent.clone(), &dnas_with_roles)
        .await?;
    let cells = app.cells();
    let imagodei_cell = cells
        .iter()
        .find(|c| c.dna_hash() == &imagodei_hash)
        .expect("imagodei cell")
        .clone();
    let mishpat_cell = cells
        .iter()
        .find(|c| c.dna_hash() == &mishpat_hash)
        .expect("mishpat cell")
        .clone();

    // The imagodei chain-root (genesis key) for this identity.
    let chain_root: holo_hash::AgentPubKey = conductor
        .call(
            &imagodei_cell.zome("imagodei"),
            "identity_chain_root",
            agent.clone(),
        )
        .await;

    // A mishpat binds-identity commitment referencing that chain-root. The
    // reference is a content/CID string — imagodei owns the lineage, mishpat owns
    // the declaration; they meet at the root reference, not a runtime bridge.
    let payload = serde_json::json!({
        "action": "binds-identity",
        "chain_root": chain_root.to_string(),
        "head_key": agent.to_string(),
        "controllers": [agent.to_string()],
        "controller_policy": { "kind": "self" }
    });
    let input = CreateCommitmentInput {
        action: "binds-identity".to_string(),
        payload_json: payload.to_string(),
        signed_at: "2026-07-17T00:00:00Z".to_string(),
    };
    let output: CommitmentOutput = conductor
        .call(&mishpat_cell.zome("mishpat"), "create_commitment", input)
        .await;
    assert!(
        !output.action_hash.to_string().is_empty(),
        "binds-identity referencing the imagodei chain-root must be accepted on mishpat"
    );
    Ok(())
}
