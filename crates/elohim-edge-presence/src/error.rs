//! Error types for elohim-edge-presence.
//!
//! All public-facing functions in this crate return [`EdgePresenceError`]. Internal
//! helpers may use `?` to propagate; the conversion is via `From<T>` impls below.

/// Unified error type for WAN registration, DNS, ACME, and TLS operations.
#[derive(Debug, thiserror::Error)]
pub enum EdgePresenceError {
    /// WAN IP discovery exhausted all configured providers.
    #[error("WAN IP discovery failed after {attempts} attempt(s): {last_error}")]
    WanIpDiscoveryFailed { attempts: u32, last_error: String },

    /// A DNS provider API call failed.
    #[error("DNS update failed for hostname '{hostname}': {source}")]
    DnsUpdateFailed { hostname: String, source: String },

    /// An ACME order or challenge step failed.
    #[error("ACME order failed for hostname '{hostname}': {source}")]
    AcmeOrderFailed { hostname: String, source: String },

    /// The DNS-01 challenge did not validate within the configured wait window.
    #[error("ACME DNS-01 challenge timed out after {elapsed_secs}s for '{hostname}'")]
    AcmeChallengeTimeout { hostname: String, elapsed_secs: u64 },

    /// Filesystem I/O error (cert store read/write).
    #[error("Certificate store I/O error: {0}")]
    CertStoreIo(#[from] std::io::Error),

    /// TLS `rustls::ServerConfig` or acceptor construction error.
    #[error("TLS configuration error: {0}")]
    TlsConfig(String),

    /// An HTTP client error occurred (WAN IP probe, DNS API, ACME communication).
    #[error("HTTP client error: {0}")]
    Http(String),

    /// Malformed or missing configuration.
    #[error("Configuration error: {0}")]
    Config(String),

    /// PEM / DER parsing error.
    #[error("Certificate parse error: {0}")]
    CertParse(String),
}

impl From<reqwest::Error> for EdgePresenceError {
    fn from(e: reqwest::Error) -> Self {
        EdgePresenceError::Http(e.to_string())
    }
}
