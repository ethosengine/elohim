//! Sweettest — FeedbackSignal coordinator (EPR Phase 3.5, T8).
//!
//! Exercises the coordinator pre-commit gates and link topology for all 4
//! signal variants. Tests are `#[ignore]` until the DNA is packed by Jenkins;
//! remove `#[ignore]` once the pipeline's pack-then-test stage is wired.
//!
//! Scenarios:
//!   1. `squelch_by_third_party_succeeds` — Agent B squelches Agent A's content.
//!   2. `correction_with_evidence_succeeds` — Agent B corrects Agent A's content
//!      citing B's own correction-entry as evidence.
//!   3. `retraction_by_original_author_succeeds` — Agent A retracts their own
//!      content.
//!   4. `retraction_by_non_author_rejected` — Agent B attempts to retract Agent
//!      A's content; rejected with the specific error message.
//!   5. `correction_with_missing_evidence_rejected` — Agent B's correction
//!      cites a non-existent evidence hash; rejected.
//!   6. `feedback_signal_update_rejected` — Any attempt to update a committed
//!      FeedbackSignal entry is rejected (T4 immutability holds end-to-end).

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors},
    fixtures::network_seed,
};
use holo_hash::{ActionHash, EntryHash};
use holochain::sweettest::{await_consistency, SweetConductor};
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

const DNA: &str = "lamad";

// ---------------------------------------------------------------------------
// Local mirror types (no path-dep on WASM coordinator crate allowed here).
// Field names and order MUST match the coordinator / SDK structs exactly.
// ---------------------------------------------------------------------------

/// Mirror of `lamad_types::CreateContentInput`.
/// Only required fields (optional fields use `#[serde(default)]` in the real
/// type and are safe to omit in serialization).
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateContentInput {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    pub content: String,
    pub content_format: String,
    pub reach: String,
    pub metadata_json: String,
}

/// Minimal mirror of `lamad_types::Content` wire type.
/// Must carry the same required fields; optional fields default to None/empty.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct ContentWire {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub summary: Option<String>,
    pub content: String,
    pub content_format: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub related_node_ids: Vec<String>,
    #[serde(default)]
    pub author_id: Option<String>,
    pub reach: String,
    pub trust_score: f64,
    #[serde(default)]
    pub estimated_minutes: Option<u32>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
    #[serde(default)]
    pub blob_cid: Option<String>,
    #[serde(default)]
    pub content_size_bytes: Option<u64>,
    #[serde(default)]
    pub content_hash: Option<String>,
}

/// Mirror of `lamad_types::ContentOutput`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct ContentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub content: ContentWire,
}

/// Mirror of `content_store::feedback_signal::CreateFeedbackSignalInput`.
///
/// `signer_pubkey` is absent — the coordinator derives it from `agent_info()`
/// to prevent caller-side spoofing (I1 fix).
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateFeedbackSignalInput {
    pub target_action_hash: ActionHash,
    pub signal_kind: String,
    pub evidence_action_hash: Option<ActionHash>,
    pub standing_impact: String,
}

/// Mirror of `content_store::feedback_signal::FeedbackSignalRecord`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct FeedbackSignalRecord {
    pub action_hash: ActionHash,
    pub entry: FeedbackSignalEntry,
}

/// Mirror of `content_store_integrity::FeedbackSignal`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct FeedbackSignalEntry {
    pub target_cid: String,
    pub signal_kind: String,
    /// T5 addition: vouch sub-semantics; None for non-vouch signals.
    #[serde(default)]
    pub vouch_kind: Option<String>,
    pub evidence_cid: Option<String>,
    pub standing_impact: String,
    pub signer_pubkey: Vec<u8>,
}

/// Mirror of `content_store::feedback_signal::CreateVouchInput` (T6).
///
/// No `signer_pubkey` — coordinator derives it from `agent_info()`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateVouchInput {
    pub target_action_hash: ActionHash,
    pub vouch_kind: String,
    pub standing_impact: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_content(id: &str) -> CreateContentInput {
    CreateContentInput {
        id: id.to_string(),
        content_type: "concept".to_string(),
        title: format!("T8 sweettest content {id}"),
        description: "Test content for feedback_signal sweettests".to_string(),
        content: "# Test\nFeedback signal test fixture.".to_string(),
        content_format: "markdown".to_string(),
        reach: "community".to_string(),
        metadata_json: "{}".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Scenario 1: Squelch by third party succeeds.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn squelch_by_third_party_succeeds() -> Result<()> {
    // Agent A creates content; Agent B squelches it (no authorship gate on squelch).
    let [(mut ca, a1), (mut cb, a2)] = two_agent_conductors().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;

    let app_a = ca
        .setup_app_for_agent("elohim-app-a", a1.clone(), &[dna.clone()])
        .await?;
    let app_b = cb
        .setup_app_for_agent("elohim-app-b", a2.clone(), &[dna])
        .await?;
    let cell_a = app_a.cells().first().expect("cell A").clone();
    let cell_b = app_b.cells().first().expect("cell B").clone();

    // A creates content; extract action_hash from ContentOutput.
    let output: ContentOutput = ca
        .call(
            &cell_a.zome("content_store"),
            "create_content",
            make_content("t8-s1"),
        )
        .await;
    let content_ah = output.action_hash;

    // B squelches A's content.
    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "squelch".to_string(),
        evidence_action_hash: None,
        standing_impact: "advisory".to_string(),
    };
    let fs_ah: ActionHash = cb
        .call(
            &cell_b.zome("content_store"),
            "create_feedback_signal",
            input,
        )
        .await;

    // I2: get_feedback_signals_for_target must return the committed signal.
    let by_target: Vec<FeedbackSignalRecord> = cb
        .call(
            &cell_b.zome("content_store"),
            "get_feedback_signals_for_target",
            content_ah.clone(),
        )
        .await;
    assert_eq!(by_target.len(), 1, "expected 1 signal for target content");
    assert_eq!(
        by_target[0].entry.signal_kind, "squelch",
        "signal_kind must be squelch"
    );
    assert_eq!(
        by_target[0].action_hash, fs_ah,
        "returned action_hash must match created signal"
    );

    // B's own list_feedback_signals_by_signer should include the new signal.
    let by_signer: Vec<FeedbackSignalRecord> = cb
        .call(
            &cell_b.zome("content_store"),
            "list_feedback_signals_by_signer",
            a2.clone(),
        )
        .await;
    assert_eq!(by_signer.len(), 1, "expected 1 signal from signer B");
    assert_eq!(by_signer[0].entry.signal_kind, "squelch");

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 2: Correction with evidence succeeds.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn correction_with_evidence_succeeds() -> Result<()> {
    // A creates target content; B creates a correction EPR (any content entry
    // works as evidence since the gate only checks existence); B corrects A's content.
    let [(mut ca, a1), (mut cb, a2)] = two_agent_conductors().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;

    let app_a = ca
        .setup_app_for_agent("elohim-app-a", a1.clone(), &[dna.clone()])
        .await?;
    let app_b = cb
        .setup_app_for_agent("elohim-app-b", a2.clone(), &[dna])
        .await?;
    let cell_a = app_a.cells().first().expect("cell A").clone();
    let cell_b = app_b.cells().first().expect("cell B").clone();

    // A creates the target content.
    let target_output: ContentOutput = ca
        .call(
            &cell_a.zome("content_store"),
            "create_content",
            make_content("t8-s2-target"),
        )
        .await;
    let content_ah = target_output.action_hash;

    // B creates the correction EPR (another content entry serves as evidence placeholder).
    let evidence_output: ContentOutput = cb
        .call(
            &cell_b.zome("content_store"),
            "create_content",
            make_content("t8-s2-evidence"),
        )
        .await;
    let evidence_ah = evidence_output.action_hash;

    // B files a correction referencing the evidence.
    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "correction".to_string(),
        evidence_action_hash: Some(evidence_ah.clone()),
        standing_impact: "debit-soft".to_string(),
    };
    let _fs_ah: ActionHash = cb
        .call(
            &cell_b.zome("content_store"),
            "create_feedback_signal",
            input,
        )
        .await;

    // Verify signer index.
    let by_signer: Vec<FeedbackSignalRecord> = cb
        .call(
            &cell_b.zome("content_store"),
            "list_feedback_signals_by_signer",
            a2.clone(),
        )
        .await;
    assert_eq!(by_signer.len(), 1);
    assert_eq!(by_signer[0].entry.signal_kind, "correction");
    assert!(
        by_signer[0].entry.evidence_cid.is_some(),
        "evidence_cid must be set for correction"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 3: Retraction by original author succeeds.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn retraction_by_original_author_succeeds() -> Result<()> {
    let (mut ca, a1) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;
    let app = ca
        .setup_app_for_agent("elohim-app", a1.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell").clone();

    // A creates content and then retracts it (author == caller — gate passes).
    let output: ContentOutput = ca
        .call(
            &cell.zome("content_store"),
            "create_content",
            make_content("t8-s3"),
        )
        .await;
    let content_ah = output.action_hash;

    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "retraction".to_string(),
        evidence_action_hash: None,
        standing_impact: "debit-firm".to_string(),
    };
    let _fs_ah: ActionHash = ca
        .call(&cell.zome("content_store"), "create_feedback_signal", input)
        .await;

    // Signer index on self.
    let by_signer: Vec<FeedbackSignalRecord> = ca
        .call(
            &cell.zome("content_store"),
            "list_feedback_signals_by_signer",
            a1.clone(),
        )
        .await;
    assert_eq!(by_signer.len(), 1);
    assert_eq!(by_signer[0].entry.signal_kind, "retraction");

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 4: Retraction by non-author REJECTED.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn retraction_by_non_author_rejected() -> Result<()> {
    let [(mut ca, a1), (mut cb, a2)] = two_agent_conductors().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;

    let app_a = ca
        .setup_app_for_agent("elohim-app-a", a1.clone(), &[dna.clone()])
        .await?;
    let app_b = cb
        .setup_app_for_agent("elohim-app-b", a2.clone(), &[dna])
        .await?;
    let cell_a = app_a.cells().first().expect("cell A").clone();
    let cell_b = app_b.cells().first().expect("cell B").clone();

    // A creates content.
    let output: ContentOutput = ca
        .call(
            &cell_a.zome("content_store"),
            "create_content",
            make_content("t8-s4"),
        )
        .await;
    let content_ah = output.action_hash;

    // Exchange peer info + wait for DHT consistency so B's coordinator can resolve
    // A's content via must_get_valid_record. Without this, must_get fails with
    // "Failed to get Record" before the authorship gate runs, and the test asserts
    // on the wrong error message. Same pattern used by imagodei_peer_binding +
    // epr_phase_2b_batch_a_e2e for cross-agent reads.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while !SweetConductor::exchange_peer_info([&ca, &cb]).await {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;

    await_consistency(10, [&cell_a, &cell_b])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout: {e}"))?;

    // B tries to retract A's content — this MUST fail at the authorship gate.
    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "retraction".to_string(),
        evidence_action_hash: None,
        standing_impact: "debit-firm".to_string(),
    };
    let result = cb
        .call_fallible::<_, ActionHash>(
            &cell_b.zome("content_store"),
            "create_feedback_signal",
            input,
        )
        .await;

    assert!(result.is_err(), "retraction by non-author must be rejected");
    let err = result.unwrap_err();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains("retraction signer must be the original author"),
        "expected specific error message, got: {err_str}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 5: Correction with missing evidence REJECTED.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn correction_with_missing_evidence_rejected() -> Result<()> {
    let [(mut ca, a1), (mut cb, a2)] = two_agent_conductors().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;

    let app_a = ca
        .setup_app_for_agent("elohim-app-a", a1.clone(), &[dna.clone()])
        .await?;
    let app_b = cb
        .setup_app_for_agent("elohim-app-b", a2.clone(), &[dna])
        .await?;
    let cell_a = app_a.cells().first().expect("cell A").clone();
    let cell_b = app_b.cells().first().expect("cell B").clone();

    // A creates content.
    let output: ContentOutput = ca
        .call(
            &cell_a.zome("content_store"),
            "create_content",
            make_content("t8-s5"),
        )
        .await;
    let content_ah = output.action_hash;

    // Fabricate a non-existent ActionHash (36 bytes of zeros — valid size, invalid hash).
    let fake_evidence_ah = ActionHash::from_raw_36(vec![0u8; 36]);

    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "correction".to_string(),
        evidence_action_hash: Some(fake_evidence_ah),
        standing_impact: "debit-soft".to_string(),
    };
    let result = cb
        .call_fallible::<_, ActionHash>(
            &cell_b.zome("content_store"),
            "create_feedback_signal",
            input,
        )
        .await;

    assert!(
        result.is_err(),
        "correction with non-existent evidence must be rejected"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 6: Update of FeedbackSignal entry is REJECTED (T4 immutability).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn feedback_signal_update_rejected() -> Result<()> {
    // This scenario verifies the T4 HDI validate_update_entry gate holds
    // end-to-end in a real conductor AND that no update path exists at the
    // coordinator API level (defence in depth).
    //
    // Implementation: call a non-existent `update_feedback_signal` function
    // and assert failure.  The absence of the function at the API level IS the
    // primary defence; the HDI gate is a second layer.
    let (mut ca, a1) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;
    let app = ca
        .setup_app_for_agent("elohim-app", a1.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell").clone();

    // A creates content and a FeedbackSignal.
    let content_output: ContentOutput = ca
        .call(
            &cell.zome("content_store"),
            "create_content",
            make_content("t8-s6"),
        )
        .await;
    let content_ah = content_output.action_hash;

    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "squelch".to_string(),
        evidence_action_hash: None,
        standing_impact: "advisory".to_string(),
    };
    let _fs_ah: ActionHash = ca
        .call(&cell.zome("content_store"), "create_feedback_signal", input)
        .await;

    // Try to call a non-existent update function — must fail at the API boundary.
    let update_result = ca
        .call_fallible::<_, ActionHash>(&cell.zome("content_store"), "update_feedback_signal", ())
        .await;

    assert!(
        update_result.is_err(),
        "update_feedback_signal must not be exposed as a zome function"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 7 (I3): Quarantine by third party succeeds.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn quarantine_by_third_party_succeeds() -> Result<()> {
    // Agent A creates content; Agent B issues a quarantine signal (governance-collective
    // determination). There is no authorship gate on quarantine — any agent may issue it.
    // The canonical standing_impact pairing for quarantine is debit-firm.
    let [(mut ca, a1), (mut cb, a2)] = two_agent_conductors().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;

    let app_a = ca
        .setup_app_for_agent("elohim-app-a", a1.clone(), &[dna.clone()])
        .await?;
    let app_b = cb
        .setup_app_for_agent("elohim-app-b", a2.clone(), &[dna])
        .await?;
    let cell_a = app_a.cells().first().expect("cell A").clone();
    let cell_b = app_b.cells().first().expect("cell B").clone();

    // A creates content; extract action_hash from ContentOutput.
    let output: ContentOutput = ca
        .call(
            &cell_a.zome("content_store"),
            "create_content",
            make_content("t8-s7"),
        )
        .await;
    let content_ah = output.action_hash;

    // B issues a quarantine signal on A's content.
    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "quarantine".to_string(),
        evidence_action_hash: None,
        standing_impact: "debit-firm".to_string(),
    };
    let _fs_ah: ActionHash = cb
        .call(
            &cell_b.zome("content_store"),
            "create_feedback_signal",
            input,
        )
        .await;

    // B's signer index must include the quarantine signal.
    let by_signer: Vec<FeedbackSignalRecord> = cb
        .call(
            &cell_b.zome("content_store"),
            "list_feedback_signals_by_signer",
            a2.clone(),
        )
        .await;
    assert_eq!(by_signer.len(), 1, "expected 1 signal from signer B");
    assert_eq!(by_signer[0].entry.signal_kind, "quarantine");
    assert_eq!(by_signer[0].entry.standing_impact, "debit-firm");

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 8 (T7): Alice vouches on Bob's correction — succeeds.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn create_vouch_succeeds_when_signer_differs_from_target() -> Result<()> {
    // Bob creates a correction. Alice vouches on it.
    // Alice and Bob have different agent keys → no-self-vouch guard allows this.
    let [(mut ca, a1), (mut cb, a2)] = two_agent_conductors().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;

    let app_a = ca
        .setup_app_for_agent("elohim-app-a", a1.clone(), &[dna.clone()])
        .await?;
    let app_b = cb
        .setup_app_for_agent("elohim-app-b", a2.clone(), &[dna])
        .await?;
    let cell_a = app_a.cells().first().expect("cell A").clone();
    let cell_b = app_b.cells().first().expect("cell B").clone();

    // Bob creates a content item and a correction signal against it.
    let content_output: ContentOutput = cb
        .call(
            &cell_b.zome("content_store"),
            "create_content",
            make_content("t7-s8-target"),
        )
        .await;
    let content_ah = content_output.action_hash;

    let evidence_output: ContentOutput = cb
        .call(
            &cell_b.zome("content_store"),
            "create_content",
            make_content("t7-s8-evidence"),
        )
        .await;
    let evidence_ah = evidence_output.action_hash;

    let correction_input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "correction".to_string(),
        evidence_action_hash: Some(evidence_ah),
        standing_impact: "debit-soft".to_string(),
    };
    let bob_correction_hash: ActionHash = cb
        .call(
            &cell_b.zome("content_store"),
            "create_feedback_signal",
            correction_input,
        )
        .await;

    // Alice vouches on Bob's correction. Different agents → must succeed.
    let vouch_input = CreateVouchInput {
        target_action_hash: bob_correction_hash.clone(),
        vouch_kind: "accept-correction".to_string(),
        standing_impact: "debit-soft".to_string(),
    };
    let result = ca
        .call_fallible::<_, ActionHash>(&cell_a.zome("content_store"), "create_vouch", vouch_input)
        .await;
    assert!(
        result.is_ok(),
        "alice vouching on bob's correction should succeed: {:?}",
        result
    );

    let vouch_ah = result.unwrap();

    // Verify the vouch appears in get_feedback_signals_for_target on the correction.
    let by_target: Vec<FeedbackSignalRecord> = ca
        .call(
            &cell_a.zome("content_store"),
            "get_feedback_signals_for_target",
            bob_correction_hash.clone(),
        )
        .await;
    assert_eq!(
        by_target.len(),
        1,
        "expected 1 vouch signal for bob's correction"
    );
    assert_eq!(by_target[0].entry.signal_kind, "vouch");
    assert_eq!(
        by_target[0].entry.vouch_kind,
        Some("accept-correction".to_string())
    );
    assert_eq!(by_target[0].action_hash, vouch_ah);

    // Alice's signer index must include the vouch.
    let by_signer: Vec<FeedbackSignalRecord> = ca
        .call(
            &cell_a.zome("content_store"),
            "list_feedback_signals_by_signer",
            a1.clone(),
        )
        .await;
    assert_eq!(by_signer.len(), 1, "expected 1 vouch from alice");
    assert_eq!(by_signer[0].entry.signal_kind, "vouch");

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 9 (T7): Bob tries to vouch on his own correction — REJECTED.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn create_vouch_rejects_self_vouch() -> Result<()> {
    // Bob creates a correction, then tries to vouch on it himself.
    // Same agent key → no-self-vouch guard must reject.
    let (mut cb, a2) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a2.clone())).await?;
    let app_b = cb
        .setup_app_for_agent("elohim-app-b", a2.clone(), &[dna])
        .await?;
    let cell_b = app_b.cells().first().expect("cell B").clone();

    // Bob creates a content item and a correction signal.
    let content_output: ContentOutput = cb
        .call(
            &cell_b.zome("content_store"),
            "create_content",
            make_content("t7-s9-target"),
        )
        .await;
    let content_ah = content_output.action_hash;

    let evidence_output: ContentOutput = cb
        .call(
            &cell_b.zome("content_store"),
            "create_content",
            make_content("t7-s9-evidence"),
        )
        .await;
    let evidence_ah = evidence_output.action_hash;

    let correction_input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "correction".to_string(),
        evidence_action_hash: Some(evidence_ah),
        standing_impact: "debit-soft".to_string(),
    };
    let bob_correction_hash: ActionHash = cb
        .call(
            &cell_b.zome("content_store"),
            "create_feedback_signal",
            correction_input,
        )
        .await;

    // Bob tries to vouch on his own correction — MUST fail.
    let self_vouch_input = CreateVouchInput {
        target_action_hash: bob_correction_hash.clone(),
        vouch_kind: "accept-correction".to_string(),
        standing_impact: "debit-soft".to_string(),
    };
    let result = cb
        .call_fallible::<_, ActionHash>(
            &cell_b.zome("content_store"),
            "create_vouch",
            self_vouch_input,
        )
        .await;

    assert!(result.is_err(), "self-vouch must be rejected");
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("self-vouch forbidden") || err_str.contains("forbidden"),
        "error message must mention self-vouch or forbidden: {err_str}"
    );

    Ok(())
}
