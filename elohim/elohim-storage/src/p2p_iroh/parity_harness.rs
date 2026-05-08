//! Test fixtures for two-node iroh harnesses.
//!
//! Phase 3 deliverable. Provides a reusable [`TwoNodeFixture`] that spins
//! up provider + fetcher [`IrohNode`]s with optional custom-ALPN handlers
//! mounted, and exposes their [`NodeAddr`]s so individual phase tests can
//! drive their own protocol-specific exchanges.
//!
//! Phase 5+ tests (sync, EPR, shard, view-fed, identity) use this fixture
//! plus their own protocol-specific assertions. The fixture itself does
//! NOT contain libp2p — true parity comparison is left to per-phase tests
//! that want to assert "wire bytes match between libp2p and iroh."

use std::path::Path;

use iroh::NodeAddr;

use super::{config::IrohConfig, node::AlpnRegistration, IrohNode};

/// Loopback IrohConfig for tests. Relays disabled — the two nodes find
/// each other through the direct addresses populated by
/// `node_addr().initialized()`.
pub fn loopback_config(dir: &Path) -> IrohConfig {
    IrohConfig {
        blobs_dir: dir.join("blobs_iroh"),
        secret_key_path: dir.join("iroh.key"),
        use_n0_relays: false,
        use_n0_discovery: false,
    }
}

/// Two iroh nodes wired up loopback-style. The provider's [`NodeAddr`] is
/// resolved before [`Self::new`] returns, so callers can immediately dial
/// from `fetcher` to `provider`.
pub struct TwoNodeFixture {
    /// First node — typically the "provider" / server side.
    pub provider: IrohNode,
    /// Second node — typically the "fetcher" / client side.
    pub fetcher: IrohNode,
    /// Pre-resolved direct address of `provider`.
    pub provider_addr: NodeAddr,
}

impl TwoNodeFixture {
    /// Spin up both nodes with the given extra protocols on each side.
    /// The same set is registered on both — most tests want symmetric
    /// handlers so either node can serve. For asymmetric setups, use
    /// [`Self::new_asymmetric`].
    pub async fn new(
        provider_dir: &Path,
        fetcher_dir: &Path,
        protocol_factory: impl Fn() -> Vec<AlpnRegistration>,
    ) -> anyhow::Result<Self> {
        let provider =
            IrohNode::start_with_protocols(loopback_config(provider_dir), (protocol_factory)())
                .await?;
        let fetcher =
            IrohNode::start_with_protocols(loopback_config(fetcher_dir), (protocol_factory)())
                .await?;

        let provider_addr = provider.node_addr().await?;

        Ok(Self {
            provider,
            fetcher,
            provider_addr,
        })
    }

    /// Asymmetric variant — provider and fetcher get different protocol
    /// sets. Useful when only one side needs to serve.
    pub async fn new_asymmetric(
        provider_dir: &Path,
        provider_protocols: Vec<AlpnRegistration>,
        fetcher_dir: &Path,
        fetcher_protocols: Vec<AlpnRegistration>,
    ) -> anyhow::Result<Self> {
        let provider =
            IrohNode::start_with_protocols(loopback_config(provider_dir), provider_protocols)
                .await?;
        let fetcher =
            IrohNode::start_with_protocols(loopback_config(fetcher_dir), fetcher_protocols).await?;

        let provider_addr = provider.node_addr().await?;

        Ok(Self {
            provider,
            fetcher,
            provider_addr,
        })
    }

    /// Shut down both nodes, awaiting clean Router shutdown on each. Tests
    /// should call this at the end to avoid noisy "task aborted" warnings.
    pub async fn shutdown(self) -> anyhow::Result<()> {
        // Order doesn't matter for correctness; sequential is simpler than
        // try_join here since each shutdown is bounded.
        self.fetcher.shutdown().await?;
        self.provider.shutdown().await?;
        Ok(())
    }
}
