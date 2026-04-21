//! Sweettest baseline — lamad (content).
//!
//! Baseline (§2.1.3): publish a content entry with CID; retrieve via CID;
//! link types resolve across agents.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};

const DNA: &str = "lamad";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn content_store_is_reachable() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("lamad-app", agent.clone(), &[dna])
        .await?;
    let _cell = app.cells().first().unwrap().clone();
    Ok(())
}

// TODO: publish + retrieve + link-type scenarios, tied to content_store
// coordinator externs.
