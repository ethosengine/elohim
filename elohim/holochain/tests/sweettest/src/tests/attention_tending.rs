//! Sweettest — AttentionTending coordinator (EPR Phase 3.5, T9).
//!
//! Exercises the coordinator functions for AttentionTending private entries.
//! Tests are `#[ignore]` until the DNA is packed by the Jenkins pipeline.
//!
//! Scenarios:
//!   1. `create_and_list_succeeds` — Agent A creates an AttentionTending;
//!      `list_my_tending` returns 1 record matching the input.
//!   2. `refresh_ttl_appends_timestamp` — Agent A creates with ttl=3600, then
//!      refreshes with new_ttl=86400. The updated entry's `tended_at.len()` is
//!      2 and `ttl_seconds` is 86400.
//!   3. `cross_agent_get_returns_none` — Agent A creates an AttentionTending.
//!      Agent B's `list_my_tending` returns an empty vec (private visibility
//!      blocks gossip; Agent B's source chain has no AttentionTending entries).
//!   4. `malformed_json_rejected` — Agent A calls `create_attention_tending`
//!      with `filter_subject_json: "not valid json {"`. Must return an error
//!      containing "filter_subject_json" or "json".

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors},
    fixtures::network_seed,
};
use holo_hash::ActionHash;
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

const DNA: &str = "elohim";

// ---------------------------------------------------------------------------
// Local mirror types — no path-dep on WASM coordinator crate allowed here.
// Field names and order MUST match the coordinator structs exactly.
// ---------------------------------------------------------------------------

/// Mirror of `content_store::attention_tending::CreateAttentionTendingInput`.
///
/// `signer_pubkey` is absent — the coordinator derives it from `agent_info()`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateAttentionTendingInput {
    pub filter_subject_json: String,
    pub classification: String,
    pub reason: Option<String>,
    pub ttl_seconds: u64,
    pub context_json: String,
}

/// Mirror of `content_store::attention_tending::RefreshTendingTtlInput`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct RefreshTendingTtlInput {
    pub original_action_hash: ActionHash,
    pub new_ttl_seconds: u64,
}

/// Mirror of `content_store_integrity::AttentionTending`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct AttentionTendingEntry {
    pub filter_subject_json: String,
    pub classification: String,
    pub reason: Option<String>,
    pub ttl_seconds: u64,
    pub tended_at: Vec<u64>,
    pub context_json: String,
    pub signer_pubkey: Vec<u8>,
}

/// Mirror of `content_store::attention_tending::AttentionTendingRecord`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct AttentionTendingRecord {
    pub action_hash: ActionHash,
    pub entry: AttentionTendingEntry,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_input(classification: &str) -> CreateAttentionTendingInput {
    CreateAttentionTendingInput {
        filter_subject_json: r#"{"contentKind":"concept"}"#.to_string(),
        classification: classification.to_string(),
        reason: None,
        ttl_seconds: 3600,
        context_json: r#"{"collective":"household"}"#.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Scenario 1: create and list succeeds.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn create_and_list_succeeds() -> Result<()> {
    let (mut ca, a1) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;
    let app = ca
        .setup_app_for_agent("elohim-app", a1.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell").clone();

    // Create an AttentionTending entry.
    let _ah: ActionHash = ca
        .call(
            &cell.zome("content_store"),
            "create_attention_tending",
            make_input("values-forward"),
        )
        .await;

    // list_my_tending must return exactly 1 record.
    let records: Vec<AttentionTendingRecord> = ca
        .call(&cell.zome("content_store"), "list_my_tending", ())
        .await;

    assert_eq!(records.len(), 1, "expected 1 record from list_my_tending");
    assert_eq!(
        records[0].entry.classification, "values-forward",
        "classification must match input"
    );
    assert_eq!(
        records[0].entry.ttl_seconds, 3600,
        "ttl_seconds must match input"
    );
    assert_eq!(
        records[0].entry.filter_subject_json,
        r#"{"contentKind":"concept"}"#,
        "filter_subject_json must match input"
    );
    assert_eq!(
        records[0].entry.tended_at.len(),
        1,
        "initial tended_at must contain exactly one timestamp"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 2: refresh_ttl appends timestamp and updates ttl_seconds.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn refresh_ttl_appends_timestamp() -> Result<()> {
    let (mut ca, a1) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;
    let app = ca
        .setup_app_for_agent("elohim-app", a1.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell").clone();

    // Create with ttl=3600.
    let original_ah: ActionHash = ca
        .call(
            &cell.zome("content_store"),
            "create_attention_tending",
            make_input("fatigue"),
        )
        .await;

    // Refresh with new_ttl=86400.
    let refresh_input = RefreshTendingTtlInput {
        original_action_hash: original_ah.clone(),
        new_ttl_seconds: 86400,
    };
    let _updated_ah: ActionHash = ca
        .call(
            &cell.zome("content_store"),
            "refresh_tending_ttl",
            refresh_input,
        )
        .await;

    // list_my_tending now returns 2 records (original + update head).
    // The updated record has tended_at.len() == 2 and ttl_seconds == 86400.
    let records: Vec<AttentionTendingRecord> = ca
        .call(&cell.zome("content_store"), "list_my_tending", ())
        .await;

    // Find the record with the longer tended_at (the updated one).
    let updated = records
        .iter()
        .find(|r| r.entry.tended_at.len() == 2)
        .expect("expected a record with 2 tended_at entries after refresh");

    assert_eq!(
        updated.entry.ttl_seconds, 86400,
        "updated ttl_seconds must be 86400"
    );
    assert_eq!(
        updated.entry.tended_at.len(),
        2,
        "tended_at must have 2 entries after one refresh"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 3: cross-agent — Agent B's list_my_tending returns empty.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn cross_agent_get_returns_none() -> Result<()> {
    // Agent A and Agent B are on separate conductors sharing an in-process
    // network. Even with gossip, Visibility::Private entries never leave the
    // author's source chain — Agent B's source chain has no AttentionTending.
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

    // Agent A creates an AttentionTending.
    let _ah: ActionHash = ca
        .call(
            &cell_a.zome("content_store"),
            "create_attention_tending",
            make_input("scope-mismatch"),
        )
        .await;

    // Agent B's list_my_tending must return an empty vec.
    // The private entry never gossiped to B's source chain.
    let b_records: Vec<AttentionTendingRecord> = cb
        .call(&cell_b.zome("content_store"), "list_my_tending", ())
        .await;

    assert!(
        b_records.is_empty(),
        "Agent B must see zero AttentionTending records — Visibility::Private blocks gossip"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 4: malformed filter_subject_json is REJECTED.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn malformed_json_rejected() -> Result<()> {
    let (mut ca, a1) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;
    let app = ca
        .setup_app_for_agent("elohim-app", a1.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell").clone();

    let bad_input = CreateAttentionTendingInput {
        filter_subject_json: "not valid json {".to_string(),
        classification: "values-forward".to_string(),
        reason: None,
        ttl_seconds: 3600,
        context_json: r#"{"collective":"household"}"#.to_string(),
    };

    let result = ca
        .call_fallible::<_, ActionHash>(
            &cell.zome("content_store"),
            "create_attention_tending",
            bad_input,
        )
        .await;

    assert!(
        result.is_err(),
        "malformed filter_subject_json must be rejected"
    );
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.to_lowercase().contains("json")
            || err_str.contains("filter_subject_json"),
        "error must mention 'json' or 'filter_subject_json', got: {err_str}"
    );

    Ok(())
}
