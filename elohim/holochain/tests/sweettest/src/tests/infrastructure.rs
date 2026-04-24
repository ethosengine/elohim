//! Sweettest baseline — infrastructure (doorway federation).
//!
//! Infrastructure is federation-native (no bootstrap steward — see Wave 1
//! execution plan §1.2 Q2 resolution). Baseline exercises self-registration
//! of a doorway via its own agent key, which is the core flow this DNA
//! enables.
//!
//! Scenarios:
//! 1. `infrastructure_installs_without_bootstrap_steward` — DNA loads, cell is live.
//! 2. `doorway_self_registers` — register_doorway binds operator_agent to caller.
//! 3. `doorway_visible_across_agents_and_operator_only_can_update` — DHT
//!    propagation + operator-only enforcement via coordinator gate.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors},
    fixtures::network_seed,
    mirrors,
};
use holo_hash::ActionHash;
use serde::{Deserialize, Serialize};

const DNA: &str = "infrastructure";

// ---------------------------------------------------------------------------
// Local mirror types — match the wire format of infrastructure_types without
// taking a direct crate dependency (the sweettest workspace is kept lean).
// Fields must match `infrastructure_types::RegisterDoorwayInput` exactly so
// that MessagePack serialization is identical.
// ---------------------------------------------------------------------------

/// Mirrors `infrastructure_types::RegisterDoorwayInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegisterDoorwayInput {
    id: String,
    url: String,
    capabilities_json: String,
    reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bandwidth_mbps: Option<u32>,
    version: String,
}

/// Mirrors `infrastructure_types::DoorwayRegistration`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DoorwayRegistration {
    id: String,
    url: String,
    operator_agent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    operator_human: Option<String>,
    capabilities_json: String,
    reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bandwidth_mbps: Option<u32>,
    version: String,
    tier: String,
    registered_at: String,
    updated_at: String,
}

/// Mirrors `infrastructure_types::DoorwayOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DoorwayOutput {
    action_hash: ActionHash,
    doorway: DoorwayRegistration,
}

// ---------------------------------------------------------------------------
// Fixture helper
// ---------------------------------------------------------------------------

fn alpha_doorway_input() -> RegisterDoorwayInput {
    RegisterDoorwayInput {
        id: "alpha".to_string(),
        url: "https://alpha.example".to_string(),
        capabilities_json: "{}".to_string(),
        reach: "regional".to_string(),
        region: Some("us-west".to_string()),
        bandwidth_mbps: Some(100),
        version: "0.1.0".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn infrastructure_installs_without_bootstrap_steward() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    // Infrastructure takes no bootstrap-steward modifier.
    let dna = load_dna(DNA, &network_seed(DNA), None).await?;
    let app = conductor
        .setup_app_for_agent("infrastructure-app", agent, &[dna])
        .await?;
    let _cell = app.cells().first().unwrap().clone();
    Ok(())
}

/// A doorway operator self-registers: the coordinator binds operator_agent to
/// the calling agent's key, so no caller can impersonate another.
#[tokio::test(flavor = "multi_thread")]
async fn doorway_self_registers() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), None).await?;
    let app = conductor
        .setup_app_for_agent("infrastructure-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();

    let output: DoorwayOutput = conductor
        .call(&cell.zome("infrastructure"), "register_doorway", alpha_doorway_input())
        .await;

    assert_eq!(
        output.doorway.operator_agent,
        agent.to_string(),
        "operator_agent should auto-bind to the calling agent"
    );
    assert_eq!(output.doorway.id, "alpha");
    assert_eq!(output.doorway.url, "https://alpha.example");
    assert_eq!(output.doorway.tier, "Emerging");

    // Retrieve by id — should round-trip successfully
    let fetched: Option<DoorwayOutput> = conductor
        .call(&cell.zome("infrastructure"), "get_doorway_by_id", "alpha".to_string())
        .await;
    assert!(fetched.is_some(), "get_doorway_by_id should return the registered doorway");
    let fetched = fetched.unwrap();
    assert_eq!(fetched.doorway.operator_agent, agent.to_string());

    Ok(())
}

/// Agent A registers a doorway. After DHT propagation, agent B can read it.
/// Agent B then attempts to update the doorway — the coordinator rejects the
/// attempt because B is not the operator.
///
/// This exercises the self-registration enforcement rule: the coordinator
/// checks `operator_agent == caller` before allowing any mutation.
#[tokio::test(flavor = "multi_thread")]
async fn doorway_visible_across_agents_and_operator_only_can_update() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;
    let dna = load_dna(DNA, &network_seed(DNA), None).await?;
    let app1 = c1
        .setup_app_for_agent("infrastructure-app", a1.clone(), &[dna.clone()])
        .await?;
    let app2 = c2
        .setup_app_for_agent("infrastructure-app", a2.clone(), &[dna])
        .await?;
    let cell1 = app1.cells().first().unwrap().clone();
    let cell2 = app2.cells().first().unwrap().clone();

    // a1 self-registers a doorway
    let _: DoorwayOutput = c1
        .call(&cell1.zome("infrastructure"), "register_doorway", alpha_doorway_input())
        .await;

    // Allow DHT gossip to settle so the entry is visible to c2
    mirrors::settle_dht(&[&cell1, &cell2]).await;

    // a2 can read a1's doorway
    let fetched: Option<DoorwayOutput> = c2
        .call(&cell2.zome("infrastructure"), "get_doorway_by_id", "alpha".to_string())
        .await;
    assert!(
        fetched.is_some(),
        "a2 should be able to read a1's doorway after DHT propagation"
    );
    let fetched = fetched.unwrap();
    assert_eq!(
        fetched.doorway.operator_agent,
        a1.to_string(),
        "operator_agent must still be a1 after cross-agent read"
    );

    // a2 attempts to update a1's doorway — must be rejected
    let hijack = RegisterDoorwayInput {
        id: "alpha".to_string(),
        url: "https://hijacked.example".to_string(),
        capabilities_json: "{}".to_string(),
        reach: "regional".to_string(),
        region: Some("us-east".to_string()),
        bandwidth_mbps: None,
        version: "0.1.0".to_string(),
    };
    let result: holochain::conductor::api::error::ConductorApiResult<DoorwayOutput> = c2
        .call_fallible(&cell2.zome("infrastructure"), "update_doorway", hijack)
        .await;
    assert!(
        result.is_err(),
        "non-operator (a2) must not be able to update a1's doorway"
    );

    Ok(())
}
