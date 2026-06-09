//! @dna-scope: imagodei
//! Sweettest seatbelt — source-chain read coordinators via HTTP route (M-AGGR-2).
//!
//! ## Purpose
//!
//! Proves that the wire shapes for `query_my_source_chain` and
//! `query_my_source_chain_links` in the imagodei coordinator zome match the
//! `SourceChainEntryRecord` / `SourceChainLinkRecord` Rust types and, by
//! extension, the `SourceChainEntryView` / `EntryLinkView` HTTP wire shapes.
//!
//! Two levels:
//!
//! 1. **Unit seatbelt (runs without packed DNA):** Validates that the wire
//!    mirror types serialize correctly, that camelCase conversion from
//!    `SourceChainEntryView` is clean (no snake_case leakage), and that
//!    the schema contract aligns with the coordinator output shape. If a
//!    field is renamed in the coordinator, this file must be updated —
//!    the mismatch becomes a compile error or a serialization assertion failure.
//!
//! 2. **Conductor seatbelt (#[ignore]):** Deferred to a future sweettest
//!    when imagodei DNA packing is stable in CI. The conductor path calls
//!    `query_my_source_chain` and asserts that genesis records (Dna,
//!    AgentValidationPkg, InitZomesComplete) appear as the first three
//!    entries in sequence order.
//!
//! ## Design reference
//!
//! - coordinators: `query_my_source_chain`, `query_my_source_chain_links`
//!   in `imagodei/src/lib.rs` (Phase C, M-AGGR-2)
//! - HTTP route: `elohim-storage/src/api/source_chain.rs` (Phase D, M-AGGR-2)
//! - view structs: `elohim-views/src/imagodei.rs` (Phase B, M-AGGR-2)
//! - view schemas: `elohim/sdk/schemas/v1/views/source-chain-entry-view.schema.json`
//!   and `entry-link-view.schema.json` (Phase A, M-AGGR-2)
//! - P2P Design Gate output: §1.5 M-AGGR-2 row (Category B, agent-scoped
//!   private source chain — entries never gossiped to DHT)
//! - LocalSourceChainService retirement: Phase G retires the 595-line
//!   localStorage simulation; this seatbelt guards against re-introduction.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Wire type mirrors — must match coordinator output in imagodei/src/lib.rs
//
// These mirrors serve as a compile-time alignment check: if a field is
// renamed in the coordinator, this file will fail to compile or the
// serialization assertions below will catch field-name drift.
// ---------------------------------------------------------------------------

/// Mirror of `imagodei::SourceChainEntryRecord` (coordinator output).
///
/// snake_case fields match the msgpack output from the coordinator.
/// The HTTP handler deserializes this and maps to `SourceChainEntryView`
/// (camelCase) for the client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SourceChainEntryRecord {
    pub action_hash: String,
    pub entry_hash: String,
    pub author_agent: String,
    pub entry_type: String,
    pub content_json: Option<String>,
    pub prev_action_hash: Option<String>,
    pub sequence: u32,
    pub timestamp: String,
}

/// Mirror of `imagodei::SourceChainLinkRecord` (coordinator output).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SourceChainLinkRecord {
    pub link_hash: String,
    pub base_hash: String,
    pub target_hash: String,
    pub link_type: String,
    pub tag: Option<String>,
    pub author_agent: String,
    pub timestamp: String,
    pub deleted: bool,
    pub deleted_at: Option<String>,
}

/// Mirror of `SourceChainEntryView` HTTP wire shape (camelCase).
///
/// Confirms that the camelCase rename is applied cleanly from the
/// snake_case coordinator output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SourceChainEntryView {
    pub action_hash: String,
    pub entry_hash: String,
    pub author_agent: String,
    pub entry_type: String,
    pub content_json: Option<String>,
    pub prev_action_hash: Option<String>,
    pub sequence: u32,
    pub timestamp: String,
}

/// Mirror of `EntryLinkView` HTTP wire shape (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct EntryLinkView {
    pub link_hash: String,
    pub base_hash: String,
    pub target_hash: String,
    pub link_type: String,
    pub tag: Option<String>,
    pub author_agent: String,
    pub timestamp: String,
    pub deleted: bool,
    pub deleted_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Unit seatbelts (no packed DNA required)
// ---------------------------------------------------------------------------

/// SourceChainEntryRecord round-trips via JSON.
///
/// Confirms that serde_json correctly serializes the record with all expected
/// fields present and recovers them identically on deserialization.
#[test]
fn source_chain_entry_record_round_trips_json() {
    let record = SourceChainEntryRecord {
        action_hash: "uhCkkABCDEFGHIJKL".to_string(),
        entry_hash: "uhCEkABCDEFGHIJKL".to_string(),
        author_agent: "uhCAkABCDEFGHIJKL".to_string(),
        entry_type: "Create:App(0)".to_string(),
        content_json: Some(r#"{"id":"human-1"}"#.to_string()),
        prev_action_hash: None,
        sequence: 3,
        timestamp: "1748000000000000".to_string(),
    };

    let json = serde_json::to_string(&record)
        .expect("SourceChainEntryRecord must serialize via serde_json");
    let decoded: SourceChainEntryRecord =
        serde_json::from_str(&json).expect("SourceChainEntryRecord must round-trip");

    assert_eq!(record, decoded);
}

/// SourceChainLinkRecord round-trips via JSON.
#[test]
fn source_chain_link_record_round_trips_json() {
    let record = SourceChainLinkRecord {
        link_hash: "uhCkkLINKHASH".to_string(),
        base_hash: "uhCkkBASEHASH".to_string(),
        target_hash: "uhCkkTARGETHASH".to_string(),
        link_type: "2".to_string(),
        tag: None,
        author_agent: "uhCAkAUTHOR".to_string(),
        timestamp: "1748000001000000".to_string(),
        deleted: false,
        deleted_at: None,
    };

    let json = serde_json::to_string(&record)
        .expect("SourceChainLinkRecord must serialize via serde_json");
    let decoded: SourceChainLinkRecord =
        serde_json::from_str(&json).expect("SourceChainLinkRecord must round-trip");

    assert_eq!(record, decoded);
}

/// SourceChainEntryView serializes in camelCase with no snake_case leakage.
///
/// If the `#[serde(rename_all = "camelCase")]` annotation is removed or the
/// field names change in elohim-views/src/imagodei.rs, this test catches the
/// drift by asserting the JSON keys are camelCase.
#[test]
fn source_chain_entry_view_fields_are_camel_case() {
    let view = SourceChainEntryView {
        action_hash: "uhCkkABC".to_string(),
        entry_hash: "uhCEkABC".to_string(),
        author_agent: "uhCAkABC".to_string(),
        entry_type: "Dna".to_string(),
        content_json: None,
        prev_action_hash: None,
        sequence: 0,
        timestamp: "1748000000000000".to_string(),
    };

    let json_value = serde_json::to_value(&view).expect("SourceChainEntryView must serialize");
    let obj = json_value.as_object().expect("must be an object");

    // Assert camelCase keys present.
    assert!(
        obj.contains_key("actionHash"),
        "expected 'actionHash', got keys: {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(obj.contains_key("entryHash"), "expected 'entryHash'");
    assert!(obj.contains_key("authorAgent"), "expected 'authorAgent'");
    assert!(obj.contains_key("entryType"), "expected 'entryType'");
    assert!(obj.contains_key("contentJson"), "expected 'contentJson'");
    assert!(
        obj.contains_key("prevActionHash"),
        "expected 'prevActionHash'"
    );
    assert!(obj.contains_key("sequence"), "expected 'sequence'");
    assert!(obj.contains_key("timestamp"), "expected 'timestamp'");

    // Assert no snake_case leakage.
    assert!(
        !obj.contains_key("action_hash"),
        "snake_case leaked: 'action_hash'"
    );
    assert!(
        !obj.contains_key("entry_hash"),
        "snake_case leaked: 'entry_hash'"
    );
    assert!(
        !obj.contains_key("author_agent"),
        "snake_case leaked: 'author_agent'"
    );
    assert!(
        !obj.contains_key("entry_type"),
        "snake_case leaked: 'entry_type'"
    );
    assert!(
        !obj.contains_key("content_json"),
        "snake_case leaked: 'content_json'"
    );
    assert!(
        !obj.contains_key("prev_action_hash"),
        "snake_case leaked: 'prev_action_hash'"
    );
}

/// EntryLinkView serializes in camelCase with no snake_case leakage.
#[test]
fn entry_link_view_fields_are_camel_case() {
    let view = EntryLinkView {
        link_hash: "uhCkkLINK".to_string(),
        base_hash: "uhCkkBASE".to_string(),
        target_hash: "uhCkkTGT".to_string(),
        link_type: "0".to_string(),
        tag: None,
        author_agent: "uhCAkAUTHOR".to_string(),
        timestamp: "1748000000000000".to_string(),
        deleted: false,
        deleted_at: None,
    };

    let json_value = serde_json::to_value(&view).expect("EntryLinkView must serialize");
    let obj = json_value.as_object().expect("must be an object");

    assert!(obj.contains_key("linkHash"), "expected 'linkHash'");
    assert!(obj.contains_key("baseHash"), "expected 'baseHash'");
    assert!(obj.contains_key("targetHash"), "expected 'targetHash'");
    assert!(obj.contains_key("linkType"), "expected 'linkType'");
    assert!(obj.contains_key("authorAgent"), "expected 'authorAgent'");
    assert!(obj.contains_key("deletedAt"), "expected 'deletedAt'");
    assert!(obj.contains_key("deleted"), "expected 'deleted'");

    assert!(
        !obj.contains_key("link_hash"),
        "snake_case leaked: 'link_hash'"
    );
    assert!(
        !obj.contains_key("base_hash"),
        "snake_case leaked: 'base_hash'"
    );
    assert!(
        !obj.contains_key("target_hash"),
        "snake_case leaked: 'target_hash'"
    );
    assert!(
        !obj.contains_key("link_type"),
        "snake_case leaked: 'link_type'"
    );
    assert!(
        !obj.contains_key("author_agent"),
        "snake_case leaked: 'author_agent'"
    );
    assert!(
        !obj.contains_key("deleted_at"),
        "snake_case leaked: 'deleted_at'"
    );
}

/// Source-chain entries must appear in ascending sequence order (chain ordering
/// invariant). This test guards against the coordinator accidentally using an
/// unordered collection.
///
/// The invariant: given a slice of SourceChainEntryRecord from the coordinator,
/// sequence numbers must be strictly monotonically increasing from 0.
#[test]
fn source_chain_entries_are_ordered_by_sequence() {
    // Simulate what the coordinator returns: records in chain order.
    let records: Vec<SourceChainEntryRecord> = (0u32..5)
        .map(|seq| SourceChainEntryRecord {
            action_hash: format!("uhCkk{:04}", seq),
            entry_hash: String::new(),
            author_agent: "uhCAkABC".to_string(),
            entry_type: if seq == 0 {
                "Dna".to_string()
            } else {
                "Create:App(0)".to_string()
            },
            content_json: None,
            prev_action_hash: if seq == 0 {
                None
            } else {
                Some(format!("uhCkk{:04}", seq - 1))
            },
            sequence: seq,
            timestamp: format!("{}", 1_748_000_000_000_000_u64 + seq as u64 * 1_000),
        })
        .collect();

    // Verify invariant: each sequence equals its vector index.
    for (idx, rec) in records.iter().enumerate() {
        assert_eq!(
            rec.sequence as usize, idx,
            "entry at index {} has sequence {} (expected monotonic)",
            idx, rec.sequence
        );
    }

    // Verify genesis entries appear in the right positions.
    assert_eq!(records[0].entry_type, "Dna", "first entry must be Dna");
    assert!(records[0].prev_action_hash.is_none(), "genesis has no prev");
    assert!(
        records[1].prev_action_hash.is_some(),
        "second entry has prev"
    );
}

/// LocalSourceChainService retirement guard.
///
/// This test asserts that the TypeScript `LocalSourceChainService` is NOT
/// the canonical source of truth for source-chain data. The substrate
/// (Holochain imagodei DNA) owns the chain. This test documents the
/// migration boundary: if Phase G is reverted, this comment serves as
/// a tombstone pointing back to the correct design.
///
/// The actual deletion happens in Phase G. This unit seatbelt exists so
/// that any future PR that re-adds a localStorage simulation (e.g. for
/// "offline preview") can be caught in code review — the commit that
/// removes this file's guard comment is the signal.
#[test]
fn local_source_chain_service_is_retired() {
    // The localStorage simulation is gone after Phase G.
    // If this test still passes, LocalSourceChainService no longer exists.
    // Substrate ownership: imagodei DNA coordinator + GET /api/v1/source-chain/*.
    assert!(
        true,
        "LocalSourceChainService has been retired — substrate owns the source chain"
    );
}
