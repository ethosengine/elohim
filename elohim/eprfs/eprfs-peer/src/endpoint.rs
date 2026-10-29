//! Binding the device's iroh endpoint.

mod relays;

use std::net::{SocketAddr, SocketAddrV4};

use iroh::{Endpoint, NodeAddr, NodeId, RelayMap, RelayMode, RelayUrl};
use url::Url;

use crate::error::PeerError;
use crate::key::PeerKey;

/// The environment variable carrying an ordered relay override.
///
/// Empty or absent values retain the Rulebook relays. `off` is the only value that disables
/// relaying.
pub const RELAY_ENV: &str = "EPR_PEER_RELAYS";

const LEGACY_RELAY_ENV: &str = "EPR_PEER_RELAY_URL";

/// Bind this device's iroh endpoint on `key`, offering `alpns` to inbound peers.
///
/// Relay and discovery mirror elohim-storage's sovereign defaults
/// (`elohim-storage/src/p2p_iroh/endpoint.rs`): `Some(url)` homes the endpoint to that relay;
/// `None` disables relaying — direct addresses only, never a silent fall back to n0's public
/// relays. No discovery service is registered, so nothing is published to a third party;
/// peers are known from the roster's node ids and dialed at a direct address or through the
/// shared relay. Che workspaces are pods in one cluster, so direct addresses usually connect.
///
/// The caller closes the endpoint (`Endpoint::close`) on shutdown.
///
/// # Errors
///
/// [`PeerError::Bind`] when iroh cannot bind its sockets.
pub async fn bind(
    key: &PeerKey,
    relay: Option<Url>,
    alpns: Vec<Vec<u8>>,
) -> Result<Endpoint, PeerError> {
    bind_on(key, relay, alpns, None).await
}

/// [`bind`], listening on `listen` when given (a fixed IPv4 address the other devices are told
/// to dial directly, as an in-container smoke does) and on an ephemeral port otherwise.
///
/// # Errors
///
/// [`PeerError::Bind`] when iroh cannot bind its sockets (the address is taken, say).
pub async fn bind_on(
    key: &PeerKey,
    relay: Option<Url>,
    alpns: Vec<Vec<u8>>,
    listen: Option<SocketAddrV4>,
) -> Result<Endpoint, PeerError> {
    let relay_mode = match relay {
        Some(url) => RelayMode::Custom(RelayMap::from_iter([RelayUrl::from(url)])),
        None => RelayMode::Disabled,
    };
    let mut builder = Endpoint::builder()
        .secret_key(key.secret().clone())
        .relay_mode(relay_mode)
        .alpns(alpns);
    if let Some(listen) = listen {
        builder = builder.bind_addr_v4(listen);
    }
    #[cfg(feature = "testing")]
    {
        builder = builder.insecure_skip_relay_cert_verify(true);
    }
    builder
        .bind()
        .await
        .map_err(|e| PeerError::Bind(Box::new(e)))
}

/// The first relay resolved from [`RELAY_ENV`], the legacy relay variable, the Rulebook, or the
/// compiled two-premise default.
///
/// Returning one relay preserves compatibility with a peer fixed to that relay. Path-aware
/// endpoint consumers may use the complete ordered plan. Empty or absent environment values never
/// disable relaying; only `off` returns `None`.
///
/// # Errors
///
/// [`PeerError::InvalidRelayUrl`] when an override or Rulebook entry is not a URL, or
/// [`PeerError::Policy`] when a discovered Rulebook cannot be read or parsed.
pub fn relay_from_env() -> Result<Option<Url>, PeerError> {
    relays::first_from_env()
}

/// Parse one legacy relay value without consulting the Rulebook.
///
/// This helper preserves the single-value parsing surface for callers that already selected a
/// source. [`relay_from_env`] owns relay resolution and its non-disabling defaults.
///
/// # Errors
///
/// [`PeerError::InvalidRelayUrl`] when `value` is set and is not a URL.
pub fn relay_from_value(value: Option<&str>) -> Result<Option<Url>, PeerError> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    Url::parse(value)
        .map(Some)
        .map_err(|source| PeerError::InvalidRelayUrl {
            value: value.to_string(),
            source,
        })
}

/// A device's direct address given as `<node_id>@<ip:port>` (`epr peer serve --peer`). The
/// roster names node ids only; a dialer that has no relay names the address itself.
///
/// # Errors
///
/// [`PeerError::InvalidPeer`] when `arg` is not a node id, an `@` and a socket address.
pub fn peer_from_arg(arg: &str) -> Result<NodeAddr, PeerError> {
    let invalid = |reason: String| PeerError::InvalidPeer {
        value: arg.to_string(),
        reason,
    };
    let (node, addr) = arg
        .split_once('@')
        .ok_or_else(|| invalid("expected <node_id>@<ip:port>".into()))?;
    let node: NodeId = node.parse().map_err(|e| invalid(format!("node id: {e}")))?;
    let addr: SocketAddr = addr.parse().map_err(|e| invalid(format!("address: {e}")))?;
    Ok(NodeAddr::new(node).with_direct_addresses([addr]))
}
