//! ACME certificate issuance and renewal via `instant-acme`.
//!
//! # Library choice
//!
//! `instant-acme` is chosen over `rustls-acme` because:
//! - DNS-01 challenge is first-class (the correct challenge type for NAT scenarios)
//! - Separates TLS termination from cert management (the `tls.rs` module owns TLS)
//! - Explicit order state machine is testable without replacing the hyper listener
//!
//! See design spec §4.1 for the full rationale.
//!
//! # Challenge type
//!
//! DNS-01 is the default and recommended challenge. HTTP-01 is available as a
//! fallback for deployments where the DNS provider API is unavailable (see
//! `AcmeChallenge::Http01` in [`crate::config`]).
//!
//! # Phase 1 scaffold status
//!
//! `instant-acme`, `rcgen`, and `rustls-pki-types` are NOT in `Cargo.toml` yet —
//! they are commented out. The types in this module use placeholder representations
//! (`Vec<u8>`, `String`) where the real types would come from those crates.
//!
//! Phase 3 adds those dependencies and replaces the stubs with real implementations.
//!
//! # Implementation checklist (Phase 3)
//!
//! - [ ] Add `instant-acme`, `rcgen`, `rustls-pki-types` to `Cargo.toml`
//! - [ ] Implement `AcmeManager::load_or_create_account()`
//! - [ ] Implement `AcmeManager::order()` — full DNS-01 flow
//! - [ ] Implement `cert_expiry_days()` — parse PEM and extract notAfter
//! - [ ] Add Pebble integration test (marked `#[ignore]`, gated on `PEBBLE_URL`)

use std::sync::Arc;

use crate::config::{AcmeChallenge, EdgePresenceConfig};
use crate::dns::DnsProvider;
use crate::error::EdgePresenceError;

// ---------------------------------------------------------------------------
// CertBundle — the output of a successful ACME order
// ---------------------------------------------------------------------------

/// A TLS certificate chain and its corresponding private key.
///
/// Both fields are PEM-encoded. The `cert_chain_pem` may contain multiple
/// PEM blocks (leaf cert + intermediate + root).
///
/// Persisted by [`crate::tls::CertStore`].
#[derive(Debug, Clone)]
pub struct CertBundle {
    /// PEM-encoded certificate chain (leaf first, then intermediates).
    pub cert_chain_pem: Vec<u8>,
    /// PEM-encoded private key (PKCS#8 or SEC1).
    pub private_key_pem: Vec<u8>,
    /// The hostname this certificate was issued for.
    pub hostname: String,
}

impl CertBundle {
    /// Returns the approximate number of days until certificate expiry.
    ///
    /// Parses the `notAfter` field from the leaf certificate in `cert_chain_pem`.
    ///
    /// # TODO (Phase 3): implement using `rustls-pki-types` + `x509-parser` or `rcgen`
    pub fn days_until_expiry(&self) -> Result<i64, EdgePresenceError> {
        // Phase 3: parse the PEM, extract the first DER block, decode the X.509
        // TBSCertificate validity.notAfter, subtract now().
        Err(EdgePresenceError::CertParse(
            "days_until_expiry not yet implemented (Phase 3 stub)".into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// AcmeManager
// ---------------------------------------------------------------------------

/// Manages ACME certificate issuance and renewal for a single hostname.
///
/// One `AcmeManager` is created per hostname per `EdgePresence` instance.
/// The manager is stateless across method calls; all persistent state (account
/// key, issued cert) lives in [`crate::tls::CertStore`].
pub struct AcmeManager {
    config: Arc<EdgePresenceConfig>,
    dns: Arc<dyn DnsProvider>,
}

impl AcmeManager {
    /// Create a new ACME manager.
    pub fn new(config: Arc<EdgePresenceConfig>, dns: Arc<dyn DnsProvider>) -> Self {
        Self { config, dns }
    }

    /// Obtain or renew a certificate for the configured hostname.
    ///
    /// Orchestrates the full ACME flow:
    /// 1. Load or create the ACME account key (from `cert_dir/account.key`).
    /// 2. Submit a new Order to the ACME directory.
    /// 3. For each Authorization → perform the configured challenge.
    /// 4. Generate a CSR via `rcgen`.
    /// 5. Finalize the Order and download the issued certificate chain.
    ///
    /// Returns a [`CertBundle`] ready to be persisted by the caller.
    ///
    /// # TODO (Phase 3): implement using `instant-acme`
    ///
    /// Pseudocode:
    /// ```text
    /// let account = self.load_or_create_account().await?;
    /// let mut order = account.new_order(&self.config.hostname, &[]).await?;
    /// for auth in order.authorizations().await? {
    ///     let challenge = match self.config.acme_challenge {
    ///         AcmeChallenge::Dns01 => self.perform_dns01(&auth).await?,
    ///         AcmeChallenge::Http01 => self.perform_http01(&auth).await?,
    ///     };
    ///     order.set_challenge_ready(&challenge.url).await?;
    /// }
    /// self.poll_until_ready(&mut order).await?;
    /// let (private_key, csr) = self.generate_csr()?;
    /// order.finalize(csr.as_ref()).await?;
    /// let cert_chain = order.certificate().await?;
    /// Ok(CertBundle { cert_chain_pem: cert_chain.into_bytes(), private_key_pem: private_key, ... })
    /// ```
    pub async fn obtain_cert(&self) -> Result<CertBundle, EdgePresenceError> {
        // Phase 3: implement full ACME flow.
        Err(EdgePresenceError::AcmeOrderFailed {
            hostname: self.config.hostname.clone(),
            source: "ACME order not yet implemented (Phase 3 stub)".into(),
        })
    }

    /// Perform a DNS-01 challenge for `auth`.
    ///
    /// Sets the `_acme-challenge.{hostname}` TXT record via the DNS provider,
    /// polls until the ACME server validates, then deletes the TXT record.
    ///
    /// # TODO (Phase 3): implement
    async fn perform_dns01(&self, challenge_token: &str) -> Result<(), EdgePresenceError> {
        let acme_challenge_name = format!("_acme-challenge.{}", self.config.hostname);

        tracing::info!(
            hostname = %self.config.hostname,
            record = %acme_challenge_name,
            "setting DNS-01 challenge TXT record"
        );

        // Phase 3: call self.dns.set_txt_record(&acme_challenge_name, challenge_token).await?
        // then poll ACME server for validation
        // then call self.dns.delete_txt_record(&acme_challenge_name).await?
        let _ = challenge_token;

        Err(EdgePresenceError::AcmeOrderFailed {
            hostname: self.config.hostname.clone(),
            source: "perform_dns01 not yet implemented (Phase 3 stub)".into(),
        })
    }

    /// Return the challenge response for an HTTP-01 challenge at `path`.
    ///
    /// Callers (doorway-service HTTP handler) invoke this for requests to
    /// `GET /.well-known/acme-challenge/{token}`. If this manager has a
    /// pending HTTP-01 challenge at that token, returns `Some(key_auth)`;
    /// otherwise returns `None` (handler should pass through).
    ///
    /// # TODO (Phase 3): implement using an in-memory token map
    pub fn http01_challenge_response(&self, path: &str) -> Option<String> {
        // Phase 3: maintain a HashMap<token, key_authorization> populated
        // during perform_http01() and cleared after validation.
        let _ = path;
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EdgePresenceConfig;
    use crate::dns::NullDnsProvider;

    fn make_manager() -> AcmeManager {
        let config = Arc::new(EdgePresenceConfig {
            hostname: "test.example".into(),
            ..Default::default()
        });
        let dns = Arc::new(NullDnsProvider);
        AcmeManager::new(config, dns)
    }

    #[tokio::test]
    async fn obtain_cert_returns_error_in_stub_phase() {
        let m = make_manager();
        let result = m.obtain_cert().await;
        // Phase 1/2: stub returns an error; that is correct and expected.
        assert!(result.is_err(), "stub should return error until Phase 3");
    }

    #[test]
    fn http01_challenge_response_returns_none_for_unknown_token() {
        let m = make_manager();
        assert!(
            m.http01_challenge_response("/.well-known/acme-challenge/unknown").is_none(),
            "unknown token should return None"
        );
    }

    /// Pebble integration test (Phase 3).
    /// Run with: PEBBLE_URL=https://localhost:14000 cargo test -- --ignored
    #[tokio::test]
    #[ignore = "requires Pebble ACME server (set PEBBLE_URL)"]
    async fn pebble_dns01_full_flow() {
        let _pebble_url = std::env::var("PEBBLE_URL")
            .expect("PEBBLE_URL must be set for this test");
        // Phase 3: wire up a real instant-acme order against Pebble.
        // The mock DNS provider captures TXT record calls; Pebble is configured
        // with a pre-loaded DNS-01 response handler.
        todo!("Phase 3: implement Pebble integration test");
    }
}
