//! Node-local SDK facing for Rakia compute. Cross-peer discovery and evidence
//! travel through DHT records, never through doorway or another peer's HTTP API.
//! The operator-only local ingress must be enabled explicitly. This adapter
//! cannot sign as a browser/requester: every write must identify this cell.
use super::operator_verbs::VERIFIED_PERFORMER_HEADER;
use crate::{
    db::{AppContext, DbPool},
    error::StorageError,
    hc_client::HcClient,
    hc_client_registry::HcClientRegistry,
    services::response,
};
use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::{body::Incoming, Method, Request, Response};
use serde_json::{json, Value};
use std::sync::{Arc, OnceLock};

/// A node-local capability, kept out of the untrusted test process/container.
/// Hash first so comparison work has a fixed size independent of token length.
pub fn local_token_authorized(headers: &hyper::HeaderMap) -> bool {
    let Ok(expected) = std::env::var("ELOHIM_COMPUTE_LOCAL_TOKEN") else {
        return false;
    };
    let Some(provided) = headers.get("x-elohim-compute-token") else {
        return false;
    };
    token_matches(expected.as_bytes(), provided.as_bytes())
}
fn token_matches(expected: &[u8], provided: &[u8]) -> bool {
    use sha2::{Digest, Sha256};
    use subtle::ConstantTimeEq;
    expected.len() >= 32
        && provided.len() <= 512
        && bool::from(Sha256::digest(expected).ct_eq(&Sha256::digest(provided)))
}

struct VerifiedGrant(crate::services::commitment_fetcher::CommitmentRecord);
#[async_trait::async_trait]
impl crate::services::commitment_fetcher::CommitmentFetcher for VerifiedGrant {
    async fn fetch(
        &self,
        cid: &str,
    ) -> Result<
        Option<crate::services::commitment_fetcher::CommitmentRecord>,
        crate::services::commitment_fetcher::FetchError,
    > {
        Ok((cid == self.0.cid).then(|| self.0.clone()))
    }
}

/// The event class a delegated Holochain feedback sweettest spends — the
/// historical (and default) grant scope.
const FEEDBACK_SCOPE: &str = "sweettest-feedback";
/// The envelope `project` prefix that marks a peer-executed a2o stage
/// (2026-09-08 stage design: `project = "a2o-stage:<name>[@after=<cid>]"`).
const STAGE_PROJECT_PREFIX: &str = "a2o-stage:";

/// The grant scope a task must be launched under.
///
/// Pure and one-way: the requester-supplied `project` string only SELECTS which
/// storage-owned scope is required; it never becomes the scope. A stage task
/// (`project` starts `a2o-stage:`) spends `measure-stage`; everything else stays
/// the historical `sweettest-feedback` lane. The envelope is the content-addressed
/// one (`verify_status` re-derives the task CID before any caller reads it).
fn required_scope(task: &Value) -> &'static str {
    match task["envelope"]["project"].as_str() {
        Some(project) if project.starts_with(STAGE_PROJECT_PREFIX) => {
            super::compute_grants::MEASURE_STAGE_SCOPE
        }
        _ => FEEDBACK_SCOPE,
    }
}

/// Does an AUTHENTICATED grant policy name this task's parties and the scope
/// the task requires? Pure. Both parties must be present strings — two absent
/// fields never "match". One home for the provider-side launch check
/// ([`authorize`]) and the requester-side completion observation
/// ([`observe_requester_completion`]).
fn grant_policy_matches(policy: &Value, task: &Value, scope: &str) -> bool {
    task["provider"].is_string()
        && task["requester"].is_string()
        && policy["provider"] == task["provider"]
        && policy["recipient"] == task["requester"]
        && policy["scope"].as_str() == Some(scope)
}

static MUTATIONS: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn content_cid(value: &Value) -> Result<String, StorageError> {
    let ipld =
        ipld_core::serde::to_ipld(value).map_err(|e| StorageError::Serialization(e.to_string()))?;
    let bytes =
        elohim_epr::cbor::encode(&ipld).map_err(|e| StorageError::Serialization(e.to_string()))?;
    Ok(elohim_epr::cid::compute_cid(&bytes).to_string())
}
fn verify_receipt(task: &Value, receipt: &Value, cid: &str) -> Result<(), StorageError> {
    if content_cid(receipt)? != cid
        || receipt["taskCid"] != task["taskCid"]
        || receipt["requestAction"] != task["requestActionHash"]
        || receipt["grantAction"] != task["grantActionHash"]
        || receipt["requester"] != task["requester"]
        || receipt["provider"] != task["provider"]
    {
        return Err(StorageError::InvalidInput(
            "receipt CID or task bindings mismatch".into(),
        ));
    }
    if receipt["schemaVersion"] != 1
        || !matches!(
            receipt["status"].as_str(),
            Some(
                "passed"
                    | "failed"
                    | "interrupted"
                    | "timedOut"
                    | "payloadLimit"
                    | "invalidInventory"
            )
        )
    {
        return Err(StorageError::InvalidInput(
            "unsupported compute receipt".into(),
        ));
    }
    if receipt["status"] == "passed" {
        let mut expected = task["envelope"]["expectedTests"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut observed = receipt["observedTests"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        expected.sort_by_key(Value::to_string);
        observed.sort_by_key(Value::to_string);
        if expected.is_empty() || expected != observed || receipt["exitCode"] != 0 {
            return Err(StorageError::InvalidInput(
                "passing receipt requires exact nonzero test inventory".into(),
            ));
        }
    }
    for field in [
        "binary",
        "dna",
        "runtimeImage",
        "expectedTests",
        "project",
        "taskKind",
    ] {
        if receipt[field] != task["envelope"][field] {
            return Err(StorageError::InvalidInput(
                "receipt differs from immutable task".into(),
            ));
        }
    }
    let retention = task["envelope"]
        .get("retention")
        .cloned()
        .unwrap_or_else(|| json!({"maxAgeSeconds":86400,"maxRuns":5,"expireWhen":"either"}));
    if receipt["retention"] != retention {
        return Err(StorageError::InvalidInput(
            "receipt retention differs from task".into(),
        ));
    }
    Ok(())
}
fn verify_status(task: &Value) -> Result<(), StorageError> {
    if content_cid(&task["envelope"])? != task["taskCid"].as_str().unwrap_or("") {
        return Err(StorageError::InvalidInput("task CID mismatch".into()));
    }
    if !task["completion"].is_null() {
        verify_receipt(
            task,
            &task["completion"]["receipt"],
            task["completion"]["receiptCid"].as_str().unwrap_or(""),
        )?;
    }
    Ok(())
}

async fn call(hc: &HcClient, input: &Value) -> Result<Value, StorageError> {
    let payload =
        rmp_serde::to_vec_named(input).map_err(|e| StorageError::Serialization(e.to_string()))?;
    // Work is bounded in WASM: fifty requests/page, sixteen events/request.
    let output = hc
        .call_zome("content_store", "compute_task", payload)
        .await?;
    let result: Value =
        rmp_serde::from_slice(&output).map_err(|e| StorageError::Serialization(e.to_string()))?;
    if let Some(tasks) = result["tasks"].as_array() {
        for task in tasks {
            verify_status(task)?;
        }
    } else {
        verify_status(&result)?;
    }
    Ok(result)
}

fn same_actor(performer: Option<&str>, local: &str) -> bool {
    performer.is_some_and(|p| !p.is_empty() && p == local)
}

/// A `delegates-compute` grant whose signed Record was fetched by its exact
/// action hash and verified against pinned author, action and entry hashes.
struct AuthenticatedGrant {
    /// The commitment's `entry_hash` — its CID, the `bounded_by` join key.
    cid: String,
    /// The notarizing action — the projection's `dht_anchor_hash` only.
    action_hash: String,
    action: String,
    payload_json: String,
    policy: Value,
}

/// Fetch the grant the immutable request pins, and authenticate it as authored
/// by `author`. The launch check pins `author` to this cell (the provider is
/// the local agent); the requester-side observer pins it to the task's
/// provider — the same verification, a different expected signer.
async fn fetch_authenticated_grant(
    hc: &HcClient,
    grant_action_hash: &str,
    author: holochain_types::prelude::AgentPubKey,
) -> Result<AuthenticatedGrant, StorageError> {
    // The immutable request pins the exact grant action. That signed action
    // binds its entry hash; a mutable projection or first Create lookup must
    // neither select another grant nor erase a still-valid native activation.
    use crate::services::commitment_record::{
        verify_commitment_record, CommitmentRecordPins, MAX_COMMITMENT_RECORD_BYTES,
    };
    use holochain_types::prelude::{ActionHash, Record};
    let action_hash = ActionHash::try_from(grant_action_hash)
        .map_err(|_| StorageError::InvalidInput("invalid grant action reference".into()))?;
    let request = rmp_serde::to_vec_named(&action_hash)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let bytes = hc
        .call_zome_mishpat("mishpat", "get_commitment_record", request)
        .await?;
    if bytes.len() > MAX_COMMITMENT_RECORD_BYTES {
        return Err(StorageError::InvalidInput(
            "grant record exceeds limit".into(),
        ));
    }
    let record: Option<Record> =
        rmp_serde::from_slice(&bytes).map_err(|e| StorageError::Serialization(e.to_string()))?;
    let entry_hash = record
        .as_ref()
        .and_then(|r| r.action().entry_hash())
        .cloned()
        .ok_or_else(|| StorageError::InvalidInput("native grant entry unavailable".into()))?;
    let authenticated = verify_commitment_record(
        &bytes,
        CommitmentRecordPins {
            action_hash: action_hash.clone(),
            entry_hash,
            author,
        },
    )?
    .ok_or_else(|| StorageError::InvalidInput("signed grant unavailable".into()))?;
    let content = authenticated.content();
    if content.action != "delegates-compute" {
        return Err(StorageError::InvalidInput(
            "signed commitment is not a compute grant".into(),
        ));
    }
    Ok(AuthenticatedGrant {
        cid: authenticated.entry_hash().to_string(),
        action_hash: action_hash.to_string(),
        action: content.action.clone(),
        payload_json: content.payload_json.clone(),
        policy: serde_json::from_str(&content.payload_json)?,
    })
}

/// Checks the exact notarized grant and current provider-authored lifecycle.
/// The requester consumes the grant; Adam is its resource provider, not an
/// identity inferred from a hostname or transport ID.
async fn authorize(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    task: &Value,
) -> Result<String, StorageError> {
    let requester = task["requester"]
        .as_str()
        .ok_or_else(|| StorageError::InvalidInput("requester missing".into()))?;
    use holochain_types::prelude::AgentPubKey;
    let grant = fetch_authenticated_grant(
        hc,
        task["grantActionHash"].as_str().unwrap_or(""),
        AgentPubKey::from_raw_39(hc.agent_pub_key()),
    )
    .await?;
    let cid = grant.cid;
    let policy = grant.policy;
    let scope = required_scope(task);
    if !grant_policy_matches(&policy, task, scope) {
        return Err(StorageError::InvalidInput(
            "signed grant party or scope mismatch".into(),
        ));
    }
    // Re-run the shared bounds validator on the authenticated payload, rather
    // than granting authority to caller-controlled policy fields in a cache.
    use crate::services::commitment_fetcher::CommitmentRecord;
    let fetcher = VerifiedGrant(CommitmentRecord {
        cid: cid.clone(),
        action: "delegates-compute".into(),
        scope: scope.into(),
        provider: hc.agent_key_uhcak(),
        recipient: requester.to_owned(),
        bounds: policy["bounds"].clone(),
        valid_from: policy["valid_from"].as_str().unwrap_or("").to_owned(),
        valid_until: policy["valid_until"].as_str().unwrap_or("").to_owned(),
        revoked_at: None,
    });
    crate::services::bounds_validator::validate(
        &crate::services::bounds_validator::EventForValidation {
            action: scope.into(),
            performer: requester.to_owned(),
            bounded_by: cid.clone(),
            target_epr_id: task["taskCid"].as_str().unwrap_or("").to_owned(),
            reach: "commons".into(),
            signed_at: chrono::Utc::now().to_rfc3339(),
        },
        &fetcher,
        &crate::services::rate_history::DieselRateHistory { pool: pool.clone() },
    )
    .await
    .map_err(|_| StorageError::InvalidInput("signed grant bounds refused".into()))?;
    let states =
        crate::services::conductor_writes::get_commitment_authority_links(hc, &cid).await?;
    if !grant_links_allow(&states, &hc.agent_key_uhcak()) {
        return Err(StorageError::InvalidInput("compute grant withdrawn".into()));
    }
    Ok(cid)
}

fn grant_links_allow(
    states: &[crate::services::conductor_writes::CommitmentStateLink],
    provider: &str,
) -> bool {
    let authored = || states.iter().filter(|state| state.author == provider);
    authored().any(|state| state.state == "active")
        && !authored()
            .any(|state| matches!(state.state.as_str(), "revoked" | "cancelled" | "sunset"))
}

/// Should this read carry the requester-side observation? Pure.
///
/// Only the REQUESTER's node observes (post-commit signals are author-local, so
/// the provider's projection never holds the requester's view and vice versa),
/// only once the provider's signed completion exists, and never for a
/// self-grant: the provider's `DieselRateHistory` counts every `bounded_by` row
/// against the grant, so a node that is both parties must not also spend here.
fn observes_requester_completion(task: &Value, local: &str) -> bool {
    task["requester"].as_str() == Some(local)
        && !task["completion"].is_null()
        && task["provider"]
            .as_str()
            .is_some_and(|provider| !provider.is_empty() && provider != local)
}

/// A receipt instant (`startedAt` / `completedAt`, epoch seconds per
/// `compute-receipt.schema.json`). Absent, non-integer or < 1 is no instant.
fn receipt_instant(receipt: &Value, field: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    receipt[field]
        .as_i64()
        .filter(|secs| *secs >= 1)
        .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
}

/// What the provider-authored lifecycle links say about a run launched at
/// `started_at` (epoch seconds). Pure.
///
/// - an `active` link authored by the provider must exist — anyone else's
///   activation is not the provider's consent;
/// - a provider withdrawal (`revoked`/`cancelled`/`sunset`) at or before the
///   launch second refuses: the run was not authorized when it started. The
///   receipt's `startedAt` is second-truncated, so a withdrawal inside the
///   launch second is read as preceding it (fail-closed);
/// - a withdrawal strictly after launch does NOT erase an earned observation;
///   its earliest signed time is returned so the projection carries it.
///
/// Links authored by anyone other than the provider are ignored entirely.
fn observation_lifecycle(
    states: &[crate::services::conductor_writes::CommitmentStateLink],
    provider: &str,
    started_at: i64,
) -> Result<Option<String>, &'static str> {
    let authored = || states.iter().filter(|state| state.author == provider);
    if !authored().any(|state| state.state == "active") {
        return Err("no provider-authored activation");
    }
    let mut earliest: Option<(i64, &str)> = None;
    for state in authored()
        .filter(|state| matches!(state.state.as_str(), "revoked" | "cancelled" | "sunset"))
    {
        let at = chrono::DateTime::parse_from_rfc3339(&state.signed_at)
            .map_err(|_| "provider withdrawal time unreadable")?
            .timestamp();
        if at <= started_at {
            return Err("grant withdrawn before launch");
        }
        if earliest.is_none_or(|(seen, _)| at < seen) {
            earliest = Some((at, state.signed_at.as_str()));
        }
    }
    Ok(earliest.map(|(_, signed_at)| signed_at.to_owned()))
}

/// Project a VERIFIED foreign grant into the requester's storage and record the
/// requester's observation of its fulfilment, bounded by the grant's entry hash.
///
/// Idempotent: the grant upsert keys on `cid`, and the `compute-fulfilled` row is
/// written only when its id is absent. An existing row that disagrees with this
/// observation (other grant, other parties) is refused, never overwritten.
/// Withdrawal is monotonic here: a `revoked_at` already projected is never
/// cleared by a later view that lacks the link.
///
/// `h_app_id`: the row is written under `AppContext::default()` — the same
/// partition the launch-admission row uses and the economic-events route reads.
fn record_requester_observation(
    conn: &mut diesel::sqlite::SqliteConnection,
    mut grant_row: crate::db::models::NewMishpatCommitment,
    withdrawn_at: Option<String>,
    event_id: &str,
    completed_at: &str,
) -> Result<(), StorageError> {
    use crate::db::{economic_events, mishpat_commitments};
    let db = |e: diesel::result::Error| StorageError::Database(e.to_string());
    let cid = grant_row.cid.clone();
    let provider = grant_row.provider.clone();
    let receiver = grant_row.recipient.clone();
    let prior = mishpat_commitments::get_by_cid(conn, &cid).map_err(db)?;
    grant_row.state = "active".into();
    grant_row.revoked_at = withdrawn_at.or_else(|| prior.and_then(|row| row.revoked_at));
    mishpat_commitments::upsert_with_anchor(conn, grant_row).map_err(db)?;
    let ctx = AppContext::default();
    match economic_events::get_economic_event(conn, &ctx, event_id)? {
        Some(existing) => {
            if existing.action != "compute-fulfilled"
                || existing.bounded_by.as_deref() != Some(cid.as_str())
                || existing.provider != provider
                || existing.receiver != receiver
            {
                return Err(StorageError::InvalidInput(
                    "existing fulfilment record disagrees with the verified grant".into(),
                ));
            }
        }
        None => {
            economic_events::record_compute_fulfilled_event(
                conn,
                &ctx,
                event_id,
                &provider,
                &receiver,
                &cid,
                completed_at,
            )?;
        }
    }
    Ok(())
}

/// The requester-side producer of `compute-fulfilled` (the dormant observer in
/// `services::rea_observed_compute` lights from this).
///
/// Runs on the requester's single-task read once the provider's completion is
/// verified by [`call`] (task CID, receipt CID and six envelope pins). It then
/// (1) authenticates the grant the request pins with the author pinned to the
/// task's PROVIDER, (2) requires the policy to name these parties and the scope
/// the task requires, (3) reads the provider-authored lifecycle, (4) projects
/// the foreign grant exactly as the provider's own issue does, and (5) records
/// `compute-fulfilled:<request>` bounded by the grant entry hash at the
/// receipt's `completedAt` — deterministic, never `now()`.
///
/// NEVER fails the read: any refusal is reported as `observed.refused` and
/// writes nothing it has not verified.
async fn observe_requester_completion(hc: &Arc<HcClient>, pool: &DbPool, task: &Value) -> Value {
    match observe(hc, pool, task).await {
        Ok(observed) => observed,
        Err(reason) => {
            tracing::warn!(%reason, "requester compute observation refused");
            json!({"verified": false, "refused": reason})
        }
    }
}

async fn observe(hc: &Arc<HcClient>, pool: &DbPool, task: &Value) -> Result<Value, String> {
    use holochain_types::prelude::AgentPubKey;
    let provider = task["provider"].as_str().ok_or("provider missing")?;
    let reference = task["requestActionHash"]
        .as_str()
        .filter(|r| !r.is_empty())
        .ok_or("request reference missing")?;
    let author = AgentPubKey::try_from(provider).map_err(|_| "provider is not an agent key")?;
    let grant =
        fetch_authenticated_grant(hc, task["grantActionHash"].as_str().unwrap_or(""), author)
            .await
            .map_err(|e| e.to_string())?;
    let scope = required_scope(task);
    if !grant_policy_matches(&grant.policy, task, scope) {
        return Err("signed grant party or scope mismatch".into());
    }
    let receipt = &task["completion"]["receipt"];
    let started = receipt_instant(receipt, "startedAt").ok_or("receipt startedAt missing")?;
    let completed = receipt_instant(receipt, "completedAt").ok_or("receipt completedAt missing")?;
    let states = crate::services::conductor_writes::get_commitment_authority_links(hc, &grant.cid)
        .await
        .map_err(|e| e.to_string())?;
    let withdrawn_at = observation_lifecycle(&states, provider, started.timestamp())?;
    let row = match crate::mishpat_projection::parse_commitment_payload(
        &grant.action,
        &grant.payload_json,
        &grant.cid,
        &grant.action_hash,
    )? {
        crate::mishpat_projection::CommitmentProjection::Upsert(row) => row,
        _ => return Err("grant projection mismatch".into()),
    };
    let event_id = format!("compute-fulfilled:{reference}");
    {
        // bounded-work: the same single local-mutation lane as admission, so the
        // absent-then-insert below cannot race another write on this node.
        let _guard = MUTATIONS
            .get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await;
        let mut conn = pool.get().map_err(|e| e.to_string())?;
        record_requester_observation(
            &mut conn,
            row,
            withdrawn_at,
            &event_id,
            &completed.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(json!({
        "verified": true,
        "grantCid": grant.cid,
        "scope": scope,
        "grantProvider": grant.policy["provider"],
        "grantRecipient": grant.policy["recipient"],
        "fulfilledEventId": event_id,
    }))
}

fn launchable(task: &Value, attempt: &Value) -> bool {
    !task["acceptance"]["attemptId"].is_null()
        && task["acceptance"]["attemptId"] == *attempt
        && task["completion"].is_null()
        && task["refusal"].is_null()
}

pub async fn handle(
    req: Request<Incoming>,
    pool: &DbPool,
    registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if std::env::var("ELOHIM_COMPUTE_LOCAL_API").as_deref() != Ok("1") {
        return Ok(response::service_unavailable("local compute API disabled"));
    }
    if !local_token_authorized(req.headers()) {
        return Ok(response::forbidden(
            &json!({"reason":"local-compute-capability-required"}),
        ));
    }
    let Some(hc) = registry.and_then(|r| r.lamad_client()) else {
        return Ok(response::service_unavailable("local conductor unavailable"));
    };
    let grant_request = req.uri().path() == "/api/v1/compute/grants";
    let method = req.method().clone();
    let path = req
        .uri()
        .path()
        .trim_start_matches("/api/v1/compute/tasks")
        .trim_matches('/')
        .to_owned();
    let query = req.uri().query().unwrap_or("").to_owned();
    let performer = req
        .headers()
        .get(VERIFIED_PERFORMER_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    if !same_actor(performer.as_deref(), &hc.agent_key_uhcak()) {
        return Ok(response::forbidden(
            &json!({"reason":"local-cell-actor-required"}),
        ));
    }
    if grant_request && method != Method::POST {
        return Ok(response::method_not_allowed());
    }
    if method == Method::GET {
        let input = if path.is_empty() {
            let params: std::collections::HashMap<String, String> =
                url::form_urlencoded::parse(query.as_bytes())
                    .into_owned()
                    .collect();
            json!({"operation":"list","provider":params.get("provider"),
                "offset":params.get("offset").and_then(|n|n.parse::<u64>().ok()).unwrap_or(0),
                "limit":params.get("limit").and_then(|n|n.parse::<u64>().ok()).unwrap_or(20)})
        } else if !path.contains('/') {
            json!({"operation":"get","requestActionHash":path})
        } else {
            return Ok(response::not_found("unknown compute read"));
        };
        let mut result = call(&hc, &input).await?;
        if !path.is_empty() && observes_requester_completion(&result, &hc.agent_key_uhcak()) {
            result["observed"] = observe_requester_completion(&hc, pool, &result).await;
        }
        return Ok(response::ok(&result));
    }
    if method != Method::POST {
        return Ok(response::method_not_allowed());
    }
    let bytes = Limited::new(req.into_body(), 131072)
        .collect()
        .await
        .map_err(|_| {
            StorageError::InvalidInput("compute request exceeds 128 KiB or is incomplete".into())
        })?
        .to_bytes();
    let mut input: Value = serde_json::from_slice(&bytes)?;
    if !input.is_object() {
        return Ok(response::bad_request("expected compute object"));
    }
    // bounded-work: one local mutation at a time, serializing admission/rate
    // accounting with native authoring. Durable attempt identity lives in DHT.
    let _guard = MUTATIONS
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await;
    if grant_request {
        // Withdrawal is a BODY VARIANT of the same route (`{grantCid, revoke:true}`),
        // never a second route family — the dev-seed lever already reads this shape.
        if input["revoke"] == json!(true) {
            return super::compute_grants::revoke(&hc, pool, &input)
                .await
                .map(|v| response::ok(&v));
        }
        return super::compute_grants::issue(&hc, pool, &input)
            .await
            .map(|v| response::ok(&v));
    }
    if path.is_empty() {
        if content_cid(&input["envelope"])? != input["taskCid"].as_str().unwrap_or("") {
            return Ok(response::bad_request("task CID mismatch"));
        }
        input["operation"] = json!("submit");
    } else {
        let Some((reference, operation)) = path.split_once('/') else {
            return Ok(response::not_found("unknown compute write"));
        };
        if !matches!(
            operation,
            "accept" | "complete" | "decline" | "authorize-launch"
        ) {
            return Ok(response::not_found("unknown compute operation"));
        }
        let task = call(
            &hc,
            &json!({"operation":"get","requestActionHash":reference}),
        )
        .await?;
        if task["provider"] != hc.agent_key_uhcak() {
            return Ok(response::forbidden(
                &json!({"reason":"selected-runner-required"}),
            ));
        }
        if operation == "complete" {
            verify_receipt(
                &task,
                &input["receipt"],
                input["receiptCid"].as_str().unwrap_or(""),
            )?;
        }
        if matches!(operation, "accept" | "authorize-launch") {
            let cid = match authorize(&hc, pool, &task).await {
                Ok(cid) => cid,
                Err(e) => {
                    tracing::warn!(error=%e,"compute grant refused");
                    return Ok(response::forbidden(
                        &json!({"reason":"compute-grant-refused"}),
                    ));
                }
            };
            if operation == "authorize-launch" {
                if !launchable(&task, &input["attemptId"]) {
                    return Ok(response::conflict(
                        "launch requires accepted, unfinished attempt",
                    ));
                }
                let mut conn = pool
                    .get()
                    .map_err(|e| StorageError::Database(e.to_string()))?;
                // One-use launch admission: a retry after a lost response is
                // indeterminate, never permission to execute a second time.
                crate::db::economic_events::record_operator_verb_event(
                    &mut conn,
                    &AppContext::default(),
                    &format!("compute-admission:{reference}"),
                    required_scope(&task),
                    task["requester"].as_str().unwrap_or(""),
                    &hc.agent_key_uhcak(),
                    &cid,
                    &chrono::Utc::now().to_rfc3339(),
                )?;
                return Ok(response::ok(
                    &json!({"authorized":true,"grantActionHash":task["grantActionHash"],"requestActionHash":reference,"attemptId":input["attemptId"]}),
                ));
            }
        }
        input["operation"] = json!(operation);
        input["requestActionHash"] = json!(reference);
    }
    call(&hc, &input).await.map(|v| response::ok(&v))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepted_attempt_cannot_launch_after_either_terminal_outcome() {
        let mut task = json!({"acceptance":{"attemptId":"attempt"}});
        assert!(launchable(&task, &json!("attempt")));
        assert!(!launchable(&task, &json!("different")));
        for terminal in ["refusal", "completion"] {
            task[terminal] = json!({"actionHash":"signed-terminal-action"});
            assert!(!launchable(&task, &json!("attempt")), "{terminal}");
            task[terminal] = Value::Null;
        }
        task["acceptance"] = Value::Null;
        assert!(!launchable(&task, &Value::Null));
    }
    #[test]
    fn grant_lifecycle_uses_signed_provider_links_and_fails_closed() {
        use crate::services::conductor_writes::CommitmentStateLink;
        let link = |author: &str, state: &str| CommitmentStateLink {
            author: author.into(),
            state: state.into(),
            signed_at: String::new(),
            event_hash: String::new(),
        };
        assert!(!grant_links_allow(&[link("", "active")], "adam"));
        assert!(!grant_links_allow(&[link("mallory", "active")], "adam"));
        assert!(grant_links_allow(
            &[link("adam", "active"), link("mallory", "revoked")],
            "adam"
        ));
        for withdrawal in ["revoked", "cancelled", "sunset"] {
            assert!(!grant_links_allow(
                &[link("adam", "active"), link("adam", withdrawal)],
                "adam"
            ));
        }
    }
    #[test]
    fn stage_tasks_require_the_measure_scope_and_feedback_tasks_the_feedback_scope() {
        let task = |project: Value| {
            json!({"requester":"matthew","provider":"adam",
                "envelope":{"project":project,"taskKind":"feedback_signal"}})
        };
        let grant = |scope: &str| json!({"provider":"adam","recipient":"matthew","scope":scope,"bounds":{}});
        let admits =
            |task: &Value, policy: &Value| grant_policy_matches(policy, task, required_scope(task));
        let stage = task(json!("a2o-stage:federation-convergence"));
        let feedback = task(json!("elohim-feedback-sweettest"));
        // 1. a stage task under a measure-stage grant is admitted
        assert_eq!(required_scope(&stage), "measure-stage");
        assert!(admits(&stage, &grant("measure-stage")));
        // 2. a stage task under a feedback grant is refused (compute-grant-refused)
        assert!(!admits(&stage, &grant("sweettest-feedback")));
        // 3. a feedback task under a feedback grant is admitted, as before
        assert_eq!(required_scope(&feedback), "sweettest-feedback");
        assert!(admits(&feedback, &grant("sweettest-feedback")));
        // 4. a feedback task under a measure-stage grant is refused: a measure
        //    grant does not widen into the sweettest lane
        assert!(!admits(&feedback, &grant("measure-stage")));
        // 5. only the exact prefix selects the measure scope; lookalikes, a
        //    missing or non-string project stay feedback, and a hosted-cell grant
        //    admits neither class
        for lookalike in [
            json!("a2o-stagex:federation"),
            json!("A2O-STAGE:federation"),
            json!(" a2o-stage:federation"),
            json!("measure-stage"),
            json!(["a2o-stage:federation"]),
            Value::Null,
        ] {
            let t = task(lookalike.clone());
            assert_eq!(required_scope(&t), "sweettest-feedback", "{lookalike}");
            assert!(!admits(&t, &grant("measure-stage")), "{lookalike}");
        }
        for t in [&stage, &feedback] {
            assert!(!admits(t, &grant("hosted-cell")));
        }
        // parties still bind: the right scope with the wrong provider or recipient,
        // or absent parties on both sides, is never a match
        let mut wrong = grant("measure-stage");
        wrong["provider"] = json!("mallory");
        assert!(!admits(&stage, &wrong));
        let mut wrong = grant("measure-stage");
        wrong["recipient"] = json!("mallory");
        assert!(!admits(&stage, &wrong));
        let partyless = json!({"envelope":{"project":"a2o-stage:x"}});
        assert!(!grant_policy_matches(
            &json!({"scope":"measure-stage"}),
            &partyless,
            required_scope(&partyless)
        ));
    }
    fn state_link(
        author: &str,
        state: &str,
        signed_at: &str,
    ) -> crate::services::conductor_writes::CommitmentStateLink {
        crate::services::conductor_writes::CommitmentStateLink {
            author: author.into(),
            state: state.into(),
            signed_at: signed_at.into(),
            event_hash: String::new(),
        }
    }
    // 2026-09-25T10:00:00Z
    const LAUNCH: i64 = 1_790_330_400;
    #[test]
    fn requester_observation_requires_provider_authored_activation() {
        assert_eq!(
            observation_lifecycle(&[], "adam", LAUNCH),
            Err("no provider-authored activation")
        );
        // an activation authored by anyone else is not the provider's consent
        assert!(observation_lifecycle(
            &[state_link("mallory", "active", "2026-09-25T09:00:00Z")],
            "adam",
            LAUNCH
        )
        .is_err());
        assert_eq!(
            observation_lifecycle(
                &[
                    state_link("adam", "active", "2026-09-25T09:00:00Z"),
                    // a bystander's "withdrawal" before launch is not the provider's
                    state_link("mallory", "revoked", "2026-09-25T09:30:00Z"),
                ],
                "adam",
                LAUNCH
            ),
            Ok(None)
        );
    }
    #[test]
    fn observation_refuses_launch_after_withdrawal() {
        let active = state_link("adam", "active", "2026-09-25T09:00:00Z");
        for withdrawal in ["revoked", "cancelled", "sunset"] {
            assert_eq!(
                observation_lifecycle(
                    &[
                        active.clone(),
                        state_link("adam", withdrawal, "2026-09-25T09:59:00Z")
                    ],
                    "adam",
                    LAUNCH
                ),
                Err("grant withdrawn before launch"),
                "{withdrawal}"
            );
        }
        // startedAt is second-truncated: a withdrawal inside the launch second is
        // read as preceding the launch (fail-closed)
        assert!(observation_lifecycle(
            &[
                active.clone(),
                state_link("adam", "revoked", "2026-09-25T10:00:00.400Z")
            ],
            "adam",
            LAUNCH
        )
        .is_err());
        // an unreadable withdrawal time is never read as "after"
        assert_eq!(
            observation_lifecycle(
                &[active.clone(), state_link("adam", "revoked", "yesterday")],
                "adam",
                LAUNCH
            ),
            Err("provider withdrawal time unreadable")
        );
        // a withdrawal after launch does not erase the earned observation; the
        // earliest withdrawal is carried for the projection
        assert_eq!(
            observation_lifecycle(
                &[
                    active,
                    state_link("adam", "sunset", "2026-09-25T12:00:00Z"),
                    state_link("adam", "revoked", "2026-09-25T11:00:00Z"),
                ],
                "adam",
                LAUNCH
            ),
            Ok(Some("2026-09-25T11:00:00Z".into()))
        );
    }
    #[test]
    fn self_grant_is_never_observed() {
        let done = json!({"actionHash":"completion"});
        let task = |requester: &str, provider: &str, completion: &Value| json!({"requester":requester,"provider":provider,"completion":completion});
        assert!(observes_requester_completion(
            &task("matthew", "adam", &done),
            "matthew"
        ));
        // a node that is both parties never records a fulfilment against itself
        assert!(!observes_requester_completion(
            &task("matthew", "matthew", &done),
            "matthew"
        ));
        // only the requester observes; the provider's node never does
        assert!(!observes_requester_completion(
            &task("matthew", "adam", &done),
            "adam"
        ));
        // nothing to observe before the provider's signed completion
        assert!(!observes_requester_completion(
            &task("matthew", "adam", &Value::Null),
            "matthew"
        ));
        assert!(!observes_requester_completion(
            &json!({"requester":"matthew","completion":done}),
            "matthew"
        ));
    }
    #[test]
    fn receipt_instants_are_epoch_seconds_and_absent_is_refused() {
        let receipt = json!({"startedAt":LAUNCH,"completedAt":"1790330400","zero":0});
        assert_eq!(
            receipt_instant(&receipt, "startedAt")
                .unwrap()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "2026-09-25T10:00:00Z"
        );
        assert!(receipt_instant(&receipt, "completedAt").is_none());
        assert!(receipt_instant(&receipt, "zero").is_none());
        assert!(receipt_instant(&receipt, "absent").is_none());
    }
    mod observation_projection {
        use super::super::record_requester_observation;
        use crate::db::{economic_events, mishpat_commitments, AppContext};
        use crate::mishpat_projection::{parse_commitment_payload, CommitmentProjection};
        use crate::services::rea_observed_compute::{fulfilled_cids_from_events, retain_fulfilled};
        use diesel::prelude::*;
        use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
        use elohim_facings::folds::rea::CommitmentRow;
        use serde_json::json;

        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
        const ENTRY_HASH: &str = "uhCEk-grant-entry";
        const ACTION_HASH: &str = "uhCkk-grant-action";

        fn test_conn() -> SqliteConnection {
            let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");
            conn.run_pending_migrations(MIGRATIONS).expect("migrations");
            conn
        }
        fn foreign_grant() -> crate::db::models::NewMishpatCommitment {
            let payload = json!({"action":"delegates-compute","scope":"measure-stage",
                "provider":"uhCAk-jessica","recipient":"uhCAk-matthew",
                "bounds":{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":2,"rotation_ttl_days":5},
                "valid_from":"2026-09-25T00:00:00+00:00","valid_until":"2026-09-30T00:00:00+00:00"});
            match parse_commitment_payload(
                "delegates-compute",
                &payload.to_string(),
                ENTRY_HASH,
                ACTION_HASH,
            )
            .expect("parses")
            {
                CommitmentProjection::Upsert(row) => row,
                _ => panic!("delegates-compute projects as an upsert"),
            }
        }

        #[test]
        fn compute_fulfilled_observation_is_idempotent_and_bounded_by_grant_entry_hash() {
            let mut conn = test_conn();
            let id = "compute-fulfilled:uhCkk-request";
            for _ in 0..2 {
                record_requester_observation(
                    &mut conn,
                    foreign_grant(),
                    None,
                    id,
                    "2026-09-25T10:05:00Z",
                )
                .expect("observation records");
            }
            let ctx = AppContext::default();
            let events = economic_events::list_compute_fulfilled_events(&mut conn, &ctx)
                .expect("list fulfilled");
            assert_eq!(events.len(), 1, "a re-read records nothing new");
            let event = &events[0];
            assert_eq!(event.id, id);
            assert_eq!(event.provider, "uhCAk-jessica");
            assert_eq!(event.receiver, "uhCAk-matthew");
            assert_eq!(event.has_point_in_time, "2026-09-25T10:05:00Z");
            assert_eq!(
                event.bounded_by.as_deref(),
                Some(ENTRY_HASH),
                "bounded_by is the grant entry hash, never its action hash"
            );

            // the foreign grant is projected active, anchored on its action hash
            let row = mishpat_commitments::get_by_cid(&mut conn, ENTRY_HASH)
                .expect("read")
                .expect("projected");
            assert_eq!(row.state, "active");
            assert_eq!(row.scope, "measure-stage");
            assert_eq!(row.dht_anchor_hash.as_deref(), Some(ACTION_HASH));
            assert!(row.revoked_at.is_none());

            // and the observed side keeps exactly that row
            let rows = vec![CommitmentRow {
                cid: row.cid.clone(),
                action: row.action.clone(),
                scope: row.scope.clone(),
                provider: row.provider.clone(),
                recipient: row.recipient.clone(),
                resource_classified_as: vec![],
                household_id: None,
                valid_from: row.valid_from.clone(),
                valid_until: row.valid_until.clone(),
                bounds_json: row.bounds_json.clone(),
                state: row.state.clone(),
            }];
            let kept = retain_fulfilled(&rows, &fulfilled_cids_from_events(&events));
            assert_eq!(
                kept.len(),
                1,
                "retain_fulfilled keeps the projected grant row"
            );
            assert_eq!(kept[0].cid, ENTRY_HASH);

            // a withdrawal seen after completion is carried, and a later view
            // lacking it never clears it; the observation stays
            record_requester_observation(
                &mut conn,
                foreign_grant(),
                Some("2026-09-25T11:00:00Z".into()),
                id,
                "2026-09-25T10:05:00Z",
            )
            .expect("re-observe after withdrawal");
            record_requester_observation(
                &mut conn,
                foreign_grant(),
                None,
                id,
                "2026-09-25T10:05:00Z",
            )
            .expect("re-observe without the link");
            let row = mishpat_commitments::get_by_cid(&mut conn, ENTRY_HASH)
                .unwrap()
                .unwrap();
            assert_eq!(row.revoked_at.as_deref(), Some("2026-09-25T11:00:00Z"));
            assert_eq!(
                economic_events::list_compute_fulfilled_events(&mut conn, &ctx)
                    .unwrap()
                    .len(),
                1
            );

            // an existing row for the same id that names another grant is refused,
            // never overwritten
            let mut other = foreign_grant();
            other.cid = "uhCEk-other-grant".into();
            assert!(record_requester_observation(
                &mut conn,
                other,
                None,
                id,
                "2026-09-25T10:05:00Z"
            )
            .is_err());
        }
    }
    #[test]
    fn receipt_binding_rejects_wrong_actor_artifact_and_empty_pass() {
        let envelope = json!({"expectedTests":["feedback_signal_one"],"binary":{"cid":"baf-example"},"retention":{"maxAgeSeconds":432000,"maxRuns":5,"expireWhen":"either"}});
        let task = json!({"taskCid":content_cid(&envelope).unwrap(),"envelope":envelope,"requestActionHash":"request","grantActionHash":"grant","requester":"alice","provider":"adam"});
        let receipt = json!({"schemaVersion":1,"taskCid":task["taskCid"],"requestAction":"request","grantAction":"grant","requester":"alice","provider":"adam","expectedTests":["feedback_signal_one"],"observedTests":["feedback_signal_one"],"binary":task["envelope"]["binary"],"retention":task["envelope"]["retention"],"status":"passed","exitCode":0});
        assert!(verify_receipt(&task, &receipt, &content_cid(&receipt).unwrap()).is_ok());
        for (field, value) in [
            ("provider", json!("mallory")),
            ("binary", json!({"cid":"different"})),
            ("observedTests", json!([])),
            ("exitCode", json!(1)),
        ] {
            let mut tampered = receipt.clone();
            tampered[field] = value;
            assert!(
                verify_receipt(&task, &tampered, &content_cid(&tampered).unwrap()).is_err(),
                "{field}"
            );
        }
        assert!(verify_receipt(&task, &receipt, "wrong-cid").is_err());
    }
    #[test]
    fn local_capability_refuses_missing_short_and_different_tokens() {
        assert!(!token_matches(b"", b""));
        assert!(!token_matches(b"short", b"short"));
        assert!(!token_matches(&[1; 32], &[2; 32]));
        assert!(token_matches(&[1; 32], &[1; 32]));
    }
    #[test]
    fn writes_never_impersonate_requester_or_trust_a_transport_id() {
        assert!(!same_actor(None, "uhCAk-local"));
        assert!(!same_actor(Some("12D3KooTransport"), "uhCAk-local"));
        assert!(!same_actor(Some("uhCAk-other"), "uhCAk-local"));
        assert!(same_actor(Some("uhCAk-local"), "uhCAk-local"));
    }
}
