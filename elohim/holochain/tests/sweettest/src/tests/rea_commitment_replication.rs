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
//! Per memory `_sweettest_cross_agent_consistency`: requires
//! `exchange_peer_info` THEN `await_consistency` after the write.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, two_agent_conductors},
    fixtures::network_seed,
};
use holo_hash::{ActionHash, EntryHash};
use holochain::sweettest::{await_consistency, SweetConductor};
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

const DNA: &str = "lamad";
const ZOME: &str = "content_store";

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
/// `#[ignore]` — requires a packed lamad.dna artifact from Jenkins; the
/// pipeline's pack-then-test stage runs this. Local invocation:
/// `just pack && cargo test --test rea_commitment_replication -- --ignored`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn project_epr_commitment_replicates_to_peer_b() -> Result<()> {
    let [(mut ca, a1), (mut cb, a2)] = two_agent_conductors().await?;
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
    let commitment_id = "test-project-epr-doorway:test|epr:lamad".to_string();
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

    // --- Exchange peer info then await DHT consistency. ---
    // The exchange_peer_info dance is required because two SweetConductor
    // instances default to isolated kitsune nets; without exchanging, bob's
    // DHT view never learns alice's agent exists. Pattern lifted from
    // qahal_collab_t0_test.rs:563 — see memory `_sweettest_cross_agent_consistency`.
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&ca, &cb]).await {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;

    await_consistency(60, [&cell_a, &cell_b])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after create_rea_commitment: {e}"))?;

    // --- Bob reads the commitment via the zome's get_rea_commitment. ---
    // Substrate proof: if DHT gossip propagated the Commitment entry +
    // its id_anchor link, Bob's get_rea_commitment returns Some(...) with
    // the same id. This is the regression seatbelt — pre-Task-6.5/6.6 the
    // entry would land in alice's chain but bob would see None.
    let bob_view: Option<ReaCommitmentOutput> = cb
        .call(
            &cell_b.zome(ZOME),
            "get_rea_commitment",
            commitment_id.clone(),
        )
        .await;

    let bob_output = bob_view.ok_or_else(|| {
        anyhow::anyhow!(
            "Bob's DHT view must contain Alice's commitment after exchange_peer_info + \
             await_consistency. None here means DHT gossip did NOT propagate — the substrate \
             replication gap the 2026-05-26 plan exists to close."
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
