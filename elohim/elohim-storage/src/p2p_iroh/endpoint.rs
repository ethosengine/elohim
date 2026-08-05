//! Iroh `Endpoint` construction.
//!
//! Phase 1 builds a bare endpoint with a persisted secret key and relay
//! mode. Phase 2 will mount iroh-blobs on the endpoint via the iroh
//! `Router` integration; later phases register custom-ALPN handlers
//! alongside iroh-blobs.

use std::io;

use iroh::{
    discovery::pkarr::{PkarrPublisher, PkarrResolver},
    Endpoint, RelayMap, RelayMode, RelayUrl,
};

use super::{config::IrohConfig, identity};

/// Errors from `build_endpoint`. Bind failures and identity load failures
/// share an enum so the caller doesn't have to plumb two error types.
#[derive(Debug, thiserror::Error)]
pub enum BuildEndpointError {
    #[error("failed to load or generate iroh secret key: {0}")]
    Identity(#[from] io::Error),

    #[error("failed to bind iroh endpoint: {0}")]
    Bind(#[from] iroh::endpoint::BindError),

    /// `config.relay_url` (or `ELOHIM_IROH_RELAY_URL`) is set but does not
    /// parse as a valid relay URL. This is a hard startup error — an
    /// unparseable operator-configured relay must never silently fall back
    /// to n0 (Wave-2 relay-sovereignty design §4.4).
    #[error("invalid ELOHIM_IROH_RELAY_URL / IrohConfig::relay_url {url:?}: {source}")]
    InvalidRelayUrl {
        url: String,
        #[source]
        source: iroh::RelayUrlParseError,
    },
}

/// Build an iroh `Endpoint` from config. Caller is responsible for shutting
/// it down on graceful exit (`endpoint.close().await`).
///
/// Relay selection order (Wave-2 relay-sovereignty design §4.4 — the
/// storage-dataplane prerequisite for E3's dual-mode enablement):
/// 1. `config.relay_url` set → `RelayMode::Custom` pointed at that
///    self-hosted relay. A parse failure here is a hard startup error, never
///    a silent fallback to n0.
/// 2. Else `config.use_n0_relays` → `RelayMode::Default` (n0's public relay
///    fleet) — opt-in only, never the default.
/// 3. Else `RelayMode::Disabled` — LAN/direct-only, degraded reach. This is
///    the sovereign default: no relay is not a silent n0 dependency.
///
/// Cutover gate #10: each entry in `config.discovery_resolvers` is registered
/// as a `(PkarrPublisher, PkarrResolver)` pair via `add_discovery`. iroh wraps
/// them in `ConcurrentDiscovery` for parallel querying. n0's hosted resolver
/// is no longer hardcoded — it is one entry among many that operators choose.
/// An empty list means "no discovery" (peer addresses must be exchanged
/// out-of-band via `Endpoint::add_node_addr` — this is what tests do).
pub async fn build_endpoint(config: &IrohConfig) -> Result<Endpoint, BuildEndpointError> {
    let secret = identity::load_or_generate(&config.secret_key_path)?;

    let relay_mode = match &config.relay_url {
        Some(url) => {
            let relay_url: RelayUrl =
                url.parse()
                    .map_err(|source| BuildEndpointError::InvalidRelayUrl {
                        url: url.clone(),
                        source,
                    })?;
            RelayMode::Custom(RelayMap::from_iter([relay_url]))
        }
        None if config.use_n0_relays => RelayMode::Default,
        None => RelayMode::Disabled,
    };

    let mut builder = Endpoint::builder()
        .secret_key(secret)
        .relay_mode(relay_mode);

    for resolver in &config.discovery_resolvers {
        builder = builder.add_discovery(PkarrPublisher::builder(resolver.url.clone()));
        builder = builder.add_discovery(PkarrResolver::builder(resolver.url.clone()));
        tracing::info!(
            url = %resolver.url, kind = ?resolver.kind,
            "iroh: registered pkarr discovery resolver"
        );
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
            relay_url: None,
            use_n0_relays: false,
            use_n0_discovery: false,
            discovery_resolvers: vec![],
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
            relay_url: None,
            use_n0_relays: false,
            use_n0_discovery: false,
            discovery_resolvers: vec![],
        };

        let ep1 = build_endpoint(&cfg).await.unwrap();
        let id1 = ep1.node_id();
        ep1.close().await;

        let ep2 = build_endpoint(&cfg).await.unwrap();
        let id2 = ep2.node_id();
        ep2.close().await;

        assert_eq!(id1, id2, "persisted secret key should yield stable NodeId");
    }

    /// An operator-configured relay URL that fails to parse must abort
    /// startup with `InvalidRelayUrl`, never silently fall back to
    /// `use_n0_relays` or `RelayMode::Disabled` (Wave-2 relay-sovereignty
    /// design §4.4 — a bad self-hosted relay config must fail loudly).
    #[tokio::test]
    async fn invalid_relay_url_is_a_hard_error_not_a_silent_fallback() {
        let dir = tempdir().unwrap();
        let cfg = IrohConfig {
            blobs_dir: dir.path().join("blobs_iroh"),
            secret_key_path: dir.path().join("iroh.key"),
            relay_url: Some("not a valid url".to_string()),
            use_n0_relays: true, // must NOT be reached — relay_url wins/fails first
            use_n0_discovery: false,
            discovery_resolvers: vec![],
        };

        let err = build_endpoint(&cfg)
            .await
            .expect_err("bad relay url must error");
        assert!(matches!(err, BuildEndpointError::InvalidRelayUrl { .. }));
    }
}
