//! **DEV/FIXTURE ONLY** doorway-local operational controls.
//!
//! These endpoints exist purely so a2o fixtures can stage doorway-local
//! OPERATIONAL state that has no real-world equivalent reachable from CI — most
//! notably a portal host at a non-resolving `.example` origin that the live
//! `/healthz` HEAD probe could never reach, and (below) a holder doorway that
//! declares itself busy for a named window so a sibling relaying to it can
//! observe a genuine load-shedding 503.
//!
//! ## Endpoints
//!
//! - `PUT /admin/dev/portal-health` — set a portal host's health override
//! - `PUT /admin/dev/shed` — declare this doorway busy for a named window
//!
//! ## Both endpoints share ONE gate: [`fixture_surface_gate`] — never `dev_mode`
//!
//! **UPDATED 2026-09-20.** Both handlers used to be gated on `state.args.dev_mode`
//! alone (mirroring the steward-grant surface in `admin_users.rs`). That was a
//! live public-web exposure: `DEV_MODE: "true"` is set on EVERY deployed doorway
//! manifest, including public alpha
//! (`genesis/orchestrator/manifests/doorway/alpha.yaml`), so an anonymous caller
//! on the open internet could flip `portal_health_override` on the deployed
//! fleet. A gate on a route that makes a doorway declare itself unavailable or
//! rewrites its own health signal is exactly the class of defect
//! `doorway-auth-posture-declared-stage` exists to prevent (see that doc's "Why
//! DEV_MODE is not a posture"). See [`fixture_surface_gate`] for the actual
//! predicate (stage `== Simulacra` AND a loopback caller) and its
//! eight-question answer.
//!
//! **The READ side had the same flaw and is fixed the same way.**
//! `probe_first_portal_host` / `portal_probe_decision`
//! (`routes/auth_routes.rs`) used to consult `AppState::portal_health_override`
//! whenever `dev_mode` was true — i.e. on every deployed doorway. They now
//! consult it only when the DECLARED stage is `Simulacra`, so an override can
//! never be read back on a non-household doorway even if one were somehow set
//! before the write-side gate closed.
//!
//! ## Truth-layer note
//!
//! The portal-health override and the declared-shed override are both
//! doorway-local OPERATIONAL state (`AppState::portal_health_override`,
//! `AppState::dev_shed`). Neither is a projection of any notarized DHT entry —
//! doorway/CLAUDE.md explicitly permits "doorway-local Operational state" as
//! legitimate doorway-resident state.

use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{info, warn};

use seam_contracts::freshness::NetworkStage;

use crate::server::AppState;

type FullBody = Full<Bytes>;

/// Body-size ceiling for both fixture-only mutators. Both bodies are a
/// handful of JSON fields (`{"hostUrl":..,"healthy":..}` /
/// `{"retryAfterSecs":..}`); 4 KiB is generous headroom with no reason for
/// an unbounded `collect()` on a route that runs before any auth-adjacent
/// throttling — mirrors `routes::p2p_manifests::MAX_MANIFEST_BODY_BYTES`'s
/// pattern (that route's own ceiling is sized for a signed manifest, hence
/// the larger 8 KiB there).
const MAX_FIXTURE_BODY_BYTES: usize = 4 * 1024;

/// Collect a fixture-route body under [`MAX_FIXTURE_BODY_BYTES`], or a
/// `413 BODY_TOO_LARGE` response. Shared by both handlers so the bound and
/// its error shape cannot drift between them.
// Matches the established pattern here — see `routes::seed::require_seed_authority`
// — a full `Response` in the Err arm is the crate's idiom for "hand the
// caller-ready rejection straight back up"; boxing it would just move the
// allocation rather than remove it.
#[allow(clippy::result_large_err)]
async fn collect_bounded_body(req: Request<Incoming>) -> Result<Bytes, Response<FullBody>> {
    match Limited::new(req.into_body(), MAX_FIXTURE_BODY_BYTES)
        .collect()
        .await
    {
        Ok(b) => Ok(b.to_bytes()),
        Err(_) => Err(error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            &format!("request body exceeds {MAX_FIXTURE_BODY_BYTES} bytes"),
            "BODY_TOO_LARGE",
        )),
    }
}

/// Request body for `PUT /admin/dev/portal-health`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortalHealthRequest {
    /// The portal host URL whose health is being overridden. Must match the
    /// `hostUrl` the human registered via `/api/v1/account/portal-hosts` exactly
    /// (the probe loop keys on the storage row's `host_url`).
    host_url: String,
    /// `true` → the probe treats this host as reachable (returns it without a
    /// live HEAD); `false` → the probe skips this host.
    healthy: bool,
}

fn json_response(status: StatusCode, body: &serde_json::Value) -> Response<FullBody> {
    let bytes = serde_json::to_vec(body).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(bytes)))
        .unwrap()
}

fn error_response(status: StatusCode, error: &str, code: &str) -> Response<FullBody> {
    json_response(status, &serde_json::json!({ "error": error, "code": code }))
}

/// `PUT /admin/dev/portal-health` — set a doorway-local portal-host health override.
///
/// Body: `{ "hostUrl": string, "healthy": bool }`.
///
/// Gated by [`fixture_surface_gate`] FIRST, before any body parsing — the same
/// predicate `PUT /admin/dev/shed` uses (household stage `Simulacra` AND a
/// loopback caller; never `dev_mode`, which is `true` on every deployed
/// manifest including public alpha). Writes into
/// `AppState::portal_health_override`, which the portal-host probe now
/// consults ONLY when the declared stage is `Simulacra` (see
/// `routes::auth_routes::portal_probe_decision`). No Mongo, no DHT, no
/// projection.
pub async fn handle_set_portal_health(
    req: Request<Incoming>,
    state: Arc<AppState>,
    peer_is_loopback: bool,
) -> Response<FullBody> {
    // (1) HARD gate FIRST — before any body parsing. Invisible off the
    // household mesh.
    if let Some(forbidden) = fixture_surface_gate(&state, peer_is_loopback) {
        return forbidden;
    }

    // (2) Parse the body, bounded.
    let body_bytes = match collect_bounded_body(req).await {
        Ok(b) => b,
        Err(too_large) => return too_large,
    };
    let request: PortalHealthRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid JSON: {e}"),
                "BAD_JSON",
            )
        }
    };

    // (3) Apply the override.
    {
        let mut overrides = state.portal_health_override.write().await;
        overrides.insert(request.host_url.clone(), request.healthy);
    }
    info!(
        target: "doorway::admin_dev",
        host_url = %request.host_url,
        healthy = request.healthy,
        "dev portal-health override set"
    );

    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "hostUrl": request.host_url,
            "healthy": request.healthy,
        }),
    )
}

// ============================================================================
// PUT /admin/dev/shed — household-fixture-only declared shed
// ============================================================================
//
// Story 3.1 (serving-edge campaign): a2o scenario "A busy holder is set aside
// until the time it named has passed"
// (genesis/a2o/features/federation/name-routing.feature, @wip). No shed
// predicate in this tree declares a window longer than the 60s federation
// discovery cycle (2s admission / 20s converging-shell / 30s upstream
// breaker), so nothing honest can stage that scenario's second leg. This
// fixture supplies the CAUSE only — the response is the real
// `catching_up::shed_response` builder every genuine predicate on the serving
// path uses, so the bytes a relaying sibling observes are indistinguishable
// in shape from a real shed. Design:
// genesis/a2o/reports/recovery/serving-edge-20260919/story-3.1-design.md §9.4-9.5.

/// Cap on a `PUT /admin/dev/shed` request's `retryAfterSecs`. A leaked or
/// forgotten fixture call must not be able to wedge a household doorway
/// indefinitely — 300s is generous against the 60s discovery cycle the
/// scenario spans, and small against a developer's attention span.
pub const DEV_SHED_MAX_SECS: u64 = 300;

/// Request body for `PUT /admin/dev/shed`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShedRequest {
    /// Seconds this doorway should declare itself busy for. `0` clears any
    /// standing window.
    retry_after_secs: u64,
}

/// Guard `PUT /admin/dev/shed` **and** `PUT /admin/dev/portal-health` — the
/// ONE predicate for both fixture surfaces. **Deliberately NOT `dev_mode`** —
/// see this module's doc comment for why. This is a TWO-CONJUNCT gate,
/// answering the eight questions from `doorway-auth-posture-declared-stage`
/// ("Adding a gate"):
///
/// 1. **Whose question is it?** "Is this a household doorway staging a
///    fixture?" — never "is this caller my admin" and never "may this caller
///    seed"; a third question, distinct from both credentials this crate
///    already has. One predicate answers it for both fixture routes because
///    it is the same question asked twice, not two different questions that
///    happen to share code.
/// 2. **What stage does this affordance belong to?** `Simulacra` — and ONLY
///    `Simulacra`, not `< Coordinated` like `require_seed_authority`.
///    `hc-mesh.sh` is the only caller in this tree that declares
///    `ELOHIM_NETWORK_STAKES=simulacra`; every deployed manifest (alpha
///    included) declares none and fail-closes to `Bootstrap`
///    (`Bootstrap < Coordinated` is also true, which is exactly why a
///    `< Coordinated` comparison would reopen this on alpha — verified
///    2026-09-20 against `genesis/orchestrator/manifests/doorway/alpha.yaml`,
///    which sets `DEV_MODE: "true"` but no `ELOHIM_NETWORK_STAKES`). Equality
///    against `Simulacra` is the one comparison that is true on the household
///    and false everywhere else this crate runs.
/// 3. **Does it fail closed?** Yes — an unparsed/undeclared stage resolves to
///    `Bootstrap` (`resolve_stage`), which fails this gate; a non-loopback
///    peer address (kernel-observed, never `X-Forwarded-For`) also fails it.
/// 4. **Is it narrower than Admin?** Yes — it grants no identity at all, only
///    a household-local operational toggle with a hard 300s ceiling.
/// 5. **Does it expire?** Yes, twice over: the gate itself is unreachable the
///    moment a doorway declares any stage other than `Simulacra`, and the
///    window it sets self-clears at `until_secs` with no further call
///    ([`shed_remaining_secs`]).
/// 6. **Could the p2p plane answer this instead?** No — this is Category C
///    Operational fixture state with no DHT equivalent (design brief §1,
///    "the p2p plane cannot answer this, and should not be asked to").
/// 7. **Which developer mode is this for?** The household mesh only —
///    `hc-mesh.sh`'s `simulacra` declaration is unique to it among every
///    caller in this tree; native local-first (Tauri/CLI) and the web
///    devspace never declare that stage either.
/// 8. **Whose account is this caller acting on?** Its own, and only its own —
///    this doorway declaring ITSELF busy. There is no second subject.
fn fixture_surface_gate(state: &AppState, peer_is_loopback: bool) -> Option<Response<FullBody>> {
    let household_fixture_stage = state.network_stage == NetworkStage::Simulacra;
    if peer_is_loopback && household_fixture_stage {
        None
    } else {
        // Same shape as `admin_users::fixture_only_gate` / the seed gate's
        // refusal precedent — invisible on a production doorway, not a
        // distinguishable 404 that would leak "this route exists but you
        // can't use it."
        Some(error_response(
            StatusCode::FORBIDDEN,
            "this is a household-fixture-only surface",
            "FIXTURE_ONLY",
        ))
    }
}

/// The pure outcome of a `PUT /admin/dev/shed` request. No clock, no state —
/// see [`apply_shed_request`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShedRequestOutcome {
    /// `retryAfterSecs: 0` — clear any standing window.
    Cleared,
    /// A new window opened, expiring at this absolute wall-clock second.
    Set { until_secs: u64 },
    /// Requested window exceeds [`DEV_SHED_MAX_SECS`] — refused, nothing
    /// changed.
    TooLarge,
}

/// Pure decision for a shed request — the crate's established clock-injection
/// idiom (`elohim_peer_fabric::guard::Clock`; `server/membrane.rs`'s
/// `EdgeClock`): logic takes `now_secs`, the boundary supplies it. Testable
/// with literal seconds, no timer, no sleep.
pub fn apply_shed_request(requested_secs: u64, now_secs: u64) -> ShedRequestOutcome {
    if requested_secs == 0 {
        return ShedRequestOutcome::Cleared;
    }
    if requested_secs > DEV_SHED_MAX_SECS {
        return ShedRequestOutcome::TooLarge;
    }
    ShedRequestOutcome::Set {
        until_secs: now_secs.saturating_add(requested_secs),
    }
}

/// Remaining whole seconds on a standing shed window, or `None` once it has
/// elapsed (or none was ever set). Pure — same idiom as
/// [`apply_shed_request`]. Because `until_secs` was minted as
/// `now_secs + requested_secs` and both are whole seconds, a `Some` result is
/// always `>= 1` — never a zero-length window that would silently be a no-op.
pub fn shed_remaining_secs(until_secs: Option<u64>, now_secs: u64) -> Option<u64> {
    let until = until_secs?;
    if now_secs >= until {
        None
    } else {
        Some(until - now_secs)
    }
}

/// Read the wall clock exactly once, at the boundary. Never called from pure
/// logic — see `apply_shed_request` / `shed_remaining_secs`.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `PUT /admin/dev/shed` — declare this doorway busy for `retryAfterSecs`
/// seconds (`0` clears). Gated FIRST, before body parsing, by
/// [`fixture_surface_gate`] — never `dev_mode`.
pub async fn handle_set_shed(
    req: Request<Incoming>,
    state: Arc<AppState>,
    peer_is_loopback: bool,
) -> Response<FullBody> {
    // (1) HARD gate FIRST — before any body parsing. Invisible off the
    // household mesh; a garbage body on a closed gate never even gets read.
    if let Some(forbidden) = fixture_surface_gate(&state, peer_is_loopback) {
        return forbidden;
    }

    // (2) Parse the body, bounded.
    let body_bytes = match collect_bounded_body(req).await {
        Ok(b) => b,
        Err(too_large) => return too_large,
    };
    let request: ShedRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Invalid JSON: {e}"),
                "BAD_JSON",
            )
        }
    };

    // (3) Apply.
    match apply_shed_request(request.retry_after_secs, now_secs()) {
        ShedRequestOutcome::TooLarge => error_response(
            StatusCode::BAD_REQUEST,
            &format!("retryAfterSecs must be <= {DEV_SHED_MAX_SECS}"),
            "RETRY_AFTER_TOO_LARGE",
        ),
        ShedRequestOutcome::Cleared => {
            *state.dev_shed.write().await = None;
            info!(
                target: "doorway::admin_dev",
                "dev shed fixture cleared"
            );
            json_response(
                StatusCode::OK,
                &serde_json::json!({ "retryAfterSecs": 0, "active": false }),
            )
        }
        ShedRequestOutcome::Set { until_secs } => {
            *state.dev_shed.write().await = Some(until_secs);
            warn!(
                target: "doorway::admin_dev",
                retry_after_secs = request.retry_after_secs,
                "dev shed fixture set — this doorway will answer as a shedding holder"
            );
            json_response(
                StatusCode::OK,
                &serde_json::json!({
                    "retryAfterSecs": request.retry_after_secs,
                    "active": true,
                }),
            )
        }
    }
}

/// Consult the standing shed window at `now`, self-clearing it (and logging
/// the promotion exactly once) when it has elapsed. Pure with respect to
/// time except for the state read/write — the HTTP path's
/// [`check_and_clear`] supplies `now_secs()`; unit tests supply literal
/// seconds so no test sleeps or pauses a clock.
///
/// Cheap on the common (inactive) path: one read-lock acquire. The write
/// lock is only taken the one time a standing window is found to have
/// elapsed, so a request under an INACTIVE fixture costs no more than the
/// admission-semaphore path it sits beside in `handle_request`.
pub async fn check_and_clear_at(state: &AppState, now: u64) -> Option<u64> {
    {
        let snapshot = *state.dev_shed.read().await;
        match shed_remaining_secs(snapshot, now) {
            Some(remaining) => return Some(remaining),
            None if snapshot.is_none() => return None,
            None => {} // was Some but has elapsed — fall through to clear it
        }
    }
    let mut guard = state.dev_shed.write().await;
    // Re-check under the write lock: a fresh PUT may have reopened the
    // window between the read above and this write.
    if let Some(remaining) = shed_remaining_secs(*guard, now) {
        return Some(remaining);
    }
    if guard.is_some() {
        *guard = None;
        info!(
            target: "doorway::admin_dev",
            "dev shed fixture window elapsed — resuming normal serving"
        );
    }
    None
}

/// [`check_and_clear_at`] with the wall clock read at the boundary. The only
/// call site outside tests.
pub async fn check_and_clear(state: &AppState) -> Option<u64> {
    check_and_clear_at(state, now_secs()).await
}

/// True for a path this doorway must keep serving even while the
/// household-fixture-only declared shed is standing:
///
/// - the existing admission-exempt liveness/metrics family
///   (`crate::server::http::admission_exempt`) — a fixture-induced shed must
///   never mask a REAL probe the same way a real outage must not;
/// - `/admin` itself or anything under `/admin/` — otherwise the fixture that
///   set the shed could never be cleared, including `PUT /admin/dev/shed`
///   itself;
/// - `/.well-known` itself or anything under `/.well-known/` — DID/JWKS/auth-
///   discovery must not go dark;
/// - any path with a `federation` segment — the scenario this fixture exists
///   for depends on the 60s discovery tick still reading this doorway as
///   LIVE (a shedding doorway is a live, answering one; the tick's own probe
///   is `GET /health`, already covered above, and `GET
///   /api/v1/federation/p2p-peers` is the peer-cache read this excludes by
///   segment, not by substring — see `read_class`'s `has_segment` precedent
///   in `routes/freshness.rs` for why a segment match and not
///   `path.contains(...)`).
///
/// Every prefix check here is EXACT-match-or-slash-delimited, never a bare
/// `starts_with` on the raw string: `path.starts_with("/admin")` alone would
/// also exempt an ordinary content path like `/administration-guide` — a
/// story could then "pass" on a path the fixture never actually shed. Pinned
/// by `an_admin_lookalike_content_path_is_not_exempt` and its siblings.
pub fn dev_shed_exempt(path: &str, admission_exempt: bool) -> bool {
    admission_exempt
        || path == "/admin"
        || path.starts_with("/admin/")
        || path == "/.well-known"
        || path.starts_with("/.well-known/")
        || path.split('/').any(|seg| seg == "federation")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Args;
    use clap::Parser;

    fn test_state(dev_mode: bool) -> AppState {
        let mut args = Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        args.dev_mode = dev_mode;
        AppState::new(args)
    }

    /// UPDATED 2026-09-20: portal-health used to be gated on `dev_mode` alone
    /// (`fixture_only_gate`, now deleted) — a live public-web exposure, since
    /// `DEV_MODE: "true"` is set on every deployed manifest including public
    /// alpha. It now shares `fixture_surface_gate` with the declared-shed
    /// fixture. This test used to assert `dev_mode=false` alone blocked the
    /// gate; it now asserts the opposite axis matters — `dev_mode=true` is
    /// NOT enough without the `Simulacra` stage declaration, even loopback.
    #[test]
    fn dev_mode_true_does_not_open_portal_health_when_stage_is_not_simulacra() {
        let mut state = test_state(true);
        state.network_stage = NetworkStage::Bootstrap;
        let resp = fixture_surface_gate(&state, true)
            .expect("Bootstrap stage must refuse portal-health regardless of dev_mode");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// Same predicate as the shed fixture: a remote caller is refused even on
    /// the household's own declared stage.
    #[test]
    fn portal_health_non_loopback_refused_at_simulacra_stage() {
        let mut state = test_state(false);
        state.network_stage = NetworkStage::Simulacra;
        let resp = fixture_surface_gate(&state, false)
            .expect("a remote caller must be refused even on the household stage");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// The only path that opens either fixture surface: the household's own
    /// declared stage PLUS a loopback caller — `dev_mode` plays no role.
    #[test]
    fn portal_health_loopback_admitted_at_simulacra_stage() {
        let mut state = test_state(false);
        state.network_stage = NetworkStage::Simulacra;
        assert!(
            fixture_surface_gate(&state, true).is_none(),
            "the household's own declared stage plus a loopback caller must open the gate"
        );
    }

    #[tokio::test]
    async fn override_round_trips_through_state_map() {
        // The handler writes into AppState::portal_health_override; assert the
        // map reflects the write so the probe (which reads the same map) sees it.
        let state = test_state(true);
        {
            let mut overrides = state.portal_health_override.write().await;
            overrides.insert("https://matthew.steward.example/account".into(), true);
        }
        let overrides = state.portal_health_override.read().await;
        assert_eq!(
            overrides.get("https://matthew.steward.example/account"),
            Some(&true)
        );
    }

    // ========================================================================
    // PUT /admin/dev/shed
    // ========================================================================

    /// Build an AppState with a specific declared network stage — the crate's
    /// established pattern (`routes/seed.rs::coordinated_stage_retires_...`)
    /// for gate tests that key on `AppState::network_stage`.
    fn shed_test_state(stage: NetworkStage, dev_mode: bool) -> AppState {
        let mut state = test_state(dev_mode);
        state.network_stage = stage;
        state
    }

    /// THE GATE'S WHOLE POINT. Household declares `Simulacra`; every deployed
    /// manifest (alpha included) declares nothing and fail-closes to
    /// `Bootstrap`. `Bootstrap` must NOT open this gate even loopback+dev_mode,
    /// or the fixture would be a public denial-of-service lever on alpha
    /// (`DEV_MODE: "true"` is set there too).
    #[test]
    fn bootstrap_stage_refuses_even_loopback() {
        let state = shed_test_state(NetworkStage::Bootstrap, true);
        let resp = fixture_surface_gate(&state, true).expect("Bootstrap must refuse the shed gate");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// THE REGRESSION PIN for the overruled design: the campaign's original
    /// proposal gated this route on `dev_mode`, exactly like portal-health.
    /// `dev_mode=true` is true on every deployed doorway, including public
    /// alpha — so with the REAL gate (declared stage) closed, dev_mode must
    /// carry no weight at all, in either direction.
    #[test]
    fn dev_mode_true_does_not_open_the_gate_when_stage_is_not_simulacra() {
        let state = shed_test_state(NetworkStage::Bootstrap, true);
        assert!(
            fixture_surface_gate(&state, true).is_some(),
            "dev_mode=true must not substitute for a Simulacra declaration"
        );
    }

    /// A `< Coordinated` comparison (the seed gate's own comparison) would
    /// reopen this on alpha, since alpha's fail-closed `Bootstrap` is also
    /// `< Coordinated`. This gate uses EQUALITY against `Simulacra` — pinned
    /// directly so a future "harmonize with require_seed_authority" edit is
    /// caught here.
    #[test]
    fn bootstrap_is_not_simulacra_even_though_both_precede_coordinated() {
        assert!(NetworkStage::Bootstrap < NetworkStage::Coordinated);
        assert!(NetworkStage::Simulacra < NetworkStage::Coordinated);
        assert_ne!(NetworkStage::Bootstrap, NetworkStage::Simulacra);
        let state = shed_test_state(NetworkStage::Bootstrap, false);
        assert!(fixture_surface_gate(&state, true).is_some());
    }

    #[test]
    fn simulacra_stage_refuses_a_non_loopback_caller() {
        let state = shed_test_state(NetworkStage::Simulacra, false);
        let resp = fixture_surface_gate(&state, false)
            .expect("a remote caller must be refused even on the household stage");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn simulacra_stage_plus_loopback_opens_the_gate() {
        let state = shed_test_state(NetworkStage::Simulacra, false);
        assert!(
            fixture_surface_gate(&state, true).is_none(),
            "the household's own declared stage plus a loopback caller must open the gate"
        );
    }

    /// The gate is a self-contained function of `(state, peer_is_loopback)` —
    /// it takes no body at all, so it structurally cannot be reached AFTER
    /// body parsing; `handle_set_shed` calls it first by construction.
    #[test]
    fn the_gate_takes_no_body_and_so_cannot_run_after_parsing() {
        let state = shed_test_state(NetworkStage::Bootstrap, false);
        // Type-level proof: fixture_surface_gate's signature has no body
        // parameter, so this call can only ever be step (1) in the handler.
        let _: Option<Response<FullBody>> = fixture_surface_gate(&state, true);
    }

    /// The bound the handler installs, asserted on the same constant it hands
    /// `Limited` — no hyper `Incoming` body needed to prove it (mirrors
    /// `p2p_manifests::the_handler_rejects_an_oversized_body_with_413`).
    #[tokio::test]
    async fn oversized_body_is_rejected_by_the_shared_limited_gate() {
        let oversized = Full::new(Bytes::from(vec![b'x'; MAX_FIXTURE_BODY_BYTES + 1]));
        assert!(
            http_body_util::Limited::new(oversized, MAX_FIXTURE_BODY_BYTES)
                .collect()
                .await
                .is_err(),
            "over the cap: the body never reaches serde"
        );

        let ok = Full::new(Bytes::from(vec![b'x'; MAX_FIXTURE_BODY_BYTES]));
        assert!(
            http_body_util::Limited::new(ok, MAX_FIXTURE_BODY_BYTES)
                .collect()
                .await
                .is_ok(),
            "exactly at the cap must still pass"
        );
    }

    #[test]
    fn zero_clears() {
        assert_eq!(apply_shed_request(0, 1_000), ShedRequestOutcome::Cleared);
    }

    #[test]
    fn a_valid_window_is_set_relative_to_now() {
        assert_eq!(
            apply_shed_request(120, 1_000),
            ShedRequestOutcome::Set { until_secs: 1_120 }
        );
    }

    #[test]
    fn the_cap_boundary_is_admitted() {
        assert_eq!(
            apply_shed_request(DEV_SHED_MAX_SECS, 1_000),
            ShedRequestOutcome::Set {
                until_secs: 1_000 + DEV_SHED_MAX_SECS
            }
        );
    }

    #[test]
    fn over_the_cap_is_rejected() {
        assert_eq!(
            apply_shed_request(DEV_SHED_MAX_SECS + 1, 1_000),
            ShedRequestOutcome::TooLarge
        );
        assert_eq!(apply_shed_request(301, 0), ShedRequestOutcome::TooLarge);
    }

    #[test]
    fn remaining_is_some_and_positive_before_the_window_elapses() {
        assert_eq!(shed_remaining_secs(Some(1_120), 1_000), Some(120));
        assert_eq!(shed_remaining_secs(Some(1_120), 1_119), Some(1));
    }

    #[test]
    fn remaining_is_none_at_and_after_the_window() {
        assert_eq!(shed_remaining_secs(Some(1_120), 1_120), None);
        assert_eq!(shed_remaining_secs(Some(1_120), 1_121), None);
    }

    #[test]
    fn remaining_is_none_when_nothing_was_ever_set() {
        assert_eq!(shed_remaining_secs(None, 1_000), None);
    }

    #[tokio::test]
    async fn check_and_clear_reports_no_shed_when_inactive() {
        let state = test_state(false);
        assert_eq!(check_and_clear_at(&state, 1_000).await, None);
    }

    #[tokio::test]
    async fn check_and_clear_reports_remaining_while_the_window_stands() {
        let state = test_state(false);
        *state.dev_shed.write().await = Some(1_120);
        assert_eq!(check_and_clear_at(&state, 1_000).await, Some(120));
        // A mere READ must not have cleared it.
        assert_eq!(*state.dev_shed.read().await, Some(1_120));
    }

    /// THE SELF-CLEARING PROPERTY, PINNED: no further PUT is needed. Setting
    /// an already-elapsed window and checking it (with no clearing call in
    /// between) must both report no shed AND clear the stored state — no
    /// sleep, no clock injection into the state, just a `now` past `until`.
    #[tokio::test]
    async fn an_elapsed_window_self_clears_on_the_next_check_with_no_call() {
        let state = test_state(false);
        *state.dev_shed.write().await = Some(1_000); // elapses at wall-clock second 1_000
        assert_eq!(
            check_and_clear_at(&state, 1_000).await,
            None,
            "the window is exactly elapsed at its own boundary"
        );
        assert_eq!(
            *state.dev_shed.read().await,
            None,
            "reading the elapsed window must have cleared it, with no PUT in between"
        );
    }

    #[tokio::test]
    async fn a_fresh_declaration_replaces_the_standing_window() {
        let state = test_state(false);
        *state.dev_shed.write().await = Some(1_050);
        assert_eq!(
            apply_shed_request(30, 1_000),
            ShedRequestOutcome::Set { until_secs: 1_030 }
        );
        *state.dev_shed.write().await = Some(1_030);
        assert_eq!(check_and_clear_at(&state, 1_000).await, Some(30));
    }

    // ── dev_shed_exempt: what must NEVER be shed by the fixture ────────────

    #[test]
    fn health_stays_exempt_via_the_admission_exempt_carry_through() {
        // http.rs passes admission_exempt(path, is_upgrade) as the second
        // arg; /health already returns true there, so it must carry through.
        assert!(dev_shed_exempt("/health", true));
    }

    #[test]
    fn admin_star_stays_exempt_so_the_fixture_can_be_cleared() {
        assert!(
            dev_shed_exempt("/admin", false),
            "the bare /admin path itself"
        );
        assert!(dev_shed_exempt("/admin/dev/shed", false));
        assert!(dev_shed_exempt("/admin/dev/portal-health", false));
        assert!(dev_shed_exempt("/admin/routes", false));
    }

    /// THE RED-TEAM FINDING, PINNED. `path.starts_with("/admin")` (no slash)
    /// would ALSO exempt an ordinary content path merely spelled with the
    /// same prefix — a story could then "pass" on a path the fixture never
    /// actually shed. Exact-or-slash-delimited only.
    #[test]
    fn an_admin_lookalike_content_path_is_not_exempt() {
        assert!(!dev_shed_exempt("/administration-guide", false));
        assert!(!dev_shed_exempt("/admin-console", false));
        assert!(!dev_shed_exempt("/db/content/administration-basics", false));
    }

    /// Same lookalike class for the other exact-or-slash-delimited prefix.
    #[test]
    fn a_well_known_lookalike_content_path_is_not_exempt() {
        assert!(!dev_shed_exempt("/.well-known-fake", false));
        assert!(!dev_shed_exempt("/well-known/did.json", false));
    }

    #[test]
    fn well_known_stays_exempt() {
        assert!(dev_shed_exempt("/.well-known/did.json", false));
        assert!(dev_shed_exempt("/.well-known/doorway-keys", false));
    }

    #[test]
    fn federation_discovery_and_peer_cache_stay_exempt() {
        assert!(dev_shed_exempt("/api/v1/federation/p2p-peers", false));
        assert!(dev_shed_exempt("/admin/federation/peers", false));
    }

    #[test]
    fn an_ordinary_content_path_is_not_exempt() {
        assert!(!dev_shed_exempt("/db/content/elohim-host-landing", false));
        assert!(!dev_shed_exempt("/blob/sha256-deadbeef", false));
    }

    /// Segment match, not substring — a content slug that merely CONTAINS
    /// "federation" must still be shed while the fixture is active (mirrors
    /// `routes/freshness.rs`'s `has_segment` precedent for the same trap).
    #[test]
    fn federation_as_a_slug_substring_is_not_exempt() {
        assert!(!dev_shed_exempt("/db/content/federation-basics", false));
    }
}
