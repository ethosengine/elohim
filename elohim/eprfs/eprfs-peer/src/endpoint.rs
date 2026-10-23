//! Binding the device's iroh endpoint.

use iroh::{Endpoint, RelayMap, RelayMode, RelayUrl};
use url::Url;

use crate::error::PeerError;
use crate::key::PeerKey;

/// The environment variable naming the relay the device peers home to. Same semantics as
/// elohim-storage's `ELOHIM_IROH_RELAY_URL`: set and non-empty, it must parse; unset or empty,
/// there is no relay.
pub const RELAY_ENV: &str = "EPR_PEER_RELAY_URL";

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
    let relay_mode = match relay {
        Some(url) => RelayMode::Custom(RelayMap::from_iter([RelayUrl::from(url)])),
        None => RelayMode::Disabled,
    };
    Endpoint::builder()
        .secret_key(key.secret().clone())
        .relay_mode(relay_mode)
        .alpns(alpns)
        .bind()
        .await
        .map_err(|e| PeerError::Bind(Box::new(e)))
}

/// The relay [`RELAY_ENV`] names, if any.
///
/// # Errors
///
/// [`PeerError::InvalidRelayUrl`] when it is set to something that is not a URL.
pub fn relay_from_env() -> Result<Option<Url>, PeerError> {
    relay_from_value(std::env::var(RELAY_ENV).ok().as_deref())
}

/// [`relay_from_env`] over a given value: unset or blank is no relay; anything else must parse.
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
