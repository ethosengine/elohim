//! Sweettest baseline — node-registry (node admission).
//!
//! Baseline (§2.4): node admission flow — bootstrap-steward admits a second
//! node; admission record is visible.
//!
//! Three scenarios:
//! 1. `node_registry_has_bootstrap_steward` — pre-existing baseline.
//! 2. `register_node_round_trips` — single-agent register + query by region.
//! 3. `admission_visible_across_agents` — cross-agent DHT propagation.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors},
    fixtures::{network_seed, node_registration, NodeRegistration},
    mirrors::settle_dht,
};
use holo_hash::ActionHash;

const DNA: &str = "node_registry";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn node_registry_has_bootstrap_steward() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("node_registry-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();
    let who: Option<holo_hash::AgentPubKey> = conductor
        .call(&cell.zome("node_registry_coordinator"), "get_bootstrap_steward", ())
        .await;
    assert_eq!(who, Some(agent));
    Ok(())
}

/// Single-agent: register a node then query it back by region.
///
/// Exercises `register_node` → `get_nodes_by_region`. Confirms the index
/// links created by `register_node` (region anchor → NodeRegistration) are
/// traversable in the same conductor.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn register_node_round_trips() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("node_registry-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();

    let registration = node_registration("alpha", &agent);
    let region = registration.region.clone();

    // Register the node — expect an ActionHash back.
    let _ah: ActionHash = conductor
        .call(
            &cell.zome("node_registry_coordinator"),
            "register_node",
            registration,
        )
        .await;

    // Query by region — node must appear.
    let nodes: Vec<NodeRegistration> = conductor
        .call(
            &cell.zome("node_registry_coordinator"),
            "get_nodes_by_region",
            region,
        )
        .await;

    assert!(
        nodes.iter().any(|n| n.node_id == "alpha"),
        "expected node 'alpha' in region result, got {:?}",
        nodes.iter().map(|n| &n.node_id).collect::<Vec<_>>()
    );
    Ok(())
}

/// Cross-agent: steward registers a node; a second agent can query it after
/// DHT gossip settles.
///
/// Exercises the DHT propagation path: `register_node` writes links anchored
/// by region; those links are gossipped to all peers on the same network seed.
/// After `settle_dht` the second conductor should be able to traverse them via
/// `get_nodes_by_region`.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn admission_visible_across_agents() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;

    // Both agents use the same network seed so they share the same DHT.
    let dna1 = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;
    let dna2 = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;

    let app1 = c1
        .setup_app_for_agent("node_registry-app", a1.clone(), &[dna1])
        .await?;
    let app2 = c2
        .setup_app_for_agent("node_registry-app", a2.clone(), &[dna2])
        .await?;

    let cell1 = app1.cells().first().unwrap().clone();
    let cell2 = app2.cells().first().unwrap().clone();

    // a1 registers a node.
    let registration = node_registration("alpha", &a1);
    let region = registration.region.clone();

    let _ah: ActionHash = c1
        .call(
            &cell1.zome("node_registry_coordinator"),
            "register_node",
            registration,
        )
        .await;

    // Allow DHT gossip to propagate to the second conductor.
    settle_dht(&[&cell1, &cell2]).await;

    // a2 queries the same region — must see a1's registration.
    let nodes: Vec<NodeRegistration> = c2
        .call(
            &cell2.zome("node_registry_coordinator"),
            "get_nodes_by_region",
            region,
        )
        .await;

    assert!(
        nodes.iter().any(|n| n.node_id == "alpha"),
        "second agent should see node 'alpha' after DHT settle, got {:?}",
        nodes.iter().map(|n| &n.node_id).collect::<Vec<_>>()
    );
    Ok(())
}
