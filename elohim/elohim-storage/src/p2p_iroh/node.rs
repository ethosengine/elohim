//! `IrohNode` — Phase 2: blob plane via iroh-blobs.
//!
//! Aggregates the iroh `Endpoint`, an [`IrohBlobStore`] (wrapping
//! `iroh-blobs`' filesystem store), and an `iroh::protocol::Router` with
//! `BlobsProtocol` mounted under [`iroh_blobs::ALPN`]. This is the
//! production shape for the iroh transport's blob plane.
//!
//! Custom-ALPN handlers (Phase 5+) will register on the same Router
//! alongside iroh-blobs.

use std::sync::OnceLock;

use anyhow::Result;
use bytes::Bytes;
use iroh::{
    protocol::{DynProtocolHandler, Router, RouterBuilder},
    Endpoint, NodeAddr, NodeId, Watcher,
};
use iroh_blobs::{BlobsProtocol, Hash};
use tracing::info;

use super::{
    blob_store::IrohBlobStore,
    codec::{read_frame_default, write_frame},
    config::IrohConfig,
    endpoint::BuildEndpointError,
    gossip::IrohGossip,
    peer_book::IrohPeerBook,
    shard::SHARD_ALPN,
};
use crate::p2p::shard_protocol::{ShardRequest, ShardResponse};

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
    #[allow(clippy::result_large_err)]
    pub async fn start(config: IrohConfig) -> Result<Self, IrohNodeError> {
        Self::start_with_protocols(config, Vec::new()).await
    }

    /// Like [`IrohNode::start`] but additionally registers each custom-ALPN
    /// handler in `extra_protocols` on the shared `Router`. Phase 3+ uses
    /// this to layer sync / EPR / shard / view-fed / identity protocols
    /// alongside iroh-blobs without forking this aggregate.
    #[allow(clippy::result_large_err)]
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

    /// Fetch a **SHA-256 content-addressed** blob or shard from `peer` over
    /// the iroh shard ALPN ([`SHARD_ALPN`]). Thin delegation to
    /// [`fetch_blob_over_iroh`] — see that function for why this plane, and
    /// not iroh-blobs, is the one heal-on-read races.
    pub async fn fetch_blob_by_content_address(
        &self,
        peer: NodeAddr,
        content_address: &str,
    ) -> Result<Vec<u8>, IrohBlobFetchError> {
        fetch_blob_over_iroh(&self.endpoint, peer, content_address).await
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

/// Typed failure of one iroh blob fetch. Variants map 1:1 onto the declared
/// `elohim_iroh_blob_fetches_total{result}` label set via
/// [`IrohBlobFetchError::metric_result`] — the counter's contract is the
/// label set, so classification lives with the error, not at the call site.
#[derive(Debug, thiserror::Error)]
pub enum IrohBlobFetchError {
    /// The QUIC connection (or its first stream) never came up.
    #[error("iroh dial failed: {0}")]
    Dial(String),
    /// The peer answered, honestly, that it does not hold the address.
    #[error("peer does not hold {0}")]
    NotFound(String),
    /// Framing, decode, or a peer-side error string — anything that is
    /// neither a dial failure nor an honest miss.
    #[error("iroh blob fetch failed: {0}")]
    Transport(String),
}

impl IrohBlobFetchError {
    /// The `result` label this failure contributes to
    /// `elohim_iroh_blob_fetches_total`. (`ok` and `verify_failed` are
    /// produced by the caller — only the caller has seen the bytes.)
    pub fn metric_result(&self) -> &'static str {
        match self {
            IrohBlobFetchError::Dial(_) => "dial_failed",
            IrohBlobFetchError::NotFound(_) => "not_found",
            IrohBlobFetchError::Transport(_) => "error",
        }
    }
}

/// Fetch a SHA-256 content-addressed blob/shard from `peer` over
/// [`SHARD_ALPN`], returning the raw bytes **unverified** — the caller
/// verifies with the same `blob_fetch::verify_blob_hash` the libp2p leg uses,
/// so both transports pass through one verification rule.
///
/// ## Why the shard ALPN and not iroh-blobs
///
/// The heal-on-read race is addressed by SHA-256 (`sha256-<hex>` / CID form);
/// iroh-blobs is BLAKE3-addressed, so serving that race over
/// [`IrohNode::fetch_blob_from`] would require a BLAKE3 alias
/// (`peer_blob_inventory.blake3_hash`) that is NULL for every blob a peer did
/// not itself ingest through the iroh store. The shard ALPN's responder
/// ([`super::shard_backend::ShardServiceBackend`] → `ShardService::handle_get`)
/// reads the SAME `BlobStore` the libp2p `/elohim/blob/1.0.0` responder reads,
/// so the iroh leg has byte-for-byte availability parity with the libp2p leg
/// for every address the race can name.
///
/// Dial and stream I/O are separated so a connection failure is classified
/// `dial_failed` rather than collapsing into a generic `error` — the counter
/// is only useful if its labels are honest. No retry: one dial, one request,
/// one answer (the caller owns the timeout).
pub async fn fetch_blob_over_iroh(
    endpoint: &Endpoint,
    peer: NodeAddr,
    content_address: &str,
) -> Result<Vec<u8>, IrohBlobFetchError> {
    let conn = endpoint
        .connect(peer, SHARD_ALPN)
        .await
        .map_err(|e| IrohBlobFetchError::Dial(e.to_string()))?;
    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .map_err(|e| IrohBlobFetchError::Dial(e.to_string()))?;

    let req = ShardRequest::Get {
        hash: content_address.to_string(),
    };
    write_frame(&mut send, &req)
        .await
        .map_err(|e| IrohBlobFetchError::Transport(e.to_string()))?;
    send.finish()
        .map_err(|e| IrohBlobFetchError::Transport(e.to_string()))?;

    let res: ShardResponse = read_frame_default(&mut recv)
        .await
        .map_err(|e| IrohBlobFetchError::Transport(e.to_string()))?;

    match res {
        ShardResponse::Data(bytes) => Ok(bytes),
        ShardResponse::NotFound | ShardResponse::ContentNotFound => {
            Err(IrohBlobFetchError::NotFound(content_address.to_string()))
        }
        ShardResponse::Error(msg) => Err(IrohBlobFetchError::Transport(msg)),
        other => Err(IrohBlobFetchError::Transport(format!(
            "unexpected shard response: {}",
            other.summary()
        ))),
    }
}

/// The co-resident iroh transport leg, as the blob plane needs it: an
/// `Endpoint` to dial from and the [`IrohPeerBook`] that says who is dialable.
///
/// Registered once at startup (see [`register_iroh_fetch_leg`]) because the
/// heal-on-read entry points — `HttpServer::get_blob_or_heal` and the libp2p
/// node's fetch task — build their `SwarmFetchParams` from a libp2p-only
/// world and have no typed handle to the iroh stack. Both the endpoint and
/// the book are already process singletons (one of each, created once in
/// `main.rs`), so this registry names a fact rather than introducing one.
#[derive(Debug, Clone)]
pub struct IrohFetchLeg {
    endpoint: Endpoint,
    book: IrohPeerBook,
}

impl IrohFetchLeg {
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// The dialable-peer book. Its `libp2p_peer_id` / `agent_cid` fields are
    /// ROUTING HINTS only — never attribution (storage CLAUDE.md, the
    /// attribution cut).
    pub fn book(&self) -> &IrohPeerBook {
        &self.book
    }

    /// One-shot fetch of a SHA-256 content address from `peer`.
    pub async fn fetch(
        &self,
        peer: NodeAddr,
        content_address: &str,
    ) -> Result<Vec<u8>, IrohBlobFetchError> {
        fetch_blob_over_iroh(&self.endpoint, peer, content_address).await
    }
}

static IROH_FETCH_LEG: OnceLock<IrohFetchLeg> = OnceLock::new();

/// Publish the process's iroh fetch leg. Returns `false` if one was already
/// registered (first registration wins — a second iroh stack in one process
/// is not a shape this substrate has).
///
/// Called from `spawn_iroh_gossip_receive`, which is the single production
/// site holding both the endpoint and the book, and which runs exactly once
/// whenever an iroh node exists (`dual` and `iroh` backends). In `libp2p`
/// mode it never runs, so [`iroh_fetch_leg`] stays `None` and no iroh leg is
/// ever constructed on the fetch path.
pub fn register_iroh_fetch_leg(endpoint: Endpoint, book: IrohPeerBook) -> bool {
    IROH_FETCH_LEG.set(IrohFetchLeg { endpoint, book }).is_ok()
}

/// The registered iroh fetch leg, or `None` on a libp2p-only node.
pub fn iroh_fetch_leg() -> Option<&'static IrohFetchLeg> {
    IROH_FETCH_LEG.get()
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
            relay_url: None,
            use_n0_relays: false,
            use_n0_discovery: false,
            discovery_resolvers: vec![],
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
