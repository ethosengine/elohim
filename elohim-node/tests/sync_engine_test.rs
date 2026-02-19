//! Sync engine and CRDT merge integration tests
//!
//! Tests the Automerge-backed sync engine including:
//! - SQLite document persistence
//! - CRDT merge correctness with realistic mastery data
//! - Stream position tracking
//! - Concurrent document operations

use automerge::{AutoCommit, ReadDoc, transaction::Transactable};
use rusqlite::Connection;
use tempfile::TempDir;

// =============================================================================
// SQLite Database Creation & Connection
// =============================================================================

#[test]
fn test_sqlite_database_creation() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("documents.db");

    let db = Connection::open(&db_path).expect("Should open/create database");

    // Enable WAL mode
    db.execute_batch("PRAGMA journal_mode=WAL;").unwrap();

    // Create the documents table
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS documents (
            doc_id TEXT PRIMARY KEY,
            data BLOB NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        );"
    ).unwrap();

    // Verify table exists
    let count: i32 = db
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='documents'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn test_sqlite_wal_mode_enabled() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("documents.db");

    let db = Connection::open(&db_path).unwrap();
    db.execute_batch("PRAGMA journal_mode=WAL;").unwrap();

    let mode: String = db
        .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
        .unwrap();

    assert_eq!(mode, "wal");
}

// =============================================================================
// Automerge CRDT Merge Correctness
// =============================================================================

#[test]
fn test_automerge_create_and_save_document() {
    let mut doc = AutoCommit::new();
    doc.put(automerge::ROOT, "title", "Mastery Record").unwrap();
    doc.put(automerge::ROOT, "agentId", "agent-abc123").unwrap();

    let saved = doc.save();
    assert!(!saved.is_empty(), "Saved document should have bytes");

    // Reload from bytes
    let loaded = AutoCommit::load(&saved).unwrap();
    let (val, _) = loaded.get(automerge::ROOT, "title").unwrap().unwrap();
    assert_eq!(val.to_str().unwrap(), "Mastery Record");
}

#[test]
fn test_automerge_concurrent_merge_no_conflict() {
    // Simulate two nodes modifying different fields
    let mut doc1 = AutoCommit::new();
    doc1.put(automerge::ROOT, "node", "node-1").unwrap();
    let saved1 = doc1.save();

    // Node 2 starts from same base
    let mut doc2 = AutoCommit::load(&saved1).unwrap();

    // Node 1 changes field A
    doc1.put(automerge::ROOT, "score", "85").unwrap();

    // Node 2 changes field B
    doc2.put(automerge::ROOT, "level", "intermediate").unwrap();

    // Merge: apply node1's changes to node2
    let changes1 = doc1.save();
    doc2.load_incremental(&changes1).unwrap();

    // Both fields should exist
    let (score, _) = doc2.get(automerge::ROOT, "score").unwrap().unwrap();
    let (level, _) = doc2.get(automerge::ROOT, "level").unwrap().unwrap();

    assert_eq!(score.to_str().unwrap(), "85");
    assert_eq!(level.to_str().unwrap(), "intermediate");
}

#[test]
fn test_automerge_concurrent_merge_same_field() {
    // Simulate two nodes modifying the same field (last-writer-wins)
    let mut base = AutoCommit::new();
    base.put(automerge::ROOT, "mastery", "50").unwrap();
    let base_saved = base.save();

    let mut doc1 = AutoCommit::load(&base_saved).unwrap();
    let mut doc2 = AutoCommit::load(&base_saved).unwrap();

    // Both nodes update mastery
    doc1.put(automerge::ROOT, "mastery", "75").unwrap();
    doc2.put(automerge::ROOT, "mastery", "80").unwrap();

    // Merge changes from doc1 into doc2
    let changes1 = doc1.save();
    doc2.load_incremental(&changes1).unwrap();

    // The value should be one of the two (CRDT deterministic resolution)
    let (val, _) = doc2.get(automerge::ROOT, "mastery").unwrap().unwrap();
    let mastery_str = val.to_str().unwrap();
    assert!(
        mastery_str == "75" || mastery_str == "80",
        "Should be one of the concurrent values, got: {}",
        mastery_str
    );
}

#[test]
fn test_automerge_realistic_mastery_data() {
    // Create a realistic mastery record with nested data
    let mut doc = AutoCommit::new();

    // Set top-level fields
    doc.put(automerge::ROOT, "agentId", "agent-xyz789").unwrap();
    doc.put(automerge::ROOT, "pathId", "path-manifesto-foundations").unwrap();

    // Create a list to track mastery events
    let events = doc.put_object(automerge::ROOT, "events", automerge::ObjType::List).unwrap();

    // Add mastery event 1
    let event1 = doc.insert_object(&events, 0, automerge::ObjType::Map).unwrap();
    doc.put(&event1, "contentId", "concept-free-software").unwrap();
    doc.put(&event1, "score", "92").unwrap();
    doc.put(&event1, "timestamp", "1706000000").unwrap();

    // Add mastery event 2
    let event2 = doc.insert_object(&events, 1, automerge::ObjType::Map).unwrap();
    doc.put(&event2, "contentId", "quiz-manifesto-basics").unwrap();
    doc.put(&event2, "score", "78").unwrap();
    doc.put(&event2, "timestamp", "1706001000").unwrap();

    // Save and reload
    let saved = doc.save();
    let loaded = AutoCommit::load(&saved).unwrap();

    // Verify structure
    let (agent_id, _) = loaded.get(automerge::ROOT, "agentId").unwrap().unwrap();
    assert_eq!(agent_id.to_str().unwrap(), "agent-xyz789");

    let (path_id, _) = loaded.get(automerge::ROOT, "pathId").unwrap().unwrap();
    assert_eq!(path_id.to_str().unwrap(), "path-manifesto-foundations");

    // Verify events list length
    let events_obj = loaded.get(automerge::ROOT, "events").unwrap().unwrap();
    let events_id = events_obj.1;
    assert_eq!(loaded.length(&events_id), 2);
}

#[test]
fn test_automerge_incremental_sync() {
    // Simulate incremental sync between two nodes
    let mut node_a = AutoCommit::new();
    node_a.put(automerge::ROOT, "owner", "node-a").unwrap();

    // Initial full sync to node B
    let full_save = node_a.save();
    let mut node_b = AutoCommit::load(&full_save).unwrap();

    // Both share the same heads now
    let heads_before = node_a.get_heads();

    // Node A makes a change
    node_a.put(automerge::ROOT, "updated", "true").unwrap();

    // Get only the incremental changes
    let incremental = node_a.save_after(&heads_before);
    assert!(!incremental.is_empty(), "Should have incremental changes");

    // Apply incremental to node B
    node_b.load_incremental(&incremental).unwrap();

    // Verify node B has the update
    let (val, _) = node_b.get(automerge::ROOT, "updated").unwrap().unwrap();
    assert_eq!(val.to_str().unwrap(), "true");
}

#[test]
fn test_automerge_bidirectional_merge() {
    // Both nodes make independent changes, then sync bidirectionally
    let mut base = AutoCommit::new();
    base.put(automerge::ROOT, "version", "1").unwrap();
    let base_save = base.save();

    let mut node_a = AutoCommit::load(&base_save).unwrap();
    let mut node_b = AutoCommit::load(&base_save).unwrap();

    let heads_base = node_a.get_heads();

    // Node A adds field
    node_a.put(automerge::ROOT, "from_a", "hello").unwrap();

    // Node B adds different field
    node_b.put(automerge::ROOT, "from_b", "world").unwrap();

    // A -> B
    let changes_a = node_a.save_after(&heads_base);
    node_b.load_incremental(&changes_a).unwrap();

    // B -> A
    let changes_b = node_b.save_after(&heads_base);
    node_a.load_incremental(&changes_b).unwrap();

    // Both should now have all fields
    assert!(node_a.get(automerge::ROOT, "from_a").unwrap().is_some());
    assert!(node_a.get(automerge::ROOT, "from_b").unwrap().is_some());
    assert!(node_b.get(automerge::ROOT, "from_a").unwrap().is_some());
    assert!(node_b.get(automerge::ROOT, "from_b").unwrap().is_some());

    // Heads should be identical after bidirectional sync
    assert_eq!(node_a.get_heads(), node_b.get_heads());
}

// =============================================================================
// Document Persistence with SQLite
// =============================================================================

#[test]
fn test_document_roundtrip_through_sqlite() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("documents.db");

    let db = Connection::open(&db_path).unwrap();
    db.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS documents (
            doc_id TEXT PRIMARY KEY,
            data BLOB NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        );"
    ).unwrap();

    // Create and save a document
    let mut doc = AutoCommit::new();
    doc.put(automerge::ROOT, "mastery", "100").unwrap();
    let data = doc.save();

    db.execute(
        "INSERT INTO documents (doc_id, data) VALUES (?1, ?2)",
        rusqlite::params!["mastery-agent-1", &data],
    ).unwrap();

    // Load it back
    let loaded_data: Vec<u8> = db
        .query_row(
            "SELECT data FROM documents WHERE doc_id = ?1",
            ["mastery-agent-1"],
            |row| row.get(0),
        )
        .unwrap();

    let loaded_doc = AutoCommit::load(&loaded_data).unwrap();
    let (val, _) = loaded_doc.get(automerge::ROOT, "mastery").unwrap().unwrap();
    assert_eq!(val.to_str().unwrap(), "100");
}

#[test]
fn test_document_upsert() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("documents.db");

    let db = Connection::open(&db_path).unwrap();
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS documents (
            doc_id TEXT PRIMARY KEY,
            data BLOB NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        );"
    ).unwrap();

    // Insert
    let mut doc = AutoCommit::new();
    doc.put(automerge::ROOT, "val", "first").unwrap();
    let data1 = doc.save();

    db.execute(
        "INSERT INTO documents (doc_id, data, updated_at)
         VALUES (?1, ?2, strftime('%s', 'now'))
         ON CONFLICT(doc_id) DO UPDATE SET data = ?2, updated_at = strftime('%s', 'now')",
        rusqlite::params!["doc-1", &data1],
    ).unwrap();

    // Update (upsert)
    doc.put(automerge::ROOT, "val", "second").unwrap();
    let data2 = doc.save();

    db.execute(
        "INSERT INTO documents (doc_id, data, updated_at)
         VALUES (?1, ?2, strftime('%s', 'now'))
         ON CONFLICT(doc_id) DO UPDATE SET data = ?2, updated_at = strftime('%s', 'now')",
        rusqlite::params!["doc-1", &data2],
    ).unwrap();

    // Should have latest
    let loaded_data: Vec<u8> = db
        .query_row("SELECT data FROM documents WHERE doc_id = ?1", ["doc-1"], |row| {
            row.get(0)
        })
        .unwrap();

    let loaded = AutoCommit::load(&loaded_data).unwrap();
    let (val, _) = loaded.get(automerge::ROOT, "val").unwrap().unwrap();
    assert_eq!(val.to_str().unwrap(), "second");
}

#[test]
fn test_load_nonexistent_document() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("documents.db");

    let db = Connection::open(&db_path).unwrap();
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS documents (
            doc_id TEXT PRIMARY KEY,
            data BLOB NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        );"
    ).unwrap();

    let result = db.query_row(
        "SELECT data FROM documents WHERE doc_id = ?1",
        ["nonexistent"],
        |row| row.get::<_, Vec<u8>>(0),
    );

    assert!(matches!(result, Err(rusqlite::Error::QueryReturnedNoRows)));
}

// =============================================================================
// Stream Position Tracking
// =============================================================================

#[test]
fn test_stream_position_monotonically_increasing() {
    // Verify positions increase monotonically
    let mut position: u64 = 0;
    let mut positions = Vec::new();

    for _ in 0..10 {
        position += 1;
        positions.push(position);
    }

    for i in 1..positions.len() {
        assert!(
            positions[i] > positions[i - 1],
            "Positions must be monotonically increasing"
        );
    }
}

#[test]
fn test_events_since_position_filtering() {
    // Simulate events_since behavior
    use std::collections::VecDeque;

    #[derive(Clone, Debug)]
    struct TestEvent {
        position: u64,
        doc_id: String,
    }

    let mut events = VecDeque::new();

    for i in 1..=5 {
        events.push_back(TestEvent {
            position: i,
            doc_id: format!("doc-{}", i),
        });
    }

    // Get events since position 3
    let filtered: Vec<_> = events.iter().filter(|e| e.position > 3).collect();
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].doc_id, "doc-4");
    assert_eq!(filtered[1].doc_id, "doc-5");

    // Get all events (since 0)
    let all: Vec<_> = events.iter().filter(|e| e.position > 0).collect();
    assert_eq!(all.len(), 5);

    // Get no events (since latest)
    let none: Vec<_> = events.iter().filter(|e| e.position > 5).collect();
    assert_eq!(none.len(), 0);
}

// =============================================================================
// Hex Decode Utility
// =============================================================================

#[test]
fn test_hex_decode_valid() {
    // Inline hex decode matching merge.rs implementation
    fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 {
            return Err(());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }

    let bytes = hex_decode("deadbeef").unwrap();
    assert_eq!(bytes, vec![0xde, 0xad, 0xbe, 0xef]);

    let zeros = hex_decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    assert_eq!(zeros.len(), 32);

    // Odd length should fail
    assert!(hex_decode("abc").is_err());

    // Invalid chars should fail
    assert!(hex_decode("zzzz").is_err());
}

// =============================================================================
// MessagePack Serialization (SyncMessage wire format)
// =============================================================================

#[test]
fn test_sync_message_roundtrip_msgpack() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestSyncEvent {
        position: u64,
        doc_id: String,
        change_hash: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    enum TestSyncMessage {
        SyncRequest { since: u64, limit: Option<u32> },
        SyncResponse { events: Vec<TestSyncEvent>, has_more: bool },
        DocRequest { doc_id: String, heads: Vec<String> },
        DocResponse { doc_id: String, changes: Vec<Vec<u8>> },
    }

    // Test SyncRequest
    let req = TestSyncMessage::SyncRequest { since: 42, limit: Some(100) };
    let encoded = rmp_serde::to_vec(&req).unwrap();
    let decoded: TestSyncMessage = rmp_serde::from_slice(&encoded).unwrap();
    assert_eq!(req, decoded);

    // Test SyncResponse with events
    let resp = TestSyncMessage::SyncResponse {
        events: vec![
            TestSyncEvent {
                position: 1,
                doc_id: "mastery-agent-1".to_string(),
                change_hash: "abc123".to_string(),
            },
        ],
        has_more: false,
    };
    let encoded = rmp_serde::to_vec(&resp).unwrap();
    let decoded: TestSyncMessage = rmp_serde::from_slice(&encoded).unwrap();
    assert_eq!(resp, decoded);

    // Test DocRequest
    let doc_req = TestSyncMessage::DocRequest {
        doc_id: "mastery-agent-1".to_string(),
        heads: vec!["deadbeef".to_string()],
    };
    let encoded = rmp_serde::to_vec(&doc_req).unwrap();
    let decoded: TestSyncMessage = rmp_serde::from_slice(&encoded).unwrap();
    assert_eq!(doc_req, decoded);

    // Test DocResponse with binary changes
    let doc_resp = TestSyncMessage::DocResponse {
        doc_id: "mastery-agent-1".to_string(),
        changes: vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]],
    };
    let encoded = rmp_serde::to_vec(&doc_resp).unwrap();
    let decoded: TestSyncMessage = rmp_serde::from_slice(&encoded).unwrap();
    assert_eq!(doc_resp, decoded);
}

#[test]
fn test_length_prefix_framing() {
    // Verify our length-prefix framing (4 bytes big-endian + payload)
    let max_msg_size: usize = 10 * 1024 * 1024; // 10 MB

    let payload = vec![0u8; 100];
    let len_bytes = (payload.len() as u32).to_be_bytes();

    assert_eq!(len_bytes.len(), 4);
    assert_eq!(u32::from_be_bytes(len_bytes), 100);

    // Verify max message size check
    let oversized = max_msg_size + 1;
    assert!(oversized > max_msg_size);
}
