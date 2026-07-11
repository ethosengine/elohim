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
use holochain::sweettest::{await_consistency, SweetConductor};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::sleep;

use elohim_sweettest::common::{
    conductors::{
        load_dna, single_agent_conductor, two_agent_conductors, two_agent_conductors_isolated,
    },
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

/// Mirrors `content_store::DeclareCanonicalHeadInput` (notary-authority Model B,
/// cross-root convergence). `head_action_hash` is REQUIRED and passed as the
/// canonical base64 String form (matches the storage-facade wire shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeclareCanonicalHeadInput {
    pub id: String,
    pub head_action_hash: String,
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

/// Notary-authority Model B (Tier-1 steward-declared binding): the CROSS-ROOT
/// canonical-head selector converges genuinely-INDEPENDENT roots.
///
/// This is the `elohim-host-landing` case: two agents author the SAME id as
/// SEPARATE roots (different root authors, no supersedes edge between them), so
/// the author-filtered election in `resolve_content_head` structurally lets each
/// peer resolve its OWN root — divergence by construction, which the author- and
/// chain-membership-gated `declare_content_head` cannot heal.
///
/// Proves, across two conductors on one DHT:
///   a. Two independent roots for id "landing-x" (authored before gossip so
///      each `create_content` sees an empty local view and both succeed).
///   b. `declare_canonical_content_head` names agent B's root (the OTHER root,
///      authored by a different agent) as canonical — a CROSS-ROOT declaration
///      the author gate would refuse.
///   c. After the canonical link gossips, BOTH agents' `resolve_content_head`
///      return the SAME canonical head (== B's root, authored by a2) —
///      convergence.
///   d. An UNDECLARED id still resolves root-author-newest (behavior unchanged).
#[tokio::test(flavor = "multi_thread")]
async fn declare_canonical_head_converges_independent_roots() -> Result<()> {
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

    // --- (a) Two INDEPENDENT roots for one id. Authored BEFORE peer-info
    // exchange, so each conductor's `content_exists_by_id` sees an empty local
    // DHT view and both creates succeed — reproducing the deploy-time double
    // authoring (adam's f41d / matthew's 6af9) deterministically.
    let root_a: ContentOutput = c1
        .call(&zome1, "create_content", test_content("landing-x"))
        .await;
    let root_a_action = root_a.action_hash.clone();

    let root_b: ContentOutput = c2
        .call(&zome2, "create_content", test_content("landing-x"))
        .await;
    let root_b_action = root_b.action_hash.clone();
    assert_ne!(
        root_a_action, root_b_action,
        "the two independent roots must be distinct actions"
    );

    // Control id: single-author create + update on c1 (undeclared canonical).
    let _solo: ContentOutput = c1
        .call(&zome1, "create_content", test_content("solo-y"))
        .await;
    let solo_updated: ContentOutput = c1
        .call(
            &zome1,
            "update_content",
            UpdateContentInput {
                id: "solo-y".to_string(),
                title: Some("Solo revision".to_string()),
            },
        )
        .await;
    let solo_update_action = solo_updated.action_hash.clone();

    // --- Exchange peer info, then await DHT consistency so BOTH roots (and the
    // control id) gossip to both conductors.
    tokio::time::timeout(Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&c1, &c2]).await {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;
    await_consistency(60, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after independent roots: {e}"))?;

    // Both agents can now resolve SOME head for the id (both roots gossiped).
    let pre_a: Option<ContentHeadOutput> = c1
        .call(&zome1, "resolve_content_head", "landing-x".to_string())
        .await;
    assert!(pre_a.is_some(), "c1 must resolve a head pre-declaration");

    // --- (b) a1 (root A's author) declares agent B's root as canonical. This is
    // CROSS-ROOT: the target's author is a2, not the declarer — an act the
    // author-gated `declare_content_head` structurally cannot perform.
    assert_ne!(a1, a2, "the two agents must be distinct");
    let declared: ContentHeadOutput = c1
        .call(
            &zome1,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: "landing-x".to_string(),
                head_action_hash: ActionHashB64::from(root_b_action.clone()).to_string(),
            },
        )
        .await;
    assert_eq!(
        declared.head_action_hash, root_b_action,
        "declared canonical head is agent B's root"
    );
    assert_eq!(
        declared.author, a2,
        "canonical head is authored by the OTHER agent (cross-root)"
    );

    // Let the canonical-head link gossip to c2.
    await_consistency(60, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after canonical declare: {e}"))?;

    // --- (c) BOTH conductors resolve the SAME canonical head (== B's root).
    // Poll c2 until the canonical link has gossiped in and overrides its
    // own-root election.
    let deadline = Instant::now() + Duration::from_secs(30);
    let head_c2: ContentHeadOutput = loop {
        let h: Option<ContentHeadOutput> = c2
            .call(&zome2, "resolve_content_head", "landing-x".to_string())
            .await;
        if let Some(h) = h {
            if h.head_action_hash == root_b_action {
                break h;
            }
        }
        if Instant::now() >= deadline {
            panic!("canonical head did not converge on c2 within 30s");
        }
        sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(head_c2.author, a2, "c2 resolves the canonical author (a2)");

    let head_c1: Option<ContentHeadOutput> = c1
        .call(&zome1, "resolve_content_head", "landing-x".to_string())
        .await;
    let head_c1 = head_c1.expect("c1 must resolve the canonical head");
    assert_eq!(
        head_c1.head_action_hash, head_c2.head_action_hash,
        "CONVERGENCE: both peers resolve the identical canonical head"
    );
    assert_eq!(head_c1.head_action_hash, root_b_action);

    // --- (d) The UNDECLARED control id still resolves root-author-newest — the
    // canonical override falls through cleanly to the unchanged election.
    let solo_head: Option<ContentHeadOutput> = c1
        .call(&zome1, "resolve_content_head", "solo-y".to_string())
        .await;
    let solo_head = solo_head.expect("undeclared id must still resolve a head");
    assert_eq!(
        solo_head.head_action_hash, solo_update_action,
        "undeclared id resolves the root author's newest version (unchanged)"
    );
    assert_eq!(
        solo_head.author, a1,
        "undeclared head author is the root author"
    );

    Ok(())
}

/// Staging-tier marker + earned-head guard (notary-authority safety): the
/// god-mode-open scaffold declare can only ever set/replace a STAGING canonical
/// — it can neither clobber nor impersonate an EARNED head. Proves:
///   b. after a1 (bootstrap steward / progenitor) declares an EARNED canonical
///      for an id, a scaffold `declare_canonical_content_head` is REFUSED with
///      "earned head is protected".
///   c. scaffold-over-scaffold still works (newest STAGING declaration wins).
///   + the earned tier is progenitor-gated: a non-steward (a2) is refused.
/// (Cross-root convergence (a) and undeclared-fallthrough (d) are covered by
/// `declare_canonical_head_converges_independent_roots`.)
#[tokio::test(flavor = "multi_thread")]
async fn earned_head_guard_and_scaffold_over_scaffold() -> Result<()> {
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

    // --- Two independent roots for "guard-c" (authored before gossip), for the
    // scaffold-over-scaffold case. a2 also authors so a1 can retarget to it.
    let root_ca: ContentOutput = c1
        .call(&zome1, "create_content", test_content("guard-c"))
        .await;
    let root_ca_action = root_ca.action_hash.clone();
    let root_cb: ContentOutput = c2
        .call(&zome2, "create_content", test_content("guard-c"))
        .await;
    let root_cb_action = root_cb.action_hash.clone();

    tokio::time::timeout(Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&c1, &c2]).await {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;
    await_consistency(60, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout: {e}"))?;

    // --- (c) scaffold-over-scaffold: a1 declares root_ca STAGING, then root_cb
    // STAGING — both allowed (current is staging/unmarked, guard falls through).
    let s1: ContentHeadOutput = c1
        .call(
            &zome1,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: "guard-c".to_string(),
                head_action_hash: ActionHashB64::from(root_ca_action.clone()).to_string(),
            },
        )
        .await;
    assert_eq!(s1.head_action_hash, root_ca_action);
    let s2: ContentHeadOutput = c1
        .call(
            &zome1,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: "guard-c".to_string(),
                head_action_hash: ActionHashB64::from(root_cb_action.clone()).to_string(),
            },
        )
        .await;
    assert_eq!(
        s2.head_action_hash, root_cb_action,
        "scaffold-over-scaffold: a newer staging declaration is allowed"
    );

    // --- Earned tier is PROGENITOR-GATED: a2 (non-steward) is refused. a2
    // authored root_cb, so it has "guard-c" content locally.
    let a2_earned: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_earned_canonical_head",
            DeclareCanonicalHeadInput {
                id: "guard-c".to_string(),
                head_action_hash: ActionHashB64::from(root_cb_action.clone()).to_string(),
            },
        )
        .await;
    assert!(
        a2_earned.is_err(),
        "non-steward earned declare must be rejected"
    );
    assert!(
        format!("{:?}", a2_earned.unwrap_err()).contains("bootstrap steward"),
        "earned gate error must name the bootstrap-steward restriction"
    );

    // --- (b) earned-head guard: a1 (progenitor) declares an EARNED canonical for
    // "guard-b" (authored locally by a1), then a scaffold declare is REFUSED.
    let root_b: ContentOutput = c1
        .call(&zome1, "create_content", test_content("guard-b"))
        .await;
    let root_b_action = root_b.action_hash.clone();

    let earned: ContentHeadOutput = c1
        .call(
            &zome1,
            "declare_earned_canonical_head",
            DeclareCanonicalHeadInput {
                id: "guard-b".to_string(),
                head_action_hash: ActionHashB64::from(root_b_action.clone()).to_string(),
            },
        )
        .await;
    assert_eq!(
        earned.head_action_hash, root_b_action,
        "earned head is a1's root"
    );

    let refused: std::result::Result<ContentHeadOutput, _> = c1
        .call_fallible(
            &zome1,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: "guard-b".to_string(),
                head_action_hash: ActionHashB64::from(root_b_action.clone()).to_string(),
            },
        )
        .await;
    assert!(
        refused.is_err(),
        "scaffold declare must be refused once an earned canonical exists"
    );
    assert!(
        format!("{:?}", refused.unwrap_err()).contains("earned head is protected"),
        "guard error must contain the HTTP-mappable 'earned head is protected' substring"
    );

    Ok(())
}

/// Defect 1 (missing target-id validation): a canonical declaration must refuse a
/// target whose Content `id` differs from the declared `id`. Without the gate a
/// declaration could name a retrievable Content authored under a DIFFERENT id as
/// this id's canonical head, silently binding an unrelated record. The gate lives
/// in the shared `declare_canonical_head_inner`, so it protects BOTH tiers.
#[tokio::test(flavor = "multi_thread")]
async fn declare_canonical_rejects_target_id_mismatch() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    // agent is the bootstrap steward (progenitor) so the EARNED tier is reachable.
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("lamad-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();
    let zome = cell.zome("content_store");

    // Two distinct ids, each with its own root.
    let alpha: ContentOutput = conductor
        .call(&zome, "create_content", test_content("mismatch-alpha"))
        .await;
    let beta: ContentOutput = conductor
        .call(&zome, "create_content", test_content("mismatch-beta"))
        .await;

    // STAGING declare for "mismatch-alpha" pointing at BETA's action — refused.
    let staging_bad: std::result::Result<ContentHeadOutput, _> = conductor
        .call_fallible(
            &zome,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: "mismatch-alpha".to_string(),
                head_action_hash: ActionHashB64::from(beta.action_hash.clone()).to_string(),
            },
        )
        .await;
    assert!(
        staging_bad.is_err(),
        "staging declare with a cross-id target must be refused"
    );
    assert!(
        format!("{:?}", staging_bad.unwrap_err()).contains("does not match the declared id"),
        "id-mismatch error must name the mismatch"
    );

    // EARNED declare (progenitor) with the same mismatch — refused by the SAME
    // gate (both tiers share declare_canonical_head_inner).
    let earned_bad: std::result::Result<ContentHeadOutput, _> = conductor
        .call_fallible(
            &zome,
            "declare_earned_canonical_head",
            DeclareCanonicalHeadInput {
                id: "mismatch-alpha".to_string(),
                head_action_hash: ActionHashB64::from(beta.action_hash.clone()).to_string(),
            },
        )
        .await;
    assert!(
        earned_bad.is_err(),
        "earned declare with a cross-id target must be refused"
    );
    assert!(
        format!("{:?}", earned_bad.unwrap_err()).contains("does not match the declared id"),
        "earned tier is protected by the same id-mismatch gate"
    );

    // Positive control: alpha→alpha (id matches) is accepted.
    let ok: ContentHeadOutput = conductor
        .call(
            &zome,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: "mismatch-alpha".to_string(),
                head_action_hash: ActionHashB64::from(alpha.action_hash.clone()).to_string(),
            },
        )
        .await;
    assert_eq!(
        ok.head_action_hash, alpha.action_hash,
        "a matching-id declaration succeeds"
    );

    Ok(())
}

/// Defect 2 (tier-blind resolution): an EARNED canonical head must win at RESOLVE
/// over a STAGING declaration with a strictly NEWER timestamp — the partition-heal
/// hazard. The declare-time earned-head guard is local-only, so a partitioned peer
/// that has not seen the earned link can write a newer staging link; once the
/// partition heals every peer must still converge on the EARNED head.
///
/// Deterministic partition: both roots + both canonical declarations are made
/// BEFORE peer-info exchange (each conductor's local view is empty), and the
/// staging declare is sequenced strictly AFTER the earned one (with a sleep) so
/// its DHT timestamp is provably newer. A tier-blind newest-wins resolver would
/// return the staging root here.
///
/// The partition MUST be genuine, which `standard()` conductors do not give: the
/// process-global kitsune2 mem-bootstrap store lets two same-space conductors on
/// one tokio worker thread auto-discover each other before any exchange, so the
/// second peer would see the first's `tier-x` root (duplicate-id collision) and
/// its earned link (staging declare refused) — exactly the two CI failures this
/// test was hitting. `two_agent_conductors_isolated` disables the bootstrap
/// module so the ONLY path between the peers is the explicit `exchange_peer_info`
/// heal below. See the helper docs + `feedback_sweettest_cross_agent_consistency`.
#[tokio::test(flavor = "multi_thread")]
async fn earned_beats_newer_staging_at_resolve() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors_isolated().await?;
    let seed = network_seed(DNA);
    // a1 is the bootstrap steward (progenitor).
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

    // (a) Two INDEPENDENT roots for one id, authored BEFORE peer exchange (empty
    // local views → both creates succeed). root_a is a1's; root_b is a2's.
    let root_a: ContentOutput = c1
        .call(&zome1, "create_content", test_content("tier-x"))
        .await;
    let root_a_action = root_a.action_hash.clone();
    let root_b: ContentOutput = c2
        .call(&zome2, "create_content", test_content("tier-x"))
        .await;
    let root_b_action = root_b.action_hash.clone();
    assert_ne!(a1, a2, "the two agents must be distinct");

    // (b) STILL PARTITIONED: a1 declares EARNED → root_a (local, progenitor). Then
    // strictly LATER a2 declares STAGING → root_b (a2's guard sees no canonical
    // locally → allowed). The staging link is therefore NEWER than the earned one.
    let earned: ContentHeadOutput = c1
        .call(
            &zome1,
            "declare_earned_canonical_head",
            DeclareCanonicalHeadInput {
                id: "tier-x".to_string(),
                head_action_hash: ActionHashB64::from(root_a_action.clone()).to_string(),
            },
        )
        .await;
    assert_eq!(
        earned.head_action_hash, root_a_action,
        "earned head is a1's root"
    );
    sleep(Duration::from_millis(50)).await; // staging timestamp > earned timestamp
    let staging: ContentHeadOutput = c2
        .call(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: "tier-x".to_string(),
                head_action_hash: ActionHashB64::from(root_b_action.clone()).to_string(),
            },
        )
        .await;
    assert_eq!(
        staging.head_action_hash, root_b_action,
        "the newer staging declare is accepted while partitioned"
    );

    // (c) Heal the partition: both roots + both canonical links gossip everywhere.
    tokio::time::timeout(Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&c1, &c2]).await {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;
    await_consistency(60, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout: {e}"))?;

    // (d) TIER-AWARE RESOLUTION: despite the staging link being strictly NEWER,
    // BOTH peers resolve the EARNED head (root_a, authored a1). Poll c2 until the
    // earned link has gossiped in and the tier guard engages.
    let deadline = Instant::now() + Duration::from_secs(30);
    let head_c2: ContentHeadOutput = loop {
        let h: Option<ContentHeadOutput> = c2
            .call(&zome2, "resolve_content_head", "tier-x".to_string())
            .await;
        if let Some(h) = h {
            if h.head_action_hash == root_a_action {
                break h;
            }
        }
        if Instant::now() >= deadline {
            let cur: Option<ContentHeadOutput> = c2
                .call(&zome2, "resolve_content_head", "tier-x".to_string())
                .await;
            panic!(
                "c2 did not converge on the EARNED head within 30s; got {:?}",
                cur.map(|h| h.head_action_hash)
            );
        }
        sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(
        head_c2.author, a1,
        "earned head is authored by the progenitor a1"
    );

    let head_c1: Option<ContentHeadOutput> = c1
        .call(&zome1, "resolve_content_head", "tier-x".to_string())
        .await;
    let head_c1 = head_c1.expect("c1 must resolve the earned head");
    assert_eq!(
        head_c1.head_action_hash, root_a_action,
        "c1 resolves the EARNED head over the newer staging"
    );
    assert_eq!(
        head_c1.head_action_hash, head_c2.head_action_hash,
        "CONVERGENCE: both peers resolve the earned head despite newer staging"
    );

    Ok(())
}
