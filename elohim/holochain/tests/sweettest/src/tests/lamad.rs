//! @dna-scope: lamad
//! Sweettest baseline — lamad (content_store coordinator).
//!
//! Scenarios (§2.3):
//! 1. `content_store_is_reachable`            — DNA installs without error.
//! 2. `content_publishes_and_retrieves_by_id` — single agent create/get round-trip.
//! 3. `content_visible_across_agents`         — cross-agent DHT visibility after settle.
//! 4. `declare_head_notarizes_and_supersedes` — notary-authority Leg 1: author-
//!    filtered HEAD election, author gate, and republish (§5.6 REQ-F13).
//!
//! The coordinator zome is `content_store` (per dna/elohim/dna.yaml).
//! DNA artifact: `dna/elohim/workdir/lamad.dna`.

use anyhow::Result;
use holo_hash::{ActionHash, ActionHashB64, AgentPubKey};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::sleep;

use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors},
    fixtures::network_seed,
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

/// Mirrors the subset of `lamad_types::UpdateContentInput` these tests send.
/// Coordinator marks the omitted fields `#[serde(default)]`, so title-only
/// patches deserialize cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateContentInput {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Mirrors `content_store::DeclareContentHeadInput` (notary-authority Leg 1).
/// The zome field is `Option<ActionHashB64>` (string-wire-safe); this mirror
/// carries `Option<String>` and passes the canonical base64 form so the
/// MessagePack wire shape matches the storage facade exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeclareContentHeadInput {
    pub id: String,
    pub head_action_hash: Option<String>,
}

/// Mirrors the fields of `content_store::ContentHeadOutput` these tests assert.
/// The wire struct carries more (entry_hash, declared_at); holochain serializes
/// structs as MessagePack maps, so this subset deserializes — extra keys are
/// ignored (same pattern as `ContentOutput`/`WireContent` above).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContentHeadOutput {
    pub content_id: String,
    pub head_action_hash: ActionHash,
    pub author: AgentPubKey,
    pub supersedes: Option<ActionHash>,
    pub content: WireContent,
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
            QueryByIdInput {
                id: "test-1".to_string(),
            },
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

/// Cross-agent visibility: agent A creates content; agent B polls
/// `get_content_by_id` until the IdToContent link gossips to its conductor.
/// Validates that the IdToContent link gossips correctly to a second
/// conductor sharing the same network seed.
#[tokio::test(flavor = "multi_thread")]
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

    // Poll c2 until the IdToContent link is gossipped, or panic on deadline.
    // Single-shot reads after a fixed sleep race the link-traversal path; see
    // tests/node_registry.rs admission_visible_across_agents for rationale.
    let deadline = Instant::now() + Duration::from_secs(30);
    let zome = cell2.zome("content_store");
    let retrieved: ContentOutput = loop {
        let result: Option<ContentOutput> = c2
            .call(
                &zome,
                "get_content_by_id",
                QueryByIdInput {
                    id: "cross-agent-1".to_string(),
                },
            )
            .await;
        if let Some(out) = result {
            break out;
        }
        if Instant::now() >= deadline {
            panic!("agent 2 could not see content 'cross-agent-1' within 30s");
        }
        sleep(Duration::from_millis(100)).await;
    };

    assert_eq!(retrieved.content.id, "cross-agent-1");
    assert_eq!(retrieved.content.title, "Test Concept cross-agent-1");
    assert_eq!(retrieved.content.content_type, "concept");

    Ok(())
}

/// Notary-authority Leg 1 (Plan C1): the author declares the version-DAG HEAD,
/// and HEAD election is AUTHOR-FILTERED so a non-author's newer write cannot
/// hijack it (closes the LWW vulnerability, VERDICT L2 #3).
///
/// Proves, across two conductors on one DHT:
///   a. A creates + updates → `resolve_content_head` returns the update as
///      head, superseding the create.
///   b. B (non-author) → `declare_content_head` errors with "not the author".
///   c. B forges a newer `update_content` → `resolve_content_head` STILL
///      returns A's update as head (author-filtered election beats recency).
///   d. A declares the ORIGINAL create as head → head moves to a NEW republish
///      action whose Content equals the original create's content.
#[tokio::test(flavor = "multi_thread")]
async fn declare_head_notarizes_and_supersedes() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;
    let seed = network_seed(DNA);
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
    let zome1 = cell1.zome("content_store");
    let zome2 = cell2.zome("content_store");

    // --- (a) A creates, then updates; head = the update, superseding the create.
    let created: ContentOutput = c1
        .call(&zome1, "create_content", test_content("notary-1"))
        .await;
    let create_action = created.action_hash.clone();
    let original_title = created.content.title.clone(); // "Test Concept notary-1"

    let updated: ContentOutput = c1
        .call(
            &zome1,
            "update_content",
            UpdateContentInput {
                id: "notary-1".to_string(),
                title: Some("Revised by A".to_string()),
            },
        )
        .await;
    let a_update_action = updated.action_hash.clone();
    assert_ne!(a_update_action, create_action);

    let head_a: Option<ContentHeadOutput> = c1
        .call(&zome1, "resolve_content_head", "notary-1".to_string())
        .await;
    let head_a = head_a.expect("resolve_content_head returned None on author conductor");
    assert_eq!(
        head_a.head_action_hash, a_update_action,
        "head is A's update"
    );
    assert_eq!(head_a.author, a1, "author is A (root Create author)");
    assert_eq!(
        head_a.supersedes,
        Some(create_action.clone()),
        "update supersedes the create"
    );

    // --- (b) B must first see the content, then be rejected by the author gate.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let seen: Option<ContentHeadOutput> = c2
            .call(&zome2, "resolve_content_head", "notary-1".to_string())
            .await;
        if seen.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            panic!("agent 2 could not see content 'notary-1' within 30s");
        }
        sleep(Duration::from_millis(100)).await;
    }

    let gate: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_content_head",
            DeclareContentHeadInput {
                id: "notary-1".to_string(),
                head_action_hash: None,
            },
        )
        .await;
    assert!(gate.is_err(), "non-author declare must be rejected");
    let err_str = format!("{:?}", gate.unwrap_err());
    assert!(
        err_str.contains("not the author"),
        "author-gate error must contain 'not the author'; got: {err_str}"
    );

    // --- (c) B forges a newer update; A's update still wins the election.
    let forged: ContentOutput = c2
        .call(
            &zome2,
            "update_content",
            UpdateContentInput {
                id: "notary-1".to_string(),
                title: Some("Forged by B".to_string()),
            },
        )
        .await;
    let b_update_action = forged.action_hash.clone();

    // Poll c2 (which now holds B's forged update locally) until A's update has
    // gossiped in, then assert it — not B's newer write — is the head.
    let deadline = Instant::now() + Duration::from_secs(30);
    let head_c: ContentHeadOutput = loop {
        let h: Option<ContentHeadOutput> = c2
            .call(&zome2, "resolve_content_head", "notary-1".to_string())
            .await;
        if let Some(h) = h {
            // The forged (recency-winning) write must never elect as head.
            assert_ne!(
                h.head_action_hash, b_update_action,
                "non-author forged update must never win HEAD election"
            );
            if h.head_action_hash == a_update_action {
                break h;
            }
        }
        if Instant::now() >= deadline {
            panic!("A's update did not converge as head on c2 within 30s");
        }
        sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(head_c.author, a1, "head author remains A after B's forgery");

    // --- (d) A republishes the ORIGINAL create as the head.
    let redeclared: ContentHeadOutput = c1
        .call(
            &zome1,
            "declare_content_head",
            DeclareContentHeadInput {
                id: "notary-1".to_string(),
                head_action_hash: Some(ActionHashB64::from(create_action.clone()).to_string()),
            },
        )
        .await;
    // A brand-new republish action, distinct from every prior version.
    assert_ne!(redeclared.head_action_hash, create_action);
    assert_ne!(redeclared.head_action_hash, a_update_action);
    // Content is restored to the original create (not A's or B's revision).
    assert_eq!(
        redeclared.content.title, original_title,
        "republish restores the original create's content"
    );
    assert_ne!(redeclared.content.title, "Revised by A");
    // It supersedes the prior author-authored head (A's update).
    assert_eq!(redeclared.supersedes, Some(a_update_action.clone()));
    assert_eq!(redeclared.author, a1);

    Ok(())
}
