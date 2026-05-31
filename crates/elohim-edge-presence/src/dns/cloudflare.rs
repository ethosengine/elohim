//! Cloudflare DNS provider implementation.
//!
//! # Authentication
//!
//! Requires a Cloudflare API token with `Zone:DNS:Edit` permission for the
//! target zone. Scoping the token to the specific zone is recommended over
//! using a global API key.
//!
//! # API usage
//!
//! - `GET  /zones/{zone_id}/dns_records?type=A&name={hostname}` — find existing A record
//! - `POST /zones/{zone_id}/dns_records`                        — create new record
//! - `PUT  /zones/{zone_id}/dns_records/{record_id}`            — replace existing record
//! - `DELETE /zones/{zone_id}/dns_records/{record_id}`          — delete TXT record after DNS-01
//!
//! TTL is set to `120` seconds (the minimum Cloudflare allows). This ensures
//! that a WAN IP change propagates within ~2 minutes.
//!
//! # Phase 1 scaffold status
//!
//! All method bodies are `todo!()` stubs. The struct shape, constructor, and
//! Cloudflare API payload types are production-ready. Phase 2 implements the
//! reqwest calls inside each method.
//!
//! # Implementation checklist (Phase 2)
//!
//! - [ ] `set_a_record`: GET existing → if found PUT else POST
//! - [ ] `set_aaaa_record`: same pattern for AAAA
//! - [ ] `set_txt_record`: POST `_acme-challenge.{name}` with TXT type
//! - [ ] `delete_txt_record`: GET to find record ID → DELETE
//! - [ ] Add `wiremock` unit tests for each method
//! - [ ] Test both "create new" and "update existing" code paths

use std::net::{Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};

use crate::dns::DnsProvider;
use crate::error::EdgePresenceError;

/// Base URL for the Cloudflare v4 API.
const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// DNS record TTL written for all records managed by this crate.
///
/// 120 seconds is the minimum Cloudflare allows. It ensures prompt propagation
/// after a WAN IP change or ACME DNS-01 challenge placement.
const CF_TTL_SECONDS: u32 = 120;

// ---------------------------------------------------------------------------
// Cloudflare API payload types
// ---------------------------------------------------------------------------

/// Request payload for creating or replacing a Cloudflare DNS record.
///
/// Ref: <https://developers.cloudflare.com/api/operations/dns-records-for-a-zone-create-dns-record>
#[derive(Debug, Serialize)]
struct CfDnsRecordWrite {
    /// DNS record type (e.g., `"A"`, `"AAAA"`, `"TXT"`).
    #[serde(rename = "type")]
    record_type: String,
    /// Fully-qualified record name (e.g., `"myfamily.elohim.host"`).
    name: String,
    /// Record content (IP address as string for A/AAAA; challenge token for TXT).
    content: String,
    /// TTL in seconds.
    ttl: u32,
    /// When `true`, the record is proxied through Cloudflare's edge.
    ///
    /// **Always `false` for this crate.** Proxied records hide the real IP from
    /// DNS, which breaks the ACME DNS-01 challenge and defeats the purpose of
    /// DDNS (the whole point is that peers see the real WAN IP).
    proxied: bool,
}

/// A single DNS record as returned by Cloudflare's list endpoint.
#[derive(Debug, Deserialize)]
struct CfDnsRecord {
    /// Cloudflare-assigned record ID. Needed for PUT/DELETE operations.
    id: String,
    /// Record type.
    #[serde(rename = "type")]
    record_type: String,
    /// Current record content.
    content: String,
}

/// Envelope for Cloudflare API list responses.
#[derive(Debug, Deserialize)]
struct CfListResponse {
    result: Vec<CfDnsRecord>,
    success: bool,
}

/// Envelope for Cloudflare API single-record responses (create/update/delete).
#[derive(Debug, Deserialize)]
struct CfRecordResponse {
    success: bool,
    errors: Vec<CfApiError>,
}

/// A Cloudflare API error object.
#[derive(Debug, Deserialize)]
struct CfApiError {
    code: u32,
    message: String,
}

// ---------------------------------------------------------------------------
// CloudflareDnsProvider
// ---------------------------------------------------------------------------

/// Cloudflare DNS provider.
///
/// # Constructor
///
/// ```rust,no_run
/// use elohim_edge_presence::dns::cloudflare::CloudflareDnsProvider;
///
/// let provider = CloudflareDnsProvider::new(
///     "your-cloudflare-api-token".into(),
///     "your-zone-id".into(),
/// );
/// ```
///
/// # Credentials
///
/// - `api_token`: Cloudflare API token with `Zone:DNS:Edit` permission.
///   Obtain at <https://dash.cloudflare.com/profile/api-tokens>.
/// - `zone_id`: The zone ID for the domain being managed. Visible in the
///   Cloudflare dashboard "Overview" tab for the domain.
pub struct CloudflareDnsProvider {
    /// Cloudflare API token (Bearer auth).
    api_token: String,
    /// Cloudflare zone ID for the managed domain.
    zone_id: String,
    /// HTTP client with the Authorization header pre-configured.
    client: reqwest::Client,
}

impl CloudflareDnsProvider {
    /// Create a new Cloudflare DNS provider.
    ///
    /// # Panics
    ///
    /// Panics if the `reqwest::Client` cannot be built (only possible if the
    /// TLS backend fails to initialise, which would be a system-level issue).
    pub fn new(api_token: impl Into<String>, zone_id: impl Into<String>) -> Self {
        let api_token = api_token.into();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", api_token)
                .parse()
                .expect("API token must be a valid HTTP header value"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().expect("content-type is valid"),
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("elohim-edge-presence/0.1")
            .build()
            .expect("failed to build Cloudflare HTTP client");
        Self {
            api_token,
            zone_id: zone_id.into(),
            client,
        }
    }

    /// Find an existing DNS record by type and name.
    ///
    /// Returns `Some(record_id)` if found, `None` if no matching record exists.
    ///
    /// # TODO (Phase 2): implement via GET /zones/{zone_id}/dns_records
    async fn find_record(
        &self,
        record_type: &str,
        name: &str,
    ) -> Result<Option<String>, EdgePresenceError> {
        // Phase 2: implement as:
        //   GET {CF_API_BASE}/zones/{zone_id}/dns_records?type={type}&name={name}
        //   → parse CfListResponse → return first result.id if non-empty
        let _ = (record_type, name); // silence unused-variable warning in scaffold
        todo!(
            "Phase 2: implement find_record via \
            GET {CF_API_BASE}/zones/{zone_id}/dns_records?type={type}&name={name}"
        )
    }

    /// Create a new DNS record.
    ///
    /// # TODO (Phase 2): implement via POST /zones/{zone_id}/dns_records
    async fn create_record(&self, payload: &CfDnsRecordWrite) -> Result<(), EdgePresenceError> {
        // Phase 2: POST {CF_API_BASE}/zones/{zone_id}/dns_records
        let _ = payload;
        todo!("Phase 2: implement create_record via POST")
    }

    /// Replace an existing DNS record identified by `record_id`.
    ///
    /// # TODO (Phase 2): implement via PUT /zones/{zone_id}/dns_records/{id}
    async fn replace_record(
        &self,
        record_id: &str,
        payload: &CfDnsRecordWrite,
    ) -> Result<(), EdgePresenceError> {
        let _ = (record_id, payload);
        todo!("Phase 2: implement replace_record via PUT")
    }

    /// Delete a DNS record by `record_id`.
    ///
    /// # TODO (Phase 2): implement via DELETE /zones/{zone_id}/dns_records/{id}
    async fn delete_record(&self, record_id: &str) -> Result<(), EdgePresenceError> {
        let _ = record_id;
        todo!("Phase 2: implement delete_record via DELETE")
    }

    /// Upsert a record: update if it already exists, create if it doesn't.
    ///
    /// This is the shared logic for `set_a_record` and `set_aaaa_record`.
    ///
    /// # TODO (Phase 2): implement after find_record / create_record / replace_record
    async fn upsert_record(&self, payload: CfDnsRecordWrite) -> Result<(), EdgePresenceError> {
        let _ = payload;
        todo!("Phase 2: implement upsert_record")
    }
}

#[async_trait::async_trait]
impl DnsProvider for CloudflareDnsProvider {
    fn name(&self) -> &'static str {
        "cloudflare"
    }

    /// Update the A record for `hostname` to `ip`.
    ///
    /// If the record already exists and the IP is unchanged, this is a no-op
    /// (Phase 2 will implement the "same IP, skip PUT" optimisation).
    ///
    /// # TODO (Phase 2): implement
    async fn set_a_record(
        &self,
        hostname: &str,
        ip: Ipv4Addr,
    ) -> Result<(), EdgePresenceError> {
        let _ = (hostname, ip);
        todo!(
            "Phase 2: upsert Cloudflare A record for {} → {}",
            hostname,
            ip
        )
    }

    /// Update the AAAA record for `hostname` to `ip`.
    ///
    /// # TODO (Phase 2): implement
    async fn set_aaaa_record(
        &self,
        hostname: &str,
        ip: Ipv6Addr,
    ) -> Result<(), EdgePresenceError> {
        let _ = (hostname, ip);
        todo!("Phase 2: upsert Cloudflare AAAA record")
    }

    /// Set the `_acme-challenge.{name}` TXT record to `value`.
    ///
    /// Used by the ACME DNS-01 challenge flow. The record name passed here
    /// is already the full `_acme-challenge.{hostname}` — this method does
    /// not prepend the `_acme-challenge.` prefix.
    ///
    /// # TODO (Phase 2): implement
    async fn set_txt_record(
        &self,
        name: &str,
        value: &str,
    ) -> Result<(), EdgePresenceError> {
        let _ = (name, value);
        todo!("Phase 2: create Cloudflare TXT record for ACME DNS-01 challenge")
    }

    /// Delete the TXT record at `name`.
    ///
    /// Called after a successful ACME DNS-01 challenge. Idempotent: if the
    /// record does not exist, returns `Ok(())`.
    ///
    /// # TODO (Phase 2): implement
    async fn delete_txt_record(&self, name: &str) -> Result<(), EdgePresenceError> {
        let _ = name;
        todo!("Phase 2: delete Cloudflare TXT record")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: the provider can be constructed without panicking.
    /// Does not make network calls.
    #[test]
    fn provider_constructs_without_panic() {
        let _ = CloudflareDnsProvider::new("test-token", "test-zone-id");
    }

    /// Phase 2 TODO: add wiremock tests for each DnsProvider method.
    ///
    /// Pattern:
    /// ```rust,no_run
    /// use wiremock::{MockServer, Mock, ResponseTemplate};
    /// use wiremock::matchers::{method, path_regex};
    ///
    /// #[tokio::test]
    /// async fn set_a_record_creates_when_not_exists() {
    ///     let server = MockServer::start().await;
    ///     // Mock GET (list) returning empty
    ///     Mock::given(method("GET"))
    ///         .respond_with(ResponseTemplate::new(200).set_body_json(
    ///             serde_json::json!({ "result": [], "success": true })
    ///         ))
    ///         .mount(&server)
    ///         .await;
    ///     // Mock POST (create) returning success
    ///     Mock::given(method("POST"))
    ///         .respond_with(ResponseTemplate::new(200).set_body_json(
    ///             serde_json::json!({ "success": true, "errors": [] })
    ///         ))
    ///         .mount(&server)
    ///         .await;
    ///
    ///     let provider = CloudflareDnsProvider::new_with_base_url("token", "zone", server.uri());
    ///     provider
    ///         .set_a_record("myfamily.elohim.host", "203.0.113.1".parse().unwrap())
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    #[test]
    fn phase_2_wiremock_tests_placeholder() {
        // This test exists to mark the test location for Phase 2.
        // It passes trivially.
    }
}
