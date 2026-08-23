//! Tier-1 sink: Cloudflare DNS A/AAAA upsert.
//!
//! `proxied=false` is MANDATORY — the Cloudflare proxy cannot carry UDP, so a
//! proxied record would break STUN/TURN. TTL is pinned to 60s so a WAN-IP
//! change propagates quickly.
//!
//! Two lanes compose, both driven from the same detected [`AddrUpdate`]:
//!
//! - **Exclusive lane** (`--record-name`) — one beacon owns the name outright.
//!   Since 2026-07-28 its writes carry the same ownership stamp as the shared
//!   lane, and unchanged-address cycles run a freshness verification that
//!   re-asserts the record if an external writer clobbered or deleted it
//!   (the apex-clobber incident: a forgotten ddclient reverted the record 25s
//!   after publish and the drift stood undetected).
//! - **Shared lanes** (`--shared-record <name>=<owner>`, repeatable; legacy
//!   `--shared-record-name` / `--record-owner` remains accepted) — the
//!   protocol primitive "address-set contribution with ownership +
//!   freshness": multiple beacon instances, one per WAN, each maintain their
//!   OWN record under one shared hostname (a multi-A "logical anycast"
//!   doorway set) without ever clobbering a sibling's record. Ownership
//!   rides the Cloudflare record `comment` field (`beacon-owner=<slug>;
//!   ts=<unix-seconds>`); a sibling's reap-staleness is judged against
//!   Cloudflare's own server-side `modified_on`, not the comment `ts` —
//!   fleet clocks skew by hours, and only Cloudflare's clock is one every
//!   instance agrees on.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracing::{debug, error, info, warn};

use super::{AddrUpdate, Sink};

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";
const RECORD_TTL: u32 = 60;

/// The JSON body used to create/update a Cloudflare DNS record. Pure and
/// serialization-tested so the wire shape is verifiable without the network.
///
/// `comment` carries the ownership+freshness stamp on both lanes (exclusive
/// since 2026-07-28); `None` omits the field from the serialized body
/// entirely.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DnsRecordBody {
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub content: String,
    pub ttl: u32,
    pub proxied: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Build the record body for a Cloudflare upsert: `ttl=60`, `proxied=false`,
/// and an optional `comment` (both lanes carry an ownership+freshness stamp
/// since 2026-07-28; `comment: None` omits the key entirely).
pub fn build_record_body_with_comment(
    record_type: &str,
    name: &str,
    content: &str,
    comment: Option<String>,
) -> DnsRecordBody {
    DnsRecordBody {
        record_type: record_type.to_string(),
        name: name.to_string(),
        content: content.to_string(),
        ttl: RECORD_TTL,
        proxied: false,
        comment,
    }
}

/// Ownership + freshness stamp, round-tripped through the Cloudflare record
/// `comment` field as `beacon-owner=<slug>; ts=<unix-seconds>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerStamp {
    pub owner: String,
    pub ts: u64,
}

/// Format an ownership+freshness stamp for the `comment` field.
pub fn format_owner_comment(owner: &str, ts: u64) -> String {
    format!("beacon-owner={owner}; ts={ts}")
}

/// Tolerantly parse an ownership+freshness stamp out of a record `comment`.
/// Unknown `key=value` segments are ignored (forward-compatible); a
/// missing/unparseable `beacon-owner` or `ts`, or an empty owner, yields
/// `None` rather than an error — a garbled comment must never be mistaken for
/// ownership (that would risk patching or deleting someone else's record).
pub fn parse_owner_comment(raw: &str) -> Option<OwnerStamp> {
    let mut owner: Option<String> = None;
    let mut ts: Option<u64> = None;
    for segment in raw.split(';') {
        let segment = segment.trim();
        let Some((key, value)) = segment.split_once('=') else {
            continue;
        };
        match key.trim() {
            "beacon-owner" => {
                let v = value.trim();
                if !v.is_empty() {
                    owner = Some(v.to_string());
                }
            }
            "ts" => ts = value.trim().parse::<u64>().ok(),
            _ => {} // unknown key — ignored, forward-compatible
        }
    }
    match (owner, ts) {
        (Some(owner), Some(ts)) => Some(OwnerStamp { owner, ts }),
        _ => None,
    }
}

/// Current unix time in seconds. Isolated behind a function so freshness
/// tests can construct explicit `OwnerStamp`/comment fixtures instead of
/// depending on wall-clock time.
fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Parse a Cloudflare `modified_on` RFC3339 timestamp (e.g.
/// `"2014-01-01T05:20:00.123Z"`) into unix seconds. `None` on anything
/// unparseable or pre-epoch — the reap loop treats that as fail-safe
/// "not reapable", never a hard error, since a garbled/absent
/// `modified_on` must never be mistaken for staleness.
fn parse_modified_on_unix(raw: &str) -> Option<u64> {
    OffsetDateTime::parse(raw, &Rfc3339)
        .ok()
        .and_then(|dt| u64::try_from(dt.unix_timestamp()).ok())
}

#[derive(Debug, Deserialize)]
struct CfId {
    id: String,
}

/// A DNS record as returned by the list endpoint, carrying enough to drive
/// the shared lane's ownership/freshness decisions.
#[derive(Debug, Clone, Deserialize)]
struct CfDnsRecord {
    id: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    comment: Option<String>,
    /// Cloudflare's server-side last-modified timestamp, RFC3339 (e.g.
    /// `"2014-01-01T05:20:00.123Z"`). Reap staleness is judged against THIS
    /// field, never the sibling's self-reported comment `ts` — fleet clocks
    /// skew by hours, and Cloudflare's clock is the one authority every
    /// beacon instance agrees on.
    #[serde(default)]
    modified_on: Option<String>,
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

/// Static per-instance configuration for the shared "logical anycast" lane.
#[derive(Debug, Clone)]
pub struct SharedRecordConfig {
    pub record_name: String,
    pub owner: String,
    pub refresh_secs: u64,
    pub stale_secs: u64,
}

/// A configured Cloudflare sink.
pub struct CloudflareSink {
    client: reqwest::Client,
    token: String,
    zone: String,
    record_name: String,
    enable_v6: bool,
    shared: Vec<SharedRecordConfig>,
    api_base: String,
}

impl CloudflareSink {
    pub fn new(
        client: reqwest::Client,
        token: String,
        zone: String,
        record_name: String,
        enable_v6: bool,
        shared: Vec<SharedRecordConfig>,
    ) -> Self {
        Self {
            client,
            token,
            zone,
            record_name,
            enable_v6,
            shared,
            api_base: CF_API_BASE.to_string(),
        }
    }

    /// Point this sink at a different API base (e.g. a wiremock server).
    /// Test-only — production always uses [`CF_API_BASE`].
    #[cfg(test)]
    fn with_api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }

    async fn zone_id(&self) -> Result<String> {
        let url = format!("{}/zones", self.api_base);
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

    /// List DNS records of `record_type` matching `name` (the same list
    /// endpoint backs both the exclusive and shared lanes).
    async fn list_records(
        &self,
        zone_id: &str,
        record_type: &str,
        name: &str,
    ) -> Result<Vec<CfDnsRecord>> {
        let url = format!("{}/zones/{zone_id}/dns_records", self.api_base);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("type", record_type), ("name", name)])
            .send()
            .await
            .context("cloudflare: list dns records")?;
        let env = parse_cf::<Vec<CfDnsRecord>>(resp, "cloudflare: list dns records").await?;
        Ok(env.result.unwrap_or_default())
    }

    async fn existing_record_id(&self, zone_id: &str, record_type: &str) -> Result<Option<String>> {
        let records = self
            .list_records(zone_id, record_type, &self.record_name)
            .await?;
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

    /// Ownership stamp for exclusive-lane writes: the shared-lane owner slug
    /// when configured (both alpha beacons set one), else a generic tool tag.
    /// Written into the record `comment` so the zone itself names its writer —
    /// the 2026-07-28 apex clobber was undiagnosable from the zone alone
    /// because the exclusive lane wrote no provenance.
    fn exclusive_stamp(&self) -> String {
        let owner = self
            .shared
            .first()
            .map(|s| s.owner.as_str())
            .unwrap_or("relay-addr-beacon");
        format_owner_comment(owner, now_unix())
    }

    /// Exclusive-lane upsert. Since 2026-07-28 the body carries an ownership
    /// stamp in `comment` (see [`Self::exclusive_stamp`]); before that it was
    /// deliberately comment-less ("byte-identical to the single-record
    /// design"), which left exclusive records with zero audit provenance.
    async fn upsert(&self, zone_id: &str, record_type: &str, content: &str) -> Result<()> {
        let body = build_record_body_with_comment(
            record_type,
            &self.record_name,
            content,
            Some(self.exclusive_stamp()),
        );
        match self.existing_record_id(zone_id, record_type).await? {
            Some(record_id) => {
                let url = format!("{}/zones/{zone_id}/dns_records/{record_id}", self.api_base);
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
                let url = format!("{}/zones/{zone_id}/dns_records", self.api_base);
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

    /// The shared lane for one `record_type`: partition the shared name's
    /// records into mine/siblings/unowned, then converge mine (create/patch)
    /// and reap only siblings whose stamp is older than `stale_secs`.
    /// Siblings are NEVER patched; unowned/garbled records are NEVER touched.
    async fn publish_shared_lane(
        &self,
        zone_id: &str,
        shared: &SharedRecordConfig,
        record_type: &str,
        content: &str,
    ) -> Result<()> {
        let records = self
            .list_records(zone_id, record_type, &shared.record_name)
            .await?;
        let now = now_unix();

        // Partition: mine (first match wins — extra records claiming our own
        // owner slug would mean two processes sharing a slug, an operator
        // misconfiguration this code does not try to arbitrate further).
        let mut mine: Option<(CfDnsRecord, OwnerStamp)> = None;
        for r in &records {
            if let Some(stamp) = r.comment.as_deref().and_then(parse_owner_comment) {
                if stamp.owner == shared.owner {
                    if mine.is_none() {
                        mine = Some((r.clone(), stamp));
                    } else {
                        warn!(
                            record = %shared.record_name,
                            record_type,
                            owner = %shared.owner,
                            record_id = %r.id,
                            "cloudflare: multiple shared records claim OUR owner slug — leaving extras untouched, operator should investigate"
                        );
                    }
                }
            }
        }

        // Converge mine. This freshness check compares OUR OWN comment `ts`
        // against OUR OWN local clock — same-clock by construction, so it's
        // skew-safe as-is (unlike the sibling reap loop below, which must
        // judge a DIFFERENT instance's staleness and cannot trust its
        // self-reported `ts` against fleet clock drift).
        match &mine {
            Some((rec, stamp)) => {
                let stamp_age = now.saturating_sub(stamp.ts);
                let stamp_stale = stamp_age >= shared.refresh_secs;
                if rec.content != content || stamp_stale {
                    let comment = format_owner_comment(&shared.owner, now);
                    let body = build_record_body_with_comment(
                        record_type,
                        &shared.record_name,
                        content,
                        Some(comment),
                    );
                    let url = format!("{}/zones/{zone_id}/dns_records/{}", self.api_base, rec.id);
                    let resp = self
                        .client
                        .patch(&url)
                        .bearer_auth(&self.token)
                        .json(&body)
                        .send()
                        .await
                        .context("cloudflare: patch shared record")?;
                    parse_cf::<serde_json::Value>(resp, "cloudflare: patch shared record").await?;
                    info!(
                        record = %shared.record_name,
                        record_type,
                        owner = %shared.owner,
                        content,
                        content_changed = rec.content != content,
                        stamp_refreshed = stamp_stale,
                        "cloudflare: shared record refreshed"
                    );
                } else {
                    debug!(
                        record = %shared.record_name,
                        record_type,
                        owner = %shared.owner,
                        "cloudflare: shared record unchanged and stamp fresh — no-op"
                    );
                }
            }
            None => {
                let comment = format_owner_comment(&shared.owner, now);
                let body = build_record_body_with_comment(
                    record_type,
                    &shared.record_name,
                    content,
                    Some(comment),
                );
                let url = format!("{}/zones/{zone_id}/dns_records", self.api_base);
                let resp = self
                    .client
                    .post(&url)
                    .bearer_auth(&self.token)
                    .json(&body)
                    .send()
                    .await
                    .context("cloudflare: create shared record")?;
                parse_cf::<serde_json::Value>(resp, "cloudflare: create shared record").await?;
                info!(
                    record = %shared.record_name,
                    record_type,
                    owner = %shared.owner,
                    content,
                    "cloudflare: shared record created"
                );
            }
        }

        // Siblings and unowned records: never patched. A sibling is reaped
        // (DELETEd) only once Cloudflare's server-side `modified_on` is older
        // than `stale_secs` — NEVER the sibling's self-reported comment `ts`,
        // which is judged against a DIFFERENT clock (fleet clock skew of
        // hours would otherwise reap a live, behind-clock sibling every
        // cycle and never reap an ahead-clock dead one). Every record
        // claiming OUR OWN owner slug is skipped here too (not just the
        // first-match `mine` record) — a second record bearing our slug is
        // an operator misconfiguration this code does not arbitrate, but it
        // must never be reap-eligible.
        for r in &records {
            let owner_stamp = r.comment.as_deref().and_then(parse_owner_comment);
            let is_own_owner = owner_stamp
                .as_ref()
                .is_some_and(|s| s.owner == shared.owner);
            if is_own_owner {
                continue;
            }
            match owner_stamp {
                Some(stamp) => {
                    let modified_unix = r.modified_on.as_deref().and_then(parse_modified_on_unix);
                    match modified_unix {
                        Some(modified_unix) => {
                            let age = now.saturating_sub(modified_unix);
                            if age >= shared.stale_secs {
                                let url = format!(
                                    "{}/zones/{zone_id}/dns_records/{}",
                                    self.api_base, r.id
                                );
                                let resp = self
                                    .client
                                    .delete(&url)
                                    .bearer_auth(&self.token)
                                    .send()
                                    .await
                                    .context("cloudflare: delete stale sibling record")?;
                                parse_cf::<serde_json::Value>(
                                    resp,
                                    "cloudflare: delete stale sibling record",
                                )
                                .await?;
                                warn!(
                                    record = %shared.record_name,
                                    record_type,
                                    sibling_owner = %stamp.owner,
                                    record_id = %r.id,
                                    age_secs = age,
                                    stale_secs = shared.stale_secs,
                                    "cloudflare: reaped stale sibling shared record (abandoned — no Cloudflare-side modification past shared-stale-secs)"
                                );
                            } else {
                                debug!(
                                    record = %shared.record_name,
                                    record_type,
                                    sibling_owner = %stamp.owner,
                                    record_id = %r.id,
                                    age_secs = age,
                                    "cloudflare: sibling shared record still fresh (per modified_on) — leaving untouched"
                                );
                            }
                        }
                        None => {
                            warn!(
                                record = %shared.record_name,
                                record_type,
                                sibling_owner = %stamp.owner,
                                record_id = %r.id,
                                modified_on = ?r.modified_on,
                                "cloudflare: sibling shared record has missing/unparseable modified_on — cannot judge staleness, leaving untouched (fail-safe)"
                            );
                        }
                    }
                }
                None => {
                    warn!(
                        record = %shared.record_name,
                        record_type,
                        record_id = %r.id,
                        content = %r.content,
                        "cloudflare: unowned/garbled-comment shared record present — never touched automatically, operator should investigate"
                    );
                }
            }
        }

        Ok(())
    }

    /// Run ONLY the shared lane (no exclusive `--record-name` upsert). The
    /// production unchanged-address path is [`Self::verify_freshness`] (which
    /// covers BOTH lanes since the 2026-07-28 apex clobber); this remains the
    /// shared-lane-only seam the shared-lane test suite isolates against.
    /// A true no-op (`Ok(())`, zero network calls) with no shared config.
    #[allow(dead_code)]
    pub async fn publish_shared_only(&self, update: &AddrUpdate) -> Result<()> {
        if self.shared.is_empty() {
            return Ok(());
        }
        let zone_id = self.zone_id().await?;
        for shared in &self.shared {
            self.publish_shared_lane(&zone_id, shared, "A", &update.wan_v4.to_string())
                .await?;
            if self.enable_v6 {
                if let Some(v6) = update.wan_v6 {
                    self.publish_shared_lane(&zone_id, shared, "AAAA", &v6.to_string())
                        .await?;
                }
            }
        }
        Ok(())
    }

    /// Exclusive-lane freshness verification (added after the 2026-07-28 apex
    /// clobber: an external writer — a forgotten ddclient — PATCHed the
    /// beacon-owned apex back 25s after publish, and the beacon never noticed
    /// because the exclusive lane only wrote on a detected-address change; the
    /// drift stood until a human diffed the zone). On unchanged-address
    /// cycles this re-reads the record and re-asserts it if an external
    /// writer clobbered or deleted it — read-only when everything matches,
    /// so it adds one GET per cycle and writes only on actual drift. It also
    /// backfills the ownership stamp onto adopted/hand-created records the
    /// first time it sees them.
    async fn verify_exclusive(
        &self,
        zone_id: &str,
        record_type: &str,
        content: &str,
    ) -> Result<()> {
        let records = self
            .list_records(zone_id, record_type, &self.record_name)
            .await?;
        match records.first() {
            None => {
                warn!(
                    record = %self.record_name,
                    record_type,
                    content,
                    "cloudflare: exclusive record MISSING — re-asserting (externally deleted?)"
                );
                self.upsert(zone_id, record_type, content).await
            }
            Some(rec) if rec.content != content => {
                warn!(
                    record = %self.record_name,
                    record_type,
                    expected = content,
                    observed = %rec.content,
                    "cloudflare: exclusive record DRIFTED — re-asserting. An external writer clobbered it; if this repeats every cycle, two controllers are fighting over the record — find and retire the competing writer"
                );
                self.upsert(zone_id, record_type, content).await
            }
            Some(rec)
                if rec
                    .comment
                    .as_deref()
                    .and_then(parse_owner_comment)
                    .is_none() =>
            {
                info!(
                    record = %self.record_name,
                    record_type,
                    "cloudflare: exclusive record correct but unstamped — backfilling ownership provenance comment"
                );
                self.upsert(zone_id, record_type, content).await
            }
            Some(_) => {
                debug!(
                    record = %self.record_name,
                    record_type,
                    "cloudflare: exclusive record fresh — no-op"
                );
                Ok(())
            }
        }
    }

    /// Full freshness pass for an unchanged-address cycle: verify (and only
    /// on drift, re-assert) the exclusive lane, then run the shared lane's
    /// stamp refresh + sibling reap. This is the sink's self-healing surface —
    /// [`Sink::publish`] handles address changes, this handles the world
    /// changing underneath an unchanged address.
    pub async fn verify_freshness(&self, update: &AddrUpdate) -> Result<()> {
        let zone_id = self.zone_id().await?;
        self.verify_exclusive(&zone_id, "A", &update.wan_v4.to_string())
            .await?;
        if self.enable_v6 {
            if let Some(v6) = update.wan_v6 {
                self.verify_exclusive(&zone_id, "AAAA", &v6.to_string())
                    .await?;
            }
        }
        for shared in &self.shared {
            self.publish_shared_lane(&zone_id, shared, "A", &update.wan_v4.to_string())
                .await?;
            if self.enable_v6 {
                if let Some(v6) = update.wan_v6 {
                    self.publish_shared_lane(&zone_id, shared, "AAAA", &v6.to_string())
                        .await?;
                }
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
        for shared in &self.shared {
            self.publish_shared_lane(&zone_id, shared, "A", &update.wan_v4.to_string())
                .await?;
            if self.enable_v6 {
                if let Some(v6) = update.wan_v6 {
                    self.publish_shared_lane(&zone_id, shared, "AAAA", &v6.to_string())
                        .await?;
                }
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
        let body = build_record_body_with_comment("A", "turn.elohim.host", "203.0.113.7", None);
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
        // `comment: None` must omit the key entirely (not even `null`). NOTE:
        // since 2026-07-28 the exclusive-lane upsert itself always passes a
        // stamp (see exclusive_upsert_carries_owner_stamp).
        assert!(!value.as_object().unwrap().contains_key("comment"));
    }

    #[test]
    fn aaaa_record_body_shape() {
        let body = build_record_body_with_comment("AAAA", "turn.elohim.host", "2001:db8::1", None);
        let value = serde_json::to_value(&body).unwrap();
        assert_eq!(value["type"], "AAAA");
        assert_eq!(value["content"], "2001:db8::1");
        // proxied MUST be false — a proxied record cannot carry UDP.
        assert_eq!(value["proxied"], false);
        assert_eq!(value["ttl"], 60);
        assert!(!value.as_object().unwrap().contains_key("comment"));
    }

    #[test]
    fn shared_record_body_carries_comment() {
        let comment = format_owner_comment("operations", 1_700_000_000);
        let body = build_record_body_with_comment(
            "A",
            "doorways.elohim.host",
            "203.0.113.7",
            Some(comment.clone()),
        );
        let value = serde_json::to_value(&body).unwrap();
        assert_eq!(value["comment"], comment);
    }

    #[test]
    fn owner_comment_roundtrips() {
        let comment = format_owner_comment("shem", 1_700_000_042);
        let parsed = parse_owner_comment(&comment).unwrap();
        assert_eq!(parsed.owner, "shem");
        assert_eq!(parsed.ts, 1_700_000_042);
    }

    #[test]
    fn owner_comment_ignores_unknown_keys() {
        let parsed = parse_owner_comment("beacon-owner=shem; ts=42; region=us-east").unwrap();
        assert_eq!(parsed.owner, "shem");
        assert_eq!(parsed.ts, 42);
    }

    #[test]
    fn owner_comment_tolerates_reordering_and_spacing() {
        let parsed = parse_owner_comment("ts=99 ; beacon-owner=operations").unwrap();
        assert_eq!(parsed.owner, "operations");
        assert_eq!(parsed.ts, 99);
    }

    #[test]
    fn owner_comment_garbled_is_none() {
        assert!(parse_owner_comment("").is_none());
        assert!(parse_owner_comment("not a stamp at all").is_none());
        assert!(parse_owner_comment("beacon-owner=shem").is_none()); // missing ts
        assert!(parse_owner_comment("ts=42").is_none()); // missing owner
        assert!(parse_owner_comment("beacon-owner=; ts=42").is_none()); // empty owner
        assert!(parse_owner_comment("beacon-owner=shem; ts=not-a-number").is_none());
    }
}

#[cfg(test)]
mod shared_lane_tests {
    use super::*;
    use std::net::Ipv4Addr;

    use serde_json::json;
    use wiremock::matchers::{method, path, path_regex, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const ZONE: &str = "elohim.host";
    const ZONE_ID: &str = "zone-abc123";
    const SHARED_NAME: &str = "doorways.elohim.host";
    const SECOND_SHARED_NAME: &str = "doorway-canary.elohim.host";
    const EXCLUSIVE_NAME: &str = "turn.elohim.host";
    const OWNER: &str = "operations";

    fn envelope_ok<T: Serialize>(result: T) -> serde_json::Value {
        json!({ "success": true, "errors": [], "messages": [], "result": result })
    }

    async fn mount_zone_lookup(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/zones"))
            .and(query_param("name", ZONE))
            .respond_with(ResponseTemplate::new(200).set_body_json(envelope_ok(json!([
                { "id": ZONE_ID }
            ]))))
            .mount(server)
            .await;
    }

    /// Mounts the shared-name list GET for the "A" type, returning `records`
    /// verbatim.
    async fn mount_shared_list(server: &MockServer, records: serde_json::Value) {
        mount_shared_list_typed(server, "A", records).await;
    }

    /// Mounts the shared-name list GET for an explicit `record_type` (e.g.
    /// "AAAA"), returning `records` verbatim. The `type` query param proves
    /// the list call is type-scoped rather than returning every record at
    /// the name regardless of type.
    async fn mount_shared_list_typed(
        server: &MockServer,
        record_type: &str,
        records: serde_json::Value,
    ) {
        mount_shared_list_named_typed(server, SHARED_NAME, record_type, records).await;
    }

    async fn mount_shared_list_named_typed(
        server: &MockServer,
        record_name: &str,
        record_type: &str,
        records: serde_json::Value,
    ) {
        Mock::given(method("GET"))
            .and(path(format!("/zones/{ZONE_ID}/dns_records")))
            .and(query_param("type", record_type))
            .and(query_param("name", record_name))
            .respond_with(ResponseTemplate::new(200).set_body_json(envelope_ok(records)))
            .mount(server)
            .await;
    }

    async fn mount_create(server: &MockServer, new_id: &str) {
        Mock::given(method("POST"))
            .and(path(format!("/zones/{ZONE_ID}/dns_records")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(envelope_ok(json!({ "id": new_id }))),
            )
            .mount(server)
            .await;
    }

    async fn mount_patch(server: &MockServer, record_id: &str) {
        Mock::given(method("PATCH"))
            .and(path(format!("/zones/{ZONE_ID}/dns_records/{record_id}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(envelope_ok(json!({ "id": record_id }))),
            )
            .mount(server)
            .await;
    }

    async fn mount_delete(server: &MockServer, record_id: &str) {
        Mock::given(method("DELETE"))
            .and(path(format!("/zones/{ZONE_ID}/dns_records/{record_id}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(envelope_ok(json!({ "id": record_id }))),
            )
            .mount(server)
            .await;
    }

    fn sink(server: &MockServer, shared: SharedRecordConfig) -> CloudflareSink {
        sink_with_v6(server, shared, false)
    }

    fn sink_with_v6(
        server: &MockServer,
        shared: SharedRecordConfig,
        enable_v6: bool,
    ) -> CloudflareSink {
        CloudflareSink::new(
            reqwest::Client::new(),
            "test-token".to_string(),
            ZONE.to_string(),
            EXCLUSIVE_NAME.to_string(),
            enable_v6,
            vec![shared],
        )
        .with_api_base(server.uri())
    }

    fn shared_cfg() -> SharedRecordConfig {
        SharedRecordConfig {
            record_name: SHARED_NAME.to_string(),
            owner: OWNER.to_string(),
            refresh_secs: 300,
            stale_secs: 900,
        }
    }

    fn update(ip: [u8; 4]) -> AddrUpdate {
        AddrUpdate {
            wan_v4: Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
            wan_v6: None,
            lan_v4: None,
        }
    }

    fn update_v6(ip: [u8; 4], v6: std::net::Ipv6Addr) -> AddrUpdate {
        AddrUpdate {
            wan_v4: Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
            wan_v6: Some(v6),
            lan_v4: None,
        }
    }

    /// Render a unix-seconds timestamp as the RFC3339 shape Cloudflare uses
    /// for `modified_on` (e.g. `"2014-01-01T05:20:00Z"`). Built from
    /// `OffsetDateTime`'s always-available field accessors rather than the
    /// `time` crate's `formatting` feature, so the crate's production
    /// dependency footprint stays limited to `parsing`.
    fn rfc3339(unix_secs: u64) -> String {
        let dt = OffsetDateTime::from_unix_timestamp(unix_secs as i64).unwrap();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            dt.year(),
            u8::from(dt.month()),
            dt.day(),
            dt.hour(),
            dt.minute(),
            dt.second()
        )
    }

    /// Count requests matching (method, exact path) among what the mock
    /// server actually received — the load-bearing assertion for "only MY
    /// record was mutated."
    async fn count_requests(server: &MockServer, http_method: &str, req_path: &str) -> usize {
        server
            .received_requests()
            .await
            .expect("request recording enabled by default")
            .iter()
            .filter(|r| r.method.as_str() == http_method && r.url.path() == req_path)
            .count()
    }

    // (a) shared create when absent.
    #[tokio::test]
    async fn creates_when_mine_absent() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        mount_shared_list(&server, json!([])).await;
        mount_create(&server, "new-rec-1").await;

        let s = sink(&server, shared_cfg());
        s.publish_shared_only(&update([203, 0, 113, 7]))
            .await
            .unwrap();

        assert_eq!(
            count_requests(&server, "POST", &format!("/zones/{ZONE_ID}/dns_records")).await,
            1
        );
    }

    #[tokio::test]
    async fn patches_each_shared_record_lane_in_order() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        let now = now_unix();
        mount_shared_list_named_typed(
            &server,
            SHARED_NAME,
            "A",
            json!([{ "id": "lane-one", "content": "203.0.113.1", "comment": format_owner_comment(OWNER, now) }]),
        )
        .await;
        mount_shared_list_named_typed(
            &server,
            SECOND_SHARED_NAME,
            "A",
            json!([{ "id": "lane-two", "content": "203.0.113.2", "comment": format_owner_comment(OWNER, now) }]),
        )
        .await;
        mount_patch(&server, "lane-one").await;
        mount_patch(&server, "lane-two").await;

        let second = SharedRecordConfig {
            record_name: SECOND_SHARED_NAME.to_string(),
            ..shared_cfg()
        };
        let s = CloudflareSink::new(
            reqwest::Client::new(),
            "test-token".to_string(),
            ZONE.to_string(),
            EXCLUSIVE_NAME.to_string(),
            false,
            vec![shared_cfg(), second],
        )
        .with_api_base(server.uri());

        s.publish_shared_only(&update([203, 0, 113, 7]))
            .await
            .unwrap();

        let patched_names = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.method.as_str() == "PATCH")
            .map(|request| {
                request.body_json::<serde_json::Value>().unwrap()["name"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(patched_names, vec![SHARED_NAME, SECOND_SHARED_NAME]);
    }

    // (b) patch only MY record among mine + fresh sibling + unowned.
    #[tokio::test]
    async fn patches_only_mine_among_three() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        let now = now_unix();
        mount_shared_list(
            &server,
            json!([
                { "id": "mine", "content": "203.0.113.1", "comment": format_owner_comment(OWNER, now - 10) },
                { "id": "sibling-fresh", "content": "203.0.113.2", "comment": format_owner_comment("shem", now - 10) },
                { "id": "unowned", "content": "203.0.113.3", "comment": serde_json::Value::Null },
            ]),
        )
        .await;
        mount_patch(&server, "mine").await;

        let s = sink(&server, shared_cfg());
        // Different content than "mine" currently has -> triggers a patch.
        s.publish_shared_only(&update([203, 0, 113, 99]))
            .await
            .unwrap();

        assert_eq!(
            count_requests(
                &server,
                "PATCH",
                &format!("/zones/{ZONE_ID}/dns_records/mine")
            )
            .await,
            1
        );
        assert_eq!(
            count_requests(
                &server,
                "PATCH",
                &format!("/zones/{ZONE_ID}/dns_records/sibling-fresh")
            )
            .await,
            0
        );
        assert_eq!(
            count_requests(
                &server,
                "PATCH",
                &format!("/zones/{ZONE_ID}/dns_records/unowned")
            )
            .await,
            0
        );
        assert_eq!(
            count_requests(
                &server,
                "DELETE",
                &format!("/zones/{ZONE_ID}/dns_records/sibling-fresh")
            )
            .await,
            0
        );
        assert_eq!(
            count_requests(
                &server,
                "DELETE",
                &format!("/zones/{ZONE_ID}/dns_records/unowned")
            )
            .await,
            0
        );
    }

    // (c) IP unchanged + fresh stamp -> zero mutating calls.
    #[tokio::test]
    async fn unchanged_ip_and_fresh_stamp_is_zero_mutation() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        let now = now_unix();
        mount_shared_list(
            &server,
            json!([
                { "id": "mine", "content": "203.0.113.7", "comment": format_owner_comment(OWNER, now - 5) },
            ]),
        )
        .await;
        // Intentionally do NOT mount PATCH/POST/DELETE — any attempt 404s and
        // fails the call, proving zero mutating requests were made.

        let s = sink(&server, shared_cfg());
        s.publish_shared_only(&update([203, 0, 113, 7]))
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        let mutating = requests
            .iter()
            .filter(|r| r.method.as_str() != "GET")
            .count();
        assert_eq!(
            mutating, 0,
            "expected zero mutating calls, got: {requests:?}"
        );
    }

    // (d) IP unchanged + stale own stamp -> PATCH refresh.
    #[tokio::test]
    async fn unchanged_ip_stale_own_stamp_refreshes() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        let now = now_unix();
        let stale_cfg = SharedRecordConfig {
            refresh_secs: 300,
            ..shared_cfg()
        };
        mount_shared_list(
            &server,
            json!([
                { "id": "mine", "content": "203.0.113.7", "comment": format_owner_comment(OWNER, now - 301) },
            ]),
        )
        .await;
        mount_patch(&server, "mine").await;

        let s = sink(&server, stale_cfg);
        s.publish_shared_only(&update([203, 0, 113, 7]))
            .await
            .unwrap();

        assert_eq!(
            count_requests(
                &server,
                "PATCH",
                &format!("/zones/{ZONE_ID}/dns_records/mine")
            )
            .await,
            1
        );
    }

    // (e) stale sibling (per Cloudflare's modified_on) -> exactly that record
    // DELETEd, mine untouched.
    #[tokio::test]
    async fn reaps_only_stale_sibling() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        let now = now_unix();
        mount_shared_list(
            &server,
            json!([
                { "id": "mine", "content": "203.0.113.7", "comment": format_owner_comment(OWNER, now - 5) },
                {
                    "id": "sibling-stale",
                    "content": "203.0.113.9",
                    "comment": format_owner_comment("shem", now - 901),
                    "modified_on": rfc3339(now - 901),
                },
            ]),
        )
        .await;
        mount_delete(&server, "sibling-stale").await;
        // Intentionally no PATCH mount for "mine" — unchanged + fresh stamp
        // must produce zero PATCH calls, proving mine is untouched.

        let s = sink(&server, shared_cfg());
        s.publish_shared_only(&update([203, 0, 113, 7]))
            .await
            .unwrap();

        assert_eq!(
            count_requests(
                &server,
                "DELETE",
                &format!("/zones/{ZONE_ID}/dns_records/sibling-stale")
            )
            .await,
            1
        );
        assert_eq!(
            count_requests(
                &server,
                "PATCH",
                &format!("/zones/{ZONE_ID}/dns_records/mine")
            )
            .await,
            0
        );
    }

    // (f) fresh sibling (per modified_on) -> no DELETE.
    #[tokio::test]
    async fn does_not_reap_fresh_sibling() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        let now = now_unix();
        mount_shared_list(
            &server,
            json!([
                { "id": "mine", "content": "203.0.113.7", "comment": format_owner_comment(OWNER, now - 5) },
                {
                    "id": "sibling-fresh",
                    "content": "203.0.113.9",
                    "comment": format_owner_comment("shem", now - 5),
                    "modified_on": rfc3339(now - 5),
                },
            ]),
        )
        .await;
        // No DELETE mount — any attempt fails the call.

        let s = sink(&server, shared_cfg());
        s.publish_shared_only(&update([203, 0, 113, 7]))
            .await
            .unwrap();

        assert_eq!(
            count_requests(
                &server,
                "DELETE",
                &format!("/zones/{ZONE_ID}/dns_records/sibling-fresh")
            )
            .await,
            0
        );
    }

    // (f2) REGRESSION PROOF (FIX 1 / clock skew): sibling with an HOURS-STALE
    // comment `ts` (a behind-clock sibling would look abandoned by the old
    // comment-ts logic) but a FRESH `modified_on` -> must NOT be reaped.
    // This is the exact scenario that caused permanent flap under fleet
    // clock skew.
    #[tokio::test]
    async fn skewed_comment_ts_but_fresh_modified_on_is_not_reaped() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        let now = now_unix();
        mount_shared_list(
            &server,
            json!([
                { "id": "mine", "content": "203.0.113.7", "comment": format_owner_comment(OWNER, now - 5) },
                {
                    "id": "sibling-skewed",
                    "content": "203.0.113.9",
                    // comment ts claims 5 hours stale — WAY past stale_secs=900
                    // if judged by the old comment-ts logic.
                    "comment": format_owner_comment("shem", now.saturating_sub(5 * 3600)),
                    // but Cloudflare's own modified_on says this record was
                    // touched 5 seconds ago — genuinely fresh.
                    "modified_on": rfc3339(now - 5),
                },
            ]),
        )
        .await;
        // No DELETE mount — any attempt fails the call, proving the sibling
        // was judged fresh (per modified_on), not stale (per comment ts).

        let s = sink(&server, shared_cfg());
        s.publish_shared_only(&update([203, 0, 113, 7]))
            .await
            .unwrap();

        assert_eq!(
            count_requests(
                &server,
                "DELETE",
                &format!("/zones/{ZONE_ID}/dns_records/sibling-skewed")
            )
            .await,
            0
        );
    }

    // (f3) sibling record with a missing modified_on -> NOT reaped
    // (fail-safe: a garbled/absent modified_on must never be mistaken for
    // staleness), regardless of how stale the comment ts claims to be.
    #[tokio::test]
    async fn missing_modified_on_is_not_reaped() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        let now = now_unix();
        mount_shared_list(
            &server,
            json!([
                { "id": "mine", "content": "203.0.113.7", "comment": format_owner_comment(OWNER, now - 5) },
                {
                    "id": "sibling-no-modified-on",
                    "content": "203.0.113.9",
                    "comment": format_owner_comment("shem", now - 901),
                    // modified_on intentionally omitted.
                },
            ]),
        )
        .await;
        // No DELETE mount — any attempt fails the call.

        let s = sink(&server, shared_cfg());
        s.publish_shared_only(&update([203, 0, 113, 7]))
            .await
            .unwrap();

        assert_eq!(
            count_requests(
                &server,
                "DELETE",
                &format!("/zones/{ZONE_ID}/dns_records/sibling-no-modified-on")
            )
            .await,
            0
        );
    }

    // (f4) FIX 2 PROOF: a SECOND record bearing our OWN owner slug, with a
    // stale modified_on, must NOT be reaped — only mine_id was skipped by
    // the old logic, so a same-owner duplicate was reap-eligible; every
    // record whose parsed owner matches ours must be skipped.
    #[tokio::test]
    async fn same_owner_duplicate_is_not_reaped() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        let now = now_unix();
        mount_shared_list(
            &server,
            json!([
                { "id": "mine", "content": "203.0.113.7", "comment": format_owner_comment(OWNER, now - 5) },
                {
                    "id": "mine-duplicate",
                    "content": "203.0.113.42",
                    "comment": format_owner_comment(OWNER, now - 901),
                    "modified_on": rfc3339(now - 901),
                },
            ]),
        )
        .await;
        // No DELETE mount for the duplicate — any attempt fails the call.

        let s = sink(&server, shared_cfg());
        s.publish_shared_only(&update([203, 0, 113, 7]))
            .await
            .unwrap();

        assert_eq!(
            count_requests(
                &server,
                "DELETE",
                &format!("/zones/{ZONE_ID}/dns_records/mine-duplicate")
            )
            .await,
            0
        );
    }

    // (g) garbled-comment record never deleted/patched, regardless of age.
    #[tokio::test]
    async fn garbled_comment_record_never_touched() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        mount_shared_list(
            &server,
            json!([
                { "id": "mine", "content": "203.0.113.7", "comment": format_owner_comment(OWNER, now_unix() - 5) },
                { "id": "garbled", "content": "203.0.113.9", "comment": "not-a-stamp-at-all" },
            ]),
        )
        .await;
        // No PATCH/DELETE mount for "garbled" — any attempt fails the call.

        let s = sink(&server, shared_cfg());
        s.publish_shared_only(&update([203, 0, 113, 7]))
            .await
            .unwrap();

        assert_eq!(
            count_requests(
                &server,
                "DELETE",
                &format!("/zones/{ZONE_ID}/dns_records/garbled")
            )
            .await,
            0
        );
        assert_eq!(
            count_requests(
                &server,
                "PATCH",
                &format!("/zones/{ZONE_ID}/dns_records/garbled")
            )
            .await,
            0
        );
    }

    // (h) exclusive-lane requests carry an ownership stamp (2026-07-28: the
    // pre-incident design wrote NO comment, which left beacon-owned records
    // with zero audit provenance — the apex clobber was undiagnosable from
    // the zone alone). With no shared owner configured, the stamp falls back
    // to the generic tool slug.
    #[tokio::test]
    async fn exclusive_upsert_carries_owner_stamp() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        // Exclusive-lane list: empty -> POST create.
        Mock::given(method("GET"))
            .and(path(format!("/zones/{ZONE_ID}/dns_records")))
            .and(query_param("type", "A"))
            .and(query_param("name", EXCLUSIVE_NAME))
            .respond_with(ResponseTemplate::new(200).set_body_json(envelope_ok(json!([]))))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path_regex(format!("^/zones/{ZONE_ID}/dns_records$")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(envelope_ok(json!({ "id": "excl-1" }))),
            )
            .mount(&server)
            .await;

        // No shared config -> exclusive lane only.
        let s = CloudflareSink::new(
            reqwest::Client::new(),
            "test-token".to_string(),
            ZONE.to_string(),
            EXCLUSIVE_NAME.to_string(),
            false,
            Vec::new(),
        )
        .with_api_base(server.uri());

        s.publish(&update([203, 0, 113, 7])).await.unwrap();

        let requests = server.received_requests().await.unwrap();
        let create = requests
            .iter()
            .find(|r| r.method.as_str() == "POST")
            .expect("expected a POST create request");
        let body: serde_json::Value = create.body_json().unwrap();
        assert!(
            body["comment"]
                .as_str()
                .expect("exclusive-lane body must carry an ownership stamp comment")
                .starts_with("beacon-owner=relay-addr-beacon; ts="),
            "exclusive-lane stamp malformed: {body:?}"
        );
    }

    /// Mounts the exclusive-name list GET for the "A" type, returning
    /// `records` verbatim.
    async fn mount_exclusive_list(server: &MockServer, records: serde_json::Value) {
        Mock::given(method("GET"))
            .and(path(format!("/zones/{ZONE_ID}/dns_records")))
            .and(query_param("type", "A"))
            .and(query_param("name", EXCLUSIVE_NAME))
            .respond_with(ResponseTemplate::new(200).set_body_json(envelope_ok(records)))
            .mount(server)
            .await;
    }

    /// Exclusive-lane-only sink (no shared config) — isolates verify_freshness
    /// to the exclusive lane.
    fn sink_exclusive_only(server: &MockServer) -> CloudflareSink {
        CloudflareSink::new(
            reqwest::Client::new(),
            "test-token".to_string(),
            ZONE.to_string(),
            EXCLUSIVE_NAME.to_string(),
            false,
            Vec::new(),
        )
        .with_api_base(server.uri())
    }

    // (j) freshness verify: fresh record with a valid stamp -> zero mutation.
    #[tokio::test]
    async fn verify_freshness_is_readonly_when_record_fresh() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        mount_exclusive_list(
            &server,
            json!([
                { "id": "excl-1", "content": "203.0.113.7", "comment": format_owner_comment("relay-addr-beacon", 1_700_000_000) },
            ]),
        )
        .await;
        // No PATCH/POST mounted — any mutating attempt 404s and fails the call.

        let s = sink_exclusive_only(&server);
        s.verify_freshness(&update([203, 0, 113, 7])).await.unwrap();

        assert_eq!(
            count_requests(&server, "POST", &format!("/zones/{ZONE_ID}/dns_records")).await,
            0
        );
    }

    // (k) freshness verify: external writer clobbered the content -> re-assert
    //     via PATCH. THE regression for the 2026-07-28 ddclient apex clobber.
    #[tokio::test]
    async fn verify_freshness_reasserts_on_external_clobber() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        mount_exclusive_list(
            &server,
            json!([
                { "id": "excl-1", "content": "198.51.100.9", "comment": format_owner_comment("relay-addr-beacon", 1_700_000_000) },
            ]),
        )
        .await;
        mount_patch(&server, "excl-1").await;

        let s = sink_exclusive_only(&server);
        s.verify_freshness(&update([203, 0, 113, 7])).await.unwrap();

        assert_eq!(
            count_requests(
                &server,
                "PATCH",
                &format!("/zones/{ZONE_ID}/dns_records/excl-1")
            )
            .await,
            1
        );
        // And the re-asserted body carries the corrected content + stamp.
        let requests = server.received_requests().await.unwrap();
        let patch = requests
            .iter()
            .find(|r| r.method.as_str() == "PATCH")
            .unwrap();
        let body: serde_json::Value = patch.body_json().unwrap();
        assert_eq!(body["content"], "203.0.113.7");
        assert!(body["comment"]
            .as_str()
            .unwrap()
            .starts_with("beacon-owner="));
    }

    // (l) freshness verify: record externally deleted -> re-create via POST.
    #[tokio::test]
    async fn verify_freshness_recreates_missing_record() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        mount_exclusive_list(&server, json!([])).await;
        mount_create(&server, "excl-new").await;

        let s = sink_exclusive_only(&server);
        s.verify_freshness(&update([203, 0, 113, 7])).await.unwrap();

        assert_eq!(
            count_requests(&server, "POST", &format!("/zones/{ZONE_ID}/dns_records")).await,
            1
        );
    }

    // (m) freshness verify: content correct but no ownership stamp (adopted
    //     hand-created record) -> one-time provenance backfill PATCH.
    #[tokio::test]
    async fn verify_freshness_backfills_missing_stamp() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        mount_exclusive_list(
            &server,
            json!([
                { "id": "excl-1", "content": "203.0.113.7", "comment": serde_json::Value::Null },
            ]),
        )
        .await;
        mount_patch(&server, "excl-1").await;

        let s = sink_exclusive_only(&server);
        s.verify_freshness(&update([203, 0, 113, 7])).await.unwrap();

        assert_eq!(
            count_requests(
                &server,
                "PATCH",
                &format!("/zones/{ZONE_ID}/dns_records/excl-1")
            )
            .await,
            1
        );
    }

    // (n) freshness verify with shared mode: both lanes run off ONE zone
    //     lookup — exclusive verified, shared stamp refreshed.
    #[tokio::test]
    async fn verify_freshness_runs_both_lanes_with_single_zone_lookup() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        let now = now_unix();
        mount_exclusive_list(
            &server,
            json!([
                { "id": "excl-1", "content": "203.0.113.7", "comment": format_owner_comment(OWNER, now - 5) },
            ]),
        )
        .await;
        mount_shared_list(
            &server,
            json!([
                { "id": "mine", "content": "203.0.113.7", "comment": format_owner_comment(OWNER, now - 5) },
            ]),
        )
        .await;

        let s = sink(&server, shared_cfg());
        s.verify_freshness(&update([203, 0, 113, 7])).await.unwrap();

        assert_eq!(count_requests(&server, "GET", "/zones").await, 1);
    }

    // (i) FIX 3: AAAA shared lane, create-when-absent. Proves the AAAA list
    // GET is type-scoped (a separate mock than the "A" list, both mounted
    // simultaneously) and that the created record body is a correct AAAA
    // shape with the v6 content.
    #[tokio::test]
    async fn shared_lane_aaaa_create_is_type_scoped_and_correct() {
        let server = MockServer::start().await;
        mount_zone_lookup(&server).await;
        // "A" lane: absent -> create (not under test, but must be satisfied
        // since publish_shared_only runs both lanes when enable_v6 is set).
        mount_shared_list_typed(&server, "A", json!([])).await;
        mount_shared_list_typed(&server, "AAAA", json!([])).await;

        Mock::given(method("POST"))
            .and(path(format!("/zones/{ZONE_ID}/dns_records")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(envelope_ok(json!({ "id": "new-rec" }))),
            )
            .mount(&server)
            .await;

        let s = sink_with_v6(&server, shared_cfg(), true);
        let v6: std::net::Ipv6Addr = "2001:db8::7".parse().unwrap();
        s.publish_shared_only(&update_v6([203, 0, 113, 7], v6))
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        let aaaa_create = requests
            .iter()
            .find(|r| {
                r.method.as_str() == "POST"
                    && r.body_json::<serde_json::Value>()
                        .ok()
                        .and_then(|b| b.get("type").and_then(|t| t.as_str()).map(str::to_string))
                        == Some("AAAA".to_string())
            })
            .expect("expected a POST create request for the AAAA record");
        let body: serde_json::Value = aaaa_create.body_json().unwrap();
        assert_eq!(body["type"], "AAAA");
        assert_eq!(body["name"], SHARED_NAME);
        assert_eq!(body["content"], "2001:db8::7");
        assert_eq!(body["ttl"], 60);
        assert_eq!(body["proxied"], false);
        assert!(body["comment"]
            .as_str()
            .expect("AAAA shared create must carry an ownership comment")
            .starts_with(&format!("beacon-owner={OWNER}; ts=")));

        // Also prove the AAAA list GET was made with the type-scoped query
        // param (not e.g. reusing the "A" list results for AAAA).
        let list_requests: Vec<_> = requests
            .iter()
            .filter(|r| {
                r.method.as_str() == "GET"
                    && r.url.path() == format!("/zones/{ZONE_ID}/dns_records")
            })
            .collect();
        let aaaa_list = list_requests
            .iter()
            .find(|r| r.url.query_pairs().any(|(k, v)| k == "type" && v == "AAAA"));
        assert!(
            aaaa_list.is_some(),
            "expected a type=AAAA list GET, got: {list_requests:?}"
        );
    }
}
