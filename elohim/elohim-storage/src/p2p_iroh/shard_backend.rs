//! Phase 11 — production [`ShardBackend`] backed by [`crate::shard_service::ShardService`].
//!
//! Adapter between the iroh shard ALPN handler ([`super::shard::IrohShardProtocol`])
//! and the daemon's transport-neutral shard service. Mirrors the libp2p
//! side's dispatch (`P2PNode::handle_shard_request` in `src/p2p/mod.rs`)
//! so the two transports return wire-byte-identical responses for the
//! same request.
//!
//! Per [`genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md`],
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
use crate::services::custody_standing::Requester;
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
        f.debug_struct("ShardServiceBackend")
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl ShardBackend for ShardServiceBackend {
    async fn handle(&self, requester: &Requester, request: ShardRequest) -> ShardResponse {
        self.service.handle(requester, request).await
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

    /// Transport parity for the db-less arm (19a746a77, station 3b follow-up):
    /// a peer composed WITHOUT a content DB cannot hold private reference rows
    /// at all, so the custody gate returns "nothing to gate" rather than failing
    /// closed, and blob serving is exactly what it was before the gate existed.
    ///
    /// This mirrors `shard_service::tests::get_blob_on_a_peer_without_a_content_db_serves_as_before`
    /// — the libp2p and iroh legs share one service, so a divergence here would
    /// mean the two transports answered the same request differently. (The
    /// previous expectation, `Error("reach-withheld: authority-unavailable")`,
    /// went stale when that commit narrowed fail-closed to "configured authority
    /// BROKE"; its `shard_service` sibling was updated and this one was missed.)
    #[tokio::test]
    async fn get_on_a_peer_without_a_content_db_serves_as_before() {
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        let stored = blob_store.store(b"db-less peer blob").await.unwrap();
        let backend = ShardServiceBackend::new(Arc::new(ShardService::new(blob_store, None)));

        match backend
            .handle(&Requester::local(), ShardRequest::Get { hash: stored.hash })
            .await
        {
            ShardResponse::Data(data) => assert_eq!(data, b"db-less peer blob"),
            other => panic!("expected the blob to serve, got {other:?}"),
        }
        // …and a hash it does not hold is an honest miss, not an authority error.
        match backend
            .handle(
                &Requester::local(),
                ShardRequest::Get {
                    hash: "missing".into(),
                },
            )
            .await
        {
            ShardResponse::NotFound => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_manifest_without_pool_is_an_honest_not_found() {
        // The composite pivot (2026-08-28): a peer with no DB pool cannot hold
        // manifests, so it answers NotFound — the iroh leg then records a miss
        // instead of an error, exactly as a whole-bytes miss would.
        let backend = fresh_backend().await;
        match backend
            .handle(
                &Requester::local(),
                ShardRequest::GetManifest {
                    hash: "sha256-0000000000000000000000000000000000000000000000000000000000000000"
                        .into(),
                },
            )
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
            .handle(
                &Requester::local(),
                ShardRequest::ListContent {
                    reach_filter: None,
                    offset: 0,
                    limit: 10,
                },
            )
            .await
        {
            ShardResponse::Error(msg) => assert!(msg.contains("No database pool")),
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
