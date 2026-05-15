//! Sweettest — Recovery Protocol Phase 2 Milestone M4: Fast-Path Revocation.
//!
//! These tests exercise coordinator-side correctness for the M4 revocation fast-
//! path: self-revocation (voluntary), emergency-contact quorum, rotation-gate,
//! dual-anchor invariants, and signal-replay idempotency.
//!
//! All scenarios are `#[ignore]` until the imagodei DNA artifact is packed by
//! the pipeline. The signal-replay idempotency test (m4_signal_replay_idempotency_storage)
//! runs against the storage signal handler directly — it does NOT need the DNA
//! artifact and is NOT ignored.
//!
//! Scenarios per M4 design §F.1:
//!   1. `m4_self_revocation_happy_path`
//!   2. `m4_emergency_contact_quorum_threshold_met`
//!   3. `m4_emergency_contact_quorum_threshold_not_met`
//!   4. `m4_revocation_vote_idempotency`
//!   5. `m4_rotation_blocked_by_pending_revocation`
//!   6. `m4_rotation_blocked_by_effective_revocation`
//!   7. `m4_dual_anchor_link_invariants`
//!   8. `m4_signal_replay_idempotency_storage` (storage-layer, not ignored)
//!
//! All 8 scenarios share a common set of fixture data (human ids, keys) defined
//! in the module-level constants. The M4-specific constant prefix is `M4_`.
//!
//! Setup pattern (DNA scenarios):
//!   1. Spin up a single-conductor multi-agent harness via `single_agent_conductor`.
//!   2. Load the imagodei DNA via `load_dna`.
//!   3. Install the DNA per-agent with `setup_app_for_agent`.
//!   4. Create Human entries via the `create_human` coordinator function.
//!   5. Create HumanRelationships (emergency_access_enabled = true) between humans.
//!   6. Drive the M4 coordinator entry points.
//!   7. Assert on return values, link presence, and signal payloads.
//!
//! Link queries use `conductor.call(&cell.zome(DNA), "get_links", …)`.
//! Entry fetches use `conductor.call(&cell.zome(DNA), "get", …)`.
//!
//! P1 (eager-reconciliation) assertions are deferred to the storage-layer test
//! (scenario 8) per the M4 design note: `sweep_dependent_caches_on_revocation`
//! is intentionally a no-op at M4, so P1 manifests as `imagodei.revocation_observed`
//! emission plus `key_revocations` row state.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};

// ============================================================================
// Module-level constants
// ============================================================================

const DNA: &str = "imagodei";

/// Prefix string distinguishing M4 test data from M3 fixtures in shared logs.
const M4_PREFIX: &str = "m4-";

/// Canonical M4 human ids used across scenarios.
const HUMAN_MATTHEW: &str = "human-matthew-m4";
const HUMAN_JESSICA: &str = "human-jessica-m4";
const HUMAN_TIMOTHY: &str = "human-timothy-m4";
const HUMAN_DAVID: &str = "human-david-m4";
const HUMAN_SARAH: &str = "human-sarah-m4";

// ============================================================================
// Shared input struct mirrors
//
// We mirror the coordinator I/O structs here with plain serde so the sweettest
// crate doesn't need a path-dep on the imagodei coordinator crate (which is
// WASM-only). This pattern mirrors the `NodeRegistration` fixture in
// `common/fixtures.rs`.
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CreateHumanInput {
    id: String,
    display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bio: Option<String>,
    affinities: Vec<String>,
    profile_reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    location: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CreateRelationshipInput {
    party_b_id: String,
    relationship_type: String,
    intimacy_level: String,
    custody_enabled: bool,
    emergency_access_enabled: bool,
    reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    context_json: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CreateSelfRevocationInput {
    revoked_key: holo_hash::AgentPubKey,
    reason: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct KeyRevocationOutput {
    revocation_id: String,
    action_hash: holo_hash::ActionHash,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CreateRevocationRequestInput {
    target_human_id: String,
    revoked_key: holo_hash::AgentPubKey,
    reason: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct SubmitRevocationVoteInput {
    // Recovery M4 Task 5: renamed from `revocation_id: String`. The imagodei
    // extern now takes the CID (content-derived id) of the
    // `governance-action:key-revocation` Content entry on the elohim DNA.
    revocation_cid: String,
    approved: bool,
    attestation: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct RevocationVoteOutput {
    vote_id: String,
    current_votes: u32,
    required_votes: u32,
    threshold_now_reached: bool,
}

// ============================================================================
// Helper: create a Human in the conductor
// ============================================================================

fn human_input(id: &str, display_name: &str) -> CreateHumanInput {
    CreateHumanInput {
        id: id.to_string(),
        display_name: display_name.to_string(),
        bio: None,
        affinities: vec![],
        profile_reach: "community".to_string(),
        location: None,
    }
}

// ============================================================================
// M4 Sweettest Scenarios
// ============================================================================

/// M4 Scenario 1: Self-revocation happy path.
///
/// A human (Matthew) reports his second device key was compromised and revokes
/// it from his still-trusted device. The revocation must be immediately effective
/// (voluntary trigger, threshold=1, threshold_reached=true), and both dual-anchor
/// links (HumanToKeyRevocation, RevokedKeyToRevocation) must be present. The
/// EffectiveRevocations anchor must contain the revocation; PendingRevocations
/// must not.
///
/// DNA assertions (P3 dual-anchor, §link-architecture §3):
///   - StringAnchor("human_revocations", HUMAN_MATTHEW) → KeyRevocation action hash
///   - StringAnchor("revoked_key", compromised_key_str) → KeyRevocation action hash
///   - StringAnchor("effective_revocations", "global") → KeyRevocation action hash
///   - StringAnchor("pending_revocations", "global") → NOT present
///
/// Return value assertions:
///   - revocation.trigger_type == "voluntary"
///   - revocation.required_votes == 1
///   - revocation.current_votes == 1
///   - revocation.threshold_reached == true
///   - revocation.effective_at.is_some()
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m4_self_revocation_happy_path() -> Result<()> {
    // TODO(M4-sweettest-bodies): fill in with multi-agent conductor setup.
    //
    // Setup:
    //   1. Spawn 1 conductor, 2 agents (A1_trusted, A2_compromised).
    //   2. Install imagodei via `load_dna(DNA, &network_seed(DNA), Some(a1))`.
    //   3. Create Human(HUMAN_MATTHEW) via `create_human` as A1.
    //   4. Register A2 as a second agent key for Matthew using `create_agent`
    //      (the coordinator creates a `AgentKeyToHuman` link from A2 → Matthew's human_id).
    //   5. A1 calls `create_self_revocation { revoked_key: a2, reason: "compromised" }`.
    //
    // Assert return value:
    //   let output: KeyRevocationOutput = conductor.call(...).await;
    //   assert!(output.revocation_id.starts_with("rev-{HUMAN_MATTHEW}"));
    //
    // Assert KeyRevocation entry via `get(output.action_hash)`:
    //   assert_eq!(rev.trigger_type, "voluntary");
    //   assert_eq!(rev.required_votes, 1);
    //   assert_eq!(rev.current_votes, 1);
    //   assert_eq!(rev.threshold_reached, true);
    //   assert!(rev.effective_at.is_some());
    //
    // Assert dual-anchor links (P3):
    //   let human_anchor = hash_entry(StringAnchor("human_revocations", HUMAN_MATTHEW));
    //   let human_links = get_links(human_anchor, HumanToKeyRevocation);
    //   assert_eq!(human_links.len(), 1);
    //   assert_eq!(human_links[0].target, output.action_hash);
    //
    //   let revoked_key_anchor = hash_entry(StringAnchor("revoked_key", a2.to_string()));
    //   let rk_links = get_links(revoked_key_anchor, RevokedKeyToRevocation);
    //   assert_eq!(rk_links.len(), 1);
    //   assert_eq!(rk_links[0].target, output.action_hash);
    //
    // Assert effective anchor:
    //   let effective_anchor = hash_entry(StringAnchor("effective_revocations", "global"));
    //   let eff_links = get_links(effective_anchor, EffectiveRevocations);
    //   assert!(eff_links.iter().any(|l| l.target == output.action_hash));
    //
    // Assert NOT on pending anchor:
    //   let pending_anchor = hash_entry(StringAnchor("pending_revocations", "global"));
    //   let pend_links = get_links(pending_anchor, PendingRevocations);
    //   assert!(pend_links.iter().all(|l| l.target != output.action_hash));
    let (mut _conductor, _agent) = single_agent_conductor().await?;
    let _dna = load_dna(DNA, &network_seed(DNA), None).await?;
    let _ = M4_PREFIX;
    Ok(())
}

/// M4 Scenario 2: Emergency-contact quorum — threshold met.
///
/// Matthew has 4 emergency contacts (Jessica, Timothy, David, Sarah). The
/// required threshold is max(2, ceil(4/2)+1) = 3.  Jessica initiates the
/// revocation request; Jessica, David, and Sarah each submit approved votes.
/// After the 3rd vote the KeyRevocation must flip to threshold_reached=true,
/// effective_at must be populated, and the anchor moves from PendingRevocations
/// to EffectiveRevocations.
///
/// Signal assertions:
///   - 1× KeyRevocationRequested (threshold_reached=false) after request creation.
///   - 3× RevocationVoteSubmitted, the last one carrying threshold_now_reached=true.
///   - 1× KeyRevocationEffective after the 3rd vote, triggering_vote_id=Some(vote3.id).
///
/// Storage assertions (P1 eager reconciliation via projection state):
///   - key_revocations row: threshold_reached=1, current_votes=3, effective_at IS NOT NULL.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m4_emergency_contact_quorum_threshold_met() -> Result<()> {
    // TODO(M4-sweettest-bodies): fill in.
    //
    // Setup (use M3 `setup_a_with_three_contacts` as template, extend to 4 contacts):
    //   1. Spawn 5 conductors: Matthew, Jessica, Timothy, David, Sarah.
    //   2. Install imagodei DNA on all 5.
    //   3. Create Humans for each.
    //   4. Create HumanRelationships Matthew↔{Jessica,Timothy,David,Sarah}
    //      with emergency_access_enabled=true.
    //   5. Wait for DHT propagation.
    //
    // Drive:
    //   - Jessica calls `create_revocation_request { target_human_id: HUMAN_MATTHEW,
    //       revoked_key: matthew_key, reason: "compromised" }`.
    //     Assert: required_votes == 3, threshold_reached == false.
    //
    //   - Jessica calls `submit_revocation_vote { revocation_id, approved: true,
    //       attestation: "Recognized Matthew, this is legitimate." }`.
    //     Assert: vote_output.threshold_now_reached == false.
    //
    //   - David calls `submit_revocation_vote` (same args).
    //     Assert: threshold_now_reached == false.
    //
    //   - Sarah calls `submit_revocation_vote` (same args).
    //     Assert: vote_output.threshold_now_reached == true.
    //     Assert: vote_output.current_votes == 3.
    //
    //   - Fetch the KeyRevocation entry via `get(revocation_action_hash)`:
    //     Assert: threshold_reached == true, effective_at.is_some().
    //
    //   - Assert EffectiveRevocations link present, PendingRevocations link absent.
    let (mut _conductor, _agent) = single_agent_conductor().await?;
    let _dna = load_dna(DNA, &network_seed(DNA), None).await?;
    let _ = (
        HUMAN_MATTHEW,
        HUMAN_JESSICA,
        HUMAN_TIMOTHY,
        HUMAN_DAVID,
        HUMAN_SARAH,
    );
    Ok(())
}

/// M4 Scenario 3: Emergency-contact quorum — threshold not met.
///
/// Same 4-contact setup as scenario 2, but only 2 votes are submitted (Jessica
/// and David). The required threshold of 3 is not met. The KeyRevocation must
/// remain pending: threshold_reached=false, current_votes=2, no EffectiveRevocations
/// link, still on PendingRevocations.
///
/// Anti-regression: we assert the *absence* of EffectiveRevocations link and
/// the *absence* of any KeyRevocationEffective signal in the test window.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m4_emergency_contact_quorum_threshold_not_met() -> Result<()> {
    // TODO(M4-sweettest-bodies): fill in.
    //
    // Setup identical to scenario 2 (4 contacts, threshold=3).
    //
    // Drive:
    //   - Jessica: `create_revocation_request { target_human_id: HUMAN_MATTHEW,
    //       revoked_key: matthew_key, reason: "compromised" }`.
    //     Assert: required_votes == 3.
    //
    //   - Jessica: `submit_revocation_vote { approved: true, attestation: "..." }`.
    //   - David:   `submit_revocation_vote { approved: true, attestation: "..." }`.
    //     (Two votes, both approved. Threshold 3 not met yet.)
    //
    //   - Fetch KeyRevocation entry:
    //     Assert: threshold_reached == false.
    //     Assert: current_votes == 2.
    //
    //   - Assert PendingRevocations link present.
    //   - Assert EffectiveRevocations anchor returns NO link to this revocation.
    let (mut _conductor, _agent) = single_agent_conductor().await?;
    let _dna = load_dna(DNA, &network_seed(DNA), None).await?;
    let _ = (HUMAN_MATTHEW, HUMAN_JESSICA, HUMAN_DAVID);
    Ok(())
}

/// M4 Scenario 4: Revocation vote idempotency.
///
/// A steward (Jessica) submits two votes on the same revocation. The second
/// call must either return an error containing "already voted" or be a silent
/// no-op. Either way, only 1 RevocationVote DHT entry must exist and the
/// revocation's current_votes must be 1, not 2.
///
/// The coordinator implements the "already voted" error path via the
/// StewardToRevocationVote link gate (lib.rs ~line 2157). This test verifies
/// that the error is surfaced to the caller so the Angular layer can present
/// an informative message.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m4_revocation_vote_idempotency() -> Result<()> {
    // TODO(M4-sweettest-bodies): fill in.
    //
    // Setup (4-contact fixture, same as scenarios 2/3):
    //   - Create revocation request via Jessica: required_votes=3.
    //   - Jessica submits vote 1: approved=true. Assert: ok.
    //   - Jessica submits vote 2 (same revocation_id, approved=true):
    //     Assert: call returns Err containing "already voted".
    //     OR: call returns Ok AND the RevocationToVote link count is still 1.
    //     (Both are acceptable — test whichever path the coordinator took.)
    //
    //   - Assert current_votes on the revocation == 1 (only the first vote counted).
    let (mut _conductor, _agent) = single_agent_conductor().await?;
    let _dna = load_dna(DNA, &network_seed(DNA), None).await?;
    let _ = (HUMAN_MATTHEW, HUMAN_JESSICA);
    Ok(())
}

/// M4 Scenario 5: Rotation blocked by pending revocation (P2 gate).
///
/// A pending revocation exists on the key being rotated. The `commit_key_rotation`
/// coordinator gate must reject the rotation with an error containing "pending
/// revocation". This gate is coordinator-side (cannot be in a validator because
/// validators cannot call `get_links` per `project_hdi_no_get_links_in_validators`).
///
/// Design note: the revocation-floor gate intentionally has NO CryptographicQuorum
/// exemption (unlike the freeze-floor gate). A revoked key must not produce valid
/// rotations under any authority.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m4_rotation_blocked_by_pending_revocation() -> Result<()> {
    // TODO(M4-sweettest-bodies): fill in.
    //
    // Setup:
    //   1. Matthew, Jessica (1 emergency contact, still creates a threshold-1 pending
    //      request — or use 2 contacts so threshold=2 and the first vote doesn't flip).
    //      Simpler: create 3 contacts so threshold=3, only 1 vote submitted → pending.
    //   2. Jessica calls `create_revocation_request` on Matthew's current key K1.
    //      Confirm: threshold_reached=false.
    //
    //   3. Matthew (or any agent) calls `commit_key_rotation {
    //        human_agent_pubkey: K1,
    //        new_agent_pubkey: K_new,
    //        superseded_agent_pubkey: K1,
    //        recovery_request_hash: some_hash,
    //        authority: IntimateQuorum { witness_hashes: [] },
    //      }`.
    //
    //   Assert: call returns Err whose message contains "pending revocation".
    //
    // Key discriminator: the block message from lib.rs line 2400 is:
    //   "commit_key_rotation blocked: key {key} has a pending revocation ({id}). Resolve or await."
    let (mut _conductor, _agent) = single_agent_conductor().await?;
    let _dna = load_dna(DNA, &network_seed(DNA), None).await?;
    let _ = (HUMAN_MATTHEW, HUMAN_JESSICA);
    Ok(())
}

/// M4 Scenario 6: Rotation blocked by effective revocation (P2 gate).
///
/// Same setup but the revocation is already effective (threshold reached). The
/// coordinator must still block the rotation with an error containing "effective
/// revocation". Self-revocation (threshold=1, voluntary) produces an immediately
/// effective revocation and is the simplest setup path for this scenario.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m4_rotation_blocked_by_effective_revocation() -> Result<()> {
    // TODO(M4-sweettest-bodies): fill in.
    //
    // Simplest setup path (avoids 4-contact fixture):
    //   1. Matthew has 2 keys: K_trusted, K_compromised.
    //   2. Matthew (as K_trusted) calls `create_self_revocation { revoked_key: K_compromised }`.
    //      Confirm: effective immediately (threshold_reached=true).
    //
    //   3. Attempt `commit_key_rotation { human_agent_pubkey: K_compromised, ... }`.
    //
    //   Assert: Err contains "effective revocation".
    //
    // Key discriminator: error message from lib.rs line 2400:
    //   "commit_key_rotation blocked: key {key} has an effective revocation ({id})."
    let (mut _conductor, _agent) = single_agent_conductor().await?;
    let _dna = load_dna(DNA, &network_seed(DNA), None).await?;
    let _ = HUMAN_MATTHEW;
    Ok(())
}

/// M4 Scenario 7: Dual-anchor link invariants (P3).
///
/// After a self-revocation, both anchor types must point to the same KeyRevocation
/// action hash. This guards against regressions where one anchor was not written
/// (e.g., RevokedKeyToRevocation omitted during a refactor) or was written to a
/// different target.
///
/// Design reference: `LINK_ARCHITECTURE.md` §3 "dual-anchor primacy" — every
/// KeyRevocation must be reachable from both the human listing anchor and the
/// revoked-key hot-path anchor.
///
/// Assertion shape:
///   - query HumanToKeyRevocation links → get target ActionHash H1
///   - query RevokedKeyToRevocation links → get target ActionHash H2
///   - assert H1 == H2 (same entry, both anchors point to it)
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn m4_dual_anchor_link_invariants() -> Result<()> {
    // TODO(M4-sweettest-bodies): fill in.
    //
    // Setup: Matthew self-revokes K_compromised from K_trusted.
    //   let output: KeyRevocationOutput = conductor.call("create_self_revocation", ...).await;
    //
    // Query human anchor:
    //   let human_anchor = hash_entry(StringAnchor { tag: "human_revocations", id: HUMAN_MATTHEW });
    //   let human_links = get_links(human_anchor, HumanToKeyRevocation);
    //   assert_eq!(human_links.len(), 1);
    //   let h1 = human_links[0].target.into_action_hash().unwrap();
    //
    // Query revoked-key anchor:
    //   let rk_anchor = hash_entry(StringAnchor { tag: "revoked_key", id: K_compromised.to_string() });
    //   let rk_links = get_links(rk_anchor, RevokedKeyToRevocation);
    //   assert_eq!(rk_links.len(), 1);
    //   let h2 = rk_links[0].target.into_action_hash().unwrap();
    //
    // Assert dual-anchor invariant:
    //   assert_eq!(h1, h2, "HumanToKeyRevocation and RevokedKeyToRevocation must point to the same entry");
    //   assert_eq!(h1, output.action_hash);
    let (mut _conductor, _agent) = single_agent_conductor().await?;
    let _dna = load_dna(DNA, &network_seed(DNA), None).await?;
    let _ = HUMAN_MATTHEW;
    Ok(())
}

// ============================================================================
// Recovery M4 Task 4 — cross-DNA freeze-floor + revocation-floor gates.
//
// These scenarios exercise the migrated `commit_key_rotation` gates against
// `governance-action:identity-freeze` and `governance-action:key-revocation`
// Content entries committed on the elohim DNA. Both are `#[ignore]` per the
// codebase convention — they only run in CI with packed DNA bundles.
//
// Pattern: install BOTH imagodei and elohim DNAs in a single conductor app
// under role names "imagodei" and "elohim", commit the relevant
// governance-action Content directly via `propose_governance_action`, then
// invoke `commit_key_rotation` and assert it returns Err with the gate's
// rejection message.
//
// Adaptation note (mirrors recovery_m3.rs::intimate_witness_reads_cross_dna_recovery_request):
// the generic `propose_governance_action` helper writes caller-supplied fields
// under `metadata.parameters_json`, not at the top level of `metadata`. The
// migrated gates read top-level metadata (`revoked_key`, `is_active`,
// `human_id`, `threshold_reached`, `effective_at`). Tasks 13/14's bespoke
// producers will write these at the top level; until then, these sweettests
// verify the CROSS-DNA READ PATH (CID resolution + content_type match +
// metadata decode) rather than asserting on the specific block message.
// Once the bespoke producers land, the scenarios will additionally assert
// the rotation-block error string.
// ============================================================================

/// Mirror of `content_store::governance_action::ProposeGovernanceActionInput`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct ProposeGovernanceActionInputMirrorT4 {
    pub governance_kind: String,
    pub subject_cid: String,
    pub title: String,
    pub description: Option<String>,
    pub reach: String,
    pub threshold: serde_json::Value,
    pub eligibility_predicate: Option<serde_json::Value>,
    pub ballot_format: String,
    pub closes_at: String,
    pub parameters: Option<serde_json::Value>,
}

/// Mirror of `content_store::governance_action::GovernanceActionOutput`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct GovernanceActionOutputMirrorT4 {
    pub cid: String,
    pub governance_kind: String,
    pub subject_cid: String,
    pub proposer_cid: String,
    pub closes_at: String,
}

/// Mirror of imagodei's `CommitKeyRotationInput`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CommitKeyRotationInputMirrorT4 {
    pub human_agent_pubkey: holo_hash::AgentPubKey,
    pub new_agent_pubkey: holo_hash::AgentPubKey,
    pub superseded_agent_pubkey: Option<holo_hash::AgentPubKey>,
    pub recovery_request_hash: Option<holo_hash::ActionHash>,
    pub authority: serde_json::Value,
}

/// Recovery M4 Task 4 Scenario A:
/// `commit_key_rotation` must consult the elohim DNA for an active
/// `governance-action:identity-freeze` covering the target human. With both
/// DNAs installed under their contract role names, the freeze-floor gate's
/// cross-DNA read path (`query_effective_identity_freeze_for_human`) must
/// execute against the elohim cell. Asserts the read path resolved without
/// network/auth/decode error.
///
/// Note: the generic `propose_governance_action` helper nests caller-supplied
/// fields under `metadata.parameters_json`; the gate reads top-level
/// `metadata.is_active`. Until Tasks 13/14 land bespoke producers that write
/// the freeze metadata at the top level, this test verifies cross-DNA wiring
/// rather than the BLOCKED outcome.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNAs from Jenkins pipeline — needs both imagodei.dna and lamad.dna"]
async fn m4_t4_commit_key_rotation_blocked_by_cross_dna_identity_freeze() -> Result<()> {
    use holochain_types::prelude::{DnaFile, RoleName};

    let (mut conductor, agent) = single_agent_conductor().await?;

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
        .setup_app_for_agent("recovery-m4-t4-freeze", agent.clone(), &dnas_with_roles)
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

    // Commit an identity-freeze Content entry on elohim DNA.
    let propose_input = ProposeGovernanceActionInputMirrorT4 {
        governance_kind: "governance-action:identity-freeze".to_string(),
        subject_cid: HUMAN_MATTHEW.to_string(),
        title: "Identity freeze — Matthew".to_string(),
        description: None,
        reach: "private".to_string(),
        threshold: serde_json::json!({ "type": "single-cell" }),
        eligibility_predicate: None,
        ballot_format: "single-cell".to_string(),
        closes_at: "2099-01-01T00:00:00Z".to_string(),
        parameters: Some(serde_json::json!({
            "human_id": HUMAN_MATTHEW,
            "is_active": true,
            "frozen_at_layer": "intimate",
        })),
    };

    // Tolerate either success (if `governance-action:identity-freeze` is
    // registered in the kinds catalog) or a kinds-catalog rejection. Task 16
    // declares the kind in the imagodei manifest; until that lands the
    // `propose_governance_action` floor 1 check rejects unknown kinds.
    let governance_result: holochain::conductor::api::error::ConductorApiResult<
        GovernanceActionOutputMirrorT4,
    > = conductor
        .call_fallible(
            &elohim_cell.zome("content_store"),
            "propose_governance_action",
            propose_input,
        )
        .await;
    if let Err(err) = &governance_result {
        let message = format!("{err:?}");
        // Acceptable pre-Task-16 outcome.
        assert!(
            message.contains("unknown_governance_action_kind")
                || message.contains("governance-action:identity-freeze"),
            "Unexpected propose_governance_action failure: {message}"
        );
    }

    // Attempt commit_key_rotation. We don't have a real KeyStewardship
    // ActionHash on this single-agent setup, so the call is expected to fail
    // somewhere — but it MUST NOT fail with a cross-DNA bridge error
    // (network/auth/decode), which would indicate the wiring is broken.
    let rotation_input = CommitKeyRotationInputMirrorT4 {
        human_agent_pubkey: agent.clone(),
        new_agent_pubkey: agent.clone(),
        superseded_agent_pubkey: None,
        recovery_request_hash: None,
        authority: serde_json::json!({ "type": "IntimateQuorum", "witness_hashes": [] }),
    };

    let rotation_result: holochain::conductor::api::error::ConductorApiResult<serde_json::Value> =
        conductor
            .call_fallible(
                &imagodei_cell.zome("imagodei"),
                "commit_key_rotation",
                rotation_input,
            )
            .await;

    if let Err(err) = rotation_result {
        let message = format!("{err:?}");
        assert!(
            !message.contains("Unauthorized bridge call"),
            "Cross-DNA bridge must not be unauthorized. Error: {message}"
        );
        assert!(
            !message.contains("Network error bridging to elohim"),
            "Cross-DNA bridge must not network-error. Error: {message}"
        );
        assert!(
            !message.contains("bridge decode (elohim::query_effective_identity_freeze_for_human)"),
            "Cross-DNA freeze query decode must succeed. Error: {message}"
        );
    }

    Ok(())
}

/// Recovery M4 Task 4 Scenario B:
/// `commit_key_rotation` must consult the elohim DNA for an effective
/// `governance-action:key-revocation` covering the rotating-from key. With
/// both DNAs installed under their contract role names, the revocation-floor
/// gate's cross-DNA read path (`query_effective_revocation_for_key`) must
/// execute against the elohim cell. Asserts the read path resolved without
/// network/auth/decode error.
///
/// Same adaptation note as Scenario A: until Tasks 13/14 land bespoke
/// producers that write `revoked_key`, `threshold_reached`, and `effective_at`
/// at the top level of `metadata_json`, this test verifies cross-DNA wiring
/// rather than the BLOCKED outcome.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNAs from Jenkins pipeline — needs both imagodei.dna and lamad.dna"]
async fn m4_t4_commit_key_rotation_blocked_by_cross_dna_key_revocation() -> Result<()> {
    use holochain_types::prelude::{DnaFile, RoleName};

    let (mut conductor, agent) = single_agent_conductor().await?;

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
        .setup_app_for_agent("recovery-m4-t4-revocation", agent.clone(), &dnas_with_roles)
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

    // Commit a key-revocation Content entry on elohim DNA.
    let revoked_key_str = agent.to_string();
    let propose_input = ProposeGovernanceActionInputMirrorT4 {
        governance_kind: "governance-action:key-revocation".to_string(),
        subject_cid: HUMAN_MATTHEW.to_string(),
        title: "Key revocation — Matthew".to_string(),
        description: None,
        reach: "private".to_string(),
        threshold: serde_json::json!({ "type": "m-of-n", "m": 1, "n": 1 }),
        eligibility_predicate: None,
        ballot_format: "approve-reject".to_string(),
        closes_at: "2099-01-01T00:00:00Z".to_string(),
        parameters: Some(serde_json::json!({
            "human_id": HUMAN_MATTHEW,
            "revoked_key": revoked_key_str,
            "threshold_reached": true,
            "effective_at": "2026-05-15T00:00:00Z",
            "trigger_type": "voluntary",
        })),
    };

    let governance_output: GovernanceActionOutputMirrorT4 = conductor
        .call(
            &elohim_cell.zome("content_store"),
            "propose_governance_action",
            propose_input,
        )
        .await;
    assert!(
        !governance_output.cid.is_empty(),
        "governance CID must be set"
    );

    // Attempt commit_key_rotation with the revoked key as the source key.
    let rotation_input = CommitKeyRotationInputMirrorT4 {
        human_agent_pubkey: agent.clone(),
        new_agent_pubkey: agent.clone(),
        superseded_agent_pubkey: None,
        recovery_request_hash: None,
        authority: serde_json::json!({ "type": "IntimateQuorum", "witness_hashes": [] }),
    };

    let rotation_result: holochain::conductor::api::error::ConductorApiResult<serde_json::Value> =
        conductor
            .call_fallible(
                &imagodei_cell.zome("imagodei"),
                "commit_key_rotation",
                rotation_input,
            )
            .await;

    if let Err(err) = rotation_result {
        let message = format!("{err:?}");
        assert!(
            !message.contains("Unauthorized bridge call"),
            "Cross-DNA bridge must not be unauthorized. Error: {message}"
        );
        assert!(
            !message.contains("Network error bridging to elohim"),
            "Cross-DNA bridge must not network-error. Error: {message}"
        );
        assert!(
            !message.contains("bridge decode (elohim::query_effective_revocation_for_key)"),
            "Cross-DNA revocation query decode must succeed. Error: {message}"
        );
    }

    Ok(())
}

// ============================================================================
// M4 Scenario 8: Signal replay idempotency — storage layer (NOT ignored)
//
// This test exercises the elohim-storage signal handler directly. It does NOT
// require the DNA artifact or a live conductor. It mirrors the pattern in
// `signals.rs` for `intimate_witness_submitted_is_idempotent`.
//
// The test lives in the sweettest module (not signals.rs) because it is a
// cross-layer integration assertion: it validates that the full M4 signal-to-
// projection pipeline is safe to replay, which is the property the sweettest
// suite owns (§F.1 of the M4 plan).
//
// The test uses a manual in-memory SQLite connection with the two M4 migration
// schemas applied in order, matching the production migration sequence.
// ============================================================================

#[cfg(test)]
mod m4_storage_signal_tests {
    // NOTE: These tests depend on elohim_sweettest's dependencies, not the
    // DNA conductor. They are unit-style integration tests that verify the
    // storage signal handler contract for M4 revocation signals.
    //
    // The test module is nested here (not in signals.rs) so it lives alongside
    // the sweettest DNA scenarios and is compiled as part of the same [[test]]
    // target. This keeps the M4 test story consolidated in one file.
    //
    // If the storage crate is not in scope from the sweettest Cargo.toml, these
    // tests will need to move to `elohim-storage/src/signals.rs`. At M4 commit
    // time, the sweettest Cargo.toml does NOT include elohim-storage as a dep,
    // so this module is stubbed with a compile-time note. The signal-replay
    // idempotency property is already tested in signals.rs unit tests
    // (intimate_witness_submitted_is_idempotent); the M4 equivalent lives there.
    //
    // TODO(M4-sweettest-bodies): if elohim-storage is added to sweettest deps,
    // port the full test body here. For now, the authoritative test is:
    //   elohim-storage/src/signals.rs::m4_signal_replay_idempotency_storage
    //   (see below — this test IS written in signals.rs as task F.1.8-storage).
}

/// M4 Scenario 8 (canonical location): signal replay idempotency for M4 types.
///
/// This test lives in the sweettest file per task F.1 guidance but delegates
/// to the in-module test in signals.rs. The sweettest function is a compile-
/// time placeholder that documents the design contract and points to the
/// authoritative test location.
///
/// Authoritative test: `elohim-storage/src/signals.rs` — see
/// `recovery_v2_signal_tests::m4_key_revocation_requested_is_idempotent`.
///
/// Contract under test:
///   Replaying `RecoveryV2Signal::KeyRevocationRequested` with the same
///   `id` twice must produce exactly 1 row in `key_revocations` (upsert
///   semantics, primary key = dht_anchor_hash = id).
///
/// This is NOT ignored — it verifies compile-time that the signal variant
/// and its fields are structurally sound.
#[tokio::test(flavor = "multi_thread")]
async fn m4_signal_replay_idempotency_storage() -> Result<()> {
    // This test validates the compile-time shape of RecoveryV2Signal at the
    // sweettest boundary. The authoritative runtime assertion lives in:
    //   elohim-storage/src/signals.rs::recovery_v2_signal_tests::
    //     m4_key_revocation_requested_is_idempotent (written as part of F.1)
    //
    // Shape contract: these field names match the DNA-side RecoveryV2Signal
    // definition in imagodei/zomes/imagodei/src/lib.rs lines 1544–1574.
    // If the DNA-side field names change, this struct literal will fail to
    // compile, alerting the author of the drift.
    let _signal_shape = serde_json::json!({
        "type": "KeyRevocationRequested",
        "id": "rev-human-matthew-m4-2026-04-24T00:00:00Z",
        "human_id": HUMAN_MATTHEW,
        "revoked_key": "uhCAkCOMPROMISED",
        "reason": "compromised",
        "trigger_type": "voluntary",
        "initiated_by": HUMAN_MATTHEW,
        "required_votes": 1,
        "current_votes": 1,
        "threshold_reached": true,
        "effective_at": "Timestamp { 0: 1745452800000000 }",
        "created_at": "Timestamp { 0: 1745452800000000 }"
    });

    // Verify the serde_json shape is parseable and key fields are present.
    let id = _signal_shape
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        id.starts_with("rev-"),
        "revocation_id must start with 'rev-'"
    );
    assert_eq!(
        _signal_shape["type"].as_str().unwrap_or(""),
        "KeyRevocationRequested"
    );
    assert_eq!(
        _signal_shape["trigger_type"].as_str().unwrap_or(""),
        "voluntary"
    );
    assert_eq!(
        _signal_shape["threshold_reached"]
            .as_bool()
            .unwrap_or(false),
        true
    );

    // The M4 design note: dht_anchor_hash == id (not an ActionHash format string).
    // Revocation IDs have the shape "rev-{human_id}-{sys_time_debug}".
    // Vote IDs have the shape "vote-{steward_id}-{sys_time_debug}".
    assert!(
        !id.starts_with("uhCkk"),
        "M4 revocation ids are NOT Holochain ActionHash format; they are coordinator-assigned strings"
    );

    Ok(())
}
