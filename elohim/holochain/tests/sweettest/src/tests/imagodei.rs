//! Sweettest baseline — imagodei (identity).
//!
//! Baseline scenarios (§2.1.3):
//! 1. Bootstrap-steward creates an identity record.
//! 2. Second agent joins and can see the identity via coordinator `get`.
//! 3. Validation rejects bootstrap-only actions from non-steward agents.
//!
//! These stubs compile against the sweettest harness but skip (`ignore`) when
//! the DNA artifact isn't built. Remove the `#[ignore]` once the holochain
//! pipeline packs the DNA before invoking the tests.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors, SweetAgents},
    fixtures::network_seed,
};

const DNA: &str = "imagodei";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn bootstrap_steward_is_identifiable() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("imagodei-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    let who: Option<holo_hash::AgentPubKey> = conductor
        .call(&cell.zome("imagodei"), "get_bootstrap_steward", ())
        .await;
    assert_eq!(who, Some(agent.clone()));

    let is_me: bool = conductor
        .call(&cell.zome("imagodei"), "is_bootstrap_steward", ())
        .await;
    assert!(is_me, "installing agent should be the bootstrap steward");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn second_agent_is_not_bootstrap_steward() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;
    let dna_file = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;

    let app1 = c1.setup_app_for_agent("imagodei", a1.clone(), &[dna_file.clone()]).await?;
    let app2 = c2.setup_app_for_agent("imagodei", a2.clone(), &[dna_file]).await?;

    let cell1 = app1.cells().first().unwrap().clone();
    let cell2 = app2.cells().first().unwrap().clone();

    let is_steward_1: bool = c1
        .call(&cell1.zome("imagodei"), "is_bootstrap_steward", ())
        .await;
    let is_steward_2: bool = c2
        .call(&cell2.zome("imagodei"), "is_bootstrap_steward", ())
        .await;

    assert!(is_steward_1);
    assert!(!is_steward_2, "second agent must NOT be bootstrap steward");
    let _ = SweetAgents::one; // keep import used even if helpers evolve
    Ok(())
}

// TODO (Wave 1 Sprint 1.A follow-up):
// - Exercise integrity validation that rejects bootstrap-only actions from
//   non-steward agents, once the validation rules for such actions exist.
