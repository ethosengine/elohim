//! Tier-1 sink: Cloudflare DNS A/AAAA upsert.
//!
//! `proxied=false` is MANDATORY — the Cloudflare proxy cannot carry UDP, so a
//! proxied record would break STUN/TURN. TTL is pinned to 60s so a WAN-IP
//! change propagates quickly.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use super::{AddrUpdate, Sink};

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";
const RECORD_TTL: u32 = 60;

/// The JSON body used to create/update a Cloudflare DNS record. Pure and
/// serialization-tested so the wire shape is verifiable without the network.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DnsRecordBody {
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub content: String,
    pub ttl: u32,
    pub proxied: bool,
}

/// Build the record body for a Cloudflare upsert. `ttl=60`, `proxied=false`.
pub fn build_record_body(record_type: &str, name: &str, content: &str) -> DnsRecordBody {
    DnsRecordBody {
        record_type: record_type.to_string(),
        name: name.to_string(),
        content: content.to_string(),
        ttl: RECORD_TTL,
        proxied: false,
    }
}

#[derive(Debug, Deserialize)]
struct CfId {
    id: String,
}

/// A single Cloudflare API error `{code, message}` from the response envelope.
#[derive(Debug, Deserialize)]
struct CfError {
    code: i64,
    message: String,
}

/// The Cloudflare API response envelope wrapping every result. `success` and
/// `errors` are what an operator actually needs to see when a call fails (e.g.
/// code 9109 "Invalid access token") — an HTTP status alone hides the cause.
#[derive(Debug, Deserialize)]
struct CfEnvelope<T> {
    success: bool,
    #[serde(default)]
    errors: Vec<CfError>,
    #[serde(default)]
    result: Option<T>,
}

/// Human-readable rendering of the CF error list (code + message each).
fn format_cf_errors(errors: &[CfError]) -> String {
    if errors.is_empty() {
        return "success=false with no error detail".to_string();
    }
    errors
        .iter()
        .map(|e| format!("code {}: {}", e.code, e.message))
        .collect::<Vec<_>>()
        .join("; ")
}

/// First 300 chars of a body, for including in a decode-failure error.
fn body_snippet(text: &str) -> String {
    text.chars().take(300).collect()
}

/// Read a Cloudflare response, parse the `{success, errors, result}` envelope,
/// and turn a `success=false`/non-empty-`errors` response into an anyhow error
/// that carries the CF error code(s) + message(s). The body is read regardless
/// of HTTP status so a 4xx (e.g. bad token) still surfaces the CF error detail
/// rather than a bare status.
async fn parse_cf<T: serde::de::DeserializeOwned + Default>(
    resp: reqwest::Response,
    ctx: &str,
) -> Result<CfEnvelope<T>> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .with_context(|| format!("{ctx}: reading response body"))?;
    let env: CfEnvelope<T> = serde_json::from_str(&text).with_context(|| {
        format!(
            "{ctx}: decoding response (HTTP {status}, body: {})",
            body_snippet(&text)
        )
    })?;
    if !env.success || !env.errors.is_empty() {
        return Err(anyhow!(
            "{ctx}: cloudflare API error (HTTP {status}): {}",
            format_cf_errors(&env.errors)
        ));
    }
    Ok(env)
}

/// A configured Cloudflare sink.
pub struct CloudflareSink {
    client: reqwest::Client,
    token: String,
    zone: String,
    record_name: String,
    enable_v6: bool,
}

impl CloudflareSink {
    pub fn new(
        client: reqwest::Client,
        token: String,
        zone: String,
        record_name: String,
        enable_v6: bool,
    ) -> Self {
        Self {
            client,
            token,
            zone,
            record_name,
            enable_v6,
        }
    }

    async fn zone_id(&self) -> Result<String> {
        let url = format!("{CF_API_BASE}/zones");
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("name", self.zone.as_str())])
            .send()
            .await
            .context("cloudflare: list zones")?;
        let env = parse_cf::<Vec<CfId>>(resp, "cloudflare: list zones").await?;
        env.result
            .unwrap_or_default()
            .into_iter()
            .next()
            .map(|z| z.id)
            .ok_or_else(|| anyhow!("cloudflare: zone {:?} not found", self.zone))
    }

    async fn existing_record_id(&self, zone_id: &str, record_type: &str) -> Result<Option<String>> {
        let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records");
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("type", record_type), ("name", self.record_name.as_str())])
            .send()
            .await
            .context("cloudflare: list dns records")?;
        let env = parse_cf::<Vec<CfId>>(resp, "cloudflare: list dns records").await?;
        let records = env.result.unwrap_or_default();
        if records.len() > 1 {
            error!(
                record = %self.record_name,
                record_type,
                count = records.len(),
                "cloudflare: MORE THAN ONE matching DNS record found — stale duplicate records must be cleaned up by the operator; patching only the first"
            );
        }
        Ok(records.into_iter().next().map(|r| r.id))
    }

    async fn upsert(&self, zone_id: &str, record_type: &str, content: &str) -> Result<()> {
        let body = build_record_body(record_type, &self.record_name, content);
        match self.existing_record_id(zone_id, record_type).await? {
            Some(record_id) => {
                let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records/{record_id}");
                let resp = self
                    .client
                    .patch(&url)
                    .bearer_auth(&self.token)
                    .json(&body)
                    .send()
                    .await
                    .context("cloudflare: patch record")?;
                parse_cf::<serde_json::Value>(resp, "cloudflare: patch record").await?;
                info!(record = %self.record_name, record_type, content, "cloudflare: updated record");
            }
            None => {
                let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records");
                let resp = self
                    .client
                    .post(&url)
                    .bearer_auth(&self.token)
                    .json(&body)
                    .send()
                    .await
                    .context("cloudflare: create record")?;
                parse_cf::<serde_json::Value>(resp, "cloudflare: create record").await?;
                info!(record = %self.record_name, record_type, content, "cloudflare: created record");
            }
        }
        Ok(())
    }
}

impl Sink for CloudflareSink {
    fn name(&self) -> &'static str {
        "cloudflare"
    }

    async fn publish(&self, update: &AddrUpdate) -> Result<()> {
        let zone_id = self.zone_id().await?;
        debug!(zone = %self.zone, zone_id, "cloudflare: resolved zone id");
        self.upsert(&zone_id, "A", &update.wan_v4.to_string())
            .await?;
        if self.enable_v6 {
            if let Some(v6) = update.wan_v6 {
                self.upsert(&zone_id, "AAAA", &v6.to_string()).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_body_shape() {
        let body = build_record_body("A", "turn.elohim.host", "203.0.113.7");
        let value = serde_json::to_value(&body).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "type": "A",
                "name": "turn.elohim.host",
                "content": "203.0.113.7",
                "ttl": 60,
                "proxied": false
            })
        );
    }

    #[test]
    fn aaaa_record_body_shape() {
        let body = build_record_body("AAAA", "turn.elohim.host", "2001:db8::1");
        let value = serde_json::to_value(&body).unwrap();
        assert_eq!(value["type"], "AAAA");
        assert_eq!(value["content"], "2001:db8::1");
        // proxied MUST be false — a proxied record cannot carry UDP.
        assert_eq!(value["proxied"], false);
        assert_eq!(value["ttl"], 60);
    }
}
