//! Admin Seed Routes - Blob upload for seeding operations
//!
//! Provides endpoints for uploading blobs during initial seeding:
//! - `PUT /admin/seed/blob` - Upload a blob, forwarded to elohim-storage
//!
//! ## Security
//!
//! These endpoints require seed authority: this doorway's own admin identity
//! (API key or JWT) at any stage, or — while the DECLARED network stage is
//! still pre-coordination — an on-the-box (loopback) caller or the fleet's
//! declared seed-authority key (`API_KEY_SEED`). Authority is never derived
//! from `DEV_MODE`; see [`require_seed_authority`].
//!
//! ## Flow
//!
//! ```text
//! Seeder → PUT /admin/seed/blob → Doorway
//!                                   ├── Local cache (fast, write-through)
//!                                   └── Forward to elohim-storage (authoritative)
//! ```
//!
//! Doorway acts as a write-cache (like SSD cache for HDD).
//! elohim-storage is the authoritative blob store.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use seam_contracts::freshness::NetworkStage;

use crate::auth::{extract_http_permission, ApiKeyValidator, PermissionLevel};
use crate::server::AppState;

// =============================================================================
// Authorization gate — Stage A (CI seed authority)
// =============================================================================

/// Require seed/cache-mutation authority for an incoming request.
///
/// Stage A — resolves to the `seed-content` operator capability at Stage C
/// (`auth/operator.rs`); transition-window Admin check per `operator.rs`
/// sanction. Until the capability-scoped resolver is wired for the seed path,
/// this gate accepts any request that resolves to [`PermissionLevel::Admin`]
/// (legacy Admin JWT or admin API key) at ANY stage, plus two affordances that
/// exist only while the DECLARED stage is pre-coordination: an on-the-box
/// (loopback) caller, and the fleet's declared seed-authority key
/// (`API_KEY_SEED`). Authority is derived from the declared stage, never from
/// `DEV_MODE` — see the body for why that swap preserves deployed behaviour.
///
/// Returns:
/// - `Ok(())` if authorized (pre-coordination loopback, pre-coordination fleet
///   seed key, or resolved level >= Admin).
/// - `Err(401)` if no/invalid bearer resolved to [`PermissionLevel::Public`].
/// - `Err(403)` if authenticated but below Admin.
///
/// Generic over the request body type so the gate runs on request **headers
/// only** — a rejected request never has its blob body consumed.
// Matches the established `require_admin`/`get_jwt_validator` pattern in
// admin_users.rs: rejection responses carry a full `Response` in the Err arm.
#[allow(clippy::result_large_err)]
pub(crate) fn require_seed_authority<B>(
    state: &AppState,
    req: &Request<B>,
    peer_is_loopback: bool,
) -> Result<(), Response<Full<Bytes>>> {
    // ── How seed authority is decided ───────────────────────────────────────
    //
    // Seed authority is priced against the network's DECLARED operating stage
    // (`ELOHIM_NETWORK_STAKES`, resolved once at boot into
    // `AppState::network_stage`, fail-closed to `Bootstrap`) — NEVER against
    // `DEV_MODE`. That is the discipline elohim-storage states in
    // `trust/stage.rs`, naming this very file's neighbour: "`NetworkStage` must
    // never derive from any `DEV_MODE` flag — neither elohim-storage's inert one
    // nor doorway's live, auth-permissive one (`config.rs`)." This gate was the
    // last place in the doorway still deriving a posture from that flag.
    //
    // WHY THE SWAP IS NOT A WIDENING: every deployed doorway manifest sets
    // `DEV_MODE: "true"` (alpha, alpha-b, prod, staging, staging-read) and NONE
    // declares `ELOHIM_NETWORK_STAKES`, so every one of them resolves to
    // `Bootstrap`. `Bootstrap < Coordinated` holds in exactly the places
    // `dev_mode` held, so deployed behaviour is preserved — while a doorway that
    // declares `coordinated`/`enforced` switches BOTH pre-coordination
    // affordances off by itself. That expiry is designed, not accidental: past
    // coordination the deploy pipeline is expected to carry a bounded, revocable
    // authority (the REA compute-commitment path already shadow-running behind
    // `DELEGATES_COMPUTE_OP_GATE`) instead of a standing key.
    let pre_coordination = state.network_stage < NetworkStage::Coordinated;

    // (1) THE CALLER IS ON THE BOX. `peer_is_loopback` is derived from the
    // ACCEPTED SOCKET's peer address, never from `X-Forwarded-For` — an attacker
    // sets headers, not the kernel's notion of who connected. Behind a cluster
    // ingress the peer is the ingress pod (not loopback), so the fleet
    // authenticates; on a developer's box the peer is 127.0.0.1, so
    // `pnpm run hc:start:seed` and the local mesh keep working with no
    // credential and nothing to configure.
    //
    // This is the conjunct commit 62b658784 added, and it carries the security
    // property: the flag ALONE used to pass, and since every deployed manifest
    // sets it, this gate and the four `/admin/cache/*` mutation routes were
    // reachable with no credential from the open web (proven 2026-08-24 on the
    // local mesh — an anonymous PUT with a deliberately wrong `X-Blob-Hash`
    // answered `409 Hash mismatch`, i.e. it passed the gate and was stopped only
    // by content addressing).
    if pre_coordination && peer_is_loopback {
        return Ok(());
    }

    // (2) THE FLEET'S DECLARED SEED AUTHORITY. One deploy pipeline operates
    // several doorways whose own operator identities are DELIBERATELY distinct
    // (`alpha-b.yaml`), so "may this caller seed?" is not the same question as
    // "is this caller MY admin?" — `API_KEY_SEED` answers the first,
    // `API_KEY_ADMIN` the second. Conflating them is what stranded the apex:
    // CI holds one key, doorway-B has another, and `DEV_MODE: "true"` stood in
    // for the missing per-host credential until (1) closed that bypass.
    //
    // Presence-keyed: a doorway declaring no seed key has NO fleet seed
    // authority (prod declares none), so this cannot become a production posture
    // by accident. Scoped to THIS gate — the key never enters
    // `extract_http_permission`'s ladder, so it authorizes seeding and cache
    // mutation and nothing else: it cannot read a user, mint a token, promote an
    // account, or reach the conductor.
    if pre_coordination && fleet_seed_authority_presented(state, req) {
        return Ok(());
    }

    // (3) THIS DOORWAY'S OWN OPERATOR IDENTITY — valid at EVERY stage.

    let level = extract_http_permission(state, req, peer_is_loopback);
    if level >= PermissionLevel::Admin {
        Ok(())
    } else if level >= PermissionLevel::Authenticated {
        // Real identity, but not operator-grade: distinguish from anonymous.
        Err(error_response(
            StatusCode::FORBIDDEN,
            "Admin permission required to seed content",
        ))
    } else {
        Err(error_response(
            StatusCode::UNAUTHORIZED,
            "Authentication required to seed content",
        ))
    }
}

/// Does this request carry the fleet's declared seed-authority key?
///
/// Header-only, like the gate that calls it, so a rejected request never has its
/// blob body consumed.
///
/// Returns `false` when no seed key is configured, when the request carries no
/// `X-API-Key`, or when the presented value does not match. The explicit
/// `Some(presented)` guard is LOAD-BEARING, not defensive:
/// [`ApiKeyValidator::validate`] answers `Some(PermissionLevel::Public)` for a
/// MISSING key, so matching on the level alone without that guard would be a
/// bug the moment the match arm widened.
///
/// `ApiKeyValidator` is used purely to reuse its constant-time comparison; the
/// level it yields is local to this function and never reaches a caller, which
/// is what keeps the fleet key strictly narrower than Admin.
fn fleet_seed_authority_presented<B>(state: &AppState, req: &Request<B>) -> bool {
    let Some(seed_key) = state.args.configured_seed_key() else {
        return false;
    };
    let Some(presented) = req.headers().get("x-api-key").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    matches!(
        ApiKeyValidator::new(None, Some(seed_key.to_string())).validate(Some(presented)),
        Some(PermissionLevel::Admin)
    )
}

// =============================================================================
// Types
// =============================================================================

/// Response from blob upload
#[derive(Debug, Serialize)]
pub struct BlobUploadResponse {
    pub success: bool,
    pub hash: String,
    /// Whether the blob was already present (in doorway cache)
    pub already_cached: bool,
    /// Whether blob was forwarded to elohim-storage
    pub forwarded_to_storage: bool,
    /// Size in bytes
    pub size: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response from elohim-storage
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StorageResponse {
    hash: Option<String>,
    size: Option<u64>,
    error: Option<String>,
}

// =============================================================================
// Route Handler
// =============================================================================

/// Handle PUT /admin/seed/blob
///
/// Expects:
/// - Body: Raw blob data
/// - Header `X-Blob-Hash`: Expected hash of the blob
/// - Header `Content-Type`: MIME type of the blob
/// - Header `X-Blob-Size`: Optional size hint
///
/// Flow:
/// 1. Verify hash matches body
/// 2. Cache locally (fast write-through cache)
/// 3. Forward to elohim-storage (authoritative store)
///
/// Returns:
/// - 200 OK with BlobUploadResponse on success
/// - 400 Bad Request if missing required headers
/// - 409 Conflict if hash mismatch
pub async fn handle_seed_blob(
    req: Request<Incoming>,
    state: Arc<AppState>,
    peer_is_loopback: bool,
) -> Response<Full<Bytes>> {
    // Stage A authorization — gate BEFORE reading the body so a rejected
    // request never streams the blob to cache/storage. Headers-only check.
    if let Err(resp) = require_seed_authority(&state, &req, peer_is_loopback) {
        return resp;
    }

    // Extract required headers
    let expected_hash = match req.headers().get("X-Blob-Hash") {
        Some(h) => match h.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid X-Blob-Hash header"),
        },
        None => return error_response(StatusCode::BAD_REQUEST, "Missing X-Blob-Hash header"),
    };

    let content_type = req
        .headers()
        .get("Content-Type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    debug!(
        hash = %expected_hash,
        content_type = %content_type,
        "Processing seed blob upload"
    );

    // Check if already cached locally
    if let Some(cached_size) = state.cache.blob_size(&expected_hash) {
        info!(hash = %expected_hash, "Blob already in local cache");

        // Still try to forward to storage in case it's missing there
        let forwarded = forward_to_storage(&state, &expected_hash, None).await;

        return json_response(
            StatusCode::OK,
            &BlobUploadResponse {
                success: true,
                hash: expected_hash,
                already_cached: true,
                forwarded_to_storage: forwarded,
                size: cached_size,
                error: None,
            },
        );
    }

    // Read request body
    let body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            warn!(error = %e, "Failed to read blob body");
            return error_response(StatusCode::BAD_REQUEST, "Failed to read request body");
        }
    };

    let blob_size = body.len();
    debug!(hash = %expected_hash, size = blob_size, "Received blob data");

    // Verify hash matches
    let computed_hash = compute_sha256(&body);
    if computed_hash != expected_hash {
        warn!(
            expected = %expected_hash,
            computed = %computed_hash,
            "Blob hash mismatch"
        );
        let error_msg = format!("Hash mismatch: expected {expected_hash}, got {computed_hash}");
        return json_response(
            StatusCode::CONFLICT,
            &BlobUploadResponse {
                success: false,
                hash: expected_hash,
                already_cached: false,
                forwarded_to_storage: false,
                size: blob_size,
                error: Some(error_msg),
            },
        );
    }

    // Store in local cache (1 hour TTL - acts as write-through cache)
    let seed_blob_ttl = std::time::Duration::from_secs(3600);
    state
        .cache
        .set(&expected_hash, body.to_vec(), &content_type, seed_blob_ttl);
    info!(hash = %expected_hash, size = blob_size, "Blob cached locally");

    // Forward to elohim-storage (authoritative store)
    let forwarded = forward_to_storage(&state, &expected_hash, Some(&body)).await;

    if forwarded {
        info!(hash = %expected_hash, "Blob forwarded to elohim-storage");
    } else {
        warn!(hash = %expected_hash, "Failed to forward blob to elohim-storage (will retry on read)");
    }

    json_response(
        StatusCode::OK,
        &BlobUploadResponse {
            success: true,
            hash: expected_hash,
            already_cached: false,
            forwarded_to_storage: forwarded,
            size: blob_size,
            error: None,
        },
    )
}

/// Base seed-blob forward timeout in seconds, before size scaling.
///
/// A fixed 30s budget was observed insufficient for a large blob (2026-08-23
/// local mesh: a 71,763,974-byte landing bundle exceeded it — storage shards
/// a blob that size slower than 30s on a debug build — so the forward
/// reported `forwarded_to_storage:false` for bytes that were, in fact, still
/// landing). See [`compute_forward_timeout_secs`] for the scaling.
const DEFAULT_SEED_FORWARD_TIMEOUT_BASE_SECS: u64 = 30;

/// Additional forward/read-back budget per MiB of blob size.
const DEFAULT_SEED_FORWARD_TIMEOUT_SECS_PER_MIB: u64 = 1;

/// Read the forward-timeout base, honoring `SEED_FORWARD_TIMEOUT_BASE_SECS`
/// the same way [`crate::services::zome_caller::zome_call_timeout`] reads
/// `DOORWAY_ZOME_CALL_TIMEOUT_MS`.
fn seed_forward_timeout_base_secs() -> u64 {
    std::env::var("SEED_FORWARD_TIMEOUT_BASE_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SEED_FORWARD_TIMEOUT_BASE_SECS)
}

/// Read the per-MiB forward-timeout increment, honoring
/// `SEED_FORWARD_TIMEOUT_SECS_PER_MIB`.
fn seed_forward_timeout_secs_per_mib() -> u64 {
    std::env::var("SEED_FORWARD_TIMEOUT_SECS_PER_MIB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SEED_FORWARD_TIMEOUT_SECS_PER_MIB)
}

/// Compute the size-scaled forward timeout budget: `base + per_mib *
/// ceil(bytes / MiB)`. Pure arithmetic, kept separate from env/network so it
/// is unit-testable on its own (see `tests::` below).
fn compute_forward_timeout_secs(bytes: usize, base_secs: u64, secs_per_mib: u64) -> u64 {
    const MIB: u64 = 1024 * 1024;
    let mib = (bytes as u64).div_ceil(MIB);
    base_secs + secs_per_mib * mib
}

/// The forward/read-back timeout budget (seconds) for a blob of `bytes` size,
/// reading the env overrides described on [`seed_forward_timeout_base_secs`]
/// and [`seed_forward_timeout_secs_per_mib`].
fn seed_forward_timeout_secs(bytes: usize) -> u64 {
    compute_forward_timeout_secs(
        bytes,
        seed_forward_timeout_base_secs(),
        seed_forward_timeout_secs_per_mib(),
    )
}

/// Forward a blob to elohim-storage
///
/// If body is None, reads from local cache.
async fn forward_to_storage(state: &AppState, hash: &str, body: Option<&Bytes>) -> bool {
    let storage_url = match &state.args.storage_url {
        Some(url) => url.clone(),
        None => {
            debug!("No storage_url configured, skipping forward");
            return false;
        }
    };

    // Get body from cache if not provided
    let data = match body {
        Some(b) => b.to_vec(),
        None => match state.cache.get(hash) {
            Some(entry) => entry.data.clone(),
            None => {
                warn!(hash = %hash, "Blob not in cache, cannot forward");
                return false;
            }
        },
    };

    // Build URL: PUT /blob/{hash}
    let url = format!("{}/blob/{}", storage_url.trim_end_matches('/'), hash);

    let timeout_secs = seed_forward_timeout_secs(data.len());
    debug!(
        url = %url,
        size = data.len(),
        timeout_secs,
        "Forwarding blob to elohim-storage"
    );

    // Use reqwest client from state or create one
    let client = reqwest::Client::new();

    match client
        .put(&url)
        .header("Content-Type", "application/octet-stream")
        .body(data)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                // Read-back verification: a 2xx PUT alone has been observed
                // reporting `forwarded_to_storage: true` while the bytes never
                // became durably servable (2026-08-16 local mesh: an 18MB SPA
                // bundle vanished while three sibling blobs from the same run
                // survived; storage accepts the same bytes fine when PUT
                // directly). Until that class has a root cause, "forwarded"
                // means storage SERVES the blob, not that a PUT returned 200.
                let verify_url = format!("{}/blob/{}", storage_url.trim_end_matches('/'), hash);
                match client
                    .get(&verify_url)
                    .timeout(std::time::Duration::from_secs(timeout_secs))
                    .send()
                    .await
                {
                    Ok(check) if check.status().is_success() => true,
                    Ok(check) => {
                        error!(
                            hash = %hash,
                            status = %check.status(),
                            "seed-blob forward read-back FAILED — storage accepted the PUT \
                             but does not serve the blob; reporting forwarded=false"
                        );
                        false
                    }
                    Err(e) => {
                        error!(
                            hash = %hash,
                            error = %e,
                            "seed-blob forward read-back unreachable — reporting forwarded=false"
                        );
                        false
                    }
                }
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                error!(
                    hash = %hash,
                    status = %status,
                    body = %body,
                    "elohim-storage returned error"
                );
                false
            }
        }
        Err(e) => {
            error!(hash = %hash, error = %e, "Failed to connect to elohim-storage");
            false
        }
    }
}

/// Check if a blob exists in the cache
pub async fn handle_check_blob(hash: &str, state: Arc<AppState>) -> Response<Full<Bytes>> {
    if state.cache.blob_size(hash).is_some() {
        Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::new()))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Compute SHA256 hash of data (hex-encoded to match seeder convention)
fn compute_sha256(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    // Use hex encoding to match seeder (blob-manager.ts) convention
    format!("sha256-{result:x}")
}

/// Create JSON response
fn json_response<T: Serialize>(status: StatusCode, data: &T) -> Response<Full<Bytes>> {
    let body = serde_json::to_string(data)
        .unwrap_or_else(|_| r#"{"error":"Serialization failed"}"#.to_string());

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Create error response
fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "success": false,
        "error": message,
    });

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{JwtValidator, TokenInput};
    use crate::config::Args;
    use clap::Parser;
    use http_body_util::Empty;

    const TEST_JWT_SECRET: &str = "test-secret-that-is-at-least-32-characters-long";

    #[test]
    fn test_compute_sha256() {
        let data = b"hello world";
        let hash = compute_sha256(data);
        // SHA256 of "hello world" is well-known
        assert!(hash.starts_with("sha256-"));
    }

    /// Build an AppState with the given dev_mode and a configured prod JWT
    /// secret so non-dev JWT validation works.
    fn test_state(dev_mode: bool) -> AppState {
        let mut args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        args.dev_mode = dev_mode;
        args.api_key_admin = Some("test-admin-key".to_string());
        args.jwt_secret = Some(TEST_JWT_SECRET.to_string());
        AppState::new(args)
    }

    /// Bearer JWT signed with the same prod secret, carrying `level`.
    fn bearer_jwt(level: PermissionLevel) -> String {
        let validator = JwtValidator::new(TEST_JWT_SECRET.into(), 3600).unwrap();
        let input = TokenInput {
            human_id: "human-123".into(),
            agent_pub_key: "uhCAk...".into(),
            identifier: "ci@example.com".into(),
            permission_level: level,
            session_id: None,
            doorway_id: None,
            doorway_url: None,
            conductor_id: None,
            installed_app_id: None,
            is_steward: false,
            has_local_conductor: false,
        };
        validator.generate_token(input).unwrap()
    }

    fn seed_req(bearer: Option<&str>) -> Request<Empty<Bytes>> {
        let mut builder = Request::builder()
            .method("PUT")
            .uri("/admin/seed/blob")
            .header("X-Blob-Hash", "sha256-deadbeef");
        if let Some(t) = bearer {
            builder = builder.header(hyper::header::AUTHORIZATION, format!("Bearer {t}"));
        }
        builder.body(Empty::<Bytes>::new()).unwrap()
    }

    /// (a) PROVE FIRST: a pre-coordination LOOPBACK caller passes the gate with
    /// NO auth header — the local-stack seeding safety invariant
    /// (`pnpm run hc:start:seed`, the local mesh). Trusted because the caller is
    /// ON THE BOX, not because a flag is set.
    #[test]
    fn pre_coordination_loopback_no_auth_passes_gate() {
        let state = test_state(true);
        let req = seed_req(None);
        assert!(
            require_seed_authority(&state, &req, true).is_ok(),
            "a pre-coordination LOOPBACK caller must pass the seed gate with no credential"
        );
    }

    /// Build a seed request carrying an `X-API-Key`.
    fn seed_req_with_api_key(key: &str) -> Request<Empty<Bytes>> {
        Request::builder()
            .method("PUT")
            .uri("/admin/seed/blob")
            .header("X-API-Key", key)
            .body(Empty::<Bytes>::new())
            .unwrap()
    }

    const FLEET_SEED_KEY: &str = "fleet-seed-authority-key";

    /// THE APEX FIX. One deploy pipeline, several doorways with deliberately
    /// distinct admin identities: a REMOTE caller (behind the cluster ingress,
    /// so never loopback) presenting the fleet's declared seed key must be
    /// admitted even though that key is NOT this doorway's `API_KEY_ADMIN`.
    /// Before this, doorway-B answered 403 to every App-pipeline seed leg and
    /// elohim.host served a stale bundle by construction.
    #[test]
    fn fleet_seed_key_admits_the_remote_deploy_caller() {
        let mut state = test_state(true);
        state.args.api_key_seed = Some(FLEET_SEED_KEY.to_string());
        let req = seed_req_with_api_key(FLEET_SEED_KEY);
        assert!(
            require_seed_authority(&state, &req, false).is_ok(),
            "the fleet's declared seed key must admit a remote deploy caller"
        );
    }

    /// Presence-keyed, not flag-keyed: a doorway that declares NO seed key has
    /// no fleet seed authority at all. This is what keeps the affordance out of
    /// production posture by construction rather than by remembering.
    #[test]
    fn an_undeclared_fleet_seed_key_admits_nobody() {
        let state = test_state(true);
        assert!(state.args.configured_seed_key().is_none());
        let req = seed_req_with_api_key(FLEET_SEED_KEY);
        let err = require_seed_authority(&state, &req, false)
            .expect_err("no declared seed key must mean no fleet seed authority");
        assert!(err.status().is_client_error());
    }

    #[test]
    fn a_wrong_fleet_seed_key_is_refused() {
        let mut state = test_state(true);
        state.args.api_key_seed = Some(FLEET_SEED_KEY.to_string());
        let req = seed_req_with_api_key("not-the-fleet-key");
        let err = require_seed_authority(&state, &req, false)
            .expect_err("a mismatched fleet seed key must be refused");
        assert!(err.status().is_client_error());
    }

    /// A blank declaration is NOT an empty credential that an empty header
    /// matches — it is the absence of a declaration. Pins
    /// `Args::configured_seed_key`'s trim/reject rule at the gate.
    #[test]
    fn a_blank_fleet_seed_key_is_not_an_authority() {
        let mut state = test_state(true);
        state.args.api_key_seed = Some("   ".to_string());
        let req = seed_req_with_api_key("   ");
        let err = require_seed_authority(&state, &req, false)
            .expect_err("a blank seed-key declaration must not authorize anything");
        assert!(err.status().is_client_error());
    }

    /// LEAST PRIVILEGE, PINNED. The fleet seed key authorizes seeding and cache
    /// mutation and NOTHING else — it must never climb the permission ladder to
    /// Admin, or it would silently become a second total-admin credential on
    /// every doorway that declares it.
    #[test]
    fn fleet_seed_key_never_grants_general_admin() {
        let mut state = test_state(false);
        state.args.api_key_seed = Some(FLEET_SEED_KEY.to_string());
        let req = seed_req_with_api_key(FLEET_SEED_KEY);
        assert!(
            extract_http_permission(&state, &req, false) < PermissionLevel::Admin,
            "the fleet seed key must not resolve to Admin on the shared ladder"
        );
    }

    /// THE DESIGNED EXPIRY. Both pre-coordination affordances switch off by
    /// themselves once a doorway declares `coordinated` — no flag to remember to
    /// unset. Past coordination the deploy pipeline must carry a bounded,
    /// revocable authority instead of a standing key.
    #[test]
    fn coordinated_stage_retires_both_pre_coordination_affordances() {
        let mut state = test_state(true);
        state.args.api_key_seed = Some(FLEET_SEED_KEY.to_string());
        state.network_stage = NetworkStage::Coordinated;

        let err = require_seed_authority(&state, &seed_req(None), true)
            .expect_err("loopback must not seed at Coordinated");
        assert!(err.status().is_client_error());

        let err = require_seed_authority(&state, &seed_req_with_api_key(FLEET_SEED_KEY), false)
            .expect_err("the fleet seed key must not seed at Coordinated");
        assert!(err.status().is_client_error());
    }

    /// ...but this doorway's own operator identity still seeds at ANY stage.
    #[test]
    fn admin_identity_still_seeds_at_coordinated_stage() {
        let mut state = test_state(false);
        state.network_stage = NetworkStage::Coordinated;
        let token = bearer_jwt(PermissionLevel::Admin);
        assert!(
            require_seed_authority(&state, &seed_req(Some(&token)), false).is_ok(),
            "an Admin identity is stage-independent"
        );
    }

    /// (a2) THE REGRESSION THIS GATE EXISTS FOR. `dev_mode` is true on EVERY
    /// deployed doorway manifest (alpha, alpha-b, prod, staging, staging-read),
    /// so before the loopback conjunct a remote, credential-free caller passed
    /// this gate — proven 2026-08-24 against the local mesh, where an anonymous
    /// `PUT /admin/seed/blob` with a wrong `X-Blob-Hash` answered `409 Hash
    /// mismatch` (gate passed; only content addressing refused the write).
    /// A NON-loopback peer must now be REFUSED even with `dev_mode` on.
    ///
    /// Status is asserted only as "a refusal" (4xx), deliberately not 401-vs-403,
    /// and that tolerance has now earned its keep. When this test was written,
    /// `extract_http_permission` granted any caller `Authenticated` under
    /// `dev_mode`, so a remote anonymous caller resolved to 403 (below Admin).
    /// That grant was CLOSED on 2026-08-27 — the ladder now derives from
    /// `network_stage < Coordinated && peer_is_loopback` — so the same caller
    /// resolves to `Public` and this gate answers 401 instead. The assertion did
    /// not need to change, because it pins the property that is this gate's job:
    /// the remote caller does not SUCCEED.
    #[test]
    fn pre_coordination_remote_caller_without_credential_is_refused() {
        let state = test_state(true);
        let req = seed_req(None);
        let err = require_seed_authority(&state, &req, false)
            .expect_err("dev_mode must NOT admit a remote caller with no credential");
        assert!(
            err.status().is_client_error(),
            "a remote no-credential seed must be refused, got {}",
            err.status()
        );
    }

    /// (b) non-dev + no bearer ⇒ 401. The gate returns before the body is read,
    /// so the blob is never forwarded to cache/storage.
    #[test]
    fn non_dev_no_bearer_is_unauthorized() {
        let state = test_state(false);
        let req = seed_req(None);
        let err = require_seed_authority(&state, &req, false)
            .expect_err("no bearer in prod must be rejected");
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    /// (c) non-dev + Admin JWT ⇒ passes the gate.
    #[test]
    fn non_dev_admin_jwt_passes_gate() {
        let state = test_state(false);
        let token = bearer_jwt(PermissionLevel::Admin);
        let req = seed_req(Some(&token));
        assert!(
            require_seed_authority(&state, &req, false).is_ok(),
            "Admin JWT must pass the seed gate"
        );
    }

    /// (d) non-dev + Authenticated (non-admin) JWT ⇒ 403 (real identity, but
    /// below operator grade).
    #[test]
    fn non_dev_authenticated_jwt_is_forbidden() {
        let state = test_state(false);
        let token = bearer_jwt(PermissionLevel::Authenticated);
        let req = seed_req(Some(&token));
        let err = require_seed_authority(&state, &req, false)
            .expect_err("non-admin identity must be forbidden from seeding");
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    /// Admin API key (X-API-Key) also passes — the operator credential CI uses
    /// during the transition window before per-cap commitments land.
    #[test]
    fn non_dev_admin_api_key_passes_gate() {
        let state = test_state(false);
        let req = Request::builder()
            .method("PUT")
            .uri("/admin/seed/blob")
            .header("X-Blob-Hash", "sha256-deadbeef")
            .header("x-api-key", "test-admin-key")
            .body(Empty::<Bytes>::new())
            .unwrap();
        assert!(
            require_seed_authority(&state, &req, false).is_ok(),
            "Admin API key must pass the seed gate"
        );
    }

    // ── size-scaled forward timeout budget (2026-08-23 71MB local-mesh miss) ─

    #[test]
    fn compute_forward_timeout_secs_zero_bytes_is_base() {
        assert_eq!(
            compute_forward_timeout_secs(
                0,
                DEFAULT_SEED_FORWARD_TIMEOUT_BASE_SECS,
                DEFAULT_SEED_FORWARD_TIMEOUT_SECS_PER_MIB
            ),
            30,
            "an empty blob gets exactly the base budget"
        );
    }

    #[test]
    fn compute_forward_timeout_secs_64_mib() {
        let sixty_four_mib = 64 * 1024 * 1024;
        assert_eq!(
            compute_forward_timeout_secs(
                sixty_four_mib,
                DEFAULT_SEED_FORWARD_TIMEOUT_BASE_SECS,
                DEFAULT_SEED_FORWARD_TIMEOUT_SECS_PER_MIB
            ),
            94,
            "64 MiB at 1s/MiB + 30s base == 94s"
        );
    }

    #[test]
    fn compute_forward_timeout_secs_rounds_partial_mib_up() {
        // 1 byte over 1 MiB must still buy the second MiB's second, not be
        // truncated away by integer division — a truncating budget is exactly
        // the kind of size-blind gap that lost the 71,763,974-byte bundle.
        let just_over_one_mib = 1024 * 1024 + 1;
        assert_eq!(
            compute_forward_timeout_secs(just_over_one_mib, 30, 1),
            32,
            "a byte past 1 MiB must round the budget up to 2 whole MiB-seconds"
        );
    }

    #[test]
    fn seed_forward_timeout_secs_env_override_is_honored() {
        // Serialized via a process-local lock so the temporary set_var cannot
        // bleed into a parallel test (the env-flake class; mirrors
        // `zome_call_timeout`'s ENV_LOCK convention in services/zome_caller.rs).
        use std::sync::Mutex;
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _g = ENV_LOCK.lock().unwrap();

        std::env::remove_var("SEED_FORWARD_TIMEOUT_BASE_SECS");
        std::env::remove_var("SEED_FORWARD_TIMEOUT_SECS_PER_MIB");
        assert_eq!(
            seed_forward_timeout_secs(64 * 1024 * 1024),
            94,
            "unset env falls back to the 30s base + 1s/MiB default"
        );

        std::env::set_var("SEED_FORWARD_TIMEOUT_BASE_SECS", "60");
        std::env::set_var("SEED_FORWARD_TIMEOUT_SECS_PER_MIB", "2");
        assert_eq!(
            seed_forward_timeout_secs(64 * 1024 * 1024),
            60 + 2 * 64,
            "a valid env override on both knobs is honored"
        );

        std::env::remove_var("SEED_FORWARD_TIMEOUT_BASE_SECS");
        std::env::remove_var("SEED_FORWARD_TIMEOUT_SECS_PER_MIB");
    }

    // ── a cache hit re-attempts the forward instead of short-circuiting ──────

    /// Locks the fix for the 2026-08-23 incident: a retry PUT that hits the
    /// pantry (`already_cached:true`) must still call the real
    /// `forward_to_storage` and report ITS result — never a hardcoded/assumed
    /// `true` that leaves the blob missing from storage while the response
    /// claims success. Exercised directly at `forward_to_storage` (the
    /// established pattern here — see `fixture_only_gate` in admin_users.rs —
    /// is to test the pure/testable helper rather than synthesize a hyper
    /// `Request<Incoming>`, which `handle_seed_blob` cannot be constructed
    /// with in a unit test).
    #[tokio::test]
    async fn cache_hit_forward_reads_cache_and_reports_real_result_not_hardcoded_true() {
        let mut state = test_state(true);
        // No reachable storage: any real attempt to forward MUST fail, so a
        // `true` result here could only mean the cache-hit path short-circuited
        // to a hardcoded success instead of actually forwarding.
        state.args.storage_url = None;

        let hash = "sha256-cachehit";
        state.cache.set(
            hash,
            b"seed blob bytes already in the pantry".to_vec(),
            "application/octet-stream",
            std::time::Duration::from_secs(3600),
        );
        assert!(
            state.cache.blob_size(hash).is_some(),
            "precondition: blob must be cache-resident (already_cached:true path)"
        );

        // Mirrors the `already_cached` branch in `handle_seed_blob`: body=None
        // forces forward_to_storage to read the bytes back out of the cache.
        let forwarded = forward_to_storage(&state, hash, None).await;
        assert!(
            !forwarded,
            "no storage_url is configured, so the real forward must fail — \
             `already_cached:true` may never be paired with an unverified \
             `forwarded_to_storage:true`"
        );
    }

    #[tokio::test]
    async fn forward_to_storage_returns_false_when_hash_absent_from_cache_and_no_body() {
        let mut state = test_state(true);
        state.args.storage_url = Some("http://127.0.0.1:1".to_string());

        let forwarded = forward_to_storage(&state, "sha256-never-cached", None).await;
        assert!(
            !forwarded,
            "a hash with no cache entry and no supplied body has nothing to forward"
        );
    }
}
