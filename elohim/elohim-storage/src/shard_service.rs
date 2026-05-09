//! Phase 11 — transport-neutral shard service.
//!
//! Extracted from `p2p::P2PNode::handle_shard_request` so both the
//! libp2p request-response handler and the iroh-side `ShardBackend`
//! can route shard fetch / probe / push / inventory through the same
//! code path.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the shard plane is dual-stack permanent. Reed-Solomon coding stays
//! in pure Rust; framing is per-transport. This service holds the
//! transport-neutral state (the blob store + the optional content DB
//! pool) and answers each request variant identically regardless of
//! which transport carried it.
//!
//! Note: this service is **not** the same plane as the iroh-blobs
//! BLAKE3-streamed blob fetch (registered separately on the iroh
//! Router under `iroh_blobs::ALPN`). The iroh-side `ShardBackend`
//! exists for legacy SHA-256 sharded fetches that the protocol still
//! supports for libp2p-fallback peers; iroh-canonical blob distribution
//! goes through iroh-blobs.

use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::blob_store::BlobStore;
use crate::db::DbPool;
use crate::p2p::shard_protocol::{self, ShardRequest, ShardResponse};

/// Holds the dependencies needed to answer a shard request.
/// `Clone` is cheap (Arc + Option<DbPool>).
#[derive(Clone)]
pub struct ShardService {
    blob_store: Arc<BlobStore>,
    db_pool: Option<DbPool>,
}

impl std::fmt::Debug for ShardService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShardService")
            .field("has_db_pool", &self.db_pool.is_some())
            .finish_non_exhaustive()
    }
}

impl ShardService {
    pub fn new(blob_store: Arc<BlobStore>, db_pool: Option<DbPool>) -> Self {
        Self {
            blob_store,
            db_pool,
        }
    }

    /// Dispatch a [`ShardRequest`].
    pub async fn handle(&self, request: ShardRequest) -> ShardResponse {
        match request {
            ShardRequest::Get { hash } => self.handle_get(hash).await,
            ShardRequest::Have { hash } => self.handle_have(hash).await,
            ShardRequest::Push { hash, data } => self.handle_push(hash, data).await,
            ShardRequest::ListContent {
                reach_filter,
                offset,
                limit,
            } => self.handle_list_content(reach_filter, offset, limit),
            ShardRequest::GetContent { id } => self.handle_get_content(id),
        }
    }

    async fn handle_get(&self, hash: String) -> ShardResponse {
        debug!(hash = %hash, "Handling shard Get request");
        match self.blob_store.get(&hash).await {
            Ok(data) => {
                info!(hash = %hash, size = data.len(), "Serving shard");
                ShardResponse::Data(data)
            }
            Err(_) => {
                debug!(hash = %hash, "Shard not found");
                ShardResponse::NotFound
            }
        }
    }

    async fn handle_have(&self, hash: String) -> ShardResponse {
        debug!(hash = %hash, "Handling shard Have request");
        let exists = self.blob_store.exists(&hash).await;
        ShardResponse::Have(exists)
    }

    async fn handle_push(&self, hash: String, data: Vec<u8>) -> ShardResponse {
        debug!(hash = %hash, size = data.len(), "Handling shard Push request");
        match self.blob_store.store(&data).await {
            Ok(result) => {
                if result.hash == hash {
                    info!(hash = %hash, "Shard stored via P2P push");
                    ShardResponse::PushAck
                } else {
                    warn!(expected = %hash, actual = %result.hash, "Shard hash mismatch");
                    ShardResponse::Error("Hash mismatch".to_string())
                }
            }
            Err(e) => {
                error!(hash = %hash, error = %e, "Failed to store shard");
                ShardResponse::Error(format!("Storage error: {}", e))
            }
        }
    }

    fn handle_list_content(
        &self,
        reach_filter: Option<String>,
        offset: u32,
        limit: u32,
    ) -> ShardResponse {
        // Validate reach_filter against schema-generated constants so
        // unknown strings don't silently return empty results.
        if let Some(ref r) = reach_filter {
            if !crate::generated_enums::CORE_REACH_LEVELS.contains(&r.as_str()) {
                return ShardResponse::Error(format!(
                    "Unknown reach level {:?}. Valid values: {:?}",
                    r,
                    crate::generated_enums::CORE_REACH_LEVELS
                ));
            }
        }
        let pool = match self.db_pool.as_ref() {
            Some(p) => p,
            None => return ShardResponse::Error("No database pool".to_string()),
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => return ShardResponse::Error(format!("DB connection failed: {}", e)),
        };
        let app_ctx = crate::db::AppContext::default_lamad();
        let query = crate::db::content_diesel::ContentQuery {
            reach: reach_filter,
            limit: limit as i64,
            offset: offset as i64,
            ..Default::default()
        };
        // P2P shard inventory — internal peer-to-peer protocol, not
        // web2 HTTP. Peers must see all local rows so replication can
        // cover pre-drain content.
        match crate::db::content_diesel::list_content(&mut conn, &app_ctx, &query, false) {
            Ok(items) => {
                let total =
                    crate::db::content_diesel::count_content(&mut conn, &app_ctx, &query, false)
                        .unwrap_or(items.len() as i64) as u64;
                let inventory: Vec<shard_protocol::ContentInventoryItem> = items
                    .iter()
                    .map(|cwt| shard_protocol::ContentInventoryItem {
                        id: cwt.content.id.clone(),
                        title: cwt.content.title.clone(),
                        content_type: cwt.content.content_type.clone(),
                        content_format: cwt.content.content_format.clone(),
                        reach: cwt.content.reach.clone(),
                        blob_cid: cwt.content.blob_cid.clone(),
                        updated_at: cwt.content.updated_at.clone(),
                    })
                    .collect();
                let has_more = (offset as u64 + inventory.len() as u64) < total;
                info!(
                    count = inventory.len(),
                    total = total,
                    "Serving content inventory"
                );
                ShardResponse::ContentList {
                    items: inventory,
                    total,
                    has_more,
                }
            }
            Err(e) => ShardResponse::Error(format!("Content query failed: {}", e)),
        }
    }

    fn handle_get_content(&self, id: String) -> ShardResponse {
        let pool = match self.db_pool.as_ref() {
            Some(p) => p,
            None => return ShardResponse::Error("No database pool".to_string()),
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => return ShardResponse::Error(format!("DB connection failed: {}", e)),
        };
        let app_ctx = crate::db::AppContext::default_lamad();
        match crate::db::content_diesel::get_content_with_tags(&mut conn, &app_ctx, &id, false) {
            Ok(Some(cwt)) => {
                debug!(id = %id, "Serving content record to peer");
                ShardResponse::Content(Box::new(shard_protocol::ContentRecord {
                    id: cwt.content.id,
                    title: cwt.content.title,
                    description: cwt.content.description,
                    content_type: cwt.content.content_type,
                    content_format: cwt.content.content_format,
                    blob_hash: cwt.content.blob_hash,
                    blob_cid: cwt.content.blob_cid,
                    content_size_bytes: cwt.content.content_size_bytes,
                    metadata_json: cwt.content.metadata_json,
                    reach: cwt.content.reach,
                    created_by: cwt.content.created_by,
                    tags: cwt.tags,
                    content_body: cwt.content.content_body,
                }))
            }
            Ok(None) => ShardResponse::ContentNotFound,
            Err(e) => ShardResponse::Error(format!("Content fetch failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob_store::BlobStore;
    use tempfile::tempdir;

    async fn fresh_service() -> ShardService {
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        ShardService::new(blob_store, None)
    }

    #[tokio::test]
    async fn get_unknown_returns_not_found() {
        let svc = fresh_service().await;
        match svc
            .handle(ShardRequest::Get {
                hash: "missing".into(),
            })
            .await
        {
            ShardResponse::NotFound => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn have_unknown_returns_have_false() {
        let svc = fresh_service().await;
        match svc
            .handle(ShardRequest::Have {
                hash: "missing".into(),
            })
            .await
        {
            ShardResponse::Have(false) => {}
            other => panic!("expected Have(false), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_content_with_unknown_reach_filter_returns_error() {
        let svc = fresh_service().await;
        match svc
            .handle(ShardRequest::ListContent {
                reach_filter: Some("super-secret-tier".into()),
                offset: 0,
                limit: 10,
            })
            .await
        {
            ShardResponse::Error(msg) => assert!(msg.contains("Unknown reach level")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_content_without_db_pool_returns_error() {
        let svc = fresh_service().await;
        match svc
            .handle(ShardRequest::ListContent {
                reach_filter: None,
                offset: 0,
                limit: 10,
            })
            .await
        {
            ShardResponse::Error(msg) => assert!(msg.contains("No database pool")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_content_without_db_pool_returns_error() {
        let svc = fresh_service().await;
        match svc
            .handle(ShardRequest::GetContent {
                id: "anything".into(),
            })
            .await
        {
            ShardResponse::Error(msg) => assert!(msg.contains("No database pool")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn push_then_get_round_trips() {
        let svc = fresh_service().await;
        let data = b"hello shard".to_vec();
        // Compute the hash by storing first to learn the hash, then re-push
        // with that hash to exercise the Push path.
        let stored = svc.blob_store.store(&data).await.unwrap();
        let res = svc
            .handle(ShardRequest::Push {
                hash: stored.hash.clone(),
                data: data.clone(),
            })
            .await;
        match res {
            ShardResponse::PushAck => {}
            other => panic!("expected PushAck, got {other:?}"),
        }

        match svc
            .handle(ShardRequest::Get {
                hash: stored.hash.clone(),
            })
            .await
        {
            ShardResponse::Data(bytes) => assert_eq!(bytes, data),
            other => panic!("expected Data, got {other:?}"),
        }
    }
}
