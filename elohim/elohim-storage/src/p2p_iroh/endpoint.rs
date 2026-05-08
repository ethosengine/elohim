//! Iroh `Endpoint` construction.
//!
//! Phase 1 builds a bare endpoint with a persisted secret key and relay
//! mode. Phase 2 will mount iroh-blobs on the endpoint via the iroh
//! `Router` integration; later phases register custom-ALPN handlers
//! alongside iroh-blobs.

use std::io;

use iroh::{Endpoint, RelayMode};

use super::{config::IrohConfig, identity};

/// Errors from `build_endpoint`. Bind failures and identity load failures
/// share an enum so the caller doesn't have to plumb two error types.
#[derive(Debug, thiserror::Error)]
pub enum BuildEndpointError {
    #[error("failed to load or generate iroh secret key: {0}")]
    Identity(#[from] io::Error),

    #[error("failed to bind iroh endpoint: {0}")]
    Bind(#[from] iroh::endpoint::BindError),
}

/// Build an iroh `Endpoint` from config. Caller is responsible for shutting
/// it down on graceful exit (`endpoint.close().await`).
///
/// Phase 10: when `config.use_n0_discovery` is `true`, n0's DNS-based
/// peer discovery is enabled. This replaces libp2p's Kademlia for record
/// publication during cutover and post-cutover. Tests with relays
/// disabled typically also disable discovery and exchange `NodeAddr`
/// directly via `endpoint.add_node_addr`.
pub async fn build_endpoint(config: &IrohConfig) -> Result<Endpoint, BuildEndpointError> {
    let secret = identity::load_or_generate(&config.secret_key_path)?;

    let relay_mode = if config.use_n0_relays {
        RelayMode::Default
    } else {
        RelayMode::Disabled
    };

    let mut builder = Endpoint::builder()
        .secret_key(secret)
        .relay_mode(relay_mode);

    if config.use_n0_discovery {
        builder = builder.discovery_n0();
    }

    let endpoint = builder.bind().await?;

    Ok(endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Loopback endpoint build (relays disabled). Verifies the SecretKey
    /// load → builder → bind path works end-to-end and we get a usable
    /// NodeId out the other side.
    #[tokio::test]
    async fn builds_endpoint_with_relays_disabled() {
        let dir = tempdir().unwrap();
        let cfg = IrohConfig {
            blobs_dir: dir.path().join("blobs_iroh"),
            secret_key_path: dir.path().join("iroh.key"),
            use_n0_relays: false,
            use_n0_discovery: false,
        };

        let ep = build_endpoint(&cfg).await.expect("endpoint binds");
        // sanity: NodeId derives from secret key
        let _id = ep.node_id();
        ep.close().await;
    }

    /// Two builds reading the same key file produce endpoints with the
    /// same NodeId — the persisted identity is stable across restarts.
    #[tokio::test]
    async fn endpoint_node_id_stable_across_restarts() {
        let dir = tempdir().unwrap();
        let cfg = IrohConfig {
            blobs_dir: dir.path().join("blobs_iroh"),
            secret_key_path: dir.path().join("iroh.key"),
            use_n0_relays: false,
            use_n0_discovery: false,
        };

        let ep1 = build_endpoint(&cfg).await.unwrap();
        let id1 = ep1.node_id();
        ep1.close().await;

        let ep2 = build_endpoint(&cfg).await.unwrap();
        let id2 = ep2.node_id();
        ep2.close().await;

        assert_eq!(id1, id2, "persisted secret key should yield stable NodeId");
    }
}
