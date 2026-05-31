//! WAN IP discovery — determines this node's current public IPv4 address.
//!
//! # Discovery strategy
//!
//! A cascade of public HTTPS IP-reflection services is tried in order. The first
//! service to return a valid IPv4 address wins. On complete failure,
//! [`WanIpDiscoveryFailed`](crate::error::EdgePresenceError::WanIpDiscoveryFailed)
//! is returned.
//!
//! An optional "injected" address allows `steward/node` to supply a peer-observed
//! WAN address from its libp2p `Identify` or iroh endpoint negotiation result,
//! bypassing the external HTTP cascade entirely.
//!
//! # No network calls in this module (Phase 1 scaffold)
//!
//! The `discover()` method body is a stub. Phase 2 wires the real reqwest calls.
//! See the design spec implementation checklist §Phase 2.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::EdgePresenceError;

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// The discovered WAN addresses for this node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WanAddresses {
    /// Public IPv4 address. Always present (discovery fails if not found).
    pub ipv4: Ipv4Addr,
    /// Public IPv6 address, if detected. May be absent.
    pub ipv6: Option<Ipv6Addr>,
}

// ---------------------------------------------------------------------------
// WanIpDiscoverer
// ---------------------------------------------------------------------------

/// Discovers the node's public WAN IPv4 address.
///
/// # Usage
///
/// ```rust,no_run
/// use elohim_edge_presence::wan_ip::WanIpDiscoverer;
///
/// # tokio_test::block_on(async {
/// let discoverer = WanIpDiscoverer::new(vec![
///     "https://api.ipify.org".into(),
///     "https://ifconfig.me/ip".into(),
/// ]);
/// let addrs = discoverer.discover().await.expect("WAN IP discovery failed");
/// println!("WAN IP: {}", addrs.ipv4);
/// # })
/// ```
pub struct WanIpDiscoverer {
    /// Ordered list of HTTPS IP-reflection service URLs.
    ///
    /// Each URL should return the caller's public IPv4 as a plain text body
    /// (e.g., `"203.0.113.1"`). `ipify.org`, `ifconfig.me/ip`, and
    /// `ipinfo.io/ip` all follow this convention.
    provider_urls: Vec<String>,

    /// Optional injected WAN address from a peer-observed transport source
    /// (e.g., libp2p `Identify` or iroh endpoint negotiation).
    ///
    /// When set, `discover()` returns this address immediately without making
    /// any external HTTP calls. Useful for `steward/node` which has an
    /// in-process P2P runtime.
    injected: Arc<RwLock<Option<WanAddresses>>>,

    /// HTTP client shared across discovery calls.
    client: reqwest::Client,
}

impl WanIpDiscoverer {
    /// Create a new discoverer with the given provider URL cascade.
    pub fn new(provider_urls: Vec<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("elohim-edge-presence/0.1")
            .build()
            .expect("failed to build WAN IP discovery HTTP client");
        Self {
            provider_urls,
            injected: Arc::new(RwLock::new(None)),
            client,
        }
    }

    /// Inject a WAN address from a peer-observed transport source.
    ///
    /// After injection, [`discover()`] returns this address until it is cleared
    /// with [`clear_injected`](Self::clear_injected).
    ///
    /// # `steward/node` usage
    ///
    /// ```rust,no_run
    /// # use elohim_edge_presence::wan_ip::WanIpDiscoverer;
    /// # use std::net::Ipv4Addr;
    /// # let discoverer = WanIpDiscoverer::new(vec![]);
    /// // Called when libp2p Identify fires with the observed address:
    /// discoverer.inject_wan_ipv4(Ipv4Addr::new(203, 0, 113, 1));
    /// ```
    pub fn inject_wan_ipv4(&self, ip: Ipv4Addr) {
        let injected = Arc::clone(&self.injected);
        // Synchronous injection for simplicity; caller is on an async runtime.
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async move {
                *injected.write().await = Some(WanAddresses { ipv4: ip, ipv6: None });
            })
        });
    }

    /// Clear an injected address, falling back to the external provider cascade.
    pub async fn clear_injected(&self) {
        *self.injected.write().await = None;
    }

    /// Discover the current public WAN IPv4 address.
    ///
    /// Returns the injected address if one has been set via
    /// [`inject_wan_ipv4`](Self::inject_wan_ipv4), otherwise tries each
    /// configured provider URL in order.
    ///
    /// # Errors
    ///
    /// Returns [`EdgePresenceError::WanIpDiscoveryFailed`] if all providers
    /// fail (network error, invalid response, or non-IPv4 content).
    ///
    /// # TODO (Phase 2): implement real reqwest cascade
    ///
    /// The body of this function is currently a stub that returns an error.
    /// Phase 2 replaces it with the actual cascade logic:
    ///
    /// ```text
    /// for url in &self.provider_urls {
    ///     match self.client.get(url).send().await {
    ///         Ok(resp) if resp.status().is_success() => {
    ///             let text = resp.text().await?.trim().to_string();
    ///             if let Ok(ip) = text.parse::<Ipv4Addr>() {
    ///                 return Ok(WanAddresses { ipv4: ip, ipv6: None });
    ///             }
    ///         }
    ///         _ => continue,
    ///     }
    /// }
    /// Err(EdgePresenceError::WanIpDiscoveryFailed { ... })
    /// ```
    pub async fn discover(&self) -> Result<WanAddresses, EdgePresenceError> {
        // Check injected address first (no external call needed).
        if let Some(injected) = self.injected.read().await.clone() {
            tracing::debug!(ipv4 = %injected.ipv4, "using injected WAN address");
            return Ok(injected);
        }

        // Phase 2: implement real cascade. Stub returns an error.
        // This makes the scaffold honest: it does not pretend to have network access.
        // TODO(Phase 2): replace with reqwest cascade over self.provider_urls
        Err(EdgePresenceError::WanIpDiscoveryFailed {
            attempts: 0,
            last_error: "WAN IP discovery not yet implemented (Phase 2 stub)".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn injected_address_bypasses_cascade() {
        let discoverer = WanIpDiscoverer::new(vec![]);
        discoverer.inject_wan_ipv4(Ipv4Addr::new(203, 0, 113, 42));
        let result = discoverer.discover().await;
        assert!(result.is_ok(), "injected address should succeed");
        assert_eq!(result.unwrap().ipv4, Ipv4Addr::new(203, 0, 113, 42));
    }

    #[tokio::test]
    async fn no_providers_returns_error() {
        let discoverer = WanIpDiscoverer::new(vec![]);
        let result = discoverer.discover().await;
        assert!(result.is_err(), "empty providers with no injection should fail");
    }

    #[tokio::test]
    async fn clear_injected_falls_back_to_cascade() {
        let discoverer = WanIpDiscoverer::new(vec!["https://api.ipify.org".into()]);
        discoverer.inject_wan_ipv4(Ipv4Addr::new(203, 0, 113, 1));
        discoverer.clear_injected().await;
        // After clearing, discover() will try the cascade. In Phase 1 the
        // cascade body is a stub that returns an error; that is the correct
        // behaviour here — it proves clearing worked.
        let result = discoverer.discover().await;
        assert!(result.is_err(), "after clearing injection, stub cascade should fail");
    }
}
