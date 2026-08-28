//! Phase 11 — transport-neutral shard service.
//!
//! Extracted from `p2p::P2PNode::handle_shard_request` so both the
//! libp2p request-response handler and the iroh-side `ShardBackend`
//! can route shard fetch / probe / push / inventory through the same
//! code path.
//!
//! Per [`genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md`],
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
    /// The iroh (BLAKE3) blob store, bound after the iroh node exists. A blob
    /// staged through the iroh cutover lives HERE, with only a sha256→blake3
    /// alias in `peer_blob_inventory`; HTTP `/blob` resolves that alias, and
    /// until 2026-08-28 this responder did not — so a peer could serve its own
    /// landing bundle over HTTP and answer `NotFound` to every peer asking for
    /// the same bytes over the shard protocol (measured: homo-iroh P2 red,
    /// 349 iroh blob fetches `not_found` against a survivor holding the blob).
    #[cfg(feature = "p2p-iroh")]
    iroh_store: std::sync::OnceLock<Arc<crate::p2p_iroh::IrohBlobStore>>,
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
            #[cfg(feature = "p2p-iroh")]
            iroh_store: std::sync::OnceLock::new(),
        }
    }

    /// Bind the iroh blob store once it exists (the shard responder is built
    /// before the iroh node). Idempotent; the first binding wins.
    #[cfg(feature = "p2p-iroh")]
    pub fn set_iroh_store(&self, store: Arc<crate::p2p_iroh::IrohBlobStore>) {
        let _ = self.iroh_store.set(store);
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
            ShardRequest::GetManifest { hash } => self.handle_get_manifest(hash),
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
                #[cfg(feature = "p2p-iroh")]
                if let Some(data) = self.get_via_iroh_alias(&hash).await {
                    info!(hash = %hash, size = data.len(), "Serving shard from the iroh store (sha256→blake3 alias)");
                    return ShardResponse::Data(data);
                }
                debug!(hash = %hash, "Shard not found");
                ShardResponse::NotFound
            }
        }
    }

    /// Serve a sha256-addressed blob from the iroh store when the sha256 store
    /// misses: resolve the alias `peer_blob_inventory` keeps for blobs staged
    /// through the iroh cutover, then read the BLAKE3 object. `None` when there
    /// is no iroh store, no pool, no alias, or no such object — never an error.
    #[cfg(feature = "p2p-iroh")]
    async fn get_via_iroh_alias(&self, hash: &str) -> Option<Vec<u8>> {
        let iroh = self.iroh_store.get()?;
        let pool = self.db_pool.as_ref()?;
        let normalized = match BlobStore::parse_content_address(hash) {
            Ok(h) => format!("sha256-{}", h),
            Err(_) => return None,
        };
        let alias = {
            let mut conn = pool.get().ok()?;
            crate::db::peer_blob_inventory::lookup_blake3_for_sha256(&mut conn, &normalized)
                .ok()
                .flatten()?
        };
        let hex = alias.strip_prefix("blake3-").unwrap_or(&alias);
        let iroh_hash: iroh_blobs::Hash = hex.parse().ok()?;
        match iroh.get_bytes(iroh_hash).await {
            Ok(bytes) => {
                // Serve only bytes that ARE the requested address. An alias can
                // point at a reassembled composite (RS-sharded bundle) whose
                // sha256 is not the composite's name — those are healed through
                // the shard manifest, never as whole bytes under this name.
                if crate::p2p::blob_fetch::verify_blob_hash(&bytes, &normalized) {
                    Some(bytes.to_vec())
                } else {
                    debug!(hash = %hash, alias = %alias, "iroh alias bytes do not hash to the requested address (composite?) — not served");
                    None
                }
            }
            Err(e) => {
                debug!(hash = %hash, alias = %alias, error = %e, "iroh store miss for aliased blob");
                None
            }
        }
    }

    /// The composite pivot: a peer that holds a blob only as RS shards (its
    /// whole-bytes `Get` misses) answers with the durable manifest so the
    /// requester can shard-fetch — what the libp2p blob protocol has always done
    /// with `BlobFetchReply::Manifest`; now on the shard protocol for BOTH planes.
    fn handle_get_manifest(&self, hash: String) -> ShardResponse {
        let Some(pool) = self.db_pool.as_ref() else {
            return ShardResponse::NotFound;
        };
        let Ok(mut conn) = pool.get() else {
            return ShardResponse::NotFound;
        };
        match crate::db::shard_manifests::get_manifest_by_blob_hash(&mut conn, &hash) {
            Ok(Some(row)) => match crate::db::shard_manifests::hydrate_manifest(&row) {
                Ok(manifest) => {
                    info!(hash = %hash, shards = manifest.shard_hashes.len(), "Serving shard manifest");
                    ShardResponse::Manifest(Box::new(manifest))
                }
                Err(e) => {
                    debug!(hash = %hash, error = %e, "shard manifest row failed to hydrate");
                    ShardResponse::NotFound
                }
            },
            _ => ShardResponse::NotFound,
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
        // ConvergenceAtom::InventoryServe — the local read cost of answering ONE
        // peer's ListContent page. Started before the pool checkout deliberately:
        // waiting for a connection IS the cost when the read pool is saturated,
        // and a timer starting after checkout would report a fast query while the
        // peer waited seconds to be served. Measured 2026-08-20 on matthew:
        // "Database read connection is saturated. Util 1387.50%" — the wait was
        // the whole story and no timer existed to say so.
        let serve_started = std::time::Instant::now();
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
        match crate::db::content_diesel::list_content(
            &mut conn,
            &app_ctx,
            &query,
            crate::db::content_diesel::MinTrust::Invisible,
        ) {
            Ok(items) => {
                let total = crate::db::content_diesel::count_content(
                    &mut conn,
                    &app_ctx,
                    &query,
                    crate::db::content_diesel::MinTrust::Invisible,
                )
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
                // Covers BOTH queries — list_content AND count_content. The pair
                // is the real cost: count_content is a full count over the whole
                // corpus (4495 rows on alpha) run on EVERY page, so a 5-page walk
                // pays five full counts. Timing only the list would have hidden
                // half of it.
                crate::metrics::observe_atom_duration(
                    crate::metrics::ConvergenceAtom::InventoryServe,
                    serve_started.elapsed(),
                );
                info!(
                    count = inventory.len(),
                    total = total,
                    elapsed_ms = serve_started.elapsed().as_secs_f64() * 1_000.0,
                    "Serving content inventory"
                );
                ShardResponse::ContentList {
                    items: inventory,
                    total,
                    has_more,
                }
            }
            Err(e) => {
                // Errors are recorded too: a query that FAILS after waiting on a
                // saturated pool still consumed the wait, and omitting it would
                // make the distribution improve exactly as the pool degrades
                // (coordinated omission).
                crate::metrics::observe_atom_duration(
                    crate::metrics::ConvergenceAtom::InventoryServe,
                    serve_started.elapsed(),
                );
                ShardResponse::Error(format!("Content query failed: {}", e))
            }
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
        match crate::db::content_diesel::get_content_with_tags(
            &mut conn,
            &app_ctx,
            &id,
            crate::db::content_diesel::MinTrust::Invisible,
        ) {
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
