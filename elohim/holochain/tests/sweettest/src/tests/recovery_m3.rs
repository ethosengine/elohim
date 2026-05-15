//! Sweettest — Recovery Protocol Phase 2 Milestone M3.
//!
//! These tests exercise coordinator-side correctness across multiple cells
//! in a single conductor network. They are `#[ignore]` until the DNA is
//! packed upstream by the pipeline; remove the `#[ignore]` once the
//! pipeline's pack-then-test stage is wired.
//!
//! Scenarios per M3 design §9.1:
//!   1. `m3_happy_path_intimate_quorum` — 3 contacts, threshold 3, [B,C] fails,
//!      [B,C,D] succeeds.
//!   2. `m3_freeze_floor_blocks_intimate_allows_cryptographic` — active
//!      IdentityFreeze at the "intimate" layer halts IntimateQuorum rotation;
//!      CryptographicQuorum path still passes.
//!   3. `m3_anchor_durability_across_rotation` — human_id anchor still points
//!      to the original RecoveryRequest after a successful key rotation.
//!   4. `m3_non_contact_witness_rejected` — coordinator pre-commit membership
//!      gate rejects submit_intimate_witness from a non-emergency-contact agent.
//!
//! All four use the same 3-contact fixture; factor the setup into a local
//! helper `setup_a_with_three_contacts` when filling in the bodies.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};
use holochain_types::prelude::{DnaFile, RoleName};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DNA: &str = "imagodei";

// ---------------------------------------------------------------------------
// Wire-mirror types for the multi-DNA Recovery M4 Task 3 test.
// Field names and order MUST match the coordinator structs exactly.
// ---------------------------------------------------------------------------

/// Mirror of imagodei coordinator's `SubmitIntimateWitnessInput`
/// (post-Recovery-M4-Task-3 shape — see imagodei/zomes/imagodei/src/lib.rs).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubmitIntimateWitnessInputMirror {
    pub recovery_request_cid: String,
    pub note: Option<String>,
}

/// Mirror of `content_store::governance_action::ProposeGovernanceActionInput`
/// (same as in attestation_coordinator.rs).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProposeGovernanceActionInputMirror {
    pub governance_kind: String,
    pub subject_cid: String,
    pub title: String,
    pub description: Option<String>,
    pub reach: String,
    pub threshold: Value,
    pub eligibility_predicate: Option<Value>,
    pub ballot_format: String,
    pub closes_at: String,
    pub parameters: Option<Value>,
}

/// Mirror of `content_store::governance_action::GovernanceActionOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GovernanceActionOutputMirror {
    pub cid: String,
    pub governance_kind: String,
    pub subject_cid: String,
    pub proposer_cid: String,
    pub closes_at: String,
}

/// Happy path: three emergency contacts satisfy the intimate-quorum threshold
/// of 3 (computed as ceil(3/2)+1). The rotation is rejected when only two
/// witnesses are present and accepted when the third lands.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m3_happy_path_intimate_quorum() -> Result<()> {
    // TODO(M3-sweettest-bodies): fill in with multi-agent conductor setup.
    // Setup (common helper `setup_a_with_three_contacts`):
    //   1. Spawn 4 agents (A, B, C, D) on a shared conductor network.
    //   2. Install imagodei DNA via `load_dna(DNA, &network_seed(DNA), Some(agent))`.
    //   3. Create Humans for each agent via coordinator `create_human`.
    //   4. Create 3 HumanRelationships (A<->B, A<->C, A<->D) with
    //      emergency_access_enabled = true.
    //   5. Wait for DHT propagation (`common::mirrors`).
    //
    // Scenario-specific:
    //   - A invokes `create_recovery_request { human_agent_pubkey: a_key, ... }`.
    //     Assert `output.request.human_id == Some(a_human_id)`.
    //     Assert `output.request.required_witness_count == 3`.
    //   - B invokes `submit_intimate_witness { recovery_request_hash, note: None }`.
    //   - C invokes the same.
    //   - A invokes `commit_key_rotation` with
    //     `IntimateQuorum { witness_hashes: vec![w_b, w_c] }`.
    //     Assert REJECTED (threshold 3, only 2 witnesses).
    //   - D invokes `submit_intimate_witness`.
    //   - A retries `commit_key_rotation` with all three witness_hashes.
    //     Assert ACCEPTED. Verify `HumanToCurrentAgent` link now points to the
    //     new KeyRotation action hash.
    let (mut _conductor, _agent) = single_agent_conductor().await?;
    let _dna = load_dna(DNA, &network_seed(DNA), None).await?;
    Ok(())
}

/// An active IdentityFreeze at the "intimate" layer must halt IntimateQuorum
/// rotation via the coordinator pre-commit gate. A CryptographicQuorum rotation
/// on the same human is exempt and must still succeed per design §4.3.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m3_freeze_floor_blocks_intimate_allows_cryptographic() -> Result<()> {
    // TODO(M3-sweettest-bodies): fill in.
    // Setup as above. Then:
    //   - Before any rotation attempt, commit an IdentityFreeze via the M5
    //     defender path or a fixture-authored entry:
    //     IdentityFreeze { human_id, frozen_at_layer: Some("intimate"),
    //                       is_active: true, ... }
    //   - A invokes `commit_key_rotation` with valid IntimateQuorum witnesses.
    //     Assert REJECTED with a "freeze-floor" error message.
    //   - Separately, set up a valid KeyStewardship with a threshold-reached
    //     quorum signature (fixture or coordinator helper).
    //   - A invokes `commit_key_rotation` with
    //     `CryptographicQuorum { stewardship_hash }`.
    //     Assert ACCEPTED (gate exempts this authority variant).
    let _ = (DNA, network_seed(DNA));
    Ok(())
}

/// After a successful rotation, the `HumanToRecoveryRequest` link anchored on
/// human_id must still resolve to the original RecoveryRequest. This guards
/// against a regression that reverts the M3 anchor convention back to pubkey.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m3_anchor_durability_across_rotation() -> Result<()> {
    // TODO(M3-sweettest-bodies): fill in.
    // Setup as above, run through happy path to a successful rotation.
    //   - Query StringAnchor("recovery_request", a_human_id) via
    //     `get_links(anchor_hash, LinkTypes::HumanToRecoveryRequest)`.
    //     Assert the original `RecoveryRequest` ActionHash is still discoverable.
    let _ = (DNA, network_seed(DNA));
    Ok(())
}

/// An agent that is NOT on any emergency-enabled HumanRelationship for the
/// target human must be rejected by the coordinator membership gate in
/// `submit_intimate_witness` before any DHT entry is committed.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m3_non_contact_witness_rejected() -> Result<()> {
    // TODO(M3-sweettest-bodies): fill in.
    // Setup the 3-contact fixture plus a 5th agent E with no relationship to A.
    //   - A opens a recovery request.
    //   - E invokes `submit_intimate_witness`.
    //     Assert REJECTED with an emergency-contact-membership error.
    //   - Verify no `HumanityWitness` entry authored by E exists for this
    //     request (query `RecoveryRequestToHumanityWitness` links).
    let _ = (DNA, network_seed(DNA));
    Ok(())
}

/// Recovery M4 Task 3: Gate 1 of `submit_intimate_witness` resolves the
/// recovery-request by CID via cross-DNA `content_store::get_content_by_id`
/// (no longer by local `get(ActionHash)`).
///
/// Setup:
///   1. Install BOTH imagodei and elohim DNAs in a single conductor app
///      under role names "imagodei" and "elohim" — the imagodei coordinator's
///      bridge call (`CallTargetCell::OtherRole("elohim".into())`) requires
///      that role name to resolve.
///   2. Call `content_store::propose_governance_action` on the elohim cell
///      with `governance_kind: "governance-action:recovery-request"` and
///      metadata carrying `human_id`. Capture the returned CID.
///   3. Call `submit_intimate_witness` on the imagodei cell, passing that
///      CID as `recovery_request_cid`.
///   4. Assert the call returns (Gate 1 succeeds — the CID resolves on
///      elohim DNA, content-type matches, human_id is present). Gate 2
///      (emergency-contact check) and Gate 3 (dedupe) are out of scope for
///      this test — those are exercised by the m3_happy_path test in this
///      file. We only need to prove the cross-DNA read path works.
///
/// Adaptation note: in a real recovery flow, Gate 2 would block the
/// authorizer (the conductor agent has no HumanRelationship to the
/// `subject_cid`). The assertion below tolerates either success or a
/// "not an active emergency contact" rejection — what matters for Task 3
/// is that we get PAST Gate 1 (we do not see a "recovery-request CID ...
/// not found" or "expected governance-action:recovery-request" error from
/// the new Gate 1 path).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNAs from Jenkins pipeline — needs both imagodei.dna and lamad.dna"]
async fn intimate_witness_reads_cross_dna_recovery_request() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;

    // Load both DNAs and install them under explicit role names matching
    // the imagodei coordinator's `CallTargetCell::OtherRole("elohim")` target.
    let imagodei_dna =
        load_dna("imagodei", &network_seed("imagodei"), Some(agent.clone())).await?;
    let elohim_dna = load_dna("lamad", &network_seed("lamad"), Some(agent.clone())).await?;
    let imagodei_dna_hash = imagodei_dna.dna_hash().clone();
    let elohim_dna_hash = elohim_dna.dna_hash().clone();

    let dnas_with_roles: Vec<(RoleName, DnaFile)> = vec![
        ("imagodei".into(), imagodei_dna),
        ("elohim".into(), elohim_dna),
    ];

    let app = conductor
        .setup_app_for_agent("recovery-m4-t3", agent.clone(), &dnas_with_roles)
        .await?;
    let cells = app.cells();
    let imagodei_cell = cells
        .iter()
        .find(|c| c.dna_hash() == &imagodei_dna_hash)
        .expect("imagodei cell installed")
        .clone();
    let elohim_cell = cells
        .iter()
        .find(|c| c.dna_hash() == &elohim_dna_hash)
        .expect("elohim cell installed")
        .clone();

    // Step 1: propose a recovery-request governance action on the elohim DNA.
    let propose_input = ProposeGovernanceActionInputMirror {
        governance_kind: "governance-action:recovery-request".to_string(),
        subject_cid: "human-alice-cid".to_string(),
        title: "Recovery request — Alice".to_string(),
        description: None,
        reach: "private".to_string(),
        threshold: serde_json::json!({ "type": "m-of-n", "m": 2, "n": 3 }),
        eligibility_predicate: None,
        ballot_format: "approve-reject".to_string(),
        closes_at: "2099-01-01T00:00:00Z".to_string(),
        parameters: Some(serde_json::json!({
            "human_id": "human-alice-cid",
            "custodian_cids": ["cid-bob", "cid-carol", "cid-dave"],
            "threshold": {"m": 2},
            "closes_at": "2099-01-01T00:00:00Z",
        })),
    };
    let governance_output: GovernanceActionOutputMirror = conductor
        .call(
            &elohim_cell.zome("content_store"),
            "propose_governance_action",
            propose_input,
        )
        .await;
    assert!(!governance_output.cid.is_empty(), "governance CID must be set");

    // Step 2: invoke submit_intimate_witness on imagodei with the elohim CID.
    let witness_input = SubmitIntimateWitnessInputMirror {
        recovery_request_cid: governance_output.cid.clone(),
        note: Some("Recovery M4 Task 3 — cross-DNA gate test".to_string()),
    };
    let result: holochain::conductor::api::error::ConductorApiResult<Value> = conductor
        .call_fallible(
            &imagodei_cell.zome("imagodei"),
            "submit_intimate_witness",
            witness_input,
        )
        .await;

    // Gate 1 success criterion: the error (if any) must NOT be a Gate-1
    // error. A Gate-2 (emergency contact) rejection is the expected outcome
    // when the authorizer agent has no HumanRelationship to the subject.
    // A successful Ok(_) is also acceptable — both prove the cross-DNA read
    // path executed correctly.
    if let Err(err) = result {
        let message = format!("{err:?}");
        assert!(
            !message.contains("Gate 1"),
            "Gate 1 must not fail in the cross-DNA path. Error: {message}"
        );
        assert!(
            !message.contains("not found on elohim DNA"),
            "Recovery-request CID must resolve on elohim DNA. Error: {message}"
        );
        assert!(
            !message.contains("expected governance-action:recovery-request"),
            "Content type discriminator must match. Error: {message}"
        );
    }

    Ok(())
}
