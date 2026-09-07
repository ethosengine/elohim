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

const CAPABILITY: &str = "sweettest-feedback";
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
    // The immutable request pins the exact grant action. That signed action
    // binds its entry hash; a mutable projection or first Create lookup must
    // neither select another grant nor erase a still-valid native activation.
    use crate::services::commitment_record::{
        verify_commitment_record, CommitmentRecordPins, MAX_COMMITMENT_RECORD_BYTES,
    };
    use holochain_types::prelude::{ActionHash, AgentPubKey, Record};
    let action_hash = ActionHash::try_from(task["grantActionHash"].as_str().unwrap_or(""))
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
            action_hash,
            entry_hash,
            author: AgentPubKey::from_raw_39(hc.agent_pub_key()),
        },
    )?
    .ok_or_else(|| StorageError::InvalidInput("signed grant unavailable".into()))?;
    let cid = authenticated.entry_hash().to_string();
    let policy: Value = serde_json::from_str(&authenticated.content().payload_json)?;
    if authenticated.content().action != "delegates-compute"
        || policy["provider"] != task["provider"]
        || policy["recipient"] != task["requester"]
        || policy["scope"] != CAPABILITY
    {
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
        scope: CAPABILITY.into(),
        provider: hc.agent_key_uhcak(),
        recipient: requester.to_owned(),
        bounds: policy["bounds"].clone(),
        valid_from: policy["valid_from"].as_str().unwrap_or("").to_owned(),
        valid_until: policy["valid_until"].as_str().unwrap_or("").to_owned(),
        revoked_at: None,
    });
    crate::services::bounds_validator::validate(
        &crate::services::bounds_validator::EventForValidation {
            action: CAPABILITY.into(),
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
        return call(&hc, &input).await.map(|v| response::ok(&v));
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
                    CAPABILITY,
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
