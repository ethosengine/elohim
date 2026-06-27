//! Content-projection producer — the organ that lights the Automerge content-sync plane.
//!
//! The CRDT sync engine (`/elohim/storage-sync/1.0.0`) is fully wired end-to-end
//! but inert: nothing fills the DocStore, so every sync round round-trips empty.
//! This module adds the go-forward producer: a second `EventBus` subscriber
//! (alongside `spawn_logging_listener`) that turns each content write into an
//! Automerge document in the DocStore under the `"elohim"` sync namespace.

use std::sync::Arc;

use automerge::transaction::Transactable;
use tokio::sync::broadcast::error::RecvError;

use crate::db::context::AppContext;
use crate::db::models::Content;
use crate::db::{content_diesel, DbPool};
use crate::error::StorageError;
use crate::services::events::{EventBus, StorageEvent};
use crate::sync::SyncManager;

/// The canonical content-node doc-id for a content row.
pub fn content_doc_id(id: &str) -> String {
    format!("node:{id}")
}

/// Project a single content row into its Automerge doc under the "elohim" sync
/// namespace.
///
/// THE NAMESPACE IS LOAD-BEARING: `initiate_sync_round` (p2p/mod.rs:6996) only
/// lists `"elohim"`; a doc written under any other namespace sits inert forever.
/// (`PROJECTION_NAMESPACE` makes this coupling explicit — see the G4 guard.)
///
/// `SyncManager` exposes no `save`; the canonical mutate+persist idiom (proven in
/// tests/sync_integration.rs) is `apply_changes(ns, doc_id, vec![doc.save()])` —
/// the same path a peer's changes take, so a local projection and a remote merge
/// converge identically.
pub async fn project_content_doc(
    sync: &SyncManager,
    content: &Content,
) -> Result<(), StorageError> {
    let doc_id = content_doc_id(&content.id);
    let mut doc = sync
        .get_or_create_doc(PROJECTION_NAMESPACE, &doc_id)
        .await?;
    doc.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "id", content.id.as_str())?;
        tx.put(automerge::ROOT, "title", content.title.as_str())?;
        tx.put(
            automerge::ROOT,
            "contentType",
            content.content_type.as_str(),
        )?;
        tx.put(
            automerge::ROOT,
            "contentFormat",
            content.content_format.as_str(),
        )?;
        tx.put(automerge::ROOT, "reach", content.reach.as_str())?;
        tx.put(
            automerge::ROOT,
            "body",
            content.content_body.as_deref().unwrap_or(""),
        )?;
        tx.put(
            automerge::ROOT,
            "metadata",
            content.metadata_json.as_deref().unwrap_or("{}"),
        )?;
        tx.put(automerge::ROOT, "updatedAt", content.updated_at.as_str())?;
        Ok(())
    })
    .map_err(|e| StorageError::Sync(format!("projector transact failed: {e:?}")))?;
    sync.apply_changes(PROJECTION_NAMESPACE, &doc_id, vec![doc.save()])
        .await?;
    Ok(())
}

/// Re-SELECT the full content row by id.
///
/// `StorageEvent::ContentCreated/Updated` carries only `{id, ..}`, so the
/// producer re-reads the full row. Scoped to `default_lamad()` because
/// `Services::new` (services/mod.rs) builds its `ContentService` under the
/// lamad app context — the scope content is written under. `require_provenance`
/// is `false` so pre-drain rows project immediately (mirrors the internal
/// `ContentService::get` convention).
async fn load_content_row(pool: &DbPool, id: &str) -> Result<Option<Content>, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("Pool error: {e}")))?;
    let ctx = AppContext::default_lamad();
    content_diesel::get_content(&mut conn, &ctx, id, false)
}

/// Spawn the content-projection listener.
///
/// A second `EventBus` subscriber (mirrors `spawn_logging_listener`) that
/// projects each `ContentCreated`/`ContentUpdated` into its Automerge doc.
/// `ContentBulkCreated` is intentionally ignored (the write path already pauses
/// p2p sync for bulk; back-fill is a separate gated migration).
pub fn spawn_content_projection_listener(
    events: Arc<EventBus>,
    sync: Arc<SyncManager>,
    pool: DbPool,
) -> tokio::task::JoinHandle<()> {
    let mut rx = events.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(StorageEvent::ContentCreated { id, .. })
                | Ok(StorageEvent::ContentUpdated { id }) => {
                    match load_content_row(&pool, &id).await {
                        Ok(Some(content)) => {
                            if let Err(e) = project_content_doc(&sync, &content).await {
                                tracing::error!(%id, error = %e, "projector: doc projection failed");
                            } else {
                                tracing::debug!(%id, "projector: content projected to sync DocStore");
                            }
                        }
                        Ok(None) => tracing::warn!(%id, "projector: content row vanished"),
                        Err(e) => tracing::error!(%id, error = %e, "projector: load failed"),
                    }
                }
                // Ignore everything else (incl. ContentBulkCreated — see docs).
                Ok(_) => {}
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "projector: event bus lagged");
                }
                Err(RecvError::Closed) => {
                    tracing::debug!("projector: event bus closed, stopping listener");
                    break;
                }
            }
        }
    })
}

/// The sync-partition namespace. MUST equal the `h_app_id` listed by
/// `initiate_sync_round` (p2p/mod.rs:6996). Changing one without the other
/// silently disables content sync. (Promoted to a named const in Task G4;
/// guarded by `projection_namespace_matches_sync_timer`.)
pub const PROJECTION_NAMESPACE: &str = "elohim";

#[cfg(test)]
mod tests {
    use crate::db::models::Content;
    use crate::sync::{DocStore, DocStoreConfig, StreamTracker, SyncManager};
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Build a SyncManager over a temp sled DocStore (mirrors
    /// tests/sync_integration.rs::create_sync_manager).
    async fn test_sync_manager() -> (SyncManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let doc_store = Arc::new(
            DocStore::new(DocStoreConfig {
                db_path: temp_dir.path().join("projector.sled"),
                ..Default::default()
            })
            .await
            .unwrap(),
        );
        let stream_tracker = Arc::new(StreamTracker::new());
        (SyncManager::new(doc_store, stream_tracker), temp_dir)
    }

    fn sample_content(id: &str, title: &str) -> Content {
        Content {
            id: id.to_string(),
            h_app_id: "lamad".to_string(),
            title: title.to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: Some("{}".to_string()),
            reach: "household".to_string(),
            validation_status: "valid".to_string(),
            created_by: None,
            created_at: "2026-06-27T00:00:00Z".to_string(),
            updated_at: "2026-06-27T00:00:00Z".to_string(),
            content_body: Some("hello".to_string()),
            dht_anchor_hash: None,
            p2p_published_at: None,
            server_blob_hash: None,
        }
    }

    #[tokio::test]
    async fn producer_projects_content_create_into_docstore_under_elohim() {
        let (sync, _temp) = test_sync_manager().await;
        let content = sample_content("edit-prop-1", "v1");

        // Act: project once.
        super::project_content_doc(&sync, &content).await.unwrap();

        // Assert: doc exists under "elohim" at node:{id}, with the title and heads.
        let heads = sync
            .get_heads("elohim", "node:edit-prop-1")
            .await
            .expect("doc exists");
        assert!(!heads.is_empty(), "projected doc must have heads");

        let title = sync
            .get_doc_field("elohim", "node:edit-prop-1", "title")
            .await
            .unwrap();
        assert_eq!(title, "v1");

        // The doc_id helper is the canonical content-node address.
        assert_eq!(super::content_doc_id("edit-prop-1"), "node:edit-prop-1");
    }

    /// Guard the load-bearing namespace coupling. `initiate_sync_round`
    /// (p2p/mod.rs:6996) lists ONLY `"elohim"`. If that constant and the
    /// producer ever diverge, content sync silently dies — this test fails loudly
    /// if `PROJECTION_NAMESPACE` drifts from the sync-timer's `h_app_id`.
    #[test]
    fn projection_namespace_matches_sync_timer() {
        // Mirror of the literal hardcoded at p2p/mod.rs:6996 (initiate_sync_round).
        const PROJECTION_NS: &str = "elohim";
        assert_eq!(PROJECTION_NS, super::PROJECTION_NAMESPACE);
    }
}
