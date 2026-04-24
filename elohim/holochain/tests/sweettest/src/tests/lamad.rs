//! Sweettest baseline — lamad (content_store coordinator).
//!
//! Scenarios (§2.3):
//! 1. `content_store_is_reachable`            — DNA installs without error.
//! 2. `content_publishes_and_retrieves_by_id` — single agent create/get round-trip.
//! 3. `content_visible_across_agents`         — cross-agent DHT visibility after settle.
//!
//! The coordinator zome is `content_store` (per dna/elohim/dna.yaml).
//! DNA artifact: `dna/elohim/workdir/lamad.dna`.
//! All tests carry `#[ignore = "requires packed DNA artifact"]` — remove per
//! DNA after Jenkins-green proof (Wave 1 ignore-flip stage).

use anyhow::Result;
use holo_hash::ActionHash;
use serde::{Deserialize, Serialize};

use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors},
    fixtures::network_seed,
    mirrors::settle_dht,
};

const DNA: &str = "lamad";

// ---------------------------------------------------------------------------
// Local wire-type mirrors
//
// `lamad-types` is not a Cargo dep of this standalone workspace. We mirror
// only the fields that the tests actually send or receive, matching the
// MessagePack field names used by the coordinator exactly.
// ---------------------------------------------------------------------------

/// Mirrors `lamad_types::CreateContentInput` — minimum viable fields for
/// `Content::validate()` (healing.rs lines 50-96):
///   - id, title, content_type  non-empty strings
///   - content_type             must be in ALL_CONTENT_TYPES  ("concept" qualifies)
///   - content_format           must be in ALL_CONTENT_FORMATS ("markdown" qualifies)
///   - reach                    must be in ALL_REACH_LEVELS    ("commons" qualifies)
///   - metadata_json            stored verbatim; "{}" is valid
///   - related_node_ids, tags   default empty vecs; validated as "no empty IDs"
///
/// Optional blob fields (blob_cid, content_hash, content_size_bytes) are
/// omitted — erasure/chunked-blob round-trips are out of scope for Wave 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateContentInput {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    pub content: String,
    pub content_format: String,
    pub reach: String,
    pub metadata_json: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub related_node_ids: Vec<String>,
}

/// Mirrors the `content` field of `lamad_types::ContentOutput` — fields
/// we assert in tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WireContent {
    pub id: String,
    pub title: String,
    pub content_type: String,
    pub content_format: String,
    pub reach: String,
}

/// Mirrors `lamad_types::ContentOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContentOutput {
    pub action_hash: ActionHash,
    pub content: WireContent,
}

/// Mirrors `lamad_types::QueryByIdInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryByIdInput {
    pub id: String,
}

// ---------------------------------------------------------------------------
// Fixture helper
// ---------------------------------------------------------------------------

fn test_content(id: &str) -> CreateContentInput {
    CreateContentInput {
        id: id.to_string(),
        content_type: "concept".to_string(),
        title: format!("Test Concept {id}"),
        description: "A sweettest fixture concept entry.".to_string(),
        content: "# Fixture\n\nMinimal lamad content entry used in sweettests.".to_string(),
        content_format: "markdown".to_string(),
        reach: "commons".to_string(),
        metadata_json: "{}".to_string(),
        tags: vec![],
        related_node_ids: vec![],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

/// Single-agent round-trip: create content, retrieve by id, retrieve by
/// ActionHash.  Validates that the create → IdToContent link → get_content_by_id
/// → get_content path through content_store is wired end-to-end without
/// needing a second conductor or DHT gossip.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn content_publishes_and_retrieves_by_id() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("lamad-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    // 1. Create content — coordinator validates type/format/reach enums
    let input = test_content("test-1");
    let output: ContentOutput = conductor
        .call(&cell.zome("content_store"), "create_content", input)
        .await;

    assert_eq!(output.content.id, "test-1");
    assert_eq!(output.content.title, "Test Concept test-1");
    assert_eq!(output.content.content_type, "concept");
    assert_eq!(output.content.content_format, "markdown");
    assert_eq!(output.content.reach, "commons");

    let created_hash: ActionHash = output.action_hash.clone();

    // 2. Retrieve by string id (follows IdToContent link)
    let by_id: Option<ContentOutput> = conductor
        .call(
            &cell.zome("content_store"),
            "get_content_by_id",
            QueryByIdInput { id: "test-1".to_string() },
        )
        .await;

    let by_id = by_id.expect("get_content_by_id returned None");
    assert_eq!(by_id.content.id, "test-1");
    assert_eq!(by_id.content.title, "Test Concept test-1");

    // 3. Retrieve by ActionHash (direct record lookup)
    let by_hash: Option<ContentOutput> = conductor
        .call(&cell.zome("content_store"), "get_content", created_hash)
        .await;

    let by_hash = by_hash.expect("get_content returned None");
    assert_eq!(by_hash.content.id, "test-1");
    assert_eq!(by_hash.content.content_type, "concept");

    Ok(())
}

/// Cross-agent visibility: agent A creates content, DHT settles via
/// `mirrors::settle_dht`, agent B retrieves it via get_content_by_id.
/// Validates that the IdToContent link gossips correctly to a second
/// conductor sharing the same network seed.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact"]
async fn content_visible_across_agents() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;
    let seed = network_seed(DNA);

    // Both agents share the same network seed → same DHT space.
    // a1 is the bootstrap steward (progenitor_pubkey in DNA properties).
    let dna_file = load_dna(DNA, &seed, Some(a1.clone())).await?;

    let app1 = c1
        .setup_app_for_agent("lamad-app", a1.clone(), &[dna_file.clone()])
        .await?;
    let app2 = c2
        .setup_app_for_agent("lamad-app", a2.clone(), &[dna_file])
        .await?;

    let cell1 = app1.cells().first().unwrap().clone();
    let cell2 = app2.cells().first().unwrap().clone();

    // Agent 1 creates content
    let input = test_content("cross-agent-1");
    let output: ContentOutput = c1
        .call(&cell1.zome("content_store"), "create_content", input)
        .await;
    assert_eq!(output.content.id, "cross-agent-1");

    // Allow DHT gossip to propagate between in-process conductors
    settle_dht(&[&cell1, &cell2]).await;

    // Agent 2 retrieves the content by id
    let result: Option<ContentOutput> = c2
        .call(
            &cell2.zome("content_store"),
            "get_content_by_id",
            QueryByIdInput { id: "cross-agent-1".to_string() },
        )
        .await;

    let retrieved = result.expect("agent 2 could not see content created by agent 1");
    assert_eq!(retrieved.content.id, "cross-agent-1");
    assert_eq!(retrieved.content.title, "Test Concept cross-agent-1");
    assert_eq!(retrieved.content.content_type, "concept");

    Ok(())
}
