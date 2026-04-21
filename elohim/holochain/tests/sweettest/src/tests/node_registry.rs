//! Sweettest baseline — node-registry (node admission).
//!
//! Baseline (§2.1.3): node admission flow — bootstrap-steward admits a second
//! node; admission record is visible.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};

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

// TODO: register_node flow + admission-record visibility across agents.
