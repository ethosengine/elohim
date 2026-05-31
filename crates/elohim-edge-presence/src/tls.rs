//! TLS certificate store and hot-reloadable acceptor handle.
//!
//! # Phase 1 scaffold status
//!
//! `tokio-rustls`, `rustls`, and `arc-swap` are NOT in `Cargo.toml` yet.
//! This module defines the public interface using concrete placeholder types
//! (`Vec<u8>`) where `rustls` types would appear. Phase 4 adds the dependencies
//! and replaces the stubs.
//!
//! # Design
//!
//! The `CertStore` handles filesystem persistence (atomic write of PEM files).
//! The `TlsAcceptorHandle` provides a cloneable, live reference to the current
//! `rustls::ServerConfig`, hot-swapped on cert renewal via `ArcSwap` so the
//! server process never needs to restart.
//!
//! # ArcSwap hot-reload pattern (Phase 4)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │ EdgePresence renewal loop                           │
//! │   → AcmeManager::obtain_cert()                     │
//! │   → CertStore::save_cert()                         │
//! │   → build new rustls::ServerConfig from saved cert  │
//! │   → TlsAcceptorHandle.inner.store(Arc::new(config)) │
//! └─────────────────────────────────────────────────────┘
//!                        ↑ ArcSwap::store()
//! ┌─────────────────────────────────────────────────────┐
//! │ hyper accept loop (per-connection)                  │
//! │   → TlsAcceptorHandle.accept(tcp_stream)           │
//! │   → tokio_rustls::TlsAcceptor::from(               │
//! │       handle.inner.load_full()                      │ ← ArcSwap::load()
//! │     ).accept(stream).await                          │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! In-flight connections keep their existing TLS session (the old `ServerConfig`
//! stays alive via the Arc until the last connection using it drops). New
//! connections get the new config immediately after the swap.
//!
//! # Implementation checklist (Phase 4)
//!
//! - [ ] Add `tokio-rustls = "0.26"`, `rustls = "0.23"`, `arc-swap = "1.7"` to Cargo.toml
//! - [ ] Replace placeholder `ServerConfigPlaceholder` with `Arc<rustls::ServerConfig>`
//! - [ ] Implement `CertStore::load_cert()` and `save_cert()` (atomic write via temp+rename)
//! - [ ] Implement `TlsAcceptorHandle::accept()` using `tokio_rustls::TlsAcceptor`
//! - [ ] Implement `TlsAcceptorHandle::cert_expiry()` for `PublicSurfaceState` probe
//! - [ ] Unit test: save cert, load cert, verify `needs_renewal()` threshold
//! - [ ] Unit test: ArcSwap hot-reload (write new cert, verify handle produces new config)

use std::path::PathBuf;
use std::time::SystemTime;

use crate::acme::CertBundle;
use crate::error::EdgePresenceError;

// ---------------------------------------------------------------------------
// Placeholder for Phase 4 (replaces rustls types until those deps are added)
// ---------------------------------------------------------------------------

/// Placeholder for `Arc<rustls::ServerConfig>` until Phase 4.
///
/// Phase 4 replaces this with the real type.
#[derive(Debug, Clone)]
pub struct ServerConfigPlaceholder;

// ---------------------------------------------------------------------------
// CertStore
// ---------------------------------------------------------------------------

/// Persistent, filesystem-backed certificate store.
///
/// # File layout
///
/// All files are stored under `base_dir` (configurable via `EdgePresenceConfig.cert_dir`):
///
/// ```text
/// {base_dir}/
///   account.key         — ACME account private key (PEM, PKCS#8)
///   {hostname}.cert     — Certificate chain (PEM, leaf first)
///   {hostname}.key      — Certificate private key (PEM, PKCS#8)
/// ```
///
/// # Atomicity
///
/// All writes go to a `.tmp` sibling first, then are renamed into place.
/// This ensures readers never see a partial write.
pub struct CertStore {
    base_dir: PathBuf,
}

impl CertStore {
    /// Open the cert store at `base_dir`, creating the directory if needed.
    pub fn open(base_dir: impl Into<PathBuf>) -> Result<Self, EdgePresenceError> {
        let base_dir = base_dir.into();
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    /// Return the path to the certificate PEM file for `hostname`.
    fn cert_path(&self, hostname: &str) -> PathBuf {
        self.base_dir.join(format!("{}.cert", hostname))
    }

    /// Return the path to the private key PEM file for `hostname`.
    fn key_path(&self, hostname: &str) -> PathBuf {
        self.base_dir.join(format!("{}.key", hostname))
    }

    /// Return the path to the ACME account key.
    fn account_key_path(&self) -> PathBuf {
        self.base_dir.join("account.key")
    }

    /// Load a certificate bundle for `hostname` from disk.
    ///
    /// Returns `Ok(None)` if no certificate exists yet (first-run or after
    /// manual deletion). Returns `Ok(Some(bundle))` if both `{hostname}.cert`
    /// and `{hostname}.key` exist and are readable.
    ///
    /// # TODO (Phase 4): implement
    pub fn load_cert(&self, hostname: &str) -> Result<Option<CertBundle>, EdgePresenceError> {
        let cert_path = self.cert_path(hostname);
        let key_path = self.key_path(hostname);
        if !cert_path.exists() || !key_path.exists() {
            return Ok(None);
        }
        // Phase 4: read files, construct CertBundle
        let _ = (cert_path, key_path);
        todo!("Phase 4: implement CertStore::load_cert()")
    }

    /// Save a certificate bundle for `hostname` to disk atomically.
    ///
    /// Writes to `.tmp` files first, then renames into place.
    ///
    /// # TODO (Phase 4): implement
    pub fn save_cert(&self, hostname: &str, bundle: &CertBundle) -> Result<(), EdgePresenceError> {
        let _ = (hostname, bundle);
        todo!("Phase 4: implement CertStore::save_cert() with atomic write")
    }

    /// Load the ACME account private key from disk.
    ///
    /// Returns `Ok(None)` if no account key exists yet.
    ///
    /// # TODO (Phase 4): implement
    pub fn load_account_key(&self) -> Result<Option<Vec<u8>>, EdgePresenceError> {
        let path = self.account_key_path();
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read(&path)?))
    }

    /// Save the ACME account private key to disk atomically.
    ///
    /// # TODO (Phase 4): implement atomic write
    pub fn save_account_key(&self, key_pem: &[u8]) -> Result<(), EdgePresenceError> {
        // Phase 4: write to .tmp then rename
        let path = self.account_key_path();
        std::fs::write(&path, key_pem)?;
        Ok(())
    }

    /// Check whether the certificate for `hostname` needs renewal.
    ///
    /// Returns `true` if:
    /// - No certificate exists, OR
    /// - The certificate expires within `threshold_days` days.
    ///
    /// Returns `false` if the certificate is valid and has more than
    /// `threshold_days` until expiry.
    ///
    /// # TODO (Phase 4): parse cert expiry from PEM using rustls-pki-types
    pub fn needs_renewal(&self, hostname: &str, threshold_days: u32) -> bool {
        let cert_path = self.cert_path(hostname);
        if !cert_path.exists() {
            return true; // No cert at all — definitely needs issuance.
        }
        // Phase 4: parse the cert, extract notAfter, compare to now + threshold_days.
        // Until Phase 4 the method returns true (conservative: always renew).
        // This is safe: ACME rate limits apply, but initial setup will always obtain a cert.
        tracing::debug!(
            hostname,
            threshold_days,
            "CertStore::needs_renewal (Phase 4 stub — conservatively returns true)"
        );
        true
    }
}

// ---------------------------------------------------------------------------
// TlsAcceptorHandle
// ---------------------------------------------------------------------------

/// A cloneable, live reference to the current TLS server configuration.
///
/// Wrap a plain `tokio::net::TcpStream` with [`accept`](Self::accept) to
/// produce a TLS stream that hyper can use.
///
/// The underlying `ServerConfig` is hot-swapped on cert renewal via `ArcSwap`
/// — no restart needed.
///
/// # Phase 4 note
///
/// The inner `Arc<ServerConfigPlaceholder>` is a stand-in for
/// `Arc<arc_swap::ArcSwap<Arc<rustls::ServerConfig>>>`. Phase 4 replaces it.
#[derive(Clone)]
pub struct TlsAcceptorHandle {
    /// Phase 4: replace with `Arc<arc_swap::ArcSwap<Arc<rustls::ServerConfig>>>`
    _inner: std::sync::Arc<std::sync::Mutex<ServerConfigPlaceholder>>,
    /// Cert store reference for expiry queries.
    cert_store: std::sync::Arc<CertStore>,
    /// Hostname this handle is associated with.
    hostname: String,
}

impl TlsAcceptorHandle {
    /// Build a handle from a cert store and hostname.
    ///
    /// Called by `EdgePresence::tls_handle()` after the first successful cert
    /// issuance. Returns `None` if no certificate is present in the store yet.
    ///
    /// # TODO (Phase 4): implement — load cert from store, build ServerConfig, wrap in ArcSwap
    pub fn from_cert_store(
        cert_store: std::sync::Arc<CertStore>,
        hostname: impl Into<String>,
    ) -> Result<Self, EdgePresenceError> {
        let hostname = hostname.into();
        // Phase 4: load cert from cert_store.load_cert(&hostname),
        // build rustls::ServerConfig, wrap in ArcSwap
        Ok(Self {
            _inner: std::sync::Arc::new(std::sync::Mutex::new(ServerConfigPlaceholder)),
            cert_store,
            hostname,
        })
    }

    /// Accept a TCP stream, performing the TLS handshake.
    ///
    /// Returns a `tokio_rustls::server::TlsStream<tokio::net::TcpStream>` that
    /// hyper can use via `TokioIo::new(tls_stream)`.
    ///
    /// # TODO (Phase 4): implement using `tokio_rustls::TlsAcceptor`
    pub async fn accept(
        &self,
        _stream: tokio::net::TcpStream,
    ) -> Result<(), EdgePresenceError> {
        // Phase 4: let acceptor = tokio_rustls::TlsAcceptor::from(self._inner.load_full());
        //           acceptor.accept(stream).await.map_err(...)
        Err(EdgePresenceError::TlsConfig(
            "TlsAcceptorHandle::accept not yet implemented (Phase 4 stub)".into(),
        ))
    }

    /// Return the number of days until the current certificate expires.
    ///
    /// Returns `None` if no certificate is loaded.
    ///
    /// Used by `doorway-service`'s `build_public_surface()` to populate
    /// `PublicSurfaceState.tls_expires_in_days`.
    ///
    /// # TODO (Phase 4): implement by querying CertBundle::days_until_expiry()
    pub fn cert_expiry_days(&self) -> Option<i64> {
        // Phase 4: load cert from store, call days_until_expiry()
        let _ = &self.cert_store;
        let _ = &self.hostname;
        None
    }

    /// Reload the TLS configuration from a newly-issued certificate bundle.
    ///
    /// Called by the `EdgePresence` renewal loop after a successful ACME order.
    /// Hot-swaps the `ArcSwap` so new connections immediately use the new cert.
    ///
    /// # TODO (Phase 4): implement using ArcSwap::store()
    pub(crate) fn reload_from_bundle(&self, bundle: &crate::acme::CertBundle) -> Result<(), EdgePresenceError> {
        let _ = bundle;
        // Phase 4: parse cert+key from bundle, build new ServerConfig, ArcSwap::store()
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cert_store_creates_dir_on_open() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let dir = tmp.path().join("edge-presence-test");
        let store = CertStore::open(&dir).expect("open should succeed");
        assert!(dir.exists(), "base_dir should be created");
        let _ = store;
    }

    #[test]
    fn needs_renewal_returns_true_when_cert_absent() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let store = CertStore::open(tmp.path()).expect("open");
        assert!(
            store.needs_renewal("test.example", 30),
            "should need renewal when no cert exists"
        );
    }

    #[test]
    fn load_account_key_returns_none_when_absent() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let store = CertStore::open(tmp.path()).expect("open");
        let result = store.load_account_key().expect("load_account_key should not error");
        assert!(result.is_none(), "no key file should return None");
    }
}
