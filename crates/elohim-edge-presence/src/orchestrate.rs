//! `EdgePresence` — the top-level orchestrator.
//!
//! Ties together WAN IP discovery, DNS updates, ACME certificate management,
//! and TLS hot-reload into a single background task.
//!
//! # Usage
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use elohim_edge_presence::{
//!     EdgePresence,
//!     config::{EdgePresenceConfig, TlsMode},
//!     dns::cloudflare::CloudflareDnsProvider,
//! };
//! use std::path::PathBuf;
//!
//! # tokio_test::block_on(async {
//! let dns = Arc::new(CloudflareDnsProvider::new("api-token", "zone-id"));
//! let config = EdgePresenceConfig {
//!     hostname: "myfamily.elohim.host".into(),
//!     admin_email: Some("admin@example.com".into()),
//!     tls_mode: TlsMode::TerminateInProcess,
//!     cert_dir: PathBuf::from("/var/lib/elohim/edge-presence"),
//!     ..Default::default()
//! };
//!
//! let ep = EdgePresence::new(config, dns).expect("edge presence init failed");
//! let tls_handle = ep.tls_handle();
//!
//! // Spawn the renewal loop as a background task.
//! tokio::spawn(ep.run());
//!
//! // tls_handle is now available for the hyper accept loop.
//! # })
//! ```
//!
//! # Phase 1 scaffold status
//!
//! `run()` is a stub that logs and returns immediately. The full orchestration
//! loop (WAN-IP → DNS → ACME → TLS hot-reload → sleep) is implemented in Phase 4.

use std::sync::Arc;

use crate::acme::AcmeManager;
use crate::config::{EdgePresenceConfig, TlsMode};
use crate::dns::DnsProvider;
use crate::error::EdgePresenceError;
use crate::tls::{CertStore, TlsAcceptorHandle};
use crate::wan_ip::WanIpDiscoverer;

/// Top-level edge presence orchestrator.
///
/// Construct with [`EdgePresence::new`], retrieve the TLS handle with
/// [`tls_handle`](Self::tls_handle), then spawn [`run`](Self::run) as a
/// background task.
pub struct EdgePresence {
    config: Arc<EdgePresenceConfig>,
    wan_ip: WanIpDiscoverer,
    acme: AcmeManager,
    cert_store: Arc<CertStore>,
    dns: Arc<dyn DnsProvider>,
}

impl EdgePresence {
    /// Construct a new `EdgePresence` orchestrator.
    ///
    /// Validates `config` and opens the certificate store. Does not make
    /// any network calls.
    ///
    /// # Errors
    ///
    /// Returns [`EdgePresenceError::Config`] if `config` is invalid (e.g.,
    /// empty hostname). Returns [`EdgePresenceError::CertStoreIo`] if
    /// `config.cert_dir` cannot be created.
    pub fn new(
        config: EdgePresenceConfig,
        dns: Arc<dyn DnsProvider>,
    ) -> Result<Self, EdgePresenceError> {
        config.validate()?;

        let cert_store = Arc::new(CertStore::open(&config.cert_dir)?);
        let config = Arc::new(config);

        let wan_ip = WanIpDiscoverer::new(config.wan_ip_providers.clone());
        let acme = AcmeManager::new(Arc::clone(&config), Arc::clone(&dns));

        tracing::info!(
            hostname = %config.hostname,
            tls_mode = ?config.tls_mode,
            dns_provider = %dns.name(),
            "EdgePresence initialised"
        );

        Ok(Self {
            config,
            wan_ip,
            acme,
            cert_store,
            dns,
        })
    }

    /// Return a `TlsAcceptorHandle` for use in the caller's TCP accept loop.
    ///
    /// The handle holds a live reference to the current `rustls::ServerConfig`
    /// (or a placeholder in Phase 1). It is safe to clone and share across tasks.
    ///
    /// When [`TlsMode::DeferToIngress`] is configured, this method returns a
    /// placeholder handle that always returns an error from `accept()` — the
    /// caller should not attempt TLS termination in this mode.
    ///
    /// # TODO (Phase 4): return a real `TlsAcceptorHandle` backed by ArcSwap
    pub fn tls_handle(&self) -> TlsAcceptorHandle {
        TlsAcceptorHandle::from_cert_store(
            Arc::clone(&self.cert_store),
            self.config.hostname.clone(),
        )
        .expect("TlsAcceptorHandle construction should not fail in Phase 1 scaffold")
    }

    /// Inject a peer-observed WAN IP address.
    ///
    /// Allows `steward/node` to supply the public address seen by a libp2p
    /// `Identify` or iroh endpoint negotiation, bypassing the external HTTP
    /// cascade for WAN IP discovery.
    ///
    /// The injected address is used on the next renewal loop iteration.
    pub fn inject_observed_wan_ip(&self, ip: std::net::Ipv4Addr) {
        self.wan_ip.inject_wan_ipv4(ip);
    }

    /// Start the background renewal loop.
    ///
    /// This async function runs indefinitely. It should be spawned with
    /// `tokio::spawn(ep.run())`.
    ///
    /// The loop:
    /// 1. Discovers the current WAN IP.
    /// 2. Updates DNS A record if IP has changed.
    /// 3. Checks if the certificate needs renewal.
    /// 4. If renewal needed: runs ACME order → saves cert → hot-reloads TLS.
    /// 5. Sleeps for `config.renewal_check_interval`.
    ///
    /// Failures in any step are logged and retried on the next iteration.
    /// Certificate expiry warnings are logged when `days_remaining < threshold_days`.
    ///
    /// # k8s-ingress safety
    ///
    /// When [`TlsMode::DeferToIngress`] is configured, steps 4 and 5 (TLS
    /// hot-reload) are skipped. DDNS updates (steps 1–2) still run because a
    /// bare-metal doorway behind a k8s NAT may still want its own DNS record.
    ///
    /// # TODO (Phase 4): implement the full loop
    pub async fn run(self) -> Result<(), EdgePresenceError> {
        match self.config.tls_mode {
            TlsMode::DeferToIngress => {
                tracing::info!(
                    hostname = %self.config.hostname,
                    "TLS mode: DeferToIngress — inbound TLS handled by k8s ingress; \
                     EdgePresence will perform DDNS updates only"
                );
            }
            TlsMode::TerminateInProcess => {
                tracing::info!(
                    hostname = %self.config.hostname,
                    cert_dir = %self.config.cert_dir.display(),
                    "TLS mode: TerminateInProcess — EdgePresence will manage \
                     DDNS + ACME + TLS hot-reload"
                );
            }
        }

        // Phase 4: implement the renewal loop.
        // Pseudocode:
        //
        // loop {
        //     // Step 1: WAN IP
        //     match self.wan_ip.discover().await {
        //         Ok(addrs) => {
        //             // Step 2: DNS update (if IP changed)
        //             if addrs.ipv4 != last_ip {
        //                 if let Err(e) = self.dns.set_a_record(&hostname, addrs.ipv4).await {
        //                     tracing::error!("DNS update failed: {e}");
        //                 } else {
        //                     last_ip = addrs.ipv4;
        //                 }
        //             }
        //         }
        //         Err(e) => tracing::error!("WAN IP discovery failed: {e}"),
        //     }
        //
        //     // Step 3: cert check
        //     if self.cert_store.needs_renewal(&hostname, threshold_days) {
        //         // Step 4: ACME
        //         match self.acme.obtain_cert().await {
        //             Ok(bundle) => {
        //                 self.cert_store.save_cert(&hostname, &bundle)?;
        //                 if tls_mode == TlsMode::TerminateInProcess {
        //                     tls_handle.reload_from_bundle(&bundle)?; // Step 5
        //                 }
        //             }
        //             Err(e) => tracing::error!("ACME order failed: {e}"),
        //         }
        //     }
        //
        //     tokio::time::sleep(renewal_check_interval).await;
        // }

        tracing::info!(
            hostname = %self.config.hostname,
            "EdgePresence::run() is a scaffold stub — renewal loop not yet implemented (Phase 4)"
        );
        Ok(())
    }
}
