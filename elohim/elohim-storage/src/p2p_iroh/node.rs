//! `IrohNode` — Phase 2: blob plane via iroh-blobs.
//!
//! Aggregates the iroh `Endpoint`, an [`IrohBlobStore`] (wrapping
//! `iroh-blobs`' filesystem store), and an `iroh::protocol::Router` with
//! `BlobsProtocol` mounted under [`iroh_blobs::ALPN`]. This is the
//! production shape for the iroh transport's blob plane.
//!
//! Custom-ALPN handlers (Phase 5+) will register on the same Router
//! alongside iroh-blobs.

use anyhow::Result;
use bytes::Bytes;
use iroh::{
    protocol::{DynProtocolHandler, Router, RouterBuilder},
    Endpoint, NodeAddr, NodeId, Watcher,
};
use iroh_blobs::{BlobsProtocol, Hash};
use tracing::info;

use super::{
    blob_store::IrohBlobStore, config::IrohConfig, endpoint::BuildEndpointError,
    gossip::IrohGossip,
};

/// An ALPN bound to a protocol handler — the unit of registration on the
/// shared iroh `Router`. Used by [`IrohNode::start_with_protocols`] so
/// later phases can mount custom-ALPN handlers (sync, EPR, etc.) alongside
/// the iroh-blobs blob plane without churning this aggregate's API.
pub type AlpnRegistration = (Vec<u8>, Box<dyn DynProtocolHandler>);

/// Iroh-side P2P node — Phase 2 holds endpoint + store + router. The
/// Router has `BlobsProtocol` mounted under `iroh_blobs::ALPN`. Phase 3+
/// will call [`IrohNode::router_builder`]-style hooks (added then) so
/// custom-ALPN handlers can register without churning this aggregate.
#[derive(Debug)]
pub struct IrohNode {
    endpoint: Endpoint,
    router: Router,
    store: IrohBlobStore,
    gossip: IrohGossip,
}

impl IrohNode {
    /// Build endpoint + store, mount `BlobsProtocol` under `iroh_blobs::ALPN`,
    /// and spawn the accept loop. Caller is responsible for shutting down
    /// via [`IrohNode::shutdown`] on graceful exit.
    pub async fn start(config: IrohConfig) -> Result<Self, IrohNodeError> {
        Self::start_with_protocols(config, Vec::new()).await
    }

    /// Like [`IrohNode::start`] but additionally registers each custom-ALPN
    /// handler in `extra_protocols` on the shared `Router`. Phase 3+ uses
    /// this to layer sync / EPR / shard / view-fed / identity protocols
    /// alongside iroh-blobs without forking this aggregate.
    pub async fn start_with_protocols(
        config: IrohConfig,
        extra_protocols: Vec<AlpnRegistration>,
    ) -> Result<Self, IrohNodeError> {
        let endpoint = super::endpoint::build_endpoint(&config).await?;
        let store = IrohBlobStore::load(&config.blobs_dir).await?;
        let gossip = IrohGossip::new(endpoint.clone());

        let blobs_protocol = BlobsProtocol::new(store.inner(), endpoint.clone(), None);
        let mut builder: RouterBuilder = RouterBuilder::new(endpoint.clone())
            .accept(iroh_blobs::ALPN, blobs_protocol)
            .accept(iroh_gossip::ALPN, gossip.inner().clone());

        let extra_count = extra_protocols.len();
        for (alpn, handler) in extra_protocols {
            builder = builder.accept(alpn, handler);
        }

        let router: Router = builder.spawn();

        info!(
            target: "elohim_storage::p2p_iroh",
            node_id = %endpoint.node_id(),
            relays = config.use_n0_relays,
            blobs_dir = %config.blobs_dir.display(),
            extra_alpns = extra_count,
            "iroh node started (blob plane + gossip + extra ALPNs registered)"
        );

        Ok(Self {
            endpoint,
            router,
            store,
            gossip,
        })
    }

    /// This node's [`iroh::NodeId`] (derived from the persisted secret key).
    pub fn node_id(&self) -> NodeId {
        self.endpoint.node_id()
    }

    /// Wait for the endpoint's `NodeAddr` to be initialized and return it.
    /// On loopback (relays disabled) this is essentially immediate; with
    /// relays it waits for the relay home address to resolve.
    pub async fn node_addr(&self) -> Result<NodeAddr> {
        let addr = self.endpoint.node_addr().initialized().await;
        Ok(addr)
    }

    /// Borrow the underlying `Endpoint` for advanced uses (Phase 3+ ALPN
    /// handlers, NAT-traversal probes, metrics).
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Borrow the local `IrohBlobStore`.
    pub fn store(&self) -> &IrohBlobStore {
        &self.store
    }

    /// Borrow the gossip handle for subscribe/broadcast on a topic by name.
    pub fn gossip(&self) -> &IrohGossip {
        &self.gossip
    }

    /// Add bytes to the local store. Returns the BLAKE3 hash.
    pub async fn add_bytes(&self, data: Vec<u8>) -> Result<Hash> {
        self.store.add_bytes(data).await
    }

    /// Read bytes from the local store.
    pub async fn get_bytes(&self, hash: Hash) -> Result<Bytes> {
        self.store.get_bytes(hash).await
    }

    /// Whether this node currently holds the given hash locally.
    pub async fn has(&self, hash: Hash) -> Result<bool> {
        self.store.has(hash).await
    }

    /// Fetch a blob from a specific peer by `NodeAddr` + `Hash`. Opens a
    /// QUIC connection on `iroh_blobs::ALPN` and uses `Store::remote().fetch`
    /// to drive chunked verified streaming into the local store. After this
    /// returns, [`IrohNode::get_bytes`] will succeed.
    ///
    /// Pattern matches iroh-blobs' own two-node test (`tests.rs:230` in
    /// upstream): connect with the full `NodeAddr`, then `remote().fetch(conn,
    /// hash)`. We deliberately do not route through `Downloader` here: the
    /// pool's `endpoint.connect(node_id, alpn)` path strips direct addresses
    /// and falls back to discovery, which is unavailable when relays are
    /// disabled (CI/loopback case).
    pub async fn fetch_blob_from(&self, peer: NodeAddr, hash: Hash) -> Result<Bytes> {
        let conn = self.endpoint.connect(peer, iroh_blobs::ALPN).await?;
        self.store.inner().remote().fetch(conn, hash).await?;
        self.store.get_bytes(hash).await
    }

    /// Shut down router (closes accept loop + endpoint) gracefully.
    pub async fn shutdown(self) -> Result<()> {
        info!(
            target: "elohim_storage::p2p_iroh",
            node_id = %self.endpoint.node_id(),
            "iroh node shutting down"
        );
        self.router
            .shutdown()
            .await
            .map_err(|e| anyhow::anyhow!("router shutdown failed: {e}"))?;
        Ok(())
    }
}

/// Errors from `IrohNode::start`. Bind/identity failures, store load
/// failures, and protocol-mount failures all funnel here.
#[derive(Debug, thiserror::Error)]
pub enum IrohNodeError {
    #[error("endpoint build failed: {0}")]
    Endpoint(#[from] BuildEndpointError),

    #[error("blob store load failed: {0}")]
    Store(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn loopback_config(dir: &std::path::Path) -> IrohConfig {
        IrohConfig {
            blobs_dir: dir.join("blobs_iroh"),
            secret_key_path: dir.join("iroh.key"),
            use_n0_relays: false,
        use_n0_discovery: false,
        }
    }

    /// Smoke test — start the node, get its NodeId, shut down cleanly.
    #[tokio::test]
    async fn start_then_shutdown() {
        let dir = tempdir().unwrap();
        let node = IrohNode::start(loopback_config(dir.path())).await.unwrap();
        let _id = node.node_id();
        node.shutdown().await.unwrap();
    }

    /// Adding bytes via the node API delegates to the local store.
    #[tokio::test]
    async fn add_then_get_local() {
        let dir = tempdir().unwrap();
        let node = IrohNode::start(loopback_config(dir.path())).await.unwrap();

        let payload = b"phase 2 hello".to_vec();
        let hash = node.add_bytes(payload.clone()).await.unwrap();
        assert!(node.has(hash).await.unwrap());

        let got = node.get_bytes(hash).await.unwrap();
        assert_eq!(&got[..], &payload[..]);

        node.shutdown().await.unwrap();
    }
}
