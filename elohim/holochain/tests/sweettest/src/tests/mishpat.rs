//! Sweettest baseline — mishpat (governance).
//!
//! Baseline (§2.1.3): bootstrap-steward creates a governance entry; a second
//! agent reads it; validation rejects unauthorized bootstrap-only creates.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};

const DNA: &str = "mishpat";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn bootstrap_steward_is_configured() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("mishpat-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();

    let who: Option<holo_hash::AgentPubKey> = conductor
        .call(&cell.zome("mishpat"), "get_bootstrap_steward", ())
        .await;
    assert_eq!(who, Some(agent));
    Ok(())
}

// TODO: governance proposal create/read/validation scenarios, driven by the
// actual mishpat coordinator surface.
