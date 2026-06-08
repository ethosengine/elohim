//! @dna-scope: lamad
//! Sweettest probe — verify `create_rea_economic_event` coordinator fires
//! in-process (Slice-2a Task 1).
//!
//! Purpose: before building a storage emit-service on top of
//! `create_rea_economic_event`, prove this coordinator is NOT similarly inert
//! to `emit_reciprocity_imbalance` (which Sprint-3 flagged as log-only
//! "conductor bridge pending"). The probe anchors a Commitment, then emits an
//! EconomicEvent that references it, and asserts a valid 32-byte ActionHash is
//! returned. If the call errors or returns an empty hash, we stop (BLOCKED) and
//! do not build on sand.
//!
//! ## Why single-conductor is enough here
//!
//! Task 1 is about the coordinator CONTRACT, not DHT replication. A single
//! conductor in-process is sufficient to prove the function executes and
//! returns a valid anchor. Two-conductor replication is Task 6.
//!
//! ## Action selection: "use"
//!
//! "republish-epr" has schema-level metadata_json validation in the
//! coordinator (validates against republish-epr.schema.json); "use" is a
//! high-stakes verb that passes through the stake-class gate without requiring
//! observation_refs and carries no metadata_json schema check. This keeps the
//! probe minimal and clean — we are testing the coordinator plumbing, not
//! republish-epr business logic.
//!
//! ## bounded_by via metadata_json
//!
//! The `CreateReaEconomicEventInput` struct carries no `bounded_by` field.
//! The convention is that bounded-by semantics are expressed in `metadata_json`
//! as `{"bounded_by": "<commitment-id>"}`. The coordinator stores metadata_json
//! verbatim and creates fulfillment links via the `fulfills` Vec (which takes
//! commitment IDs to look up by ID anchor). We use `fulfills` for the structural
//! link and `metadata_json` to carry the human-readable bounded_by annotation.
//!
//! `#[ignore]` — requires packed lamad.dna artifact from Jenkins pipeline.
//! Local invocation:
//! `just pack && cargo test --test rea_event_emit_graduation_test -- --ignored`

use anyhow::Result;
use elohim_sweettest::common::{conductors::load_dna, fixtures::network_seed};
use holo_hash::{ActionHash, EntryHash};
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

use elohim_sweettest::common::conductors::single_agent_conductor;

const DNA: &str = "lamad";
const ZOME: &str = "content_store";

// ---------------------------------------------------------------------------
// Local mirror types — field names MUST match the coordinator structs exactly.
// Avoid path-dep on WASM coordinator crate to keep the test surface lean.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_beginning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub in_scope_of: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
}

/// Mirror of `shefa_types::CommitmentWire` — only fields the probe asserts.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommitmentWire {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub metadata_json: String,
}

/// Mirror of `shefa_types::ReaCommitmentOutput`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct ReaCommitmentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub commitment: CommitmentWire,
}

/// Mirror of `shefa_types::CreateReaEconomicEventInput`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateReaEconomicEventInput {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_classified_as: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_quantity_unit: Option<String>,
    pub has_point_in_time: String,
    #[serde(default)]
    pub fulfills: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realization_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lamad_event_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_refs: Option<Vec<String>>,
}

/// Mirror of `shefa_types::EconomicEvent` — only fields the probe asserts.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EconomicEvent {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub metadata_json: String,
    #[serde(default)]
    pub has_point_in_time: String,
    #[serde(default)]
    pub state: String,
    // Remaining fields skipped — not asserted in this probe.
    #[serde(default)]
    pub resource_classified_as_json: String,
    #[serde(default)]
    pub fulfills_json: String,
    #[serde(default)]
    pub satisfies_json: String,
    #[serde(default)]
    pub in_scope_of_json: String,
    #[serde(default)]
    pub created_at: String,
}

/// Mirror of `shefa_types::ReaEconomicEventOutput`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct ReaEconomicEventOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub event: EconomicEvent,
    pub fulfillment_links: Vec<ActionHash>,
}

// ---------------------------------------------------------------------------
// Probe test
// ---------------------------------------------------------------------------

/// Prove `create_rea_economic_event` fires in-process and returns a valid
/// 32-byte ActionHash anchor.
///
/// Steps:
/// 1. Author a project-epr REA Commitment (gives a real commitment ID the
///    event can reference via `fulfills` + `metadata_json.bounded_by`).
/// 2. Emit an EconomicEvent with action="use" (high-stakes verb; no
///    observation_refs required, no metadata_json schema check) referencing
///    the commitment via `fulfills` and `metadata_json`.
/// 3. Assert the returned ActionHash is 32 bytes — the coordinator executed
///    and anchored the event on the DHT source chain.
///
/// If this test fails with a WasmError the coordinator is BLOCKED for
/// bounded-event emission and the slice must stop here.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed lamad.dna artifact from Jenkins pipeline"]
async fn create_rea_economic_event_coordinator_fires_with_bounded_by() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;

    let app = conductor
        .setup_app_for_agent("elohim-app-probe", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();
    let zome = cell.zome(ZOME);

    // -----------------------------------------------------------------------
    // Step 1: author a Commitment so the event has a real `bounded_by` target.
    // -----------------------------------------------------------------------
    let commitment_id = "probe-commitment-provides-content|probe-agent".to_string();
    let commitment_input = CreateReaCommitmentInput {
        id: commitment_id.clone(),
        action: "project-epr".to_string(),
        provider: "doorway:probe-doorway".to_string(),
        receiver: "epr:probe-content".to_string(),
        resource_classified_as: Vec::new(),
        resource_quantity_value: None,
        resource_quantity_unit: None,
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_beginning: None,
        has_end: None,
        due: None,
        clause_of: None,
        in_scope_of: vec!["doorway:probe-doorway|epr:probe-content".to_string()],
        note: None,
        metadata_json: None,
    };

    let commitment_out: ReaCommitmentOutput = conductor
        .call(&zome, "create_rea_commitment", commitment_input)
        .await;

    assert_eq!(
        commitment_out.commitment.id, commitment_id,
        "commitment id must round-trip"
    );
    assert_eq!(
        commitment_out.action_hash.get_raw_32().len(),
        32,
        "create_rea_commitment must return a 32-byte ActionHash"
    );

    // -----------------------------------------------------------------------
    // Step 2: emit an EconomicEvent bounded by the commitment above.
    //
    // action="use" — a high-stakes verb. Passes the stake-class gate without
    // observation_refs; no metadata_json schema validation. We carry the
    // bounded_by relationship two ways:
    //   - `fulfills`: structural DHT link (Event → Commitment via ID anchor)
    //   - `metadata_json`: human-readable `{"bounded_by": "<id>"}` annotation
    // -----------------------------------------------------------------------
    let event_id = format!("probe-event-bounded-by|{}", commitment_id);
    let bounded_by_json = format!(r#"{{"bounded_by":"{}"}}"#, commitment_id);

    let event_input = CreateReaEconomicEventInput {
        id: event_id.clone(),
        action: "use".to_string(),
        provider: "agent:probe".to_string(),
        receiver: "epr:probe-content".to_string(),
        resource_classified_as: Vec::new(),
        resource_quantity_value: None,
        resource_quantity_unit: None,
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_point_in_time: "2026-06-08T00:00:00Z".to_string(),
        fulfills: vec![commitment_id.clone()],
        realization_of: None,
        lamad_event_type: None,
        note: None,
        metadata_json: Some(bounded_by_json.clone()),
        observation_refs: None,
    };

    // `call` panics on WasmError — any coordinator rejection surfaces here.
    let event_out: ReaEconomicEventOutput = conductor
        .call(&zome, "create_rea_economic_event", event_input)
        .await;

    // -----------------------------------------------------------------------
    // Step 3: assertions — coordinator returned a valid 32-byte anchor.
    // -----------------------------------------------------------------------
    assert_eq!(
        event_out.action_hash.get_raw_32().len(),
        32,
        "create_rea_economic_event must return a 32-byte ActionHash \
         (coordinator fired and anchored the event on the DHT source chain)"
    );
    assert_eq!(
        event_out.entry_hash.get_raw_32().len(),
        32,
        "entry_hash must also be 32 bytes (entry was created)"
    );
    assert_eq!(
        event_out.event.id, event_id,
        "event id must round-trip through the coordinator"
    );
    assert_eq!(
        event_out.event.action, "use",
        "action must round-trip as 'use'"
    );
    assert_eq!(
        event_out.event.metadata_json, bounded_by_json,
        "metadata_json carrying bounded_by must be stored verbatim"
    );
    // fulfillment_links: the coordinator attempts to resolve commitment_id
    // via IdToCommitment link. Since we created the commitment in the same
    // source chain, the link should be present and the fulfillment resolved.
    assert_eq!(
        event_out.fulfillment_links.len(),
        1,
        "one fulfillment link must be created: Event → Commitment via ID anchor"
    );

    Ok(())
}
