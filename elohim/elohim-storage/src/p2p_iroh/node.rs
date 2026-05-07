//! `IrohNode` — Phase 1 stub.
//!
//! Phase 1 holds only the endpoint; just enough to verify the boot path and
//! prove the persisted secret key resolves to a stable [`iroh::NodeId`].
//! Phase 2 expands this to mount iroh-blobs on the endpoint via the
//! `iroh::protocol::Router` integration; later phases register custom-ALPN
//! handlers alongside iroh-blobs.

use iroh::{Endpoint, NodeId};
use tracing::info;

use super::{config::IrohConfig, endpoint::BuildEndpointError};

/// Iroh-side P2P node. Phase 1 holds only the endpoint.
#[derive(Debug)]
pub struct IrohNode {
    endpoint: Endpoint,
}

impl IrohNode {
    /// Build the endpoint and announce ourselves. Caller is responsible for
    /// shutting down via [`IrohNode::shutdown`] on graceful exit.
    pub async fn start(config: IrohConfig) -> Result<Self, BuildEndpointError> {
        let endpoint = super::endpoint::build_endpoint(&config).await?;

        info!(
            target: "elohim_storage::p2p_iroh",
            node_id = %endpoint.node_id(),
            relays = config.use_n0_relays,
            blobs_dir = %config.blobs_dir.display(),
            "iroh node started (Phase 1 stub: blob plane not yet mounted)"
        );

        Ok(Self { endpoint })
    }

    /// This node's [`iroh::NodeId`] (derived from the persisted secret key).
    pub fn node_id(&self) -> NodeId {
        self.endpoint.node_id()
    }

    /// Borrow the endpoint. Phase 2 promotes this surface to richer
    /// blob-plane methods.
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Shut down the endpoint gracefully.
    pub async fn shutdown(self) {
        info!(
            target: "elohim_storage::p2p_iroh",
            node_id = %self.endpoint.node_id(),
            "iroh node shutting down"
        );
        self.endpoint.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn start_then_shutdown() {
        let dir = tempdir().unwrap();
        let cfg = IrohConfig {
            blobs_dir: dir.path().join("blobs_iroh"),
            secret_key_path: dir.path().join("iroh.key"),
            use_n0_relays: false,
        };

        let node = IrohNode::start(cfg).await.unwrap();
        let _id = node.node_id();
        node.shutdown().await;
    }
}
