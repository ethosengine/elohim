//! Sweettest baseline — infrastructure (doorway federation).
//!
//! Infrastructure is federation-native (no bootstrap steward — see Wave 1
//! execution plan §1.2 Q2 resolution). Baseline exercises self-registration
//! of a doorway via its own agent key, which is the core flow this DNA
//! enables.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};

const DNA: &str = "infrastructure";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
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

// TODO: doorway self-registration scenario exercising the validation rule
// that operator_agent must equal entry author.
