//! Integration tests for two-node Automerge sync
//!
//! These tests simulate the sync protocol exchange between two nodes
//! without requiring actual network connectivity.

use automerge::transaction::Transactable;
use automerge::ReadDoc;
use automerge::Automerge;
use elohim_storage::sync::{DocStore, DocStoreConfig, SyncManager, StreamTracker};
use std::sync::Arc;
use tempfile::TempDir;

/// Default app_id for tests
const TEST_APP_ID: &str = "lamad";

/// Helper to create a SyncManager with a temporary storage directory
async fn create_sync_manager(name: &str) -> (SyncManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let doc_store = Arc::new(
        DocStore::new(DocStoreConfig {
            db_path: temp_dir.path().join(format!("{}.sled", name)),
            ..Default::default()
        })
        .await
        .unwrap(),
    );
    let stream_tracker = Arc::new(StreamTracker::new());
    let manager = SyncManager::new(doc_store, stream_tracker);
    (manager, temp_dir)
}

/// Test basic document creation and retrieval
#[tokio::test]
async fn test_single_node_document_lifecycle() {
    let (manager, _temp) = create_sync_manager("node1").await;

    // Create a document
    let mut doc = manager.get_or_create_doc(TEST_APP_ID, "graph:test").await.unwrap();

    // Make some changes
    doc.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "title", "Test Graph")?;
        tx.put(automerge::ROOT, "version", 1)?;
        Ok(())
    })
    .unwrap();

    // Get heads before saving
    let heads_before: Vec<String> = doc
        .get_heads()
        .iter()
        .map(|h| hex::encode(h.0))
        .collect();

    // Save via apply_changes (simulating receiving changes)
    let changes = doc.save();
    manager.apply_changes(TEST_APP_ID, "graph:test", vec![changes]).await.unwrap();

    // Retrieve and verify
    let heads = manager.get_heads(TEST_APP_ID, "graph:test").await.unwrap();
    assert!(!heads.is_empty());

    // List documents
    let (docs, total) = manager.list_documents(TEST_APP_ID, Some("graph"), 0, 10).await.unwrap();
    assert_eq!(total, 1);
    assert_eq!(docs[0].doc_id, "graph:test");
}

/// Test two nodes syncing a document
#[tokio::test]
async fn test_two_node_sync() {
    let (node1, _temp1) = create_sync_manager("node1").await;
    let (node2, _temp2) = create_sync_manager("node2").await;

    // Node 1 creates a document
    let mut doc = Automerge::new();
    doc.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "title", "Shared Document")?;
        tx.put(automerge::ROOT, "counter", 0)?;
        Ok(())
    })
    .unwrap();

    let doc_id = "graph:shared";

    // Save to node 1
    let changes = doc.save();
    let node1_heads = node1.apply_changes(TEST_APP_ID, doc_id, vec![changes.clone()]).await.unwrap();

    // Node 2 doesn't have the document yet
    let node2_heads = node2.get_heads(TEST_APP_ID, doc_id).await.unwrap();
    assert!(node2_heads.is_empty());

    // Simulate sync: node 2 asks node 1 for changes since empty heads
    let (changes_to_send, _) = node1.get_changes_since(TEST_APP_ID, doc_id, &[]).await.unwrap();
    assert!(!changes_to_send.is_empty());

    // Node 2 applies the changes
    let node2_heads_after = node2.apply_changes(TEST_APP_ID, doc_id, changes_to_send).await.unwrap();

    // Both nodes should have the same heads
    assert_eq!(node1_heads, node2_heads_after);

    // Verify content on node 2
    let doc2 = node2.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();
    let title: Option<String> = doc2
        .get(automerge::ROOT, "title")
        .unwrap()
        .and_then(|(v, _)| {
            if let automerge::Value::Scalar(s) = v {
                if let automerge::ScalarValue::Str(smol) = s.as_ref() {
                    return Some(smol.to_string());
                }
            }
            None
        });
    assert_eq!(title, Some("Shared Document".to_string()));
}

/// Test offline changes merging correctly after reconnection
#[tokio::test]
async fn test_offline_merge() {
    let (node1, _temp1) = create_sync_manager("node1").await;
    let (node2, _temp2) = create_sync_manager("node2").await;

    let doc_id = "personal:notes";

    // Both nodes start with same document
    let mut initial_doc = Automerge::new();
    initial_doc
        .transact::<_, _, automerge::AutomergeError>(|tx| {
            tx.put(automerge::ROOT, "note", "Initial note")?;
            Ok(())
        })
        .unwrap();

    let initial_changes = initial_doc.save();
    node1.apply_changes(TEST_APP_ID, doc_id, vec![initial_changes.clone()]).await.unwrap();
    node2.apply_changes(TEST_APP_ID, doc_id, vec![initial_changes]).await.unwrap();

    // Verify both nodes have same heads initially
    let node1_heads_initial = node1.get_heads(TEST_APP_ID, doc_id).await.unwrap();
    let node2_heads_initial = node2.get_heads(TEST_APP_ID, doc_id).await.unwrap();
    assert_eq!(node1_heads_initial, node2_heads_initial);

    // --- OFFLINE: Both nodes make independent changes ---

    // Node 1 adds a field
    let mut doc1 = node1.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();
    doc1.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "node1_field", "from node 1")?;
        Ok(())
    })
    .unwrap();
    let node1_offline_changes = doc1.save_after(
        &node1_heads_initial
            .iter()
            .filter_map(|h| {
                let bytes = hex::decode(h).ok()?;
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Some(automerge::ChangeHash(arr))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>(),
    );
    node1.apply_changes(TEST_APP_ID, doc_id, vec![doc1.save()]).await.unwrap();

    // Node 2 adds a different field
    let mut doc2 = node2.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();
    doc2.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "node2_field", "from node 2")?;
        Ok(())
    })
    .unwrap();
    node2.apply_changes(TEST_APP_ID, doc_id, vec![doc2.save()]).await.unwrap();

    // Heads should now be different
    let node1_heads_offline = node1.get_heads(TEST_APP_ID, doc_id).await.unwrap();
    let node2_heads_offline = node2.get_heads(TEST_APP_ID, doc_id).await.unwrap();
    assert_ne!(node1_heads_offline, node2_heads_offline);

    // --- RECONNECT: Sync changes ---

    // Node 1 sends changes to node 2 (changes since node2's known heads)
    let (changes_1_to_2, _) = node1
        .get_changes_since(TEST_APP_ID, doc_id, &node1_heads_initial)
        .await
        .unwrap();

    // Node 2 sends changes to node 1
    let (changes_2_to_1, _) = node2
        .get_changes_since(TEST_APP_ID, doc_id, &node2_heads_initial)
        .await
        .unwrap();

    // Both apply each other's changes
    node2.apply_changes(TEST_APP_ID, doc_id, changes_1_to_2).await.unwrap();
    node1.apply_changes(TEST_APP_ID, doc_id, changes_2_to_1).await.unwrap();

    // --- VERIFY: Both nodes have merged state ---

    let node1_heads_final = node1.get_heads(TEST_APP_ID, doc_id).await.unwrap();
    let node2_heads_final = node2.get_heads(TEST_APP_ID, doc_id).await.unwrap();

    // Heads should now be the same (both have all changes)
    let mut node1_sorted = node1_heads_final.clone();
    let mut node2_sorted = node2_heads_final.clone();
    node1_sorted.sort();
    node2_sorted.sort();
    assert_eq!(node1_sorted, node2_sorted);

    // Verify both nodes have both fields
    let doc1_final = node1.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();
    let doc2_final = node2.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();

    let get_str = |doc: &Automerge, key: &str| -> Option<String> {
        doc.get(automerge::ROOT, key)
            .unwrap()
            .and_then(|(v, _)| {
                if let automerge::Value::Scalar(s) = v {
                    if let automerge::ScalarValue::Str(smol) = s.as_ref() {
                        return Some(smol.to_string());
                    }
                }
                None
            })
    };

    // Node 1 should have both fields
    assert_eq!(get_str(&doc1_final, "node1_field"), Some("from node 1".to_string()));
    assert_eq!(get_str(&doc1_final, "node2_field"), Some("from node 2".to_string()));

    // Node 2 should have both fields
    assert_eq!(get_str(&doc2_final, "node1_field"), Some("from node 1".to_string()));
    assert_eq!(get_str(&doc2_final, "node2_field"), Some("from node 2".to_string()));
}

/// Test incremental sync (only sending missing changes)
#[tokio::test]
async fn test_incremental_sync() {
    let (node1, _temp1) = create_sync_manager("node1").await;
    let (node2, _temp2) = create_sync_manager("node2").await;

    let doc_id = "graph:incremental";

    // Create initial document on node 1
    let mut doc = Automerge::new();
    doc.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "version", 1)?;
        Ok(())
    })
    .unwrap();

    node1.apply_changes(TEST_APP_ID, doc_id, vec![doc.save()]).await.unwrap();

    // Sync to node 2
    let (initial_changes, _) = node1.get_changes_since(TEST_APP_ID, doc_id, &[]).await.unwrap();
    let shared_heads = node2.apply_changes(TEST_APP_ID, doc_id, initial_changes).await.unwrap();

    // Node 1 makes more changes
    let mut doc1 = node1.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();
    doc1.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "version", 2)?;
        Ok(())
    })
    .unwrap();
    node1.apply_changes(TEST_APP_ID, doc_id, vec![doc1.save()]).await.unwrap();

    // Request only changes since shared_heads (incremental)
    let (incremental_changes, new_heads) = node1
        .get_changes_since(TEST_APP_ID, doc_id, &shared_heads)
        .await
        .unwrap();

    // Should have some changes
    assert!(!incremental_changes.is_empty() || !new_heads.is_empty());

    // Apply incremental changes to node 2
    if !incremental_changes.is_empty() {
        node2.apply_changes(TEST_APP_ID, doc_id, incremental_changes).await.unwrap();
    }

    // Verify node 2 has version 2
    let doc2 = node2.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();
    let version: Option<i64> = doc2
        .get(automerge::ROOT, "version")
        .unwrap()
        .and_then(|(v, _)| {
            if let automerge::Value::Scalar(s) = v {
                if let automerge::ScalarValue::Int(i) = s.as_ref() {
                    return Some(*i);
                }
            }
            None
        });
    assert_eq!(version, Some(2));
}

/// Test concurrent edits to the same field (CRDT conflict resolution)
#[tokio::test]
async fn test_concurrent_same_field_edits() {
    let (node1, _temp1) = create_sync_manager("node1").await;
    let (node2, _temp2) = create_sync_manager("node2").await;

    let doc_id = "test:conflict";

    // Initial document
    let mut initial_doc = Automerge::new();
    initial_doc
        .transact::<_, _, automerge::AutomergeError>(|tx| {
            tx.put(automerge::ROOT, "value", "initial")?;
            Ok(())
        })
        .unwrap();

    let initial_changes = initial_doc.save();
    node1.apply_changes(TEST_APP_ID, doc_id, vec![initial_changes.clone()]).await.unwrap();
    node2.apply_changes(TEST_APP_ID, doc_id, vec![initial_changes]).await.unwrap();

    let initial_heads = node1.get_heads(TEST_APP_ID, doc_id).await.unwrap();

    // Both nodes edit the same field concurrently
    let mut doc1 = node1.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();
    doc1.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "value", "node1 wins")?;
        Ok(())
    })
    .unwrap();
    node1.apply_changes(TEST_APP_ID, doc_id, vec![doc1.save()]).await.unwrap();

    let mut doc2 = node2.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();
    doc2.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "value", "node2 wins")?;
        Ok(())
    })
    .unwrap();
    node2.apply_changes(TEST_APP_ID, doc_id, vec![doc2.save()]).await.unwrap();

    // Exchange changes
    let (changes_1, _) = node1.get_changes_since(TEST_APP_ID, doc_id, &initial_heads).await.unwrap();
    let (changes_2, _) = node2.get_changes_since(TEST_APP_ID, doc_id, &initial_heads).await.unwrap();

    node2.apply_changes(TEST_APP_ID, doc_id, changes_1).await.unwrap();
    node1.apply_changes(TEST_APP_ID, doc_id, changes_2).await.unwrap();

    // Both nodes should converge to the same value (deterministic winner)
    let doc1_final = node1.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();
    let doc2_final = node2.get_or_create_doc(TEST_APP_ID, doc_id).await.unwrap();

    let get_value = |doc: &Automerge| -> Option<String> {
        doc.get(automerge::ROOT, "value")
            .unwrap()
            .and_then(|(v, _)| {
                if let automerge::Value::Scalar(s) = v {
                    if let automerge::ScalarValue::Str(smol) = s.as_ref() {
                        return Some(smol.to_string());
                    }
                }
                None
            })
    };

    let value1 = get_value(&doc1_final);
    let value2 = get_value(&doc2_final);

    // Values must match (CRDT convergence)
    assert_eq!(value1, value2);
    // Value should be one of the two (deterministic based on actor ID ordering)
    assert!(value1 == Some("node1 wins".to_string()) || value1 == Some("node2 wins".to_string()));
}
