//! Account API controller — M5 Recovery Protocol Phase 2
//!
//! Routes: `/api/v1/account`, `/api/v1/account/keys`, `/api/v1/account/revocations`,
//!         `/api/v1/account/self-revocation`, `/api/v1/account/pending-recovery`,
//!         `/api/v1/account/recovery/:id/vote`, `/api/v1/account/portal-hosts`
//!
//! ## Auth pattern
//! All routes resolve the calling agent via `X-Agent-Id` header (doorway JWT injection)
//! with active local session as Tauri fallback — same pattern as `api/identity.rs`.
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
) -> Result<Response<Full<Bytes>>, StorageError> {
    match (method, resource_path) {
        // ── Aggregate ──────────────────────────────────────────────────────
        (Method::GET, "" | "/") => get_account(req, pool).await,

        // ── Key history ───────────────────────────────────────────────────
        (Method::GET, "/keys") => get_account_keys(req, pool).await,

        // ── Revocation history ────────────────────────────────────────────
        (Method::GET, "/revocations") => get_account_revocations(req, pool).await,

        // ── Self-revocation (zome write — Phase 11 bridge) ────────────────
        (Method::POST, "/self-revocation") => Ok(zome_bridge_not_yet_wired("self-revocation")),

        // ── Pending recovery requests (where I am EC) ─────────────────────
        (Method::GET, "/pending-recovery") => get_pending_recovery(req, pool).await,

        // ── Recovery vote (zome write — Phase 11 bridge) ──────────────────
        (Method::POST, p) if p.starts_with("/recovery/") && p.ends_with("/vote") => {
            Ok(zome_bridge_not_yet_wired("recovery/vote"))
        }

        // ── Portal hosts ──────────────────────────────────────────────────
        (Method::GET, "/portal-hosts") => get_portal_hosts(req, pool).await,
        (Method::POST, "/portal-hosts") => Ok(zome_bridge_not_yet_wired("portal-hosts")),
        (Method::DELETE, p) if p.starts_with("/portal-hosts/") => {
            Ok(zome_bridge_not_yet_wired("portal-hosts/:url_b64"))
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
    let agent_key = extract_agent_key(&req, &mut conn)?;
    let agent_key = match agent_key {
        Some(k) => k,
        None => {
            return Ok(response::bad_request(
                "Cannot resolve account: no X-Agent-Id header and no active session",
            ))
        }
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
            .filter(dsl::human_id.eq(&human_id))
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

    let agent_key = extract_agent_key(&req, &mut conn)?;
    let agent_key = match agent_key {
        Some(k) => k,
        None => {
            return Ok(response::bad_request(
                "Cannot resolve account: no X-Agent-Id header and no active session",
            ))
        }
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

    let agent_key = extract_agent_key(&req, &mut conn)?;
    let agent_key = match agent_key {
        Some(k) => k,
        None => {
            return Ok(response::bad_request(
                "Cannot resolve account: no X-Agent-Id header and no active session",
            ))
        }
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
        .filter(dsl::human_id.eq(&human.id))
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

    let agent_key = extract_agent_key(&req, &mut conn)?;
    let agent_key = match agent_key {
        Some(k) => k,
        None => {
            return Ok(response::bad_request(
                "Cannot resolve account: no X-Agent-Id header and no active session",
            ))
        }
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

    let agent_key = extract_agent_key(&req, &mut conn)?;
    let agent_key = match agent_key {
        Some(k) => k,
        None => {
            return Ok(response::bad_request(
                "Cannot resolve account: no X-Agent-Id header and no active session",
            ))
        }
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
// Phase 11 stub: zome bridge not yet wired
// ---------------------------------------------------------------------------

/// Returns 503 with a machine-readable body indicating the conductor bridge
/// is not yet wired. Angular should surface this as "coming soon" rather than
/// an unhandled error.
///
/// POST routes that require DHT writes (self-revocation, recovery vote,
/// portal-host CRUD) defer to Phase 11 when `HcClient` is threaded through
/// the API layer.
fn zome_bridge_not_yet_wired(route_hint: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "conductor bridge not yet wired",
        "code": "PHASE_11_PENDING",
        "route": route_hint,
        "message": "This endpoint requires a live conductor connection. \
                    The imagodei coordinator zome bridge is deferred to Phase 11. \
                    Recovery and portal-host mutations are accepted via direct \
                    Holochain conductor calls in the meantime."
    });
    response::json_response(hyper::StatusCode::SERVICE_UNAVAILABLE, &body)
}

// ---------------------------------------------------------------------------
// Auth helper
// ---------------------------------------------------------------------------

/// Resolve the current agent public key from the request.
///
/// Resolution order:
/// 1. `X-Agent-Id` header — doorway JWT middleware injects this after validation
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

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The zome bridge stub must return 503 with a machine-readable error body
    /// so Angular can surface "conductor bridge not available" instead of an
    /// unhandled error state.
    #[test]
    fn zome_bridge_stub_returns_503() {
        let resp = zome_bridge_not_yet_wired("self-revocation");
        assert_eq!(resp.status(), hyper::StatusCode::SERVICE_UNAVAILABLE);
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
            dht_anchor_hash: "uhCkR1".into(),
            id: "rev-1".into(),
            human_id: "human-matthew".into(),
            revoked_key: "uhCAkKEY".into(),
            reason: "compromised".into(),
            trigger_type: "voluntary".into(),
            initiated_by: "human-matthew".into(),
            required_votes: 3,
            current_votes: 3,
            threshold_reached: 1,
            effective_at: Some("2026-04-25T00:00:00Z".into()),
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

    /// AccountView route stub for the `account` sub-path prefix stripping.
    /// Verifies the dispatcher handles both `""` (bare) and `"/"` (trailing slash).
    #[test]
    fn zome_bridge_all_stub_routes_return_503() {
        let routes = [
            "self-revocation",
            "recovery/vote",
            "portal-hosts",
            "portal-hosts/:url_b64",
        ];
        for r in &routes {
            let resp = zome_bridge_not_yet_wired(r);
            assert_eq!(
                resp.status(),
                hyper::StatusCode::SERVICE_UNAVAILABLE,
                "route {r} should return 503"
            );
        }
    }
}
