//! Content-projection producer — the organ that lights the Automerge content-sync plane.
//!
//! The CRDT sync engine (`/elohim/storage-sync/1.0.0`) is fully wired end-to-end
//! but inert: nothing fills the DocStore, so every sync round round-trips empty.
//! This module adds the go-forward producer: a second `EventBus` subscriber
//! (alongside `spawn_logging_listener`) that turns each content write into an
//! Automerge document in the DocStore under the `"elohim"` sync namespace.

use std::sync::Arc;

use automerge::transaction::Transactable;
use automerge::{Automerge, ReadDoc, ScalarValue, Value};
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

/// One projected field value — a string or an integer scalar. Kept as an enum
/// so the projection WRITE and the idempotency COMPARE share a single field
/// list (`projected_fields`) and can never drift.
enum FieldVal {
    S(String),
    I(i64),
}

/// The ordered field set projected from a content row into its Automerge doc —
/// the single source of truth for both the write and the idempotency compare.
///
/// `blobHash`/`serverBlobHash`/`blobCid`/`contentSizeBytes` are LOAD-BEARING for
/// peer convergence of the SERVING path: a peer whose content row lost its
/// `blobHash` (e.g. a deploy PATCH that never landed on a degraded conductor) can
/// only re-derive it by converging this doc from a healthy peer. `None` string
/// fields project as `""` (the consumer treats empty as absent); a missing
/// numeric projects as `0`.
fn projected_fields(content: &Content) -> Vec<(&'static str, FieldVal)> {
    vec![
        ("id", FieldVal::S(content.id.clone())),
        ("hAppId", FieldVal::S(content.h_app_id.clone())),
        ("title", FieldVal::S(content.title.clone())),
        (
            "description",
            FieldVal::S(content.description.clone().unwrap_or_default()),
        ),
        ("contentType", FieldVal::S(content.content_type.clone())),
        ("contentFormat", FieldVal::S(content.content_format.clone())),
        ("reach", FieldVal::S(content.reach.clone())),
        (
            "body",
            FieldVal::S(content.content_body.clone().unwrap_or_default()),
        ),
        (
            "metadata",
            FieldVal::S(
                content
                    .metadata_json
                    .clone()
                    .unwrap_or_else(|| "{}".to_string()),
            ),
        ),
        (
            "blobHash",
            FieldVal::S(content.blob_hash.clone().unwrap_or_default()),
        ),
        (
            "serverBlobHash",
            FieldVal::S(content.server_blob_hash.clone().unwrap_or_default()),
        ),
        (
            "blobCid",
            FieldVal::S(content.blob_cid.clone().unwrap_or_default()),
        ),
        (
            "contentSizeBytes",
            FieldVal::I(content.content_size_bytes.unwrap_or(0) as i64),
        ),
        ("updatedAt", FieldVal::S(content.updated_at.clone())),
    ]
}

/// True when the doc ALREADY carries exactly the projected field set — the
/// idempotency guard. A missing field, a changed value, or a scalar-type
/// mismatch each mean "needs (re)projection". A freshly-created empty doc has no
/// fields, so it never matches → always projects on first sight.
fn doc_matches(doc: &Automerge, fields: &[(&'static str, FieldVal)]) -> bool {
    for (key, val) in fields {
        match doc.get(automerge::ROOT, *key) {
            Ok(Some((Value::Scalar(scalar), _))) => match (val, scalar.as_ref()) {
                (FieldVal::S(want), ScalarValue::Str(got)) if got.as_str() == want => {}
                (FieldVal::I(want), ScalarValue::Int(got)) if got == want => {}
                _ => return false,
            },
            _ => return false,
        }
    }
    true
}

/// Project a single content row into its Automerge doc under the "elohim" sync
/// namespace. Returns `true` if the doc was (re)written, `false` if it already
/// matched — an **idempotent** skip that appends no new change, so re-running the
/// producer or the corpus back-fill never inflates the doc's change history.
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
) -> Result<bool, StorageError> {
    let doc_id = content_doc_id(&content.id);
    let mut doc = sync
        .get_or_create_doc(PROJECTION_NAMESPACE, &doc_id)
        .await?;
    let fields = projected_fields(content);
    if doc_matches(&doc, &fields) {
        return Ok(false);
    }
    doc.transact::<_, _, automerge::AutomergeError>(|tx| {
        for (key, val) in &fields {
            match val {
                FieldVal::S(s) => tx.put(automerge::ROOT, *key, s.as_str())?,
                FieldVal::I(i) => tx.put(automerge::ROOT, *key, *i)?,
            }
        }
        Ok(())
    })
    .map_err(|e| StorageError::Sync(format!("projector transact failed: {e:?}")))?;
    sync.apply_changes(PROJECTION_NAMESPACE, &doc_id, vec![doc.save()])
        .await?;
    Ok(true)
}

/// Outcome of a corpus back-fill pass.
#[derive(Debug, Default, Clone, Copy)]
pub struct BackfillStats {
    pub scanned: u64,
    pub projected: u64,
    pub skipped: u64,
    pub failed: u64,
}

/// Back-fill the Automerge DocStore from the pre-existing SQL content corpus.
///
/// The go-forward producer (`spawn_content_projection_listener`) only projects
/// content written AFTER it starts. On a fresh deploy or a sled DocStore reset,
/// the large already-seeded corpus would never converge until each row was
/// re-written. This one-shot, **idempotent** pass closes that gap: it pages every
/// content row (all app scopes, provenance-ungated — a projection REBUILD over
/// already-notarized content, not a read surface) and projects each via
/// `project_content_doc`, which skips rows already present with matching values.
/// Safe to run on every cold start (gated at the call site).
///
/// Batched, and yields between pages so it never starves the live write path or
/// the 60s sync timer. `batch` bounds the per-page SELECT (clamped to ≥ 1).
pub async fn backfill_content_docs(
    sync: &SyncManager,
    pool: &DbPool,
    batch: i64,
) -> Result<BackfillStats, StorageError> {
    let batch = batch.max(1);
    let mut stats = BackfillStats::default();
    let mut offset: i64 = 0;
    loop {
        let rows = {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("Pool error: {e}")))?;
            content_diesel::list_all_content_rows(&mut conn, offset, batch)?
        };
        if rows.is_empty() {
            break;
        }
        let page_len = rows.len();
        for content in &rows {
            stats.scanned += 1;
            match project_content_doc(sync, content).await {
                Ok(true) => stats.projected += 1,
                Ok(false) => stats.skipped += 1,
                Err(e) => {
                    stats.failed += 1;
                    tracing::error!(id = %content.id, error = %e, "backfill: projection failed");
                }
            }
        }
        offset += page_len as i64;
        // Yield so a large back-fill never monopolises the runtime.
        tokio::task::yield_now().await;
        if (page_len as i64) < batch {
            break;
        }
    }
    tracing::info!(
        scanned = stats.scanned,
        projected = stats.projected,
        skipped = stats.skipped,
        failed = stats.failed,
        "backfill: corpus projection pass complete"
    );
    Ok(stats)
}

/// Reverse-project a converged content doc back into the SQL content projection —
/// the CONSUMER heal leg (the missing other half of the sync plane; today
/// `apply_changes` writes the sled DocStore ONLY). When a peer's content doc
/// converges into our DocStore, re-derive the serving-critical `blob_hash` into the
/// local SQL row at the **amber** tier, so SERVING (which reads SQL, not the
/// DocStore) heals WITHOUT the notary. Returns `true` if a heal was written.
///
/// Division of labor (spec §5.5): the shard/replication plane heals ABSENCE (a
/// missing row/bytes, via `bulk_create_content`); this heals DRIFT — a stale/null
/// `blob_hash` on a row that already EXISTS locally. An absent row is skipped
/// (`Ok(false)`) — that is the shard plane's job, not the drift-heal's.
///
/// Guards: (1) **empty-never-wins** — only heals when the converged `blob_hash` is
/// non-empty (a peer holding `""` never marks us amber or clobbers us); (2)
/// **no-clobber** — the write rides `update_content`'s amber path (`crdt_converged_at`
/// set), which never overwrites a present (possibly green/notarized) `blob_hash`
/// (A3); (3) **namespace** — writes under `default_lamad` (the serving scope), never
/// the doc's `"elohim"` sync namespace (REQ-F5).
pub async fn reverse_project_content_doc(
    sync: &SyncManager,
    pool: &DbPool,
    doc_id: &str,
) -> Result<bool, StorageError> {
    let Some(id) = doc_id.strip_prefix("node:") else {
        return Ok(false); // not a content-node doc
    };
    // Empty-never-wins: read the converged blob_hash; skip if absent/empty.
    let blob_hash = match sync
        .get_doc_field(PROJECTION_NAMESPACE, doc_id, "blobHash")
        .await
    {
        Ok(h) if !h.is_empty() => h,
        _ => return Ok(false),
    };
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("Pool error: {e}")))?;
    let ctx = AppContext::default_lamad();
    let input = content_diesel::UpdateContentInput {
        id: id.to_string(),
        blob_hash: Some(blob_hash),
        // The amber marker: switches update_content into the no-clobber amber path
        // (stamps crdt_converged_at, NEVER dht_anchor_hash).
        crdt_converged_at: Some(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    };
    match content_diesel::update_content(&mut conn, &ctx, input) {
        Ok(_) => Ok(true),
        // Absent row → the shard/replication plane heals it, not the drift-heal.
        Err(StorageError::NotFound(_)) => Ok(false),
        Err(e) => Err(e),
    }
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
    content_diesel::get_content(&mut conn, &ctx, id, content_diesel::MinTrust::Invisible)
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

/// The sync-partition namespace — the single source of truth for the `h_app_id`
/// that partitions content sync. BOTH sides reference this const: the producer
/// (`project_content_doc`, below) writes docs under it, and the consumer
/// (`initiate_sync_round` in p2p/mod.rs) lists documents under it. Because both
/// reference the const directly, producer/consumer drift is now compile-impossible
/// (each used to carry a bare `"elohim"` literal — a silent content-sync killer if
/// either drifted). The remaining invariant — the wire value stays `"elohim"`
/// unless every peer + the DNA migrate in lockstep — is pinned by
/// `projection_namespace_is_wire_contract`.
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
            crdt_converged_at: None,
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

    /// Pin the on-the-wire sync namespace. Producer/consumer drift is now
    /// compile-enforced — `initiate_sync_round` (p2p/mod.rs) and
    /// `project_content_doc` both reference `PROJECTION_NAMESPACE` directly, so
    /// neither can silently diverge. (The previous version of this test compared
    /// a LOCAL COPY of the `"elohim"` literal to the const and so guarded nothing:
    /// renaming the sync-timer's `h_app_id` would have passed while killing content
    /// sync fleet-wide.) What remains to guard is the WIRE VALUE: remote peers and
    /// the DNA expect `"elohim"`. Changing the const moves both sides coherently
    /// but alters the wire protocol — this fails loudly so that change is deliberate
    /// and coordinated, never an incidental rename.
    #[test]
    fn projection_namespace_is_wire_contract() {
        assert_eq!(
            super::PROJECTION_NAMESPACE,
            "elohim",
            "sync-partition namespace is a wire contract with every peer + the DNA"
        );
    }

    /// Re-projecting identical content is a no-op — no new change is appended, so
    /// the corpus back-fill (and a redundant go-forward update) never inflate a
    /// doc's change history. This is the idempotency the back-fill relies on.
    #[tokio::test]
    async fn project_is_idempotent_on_unchanged_content() {
        let (sync, _temp) = test_sync_manager().await;
        let content = sample_content("idem-1", "v1");

        assert!(
            super::project_content_doc(&sync, &content).await.unwrap(),
            "first projection must write"
        );
        let heads1 = sync.get_heads("elohim", "node:idem-1").await.unwrap();
        assert!(!heads1.is_empty());

        assert!(
            !super::project_content_doc(&sync, &content).await.unwrap(),
            "re-projecting unchanged content must be a skip"
        );
        let heads2 = sync.get_heads("elohim", "node:idem-1").await.unwrap();
        assert_eq!(
            heads1, heads2,
            "idempotent re-projection must not append a change"
        );
    }

    /// A changed serving-critical field (blobHash) forces a re-projection and the
    /// new value converges into the doc.
    #[tokio::test]
    async fn project_rewrites_when_a_field_changes() {
        let (sync, _temp) = test_sync_manager().await;
        let mut content = sample_content("chg-1", "v1");

        assert!(super::project_content_doc(&sync, &content).await.unwrap());
        let heads1 = sync.get_heads("elohim", "node:chg-1").await.unwrap();

        content.blob_hash = Some("sha256-deadbeef".to_string());
        assert!(
            super::project_content_doc(&sync, &content).await.unwrap(),
            "a changed field must re-project"
        );
        let heads2 = sync.get_heads("elohim", "node:chg-1").await.unwrap();
        assert_ne!(heads1, heads2, "a changed field must append a change");

        let blob = sync
            .get_doc_field("elohim", "node:chg-1", "blobHash")
            .await
            .unwrap();
        assert_eq!(blob, "sha256-deadbeef");
    }

    /// The projected doc carries the serving-critical blob fields (this is what
    /// lets a degraded peer re-derive a lost `blobHash` from a healthy peer).
    #[tokio::test]
    async fn projects_serving_critical_blob_fields() {
        let (sync, _temp) = test_sync_manager().await;
        let mut content = sample_content("blob-1", "landing");
        content.blob_hash = Some("sha256-abc".to_string());
        content.server_blob_hash = Some("sha256-ssr".to_string());

        super::project_content_doc(&sync, &content).await.unwrap();

        assert_eq!(
            sync.get_doc_field("elohim", "node:blob-1", "blobHash")
                .await
                .unwrap(),
            "sha256-abc"
        );
        assert_eq!(
            sync.get_doc_field("elohim", "node:blob-1", "serverBlobHash")
                .await
                .unwrap(),
            "sha256-ssr"
        );
        assert_eq!(
            sync.get_doc_field("elohim", "node:blob-1", "hAppId")
                .await
                .unwrap(),
            "lamad"
        );
    }

    /// End-to-end back-fill over a real (in-memory) SQL corpus: the first pass
    /// projects every row; the second is a full no-op. Proves the O(rows) back-fill
    /// is idempotent and safe to run on every cold start.
    #[tokio::test]
    async fn backfill_projects_all_rows_and_is_idempotent() {
        use crate::db::content_diesel::{self, CreateContentInput};
        use crate::db::context::AppContext;

        let (sync, _temp) = test_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();

        {
            let mut conn = pool.get().unwrap();
            for i in 0..3 {
                let input = CreateContentInput {
                    id: format!("bf-{i}"),
                    title: format!("row {i}"),
                    description: None,
                    content_type: "concept".to_string(),
                    content_format: "markdown".to_string(),
                    blob_hash: Some(format!("sha256-{i}")),
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "household".to_string(),
                    created_by: None,
                    tags: vec![],
                    content_body: Some("body".to_string()),
                    dht_anchor_hash: None,
                };
                content_diesel::create_content(&mut conn, &ctx, input).unwrap();
            }
        }

        // First pass projects all 3 (batch smaller than corpus exercises paging).
        let s1 = super::backfill_content_docs(&sync, &pool, 2).await.unwrap();
        assert_eq!(s1.scanned, 3);
        assert_eq!(s1.projected, 3);
        assert_eq!(s1.skipped, 0);
        assert_eq!(sync.count_documents("elohim").await.unwrap(), 3);

        // Second pass is a full no-op — the idempotency guarantee.
        let s2 = super::backfill_content_docs(&sync, &pool, 2).await.unwrap();
        assert_eq!(s2.scanned, 3);
        assert_eq!(s2.projected, 0);
        assert_eq!(s2.skipped, 3);
        assert_eq!(sync.count_documents("elohim").await.unwrap(), 3);

        // The seeded blobHash converged into the sync doc.
        assert_eq!(
            sync.get_doc_field("elohim", "node:bf-0", "blobHash")
                .await
                .unwrap(),
            "sha256-0"
        );
    }

    /// B1 proof — the consumer heal leg: a converged real blob_hash re-derives into
    /// a NULL SQL row (amber-stamped, never notarized); an EMPTY converged blob_hash
    /// never overwrites a present real hash (empty-never-wins). This is the
    /// elohim.host `blobHash: null` 404 as a red→green, WITHOUT the notary.
    #[tokio::test]
    async fn reverse_project_heals_null_and_empty_never_wins() {
        use crate::db::content_diesel::{self, CreateContentInput};
        use crate::db::context::AppContext;

        let (sync, _temp) = test_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();

        // Local rows: heal-null (blob_hash NULL — the elohim.host case); heal-real
        // (a present real blob_hash that empty must never clobber).
        {
            let mut conn = pool.get().unwrap();
            for (id, bh) in [
                ("heal-null", None),
                ("heal-real", Some("sha256-existing".to_string())),
            ] {
                content_diesel::create_content(
                    &mut conn,
                    &ctx,
                    CreateContentInput {
                        id: id.to_string(),
                        title: "t".to_string(),
                        description: None,
                        content_type: "concept".to_string(),
                        content_format: "markdown".to_string(),
                        blob_hash: bh,
                        blob_cid: None,
                        content_size_bytes: None,
                        metadata_json: None,
                        reach: "household".to_string(),
                        created_by: None,
                        tags: vec![],
                        content_body: Some("b".to_string()),
                        dht_anchor_hash: None,
                    },
                )
                .unwrap();
            }
        }

        // Peer A's converged doc for heal-null carries a REAL blob_hash.
        let mut peer_a = sample_content("heal-null", "t");
        peer_a.blob_hash = Some("sha256-converged".to_string());
        super::project_content_doc(&sync, &peer_a).await.unwrap();
        // Peer B's converged doc for heal-real carries an EMPTY blob_hash (None → "").
        let peer_b = sample_content("heal-real", "t");
        super::project_content_doc(&sync, &peer_b).await.unwrap();

        assert!(
            super::reverse_project_content_doc(&sync, &pool, "node:heal-null")
                .await
                .unwrap(),
            "null local + real converged → heals"
        );
        assert!(
            !super::reverse_project_content_doc(&sync, &pool, "node:heal-real")
                .await
                .unwrap(),
            "empty converged → skipped (empty never wins)"
        );

        let mut conn = pool.get().unwrap();
        let healed =
            content_diesel::get_content(&mut conn, &ctx, "heal-null", content_diesel::MinTrust::Invisible)
                .unwrap()
                .unwrap();
        assert_eq!(healed.blob_hash.as_deref(), Some("sha256-converged"));
        assert!(healed.crdt_converged_at.is_some(), "healed row is amber-stamped");
        assert!(healed.dht_anchor_hash.is_none(), "heal never notarizes");

        let untouched =
            content_diesel::get_content(&mut conn, &ctx, "heal-real", content_diesel::MinTrust::Invisible)
                .unwrap()
                .unwrap();
        assert_eq!(
            untouched.blob_hash.as_deref(),
            Some("sha256-existing"),
            "empty never clobbers a real hash"
        );
    }
}
