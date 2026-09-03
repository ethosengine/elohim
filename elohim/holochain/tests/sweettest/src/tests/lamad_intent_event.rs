//! @dna-scope: lamad
//! Sweettest — intent-driven EconomicEvent composition seatbelt (M-REA-1).
//!
//! Validates the end-to-end path for `create_rea_economic_event_from_intent`:
//!
//! 1. Agent A calls `create_rea_economic_event_from_intent` with a sample
//!    `LamadEventIntentInput` (e.g. `content-view`).
//! 2. The coordinator maps the intent to `(action="use", provider=agentId,
//!    receiver=contentId)` using `intent_to_rea_triple`.
//! 3. After `exchange_peer_info + await_consistency`, Agent B reads the event
//!    via `get_rea_economic_event` and asserts action/provider/receiver.
//!
//! This is the regression seatbelt for the PROTOCOL_EVENT_MAPPINGS table that
//! exists in two places: `intent_to_rea_triple` (zome) and
//! `compose_event_from_intent` (elohim-storage). If either drifts the test
//! will catch it when the packed DNA is exercised.
//!
//! Per memory `_sweettest_cross_agent_consistency`: two SweetConductor instances
//! require `exchange_peer_info` THEN `await_consistency` before cross-agent
//! assertions.
//!
//! `#[ignore]` — requires a packed lamad.dna artifact from the Jenkins pipeline.
//! Local invocation: `just pack && cargo test --test lamad_intent_event -- --ignored`.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, two_agent_conductors},
    fixtures::network_seed,
};
use holo_hash::{ActionHash, EntryHash};
use holochain::sweettest::{await_consistency_s, SweetConductor};
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

const DNA: &str = "lamad";
const ZOME: &str = "content_store";

// ---------------------------------------------------------------------------
// Local mirror types — field names MUST match the coordinator structs exactly.
// Avoid path-dep on WASM coordinator crate (keeps test surface lean).
// ---------------------------------------------------------------------------

/// Mirror of `LamadEventIntentInput` in content_store/src/lib.rs.
/// Optional fields carry `#[serde(default)]` so missing keys do not fail
/// MessagePack deserialization.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct LamadEventIntentInput {
    pub agent_id: String,
    pub lamad_event_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributor_presence_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
}

/// Mirror of `shefa_types::EconomicEvent` — only the fields asserted here.
/// All fields carry `#[serde(default)]` so extra / missing keys don't fail.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EconomicEvent {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_inventoried_as: Option<String>,
    #[serde(default)]
    pub to_resource_inventoried_as: Option<String>,
    pub resource_classified_as_json: String,
    #[serde(default)]
    pub resource_quantity_value: Option<f64>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f64>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    pub has_point_in_time: String,
    #[serde(default)]
    pub has_duration: Option<String>,
    #[serde(default)]
    pub input_of: Option<String>,
    #[serde(default)]
    pub output_of: Option<String>,
    pub fulfills_json: String,
    #[serde(default)]
    pub realization_of: Option<String>,
    pub satisfies_json: String,
    pub in_scope_of_json: String,
    #[serde(default)]
    pub note: Option<String>,
    pub state: String,
    #[serde(default)]
    pub triggered_by: Option<String>,
    #[serde(default)]
    pub at_location: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub lamad_event_type: Option<String>,
    pub metadata_json: String,
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
// Helper
// ---------------------------------------------------------------------------

fn make_content_view_intent(agent_id: &str, content_id: &str) -> LamadEventIntentInput {
    LamadEventIntentInput {
        agent_id: agent_id.to_string(),
        lamad_event_type: "content-view".to_string(),
        content_id: Some(content_id.to_string()),
        path_id: None,
        contributor_presence_id: None,
        resource_quantity_value: None,
        resource_quantity_unit: None,
        note: None,
        metadata_json: None,
    }
}

fn make_path_complete_intent(agent_id: &str, path_id: &str) -> LamadEventIntentInput {
    LamadEventIntentInput {
        agent_id: agent_id.to_string(),
        lamad_event_type: "path-complete".to_string(),
        content_id: None,
        path_id: Some(path_id.to_string()),
        contributor_presence_id: None,
        resource_quantity_value: None,
        resource_quantity_unit: None,
        note: None,
        metadata_json: None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// `content-view` intent composes to action="use", receiver=contentId on DHT.
///
/// Verifies the basic intent→REA mapping works end-to-end through the zome
/// coordinator and that the DHT entry is readable by a peer after gossip.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn content_view_intent_composes_use_action_and_replicates() -> Result<()> {
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

    let agent_id = "agent-alice-test";
    let content_id = "content-lamad-intro";

    // --- Agent A creates the EconomicEvent from an intent. ---
    let intent = make_content_view_intent(agent_id, content_id);
    let alice_output: ReaEconomicEventOutput = ca
        .call(
            &cell_a.zome(ZOME),
            "create_rea_economic_event_from_intent",
            intent,
        )
        .await;

    // Verify composition happened correctly on Alice's side.
    assert_eq!(
        alice_output.event.action, "use",
        "content-view must map to action='use'"
    );
    assert_eq!(
        alice_output.event.provider, agent_id,
        "provider must be the initiating agent"
    );
    assert_eq!(
        alice_output.event.receiver, content_id,
        "content-view receiver must be contentId"
    );
    assert_eq!(
        alice_output.event.lamad_event_type,
        Some("content-view".to_string()),
        "lamadEventType must round-trip"
    );

    // --- Exchange peer info then await DHT consistency. ---
    // Pattern from rea_commitment_replication.rs; see memory
    // `_sweettest_cross_agent_consistency`.
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&ca, &cb]).await {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;

    await_consistency_s(60, [&cell_a, &cell_b])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout: {e}"))?;

    // --- Agent B reads the event via the existing get_rea_economic_event. ---
    let event_id = alice_output.event.id.clone();
    let bob_view: Option<ReaEconomicEventOutput> = cb
        .call(
            &cell_b.zome(ZOME),
            "get_rea_economic_event",
            event_id.clone(),
        )
        .await;

    let bob_output = bob_view.ok_or_else(|| {
        anyhow::anyhow!(
            "Bob must see Alice's EconomicEvent after exchange_peer_info + await_consistency. \
             None means DHT gossip did not propagate the intent-composed event — \
             the substrate replication path for create_rea_economic_event_from_intent is broken."
        )
    })?;

    assert_eq!(
        bob_output.event.id, event_id,
        "event id must match across peers"
    );
    assert_eq!(
        bob_output.event.action, "use",
        "Bob's view: action must be 'use'"
    );
    assert_eq!(
        bob_output.event.provider, agent_id,
        "Bob's view: provider must be the initiating agent"
    );
    assert_eq!(
        bob_output.event.receiver, content_id,
        "Bob's view: content-view receiver must be contentId"
    );
    assert_eq!(
        bob_output.action_hash, alice_output.action_hash,
        "ActionHash must be byte-identical across peers"
    );

    Ok(())
}

/// `path-complete` intent composes to action="produce", self-receiver.
///
/// Verifies the self-production pattern (provider == receiver == agentId)
/// works correctly. Single-conductor test — composition correctness doesn't
/// need cross-peer assertion.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn path_complete_intent_composes_produce_self_receiver() -> Result<()> {
    use elohim_sweettest::common::conductors::single_agent_conductor;

    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("elohim-app-single", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();
    let zome = cell.zome(ZOME);

    let agent_id = "agent-path-tester";
    let path_id = "path-elohim-foundations";
    let intent = make_path_complete_intent(agent_id, path_id);

    let output: ReaEconomicEventOutput = conductor
        .call(&zome, "create_rea_economic_event_from_intent", intent)
        .await;

    assert_eq!(
        output.event.action, "produce",
        "path-complete must map to action='produce'"
    );
    assert_eq!(
        output.event.provider, agent_id,
        "provider must be the initiating agent"
    );
    assert_eq!(
        output.event.receiver, agent_id,
        "path-complete is self-production: receiver == provider"
    );
    assert_eq!(
        output.event.lamad_event_type,
        Some("path-complete".to_string()),
        "lamadEventType must round-trip"
    );

    Ok(())
}

/// Unknown lamadEventType is rejected by the coordinator (not silently ignored).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn unknown_event_type_rejected_by_coordinator() -> Result<()> {
    use elohim_sweettest::common::conductors::single_agent_conductor;

    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("elohim-app-single-reject", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();
    let zome = cell.zome(ZOME);

    let bad_intent = LamadEventIntentInput {
        agent_id: "agent-test".to_string(),
        lamad_event_type: "not-a-real-event-type".to_string(),
        content_id: None,
        path_id: None,
        contributor_presence_id: None,
        resource_quantity_value: None,
        resource_quantity_unit: None,
        note: None,
        metadata_json: None,
    };

    let result: Result<ReaEconomicEventOutput, _> = conductor
        .call_fallible(&zome, "create_rea_economic_event_from_intent", bad_intent)
        .await;

    assert!(
        result.is_err(),
        "Unknown lamadEventType must be rejected by the coordinator — \
         not silently mapped to a default action"
    );

    Ok(())
}
