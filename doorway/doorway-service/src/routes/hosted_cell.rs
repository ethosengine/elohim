//! The hosted cell as a notarized promise, not a silent favour.
//!
//! When a doorway provisions a newcomer their own Holochain cell, some real
//! household is lending real compute. Before this module that lending was
//! invisible: a row in the doorway's own database said "conductor-0" and nothing
//! anywhere else recorded that a promise had been made, by whom, or until when.
//! A human could not check it, a household could not point at it, and nobody
//! could tell a withdrawn promise from one that was never made.
//!
//! So the hosted cell is issued as what it already was — **contracted compute**.
//! It reuses the substrate's ONE primitive for delegated privilege: a
//! `Mishpat::Commitment` with a `delegates-compute` action, scoped
//! `hosted-cell`. No new entry type, no new HTTP route on the doorway, and no
//! out-of-band admin grant.
//!
//! # Who promises, and to whom
//!
//! The **provider** is the steward's pool peer acting for itself — the
//! elohim-storage peer that runs beside the conductor doing the hosting. That is
//! not a doorway choice; it is forced by the grant surface, which refuses any
//! request whose `X-Verified-Performer` is not that peer's own cell actor. The
//! **recipient** is the newcomer's freshly provisioned agent key. The doorway is
//! the messenger, never the promiser.
//!
//! # Why the whole leg is non-fatal
//!
//! A doorway with no pool-compute configuration registers humans EXACTLY as it
//! did before and leaves `hosted_cell_grant_cid` unset. A doorway whose pool peer
//! is unreachable still registers the human. Turning "the notary was briefly
//! unreachable" into "you cannot have an account" would trade a missing record
//! for a missing person.
//!
//! # What this module's count is, and is not
//!
//! [`live_hosted_cell_filter`] and [`humans_served`] answer "how many humans is
//! THIS doorway hosting right now". That answer is a doorway-local PROJECTION of
//! the notary's answer, not a substrate read — see the honesty note on
//! [`live_hosted_cell_filter`].

use std::sync::OnceLock;
use std::time::Duration;

use bson::{doc, Document};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::config::Args;

/// Header the pool peer's local compute API checks its capability token on.
/// Must match `local_token_authorized` in
/// `elohim/elohim-storage/src/api/compute_tasks.rs`.
const COMPUTE_TOKEN_HEADER: &str = "x-elohim-compute-token";

/// Header naming the actor the grant is performed BY. The grant surface refuses
/// unless this is the storage peer's own cell actor (`same_actor`), which is why
/// the provider is structurally the pool peer and can never be the doorway.
const VERIFIED_PERFORMER_HEADER: &str = "x-elohim-verified-performer";

/// The one grant surface. A body variant selects issue vs withdraw — there is no
/// second route family, and this module must not invent one.
const GRANTS_PATH: &str = "/api/v1/compute/grants";

/// The delegated scope a hosted cell is issued under. An entry in the storage
/// peer's `GRANT_SCOPES` list, not caller-invented bytes.
pub const HOSTED_CELL_SCOPE: &str = "hosted-cell";

/// How long one hosted-cell promise runs before it must be renewed.
///
/// Bounded on purpose: an unbounded promise is not a promise, it is an
/// assumption. Renewal is a household's choice to make again, and a lapsed
/// promise reads as lapsed rather than as betrayal.
pub const HOSTED_CELL_DAYS: i64 = 30;

/// Bounds pinned by D2 and by `compute_grants::grant_input`'s validator.
const RATE_PER_HOUR: u64 = 60;
const ROTATION_TTL_DAYS: u64 = 30;
const REACH_CEILING: &str = "commons";

/// Where this doorway's pool peer answers, and the credentials to speak to it.
///
/// All three parts are required together. The performer key is not optional
/// convenience: without it the grant surface refuses every request, so a
/// two-thirds-configured doorway would fail on every registration rather than
/// skip the leg — which is the failure this type exists to make impossible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolComputeConfig {
    pub url: String,
    pub token: String,
    pub performer: String,
}

/// Is this doorway configured to notarize the cells its pool hosts?
///
/// Pure. Absent or blank configuration answers `None`, and `None` means the
/// grant leg is SKIPPED — registration proceeds untouched. Keyed on
/// configuration alone; `dev_mode` is deliberately not read, because it is true
/// on every deployed manifest and so cannot discriminate anything (D1).
pub fn pool_compute_config(args: &Args) -> Option<PoolComputeConfig> {
    let url = args.pool_compute_url.as_deref()?.trim();
    let token = args.pool_compute_token.as_deref()?.trim();
    let performer = args.pool_compute_performer.as_deref()?.trim();
    if url.is_empty() || token.is_empty() || performer.is_empty() {
        return None;
    }
    Some(PoolComputeConfig {
        url: url.trim_end_matches('/').to_string(),
        token: token.to_string(),
        performer: performer.to_string(),
    })
}

/// A hosted-cell promise, as the doorway records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedCellGrant {
    /// The commitment's CID — its ENTRY hash, which is what
    /// `GET /api/v1/commitments/{id}` reads back. Never the action hash.
    pub grant_cid: String,
    /// RFC3339 UTC, seconds precision. See [`rfc3339_utc_secs`].
    pub valid_until: String,
    /// The steward agent key the POOL PEER named as provider of this promise.
    ///
    /// Read straight off the peer's own answer (`provider` on
    /// `POST /api/v1/compute/grants`), where it is written as
    /// `hc.agent_key_uhcak()` — the peer's OWN conductor cell key
    /// (`elohim/elohim-storage/src/api/compute_grants.rs:261`). The doorway
    /// never computes it and never substitutes its own identity for it: this is
    /// the performing household naming itself, which is the only reason the
    /// account page may say who is hosting someone.
    ///
    /// `None` when the peer answered without one — the promise still stands,
    /// the household just goes unnamed rather than mis-named.
    pub provider: Option<String>,
}

/// Canonical stamp format for every hosted-cell time this doorway writes.
///
/// Seconds precision and a literal `Z`, so every stamp is exactly 20 characters
/// and lexicographic order IS chronological order. [`live_hosted_cell_filter`]
/// leans on that: it compares stamps as strings in the query, which is only
/// correct because nothing writes a different shape.
pub fn rfc3339_utc_secs(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// The exact body that issues one hosted-cell promise.
///
/// Pure, so the bounds can be read and pinned without a conductor. Every field
/// is constrained by the grant surface's own validator: `epr_scope` 1..64
/// entries, `reach_ceiling` exactly `commons`, positive finite `rate_per_hour`
/// and `rotation_ttl_days`, and a window no longer than
/// `rotation_ttl_days * 86400`. Sending a body that violates any of them is a
/// refusal, not a partial grant.
pub fn hosted_cell_grant_body(recipient: &str, now: DateTime<Utc>) -> (Value, String) {
    let valid_until = rfc3339_utc_secs(now + chrono::Duration::days(HOSTED_CELL_DAYS));
    let stamp = rfc3339_utc_secs(now);
    let body = json!({
        "recipient": recipient,
        "scope": HOSTED_CELL_SCOPE,
        "issuedAt": stamp,
        "validFrom": stamp,
        "validUntil": valid_until,
        "bounds": {
            "epr_scope": ["*"],
            "reach_ceiling": REACH_CEILING,
            "rate_per_hour": RATE_PER_HOUR,
            "rotation_ttl_days": ROTATION_TTL_DAYS,
        }
    });
    (body, valid_until)
}

/// The body that WITHDRAWS a hosted-cell promise.
///
/// Same route, body variant — `{grantCid, revoke: true}` — because withdrawal is
/// the same authority acting in reverse, not a different capability.
pub fn hosted_cell_revoke_body(grant_cid: &str) -> Value {
    json!({ "grantCid": grant_cid, "revoke": true })
}

/// One pooled client for the pool-peer hop, with short bounded timeouts.
///
/// A registration must never hang on an unreachable notary: the human is
/// standing at the door. Ten seconds, then the leg is skipped with a warning.
fn pool_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default()
    })
}

/// Issue the hosted-cell promise for a freshly provisioned agent.
///
/// Returns `Err` with a human-readable reason on ANY failure. Every caller must
/// treat that as "no promise recorded", never as "registration failed".
pub async fn issue_hosted_cell_grant(
    cfg: &PoolComputeConfig,
    recipient_agent_pub_key: &str,
    now: DateTime<Utc>,
) -> Result<HostedCellGrant, String> {
    let (body, valid_until) = hosted_cell_grant_body(recipient_agent_pub_key, now);
    let value = post_grant(cfg, &body).await?;
    let grant_cid = value
        .get("grantCid")
        .and_then(Value::as_str)
        .filter(|c| !c.is_empty())
        .ok_or_else(|| format!("grant answer carried no grantCid: {value}"))?
        .to_string();
    let provider = provider_named_by_peer(&value);
    debug!(
        recipient = %recipient_agent_pub_key,
        grant_cid = %grant_cid,
        provider = provider.as_deref().unwrap_or("<unnamed>"),
        "hosted-cell promise notarized"
    );
    Ok(HostedCellGrant {
        grant_cid,
        valid_until,
        provider,
    })
}

/// The steward agent key the pool peer named as the promise's provider.
///
/// Pure. A blank or missing `provider` is `None`, never an empty string: an
/// empty name is worse than no name, because the row would then claim a
/// household exists to be looked up.
pub fn provider_named_by_peer(grant_answer: &Value) -> Option<String> {
    grant_answer
        .get("provider")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
}

/// Withdraw a hosted-cell promise. `Ok(true)` means the provider had already
/// withdrawn it — which is a success, not an error: withdrawal is terminal, and
/// asking twice must not read as a failure to the human closing their account.
pub async fn revoke_hosted_cell_grant(
    cfg: &PoolComputeConfig,
    grant_cid: &str,
) -> Result<bool, String> {
    let value = post_grant(cfg, &hosted_cell_revoke_body(grant_cid)).await?;
    Ok(value
        .get("alreadyWithdrawn")
        .and_then(Value::as_bool)
        .unwrap_or(false))
}

async fn post_grant(cfg: &PoolComputeConfig, body: &Value) -> Result<Value, String> {
    let response = pool_client()
        .post(format!("{}{GRANTS_PATH}", cfg.url))
        .header(COMPUTE_TOKEN_HEADER, &cfg.token)
        .header(VERIFIED_PERFORMER_HEADER, &cfg.performer)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("pool compute unreachable: {e}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("pool compute answer unreadable: {e}"))?;
    if !status.is_success() {
        // Carry the peer's own words: "local compute API disabled" and
        // "local-cell-actor-required" are different operator problems, and
        // flattening them to "grant failed" hides which one to fix.
        warn!(status = %status, body = %text, "pool compute refused a hosted-cell grant");
        return Err(format!("pool compute answered {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("pool compute answer was not JSON: {e}"))
}

// =============================================================================
// hostedByHousehold — naming the household, never the machine (13b)
// =============================================================================

/// Which steward key, if any, may be looked up to NAME the hosting household.
///
/// Pure, and deliberately a conjunction of two facts that must travel together:
///
/// 1. `grant_cid` — this doorway's pool actually made a hosting promise for
///    this human. Without it there is nothing to name a household *about*, and
///    a name with no checkable promise is a portal painting reassurance.
/// 2. `provider` — the pool PEER named itself as that promise's provider
///    ([`provider_named_by_peer`]). This is the household's own word about who
///    it is, recorded at the moment it took the obligation on.
///
/// `None` on either missing link. The one thing this function must never do is
/// substitute the doorway's own identity — `doorway_id`, the gateway hostname,
/// anything the doorway knows about itself. The doorway ARRANGES hosting; a
/// household PERFORMS it, and
/// `genesis/a2o/features/auth/hosted-human/07-hosted-by-a-household.feature`
/// is explicit that they are not the same party. It must also never fall back
/// to `conductor_id` or the commitment cid: those name a machine and an
/// address, and the story asks for the name a person would use.
pub fn household_steward_key(grant_cid: Option<&str>, provider: Option<&str>) -> Option<String> {
    let _promise = grant_cid.map(str::trim).filter(|c| !c.is_empty())?;
    provider
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
}

/// The name a person READS, from the `Human` the steward key resolved to.
///
/// Pure. A blank or whitespace-only display name answers `None` so the strip
/// renders the promised-until row alone rather than an empty "Hosted by" value
/// — absent, never blank (Task 11's rendering contract).
pub fn household_display_name(human_display_name: Option<&str>) -> Option<String> {
    human_display_name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
}

/// `hostedByHousehold` on `GET /auth/account`, end to end.
///
/// Hops, in order, each one shorting to `None`:
/// grant cid + peer-named provider ([`household_steward_key`]) → the Human that
/// key belongs to (`lookup`, an `imagodei::get_human_by_agent_key` call at the
/// one live call site) → that Human's display name
/// ([`household_display_name`]).
///
/// `lookup` is injected rather than called directly so the whole chain is
/// testable without a conductor, and so this module stays free of zome plumbing.
pub async fn hosted_by_household<F, Fut>(
    grant_cid: Option<&str>,
    provider: Option<&str>,
    lookup: F,
) -> Option<String>
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let steward_key = household_steward_key(grant_cid, provider)?;
    let name = lookup(steward_key).await;
    household_display_name(name.as_deref())
}

/// [`hosted_by_household`] bound to the live surfaces, for the account route.
///
/// Lives here rather than in `auth_routes` so the whole "who is hosting this
/// person" concern stays in one module and the route reads as one line. The
/// Human read is an `imagodei::get_human_by_agent_key` call on the doorway's
/// conductor; a read that FAILS answers `None` and warns — a household the
/// doorway could not look up goes unnamed, and is never replaced by a name the
/// doorway could supply from its own configuration.
pub async fn hosted_by_household_for(
    state: &crate::server::AppState,
    user: &crate::db::UserDoc,
) -> Option<String> {
    hosted_by_household(
        user.hosted_cell_grant_cid.as_deref(),
        user.hosted_cell_provider.as_deref(),
        |steward_key| async move {
            match crate::routes::zome_helpers::call_get_human_by_agent_key(state, &steward_key)
                .await
            {
                Ok(found) => found.map(|h| h.human.display_name),
                Err(e) => {
                    warn!(
                        steward_key = %steward_key,
                        "account: could not read the hosting household's Human: {}", e
                    );
                    None
                }
            }
        },
    )
    .await
}

// =============================================================================
// humansServed — the doorway's own hosting count (D3)
// =============================================================================

/// Credential rows that carry a LIVE hosted-cell promise right now.
///
/// Active, not soft-deleted, carrying a grant cid the substrate returned, and
/// not yet past its promised-until stamp. The string comparison on
/// `hosted_cell_valid_until` is exact only because every stamp this doorway
/// writes comes from [`rfc3339_utc_secs`]; a differently-formatted stamp would
/// compare wrong rather than error, which is why that formatter is the single
/// writer and `hosted_cell_valid_until_is_lexicographically_ordered` pins it.
///
/// **Honesty caveat (chief, 2026-09-10):** this count is a doorway-local
/// PROJECTION of the notary's answer. The row carries the grant cid the
/// substrate returned and is cleared by the doorway's own close path. A
/// commitment revoked provider-side ON THE SUBSTRATE — without passing through
/// `POST /auth/close-account` — is NOT reflected here until a reconcile reads it
/// back, and that reconcile is deferred. The number is therefore "promises this
/// doorway believes it is keeping", which may exceed "promises the notary still
/// records". It is named, not hidden.
pub fn live_hosted_cell_filter(now: DateTime<Utc>) -> Document {
    doc! {
        "is_active": true,
        "metadata.is_deleted": { "$ne": true },
        "hosted_cell_grant_cid": { "$nin": [bson::Bson::Null, bson::Bson::String(String::new())] },
        "hosted_cell_valid_until": { "$gt": rfc3339_utc_secs(now) },
    }
}

/// `humansServed` on `/status.json`.
///
/// `None` only when this doorway operates no conductor pool — it hosts nobody's
/// cell, so the question does not apply to it and the landing renders `—`.
/// A doorway that DOES operate a pool and currently hosts nobody answers
/// `Some(0)`: zero is an answer, and rendering it as "unknown" was the thing
/// that made this number meaningless.
pub fn humans_served(pool_present: bool, live_count: Option<u64>) -> Option<u32> {
    if !pool_present {
        return None;
    }
    // A pool exists but the count could not be taken (no credential store, or
    // the query failed). Unknown is honestly unknown — never a fabricated 0.
    let count = live_count?;
    Some(count.min(u64::from(u32::MAX)) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn cfg() -> PoolComputeConfig {
        PoolComputeConfig {
            url: "http://pool.local:8090".into(),
            token: "t".repeat(40),
            performer: "uhCAkPoolPeer".into(),
        }
    }

    fn at(stamp: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(stamp)
            .expect("fixture stamp")
            .with_timezone(&Utc)
    }

    #[test]
    fn hosted_cell_grant_body_carries_the_d2_bounds() {
        let (body, valid_until) =
            hosted_cell_grant_body("uhCAkNewcomer", at("2026-09-11T09:00:00Z"));
        assert_eq!(body["scope"], HOSTED_CELL_SCOPE);
        assert_eq!(body["recipient"], "uhCAkNewcomer");
        assert_eq!(body["issuedAt"], "2026-09-11T09:00:00Z");
        assert_eq!(body["validFrom"], "2026-09-11T09:00:00Z");
        assert_eq!(body["validUntil"], "2026-10-11T09:00:00Z");
        assert_eq!(valid_until, "2026-10-11T09:00:00Z");
        assert_eq!(body["bounds"]["reach_ceiling"], REACH_CEILING);
        assert_eq!(body["bounds"]["rate_per_hour"], RATE_PER_HOUR);
        assert_eq!(body["bounds"]["rotation_ttl_days"], ROTATION_TTL_DAYS);
        let scope = body["bounds"]["epr_scope"].as_array().expect("epr_scope");
        assert!(
            !scope.is_empty() && scope.len() <= 64,
            "the grant validator refuses an empty or over-long epr_scope outright"
        );
        // The window must not exceed rotation_ttl_days * 86400, or the grant
        // surface refuses the whole request rather than trimming it.
        let window = at(valid_until.as_str()) - at("2026-09-11T09:00:00Z");
        assert!(window.num_seconds() <= (ROTATION_TTL_DAYS as i64) * 86_400);
    }

    #[test]
    fn hosted_register_without_pool_compute_config_still_registers() {
        // The decision the register arm actually makes: no configuration means
        // no grant leg at all, and registration is byte-identical to before.
        let mut args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        assert_eq!(
            pool_compute_config(&args),
            None,
            "an unconfigured doorway must skip the grant leg, not fail registration"
        );
        args.pool_compute_url = Some("http://pool.local:8090".into());
        assert_eq!(
            pool_compute_config(&args),
            None,
            "a partly-configured doorway must also skip: the grant surface refuses a \
             request missing the performer, so 'try anyway' would fail every registration"
        );
        args.pool_compute_token = Some("t".repeat(40));
        args.pool_compute_performer = Some("   ".into());
        assert_eq!(pool_compute_config(&args), None, "blank is not configured");
        args.pool_compute_performer = Some("uhCAkPoolPeer".into());
        assert_eq!(pool_compute_config(&args), Some(cfg()));
    }

    #[tokio::test]
    async fn hosted_register_records_the_grant_cid_when_the_pool_answers() {
        use wiremock::matchers::{body_partial_json, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(GRANTS_PATH))
            .and(header(COMPUTE_TOKEN_HEADER, "t".repeat(40).as_str()))
            .and(header(VERIFIED_PERFORMER_HEADER, "uhCAkPoolPeer"))
            .and(body_partial_json(json!({"scope": "hosted-cell"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "grantCid": "uhCEkHostedCell",
                "grantActionHash": "uhCkkAction",
                "provider": "uhCAkPoolPeer",
                "recipient": "uhCAkNewcomer",
                "state": "active"
            })))
            .mount(&server)
            .await;

        let config = PoolComputeConfig {
            url: server.uri(),
            ..cfg()
        };
        let grant = issue_hosted_cell_grant(&config, "uhCAkNewcomer", at("2026-09-11T09:00:00Z"))
            .await
            .expect("the pool answered");
        assert_eq!(grant.grant_cid, "uhCEkHostedCell");
        assert_eq!(grant.valid_until, "2026-10-11T09:00:00Z");
        assert_eq!(
            grant.provider.as_deref(),
            Some("uhCAkPoolPeer"),
            "the promise carries the key the PEER named itself by"
        );
    }

    /// The peer's own word is the only source for who is hosting.
    ///
    /// The mocked answer here carries a `provider` the doorway could not have
    /// invented, and the grant records exactly that. Nothing in the doorway's
    /// own configuration contributes to it.
    #[tokio::test]
    async fn issue_records_the_steward_key_the_peer_names_not_the_doorways() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(GRANTS_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "grantCid": "uhCEkHostedCell",
                // NOT cfg().performer — a different key, so a doorway echoing
                // its own configuration back would fail this test.
                "provider": "uhCAkJessicaHousehold",
                "state": "active"
            })))
            .mount(&server)
            .await;

        let config = PoolComputeConfig {
            url: server.uri(),
            ..cfg()
        };
        let grant = issue_hosted_cell_grant(&config, "uhCAkNewcomer", at("2026-09-11T09:00:00Z"))
            .await
            .expect("the pool answered");
        assert_eq!(grant.provider.as_deref(), Some("uhCAkJessicaHousehold"));
    }

    /// A peer that answers without naming itself still makes the promise.
    #[tokio::test]
    async fn a_grant_without_a_named_provider_is_still_a_promise() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(GRANTS_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "grantCid": "uhCEkHostedCell",
                "provider": "   ",
                "state": "active"
            })))
            .mount(&server)
            .await;

        let config = PoolComputeConfig {
            url: server.uri(),
            ..cfg()
        };
        let grant = issue_hosted_cell_grant(&config, "uhCAkNewcomer", at("2026-09-11T09:00:00Z"))
            .await
            .expect("the pool answered");
        assert_eq!(grant.grant_cid, "uhCEkHostedCell");
        assert_eq!(
            grant.provider, None,
            "a blank provider is unnamed, never an empty household name"
        );
    }

    // ── hostedByHousehold (13b) ──────────────────────────────────────────────

    #[test]
    fn naming_a_household_needs_both_the_promise_and_the_peers_own_word() {
        assert_eq!(
            household_steward_key(Some("uhCEkHostedCell"), Some("uhCAkJessica")),
            Some("uhCAkJessica".to_string())
        );
        assert_eq!(
            household_steward_key(None, Some("uhCAkJessica")),
            None,
            "no promise, no household to name"
        );
        assert_eq!(
            household_steward_key(Some("uhCEkHostedCell"), None),
            None,
            "a promise whose provider the peer never named stays unnamed"
        );
        assert_eq!(
            household_steward_key(Some("  "), Some("uhCAkJessica")),
            None
        );
        assert_eq!(
            household_steward_key(Some("uhCEkHostedCell"), Some("")),
            None
        );
    }

    #[test]
    fn a_blank_human_name_renders_absent_not_empty() {
        assert_eq!(
            household_display_name(Some("  The Ellis Household ")),
            Some("The Ellis Household".to_string())
        );
        assert_eq!(household_display_name(Some("   ")), None);
        assert_eq!(household_display_name(None), None);
    }

    /// The whole chain, with the Human read stubbed: a hosted account whose
    /// pool peer named steward X reads X's display name.
    #[tokio::test]
    async fn a_hosted_account_reads_the_stewards_display_name() {
        let seen = std::cell::RefCell::new(None);
        let name = hosted_by_household(Some("uhCEkHostedCell"), Some("uhCAkJessica"), |key| {
            *seen.borrow_mut() = Some(key);
            async { Some("The Ellis Household".to_string()) }
        })
        .await;
        assert_eq!(name.as_deref(), Some("The Ellis Household"));
        assert_eq!(
            seen.into_inner().as_deref(),
            Some("uhCAkJessica"),
            "the Human read is keyed by the steward key the PEER named"
        );
    }

    /// An unhosted account answers `null`, and never reaches the Human read.
    #[tokio::test]
    async fn an_unhosted_account_names_no_household_and_asks_nobody() {
        let asked = std::cell::Cell::new(false);
        let name = hosted_by_household(None, None, |_key| {
            asked.set(true);
            async { Some("doorway-alpha".to_string()) }
        })
        .await;
        assert_eq!(name, None);
        assert!(!asked.get(), "no promise means no lookup at all");
    }

    /// A steward key that resolves to no Human answers `null` — the strip then
    /// renders the promised-until row alone, which Task 11 already handles.
    #[tokio::test]
    async fn a_steward_key_with_no_human_answers_null() {
        let name = hosted_by_household(Some("uhCEkHostedCell"), Some("uhCAkGhost"), |_key| async {
            None
        })
        .await;
        assert_eq!(name, None);
    }

    #[tokio::test]
    async fn close_account_succeeds_when_the_revoke_call_fails() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(GRANTS_PATH))
            .respond_with(ResponseTemplate::new(503).set_body_string("local conductor unavailable"))
            .mount(&server)
            .await;

        let config = PoolComputeConfig {
            url: server.uri(),
            ..cfg()
        };
        let outcome = revoke_hosted_cell_grant(&config, "uhCEkHostedCell").await;
        assert!(
            outcome.is_err(),
            "an unreachable notary must be reported, not swallowed"
        );
        // And it is a REPORT, not a refusal: the close route treats this Err as
        // a warning line. `close_account_verdict` never sees it, so the human
        // still leaves. Pinned in auth_routes.rs's
        // close_account_revokes_the_hosted_cell_grant_and_clears_the_row.
    }

    #[tokio::test]
    async fn an_already_withdrawn_grant_is_not_an_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(GRANTS_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "grantCid": "uhCEkHostedCell",
                "provider": "uhCAkPoolPeer",
                "state": "revoked",
                "revokedAt": "2026-09-11T09:00:00Z",
                "alreadyWithdrawn": true
            })))
            .mount(&server)
            .await;

        let config = PoolComputeConfig {
            url: server.uri(),
            ..cfg()
        };
        assert_eq!(
            revoke_hosted_cell_grant(&config, "uhCEkHostedCell").await,
            Ok(true),
            "withdrawal is terminal; asking twice is success, not failure"
        );
    }

    // ── humansServed (D3) ───────────────────────────────────────────────────

    #[test]
    fn humans_served_counts_only_live_unexpired_hosted_rows() {
        let filter = live_hosted_cell_filter(at("2026-09-11T09:00:00Z"));
        assert_eq!(
            filter.get_bool("is_active").ok(),
            Some(true),
            "a closed account is not a human this doorway is hosting"
        );
        assert!(
            filter.get_document("metadata.is_deleted").is_ok(),
            "soft-deleted rows are excluded"
        );
        assert!(
            filter.get_document("hosted_cell_grant_cid").is_ok(),
            "a row with no grant cid was never promised anything — it must not be counted"
        );
        assert_eq!(
            filter
                .get_document("hosted_cell_valid_until")
                .and_then(|d| d.get_str("$gt").map(str::to_owned))
                .ok(),
            Some("2026-09-11T09:00:00Z".to_string()),
            "an expired promise is not a live one"
        );
    }

    /// The filter above compares timestamps as STRINGS. That is only correct
    /// while every stamp comes from `rfc3339_utc_secs` — a stamp with
    /// sub-second precision or a `+00:00` offset would sort wrong and silently
    /// mis-count rather than error.
    #[test]
    fn hosted_cell_valid_until_is_lexicographically_ordered() {
        let earlier = rfc3339_utc_secs(at("2026-09-11T09:00:00Z"));
        let later = rfc3339_utc_secs(at("2026-10-11T09:00:00Z"));
        let next_year = rfc3339_utc_secs(at("2027-01-01T00:00:00Z"));
        assert!(earlier < later && later < next_year);
        assert_eq!(
            earlier.len(),
            20,
            "fixed width is what makes the order total"
        );
        assert!(earlier.ends_with('Z'));
        let (_, issued) = hosted_cell_grant_body("uhCAkNewcomer", at("2026-09-11T09:00:00Z"));
        assert_eq!(issued.len(), 20, "issued stamps use the same single writer");
    }

    #[test]
    fn humans_served_is_zero_not_none_when_a_pool_exists_and_hosts_nobody() {
        assert_eq!(
            humans_served(true, Some(0)),
            Some(0),
            "a household that hosts nobody yet hosts ZERO — the landing must render 0, not —"
        );
        assert_eq!(humans_served(true, Some(3)), Some(3));
    }

    #[test]
    fn humans_served_is_none_without_a_pool() {
        assert_eq!(
            humans_served(false, Some(7)),
            None,
            "a doorway with no pool hosts nobody's cell; the question does not apply to it"
        );
        assert_eq!(
            humans_served(true, None),
            None,
            "a pool with an untakeable count is honestly unknown, never a fabricated 0"
        );
    }
}
