//! @dna-scope: lamad
//! Sweettest — REA commitment DHT replication (substrate-rea-replication-fix Task 9).
//!
//! Regression seatbelt for the substrate-correct write path landed in
//! Tasks 4-6 + 6.5 + 6.6. Validates the core promise: a project-epr REA
//! commitment created by Agent A on Conductor 1 reaches Agent B on
//! Conductor 2 via Holochain DHT gossip. Without this, every peer in the
//! alpha cluster sees only the subset of project-epr commitments that
//! happened to be POSTed against its own doorway — the split-doorway
//! divergence captured in the 2026-05-26 sprint-result's Gap D.
//!
//! What this test does NOT cover: the elohim-storage side (HTTP → conductor
//! → SQL projection with dht_anchor_hash). That layer has its own unit
//! tests in `elohim/elohim-storage/src/rea_projection.rs::tests`
//! (DNA-wire-shape decode) and `services/conductor_writes.rs::tests`
//! (msgpack roundtrip). Together they form a two-leg seatbelt: the storage
//! tests prove the on-the-wire shape decodes; this sweettest proves the
//! DHT propagates that shape to peer B.
//!
//! The conductors start genuinely isolated, then exchange peer info after the
//! write. Peer B polls `get_rea_commitment` within a bounded window so the test
//! exercises the late-join DHT-fetch path directly.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, two_agent_conductors_isolated},
    fixtures::network_seed,
};
use holo_hash::{ActionHash, EntryHash};
use holochain::sweettest::{await_consistency, SweetConductor};
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DNA: &str = "lamad";
const ZOME: &str = "content_store";

/// Per-invocation unique commitment id. nextest retries run in the same
/// process, where kitsune2's mem-bootstrap store is process-global; a fixed id
/// can therefore rejoin residue from an earlier attempt and self-poison the
/// retry.
fn unique_id(base: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before UNIX_EPOCH")
        .as_nanos();
    format!("{base}-{nanos}")
}

// ---------------------------------------------------------------------------
// Local mirrors — must match `shefa_types::CreateReaCommitmentInput` etc.
// exactly. The sweettest crate avoids path-dep on coordinator crates to keep
// the test surface lean.
// ---------------------------------------------------------------------------

/// Mirror of `shefa_types::CreateReaCommitmentInput`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateReaCommitmentInput {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_classified_as: Vec<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f64>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f64>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_beginning: Option<String>,
    #[serde(default)]
    pub has_end: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub in_scope_of: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

/// Mirror of `shefa_types::Commitment` (the wire shape returned by
/// `commitment_to_wire`). Only the fields the assertions touch are typed
/// strictly; the rest pass through serde defaults.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CommitmentWire {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_inventoried_as: Option<String>,
    #[serde(default)]
    pub resource_classified_as: Vec<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f64>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f64>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_point_in_time: Option<String>,
    #[serde(default)]
    pub has_beginning: Option<String>,
    #[serde(default)]
    pub has_end: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub agreed_in: Option<String>,
    #[serde(default)]
    pub input_of: Option<String>,
    #[serde(default)]
    pub output_of: Option<String>,
    #[serde(default)]
    pub satisfies: Option<String>,
    #[serde(default)]
    pub in_scope_of: Vec<String>,
    #[serde(default)]
    pub finished: bool,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata_json: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

/// Mirror of `shefa_types::ReaCommitmentOutput`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct ReaCommitmentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub commitment: CommitmentWire,
}

// ---------------------------------------------------------------------------
// Scenario: Alice creates a project-epr commitment; Bob reads it after gossip.
// ---------------------------------------------------------------------------

/// The core regression seatbelt. Alice (Conductor 1) creates a project-epr
/// REA commitment via the substrate-correct path (content_store coordinator
/// → DHT entry → post-commit signal). Bob (Conductor 2) calls
/// `get_rea_commitment` and must see the same commitment. If this test
/// regresses, alpha's split-doorway divergence will return.
///
/// The DNA pipeline packs `lamad.dna` before running the sweettest shard, so
/// this test must remain unignored: the shard does not pass
/// `--run-ignored all`. Local invocation after packing:
/// `cargo test --test rea_commitment_replication -- --nocapture`.
#[tokio::test(flavor = "multi_thread")]
async fn project_epr_commitment_replicates_to_peer_b() -> Result<()> {
    let [(mut ca, a1), (mut cb, a2)] = two_agent_conductors_isolated().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;

    let app_a = ca
        .setup_app_for_agent("elohim-app-alice", a1.clone(), &[dna.clone()])
        .await?;
    let app_b = cb
        .setup_app_for_agent("elohim-app-bob", a2.clone(), &[dna])
        .await?;
    let cell_a = app_a.cells().first().expect("cell A").clone();
    let cell_b = app_b.cells().first().expect("cell B").clone();

    // --- Alice writes the commitment. ---
    let commitment_id = unique_id("test-project-epr-doorway:test|epr:lamad");
    let input = CreateReaCommitmentInput {
        id: commitment_id.clone(),
        action: "project-epr".to_string(),
        provider: "doorway:test-doorway".to_string(),
        receiver: "epr:lamad-spa".to_string(),
        resource_classified_as: Vec::new(),
        resource_quantity_value: None,
        resource_quantity_unit: None,
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_beginning: None,
        has_end: None,
        due: None,
        clause_of: None,
        in_scope_of: vec!["doorway:test-doorway|epr:lamad-spa".to_string()],
        note: None,
        metadata_json: None,
    };

    let alice_output: ReaCommitmentOutput = ca
        .call(&cell_a.zome(ZOME), "create_rea_commitment", input)
        .await;

    assert_eq!(alice_output.commitment.id, commitment_id);
    assert_eq!(alice_output.commitment.action, "project-epr");

    // --- Connect Bob only after Alice has authored the commitment. ---
    // Bootstrap is disabled for both conductors, so this is a genuine
    // late-join fetch rather than an accidentally pre-gossiped read through
    // kitsune2's process-global mem-bootstrap store.
    tokio::time::timeout(Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&ca, &cb]).await {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;

    // --- Wait for op convergence before asserting the READ. ---
    // Restored 2026-07-25 (operator-approved). Commit 837b772c9 switched this
    // test to isolated conductors, un-`#[ignore]`d it, AND dropped this barrier
    // in one move — collapsing the green sibling idiom (60s consistency + poll,
    // see tests/lamad.rs:542/576/680/951) into a bare 60s poll. Without it the
    // test cannot distinguish "peer B cannot RESOLVE the commitment" from "the
    // ops have not finished integrating yet", and in a debug-profile conductor
    // with a 3.5MB DNA (75 entry types x 225 link types validated op-by-op)
    // late-join convergence lands near the minute mark — two CONTENT sweettests
    // that are green in CI also red in this container on the shorter budget.
    // The barrier is what makes a failure of the poll below MEAN something: it
    // is the difference between a substrate red and a stopwatch.
    await_consistency(60, [&cell_a, &cell_b])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after create_rea_commitment: {e}"))?;

    // --- Bob resolves the commitment within a bounded window. ---
    // Polling the coordinator read directly exercises the production
    // `get_links` + `get(record)` DHT-fetch path. A transient None (link not
    // visible yet) or zome error (link visible before its target record) is
    // retried; failing the outer bound is the actionable notary-authority red.
    let zome_b = cell_b.zome(ZOME);
    let fetch_id = commitment_id.clone();
    let bob_output = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let read: holochain::conductor::api::error::ConductorApiResult<
                Option<ReaCommitmentOutput>,
            > = cb
                .call_fallible(&zome_b, "get_rea_commitment", fetch_id.clone())
                .await;

            if let Ok(Some(output)) = read {
                break output;
            }

            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "Bob could not retrieve Alice's REA commitment {commitment_id} via \
             get_rea_commitment within 60s after peer exchange"
        )
    })?;
    assert_eq!(
        bob_output.commitment.id, commitment_id,
        "Bob's commitment id must match Alice's"
    );
    assert_eq!(
        bob_output.commitment.action, "project-epr",
        "action must round-trip"
    );
    assert_eq!(
        bob_output.commitment.provider, "doorway:test-doorway",
        "provider must round-trip"
    );
    assert_eq!(
        bob_output.action_hash, alice_output.action_hash,
        "ActionHash must be byte-identical across peers"
    );

    Ok(())
}
