//! @dna-scope: lamad
//! Sweettest baseline — lamad (content_store coordinator).
//!
//! Scenarios (§2.3):
//! 1. `content_store_is_reachable`            — DNA installs without error.
//! 2. `content_publishes_and_retrieves_by_id` — single agent create/get round-trip.
//! 3. `content_visible_across_agents`         — cross-agent DHT visibility after settle.
//! 4. `declare_head_notarizes_and_supersedes` — notary-authority Leg 1: author-
//!    filtered HEAD election, author gate, and republish (§5.6 REQ-F13).
//! 5. `resolve_content_head_local_is_nonblocking_and_converges` — a cold
//!    conductor returns local absence without a network wait, then returns the
//!    elected head after its DHT view converges.
//! 6. `resolve_content_heads_local_batches_in_order_and_honors_budget` —
//!    Head-Plane Trust-Gradient Program T1 (contract revised per the
//!    operator review 2026-08-08): batched, budget-bounded
//!    `resolve_content_heads_local` / `resolve_canonical_elections` against a
//!    live conductor (order preserved, honest per-id `Resolved(None)`
//!    absence, zero-budget starves every id as `unattempted` with
//!    `stop_reason: BudgetExhausted`, `schema_version` present).
//! 7. `absent_bootstrap_steward_refuses_earned_declaration_cleanly` — an
//!    unset/null progenitor is honest absence: identity is false and the
//!    earned authority arm returns its clean refusal rather than raw Serde.
//!
//! The coordinator zome is `content_store` (per dna/elohim/dna.yaml).
//! DNA artifact: `dna/elohim/workdir/lamad.dna`.

use anyhow::Result;
// `Record` is used by the declare-carries-Record test to decode a served
// record and construct the entry-swap forgery the coordinator must refuse.
// 0.7: `Record::new` takes a `RecordEntry` (0.6 derived it from an Option<Entry>).
use hdk::prelude::{Record, RecordEntry, Timestamp};
use holo_hash::{ActionHash, ActionHashB64, AgentPubKey};
use holochain::sweettest::{await_consistency_s, SweetConductor};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::sleep;

use elohim_sweettest::common::{
    conductors::{
        load_dna, single_agent_conductor, two_agent_conductors, two_agent_conductors_isolated,
        SweetAgents,
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

/// Mirrors `content_store::DeclareCanonicalHeadInput` WITH the sprint-3
/// declare-carries-Record field.
///
/// Kept as a SEPARATE mirror on purpose: every existing test keeps sending the
/// two-field shape above, so those calls double as the backward-compatibility
/// proof — an old-shaped MessagePack map (no `carried_record` key) must still
/// decode on the new coordinator via `#[serde(default)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeclareCanonicalHeadWithRecordInput {
    pub id: String,
    pub head_action_hash: String,
    pub carried_record: Option<Vec<u8>>,
}

/// Mirrors `content_store::DeclareCanonicalHeadInput` WITH the ADOPT-BEFORE-AUTHOR
/// flag — the operator-reserved bypass of the target-independent no-chain gate.
///
/// A THIRD mirror rather than a field on the second, for the same reason the
/// second exists: every call that keeps sending the narrower shape is a live
/// backward-compatibility proof that `#[serde(default)]` decodes an absent key
/// as `false`. Only the tests that mean to exercise the flag send this one.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeclareCanonicalHeadAdoptInput {
    pub id: String,
    pub head_action_hash: String,
    pub carried_record: Option<Vec<u8>>,
    pub adopt_before_author: bool,
}

/// Mirrors `content_store::CanonicalElectionOutput`. `winner_target` is `String`
/// because the zome field is `ActionHashB64`, which serializes as a canonical
/// base64 STRING (a raw-bytes `ActionHash` would fail to deserialize it).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CanonicalElectionOutput {
    pub winner_target: String,
    pub canonical_declared_at: Timestamp,
    pub canonical_earned: bool,
}

/// Mirrors `content_store::GetRecordForActionInput` — the SOURCE half of
/// declare-carries-Record.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GetRecordForActionInput {
    pub action_hash: String,
}

/// Mirrors `content_store::CarriedRecordOutput`. `action_hash` is `String`
/// because the zome field is `ActionHashB64`, which serializes as a canonical
/// base64 STRING (a raw-bytes `ActionHash` would fail to deserialize it).
/// `record` is the opaque `holochain_serialized_bytes` encoding of a `Record`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CarriedRecordOutput {
    pub action_hash: String,
    pub record: Vec<u8>,
}

/// Mirrors the fields of `content_store::ContentHeadOutput` these tests assert.
/// The wire struct carries more (entry_hash); holochain serializes structs as
/// MessagePack maps, so this subset deserializes — extra keys are ignored (same
/// pattern as `ContentOutput`/`WireContent` above). `declared_at` is included so
/// the declare-carries-Record test can assert the carried-branch timestamp is
/// the local declaration time, not the carried action's own timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContentHeadOutput {
    pub content_id: String,
    pub head_action_hash: ActionHash,
    pub author: AgentPubKey,
    pub declared_at: Timestamp,
    pub supersedes: Option<ActionHash>,
    pub content: WireContent,
}

// ---------------------------------------------------------------------------
// Batched head-plane reads (Head-Plane Trust-Gradient Program, T1)
// ---------------------------------------------------------------------------

/// Mirrors `lamad_types::BatchResolveHeadsInput` /
/// `lamad_types::BatchResolveElectionsInput` — identical shape, shared mirror.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchIdsInput {
    pub ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_ms: Option<u32>,
}

// ---------------------------------------------------------------------------
// Batched head-plane reads — contract revision (operator review 2026-08-08)
//
// Mirrors of `content_store::{BatchFailureReason, BatchResolvePhase,
// BatchResolveFailure, BatchOutcome<T>, BatchAttempt<T>, BatchStopReason,
// BatchResolveHeadsOutput, BatchResolveElectionsOutput}`. Live-poison
// injection (a coordinator-compatible bad link, a cross-id link) is not
// feasible from this standalone sweettest harness, so this suite proves
// SHAPE + ORDERING + BUDGET STARVATION + `schema_version` presence against a
// real conductor — the per-item failure-disposition semantics themselves are
// pinned by the zome's own `run_admitted_batch_tests`
// (deterministic-continue / shared-stop / duplicate-ordering /
// clock-failure), which inject synthetic outcomes and need no conductor.
// ---------------------------------------------------------------------------

/// Mirrors `content_store::BatchFailureReason`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum BatchFailureReason {
    WrongContentId,
    RecordShape,
    PermitTimeout,
    Transport,
    ClockUnavailable,
    Unknown,
}

/// Mirrors `content_store::BatchResolvePhase`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BatchResolvePhase {
    HeadResolve,
    Election,
    Clock,
}

/// Mirrors `content_store::BatchResolveFailure`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchResolveFailure {
    #[allow(dead_code)]
    reason: BatchFailureReason,
    #[allow(dead_code)]
    phase: BatchResolvePhase,
}

/// Mirrors `content_store::BatchHeadOutcome` (`BatchOutcome<ContentHeadOutput>`).
#[derive(Debug, Clone, Serialize, Deserialize)]
enum BatchHeadOutcome {
    Resolved(Option<ContentHeadOutput>),
    #[allow(dead_code)]
    Failed(BatchResolveFailure),
}

/// Mirrors `content_store::BatchHeadAttempt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchHeadAttempt {
    id: String,
    outcome: BatchHeadOutcome,
}

/// Mirrors `content_store::BatchElectionOutcome` (`BatchOutcome<CanonicalElectionOutput>`).
#[derive(Debug, Clone, Serialize, Deserialize)]
enum BatchElectionOutcome {
    Resolved(Option<CanonicalElectionOutput>),
    #[allow(dead_code)]
    Failed(BatchResolveFailure),
}

/// Mirrors `content_store::BatchElectionAttempt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchElectionAttempt {
    id: String,
    outcome: BatchElectionOutcome,
}

/// Mirrors `content_store::BatchStopReason`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
enum BatchStopReason {
    BudgetExhausted,
    #[allow(dead_code)]
    IdCeiling,
    #[allow(dead_code)]
    SharedFailure(BatchFailureReason),
}

/// Mirrors `content_store::BatchResolveHeadsOutput` (post-revision shape:
/// `schema_version` + `attempted: Vec<BatchHeadAttempt>` + `unattempted` +
/// `stop_reason` + `elapsed_ms` — T1's original `resolved`/`unattempted`/
/// `elapsed_ms` shape is gone; the zome's own
/// `batch_output_wire_contract_tests` pins that an old-shape decode of a
/// new-shape response fails outright).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchResolveHeadsOutput {
    schema_version: u16,
    attempted: Vec<BatchHeadAttempt>,
    unattempted: Vec<String>,
    stop_reason: Option<BatchStopReason>,
    elapsed_ms: u32,
}

/// Mirrors `content_store::BatchResolveElectionsOutput` (same post-revision
/// shape as [`BatchResolveHeadsOutput`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchResolveElectionsOutput {
    schema_version: u16,
    attempted: Vec<BatchElectionAttempt>,
    unattempted: Vec<String>,
    stop_reason: Option<BatchStopReason>,
    elapsed_ms: u32,
}

// ---------------------------------------------------------------------------
// Fixture helper
// ---------------------------------------------------------------------------

/// Per-invocation unique content id. nextest RETRIES run in the same process,
/// and the kitsune2 mem-bootstrap store is process-global — a retry that
/// re-uses fixed ids re-joins the first attempt's DHT residue and
/// `create_content` refuses with "already exists" (dna #1357, two failed
/// attempts of the same build). An epoch-nanos suffix gives every attempt
/// fresh ids while keeping the scenario byte-identical.
fn unique_id(base: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before UNIX_EPOCH")
        .as_nanos();
    format!("{base}-{nanos}")
}

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

    // Agent 1 creates content. unique_id, not a fixed string: nextest retries
    // share the process-global mem-bootstrap store, so a fixed id makes every
    // retry re-join the first attempt's DHT residue and self-poison (dna #1357
    // class — the #1379/#1380 in-build retries were doomed by this
    // independently of the node-CPU contention the wider deadline absorbs).
    let content_id = unique_id("cross-agent-1");
    let input = test_content(&content_id);
    let output: ContentOutput = c1
        .call(&cell1.zome("content_store"), "create_content", input)
        .await;
    assert_eq!(output.content.id, content_id);

    // Poll c2 until the IdToContent link is gossipped, or panic on deadline.
    // Single-shot reads after a fixed sleep race the link-traversal path; see
    // tests/node_registry.rs admission_visible_across_agents for rationale.
    //
    // 120s, not 30s: CI packs 2-3 conductor-heavy sweettest shards onto one
    // node (hostpath-PVC pinning), and at 95-99% node CPU the gossip round
    // that carries this link can exceed 30s (#1376/#1379/#1380 all failed
    // exactly here at 95%+ saturation; passes ran at 78-85%). The loop breaks
    // on first success, so a healthy run never pays the wider budget — see
    // backlog/ci-sweettest-shard-packing-contention.md for the node evidence.
    let deadline = Instant::now() + Duration::from_secs(120);
    let zome = cell2.zome("content_store");
    let retrieved: ContentOutput = loop {
        let result: Option<ContentOutput> = c2
            .call(
                &zome,
                "get_content_by_id",
                QueryByIdInput {
                    id: content_id.clone(),
                },
            )
            .await;
        if let Some(out) = result {
            break out;
        }
        if Instant::now() >= deadline {
            panic!("agent 2 could not see content '{content_id}' within 120s");
        }
        sleep(Duration::from_millis(100)).await;
    };

    assert_eq!(retrieved.content.id, content_id);
    assert_eq!(
        retrieved.content.title,
        format!("Test Concept {content_id}")
    );
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

/// Station 3 (stewarded-device-sync): a root author DELEGATES head authority to
/// a second device agent, and the delegate — not the author — moves the head.
///
/// Proves, across two conductors on one DHT:
///   a. A creates content; A mints a signed `HeadDelegation` for B via
///      `grant_head_delegation` (the conductor signs with A's key).
///   b. B (non-author) updates the content and declares the EARNED canonical
///      head at its own update, carrying the delegation → accepted.
///   c. Both conductors' `resolve_content_head` converge on B's update (the
///      canonical override wins over the author-filtered per-root election).
///   d. Without a delegation the same declare is refused ("restricted to the");
///      with a delegation whose scope does not cover the id it is refused
///      ("scope").
#[tokio::test(flavor = "multi_thread")]
async fn delegated_device_moves_the_head() -> Result<()> {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct GrantHeadDelegationInput {
        delegate: AgentPubKey,
        scope: String,
        valid_until: Timestamp,
    }
    // Opaque mirror: the zome returns { payload: {...}, signature: ... }; we
    // round-trip it verbatim into the declare input, so the test cannot drift
    // from the zome's serialization (serde_json::Value would mangle msgpack
    // bytes; a typed passthrough keeps them exact).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct HeadDelegationPayloadMirror {
        grantor: AgentPubKey,
        delegate: AgentPubKey,
        scope: String,
        valid_until: Timestamp,
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct HeadDelegationMirror {
        payload: HeadDelegationPayloadMirror,
        signature: hdk::prelude::Signature,
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct DeclareEarnedWithDelegationInput {
        id: String,
        head_action_hash: String,
        delegation: Option<HeadDelegationMirror>,
    }

    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;
    let seed = network_seed(DNA);
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

    // (a) A creates; A grants B a delegation valid for an hour.
    // unique_id: a nextest retry shares the process-global mem-bootstrap DHT,
    // so a fixed id would self-poison the retry (dna #1357).
    let id = unique_id("device-1");
    let _created: ContentOutput = c1.call(&zome1, "create_content", test_content(&id)).await;
    let valid_until = Timestamp::from_micros(Timestamp::now().as_micros() + 3_600_000_000);
    let delegation: HeadDelegationMirror = c1
        .call(
            &zome1,
            "grant_head_delegation",
            GrantHeadDelegationInput {
                delegate: a2.clone(),
                scope: "*".to_string(),
                valid_until,
            },
        )
        .await;
    assert_eq!(delegation.payload.grantor, a1);
    assert_eq!(delegation.payload.delegate, a2);

    // B must see the content before acting on it.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let seen: Option<ContentHeadOutput> =
            c2.call(&zome2, "resolve_content_head", id.clone()).await;
        if seen.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            panic!("agent 2 could not see content 'device-1' within 30s");
        }
        sleep(Duration::from_millis(100)).await;
    }

    // (b) B updates (its own device-authored version) and declares the EARNED
    // canonical head at it, carrying the delegation.
    let b_update: ContentOutput = c2
        .call(
            &zome2,
            "update_content",
            UpdateContentInput {
                id: id.clone(),
                title: Some("Revised on the second device".to_string()),
            },
        )
        .await;
    let b_action = b_update.action_hash.clone();
    let declared: ContentHeadOutput = c2
        .call(
            &zome2,
            "declare_earned_canonical_head",
            DeclareEarnedWithDelegationInput {
                id: id.clone(),
                head_action_hash: ActionHashB64::from(b_action.clone()).to_string(),
                delegation: Some(delegation.clone()),
            },
        )
        .await;
    assert_eq!(declared.head_action_hash, b_action);

    // (c) Both conductors converge on B's update as the head.
    for (c, zome, who) in [(&c1, &zome1, "author"), (&c2, &zome2, "delegate")] {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let h: Option<ContentHeadOutput> =
                c.call(zome, "resolve_content_head", id.clone()).await;
            if let Some(h) = h {
                if h.head_action_hash == b_action {
                    break;
                }
            }
            if Instant::now() >= deadline {
                panic!("{who} conductor did not converge on the delegate's head within 30s");
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    // (d) No delegation → refused; wrong scope → refused.
    let bare: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_earned_canonical_head",
            DeclareEarnedWithDelegationInput {
                id: id.clone(),
                head_action_hash: ActionHashB64::from(b_action.clone()).to_string(),
                delegation: None,
            },
        )
        .await;
    let err = format!(
        "{:?}",
        bare.expect_err("undelegated earned declare must be refused")
    );
    assert!(
        err.contains("restricted to the"),
        "refusal must name the authority rule; got: {err}"
    );

    let wrong_scope: HeadDelegationMirror = c1
        .call(
            &zome1,
            "grant_head_delegation",
            GrantHeadDelegationInput {
                delegate: a2.clone(),
                scope: "some-other-id".to_string(),
                valid_until,
            },
        )
        .await;
    let scoped: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_earned_canonical_head",
            DeclareEarnedWithDelegationInput {
                id: id.clone(),
                head_action_hash: ActionHashB64::from(b_action).to_string(),
                delegation: Some(wrong_scope),
            },
        )
        .await;
    let err = format!(
        "{:?}",
        scoped.expect_err("out-of-scope delegation must be refused")
    );
    assert!(
        err.contains("scope"),
        "refusal must name the scope; got: {err}"
    );

    Ok(())
}

/// The projection-heal read must only inspect the conductor's local view: on a
/// cold storage arc it returns `None` promptly, rather than waiting for the
/// network request timeout. After the normal peer exchange and DHT convergence,
/// that same local read returns the root author's elected head.
///
/// This is deliberately an isolated two-conductor setup. It makes B's first
/// answer a genuine local miss, rather than relying on a race with background
/// gossip to simulate a cold conductor.
#[tokio::test(flavor = "multi_thread")]
async fn resolve_content_head_local_is_nonblocking_and_converges() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors_isolated().await?;
    let seed = network_seed(DNA);
    let dna_file = load_dna(DNA, &seed, Some(a1.clone())).await?;
    let app1 = c1
        .setup_app_for_agent("lamad-app", a1, &[dna_file.clone()])
        .await?;
    let app2 = c2.setup_app_for_agent("lamad-app", a2, &[dna_file]).await?;
    let cell1 = app1.cells().first().expect("cell installed").clone();
    let cell2 = app2.cells().first().expect("cell installed").clone();
    let zome1 = cell1.zome("content_store");
    let zome2 = cell2.zome("content_store");
    let id = unique_id("local-head");

    // Initialize B's Wasm runtime without reading its DHT. The bounded call
    // below now measures a cold LOCAL VIEW, not one-time Wasm initialization.
    let schema_version: String = c2.call(&zome2, "export_schema_version", ()).await;
    assert_eq!(schema_version, "v1");

    // A creates while conductors are deliberately disconnected. B has no local
    // IdToContent link or record for this id yet.
    let created: ContentOutput = c1.call(&zome1, "create_content", test_content(&id)).await;

    // A Local lookup must not turn this cold-view miss into a network fetch.
    // The former Network implementation can wait for the conductor's 60s
    // request timeout; two seconds is ample after Wasm initialization while
    // making that regression fail decisively.
    let cold: Option<ContentHeadOutput> = tokio::time::timeout(
        Duration::from_secs(2),
        c2.call(&zome2, "resolve_content_head_local", id.clone()),
    )
    .await
    .map_err(|_| anyhow::anyhow!("local head lookup waited for the network on a cold peer"))?;
    assert!(
        cold.is_none(),
        "a cold peer's local answer is an observation of local absence, not a network result"
    );

    // Integrate the peers explicitly, then wait until B's local DHT view has
    // received both the link and target record.
    tokio::time::timeout(Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&c1, &c2]).await {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;
    await_consistency_s(60, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after local-head create: {e}"))?;

    let converged: Option<ContentHeadOutput> = c2
        .call(&zome2, "resolve_content_head_local", id.clone())
        .await;
    let converged = converged.expect("integrated peer must resolve the local head");
    assert_eq!(converged.content_id, id);
    assert_eq!(
        converged.head_action_hash, created.action_hash,
        "once ops are integrated, Local and Network choose the same elected head"
    );

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
    // ISOLATED conductors: this test authors the SAME id on both conductors and
    // depends on a genuine pre-exchange partition. Under the default config the
    // process-global mem-bootstrap store (thread-id-keyed) can join the peers
    // before the author phase, so c2's create_content sees c1's root and refuses
    // with "already exists" — the dna #1357/#1358 red. Same cure as
    // earned_beats_newer_staging_at_resolve; see two_agent_conductors_isolated.
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors_isolated().await?;
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

    let landing_x = unique_id("landing-x");
    let solo_y = unique_id("solo-y");

    // --- (a) Two INDEPENDENT roots for one id. Authored BEFORE peer-info
    // exchange, so each conductor's `content_exists_by_id` sees an empty local
    // DHT view and both creates succeed — reproducing the deploy-time double
    // authoring (adam's f41d / matthew's 6af9) deterministically.
    let root_a: ContentOutput = c1
        .call(&zome1, "create_content", test_content(&landing_x))
        .await;
    let root_a_action = root_a.action_hash.clone();

    let root_b: ContentOutput = c2
        .call(&zome2, "create_content", test_content(&landing_x))
        .await;
    let root_b_action = root_b.action_hash.clone();
    assert_ne!(
        root_a_action, root_b_action,
        "the two independent roots must be distinct actions"
    );

    // Control id: single-author create + update on c1 (undeclared canonical).
    let _solo: ContentOutput = c1
        .call(&zome1, "create_content", test_content(&solo_y))
        .await;
    let solo_updated: ContentOutput = c1
        .call(
            &zome1,
            "update_content",
            UpdateContentInput {
                id: solo_y.clone(),
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
    await_consistency_s(60, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after independent roots: {e}"))?;

    // Both agents can now resolve SOME head for the id (both roots gossiped).
    let pre_a: Option<ContentHeadOutput> = c1
        .call(&zome1, "resolve_content_head", landing_x.clone())
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
                id: landing_x.clone(),
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
    await_consistency_s(60, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after canonical declare: {e}"))?;

    // --- (c) BOTH conductors resolve the SAME canonical head (== B's root).
    // Poll c2 until the canonical link has gossiped in and overrides its
    // own-root election.
    let deadline = Instant::now() + Duration::from_secs(30);
    let head_c2: ContentHeadOutput = loop {
        let h: Option<ContentHeadOutput> = c2
            .call(&zome2, "resolve_content_head", landing_x.clone())
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
        .call(&zome1, "resolve_content_head", landing_x.clone())
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
        .call(&zome1, "resolve_content_head", solo_y.clone())
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
    // ISOLATED conductors: authors "guard-c" on BOTH conductors pre-exchange —
    // same partition assumption (and same nondeterministic "already exists"
    // failure mode) as declare_canonical_head_converges_independent_roots; it
    // passed #1358 only by winning the thread-placement race.
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors_isolated().await?;
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

    let guard_c = unique_id("guard-c");
    let guard_b = unique_id("guard-b");

    // --- Two independent roots for "guard-c" (authored before gossip), for the
    // scaffold-over-scaffold case. a2 also authors so a1 can retarget to it.
    let root_ca: ContentOutput = c1
        .call(&zome1, "create_content", test_content(&guard_c))
        .await;
    let root_ca_action = root_ca.action_hash.clone();
    let root_cb: ContentOutput = c2
        .call(&zome2, "create_content", test_content(&guard_c))
        .await;
    let root_cb_action = root_cb.action_hash.clone();

    tokio::time::timeout(Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&c1, &c2]).await {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;
    await_consistency_s(60, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout: {e}"))?;

    // --- (c) scaffold-over-scaffold: a1 declares root_ca STAGING, then root_cb
    // STAGING — both allowed (current is staging/unmarked, guard falls through).
    let s1: ContentHeadOutput = c1
        .call(
            &zome1,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: guard_c.clone(),
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
                id: guard_c.clone(),
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
                id: guard_c.clone(),
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
        .call(&zome1, "create_content", test_content(&guard_b))
        .await;
    let root_b_action = root_b.action_hash.clone();

    let earned: ContentHeadOutput = c1
        .call(
            &zome1,
            "declare_earned_canonical_head",
            DeclareCanonicalHeadInput {
                id: guard_b.clone(),
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
                id: guard_b.clone(),
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

/// An unset/null `progenitor_pubkey` is a legitimate network shape, not a
/// malformed configured identity. This is the isolated equivalent of the
/// release-ceremony driver's `promote --as jessica` negative control: the
/// non-author is refused cleanly, while the bootstrap-steward predicate
/// degrades to false instead of leaking a Deserialize failure.
#[tokio::test(flavor = "multi_thread")]
async fn absent_bootstrap_steward_refuses_earned_declaration_cleanly() -> Result<()> {
    let (mut conductor, a1) = single_agent_conductor().await?;
    let a2 = SweetAgents::one(conductor.keystore()).await;
    let dna_file = load_dna(DNA, &network_seed(DNA), None).await?;
    let app1 = conductor
        .setup_app_for_agent("content-no-bootstrap", a1.clone(), &[dna_file.clone()])
        .await?;
    let app2 = conductor
        .setup_app_for_agent("content-no-bootstrap-non-author", a2.clone(), &[dna_file])
        .await?;
    let cell1 = app1.cells().first().expect("first cell installed").clone();
    let cell2 = app2.cells().first().expect("second cell installed").clone();
    let zome1 = cell1.zome("content_store");
    let zome2 = cell2.zome("content_store");

    let steward: Option<AgentPubKey> = conductor.call(&zome2, "get_bootstrap_steward", ()).await;
    let is_steward: bool = conductor.call(&zome2, "is_bootstrap_steward", ()).await;
    assert!(steward.is_none(), "unset progenitor must be honest absence");
    assert!(!is_steward, "no agent self-elects when progenitor is unset");

    let id = unique_id("absent-bootstrap-steward");
    let root: ContentOutput = conductor
        .call(&zome1, "create_content", test_content(&id))
        .await;

    let refused: std::result::Result<ContentHeadOutput, _> = conductor
        .call_fallible(
            &zome2,
            "declare_earned_canonical_head",
            DeclareCanonicalHeadInput {
                id,
                head_action_hash: ActionHashB64::from(root.action_hash).to_string(),
            },
        )
        .await;
    let error = format!(
        "{:?}",
        refused.expect_err("non-author without bootstrap stewardship must be refused")
    );
    assert!(
        error.contains("restricted to the root author, a device it delegated, or the bootstrap"),
        "negative control must reach the clean authority refusal: {error}"
    );
    assert!(
        !error.contains("Deserialize"),
        "null progenitor must not leak a raw Deserialize error: {error}"
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
    await_consistency_s(60, [&cell1, &cell2])
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

/// DECLARE-CARRIES-RECORD (sprint-3 centerpiece): a conductor can declare a
/// canonical head it CANNOT retrieve, by carrying the target's signed `Record`
/// and verifying it in wasm.
///
/// Why this exists: on a full-arc fleet (`target_arc_factor = 1`) every peer is
/// an authority for every hash, so the `get` cascade short-circuits WITHOUT a
/// network fetch. A gossip gap therefore reads as permanent ABSENCE, and the
/// coordinator's "target action is not retrievable" refusal is unclearable by
/// any retry ladder. Carrying the record replaces "wait for gossip" with
/// "prove it locally".
///
/// Two conductors, deliberately NEVER peer-exchanged, so c2 genuinely does not
/// hold c1's record. Proves:
///   a. `get_record_for_action` serves on the holder (c1) and honestly answers
///      `None` on the non-holder (c2).
///   b. WITHOUT a carried record, c2's declare is refused "not retrievable" —
///      the unchanged pre-sprint-3 behaviour (backward compatibility, sent on
///      the OLD two-field wire shape).
///   c. NEGATIVE — a carried record for a DIFFERENT action is refused
///      (action-hash equality gate).
///   d. NEGATIVE — a genuinely-signed action paired with a SWAPPED entry is
///      refused (entry↔action binding gate). This is the dangerous forgery:
///      checks 1 and 2 pass, only the entry-hash binding catches it.
///   e. NEGATIVE — byte-tampered record bytes are refused.
///   f. WITH the genuine carried record, the declare SUCCEEDS and the canonical
///      head points at c1's ORIGINAL action hash, authored by a1 — the
///      anchor-equality property (no local re-author, no new action hash).
#[tokio::test(flavor = "multi_thread")]
async fn declare_carries_record_for_unretrievable_target() -> Result<()> {
    // ISOLATED and NEVER exchanged: the whole point is that c2 cannot reach
    // c1's record. `two_agent_conductors_isolated` keeps the process-global
    // mem-bootstrap store from joining them behind our back.
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors_isolated().await?;
    let seed = network_seed(DNA);
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

    let carried_x = unique_id("carried-x");
    let decoy_z = unique_id("carried-decoy-z");

    // c1 authors the TARGET root. c2 authors its own independent root for the
    // same id, so c2 passes the id-exists gate locally (that gate is not what
    // this test is about) while still not holding c1's action.
    let root_a: ContentOutput = c1
        .call(&zome1, "create_content", test_content(&carried_x))
        .await;
    let root_a_action = root_a.action_hash.clone();
    let root_b: ContentOutput = c2
        .call(&zome2, "create_content", test_content(&carried_x))
        .await;
    assert_ne!(
        root_a_action, root_b.action_hash,
        "the two independent roots must be distinct actions"
    );
    // A second, unrelated record on c1 — the decoy for the negative cases.
    let decoy: ContentOutput = c1
        .call(&zome1, "create_content", test_content(&decoy_z))
        .await;

    let target_b64 = ActionHashB64::from(root_a_action.clone()).to_string();

    // --- (a) the holder serves; the non-holder honestly answers None.
    let served: Option<CarriedRecordOutput> = c1
        .call(
            &zome1,
            "get_record_for_action",
            GetRecordForActionInput {
                action_hash: target_b64.clone(),
            },
        )
        .await;
    let served = served.expect("the authoring peer must serve its own record");
    assert_eq!(
        served.action_hash, target_b64,
        "served record is for the requested action"
    );
    assert!(!served.record.is_empty(), "served record must carry bytes");

    let c2_view: Option<CarriedRecordOutput> = c2
        .call(
            &zome2,
            "get_record_for_action",
            GetRecordForActionInput {
                action_hash: target_b64.clone(),
            },
        )
        .await;
    assert!(
        c2_view.is_none(),
        "c2 must NOT hold c1's action (no peer exchange) — otherwise this test \
         proves nothing"
    );

    // --- (b) no carried record → the unchanged "not retrievable" refusal.
    // Sent on the OLD two-field mirror: this doubles as the backward-compat
    // proof that a payload with no `carried_record` key still decodes.
    let bare: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: carried_x.clone(),
                head_action_hash: target_b64.clone(),
            },
        )
        .await;
    assert!(
        bare.is_err(),
        "without a carried record the declare must still be refused"
    );
    let bare_err = format!("{:?}", bare.unwrap_err());
    assert!(
        bare_err.contains("not retrievable"),
        "the classic refusal must be preserved verbatim; got: {bare_err}"
    );

    // --- (c) NEGATIVE: carry the DECOY record while declaring the target hash.
    let decoy_served: Option<CarriedRecordOutput> = c1
        .call(
            &zome1,
            "get_record_for_action",
            GetRecordForActionInput {
                action_hash: ActionHashB64::from(decoy.action_hash.clone()).to_string(),
            },
        )
        .await;
    let decoy_served = decoy_served.expect("c1 must serve the decoy record");
    let mismatched: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadWithRecordInput {
                id: carried_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: Some(decoy_served.record.clone()),
            },
        )
        .await;
    assert!(
        mismatched.is_err(),
        "a carried record for a DIFFERENT action must be refused"
    );
    let mismatched_err = format!("{:?}", mismatched.unwrap_err());
    assert!(
        mismatched_err.contains("does not match"),
        "the action-hash gate must name the mismatch; got: {mismatched_err}"
    );

    // --- (d) NEGATIVE: genuine signed action + SWAPPED entry. Checks 1 (hash)
    // and 2 (signature) pass — only the entry↔action binding catches this, so
    // it is the sharpest guard in the set.
    let genuine: Record = holochain_serialized_bytes::decode(&served.record)
        .map_err(|e| anyhow::anyhow!("decode served record: {e}"))?;
    let decoy_record: Record = holochain_serialized_bytes::decode(&decoy_served.record)
        .map_err(|e| anyhow::anyhow!("decode decoy record: {e}"))?;
    // 0.7: `Record::new` takes `RecordEntry` directly (0.6 derived it internally).
    // Reproducing 0.6's derivation verbatim keeps the swap byte-identical.
    let swapped = Record::new(
        genuine.signed_action().clone(),
        RecordEntry::new(
            genuine.action().entry_visibility(),
            decoy_record.entry().as_option().cloned(),
        ),
    );
    let swapped_bytes: Vec<u8> = holochain_serialized_bytes::encode(&swapped)
        .map_err(|e| anyhow::anyhow!("encode swapped record: {e}"))?;
    let swapped_declare: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadWithRecordInput {
                id: carried_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: Some(swapped_bytes),
            },
        )
        .await;
    assert!(
        swapped_declare.is_err(),
        "a genuine action paired with a SWAPPED entry must be refused"
    );
    let swapped_err = format!("{:?}", swapped_declare.unwrap_err());
    assert!(
        swapped_err.contains("does not match the action's entry hash"),
        "the entry↔action binding gate (check 3) must name the entry-hash \
         mismatch — that binding is the ONLY check that catches a genuine \
         signed action with substituted Content bytes; got: {swapped_err}"
    );

    // --- (e) NEGATIVE: byte-tampered record. Either the decode fails or a hash
    // check does; both are refusals, and neither may write a canonical link.
    let mut tampered = served.record.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0xFF;
    let tampered_declare: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadWithRecordInput {
                id: carried_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: Some(tampered),
            },
        )
        .await;
    assert!(
        tampered_declare.is_err(),
        "byte-tampered carried record must be refused"
    );

    // --- (f) POSITIVE: the genuine carried record lands the declaration.
    let declared: ContentHeadOutput = c2
        .call(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadWithRecordInput {
                id: carried_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: Some(served.record.clone()),
            },
        )
        .await;
    assert_eq!(
        declared.head_action_hash, root_a_action,
        "ANCHOR EQUALITY: the canonical head is c1's ORIGINAL action hash — the \
         carried record is verified in place, never re-authored locally"
    );
    assert_eq!(
        declared.author, a1,
        "the declared head keeps its foreign author (a1), not the declarer (a2)"
    );
    assert_eq!(declared.content_id, carried_x);
    assert_ne!(
        a1, a2,
        "the two agents must be distinct for (f) to mean anything"
    );

    // HEAL-GUARD LOCKOUT GUARD: declared_at on the CARRIED branch is stamped
    // with the DECLARING conductor's LOCAL sys_time — never copied from the
    // carried action's own timestamp. If the carried timestamp propagated
    // verbatim, a carried action stamped at i64::MAX would flow into
    // declared_head_at and permanently lock out the storage monotonic heal
    // guard (which keys off declared_at), starving the deploy-path heal. The
    // carried record proves the target BYTES, not WHEN this head was declared.
    // The genuine carried action (root_a) was authored strictly BEFORE this
    // declaration, so its timestamp must NOT equal declared_at — and WITHOUT
    // the sys_time override it would equal it EXACTLY (a verbatim copy), so
    // this assert_ne! is a tight regression catch.
    let carried_ts = genuine.action().timestamp();
    assert_ne!(
        declared.declared_at, carried_ts,
        "declared_at must be the local declaration time, NOT the carried \
         action's own timestamp (heal-guard lockout guard)"
    );
    assert!(
        declared.declared_at >= carried_ts,
        "declared_at ({:?}, local sys_time at declare) must be >= the carried \
         action's timestamp ({:?})",
        declared.declared_at,
        carried_ts
    );

    // --- (g) The link is REAL, not just a 200. `resolve_content_head` cannot
    // show it yet: `gather_canonical_head_record` resolves the winning
    // declaration's target with a local `get` and returns None when the target
    // is unretrievable, so c2 still degrades to its own root election. That is
    // the pre-existing, correct degrade — and on the live fleet it does not
    // matter, because the HTTP declare arm stamps the projection from the
    // declare OUTPUT (built here from the carried record), not from a re-resolve.
    let pre_gossip: Option<ContentHeadOutput> = c2
        .call(&zome2, "resolve_content_head", carried_x.clone())
        .await;
    assert_eq!(
        pre_gossip
            .expect("c2 still resolves SOME head")
            .head_action_hash,
        root_b.action_hash,
        "before gossip, resolve degrades to c2's own root — the documented \
         behaviour of gather_canonical_head_record on an unretrievable target"
    );

    // Now let the peers meet. The canonical link c2 wrote from carried evidence
    // must be a genuine DHT link pointing at c1's action — so once the target
    // gossips in, BOTH peers resolve it. This is the assertion that separates
    // "the declare returned 200" from "the canonical head actually landed".
    tokio::time::timeout(Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&c1, &c2]).await {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;
    await_consistency_s(60, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after carried declare: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(30);
    let resolved: ContentHeadOutput = loop {
        let h: Option<ContentHeadOutput> = c2
            .call(&zome2, "resolve_content_head", carried_x.clone())
            .await;
        if let Some(h) = h {
            if h.head_action_hash == root_a_action {
                break h;
            }
        }
        if Instant::now() >= deadline {
            panic!("the carried canonical head did not resolve on c2 within 30s");
        }
        sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(
        resolved.author, a1,
        "the resolved carried head keeps a1's authorship"
    );

    let head_c1: Option<ContentHeadOutput> = c1
        .call(&zome1, "resolve_content_head", carried_x.clone())
        .await;
    assert_eq!(
        head_c1.expect("c1 must resolve the head").head_action_hash,
        root_a_action,
        "CONVERGENCE: the link c2 wrote from carried evidence is honored by the \
         authoring peer too"
    );

    Ok(())
}

/// ADOPT-BEFORE-AUTHOR: a conductor that holds **no chain for the id at all**
/// can declare a canonical head from a validated carried record — and does so
/// ONLY when the operator has opted in.
///
/// ## The residual this closes
///
/// `declare_canonical_head_inner`'s first gate (`gather_content_chain(id)`) is
/// TARGET-INDEPENDENT: it asks whether the DECLARER knows the id, so no choice
/// of target can pass it. When BOTH sides of a divergent pair are in that state
/// neither can mint a canonical-head candidate, the DHT arbiter has an empty
/// candidate set, and the pair cannot converge by ANY automated path. That is
/// the "both-sides-missing" residual. This test is its exit.
///
/// ## Why it is not a weakening
///
/// The bypass is licensed by RE-DERIVED EVIDENCE and nothing else. Every clause
/// is exercised below with the flag ON, so each one is proven to still refuse:
///
///   a. no record, old wire shape → the classic no-chain refusal (compat proof).
///   b. FLAG OFF + a genuine record → STILL refused. Evidence alone never opens
///      the gate; this is the behaviour-identity assertion at the zome boundary.
///   c. FLAG ON + no record → refused. Opt-in alone never opens it either.
///   d. FLAG ON + a record for a DIFFERENT action → refused (action-hash gate).
///   e. FLAG ON + a genuine signed action with a SWAPPED entry → refused
///      (entry↔action binding — the only gate that catches this forgery).
///   f. FLAG ON + byte-tampered record → refused.
///   g. FLAG ON + a fully genuine record for a DIFFERENT CONTENT ID → refused
///      (TARGET-ID GATE). Checks 1-3 all PASS here, so this is the sharpest
///      proof that the bypass did not drop the id binding.
///   h. FLAG ON + the genuine record → the declaration LANDS, anchored on c1's
///      original action hash and c1's authorship (no local re-author).
///   i. `declared_at` is the local declaration time (heal-guard lockout guard).
///   j. IDEMPOTENCE: a replay elects the identical winner — a second run adds no
///      candidate the arbiter can tell apart, so nothing new is minted in effect.
///   k. CONVERGENCE: once the peers meet, both resolve the same head.
#[tokio::test(flavor = "multi_thread")]
async fn adopt_before_author_declares_without_a_local_chain() -> Result<()> {
    // ISOLATED and never exchanged until the very end: c2 must genuinely hold
    // neither c1's record NOR any chain of its own for the id.
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors_isolated().await?;
    let seed = network_seed(DNA);
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

    let adopt_x = unique_id("adopt-before-author-x");
    let other_id = unique_id("adopt-before-author-other");

    // c1 authors the TARGET root, and a second root under a DIFFERENT id (the
    // target-id-gate probe in (g), and the decoy for (d)/(e)).
    let root_a: ContentOutput = c1
        .call(&zome1, "create_content", test_content(&adopt_x))
        .await;
    let root_a_action = root_a.action_hash.clone();
    let other: ContentOutput = c1
        .call(&zome1, "create_content", test_content(&other_id))
        .await;

    // THE PRECONDITION THIS WHOLE TEST RESTS ON: c2 authors NOTHING for
    // `adopt_x`. Asserted, not assumed — if c2 somehow held a chain, every
    // refusal below would be proving the wrong gate.
    let c2_pre: Option<ContentHeadOutput> = c2
        .call(&zome2, "resolve_content_head", adopt_x.clone())
        .await;
    assert!(
        c2_pre.is_none(),
        "c2 must hold NO chain for the id — otherwise this test exercises the \
         not-retrievable gate, not the no-chain gate it exists for"
    );

    let target_b64 = ActionHashB64::from(root_a_action.clone()).to_string();
    let other_b64 = ActionHashB64::from(other.action_hash.clone()).to_string();

    let served: Option<CarriedRecordOutput> = c1
        .call(
            &zome1,
            "get_record_for_action",
            GetRecordForActionInput {
                action_hash: target_b64.clone(),
            },
        )
        .await;
    let served = served.expect("the authoring peer must serve its own record");
    let other_served: Option<CarriedRecordOutput> = c1
        .call(
            &zome1,
            "get_record_for_action",
            GetRecordForActionInput {
                action_hash: other_b64.clone(),
            },
        )
        .await;
    let other_served = other_served.expect("c1 must serve the other-id record");

    // --- (a) no record at all, on the OLD two-field wire shape. Doubles as the
    // backward-compat proof that a payload with neither `carried_record` nor
    // `adopt_before_author` still decodes on the new coordinator.
    let bare: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: adopt_x.clone(),
                head_action_hash: target_b64.clone(),
            },
        )
        .await;
    let bare_err = format!("{:?}", bare.expect_err("a chainless declare must refuse"));
    assert!(
        bare_err.contains("no content found for id"),
        "the no-chain refusal must be preserved VERBATIM — the storage caller \
         classifies on this substring; got: {bare_err}"
    );

    // --- (b) FLAG OFF + a fully genuine carried record → STILL refused, with the
    // SAME message. This is the behaviour-identity proof at the zome boundary: a
    // default-off build cannot be told apart from one that never had the flag.
    let flag_off: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadAdoptInput {
                id: adopt_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: Some(served.record.clone()),
                adopt_before_author: false,
            },
        )
        .await;
    let flag_off_err = format!(
        "{:?}",
        flag_off.expect_err("evidence alone must not open the gate")
    );
    assert!(
        flag_off_err.contains("no content found for id"),
        "with the flag OFF a valid carried record must change NOTHING; got: {flag_off_err}"
    );

    // --- (c) FLAG ON + no record → refused. Opt-in is not a bypass; an
    // unevidenced declaration from a node that does not know the id would be
    // exactly the self-crowning this seam refuses.
    let no_evidence: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadAdoptInput {
                id: adopt_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: None,
                adopt_before_author: true,
            },
        )
        .await;
    let no_evidence_err = format!(
        "{:?}",
        no_evidence.expect_err("opt-in without evidence must refuse")
    );
    assert!(
        no_evidence_err.contains("no content found for id"),
        "adopt_before_author with NO carried record must refuse exactly as \
         before; got: {no_evidence_err}"
    );

    // --- (d) FLAG ON + a record for a DIFFERENT action (action-hash gate).
    let mismatched: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadAdoptInput {
                id: adopt_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: Some(other_served.record.clone()),
                adopt_before_author: true,
            },
        )
        .await;
    let mismatched_err = format!(
        "{:?}",
        mismatched.expect_err("a record for another action must refuse")
    );
    assert!(
        mismatched_err.contains("does not match"),
        "the action-hash gate must still fire under the flag; got: {mismatched_err}"
    );

    // --- (e) FLAG ON + genuine signed action with a SWAPPED entry. Checks 1
    // (hash) and 2 (signature) pass; only the entry↔action binding catches it,
    // so this is the sharpest guard in the set.
    let genuine: Record = holochain_serialized_bytes::decode(&served.record)
        .map_err(|e| anyhow::anyhow!("decode served record: {e}"))?;
    let other_record: Record = holochain_serialized_bytes::decode(&other_served.record)
        .map_err(|e| anyhow::anyhow!("decode other record: {e}"))?;
    // 0.7: `Record::new` takes `RecordEntry` directly (0.6 derived it internally).
    // Reproducing 0.6's derivation verbatim keeps the swap byte-identical.
    let swapped = Record::new(
        genuine.signed_action().clone(),
        RecordEntry::new(
            genuine.action().entry_visibility(),
            other_record.entry().as_option().cloned(),
        ),
    );
    let swapped_bytes: Vec<u8> = holochain_serialized_bytes::encode(&swapped)
        .map_err(|e| anyhow::anyhow!("encode swapped record: {e}"))?;
    let swapped_declare: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadAdoptInput {
                id: adopt_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: Some(swapped_bytes),
                adopt_before_author: true,
            },
        )
        .await;
    let swapped_err = format!(
        "{:?}",
        swapped_declare.expect_err("a swapped entry must refuse")
    );
    assert!(
        swapped_err.contains("does not match the action's entry hash"),
        "the entry↔action binding must still fire under the flag — it is the \
         ONLY check that catches a genuine signed action with substituted \
         Content bytes; got: {swapped_err}"
    );

    // --- (f) FLAG ON + byte-tampered record. Either the decode fails or a hash
    // check does; both are refusals, and neither may write a canonical link.
    let mut tampered = served.record.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0xFF;
    let tampered_declare: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadAdoptInput {
                id: adopt_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: Some(tampered),
                adopt_before_author: true,
            },
        )
        .await;
    assert!(
        tampered_declare.is_err(),
        "a byte-tampered carried record must be refused under the flag too"
    );

    // --- (g) FLAG ON + a FULLY GENUINE record for a DIFFERENT content id,
    // declared (honestly) under its own action hash. Checks 1-3 ALL pass — only
    // the TARGET-ID GATE stands between this and binding an unrelated record as
    // `adopt_x`'s head. The most important negative in the set, because the
    // bypass removed the only other id-scoped gate on this path.
    let wrong_id: std::result::Result<ContentHeadOutput, _> = c2
        .call_fallible(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadAdoptInput {
                id: adopt_x.clone(),
                head_action_hash: other_b64.clone(),
                carried_record: Some(other_served.record.clone()),
                adopt_before_author: true,
            },
        )
        .await;
    let wrong_id_err = format!(
        "{:?}",
        wrong_id.expect_err("a genuine record for another id must refuse")
    );
    assert!(
        wrong_id_err.contains("does not match the declared id"),
        "the TARGET-ID GATE is the last id-scoped guard once the chain gate is \
         bypassed — it must name the mismatch; got: {wrong_id_err}"
    );

    // --- (h) POSITIVE. The genuine record lands the declaration from a node
    // with NO chain for the id: the residual's exit, demonstrated.
    let declared: ContentHeadOutput = c2
        .call(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadAdoptInput {
                id: adopt_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: Some(served.record.clone()),
                adopt_before_author: true,
            },
        )
        .await;
    assert_eq!(
        declared.head_action_hash, root_a_action,
        "ANCHOR EQUALITY: the canonical head is c1's ORIGINAL action hash — the \
         carried record is verified in place, never re-authored locally (which \
         is what makes this ADOPT-before-author rather than author-then-adopt)"
    );
    assert_eq!(
        declared.author, a1,
        "the declared head keeps its foreign author (a1), not the declarer (a2)"
    );
    assert_eq!(declared.content_id, adopt_x);
    assert_ne!(
        a1, a2,
        "the two agents must be distinct for (h) to mean anything"
    );

    // --- (i) HEAL-GUARD LOCKOUT GUARD, same law as the sibling carried-record
    // test: `declared_at` on the evidence branch is the DECLARING conductor's
    // local sys_time, never the carried action's own (attacker-influenced)
    // timestamp, which would permanently lock out the storage monotonic heal.
    let carried_ts = genuine.action().timestamp();
    assert_ne!(
        declared.declared_at, carried_ts,
        "declared_at must be the local declaration time, NOT the carried \
         action's own timestamp"
    );
    assert!(
        declared.declared_at >= carried_ts,
        "declared_at ({:?}) must be >= the carried action's timestamp ({:?})",
        declared.declared_at,
        carried_ts
    );

    // --- (j) IDEMPOTENCE. A chainless node's declaration is now visible to its
    // OWN election read — that is the candidate the arbiter was starved of. A
    // REPLAY of the identical declare elects the identical winner: the second
    // link names the same target, so it adds no candidate the arbiter can tell
    // apart and the outcome is unmoved. (The storage side does not even issue
    // the replay — its `(id, target)` candidacy ledger and declare-storm gate
    // stop it before the conductor is called.)
    let election_once: Option<CanonicalElectionOutput> = c2
        .call(&zome2, "resolve_canonical_election", adopt_x.clone())
        .await;
    let election_once =
        election_once.expect("the chainless declarer must SEE the election it just minted");
    assert_eq!(
        election_once.winner_target, target_b64,
        "the minted candidate must elect c1's action"
    );
    assert!(
        !election_once.canonical_earned,
        "a staging-tier declare must never report the earned tier"
    );

    let _replay: ContentHeadOutput = c2
        .call(
            &zome2,
            "declare_canonical_content_head",
            DeclareCanonicalHeadAdoptInput {
                id: adopt_x.clone(),
                head_action_hash: target_b64.clone(),
                carried_record: Some(served.record.clone()),
                adopt_before_author: true,
            },
        )
        .await;
    let election_twice: Option<CanonicalElectionOutput> = c2
        .call(&zome2, "resolve_canonical_election", adopt_x.clone())
        .await;
    assert_eq!(
        election_twice
            .expect("the election must still resolve after a replay")
            .winner_target,
        election_once.winner_target,
        "IDEMPOTENCE: replaying the declaration must elect the identical winner \
         — a replay may not move the head"
    );

    // --- (k) CONVERGENCE. The link is REAL, not just a 200: once the peers
    // meet, the node that never held the id resolves c1's action as the head,
    // and so does c1.
    tokio::time::timeout(Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&c1, &c2]).await {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;
    await_consistency_s(60, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after adopt declare: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(30);
    let resolved: ContentHeadOutput = loop {
        let h: Option<ContentHeadOutput> = c2
            .call(&zome2, "resolve_content_head", adopt_x.clone())
            .await;
        if let Some(h) = h {
            if h.head_action_hash == root_a_action {
                break h;
            }
        }
        if Instant::now() >= deadline {
            panic!("the adopted canonical head did not resolve on c2 within 30s");
        }
        sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(
        resolved.author, a1,
        "the resolved adopted head keeps a1's authorship"
    );

    let head_c1: Option<ContentHeadOutput> = c1
        .call(&zome1, "resolve_content_head", adopt_x.clone())
        .await;
    assert_eq!(
        head_c1.expect("c1 must resolve the head").head_action_hash,
        root_a_action,
        "CONVERGENCE: the declaration a CHAINLESS node minted from carried \
         evidence is honored by the authoring peer too — the both-sides-missing \
         residual has an automated exit"
    );

    Ok(())
}

/// Head-Plane Trust-Gradient Program (L1/T1, contract revised per the
/// operator review 2026-08-08) — batched, budget-bounded reads against a
/// LIVE conductor. Complements the pure `batch_deadline_admission` /
/// `run_admitted_batch` unit tests in the zome (ceiling clamp, default fill,
/// deadline evaluation, deterministic-continue, shared-stop,
/// duplicate-ordering, clock-failure — none of which need a conductor) with
/// the one thing they cannot exercise: real `sys_time()` host calls and a
/// real `select_election`/`resolve_content_head_inner` delegation through
/// the NEW typed-outcome output shape.
///
/// Live-poison injection (a coordinator-compatible bad link, a cross-id
/// link) is not feasible from this standalone sweettest harness — those
/// defect-2/defect-3 failure paths are pinned at the zome's own unit level
/// (`run_admitted_batch_tests::deterministic_failure_preserves_earlier_and_later_successes`,
/// `run_admitted_batch_tests::wrong_id_refusal_is_typed_failed_not_mislabeled_resolved`).
/// This test proves SHAPE + ORDERING + BUDGET STARVATION +
/// `schema_version` presence instead:
///   a. `resolve_content_heads_local` answers a batch of `[known, MISSING,
///      known]` in ONE call, preserving input order, and the missing id
///      comes back `BatchHeadOutcome::Resolved(None)` — the SAME honest
///      local-absence contract as the single-id extern, never a `Failed`
///      and never dropped from `attempted`. `schema_version` is present and
///      matches the current contract. `stop_reason` is `None` (everything
///      was attempted).
///   b. `budget_ms: Some(0)` admits NOTHING: every id comes back
///      `unattempted` (never started) rather than an `attempted` entry —
///      the in-wasm deadline is wired end-to-end through a real conductor,
///      not only provable in the pure predicate's unit tests — and
///      `stop_reason` names it `BudgetExhausted`, never confused with a
///      per-item or shared failure.
///   c. `resolve_canonical_elections` mirrors the same batch shape for the
///      bare DHT election: after A declares its own root canonical, the
///      batch answer reports A's election and honest absence for B (never
///      declared) — in order, in one call, `schema_version` present.
#[tokio::test(flavor = "multi_thread")]
async fn resolve_content_heads_local_batches_in_order_and_honors_budget() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("lamad-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();
    let zome = cell.zome("content_store");

    let id_a = unique_id("batch-a");
    let id_b = unique_id("batch-b");
    let missing_id = unique_id("batch-missing");

    let created_a: ContentOutput = conductor
        .call(&zome, "create_content", test_content(&id_a))
        .await;
    let created_b: ContentOutput = conductor
        .call(&zome, "create_content", test_content(&id_b))
        .await;

    // --- (a) order preserved; a genuinely-unknown id resolves to
    // `Resolved(None)`, never `Failed` and never a hole in `attempted`.
    let batch: BatchResolveHeadsOutput = conductor
        .call(
            &zome,
            "resolve_content_heads_local",
            BatchIdsInput {
                ids: vec![id_a.clone(), missing_id.clone(), id_b.clone()],
                budget_ms: None,
            },
        )
        .await;
    assert_eq!(
        batch.schema_version, 1,
        "schema_version is present and names the current contract"
    );
    assert_eq!(
        batch.attempted.len(),
        3,
        "all three ids are attempted under the default budget"
    );
    assert!(
        batch.unattempted.is_empty(),
        "nothing should be starved under the default budget"
    );
    assert!(
        batch.stop_reason.is_none(),
        "every id was attempted — no stop reason"
    );
    assert_eq!(
        batch.attempted[0].id, id_a,
        "input order is preserved (index 0)"
    );
    let BatchHeadOutcome::Resolved(Some(head_a)) = &batch.attempted[0].outcome else {
        panic!("id_a must resolve to a head, not a failure or absence");
    };
    assert_eq!(head_a.head_action_hash, created_a.action_hash);
    assert_eq!(
        batch.attempted[1].id, missing_id,
        "input order is preserved (index 1)"
    );
    assert!(
        matches!(batch.attempted[1].outcome, BatchHeadOutcome::Resolved(None)),
        "a genuinely unknown id is honest local absence — Resolved(None), \
         never Failed, never dropped from `attempted`"
    );
    assert_eq!(
        batch.attempted[2].id, id_b,
        "input order is preserved (index 2)"
    );
    let BatchHeadOutcome::Resolved(Some(head_b)) = &batch.attempted[2].outcome else {
        panic!("id_b must resolve to a head, not a failure or absence");
    };
    assert_eq!(head_b.head_action_hash, created_b.action_hash);

    // --- (b) a zero budget admits nothing: every id is `unattempted` (never
    // started), not an `attempted` entry — the two must never be confused,
    // and `stop_reason` names ordinary budget exhaustion, not a failure.
    let starved: BatchResolveHeadsOutput = conductor
        .call(
            &zome,
            "resolve_content_heads_local",
            BatchIdsInput {
                ids: vec![id_a.clone(), id_b.clone()],
                budget_ms: Some(0),
            },
        )
        .await;
    assert_eq!(starved.schema_version, 1);
    assert!(
        starved.attempted.is_empty(),
        "zero budget must start no per-id work at all"
    );
    assert_eq!(
        starved.unattempted,
        vec![id_a.clone(), id_b.clone()],
        "every id is reported unattempted, in order — NOT a failure, and NOT \
         the same as a resolved None"
    );
    assert_eq!(
        starved.stop_reason,
        Some(BatchStopReason::BudgetExhausted),
        "zero budget stops on ordinary budget exhaustion, never a shared failure"
    );

    // --- (c) `resolve_canonical_elections` mirrors the batch shape for the
    // bare DHT election. A declares its own root canonical; B never declares.
    let declared: ContentHeadOutput = conductor
        .call(
            &zome,
            "declare_canonical_content_head",
            DeclareCanonicalHeadInput {
                id: id_a.clone(),
                head_action_hash: ActionHashB64::from(created_a.action_hash.clone()).to_string(),
            },
        )
        .await;
    assert_eq!(declared.content_id, id_a);

    let elections: BatchResolveElectionsOutput = conductor
        .call(
            &zome,
            "resolve_canonical_elections",
            BatchIdsInput {
                ids: vec![id_a.clone(), id_b.clone()],
                budget_ms: None,
            },
        )
        .await;
    assert_eq!(elections.schema_version, 1);
    assert_eq!(elections.attempted.len(), 2);
    assert!(elections.unattempted.is_empty());
    assert!(elections.stop_reason.is_none());
    assert_eq!(elections.attempted[0].id, id_a);
    let BatchElectionOutcome::Resolved(Some(election_a)) = &elections.attempted[0].outcome else {
        panic!("A must have a canonical election after declaring");
    };
    assert!(
        !election_a.canonical_earned,
        "the god-mode scaffold declares STAGING, never earned"
    );
    assert_eq!(
        election_a.winner_target,
        ActionHashB64::from(declared.head_action_hash.clone()).to_string(),
        "the batched election answer names the same winner the declare returned"
    );
    assert_eq!(elections.attempted[1].id, id_b);
    assert!(
        matches!(
            elections.attempted[1].outcome,
            BatchElectionOutcome::Resolved(None)
        ),
        "B never declared a canonical head — honest absence, not an error"
    );

    Ok(())
}

/// Version chain is a CHAIN, not a star (2026-09-02, backlog
/// `content-store-update-content-targets-root-not-latest-version`).
///
/// `update_content` used to take the FIRST `IdToContent` link — `get_links`
/// arrival order, which kept answering the root create — so every update
/// targeted the root and `supersedes` named the root on every version. The
/// coordinator now selects the NEWEST link by (timestamp, create-link hash).
///
/// Proves on one conductor: create → update₁ → update₂ yields
///   head == update₂, head.supersedes == update₁ (not the create), and
///   get_content_by_id serves update₂'s title. Coordinator-only, no DNA-hash move.
#[tokio::test(flavor = "multi_thread")]
async fn update_of_update_supersedes_previous_version_not_root() -> Result<()> {
    let [(mut c1, a1), (_c2, _a2)] = two_agent_conductors().await?;
    let seed = network_seed(DNA);
    let dna_file = load_dna(DNA, &seed, Some(a1.clone())).await?;
    let app1 = c1
        .setup_app_for_agent("lamad-app", a1.clone(), &[dna_file])
        .await?;
    let cell1 = app1.cells().first().unwrap().clone();
    let zome1 = cell1.zome("content_store");

    // unique_id: a fixed id self-poisons a nextest retry (shared mem-bootstrap
    // DHT residue → create_content "already exists"; dna #1357).
    let id = unique_id("chain-not-star");
    let id = id.as_str();
    let created: ContentOutput = c1.call(&zome1, "create_content", test_content(id)).await;
    let create_action = created.action_hash.clone();

    let first: ContentOutput = c1
        .call(
            &zome1,
            "update_content",
            UpdateContentInput {
                id: id.to_string(),
                title: Some("version 1".to_string()),
            },
        )
        .await;
    let update1_action = first.action_hash.clone();
    assert_ne!(update1_action, create_action);

    let second: ContentOutput = c1
        .call(
            &zome1,
            "update_content",
            UpdateContentInput {
                id: id.to_string(),
                title: Some("version 2".to_string()),
            },
        )
        .await;
    let update2_action = second.action_hash.clone();
    assert_ne!(update2_action, update1_action);

    let head: Option<ContentHeadOutput> = c1
        .call(&zome1, "resolve_content_head", id.to_string())
        .await;
    let head = head.expect("resolve_content_head returned None on the author conductor");
    assert_eq!(
        head.head_action_hash, update2_action,
        "head is the second update"
    );
    assert_eq!(
        head.supersedes,
        Some(update1_action.clone()),
        "the second update supersedes the FIRST update, not the root create (star → chain)"
    );
    assert_ne!(
        head.supersedes,
        Some(create_action),
        "a version chain never re-parents onto the root"
    );

    let served: Option<ContentOutput> = c1
        .call(
            &zome1,
            "get_content_by_id",
            QueryByIdInput { id: id.to_string() },
        )
        .await;
    let served = served.expect("get_content_by_id returned None");
    assert_eq!(
        served.content.title, "version 2",
        "readers select the newest version"
    );

    Ok(())
}
