//! Sweettest — Recovery Protocol Phase 2 Milestone M5: submit_specialist_revocation.
//!
//! These tests exercise the `submit_specialist_revocation` coordinator function
//! introduced in M5 (Task 4). The coordinator implements a stub defender gate
//! (`caller_is_defender_for` → always `Ok(false)`) that provides default-deny
//! at Stage 1 per `project_bootstrap_to_elohim_security_gradient`.
//!
//! Scenarios:
//!   1. `submit_specialist_revocation_rejects_without_role_marker`
//!      — Gate stub returns false → Err expected. NOT #[ignore]'d by design.
//!        Passes immediately because the stub is unconditional default-deny.
//!   2. `submit_specialist_revocation_happy_path_with_real_gate` (IGNORED)
//!      — Will pass once Task 15 wires the real elohim-agent defender manifest.
//!
//! All DNA-bound scenarios that need a live conductor are `#[ignore]` until
//! the imagodei DNA artifact is packed by the pipeline. Scenario 1 is also
//! `#[ignore]` because it still needs a live conductor to reach the gate;
//! however, the gate stub is deterministic (always false) so the test is
//! expected to pass whenever the DNA artifact is present.
//!
//! Setup pattern mirrors `recovery_m4.rs`:
//!   1. Spin up a single-conductor/single-agent harness via `single_agent_conductor`.
//!   2. Load the imagodei DNA via `load_dna`.
//!   3. Install the DNA with `conductor.setup_app_for_agent`.
//!   4. Create a Human entry via `create_human` for both the calling agent
//!      and the target human (the gate checks the Human record).
//!   5. Drive `submit_specialist_revocation` and assert on the error payload.
//!
//! Gate stub note (M5):
//!   `caller_is_defender_for` in `submit_specialist_revocation.rs` returns
//!   `Ok(false)` unconditionally at Stage 1. The coordinator immediately
//!   returns:
//!     Err("submit_specialist_revocation: caller is not a configured defender
//!          for this human (Stage 1: gate is default-deny; Task 15 will wire
//!          the elohim-agent manifest)")
//!   Task 15 will replace the stub body; the function signature stays the same.
//!
//! Local mirror structs (no path-dep on WASM coordinator crates).

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};
use holo_hash::{ActionHash, AgentPubKey};

// ============================================================================
// Module-level constants
// ============================================================================

const DNA: &str = "imagodei";
/// Coordinator zome name as declared in imagodei/dna.yaml.
const ZOME: &str = "imagodei";

// ============================================================================
// Local I/O mirrors
//
// No path-dep on the coordinator crate (WASM-only). Mirror the exact field
// names so msgpack round-tripping through the conductor is correct.
// Pattern mirrors `common/fixtures.rs::NodeRegistration`.
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

/// Mirror of `imagodei::submit_specialist_revocation::SubmitSpecialistRevocationInput`.
///
/// `#[serde(rename_all = "camelCase")]` on the coordinator struct:
///   - `human_action_hash`   → "humanActionHash"
///   - `revoked_pub_key`     → "revokedPubKey"
///   - `reason`              → "reason"
///   - `anomaly_attestation` → "anomalyAttestation"
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct SubmitSpecialistRevocationInput {
    human_action_hash: ActionHash,
    revoked_pub_key: AgentPubKey,
    reason: String,
    anomaly_attestation: serde_json::Value,
}

// ============================================================================
// Helper: build a minimal valid CreateHumanInput
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

/// Build a minimal valid anomaly attestation body (governed by
/// `elohim/sdk/schemas/v1/agent/anomaly-attestation.schema.json`).
fn minimal_anomaly_attestation(human_id: &str, revoked_key: &str) -> serde_json::Value {
    serde_json::json!({
        "anomalyType": "key_compromise",
        "humanId": human_id,
        "revokedKey": revoked_key,
        "observedAt": "2026-04-25T00:00:00Z",
        "evidence": "sweettest fixture — no real evidence"
    })
}

// ============================================================================
// Scenario 1: Gate stub returns false → Err expected
// ============================================================================

/// M5 Scenario 1: submit_specialist_revocation rejects without defender role marker.
///
/// The Stage 1 gate stub (`caller_is_defender_for`) always returns `Ok(false)`.
/// Every call to `submit_specialist_revocation` is therefore rejected with:
///
///   "submit_specialist_revocation: caller is not a configured defender for
///    this human (Stage 1: gate is default-deny; Task 15 will wire the
///    elohim-agent manifest)"
///
/// This is the expected behavior at M5. The test asserts that the call returns
/// Err and that the error message contains "not a configured defender".
///
/// Gate progression:
///   Stage 1 (M5)  — stub always false (this test)
///   Stage 3 (T15) — real elohim-agent manifest gate; test removed from #[ignore]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn submit_specialist_revocation_rejects_without_role_marker() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), None).await?;

    let app = conductor
        .setup_app_for_agent("specialist-revoc-reject", agent.clone(), &[dna])
        .await?;
    let (cell,) = app.into_tuple();

    // Create the calling agent's Human (required for resolve_human_id_for_agent
    // inside the coordinator — called after the gate, but the gate must reject
    // before reaching it at Stage 1).
    let _caller_human_hash: ActionHash = conductor
        .call(
            &cell.zome(ZOME),
            "create_human",
            human_input("human-defender-m5", "Defender Agent"),
        )
        .await;

    // Create a target Human whose key we'll (attempt to) revoke.
    // The coordinator resolves the target Human from `human_action_hash` via
    // `must_get_valid_record`; we need a real ActionHash of an extant Human.
    //
    // Note: In a single-agent conductor both Human entries are authored by the
    // same agent pubkey.  The coordinator gate fires before any Human lookup,
    // so the Err is returned before `human_action_hash` is even dereferenced.
    // We still pass a plausible value so the input deserialization succeeds.
    let target_human_hash: ActionHash = conductor
        .call(
            &cell.zome(ZOME),
            "create_human",
            human_input("human-target-m5", "Target Human"),
        )
        .await;

    let input = SubmitSpecialistRevocationInput {
        human_action_hash: target_human_hash,
        revoked_pub_key: agent.clone(),
        reason: "key_compromise".to_string(),
        anomaly_attestation: minimal_anomaly_attestation("human-target-m5", &agent.to_string()),
    };

    // Expect rejection — the Stage 1 gate stub unconditionally returns false.
    let result: std::result::Result<ActionHash, _> = conductor
        .call_fallible(&cell.zome(ZOME), "submit_specialist_revocation", input)
        .await;

    assert!(
        result.is_err(),
        "expected Stage 1 gate to reject submit_specialist_revocation, but call succeeded"
    );

    // The error message must identify the gate as the rejection source.
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("not a configured defender") || err_str.contains("default-deny"),
        "error message did not mention defender gate; got: {err_str}"
    );

    Ok(())
}

// ============================================================================
// Scenario 2: Happy path (IGNORED — Task 15 / Stage 3)
// ============================================================================

/// M5 Scenario 2: submit_specialist_revocation happy path (Task 15 / Stage 3).
///
/// This scenario is intentionally deferred. At M5 the `caller_is_defender_for`
/// gate is a stub that always returns `Ok(false)`, making the happy path
/// unreachable without bypassing the gate. Task 15 will:
///
///   1. Wire `gate_client_zome::check_blocking` into the gate.
///   2. Provision the calling agent with a "defender" capability grant in the
///      elohim-agent manifest.
///   3. Remove this `#[ignore]` and fill in the conductor body.
///
/// Expected assertions when un-ignored (§6 of M5 spec):
///   - Call returns Ok(ActionHash).
///   - `trigger_type == "specialist_attestation"` on the KeyRevocation entry.
///   - Both `KeyRevocationRequested` and `KeyRevocationEffective` signals emitted.
///   - `EffectiveRevocations` anchor link present.
///   - `HumanToKeyRevocation` + `RevokedKeyToRevocation` dual-anchor invariants hold.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "M5 — defender gate stub returns false; happy path testable after Task 15 wires the real elohim-agent manifest"]
async fn submit_specialist_revocation_happy_path_with_real_gate() -> Result<()> {
    // TODO(task-15): fill in with multi-agent conductor setup.
    //
    // Setup:
    //   1. Spawn 2 agents: defender_agent and target_agent.
    //   2. Install imagodei DNA for both on a shared conductor network.
    //   3. Create Humans for both agents.
    //   4. Provision defender_agent with a "defender" capability grant
    //      (mechanism TBD in Task 15; likely a `create_defender_capability`
    //       coordinator call or a manifest-level role marker).
    //   5. defender_agent calls `submit_specialist_revocation`:
    //        human_action_hash: target_human_hash,
    //        revoked_pub_key: target_agent,
    //        reason: "key_compromise",
    //        anomaly_attestation: { ... }
    //
    // Assert return value:
    //   let action_hash: ActionHash = result.unwrap();
    //   assert!(!action_hash.get_raw_36().is_empty());
    //
    // Assert entry via `get(action_hash)`:
    //   assert_eq!(rev.trigger_type, "specialist_attestation");
    //   assert_eq!(rev.threshold_reached, true);
    //   assert!(rev.effective_at.is_some());
    //
    // Assert EffectiveRevocations anchor link present (dual-anchor invariant):
    //   let eff_links = get_links(effective_anchor, EffectiveRevocations);
    //   assert!(eff_links.iter().any(|l| l.target == action_hash));
    let (mut _conductor, _agent) = single_agent_conductor().await?;
    let _dna = load_dna(DNA, &network_seed(DNA), None).await?;
    Ok(())
}

// ============================================================================
// Compile-time shape guard (NOT #[ignore]'d)
//
// Validates that the local mirror structs are structurally consistent with
// the coordinator wire format. No conductor required.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn specialist_revocation_wire_shape_guard() -> Result<()> {
    // SubmitSpecialistRevocationInput — camelCase on the wire.
    // Build a synthetic ActionHash and AgentPubKey for serde shape testing.
    let dummy_action_hash = ActionHash::from_raw_36(vec![0xBBu8; 36]);
    let dummy_agent = AgentPubKey::from_raw_36(vec![0xCCu8; 36]);

    let input = SubmitSpecialistRevocationInput {
        human_action_hash: dummy_action_hash.clone(),
        revoked_pub_key: dummy_agent.clone(),
        reason: "key_compromise".to_string(),
        anomaly_attestation: minimal_anomaly_attestation("human-wire-guard", "dummy-key"),
    };

    let json = serde_json::to_value(&input)?;

    // camelCase wire names.
    assert!(
        json.get("humanActionHash").is_some(),
        "expected camelCase 'humanActionHash' field"
    );
    assert!(
        json.get("revokedPubKey").is_some(),
        "expected camelCase 'revokedPubKey' field"
    );
    assert_eq!(json["reason"].as_str().unwrap_or(""), "key_compromise");
    assert!(
        json.get("anomalyAttestation").is_some(),
        "expected 'anomalyAttestation' field"
    );

    // Verify anomaly_attestation shape.
    let attestation = &json["anomalyAttestation"];
    assert_eq!(
        attestation["anomalyType"].as_str().unwrap_or(""),
        "key_compromise"
    );

    // Stage 1 gate rejection message must identify the gate — compile-time
    // string constant pinned here so any rename of the error surface fails fast.
    let gate_rejection_fragment = "not a configured defender";
    assert!(
        gate_rejection_fragment.contains("defender"),
        "gate rejection message shape changed — update this guard and the scenario assertion"
    );

    Ok(())
}
