//! Configuration types for the edge-presence crate.
//!
//! [`EdgePresenceConfig`] is the single entry point a caller constructs.
//! The [`TlsMode`] enum is the gating mechanism that ensures existing k8s-ingress
//! deployments are never affected (the default is [`TlsMode::DeferToIngress`]).

use std::path::PathBuf;
use std::time::Duration;

// ---------------------------------------------------------------------------
// TLS termination mode
// ---------------------------------------------------------------------------

/// Determines how TLS is handled for inbound connections.
///
/// # k8s-ingress safety guarantee
///
/// The default is [`TlsMode::DeferToIngress`]. Existing k8s deployments that
/// rely on NGINX ingress + cert-manager for TLS termination never set this to
/// `TerminateInProcess` and are therefore unaffected. The startup log emits a
/// clear message for each mode so the operator can audit the running posture.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TlsMode {
    /// TLS is terminated externally (k8s ingress, NGINX, Cloudflare proxy).
    ///
    /// `doorway-service` or `steward/node` listens on plain TCP. No TLS
    /// acceptor is constructed. `EdgePresence` may still perform DDNS updates
    /// and certificate management, but certs are not loaded into the process.
    ///
    /// **This is the default.** Existing k8s-ingress deployments keep this.
    #[default]
    DeferToIngress,

    /// TLS is terminated in-process by the `TlsAcceptorHandle` provided by
    /// this crate.
    ///
    /// Used by bare-metal doorways and `steward/node` that are not behind a
    /// TLS-terminating reverse proxy. Requires a valid DNS-resolvable hostname
    /// and an ACME provider.
    TerminateInProcess,
}

// ---------------------------------------------------------------------------
// ACME challenge mode
// ---------------------------------------------------------------------------

/// Which ACME challenge type to use for certificate issuance.
///
/// See design spec §4.4 for the rationale. DNS-01 is the default because it
/// works behind NAT and does not require port 80 to be publicly reachable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AcmeChallenge {
    /// DNS-01 challenge: set a `_acme-challenge.{hostname}` TXT record.
    ///
    /// Requires the configured [`crate::dns::DnsProvider`] to implement
    /// `set_txt_record` and `delete_txt_record`. Works behind NAT. Supports
    /// wildcard certificates.
    ///
    /// **This is the default.**
    #[default]
    Dns01,

    /// HTTP-01 challenge: serve `/.well-known/acme-challenge/{token}` on port 80.
    ///
    /// Requires port 80 to be publicly reachable from the ACME server (Let's
    /// Encrypt). Does not require DNS API write access. Does not support wildcards.
    ///
    /// When this variant is selected, the caller must wire
    /// [`crate::orchestrate::EdgePresence::challenge_response`] into its HTTP
    /// handler for the well-known path.
    Http01,
}

// ---------------------------------------------------------------------------
// Main config struct
// ---------------------------------------------------------------------------

/// Configuration for the `EdgePresence` orchestrator.
///
/// Construct this in the caller (`doorway-service` or `steward/node`) and pass
/// it to [`crate::orchestrate::EdgePresence::new`].
///
/// # Minimal example (bare-metal doorway)
///
/// ```rust,no_run
/// use elohim_edge_presence::config::{EdgePresenceConfig, TlsMode};
/// use std::path::PathBuf;
///
/// let cfg = EdgePresenceConfig {
///     hostname: "myfamily.elohim.host".into(),
///     admin_email: Some("admin@example.com".into()),
///     tls_mode: TlsMode::TerminateInProcess,
///     cert_dir: PathBuf::from("/var/lib/elohim/edge-presence"),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct EdgePresenceConfig {
    /// Fully-qualified public hostname this node is registering.
    ///
    /// Example: `"myfamily.elohim.host"`.
    ///
    /// Used as the DNS record name, the ACME order identifier, and the
    /// `ServerName` in the TLS `ServerConfig`.
    pub hostname: String,

    /// Admin email for ACME account registration (Let's Encrypt contact).
    ///
    /// Let's Encrypt uses this for expiry warnings and policy notices.
    /// May be `None` for local / Pebble testing.
    pub admin_email: Option<String>,

    /// TLS termination mode. Defaults to [`TlsMode::DeferToIngress`].
    ///
    /// Set to [`TlsMode::TerminateInProcess`] only for bare-metal deployments
    /// that are not behind a TLS-terminating reverse proxy.
    pub tls_mode: TlsMode,

    /// Directory where ACME account key and issued certificates are stored.
    ///
    /// Files written:
    /// - `account.key`         — ACME account private key (PEM)
    /// - `{hostname}.cert`     — Certificate chain (PEM)
    /// - `{hostname}.key`      — Certificate private key (PEM)
    ///
    /// Must be writable by the process. Files are written atomically.
    pub cert_dir: PathBuf,

    /// ACME directory URL.
    ///
    /// Defaults to Let's Encrypt production:
    /// `https://acme-v02.api.letsencrypt.org/directory`
    ///
    /// For local testing with Pebble, set to `https://localhost:14000/dir`.
    /// For Let's Encrypt staging, set to
    /// `https://acme-staging-v02.api.letsencrypt.org/directory`.
    pub acme_directory_url: String,

    /// ACME challenge type. Defaults to [`AcmeChallenge::Dns01`].
    pub acme_challenge: AcmeChallenge,

    /// Days before certificate expiry at which renewal is triggered.
    ///
    /// Default: 30. Let's Encrypt certs are valid for 90 days; renewing at
    /// 30 days gives a 60-day window to recover from transient failures.
    pub renewal_threshold_days: u32,

    /// How often the renewal loop checks certificate expiry and WAN IP drift.
    ///
    /// Default: 12 hours. A short interval (e.g., 1 hour) is useful for testing.
    pub renewal_check_interval: Duration,

    /// WAN IP discovery provider URLs, tried in order.
    ///
    /// The first URL to return a valid IPv4 address wins.
    /// Default: `["https://api.ipify.org", "https://ifconfig.me/ip", "https://ipinfo.io/ip"]`
    pub wan_ip_providers: Vec<String>,

    /// Maximum number of retries for external API calls (WAN IP, DNS, ACME).
    ///
    /// Default: 5. Backoff is exponential starting at 5s, capped at 5 minutes.
    pub max_retries: u32,

    /// Maximum wait time for DNS-01 challenge validation (DNS propagation).
    ///
    /// Default: 5 minutes. Cloudflare propagates in ~30s; other providers may
    /// be slower. Pebble validates immediately (useful to set short for tests).
    pub dns01_validation_timeout: Duration,
}

impl Default for EdgePresenceConfig {
    fn default() -> Self {
        Self {
            hostname: String::new(),
            admin_email: None,
            tls_mode: TlsMode::DeferToIngress,
            cert_dir: PathBuf::from("/var/lib/elohim/edge-presence"),
            acme_directory_url: "https://acme-v02.api.letsencrypt.org/directory".into(),
            acme_challenge: AcmeChallenge::Dns01,
            renewal_threshold_days: 30,
            renewal_check_interval: Duration::from_secs(12 * 3600),
            wan_ip_providers: vec![
                "https://api.ipify.org".into(),
                "https://ifconfig.me/ip".into(),
                "https://ipinfo.io/ip".into(),
            ],
            max_retries: 5,
            dns01_validation_timeout: Duration::from_secs(5 * 60),
        }
    }
}

impl EdgePresenceConfig {
    /// Validate the configuration, returning an error description if invalid.
    pub fn validate(&self) -> Result<(), crate::error::EdgePresenceError> {
        if self.hostname.is_empty() {
            return Err(crate::error::EdgePresenceError::Config(
                "hostname must not be empty".into(),
            ));
        }
        if self.tls_mode == TlsMode::TerminateInProcess && self.admin_email.is_none() {
            // Warn but do not hard-fail: Let's Encrypt allows registration without
            // an email (not recommended; cert expiry warnings won't be sent).
            tracing::warn!(
                hostname = %self.hostname,
                "TlsMode::TerminateInProcess set but admin_email is None; \
                 Let's Encrypt expiry notices will not be delivered"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tls_mode_is_defer_to_ingress() {
        let cfg = EdgePresenceConfig::default();
        assert_eq!(cfg.tls_mode, TlsMode::DeferToIngress,
            "default TlsMode must be DeferToIngress to preserve k8s-ingress safety");
    }

    #[test]
    fn default_acme_challenge_is_dns01() {
        let cfg = EdgePresenceConfig::default();
        assert_eq!(cfg.acme_challenge, AcmeChallenge::Dns01,
            "default ACME challenge must be Dns01");
    }

    #[test]
    fn validate_rejects_empty_hostname() {
        let cfg = EdgePresenceConfig::default();
        let result = cfg.validate();
        assert!(result.is_err(), "empty hostname should fail validation");
    }

    #[test]
    fn validate_accepts_valid_config() {
        let cfg = EdgePresenceConfig {
            hostname: "myfamily.elohim.host".into(),
            admin_email: Some("admin@example.com".into()),
            ..Default::default()
        };
        cfg.validate().expect("valid config should pass validation");
    }
}
