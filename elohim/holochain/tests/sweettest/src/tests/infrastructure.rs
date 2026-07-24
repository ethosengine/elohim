//! @dna-scope: infrastructure
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
//! 4. `get_all_doorways_lists_every_registered_doorway` — federation peer
//!    discovery: the list-all sentinel anchor surfaces every peer (not just
//!    self) across conductors, with latest-wins dedup on re-registration.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors},
    fixtures::network_seed,
};
use holo_hash::ActionHash;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::sleep;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    identity_root: Option<String>,
    #[serde(default)]
    endpoints: Vec<DoorwayEndpoint>,
    capabilities_json: String,
    reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bandwidth_mbps: Option<u32>,
    version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DoorwayEndpoint {
    service: String,
    url: String,
    priority: u16,
    ttl_secs: u32,
}

/// Mirrors `infrastructure_types::DoorwayRegistration`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DoorwayRegistration {
    id: String,
    url: String,
    identity_root: String,
    signing_key: String,
    endpoints: Vec<DoorwayEndpoint>,
    record_serial: u64,
    record_signature: Vec<u8>,
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
        identity_root: None,
        endpoints: Vec::new(),
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
        .call(
            &cell.zome("infrastructure"),
            "register_doorway",
            alpha_doorway_input(),
        )
        .await;

    assert_eq!(
        output.doorway.operator_agent,
        agent.to_string(),
        "operator_agent should auto-bind to the calling agent"
    );
    assert_eq!(output.doorway.id, "alpha");
    assert_eq!(output.doorway.url, "https://alpha.example");
    assert_eq!(output.doorway.identity_root, agent.to_string());
    assert_eq!(output.doorway.signing_key, agent.to_string());
    assert_eq!(
        output.doorway.endpoints,
        vec![DoorwayEndpoint {
            service: "gateway".to_string(),
            url: "https://alpha.example".to_string(),
            priority: 0,
            ttl_secs: 300,
        }]
    );
    assert_eq!(output.doorway.record_signature.len(), 64);
    assert!(output.doorway.record_serial > 0);
    assert_eq!(output.doorway.tier, "Emerging");

    // Retrieve by id — should round-trip successfully
    let fetched: Option<DoorwayOutput> = conductor
        .call(
            &cell.zome("infrastructure"),
            "get_doorway_by_id",
            "alpha".to_string(),
        )
        .await;
    assert!(
        fetched.is_some(),
        "get_doorway_by_id should return the registered doorway"
    );
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
        .call(
            &cell1.zome("infrastructure"),
            "register_doorway",
            alpha_doorway_input(),
        )
        .await;

    // Poll c2 until the doorway anchor link is gossipped, or panic on deadline.
    // Gossip quiescence is not a hard guarantee for link traversal; see
    // tests/node_registry.rs admission_visible_across_agents for rationale.
    let deadline = Instant::now() + Duration::from_secs(30);
    let zome = cell2.zome("infrastructure");
    let fetched: DoorwayOutput = loop {
        let result: Option<DoorwayOutput> = c2
            .call(&zome, "get_doorway_by_id", "alpha".to_string())
            .await;
        if let Some(out) = result {
            break out;
        }
        if Instant::now() >= deadline {
            panic!("a2 should be able to read a1's doorway within 30s of DHT propagation");
        }
        sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(
        fetched.doorway.operator_agent,
        a1.to_string(),
        "operator_agent must still be a1 after cross-agent read"
    );

    // a2 attempts to update a1's doorway — must be rejected
    let hijack = RegisterDoorwayInput {
        id: "alpha".to_string(),
        url: "https://hijacked.example".to_string(),
        identity_root: None,
        endpoints: Vec::new(),
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

/// Federation peer discovery. Two operators each self-register a distinct doorway;
/// `get_all_doorways` must enumerate BOTH from a single conductor once the DHT has
/// gossipped the list-all anchor links. This is the read side that replaces the
/// self-only stub in doorway-service — the regression that made every doorway's
/// `/threshold/doorways` selector list only itself. Also asserts latest-wins dedup:
/// re-registering the same id collapses to one (newest) record, never duplicates.
#[tokio::test(flavor = "multi_thread")]
async fn get_all_doorways_lists_every_registered_doorway() -> Result<()> {
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

    // a1 registers "alpha"
    let _: DoorwayOutput = c1
        .call(
            &cell1.zome("infrastructure"),
            "register_doorway",
            alpha_doorway_input(),
        )
        .await;

    // a2 registers "beta"
    let beta = RegisterDoorwayInput {
        id: "beta".to_string(),
        url: "https://beta.example".to_string(),
        identity_root: None,
        endpoints: Vec::new(),
        capabilities_json: "{}".to_string(),
        reach: "regional".to_string(),
        region: Some("us-east".to_string()),
        bandwidth_mbps: Some(50),
        version: "0.1.0".to_string(),
    };
    let _: DoorwayOutput = c2
        .call(&cell2.zome("infrastructure"), "register_doorway", beta)
        .await;

    // From a2's conductor, poll get_all_doorways until BOTH are visible — a2's own
    // "beta" is local, a1's "alpha" must gossip in. Discovering a peer you did not
    // already know the id of is the whole point of the list-all anchor.
    let deadline = Instant::now() + Duration::from_secs(30);
    let zome2 = cell2.zome("infrastructure");
    loop {
        let all: Vec<DoorwayOutput> = c2.call(&zome2, "get_all_doorways", ()).await;
        let ids: Vec<&str> = all.iter().map(|d| d.doorway.id.as_str()).collect();
        if ids.contains(&"alpha") && ids.contains(&"beta") {
            break;
        }
        if Instant::now() >= deadline {
            panic!("get_all_doorways should list both doorways within 30s; saw: {ids:?}");
        }
        sleep(Duration::from_millis(100)).await;
    }

    // Latest-wins dedup: a1 re-registers "alpha" with a new url (re-registration
    // appends a second link + entry off the sentinel). Read locally from c1 (no
    // gossip needed) and assert alpha collapses to exactly one record, newest url.
    let mut alpha_v2 = alpha_doorway_input();
    alpha_v2.url = "https://alpha-v2.example".to_string();
    let _: DoorwayOutput = c1
        .call(&cell1.zome("infrastructure"), "register_doorway", alpha_v2)
        .await;

    let zome1 = cell1.zome("infrastructure");
    let all_local: Vec<DoorwayOutput> = c1.call(&zome1, "get_all_doorways", ()).await;
    let alpha: Vec<&DoorwayOutput> = all_local
        .iter()
        .filter(|d| d.doorway.id == "alpha")
        .collect();
    assert_eq!(
        alpha.len(),
        1,
        "alpha must appear exactly once after re-registration (latest-wins dedup)"
    );
    assert_eq!(
        alpha[0].doorway.url, "https://alpha-v2.example",
        "dedup must keep the NEWEST registration"
    );

    Ok(())
}
