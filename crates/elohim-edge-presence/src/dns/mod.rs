//! DNS provider abstraction for dynamic DNS record management.
//!
//! # `DnsProvider` trait
//!
//! [`DnsProvider`] is the extensibility seam. Any DNS management backend
//! (Cloudflare, DuckDNS, No-IP, ddclient, test mocks) implements this trait
//! and is passed as `Arc<dyn DnsProvider>` to [`crate::orchestrate::EdgePresence`].
//!
//! # Current implementations
//!
//! | Module | Type | Status |
//! |--------|------|--------|
//! | [`cloudflare`] | [`cloudflare::CloudflareDnsProvider`] | Scaffold (Phase 2) |
//!
//! # Planned implementations (Phase 2+)
//!
//! - `duckdns.rs` — `DuckDnsDnsProvider` (HTTP GET-based update URL)
//! - `noip.rs`   — `NoIpDnsProvider` (Basic auth over HTTPS)
//! - `ddclient.rs` — `DdclientBridge` (write a ddclient config file; shell out)
//!
//! # Care-class / compute-class isolation note
//!
//! This module handles EXTERNAL operational state (Category C per the design spec
//! p2p-design-gate). DNS records are owned by the provider; the crate sets them
//! but does not store them as source-of-truth. No DHT entry types, no REA events.

pub mod cloudflare;

use std::net::{Ipv4Addr, Ipv6Addr};

use crate::error::EdgePresenceError;

// ---------------------------------------------------------------------------
// Record types (helpers for provider implementations)
// ---------------------------------------------------------------------------

/// A DNS record type discriminant used in provider API calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsRecordType {
    /// IPv4 address record.
    A,
    /// IPv6 address record.
    Aaaa,
    /// Text record (used for ACME DNS-01 challenge tokens).
    Txt,
}

impl DnsRecordType {
    /// Returns the string representation used in DNS provider APIs.
    pub fn as_str(&self) -> &'static str {
        match self {
            DnsRecordType::A => "A",
            DnsRecordType::Aaaa => "AAAA",
            DnsRecordType::Txt => "TXT",
        }
    }
}

// ---------------------------------------------------------------------------
// DnsProvider trait
// ---------------------------------------------------------------------------

/// Abstraction over a DNS provider that supports dynamic record updates.
///
/// Implementations must be `Send + Sync + 'static` so they can be held in an
/// `Arc` and shared across async tasks.
///
/// # Implementing a new provider
///
/// 1. Create a new module under `src/dns/` (e.g., `src/dns/duckdns.rs`).
/// 2. Declare `pub mod duckdns;` in this file.
/// 3. Implement [`DnsProvider`] for the new type.
/// 4. Document the required credentials / config in the type's doc comment.
///
/// # DNS-01 ACME challenge
///
/// The ACME manager calls `set_txt_record` to place the `_acme-challenge.{hostname}`
/// TXT record, waits for validation, then calls `delete_txt_record` to clean up.
/// All providers **must** implement these two methods even if they return
/// `Ok(())` as a no-op (only applicable if `AcmeChallenge::Http01` is selected).
#[async_trait::async_trait]
pub trait DnsProvider: Send + Sync + 'static {
    /// Human-readable provider name for log messages.
    fn name(&self) -> &'static str;

    /// Set or update the DNS A record for `hostname` to `ip`.
    ///
    /// If a record already exists, it should be updated (not duplicated).
    async fn set_a_record(
        &self,
        hostname: &str,
        ip: Ipv4Addr,
    ) -> Result<(), EdgePresenceError>;

    /// Set or update the DNS AAAA record for `hostname` to `ip`.
    ///
    /// Providers that do not support IPv6 may return `Ok(())` as a documented
    /// no-op. Log a warning if IPv6 was requested but is unsupported.
    async fn set_aaaa_record(
        &self,
        hostname: &str,
        ip: Ipv6Addr,
    ) -> Result<(), EdgePresenceError>;

    /// Set or update a DNS TXT record.
    ///
    /// Used for ACME DNS-01 challenges. `name` is the fully-qualified record name
    /// (e.g., `_acme-challenge.myfamily.elohim.host`). `value` is the challenge
    /// token string.
    ///
    /// If a TXT record with the same name already exists, it should be replaced.
    async fn set_txt_record(
        &self,
        name: &str,
        value: &str,
    ) -> Result<(), EdgePresenceError>;

    /// Delete a DNS TXT record by name.
    ///
    /// Called after a successful ACME DNS-01 challenge to clean up the
    /// `_acme-challenge.{hostname}` record. Idempotent: if the record does not
    /// exist, return `Ok(())`.
    async fn delete_txt_record(&self, name: &str) -> Result<(), EdgePresenceError>;
}

// ---------------------------------------------------------------------------
// NullDnsProvider (test / no-op)
// ---------------------------------------------------------------------------

/// A no-op DNS provider for testing and for `DnsProvider::None` configurations.
///
/// Logs all calls at `debug` level and returns `Ok(())` for everything.
/// Used in unit tests and when the operator has configured `DnsProvider::None`
/// (static IP or manually-managed DNS).
pub struct NullDnsProvider;

#[async_trait::async_trait]
impl DnsProvider for NullDnsProvider {
    fn name(&self) -> &'static str {
        "null"
    }

    async fn set_a_record(&self, hostname: &str, ip: Ipv4Addr) -> Result<(), EdgePresenceError> {
        tracing::debug!(hostname, %ip, "NullDnsProvider: set_a_record (no-op)");
        Ok(())
    }

    async fn set_aaaa_record(&self, hostname: &str, ip: Ipv6Addr) -> Result<(), EdgePresenceError> {
        tracing::debug!(hostname, %ip, "NullDnsProvider: set_aaaa_record (no-op)");
        Ok(())
    }

    async fn set_txt_record(&self, name: &str, value: &str) -> Result<(), EdgePresenceError> {
        tracing::debug!(name, value, "NullDnsProvider: set_txt_record (no-op)");
        Ok(())
    }

    async fn delete_txt_record(&self, name: &str) -> Result<(), EdgePresenceError> {
        tracing::debug!(name, "NullDnsProvider: delete_txt_record (no-op)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn null_provider_returns_ok_for_all_operations() {
        let p = NullDnsProvider;
        assert!(p.set_a_record("test.example", Ipv4Addr::new(127, 0, 0, 1)).await.is_ok());
        assert!(p.set_txt_record("_acme-challenge.test.example", "token123").await.is_ok());
        assert!(p.delete_txt_record("_acme-challenge.test.example").await.is_ok());
    }
}
