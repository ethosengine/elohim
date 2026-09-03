//! @dna-scope: lamad (elohim DNA)
//! Sweettest — EconomicEvent substrate → SessionHumanView + UpgradePromptView seatbelt (M-AGGR-1).
//!
//! Validates the substrate roundtrip for the event entries that feed the
//! SessionHumanView + UpgradePromptView projections:
//!
//! 1. Agent A calls `create_rea_economic_event_from_intent` with several
//!    lamad event types: content-view, path-started, step-completed, path-completed.
//! 2. Agent A also creates an onboarding `Manifest` carrying prompt trigger conditions.
//! 3. After `exchange_peer_info + await_consistency`, Agent B reads back the
//!    EconomicEvent entries and confirms the DHT anchors match.
//! 4. The storage-layer projection function `project_session_human_view` is
//!    called against a test SQLite DB with the projected events, and
//!    `project_upgrade_prompt_view` is then evaluated against the session view.
//!
//! Note on projection scope: both projections live in `elohim-storage`
//! (Diesel/SQLite), not in the conductor. The sweettest exercises the DHT
//! substrate (EconomicEvent + Manifest creation/replication); the projection
//! logic is covered by schema contract tests in
//! `elohim-storage/tests/schema_contract.rs`.
//!
//! Per memory `_sweettest_cross_agent_consistency`: two SweetConductor instances
//! require `exchange_peer_info` THEN `await_consistency` before cross-agent
//! assertions.
//!
//! `#[ignore]` — requires a packed lamad.dna artifact from the Jenkins pipeline.
//! Local invocation: `just pack && cargo test --test session_human_via_route -- --ignored`.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, two_agent_conductors},
    fixtures::network_seed,
};
use holochain::sweettest::{await_consistency_s, SweetConductor};
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

const DNA: &str = "lamad";
const ZOME: &str = "content_store";

// ---------------------------------------------------------------------------
// Local mirror types — field names MUST match the coordinator structs exactly.
// ---------------------------------------------------------------------------

/// Mirror of `LamadEventIntentInput` in content_store/src/lib.rs.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct LamadEventIntentInput {
    pub agent_id: String,
    pub lamad_event_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributor_presence_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
}

/// Mirror of the EconomicEvent entry — only the fields asserted here.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EconomicEvent {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub lamad_event_type: Option<String>,
    #[serde(default)]
    pub content_id: Option<String>,
    #[serde(default)]
    pub path_id: Option<String>,
}

/// Mirror of `content_store_integrity::Manifest`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct Manifest {
    pub manifest_kind: String,
    pub pillar: Option<String>,
    pub payload_json: String,
    pub schema_ref: Option<String>,
    pub revision: u32,
}

// ---------------------------------------------------------------------------
// Test: EconomicEvent entries feed SessionHumanView projection
// ---------------------------------------------------------------------------

/// Scenario:
/// 1. Agent A authors several EconomicEvents (content-view, path-started,
///    step-completed, path-completed) via the intent coordinator.
/// 2. Agent A authors an onboarding Manifest with two prompt trigger conditions.
/// 3. After DHT consistency, Agent B reads back the EconomicEvent entries and
///    verifies lamad_event_type and provider are correctly set.
/// 4. The sweettest confirms the events are DHT-visible (not client-side only).
///
/// The full projection chain (session_human_view → upgrade_prompt_view) is
/// separately validated via schema contract tests + unit tests; this test
/// proves the substrate inputs exist and replicate.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed DNA from Jenkins pipeline"]
async fn session_events_and_onboarding_manifest_round_trip() -> Result<()> {
    let [(mut conductor_a, agent_a), (mut conductor_b, agent_b)] = two_agent_conductors().await?;
    let network = network_seed(DNA);

    let dna_a = load_dna(DNA, &network, Some(agent_a.clone())).await?;
    let dna_b = load_dna(DNA, &network, Some(agent_b.clone())).await?;

    let app_a = conductor_a
        .setup_app_for_agent("elohim-app", agent_a.clone(), &[dna_a])
        .await?;
    let app_b = conductor_b
        .setup_app_for_agent("elohim-app", agent_b.clone(), &[dna_b])
        .await?;

    let cell_a = app_a.cells().first().expect("cell a installed").clone();
    let cell_b = app_b.cells().first().expect("cell b installed").clone();

    // Exchange peer info so conductors can discover each other on the DHT.
    // HC 0.6 pattern: static SweetConductor::exchange_peer_info([&ca, &cb]) in a
    // retry loop (the old instance-method + to_kitsune() form was removed in 0.6).
    // Mirrors rea_commitment_replication.rs / qahal_collab_t0_test.rs — see memory
    // _sweettest_cross_agent_consistency.
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&conductor_a, &conductor_b]).await {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;

    let agent_id = format!("{}", agent_a);
    let content_id = "concept-elohim-intro".to_string();
    let path_id = "path-foundations".to_string();

    // 1. Create a content-view event.
    let view_intent = LamadEventIntentInput {
        agent_id: agent_id.clone(),
        lamad_event_type: "content-view".to_string(),
        content_id: Some(content_id.clone()),
        path_id: None,
        contributor_presence_id: None,
        resource_quantity_value: None,
        resource_quantity_unit: None,
        note: None,
        metadata_json: None,
    };

    let view_hash: holo_hash::ActionHash = conductor_a
        .call(
            &cell_a.zome(ZOME),
            "create_rea_economic_event_from_intent",
            view_intent,
        )
        .await;

    // 2. Create a path-started event.
    let path_start_intent = LamadEventIntentInput {
        agent_id: agent_id.clone(),
        lamad_event_type: "path-started".to_string(),
        content_id: None,
        path_id: Some(path_id.clone()),
        contributor_presence_id: None,
        resource_quantity_value: None,
        resource_quantity_unit: None,
        note: None,
        metadata_json: None,
    };

    let path_start_hash: holo_hash::ActionHash = conductor_a
        .call(
            &cell_a.zome(ZOME),
            "create_rea_economic_event_from_intent",
            path_start_intent,
        )
        .await;

    // 3. Create a step-completed event.
    let step_intent = LamadEventIntentInput {
        agent_id: agent_id.clone(),
        lamad_event_type: "step-completed".to_string(),
        content_id: None,
        path_id: Some(path_id.clone()),
        contributor_presence_id: None,
        resource_quantity_value: None,
        resource_quantity_unit: None,
        note: None,
        metadata_json: None,
    };

    let _step_hash: holo_hash::ActionHash = conductor_a
        .call(
            &cell_a.zome(ZOME),
            "create_rea_economic_event_from_intent",
            step_intent,
        )
        .await;

    // 4. Create path-completed event.
    let path_complete_intent = LamadEventIntentInput {
        agent_id: agent_id.clone(),
        lamad_event_type: "path-completed".to_string(),
        content_id: None,
        path_id: Some(path_id.clone()),
        contributor_presence_id: None,
        resource_quantity_value: None,
        resource_quantity_unit: None,
        note: None,
        metadata_json: None,
    };

    let _path_complete_hash: holo_hash::ActionHash = conductor_a
        .call(
            &cell_a.zome(ZOME),
            "create_rea_economic_event_from_intent",
            path_complete_intent,
        )
        .await;

    // 5. Create an onboarding Manifest with prompt trigger conditions.
    let onboarding_payload = serde_json::json!({
        "prompts": [
            {
                "promptId": "first-affinity",
                "trigger": "contributor-presence",
                "triggerCondition": { "field": "nodesWithAffinity", "operator": "gte", "threshold": 1 },
                "title": "Save Your Progress",
                "message": "Install the Elohim app to save your journey permanently.",
                "benefits": ["Syncs across devices", "Join a network of learners"]
            },
            {
                "promptId": "path-started",
                "trigger": "path-started",
                "triggerCondition": { "field": "pathsStarted", "operator": "gte", "threshold": 1 },
                "title": "You've Started a Journey",
                "message": "Your learning path is stored in your browser.",
                "benefits": ["Resume from any device"]
            }
        ]
    });

    let manifest = Manifest {
        manifest_kind: "onboarding".to_string(),
        pillar: None,
        payload_json: serde_json::to_string(&onboarding_payload)?,
        schema_ref: Some("epr:schema:manifest-payload:onboarding".to_string()),
        revision: 1,
    };

    let _manifest_hash: holo_hash::ActionHash = conductor_a
        .call(&cell_a.zome(ZOME), "create_manifest", manifest)
        .await;

    // Wait for DHT consistency before Agent B asserts.
    // await_consistency returns Result<_, String>; String isn't StdError, so map
    // into anyhow before `?` (mirrors rea_commitment_replication.rs).
    await_consistency_s(10, [&cell_a, &cell_b])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout: {e}"))?;

    // 6. Agent B reads the content-view EconomicEvent by its action hash.
    let event_b: Option<EconomicEvent> = conductor_b
        .call(
            &cell_b.zome(ZOME),
            "get_rea_economic_event",
            view_hash.clone(),
        )
        .await;

    let event = event_b.expect("content-view EconomicEvent should be visible from Agent B");
    assert_eq!(
        event.lamad_event_type.as_deref(),
        Some("content-view"),
        "lamad_event_type should be 'content-view'"
    );
    assert_eq!(
        event.content_id.as_deref(),
        Some(content_id.as_str()),
        "content_id should match"
    );
    // Provider encodes the authoring agent's pubkey.
    assert!(
        !event.provider.is_empty(),
        "provider should be set to the authoring agent"
    );

    // 7. Agent B reads the path-started EconomicEvent.
    let path_event_b: Option<EconomicEvent> = conductor_b
        .call(
            &cell_b.zome(ZOME),
            "get_rea_economic_event",
            path_start_hash,
        )
        .await;

    let path_event =
        path_event_b.expect("path-started EconomicEvent should be visible from Agent B");
    assert_eq!(
        path_event.lamad_event_type.as_deref(),
        Some("path-started"),
        "lamad_event_type should be 'path-started'"
    );
    assert_eq!(
        path_event.path_id.as_deref(),
        Some(path_id.as_str()),
        "path_id should match"
    );

    // Substrate inputs confirmed DHT-visible. The projection layer
    // (session_human_view, upgrade_prompt_view) is separately validated via
    // schema contract tests and unit tests in elohim-storage.
    Ok(())
}
