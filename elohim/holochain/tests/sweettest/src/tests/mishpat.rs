//! Sweettest baseline — mishpat (governance).
//!
//! Baseline (§2.1.3): bootstrap-steward creates a governance entry; a second
//! agent reads it; validation rejects unauthorized bootstrap-only creates.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors},
    fixtures::network_seed,
};
use qahal_types::{CreateProposalInput, ProposalOutput};
use std::time::{Duration, Instant};
use tokio::time::sleep;

const DNA: &str = "mishpat";

#[tokio::test(flavor = "multi_thread")]
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

/// A proposal created by the bootstrap steward (a1) is visible to a second
/// agent (a2) after DHT gossip settles.
///
/// Minimum viable `CreateProposalInput`:
/// - `proposal_type` must be one of `["sense-check", "consent", "consensus", "supermajority"]`
/// - `status` must be one of `["draft", "discussion", "voting", "decided", "dismissed"]`
/// - `id` is optional — the coordinator generates one from `sys_time` when absent
/// - JSON blob fields (`amendments_json`, `voting_config_json`, `current_votes_json`,
///   `metadata_json`) must be present but the validator only checks `id`, `proposal_type`,
///   and `status` — empty-array / empty-object strings are sufficient.
/// - No referential IDs (no `precedent_id`, no `challenge_id`) are required.
#[tokio::test(flavor = "multi_thread")]
async fn proposal_round_trips_across_agents() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;

    // Both agents install the same DNA (same network seed, same bootstrap steward = a1).
    let dna = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;
    let app1 = c1
        .setup_app_for_agent("mishpat-app", a1.clone(), &[dna.clone()])
        .await?;
    let app2 = c2
        .setup_app_for_agent("mishpat-app", a2.clone(), &[dna])
        .await?;

    let cell1 = app1.cells().first().unwrap().clone();
    let cell2 = app2.cells().first().unwrap().clone();

    // a1 (bootstrap steward) creates a proposal.
    // Provides an explicit id so the assertion below can match it precisely.
    let proposal_input = CreateProposalInput {
        id: Some("sweettest-prop-1".to_string()),
        title: "Test governance proposal".to_string(),
        proposal_type: "sense-check".to_string(),
        description: "A baseline proposal created by the bootstrap steward in sweettests."
            .to_string(),
        proposer_id: a1.to_string(),
        proposer_name: "Bootstrap Steward".to_string(),
        rationale: "Verifying proposal round-trip across agents.".to_string(),
        status: "draft".to_string(),
        phase: "open".to_string(),
        amendments_json: "[]".to_string(),
        voting_config_json: "{}".to_string(),
        current_votes_json: "{}".to_string(),
        outcome_json: None,
        related_entity_type: None,
        related_entity_id: None,
        metadata_json: "{}".to_string(),
    };

    let created: ProposalOutput = c1
        .call(&cell1.zome("mishpat"), "create_proposal", proposal_input)
        .await;

    assert_eq!(
        created.proposal.id, "sweettest-prop-1",
        "created proposal should use the supplied id"
    );
    assert_eq!(created.proposal.proposal_type, "sense-check");
    assert_eq!(created.proposal.status, "draft");

    // Poll c2 until the proposal anchor link is gossipped, or panic on deadline.
    // Gossip quiescence is not a hard guarantee for link traversal; see
    // tests/node_registry.rs admission_visible_across_agents for rationale.
    let deadline = Instant::now() + Duration::from_secs(30);
    let zome = cell2.zome("mishpat");
    let fetched: ProposalOutput = loop {
        let result: Option<ProposalOutput> = c2
            .call(&zome, "get_proposal_by_id", "sweettest-prop-1".to_string())
            .await;
        if let Some(out) = result {
            break out;
        }
        if Instant::now() >= deadline {
            panic!("second agent did not see steward's proposal within 30s");
        }
        sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(
        fetched.proposal.id, created.proposal.id,
        "fetched proposal id must match created proposal id"
    );
    assert_eq!(fetched.proposal.title, created.proposal.title);
    assert_eq!(fetched.proposal.proposal_type, created.proposal.proposal_type);

    Ok(())
}
