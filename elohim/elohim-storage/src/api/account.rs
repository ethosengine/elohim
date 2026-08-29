//! Account API controller — M5 Recovery Protocol Phase 2
//!
//! Routes: `/api/v1/account`, `/api/v1/account/keys`, `/api/v1/account/revocations`,
//!         `/api/v1/account/self-revocation`, `/api/v1/account/pending-recovery`,
//!         `/api/v1/account/recovery/:id/vote`, `/api/v1/account/portal-hosts`
//!
//! ## Auth pattern
//! GET routes resolve the calling agent from the REQUEST ALONE via
//! [`resolve_account_caller`] (`X-Agent-Id`, then `X-Agent-Cid`) and answer 401
//! when neither header is present — there is no ambient-session fallback on a
//! read (see [`extract_agent_key_explicit`]). The zome-write routes still use
//! [`extract_agent_key`]'s cascade (`X-Agent-Id` with active local session as
//! Tauri fallback), the same pattern as `api/identity.rs`.
//!
//! ## Truth layer
//! GET handlers read from SQLite projections (Category A — rebuildable from DHT signals).
//! POST/DELETE handlers that write to the DHT (self-revocation, recovery vote, portal-host
//! CRUD) require the conductor bridge, which is deferred to Phase 11 (threaded HcClient).
//! Until then those routes return 503 with a machine-readable `notImplemented` code so the
//! Angular layer can surface a graceful message instead of an unhandled error.
//!
//! ## Provenance assumption (Phase 11)
//! `HcClient::call_zome` signs the call with admin-issued credentials, but
//! the conductor presents the call to the zome as the cell's owner agent.
//! Empirically verified by the heartbeat path: `record_peer_status` reads
//! `agent_info()?.agent_initial_pubkey` and the resulting peer statuses are
//! correctly attributed to peers (not to storage's signer). The mode gate
//! `verify_caller_owns_cell` exploits this: if the connected cell's owner
//! matches the resolved human key, the zome will see the human as caller.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::humans::{get_human_by_agent_key, get_human_by_id};
use crate::db::recovery_requests::list_recovery_requests_for_agent;
use crate::db::DbPool;
use crate::error::StorageError;
use crate::services::response;
use crate::views::{
    AccountView, HumanRelationshipView, HumanView, KeyRevocationView, KeyRotationView,
    PortalHostView, RecoveryRequestView,
};

use super::get_conn;

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Dispatch `/api/v1/account/*` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    hc_registry: Option<&Arc<crate::hc_client_registry::HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    match (method, resource_path) {
        // ── Aggregate ──────────────────────────────────────────────────────
        (Method::GET, "" | "/") => get_account(req, pool).await,

        // ── Key history ───────────────────────────────────────────────────
        (Method::GET, "/keys") => get_account_keys(req, pool).await,

        // ── Revocation history ────────────────────────────────────────────
        (Method::GET, "/revocations") => get_account_revocations(req, pool).await,

        // ── Self-revocation (zome write — Phase 11 bridge) ────────────────
        (Method::POST, "/self-revocation") => handle_self_revocation(req, hc_registry, pool).await,

        // ── Pending recovery requests (where I am EC) ─────────────────────
        (Method::GET, "/pending-recovery") => get_pending_recovery(req, pool).await,

        // ── Recovery vote (zome write — Phase 11 bridge) ──────────────────
        (Method::POST, p) if p.starts_with("/recovery/") && p.ends_with("/vote") => {
            let revocation_id = p.trim_start_matches("/recovery/").trim_end_matches("/vote");
            if revocation_id.is_empty() {
                return Ok(response::bad_request("missing revocation id in URL path"));
            }
            handle_revocation_vote(req, hc_registry, pool, revocation_id.to_string()).await
        }

        // ── Portal hosts ──────────────────────────────────────────────────
        (Method::GET, "/portal-hosts") => get_portal_hosts(req, pool).await,
        (Method::POST, "/portal-hosts") => handle_add_portal_host(req, hc_registry, pool).await,
        (Method::DELETE, p) if p.starts_with("/portal-hosts/") => {
            let url_b64 = p.trim_start_matches("/portal-hosts/");
            if url_b64.is_empty() {
                return Ok(response::bad_request("missing url_b64 in URL path"));
            }
            handle_remove_portal_host(req, hc_registry, pool, url_b64.to_string()).await
        }

        _ => Ok(response::not_found(&format!(
            "Unknown account route: /api/v1/account{}",
            resource_path
        ))),
    }
}

// ---------------------------------------------------------------------------
// GET /api/v1/account — aggregate AccountView
// ---------------------------------------------------------------------------

/// GET /api/v1/account
///
/// Aggregates identity, key history, revocation events, pending recovery
/// requests where the caller is an emergency contact, portal hosts, and
/// conductor availability into a single `AccountView`.
///
/// All sub-queries read from SQLite projections (DHT-derived, Category A).
async fn get_account(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    // 1. Resolve calling agent
    let agent_key = match resolve_account_caller(&req, &mut conn)? {
        Some(k) => k,
        None => return Ok(unauthorized_no_caller()),
    };

    // 2. Resolve Human from agent key
    let human = match get_human_by_agent_key(&mut conn, &agent_key)? {
        Some(h) => h,
        None => {
            return Ok(response::not_found(
                "No human record found for current agent",
            ))
        }
    };
    let human_id = human.id.clone();
    let human_view = HumanView::from(human);

    // 3. Active key rotation: latest rotation for this human's agent pubkey
    let active_key_rotation = {
        use crate::db::diesel_schema::key_rotations::dsl;
        use diesel::prelude::*;
        dsl::key_rotations
            .filter(dsl::human_agent_pubkey.eq(&agent_key))
            .order(dsl::rotated_at.desc())
            .first::<crate::db::models::KeyRotationRow>(&mut conn)
            .optional()
            .map_err(|e| StorageError::Internal(format!("key_rotations query failed: {e}")))?
            .map(|r| KeyRotationView {
                dht_anchor_hash: r.dht_anchor_hash,
                human_agent_pubkey: r.human_agent_pubkey,
                new_agent_pubkey: r.new_agent_pubkey,
                superseded_agent_pubkey: r.superseded_agent_pubkey,
                recovery_request_hash: r.recovery_request_hash,
                authority_kind: r.authority_kind,
                authority_json: r.authority_json,
                rotated_at: r.rotated_at,
            })
    };

    // 4. Recent revocations: last 10 for this human
    let recent_revocations = {
        use crate::db::diesel_schema::key_revocations::dsl;
        use diesel::prelude::*;
        dsl::key_revocations
            .filter(dsl::subject_human_id.eq(&human_id))
            .order(dsl::created_at.desc())
            .limit(10)
            .select(crate::db::models::KeyRevocationRow::as_select())
            .load(&mut conn)
            .map_err(|e| StorageError::Internal(format!("key_revocations query failed: {e}")))?
            .into_iter()
            .map(KeyRevocationView::from)
            .collect::<Vec<_>>()
    };

    // 5. Pending recovery requests where I am an emergency contact.
    //    Strategy: find humans who have my human_id as an EC, then look up
    //    their pending recovery requests.
    let pending_recovery_requests = {
        // Find all relationships where this human is party_b and emergency_access_enabled
        use crate::db::diesel_schema::human_relationships::dsl as hr_dsl;
        use diesel::prelude::*;
        let ec_of: Vec<String> = hr_dsl::human_relationships
            .filter(hr_dsl::h_app_id.eq("imagodei"))
            .filter(hr_dsl::party_b_id.eq(&human_id))
            .filter(hr_dsl::emergency_access_enabled.eq(1))
            .select(hr_dsl::party_a_id)
            .load::<String>(&mut conn)
            .map_err(|e| StorageError::Internal(format!("emergency contact query failed: {e}")))?;

        if ec_of.is_empty() {
            Vec::new()
        } else {
            // Look up agent_pub_keys for those humans, then find their recovery requests
            let mut requests = Vec::new();
            for other_human_id in &ec_of {
                if let Some(other_human) = get_human_by_id(&mut conn, other_human_id)? {
                    // agent_pub_key is Option<String>; skip humans without one
                    if let Some(ref pubkey) = other_human.agent_pub_key {
                        let rows = list_recovery_requests_for_agent(&mut conn, pubkey)?;
                        for r in rows {
                            requests.push(RecoveryRequestView {
                                dht_anchor_hash: r.dht_anchor_hash,
                                human_agent_pubkey: r.human_agent_pubkey,
                                new_agent_pubkey: r.new_agent_pubkey,
                                hosting_doorway_pubkey: r.hosting_doorway_pubkey,
                                proposed_authority_kind: r.proposed_authority_kind,
                                proposed_authority_json: r.proposed_authority_json,
                                request_nonce: r.request_nonce,
                                human_id: r.human_id,
                                required_witness_count: r.required_witness_count as u32,
                                created_at: r.created_at,
                            });
                        }
                    }
                }
            }
            requests
        }
    };

    // 6. Emergency contacts: relationships where this human is party_a and emergency_access_enabled
    let emergency_contacts = {
        use crate::db::diesel_schema::human_relationships::dsl as hr_dsl;
        use diesel::prelude::*;
        hr_dsl::human_relationships
            .filter(hr_dsl::h_app_id.eq("imagodei"))
            .filter(hr_dsl::party_a_id.eq(&human_id))
            .filter(hr_dsl::emergency_access_enabled.eq(1))
            .load::<crate::db::models::HumanRelationship>(&mut conn)
            .map_err(|e| StorageError::Internal(format!("emergency contacts query failed: {e}")))?
            .into_iter()
            .map(HumanRelationshipView::from)
            .collect::<Vec<_>>()
    };

    // 7. Portal hosts
    let portal_hosts = {
        crate::db::portal_hosts::list_for_human(&mut conn, &human_id)?
            .into_iter()
            .map(|r| PortalHostView {
                human_id: r.human_id,
                host_url: r.host_url,
                label: r.label,
                added_at: r.added_at,
                last_reachable_at: r.last_reachable_at,
                reach: r.reach,
                dht_anchor_hash: r.dht_anchor_hash,
            })
            .collect::<Vec<_>>()
    };

    // 8. is_steward: human has a node_stewardship record on the local conductor
    let is_steward = {
        use crate::db::diesel_schema::node_stewardship::dsl as ns_dsl;
        use diesel::prelude::*;
        let count: i64 = ns_dsl::node_stewardship
            .filter(ns_dsl::human_id.eq(&human_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);
        count > 0
    };

    // 9. has_local_conductor — true when the pool is backed by a local conductor
    //    (always true in elohim-storage direct mode; doorway-only nodes have no pool)
    let has_local_conductor = true;

    let view = AccountView {
        human: human_view,
        active_key_rotation,
        recent_revocations,
        pending_recovery_requests,
        emergency_contacts,
        portal_hosts,
        is_steward,
        has_local_conductor,
    };

    Ok(response::ok(&view))
}

// ---------------------------------------------------------------------------
// GET /api/v1/account/keys
// ---------------------------------------------------------------------------

/// GET /api/v1/account/keys
///
/// Returns the full key rotation history for the authenticated agent, ordered
/// most-recent first. Each row carries `dhtAnchorHash` for provenance.
async fn get_account_keys(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    let agent_key = match resolve_account_caller(&req, &mut conn)? {
        Some(k) => k,
        None => return Ok(unauthorized_no_caller()),
    };

    use crate::db::diesel_schema::key_rotations::dsl;
    use diesel::prelude::*;
    let rows = dsl::key_rotations
        .filter(dsl::human_agent_pubkey.eq(&agent_key))
        .order(dsl::rotated_at.desc())
        .load::<crate::db::models::KeyRotationRow>(&mut conn)
        .map_err(|e| StorageError::Internal(format!("key_rotations query failed: {e}")))?;

    let views: Vec<KeyRotationView> = rows
        .into_iter()
        .map(|r| KeyRotationView {
            dht_anchor_hash: r.dht_anchor_hash,
            human_agent_pubkey: r.human_agent_pubkey,
            new_agent_pubkey: r.new_agent_pubkey,
            superseded_agent_pubkey: r.superseded_agent_pubkey,
            recovery_request_hash: r.recovery_request_hash,
            authority_kind: r.authority_kind,
            authority_json: r.authority_json,
            rotated_at: r.rotated_at,
        })
        .collect();

    Ok(response::ok(&views))
}

// ---------------------------------------------------------------------------
// GET /api/v1/account/revocations
// ---------------------------------------------------------------------------

/// GET /api/v1/account/revocations
///
/// Returns up to 50 key revocation events for the authenticated human,
/// ordered most-recent first. Includes both pending and effective revocations.
async fn get_account_revocations(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    let agent_key = match resolve_account_caller(&req, &mut conn)? {
        Some(k) => k,
        None => return Ok(unauthorized_no_caller()),
    };

    // Resolve human_id from agent_key
    let human = match get_human_by_agent_key(&mut conn, &agent_key)? {
        Some(h) => h,
        None => {
            return Ok(response::not_found(
                "No human record found for current agent",
            ))
        }
    };

    use crate::db::diesel_schema::key_revocations::dsl;
    use diesel::prelude::*;
    let rows = dsl::key_revocations
        .filter(dsl::subject_human_id.eq(&human.id))
        .order(dsl::created_at.desc())
        .limit(50)
        .select(crate::db::models::KeyRevocationRow::as_select())
        .load(&mut conn)
        .map_err(|e| StorageError::Internal(format!("key_revocations query failed: {e}")))?;

    let views: Vec<KeyRevocationView> = rows.into_iter().map(KeyRevocationView::from).collect();
    Ok(response::ok(&views))
}

// ---------------------------------------------------------------------------
// GET /api/v1/account/pending-recovery
// ---------------------------------------------------------------------------

/// GET /api/v1/account/pending-recovery
///
/// Returns recovery requests from humans for whom the authenticated agent is
/// listed as an emergency contact (emergency_access_enabled = true in
/// human_relationships). Uses the projection table — no DHT traversal.
async fn get_pending_recovery(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    let agent_key = match resolve_account_caller(&req, &mut conn)? {
        Some(k) => k,
        None => return Ok(unauthorized_no_caller()),
    };

    let human = match get_human_by_agent_key(&mut conn, &agent_key)? {
        Some(h) => h,
        None => {
            return Ok(response::not_found(
                "No human record found for current agent",
            ))
        }
    };

    // Find all humans who list this human as an EC (party_b is the EC)
    use crate::db::diesel_schema::human_relationships::dsl as hr_dsl;
    use diesel::prelude::*;
    let principal_human_ids: Vec<String> = hr_dsl::human_relationships
        .filter(hr_dsl::h_app_id.eq("imagodei"))
        .filter(hr_dsl::party_b_id.eq(&human.id))
        .filter(hr_dsl::emergency_access_enabled.eq(1))
        .select(hr_dsl::party_a_id)
        .load::<String>(&mut conn)
        .map_err(|e| StorageError::Internal(format!("emergency contact lookup failed: {e}")))?;

    let mut views = Vec::new();
    for other_id in &principal_human_ids {
        if let Some(other_human) = get_human_by_id(&mut conn, other_id)? {
            // agent_pub_key is Option<String>; skip humans not yet keyed
            if let Some(ref pubkey) = other_human.agent_pub_key {
                let rows = list_recovery_requests_for_agent(&mut conn, pubkey)?;
                for r in rows {
                    views.push(RecoveryRequestView {
                        dht_anchor_hash: r.dht_anchor_hash,
                        human_agent_pubkey: r.human_agent_pubkey,
                        new_agent_pubkey: r.new_agent_pubkey,
                        hosting_doorway_pubkey: r.hosting_doorway_pubkey,
                        proposed_authority_kind: r.proposed_authority_kind,
                        proposed_authority_json: r.proposed_authority_json,
                        request_nonce: r.request_nonce,
                        human_id: r.human_id,
                        required_witness_count: r.required_witness_count as u32,
                        created_at: r.created_at,
                    });
                }
            }
        }
    }

    Ok(response::ok(&views))
}

// ---------------------------------------------------------------------------
// GET /api/v1/account/portal-hosts
// ---------------------------------------------------------------------------

/// GET /api/v1/account/portal-hosts
///
/// Returns all portal hosts registered for the authenticated human, ordered
/// most-recently-added first. Projection from the `portal_hosts` table.
async fn get_portal_hosts(
    req: Request<Incoming>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    let agent_key = match resolve_account_caller(&req, &mut conn)? {
        Some(k) => k,
        None => return Ok(unauthorized_no_caller()),
    };

    let human = match get_human_by_agent_key(&mut conn, &agent_key)? {
        Some(h) => h,
        None => {
            return Ok(response::not_found(
                "No human record found for current agent",
            ))
        }
    };

    let rows = crate::db::portal_hosts::list_for_human(&mut conn, &human.id)?;
    let views: Vec<PortalHostView> = rows
        .into_iter()
        .map(|r| PortalHostView {
            human_id: r.human_id,
            host_url: r.host_url,
            label: r.label,
            added_at: r.added_at,
            last_reachable_at: r.last_reachable_at,
            reach: r.reach,
            dht_anchor_hash: r.dht_anchor_hash,
        })
        .collect();

    Ok(response::ok(&views))
}

// ---------------------------------------------------------------------------
// Phase 11: mode gate + 503 contracts
// ---------------------------------------------------------------------------

/// Asserts the caller's resolved agent key matches the connected cell's
/// owner. The Tauri-direct invariant — when matched, the imagodei zome
/// will see the human as caller (cell owner == caller per the provenance
/// note in this module's doc).
///
/// `Err(Response<...>)` is returned with `503 BROWSER_WRITE_PATH_PENDING`
/// when the keys do not match — the browser-via-doorway path that lands
/// in M6 once the hosting trust model is settled.
///
/// The `Response<Full<Bytes>>` in the `Err` variant is intentionally large —
/// it is the early-exit response that route handlers return directly to the
/// caller, avoiding an unwrap chain. Boxing would break the uniform response
/// pattern used by every handler in this module.
#[allow(clippy::result_large_err)]
fn verify_caller_owns_cell(
    owner: &dyn crate::hc_client::CellOwner,
    agent_key: &str,
) -> Result<(), Response<Full<Bytes>>> {
    if owner.agent_key_hex() != agent_key {
        return Err(response_503_browser_write_path_pending());
    }
    Ok(())
}

/// Pure body builder for the BROWSER_WRITE_PATH_PENDING 503 contract.
/// Extracted so unit tests can assert against the JSON Value directly
/// without setting up async machinery to read a hyper Body.
fn browser_write_path_pending_body() -> serde_json::Value {
    serde_json::json!({
        "error": "browser write path not yet implemented",
        "code": "BROWSER_WRITE_PATH_PENDING",
        "message": "Self-sovereign writes require a peer the human controls. \
                    The browser-via-doorway write path is deferred to M6 where \
                    the hosting trust model is settled."
    })
}

/// 503 response for the browser-via-doorway write path (M6+).
fn response_503_browser_write_path_pending() -> Response<Full<Bytes>> {
    response::json_response(
        hyper::StatusCode::SERVICE_UNAVAILABLE,
        &browser_write_path_pending_body(),
    )
}

/// Pure body builder for the IMAGODEI_BRIDGE_OFFLINE 503 contract.
fn imagodei_bridge_offline_body() -> serde_json::Value {
    serde_json::json!({
        "error": "imagodei bridge offline",
        "code": "IMAGODEI_BRIDGE_OFFLINE",
        "message": "The storage process did not connect to the imagodei \
                    coordinator zome at startup. Account write routes are \
                    unavailable until storage restarts with the imagodei DNA \
                    installed."
    })
}

/// 503 response when the imagodei HcClient failed to connect at startup.
/// Recovery: restart storage with the imagodei DNA installed.
fn response_503_imagodei_bridge_offline() -> Response<Full<Bytes>> {
    response::json_response(
        hyper::StatusCode::SERVICE_UNAVAILABLE,
        &imagodei_bridge_offline_body(),
    )
}

/// Maps a `StorageError` from a zome call to an HTTP response.
///
/// PHASE-11-DEBT: this string-matches well-known zome error prefixes.
/// Brittle by design — matches what `imagodei` returns today. Typed
/// errors over the conductor wire are an M6+ refactor.
fn map_zome_err_to_http(err: &StorageError) -> Response<Full<Bytes>> {
    let msg = err.to_string();

    // 403 — gate rejections (defender, EC, ownership)
    if msg.contains("not a configured defender")
        || msg.contains("not an active emergency contact")
        || msg.contains("does not control")
        || msg.contains("does not belong to")
    {
        let body = serde_json::json!({
            "error": "forbidden",
            "code": "ZOME_GATE_REJECTED",
            "message": msg,
        });
        return response::json_response(hyper::StatusCode::FORBIDDEN, &body);
    }

    // 400 — input validation
    if msg.contains("invalid reason")
        || msg.contains("already effective")
        || msg.contains("votes not accepted")
        || msg.contains("attestation cannot be empty")
        || msg.contains("no KeyRevocation with id")
    {
        return response::bad_request(&msg);
    }

    // 503 — conductor connectivity
    if matches!(err, StorageError::Connection(_)) {
        return response::service_unavailable(&msg);
    }

    response::internal_error(&msg)
}

// ---------------------------------------------------------------------------
// Phase 11: generic zome-call forwarder
// ---------------------------------------------------------------------------

/// Forward a zome call to the imagodei coordinator and return the decoded
/// output. MessagePack-encodes `input`, calls `hc.call_zome("imagodei",
/// fn_name, payload)`, and MessagePack-decodes the response into `O`.
///
/// Errors are returned as `StorageError`; route handlers map them to HTTP
/// via `map_zome_err_to_http`.
async fn forward_to_imagodei<I, O>(
    hc: &crate::hc_client::HcClient,
    fn_name: &str,
    input: &I,
) -> Result<O, StorageError>
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned,
{
    let payload = rmp_serde::to_vec_named(input)
        .map_err(|e| StorageError::Conductor(format!("encode {fn_name} input: {e}")))?;
    let bytes = hc.call_zome("imagodei", fn_name, payload).await?;
    let output: O = rmp_serde::from_slice(&bytes)
        .map_err(|e| StorageError::Conductor(format!("decode {fn_name} output: {e}")))?;
    Ok(output)
}

// ---------------------------------------------------------------------------
// Phase 11: zome-input wrappers
// ---------------------------------------------------------------------------
//
// Match the imagodei coordinator zome's input/output structs exactly. We
// keep them in `account.rs` rather than `views.rs` because they are wire-
// internal — they do NOT cross the HTTP boundary.

#[derive(serde::Serialize)]
struct CreateSelfRevocationZomeInput {
    revoked_key: holochain_types::prelude::AgentPubKey,
    reason: String,
}

#[derive(serde::Deserialize)]
struct CreateSelfRevocationZomeOutput {
    revocation_id: String,
    action_hash: holochain_types::prelude::ActionHash,
}

#[derive(serde::Serialize)]
struct SubmitRevocationVoteZomeInput {
    revocation_id: String,
    approved: bool,
    attestation: String,
}

#[derive(serde::Deserialize)]
struct SubmitRevocationVoteZomeOutput {
    vote_id: String,
    current_votes: u32,
    required_votes: u32,
    threshold_now_reached: bool,
}

#[derive(serde::Serialize)]
struct AddPortalHostZomeInput {
    host_url: String,
    label: Option<String>,
    /// One of "Public", "Trusted", "Private" — the zome enum's serde
    /// representation. Defaults to "Trusted" when None at the InputView.
    reach: Option<String>,
}

// ---------------------------------------------------------------------------
// Phase 11: request body reader
// ---------------------------------------------------------------------------

/// Read the entire request body into a `Bytes`. Hyper streams bodies; we
/// collect to a `Bytes` for serde decode. Used by the four Phase 11
/// forwarder helpers.
async fn read_request_body(req: Request<Incoming>) -> Result<Bytes, StorageError> {
    use http_body_util::BodyExt;
    let body = req.into_body();
    let collected = body
        .collect()
        .await
        .map_err(|e| StorageError::InvalidInput(format!("read request body: {e}")))?;
    Ok(collected.to_bytes())
}

// ---------------------------------------------------------------------------
// Phase 11: handle_self_revocation
// ---------------------------------------------------------------------------

async fn handle_self_revocation(
    req: Request<Incoming>,
    hc_registry: Option<&std::sync::Arc<crate::hc_client_registry::HcClientRegistry>>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let agent_key = match extract_agent_key(&req, &mut conn)? {
        Some(k) => k,
        None => {
            return Ok(response::bad_request(
                "missing X-Agent-Id and no active session",
            ));
        }
    };

    let registry = match hc_registry {
        Some(r) => r,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };
    let hc = match registry.imagodei_client() {
        Some(h) => h,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };

    if let Err(resp) = verify_caller_owns_cell(hc.as_ref(), &agent_key) {
        return Ok(resp);
    }

    let body_bytes = read_request_body(req).await?;
    let input_view: crate::views::CreateSelfRevocationInputView =
        serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::InvalidInput(format!("invalid request body: {e}")))?;

    let revoked_key =
        holochain_types::prelude::AgentPubKey::try_from(input_view.revoked_key.as_str())
            .map_err(|e| StorageError::InvalidInput(format!("invalid revokedKey: {e}")))?;

    let zome_input = CreateSelfRevocationZomeInput {
        revoked_key,
        reason: input_view.reason,
    };

    match forward_to_imagodei::<_, CreateSelfRevocationZomeOutput>(
        &hc,
        "create_self_revocation",
        &zome_input,
    )
    .await
    {
        Ok(out) => {
            let view = crate::views::CreateSelfRevocationOutputView {
                revocation_id: out.revocation_id,
                action_hash: out.action_hash.to_string(),
            };
            Ok(response::created(&view))
        }
        Err(e) => Ok(map_zome_err_to_http(&e)),
    }
}

// ---------------------------------------------------------------------------
// Phase 11: handle_revocation_vote
// ---------------------------------------------------------------------------

async fn handle_revocation_vote(
    req: Request<Incoming>,
    hc_registry: Option<&std::sync::Arc<crate::hc_client_registry::HcClientRegistry>>,
    pool: &DbPool,
    revocation_id: String,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let agent_key = match extract_agent_key(&req, &mut conn)? {
        Some(k) => k,
        None => {
            return Ok(response::bad_request(
                "missing X-Agent-Id and no active session",
            ));
        }
    };

    let registry = match hc_registry {
        Some(r) => r,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };
    let hc = match registry.imagodei_client() {
        Some(h) => h,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };

    if let Err(resp) = verify_caller_owns_cell(hc.as_ref(), &agent_key) {
        return Ok(resp);
    }

    let body_bytes = read_request_body(req).await?;
    let input_view: crate::views::SubmitRevocationVoteInputView =
        serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::InvalidInput(format!("invalid request body: {e}")))?;

    let zome_input = SubmitRevocationVoteZomeInput {
        revocation_id,
        approved: input_view.approved,
        attestation: input_view.attestation,
    };

    match forward_to_imagodei::<_, SubmitRevocationVoteZomeOutput>(
        &hc,
        "submit_revocation_vote",
        &zome_input,
    )
    .await
    {
        Ok(out) => {
            let view = crate::views::SubmitRevocationVoteOutputView {
                vote_id: out.vote_id,
                current_votes: out.current_votes,
                required_votes: out.required_votes,
                threshold_now_reached: out.threshold_now_reached,
            };
            Ok(response::ok(&view))
        }
        Err(e) => Ok(map_zome_err_to_http(&e)),
    }
}

// ---------------------------------------------------------------------------
// Phase 11: handle_add_portal_host
// ---------------------------------------------------------------------------

async fn handle_add_portal_host(
    req: Request<Incoming>,
    hc_registry: Option<&std::sync::Arc<crate::hc_client_registry::HcClientRegistry>>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let agent_key = match extract_agent_key(&req, &mut conn)? {
        Some(k) => k,
        None => {
            return Ok(response::bad_request(
                "missing X-Agent-Id and no active session",
            ));
        }
    };

    let registry = match hc_registry {
        Some(r) => r,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };
    let hc = match registry.imagodei_client() {
        Some(h) => h,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };

    if let Err(resp) = verify_caller_owns_cell(hc.as_ref(), &agent_key) {
        return Ok(resp);
    }

    let body_bytes = read_request_body(req).await?;
    let input_view: crate::views::AddPortalHostInputView = serde_json::from_slice(&body_bytes)
        .map_err(|e| StorageError::InvalidInput(format!("invalid request body: {e}")))?;

    let zome_input = AddPortalHostZomeInput {
        host_url: input_view.host_url,
        label: input_view.label,
        reach: input_view.reach,
    };

    match forward_to_imagodei::<_, holochain_types::prelude::ActionHash>(
        &hc,
        "add_portal_host",
        &zome_input,
    )
    .await
    {
        Ok(action_hash) => {
            let view = crate::views::AddPortalHostOutputView {
                action_hash: action_hash.to_string(),
            };
            Ok(response::created(&view))
        }
        Err(e) => Ok(map_zome_err_to_http(&e)),
    }
}

// ---------------------------------------------------------------------------
// Phase 11: handle_remove_portal_host
// ---------------------------------------------------------------------------

async fn handle_remove_portal_host(
    req: Request<Incoming>,
    hc_registry: Option<&std::sync::Arc<crate::hc_client_registry::HcClientRegistry>>,
    pool: &DbPool,
    url_b64: String,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let agent_key = match extract_agent_key(&req, &mut conn)? {
        Some(k) => k,
        None => {
            return Ok(response::bad_request(
                "missing X-Agent-Id and no active session",
            ));
        }
    };

    let registry = match hc_registry {
        Some(r) => r,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };
    let hc = match registry.imagodei_client() {
        Some(h) => h,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };

    if let Err(resp) = verify_caller_owns_cell(hc.as_ref(), &agent_key) {
        return Ok(resp);
    }

    // The zome's `remove_portal_host` takes the URL as a plain `String`,
    // so we URL-safe base64 decode the path segment back to the URL.
    use base64::Engine;
    let host_url_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(url_b64.as_bytes())
        .map_err(|e| StorageError::InvalidInput(format!("invalid url_b64: {e}")))?;
    let host_url = String::from_utf8(host_url_bytes)
        .map_err(|e| StorageError::InvalidInput(format!("url_b64 is not valid UTF-8: {e}")))?;

    // Zome returns `()` on success.
    match forward_to_imagodei::<_, ()>(&hc, "remove_portal_host", &host_url).await {
        Ok(()) => {
            let view = crate::views::RemovePortalHostOutputView { deleted: true };
            Ok(response::ok(&view))
        }
        Err(e) => Ok(map_zome_err_to_http(&e)),
    }
}

// ---------------------------------------------------------------------------
// Auth helper
// ---------------------------------------------------------------------------

/// Resolve the current agent public key from the request.
///
/// Resolution order:
/// 1. `X-Agent-Id` header — set by doorway's bespoke portal-host handlers in
///    `auth_routes.rs` after JWT validation (NOT by generic middleware)
/// 2. Active local session's `agent_pub_key` — Tauri direct-connection fallback
///
/// Returns `Ok(Some(key))` when resolved, `Ok(None)` when no identity signal
/// is present (caller decides whether to 400 or 404).
fn extract_agent_key(
    req: &Request<Incoming>,
    conn: &mut diesel::SqliteConnection,
) -> Result<Option<String>, StorageError> {
    if let Some(key) = req
        .headers()
        .get("X-Agent-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        return Ok(Some(key));
    }

    if let Some(session) = crate::db::local_sessions::get_active_session(conn)? {
        return Ok(Some(session.agent_pub_key));
    }

    Ok(None)
}

/// Resolve the calling agent's `agent_cid` for view services and reach gates.
///
/// **Source of truth:** the imagodei DHT (Agent EPR entry).
///
/// **Alpha-substrate equivalence:** `agent_cid` is currently sourced from
/// `human_id` (the slug). The seeder authors `AgentPeerBinding` entries with
/// `agent_cid: human.id` (`genesis/seeder/src/seed-agent-bindings.ts:311`)
/// and view-service tests use slug-style strings like `"agent-matthew"`
/// (`tests/distribution_view.rs:177`). The "CIDv1 base32 of the agent's EPR
/// atom" comment in `views.rs` is forward-looking; CIDv1 dag-cbor sha256
/// derivation of the Agent entry is not enforced anywhere in the running
/// stack today.
///
/// **Resolution cascade** (mirrors `extract_agent_key`):
/// 1. `X-Agent-Cid` header — doorway's `forward_to_storage` injects this from
///    the JWT's `claims.human_id` after bearer validation
/// 2. Active `local_sessions.human_id` — Tauri direct-connection fallback
/// 3. `None` — Session Visitor or unauthenticated request; the caller decides
///    whether to 401 or fall through to a visitor-shaped response
///
/// **Future migration (out of Phase 5 scope):** when CIDv1 enforcement lands,
/// doorway will resolve `human_id → agent_cid` once at user creation, persist
/// on `UserDoc.agent_cid`, and source the header from there. `local_sessions`
/// gains a parallel `agent_cid` column. This helper's signature does not
/// change. The P2P design gate fires again at that point to add the cache
/// fields and audit the SQL JOIN behavior.
pub(crate) fn extract_agent_cid(
    req: &Request<Incoming>,
    conn: &mut diesel::SqliteConnection,
) -> Result<Option<String>, StorageError> {
    if let Some(cid) = req
        .headers()
        .get("X-Agent-Cid")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        return Ok(Some(cid));
    }

    if let Some(session) = crate::db::local_sessions::get_active_session(conn)? {
        return Ok(Some(session.human_id));
    }

    Ok(None)
}

/// Resolve the requester from the EXPLICIT `X-Agent-Cid` header ONLY — step 1
/// of `extract_agent_cid`'s cascade, without step 2's ambient session fallback.
///
/// **Use this for every REACH authorization decision.** `extract_agent_cid`
/// falls back to the active `local_sessions` row, which is a single-tenant
/// desktop convenience and NOT a statement about who sent this request. On a
/// hosted pod `genesis_self_heal` mints an active session for the node's own
/// human (`services::genesis_self_heal`, "session arm"), and the doorway omits
/// `X-Agent-Cid` when it has no verified caller (`storage_proxy`'s
/// `forward_to_storage_omits_x_agent_cid_when_absent`). Composed, those two
/// facts mean an ANONYMOUS request would resolve as the node's own human and
/// be served that human's authorized reach — the same class of exposure the
/// header-presence bug produced, reached through the identity door instead.
///
/// A caller that asserts no identity gets no identity: deny-by-default.
pub(crate) fn extract_agent_cid_explicit<B>(req: &Request<B>) -> Option<String> {
    req.headers()
        .get("X-Agent-Cid")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Read the caller's agent public key from the EXPLICIT `X-Agent-Id` header
/// ONLY — step 1 of [`extract_agent_key`]'s cascade, without step 2's ambient
/// session fallback.
///
/// `X-Agent-Id` is set by doorway's bespoke portal-host handlers after JWT
/// validation (`doorway/doorway-service/src/routes/auth_routes.rs`), never by
/// generic middleware.
pub(crate) fn extract_agent_key_explicit<B>(req: &Request<B>) -> Option<String> {
    req.headers()
        .get("X-Agent-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Resolve the agent public key of whoever sent THIS request — the read-path
/// resolver for every `/api/v1/account` GET.
///
/// Resolution order:
/// 1. `X-Agent-Id` — already an agent pubkey (bespoke portal-host handlers)
/// 2. `X-Agent-Cid` — injected by the general `/api/v1` proxy from the JWT's
///    `claims.human_id`, so it carries EITHER a `uhCA…` agent key (Tauri-direct
///    callers set it themselves) OR a human slug. A slug is resolved through
///    `humans.agent_pub_key` before it is returned: agent key and slug are
///    distinct identity namespaces and a raw cross-namespace value silently
///    empties every downstream join (crate CLAUDE.md, "Identity &
///    Transport-Identity Coherence"). Same discrimination as the head-declare
///    gate in `http.rs`.
/// 3. `None` — the caller asserted no identity; the handler answers 401.
///
/// **There is deliberately no `local_sessions` fallback.** [`extract_agent_key`]
/// has one, and on a hosted pod `services::genesis_self_heal` mints an ACTIVE
/// session for the node's OWN human at boot, while the doorway forwards neither
/// header for an unverified caller. Composed, those two facts mean an ANONYMOUS
/// `GET /api/v1/account` resolved as the node's own human and was served that
/// human's account — the identity-door twin of the reach exposure cured in
/// [`extract_agent_cid_explicit`]. A caller that asserts no identity gets no
/// identity: deny-by-default.
pub(crate) fn resolve_account_caller<B>(
    req: &Request<B>,
    conn: &mut diesel::SqliteConnection,
) -> Result<Option<String>, StorageError> {
    if let Some(key) = extract_agent_key_explicit(req) {
        return Ok(Some(key));
    }

    match extract_agent_cid_explicit(req) {
        Some(cid) if cid.starts_with("uhCA") => Ok(Some(cid)),
        Some(slug) => Ok(get_human_by_id(conn, &slug)?.and_then(|h| h.agent_pub_key)),
        None => Ok(None),
    }
}

/// 401 for a read whose caller cannot be resolved from the request itself.
///
/// This was a 400 while the ambient session made "no identity" unreachable;
/// with the fallback gone the condition is missing authentication, not a
/// malformed request.
fn unauthorized_no_caller() -> Response<Full<Bytes>> {
    response::json_response(
        hyper::StatusCode::UNAUTHORIZED,
        &serde_json::json!({
            "error": "authentication required: no X-Agent-Id or X-Agent-Cid on the request"
        }),
    )
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use crate::hc_client::CellOwner;

    struct StubOwner(&'static str);
    impl CellOwner for StubOwner {
        fn agent_key_hex(&self) -> String {
            self.0.to_string()
        }
    }

    /// When the caller's resolved agent key matches the connected cell's
    /// owner (Tauri-direct invariant), the gate returns Ok.
    #[test]
    fn verify_caller_owns_cell_passes_when_keys_match() {
        let owner = StubOwner("uhCAkMATCH");
        let result = verify_caller_owns_cell(&owner, "uhCAkMATCH");
        assert!(result.is_ok(), "expected Ok when keys match");
    }

    /// On mismatch, the gate returns Err. Verify by inspecting the body
    /// helper directly (avoids async test machinery for body extraction).
    #[test]
    fn verify_caller_owns_cell_returns_browser_pending_on_mismatch() {
        let owner = StubOwner("uhCAkOWNER");
        let result = verify_caller_owns_cell(&owner, "uhCAkCALLER");
        let resp = result.expect_err("expected Err with 503 response");
        assert_eq!(resp.status(), hyper::StatusCode::SERVICE_UNAVAILABLE);

        // Body contents are asserted via the body-builder pure function.
        let body = browser_write_path_pending_body();
        assert_eq!(body["code"], "BROWSER_WRITE_PATH_PENDING");
    }

    /// The reach gates resolve identity with `extract_agent_cid_explicit`,
    /// which reads the doorway-injected header and NOTHING else.
    ///
    /// Regression, 2026-08-20: the gates first shipped using the cascade form,
    /// whose step 2 falls back to the active `local_sessions` row. A hosted pod
    /// always has one (genesis self-heal mints it for the node's own human) and
    /// the doorway sends no `X-Agent-Cid` for an unverified caller — so an
    /// anonymous listing request resolved as the node's human and was served
    /// that human's reach. A caller asserting no identity must resolve to
    /// nobody; the ambient session belongs to the node, not to this request.
    #[test]
    fn explicit_agent_cid_ignores_everything_but_the_header() {
        let with_header = Request::builder()
            .uri("/db/content")
            .header("X-Agent-Cid", "matthew")
            .body(())
            .unwrap();
        assert_eq!(
            extract_agent_cid_explicit(&with_header).as_deref(),
            Some("matthew")
        );

        let anonymous = Request::builder().uri("/db/content").body(()).unwrap();
        assert_eq!(
            extract_agent_cid_explicit(&anonymous),
            None,
            "anonymous request must resolve to no identity even where an active \
             local session exists"
        );

        // A bearer token is not an identity: it was the original bypass.
        let bearer_only = Request::builder()
            .uri("/db/content")
            .header("Authorization", "Bearer bogus")
            .body(())
            .unwrap();
        assert_eq!(
            extract_agent_cid_explicit(&bearer_only),
            None,
            "an unvalidated bearer header must not resolve an identity"
        );
    }

    /// A `/api/v1/account` READ resolves its caller from the request alone.
    ///
    /// Regression companion to `explicit_agent_cid_ignores_everything_but_the_header`,
    /// reached through the account door instead of the reach door: the GET
    /// handlers used `extract_agent_key`, whose step 2 falls back to the active
    /// `local_sessions` row. A hosted pod always has one — `genesis_self_heal`
    /// mints it for the node's OWN human — and the doorway forwards no identity
    /// header for an unverified caller, so an ANONYMOUS `GET /api/v1/account`
    /// resolved as the node's human and was served that human's account.
    ///
    /// The fixture is that exact production shape: a node human with an active
    /// ambient session, plus an unrelated visitor.
    #[test]
    fn account_caller_resolves_from_the_request_not_the_ambient_session() {
        use crate::db::humans::{create_human, CreateHumanInput};
        use crate::db::run_migrations;
        use diesel::r2d2::{ConnectionManager, Pool};
        use diesel::sqlite::SqliteConnection;

        fn insert_human(conn: &mut SqliteConnection, id: &str, key: Option<&str>) {
            create_human(
                conn,
                CreateHumanInput {
                    id: id.to_string(),
                    agent_pub_key: key.map(|k| k.to_string()),
                    display_name: id.to_string(),
                    bio: None,
                    affinities: "[]".to_string(),
                    profile_reach: "commons".to_string(),
                    location: None,
                    profile_photo_url: None,
                    h_app_id: "imagodei".to_string(),
                    household_id: None,
                },
            )
            .expect("insert human");
        }

        // Shared-cache in-memory pool with the real migrations — gives us both
        // `humans` and `local_sessions`.
        let url = format!(
            "file:account_caller_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool: crate::db::DbPool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        let mut conn = pool.get().expect("conn");

        insert_human(&mut conn, "human-node", None);
        insert_human(&mut conn, "human-visitor", Some("uhCAkVISITOR"));

        // The ambient session the hosted pod mints for its OWN human at boot.
        crate::services::genesis_self_heal::genesis_self_heal_identity(
            &mut conn,
            "human-node",
            "uhCAkNODE",
            None,
        )
        .expect("self-heal");
        assert_eq!(
            crate::db::local_sessions::get_active_session(&mut conn)
                .unwrap()
                .expect("ambient session present")
                .agent_pub_key,
            "uhCAkNODE",
            "fixture must reproduce the hosted-pod ambient session"
        );

        // (a) A doorway-forwarded slug resolves to THAT human's agent key —
        // never to the node's.
        let visitor = Request::builder()
            .uri("/api/v1/account")
            .header("X-Agent-Cid", "human-visitor")
            .body(())
            .unwrap();
        assert_eq!(
            resolve_account_caller(&visitor, &mut conn)
                .unwrap()
                .as_deref(),
            Some("uhCAkVISITOR")
        );

        // (b) The anonymous case: no identity asserted, no identity resolved.
        let anonymous = Request::builder().uri("/api/v1/account").body(()).unwrap();
        assert_eq!(
            resolve_account_caller(&anonymous, &mut conn).unwrap(),
            None,
            "an anonymous read must not resolve the node's own human through \
             the ambient local session"
        );

        // (c) A bearer token is not an identity — storage never validates it.
        let bearer_only = Request::builder()
            .uri("/api/v1/account")
            .header("Authorization", "Bearer bogus")
            .body(())
            .unwrap();
        assert_eq!(
            resolve_account_caller(&bearer_only, &mut conn).unwrap(),
            None,
            "an unvalidated bearer header must not resolve an identity"
        );

        // (d) `X-Agent-Id` still resolves — doorway's bespoke portal-host
        // handlers inject it after JWT validation (`auth_routes.rs`).
        let portal_host = Request::builder()
            .uri("/api/v1/account/portal-hosts")
            .header("X-Agent-Id", "uhCAkVISITOR")
            .body(())
            .unwrap();
        assert_eq!(
            resolve_account_caller(&portal_host, &mut conn)
                .unwrap()
                .as_deref(),
            Some("uhCAkVISITOR")
        );
    }

    /// IMAGODEI_BRIDGE_OFFLINE response has the correct status and code.
    #[test]
    fn imagodei_bridge_offline_response_has_correct_code() {
        let resp = response_503_imagodei_bridge_offline();
        assert_eq!(resp.status(), hyper::StatusCode::SERVICE_UNAVAILABLE);

        let body = imagodei_bridge_offline_body();
        assert_eq!(body["code"], "IMAGODEI_BRIDGE_OFFLINE");
    }

    /// PortalHostView must serialise with camelCase keys so the Angular SDK
    /// receives clean field names without any TypeScript-side transformation.
    #[test]
    fn portal_host_view_serialises_camel_case() {
        let view = PortalHostView {
            human_id: "uhCEkTest".into(),
            host_url: "https://doorway.example.com".into(),
            label: Some("Home".into()),
            added_at: "2026-04-25T00:00:00Z".into(),
            last_reachable_at: None,
            reach: "trusted".into(),
            dht_anchor_hash: "uhCAkAnchor".into(),
        };
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["humanId"], "uhCEkTest");
        assert_eq!(json["hostUrl"], "https://doorway.example.com");
        assert_eq!(json["dhtAnchorHash"], "uhCAkAnchor");
        assert!(json["lastReachableAt"].is_null());
    }

    /// KeyRevocationView must coerce the SQLite INTEGER `threshold_reached`
    /// column (0/1) to a proper JSON boolean.
    #[test]
    fn revocation_view_threshold_bool_coercion() {
        use crate::db::models::KeyRevocationRow;
        let row = KeyRevocationRow {
            id: "rev-1".into(),
            dht_anchor_hash: b"uhCkR1".to_vec(),
            subject_human_id: "human-matthew".into(),
            revoked_key: "uhCAkKEY".into(),
            trigger_type: "voluntary".into(),
            reason: "compromised".into(),
            initiated_by_cid: "human-matthew".into(),
            required_votes: 3,
            current_votes: 3,
            threshold_reached: 1,
            effective_at: Some("2026-04-25T00:00:00Z".into()),
            derived_compromise_at: None,
            created_at: "2026-04-25T00:00:00Z".into(),
            updated_at: "2026-04-25T00:00:00Z".into(),
        };
        let view = KeyRevocationView::from(row);
        assert!(
            view.threshold_reached,
            "threshold_reached should be true when DB value is 1"
        );
        assert_eq!(view.required_votes, 3);
        assert_eq!(view.current_votes, 3);

        // Verify JSON output uses boolean
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["thresholdReached"], true);
    }

    use crate::error::StorageError;

    /// Gate-rejection messages from the imagodei coordinator map to 403.
    #[test]
    fn map_zome_err_to_http_403_for_gate_rejection() {
        let cases = [
            "Conductor(\"create_self_revocation: caller does not control revoked_key (different human_id)\")",
            "Conductor(\"submit_revocation_vote: caller is not an active emergency contact for human-x\")",
            "Conductor(\"submit_specialist_revocation: caller is not a configured defender for this human\")",
            "Conductor(\"submit_specialist_revocation: revoked_pub_key does not belong to target human\")",
        ];
        for msg in cases {
            let err = StorageError::Conductor(msg.to_string());
            let resp = map_zome_err_to_http(&err);
            assert_eq!(
                resp.status(),
                hyper::StatusCode::FORBIDDEN,
                "expected 403 for {msg}"
            );
        }
    }

    /// Input-validation failures map to 400.
    #[test]
    fn map_zome_err_to_http_400_for_invalid_input() {
        let cases = [
            "Conductor(\"create_self_revocation: invalid reason 'bogus'\")",
            "Conductor(\"submit_revocation_vote: revocation rev-x already effective\")",
            "Conductor(\"submit_revocation_vote: revocation rev-x has trigger_type=voluntary, votes not accepted\")",
            "Conductor(\"submit_revocation_vote: attestation cannot be empty\")",
            "Conductor(\"submit_revocation_vote: no KeyRevocation with id rev-missing\")",
        ];
        for msg in cases {
            let err = StorageError::Conductor(msg.to_string());
            let resp = map_zome_err_to_http(&err);
            assert_eq!(
                resp.status(),
                hyper::StatusCode::BAD_REQUEST,
                "expected 400 for {msg}"
            );
        }
    }

    /// Connectivity failures map to 503.
    #[test]
    fn map_zome_err_to_http_503_for_connection_error() {
        let err = StorageError::Connection("Admin connect failed: refused".to_string());
        let resp = map_zome_err_to_http(&err);
        assert_eq!(resp.status(), hyper::StatusCode::SERVICE_UNAVAILABLE);
    }

    /// Anything else falls through to 500.
    #[test]
    fn map_zome_err_to_http_500_for_unknown() {
        let err =
            StorageError::Conductor("Zome call failed: unexpected internal error".to_string());
        let resp = map_zome_err_to_http(&err);
        assert_eq!(resp.status(), hyper::StatusCode::INTERNAL_SERVER_ERROR);
    }
}
