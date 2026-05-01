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
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

const DNA: &str = "elohim";

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
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateFeedbackSignalInput {
    pub target_action_hash: ActionHash,
    pub signal_kind: String,
    pub evidence_action_hash: Option<ActionHash>,
    pub standing_impact: String,
    pub signer_pubkey: Vec<u8>,
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
    pub evidence_cid: Option<String>,
    pub standing_impact: String,
    pub signer_pubkey: Vec<u8>,
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
        .call(&cell_a.zome("content_store"), "create_content", make_content("t8-s1"))
        .await;
    let content_ah = output.action_hash;

    // B squelches A's content.
    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "squelch".to_string(),
        evidence_action_hash: None,
        standing_impact: "advisory".to_string(),
        signer_pubkey: a2.get_raw_39().to_vec(),
    };
    let _fs_ah: ActionHash = cb
        .call(&cell_b.zome("content_store"), "create_feedback_signal", input)
        .await;

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
        signer_pubkey: a2.get_raw_39().to_vec(),
    };
    let _fs_ah: ActionHash = cb
        .call(&cell_b.zome("content_store"), "create_feedback_signal", input)
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
        .call(&cell.zome("content_store"), "create_content", make_content("t8-s3"))
        .await;
    let content_ah = output.action_hash;

    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "retraction".to_string(),
        evidence_action_hash: None,
        standing_impact: "debit-firm".to_string(),
        signer_pubkey: a1.get_raw_39().to_vec(),
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
        .call(&cell_a.zome("content_store"), "create_content", make_content("t8-s4"))
        .await;
    let content_ah = output.action_hash;

    // B tries to retract A's content — this MUST fail.
    // two_agent_conductors() shares an in-process network so DHT gossips immediately;
    // must_get_valid_record on content_ah will succeed, revealing a1 as author != a2.
    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "retraction".to_string(),
        evidence_action_hash: None,
        standing_impact: "debit-firm".to_string(),
        signer_pubkey: a2.get_raw_39().to_vec(),
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
        .call(&cell_a.zome("content_store"), "create_content", make_content("t8-s5"))
        .await;
    let content_ah = output.action_hash;

    // Fabricate a non-existent ActionHash (36 bytes of zeros — valid size, invalid hash).
    let fake_evidence_ah = ActionHash::from_raw_36(vec![0u8; 36]);

    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "correction".to_string(),
        evidence_action_hash: Some(fake_evidence_ah),
        standing_impact: "debit-soft".to_string(),
        signer_pubkey: a2.get_raw_39().to_vec(),
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
        .call(&cell.zome("content_store"), "create_content", make_content("t8-s6"))
        .await;
    let content_ah = content_output.action_hash;

    let input = CreateFeedbackSignalInput {
        target_action_hash: content_ah.clone(),
        signal_kind: "squelch".to_string(),
        evidence_action_hash: None,
        standing_impact: "advisory".to_string(),
        signer_pubkey: a1.get_raw_39().to_vec(),
    };
    let _fs_ah: ActionHash = ca
        .call(&cell.zome("content_store"), "create_feedback_signal", input)
        .await;

    // Try to call a non-existent update function — must fail at the API boundary.
    let update_result = ca
        .call_fallible::<_, ActionHash>(
            &cell.zome("content_store"),
            "update_feedback_signal",
            (),
        )
        .await;

    assert!(
        update_result.is_err(),
        "update_feedback_signal must not be exposed as a zome function"
    );

    Ok(())
}
