//! Phase 11 — production [`ShardBackend`] backed by [`crate::shard_service::ShardService`].
//!
//! Adapter between the iroh shard ALPN handler ([`super::shard::IrohShardProtocol`])
//! and the daemon's transport-neutral shard service. Mirrors the libp2p
//! side's dispatch (`P2PNode::handle_shard_request` in `src/p2p/mod.rs`)
//! so the two transports return wire-byte-identical responses for the
//! same request.
//!
//! Per [`genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`],
//! the shard plane is dual-stack permanent. Reed-Solomon coding stays
//! in pure Rust; framing is per-transport.
//!
//! ## Relation to the iroh-blobs plane
//!
//! This is **not** the same plane as the BLAKE3-streamed blob fetch
//! that iroh-blobs registers under `iroh_blobs::ALPN`. The iroh-side
//! shard ALPN exists for legacy SHA-256 sharded fetches that the
//! protocol still supports for libp2p-fallback peers; iroh-canonical
//! blob distribution goes through iroh-blobs (BLAKE3, chunked,
//! verified streaming).

use std::sync::Arc;

use super::shard::ShardBackend;
use crate::p2p::shard_protocol::{ShardRequest, ShardResponse};
use crate::shard_service::ShardService;

/// Routes [`ShardRequest`] variants into a shared
/// [`ShardService`] and produces the matching [`ShardResponse`].
pub struct ShardServiceBackend {
    service: Arc<ShardService>,
}

impl ShardServiceBackend {
    pub fn new(service: Arc<ShardService>) -> Self {
        Self { service }
    }
}

impl std::fmt::Debug for ShardServiceBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShardServiceBackend").finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl ShardBackend for ShardServiceBackend {
    async fn handle(&self, request: ShardRequest) -> ShardResponse {
        self.service.handle(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob_store::BlobStore;
    use tempfile::tempdir;

    async fn fresh_backend() -> ShardServiceBackend {
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        let service = Arc::new(ShardService::new(blob_store, None));
        ShardServiceBackend::new(service)
    }

    #[tokio::test]
    async fn get_unknown_returns_not_found() {
        let backend = fresh_backend().await;
        match backend
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
    async fn list_content_without_pool_errors() {
        let backend = fresh_backend().await;
        match backend
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
}
