//! Sweettest — stake-class gate for create_rea_economic_event (§8.3).
//!
//! Verifies that:
//! 1. Operational verbs (e.g. `served-blob-summary`) without `observation_refs`
//!    are rejected by the coordinator.
//! 2. The same verb WITH non-empty `observation_refs` is accepted.
//! 3. High-stakes verbs (e.g. `transfer-standing`) without `observation_refs`
//!    are accepted (no gate applies).
//!
//! Uses the same lamad DNA (content_store coordinator) as the other shefa sweettests.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};
use serde::{Deserialize, Serialize};

const DNA: &str = "lamad";

// ---------------------------------------------------------------------------
// Local mirror types (no path-dep on WASM coordinator crate allowed here).
// Field names MUST match the coordinator / SDK structs exactly.
// ---------------------------------------------------------------------------

/// Mirror of `shefa_types::CreateReaEconomicEventInput`.
/// Only includes the fields exercised by these tests; optional fields with
/// `#[serde(default)]` in the real struct can be safely omitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateReaEconomicEventInput {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_classified_as: Vec<String>,
    pub has_point_in_time: String,
    #[serde(default)]
    pub fulfills: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_refs: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn make_event(id: &str, action: &str, observation_refs: Option<Vec<String>>) -> CreateReaEconomicEventInput {
    CreateReaEconomicEventInput {
        id: id.to_string(),
        action: action.to_string(),
        provider: "agent-a".to_string(),
        receiver: "agent-b".to_string(),
        resource_classified_as: vec![],
        has_point_in_time: "2026-05-12T00:00:00Z".to_string(),
        fulfills: vec![],
        observation_refs,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Operational verb without observation_refs is rejected (§8.3 gate active).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn operational_event_without_observation_refs_rejected() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("stake-gate-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();
    let zome = cell.zome("content_store");

    let input = make_event("op-no-refs-1", "served-blob-summary", None);
    let result: Result<serde_json::Value, _> = conductor
        .call_fallible(&zome, "create_rea_economic_event", input)
        .await;

    assert!(
        result.is_err(),
        "operational verb 'served-blob-summary' without observation_refs must be rejected"
    );
    Ok(())
}

/// Operational verb WITH non-empty observation_refs is accepted.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn operational_event_with_observation_refs_accepted() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("stake-gate-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();
    let zome = cell.zome("content_store");

    let refs = Some(vec!["iroh://agent:a@blake3:deadbeef#0".to_string()]);
    let input = make_event("op-with-refs-1", "served-blob-summary", refs);
    let result: Result<serde_json::Value, _> = conductor
        .call_fallible(&zome, "create_rea_economic_event", input)
        .await;

    assert!(
        result.is_ok(),
        "operational verb with observation_refs must succeed: {:?}",
        result
    );
    Ok(())
}

/// High-stakes verb without observation_refs is accepted (no gate).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn high_stakes_event_without_observation_refs_accepted() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("stake-gate-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();
    let zome = cell.zome("content_store");

    let input = make_event("high-no-refs-1", "transfer-standing", None);
    let result: Result<serde_json::Value, _> = conductor
        .call_fallible(&zome, "create_rea_economic_event", input)
        .await;

    assert!(
        result.is_ok(),
        "high-stakes verb without observation_refs must succeed: {:?}",
        result
    );
    Ok(())
}
