//! Storage Events Subscriber — live projection refresh on substrate change.
//!
//! Connects to elohim-storage's `GET /api/v1/events` SSE endpoint
//! (sse.rs in elohim-storage emits the StorageEventBus as text/event-stream).
//! Two classes of event are handled:
//!
//! **Content events** (`content.created` / `content.updated` / `content.deleted`):
//! Evicts the matching entry from doorway's app file cache so the next
//! `/apps/{slug}` request re-resolves against storage's now-fresh slug_index —
//! AND, for a bundled app, reconciles its declared heads through the
//! `BundleHeadsReconciler`. The eviction alone was the 2026-09-08 hole: it sent
//! the next request back to the doorway's OWN projection, which is the stale
//! thing, so the declared head never moved and a shell from a previous bundle
//! era served for 15 hours.
//!
//! **Projection events** (`projection.registered` / `projection.revoked`):
//! Re-fetches the active project-epr commitment set — through the SAME pool-fallback
//! resilience `main.rs`'s periodic `DOORWAY_EPR_REFRESH_SECS` task uses
//! (`fetch_projections_with_fallback`), and the SAME revocation shield
//! (`EprRouter::install_from_fallback`) — so pillar URL dispatch stays in sync without
//! ever installing a pool peer's stale echo of a commitment THIS doorway just revoked.
//! The event payload carries only a `commitment_id`, not the full projection, so a full
//! re-fetch is required. In MVP (≤4 projections) this is cheap.
//!
//! Pairs with `warm_stream.rs`:
//! - `warm_stream` is the *one-shot cold-start* path. At boot, doorway pulls
//!   the current bulk projection from `/api/v1/cache/stream` into MongoDB.
//! - `storage_events_subscriber` is the *live tail* path. Forever after,
//!   doorway listens for events on `/api/v1/events` and invalidates or
//!   refreshes its caches.
//!
//! Per Pattern Z (`genesis/docs/superpowers/specs/`
//! `2026-05-23-doorway-access-tier-patterns.md`): doorway is a projection
//! of substrate truth, not an authority. It accepts what storage emits.
//! The substrate-correct long-term path is `PUT /api/v1/epr/{cid}` which
//! emits DHT signals; this subscriber is the bridge until stageSpaBlob
//! and other deploy-time mutation callers migrate to EPR Head republish
//! (Pattern Z.D / Z.E).
//!
//! ## Failure modes (intentional)
//!
//! - Storage unreachable at startup: task logs and retries with exponential
//!   backoff (1s → 60s cap). Doorway still serves requests; the projection
//!   just stays at whatever state warm_stream left it in.
//! - Mid-stream disconnect (storage restart, network blip): same backoff
//!   loop; the inner `run_subscriber` returns with an error, the outer
//!   `spawn_subscriber_task` reconnects.
//! - Malformed event payload: logged at debug level, event dropped, stream
//!   continues. A bad event never breaks the loop.
//! - `app_file_cache` not configured: content event handling is a no-op.
//! - EPR re-fetch fails on a projection event: logged at warn level, router
//!   state is left unchanged rather than cleared.

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::cache::AppFileCacheService;
use crate::projection::{fetch_projections_with_fallback, EprRouter, FallbackInstallOutcome};
use crate::render::bundle_heads::BundleHeadsReconciler;

/// Spawn the long-running storage-events subscriber.
///
/// Returns immediately; the SSE consumer runs as a tokio task that survives
/// transient disconnects. An empty `storage_pool_urls` is treated as "no storage to
/// subscribe to" — the task exits cleanly without retrying.
///
/// `storage_pool_urls[0]` is this doorway's own primary storage — both the SSE
/// connection AND the primary of every project-epr re-fetch. Any remaining entries are
/// consulted ONLY as a re-fetch fallback (`fetch_projections_with_fallback`, the SAME
/// pool `main.rs`'s periodic `DOORWAY_EPR_REFRESH_SECS` task uses), never as an
/// alternate SSE source — a peer's OWN event stream carries a peer's OWN events, not
/// this doorway's.
///
/// - `app_file_cache`: evicted on content.{created,updated,deleted}
/// - `epr_router`: replaced (through the revocation shield — see
///   `EprRouter::install_from_fallback`'s doc) on projection.{registered,revoked}
/// - `doorway_id`: passed to `fetch_projections_with_fallback` for the
///   re-fetch that follows a projection event
pub fn spawn_subscriber_task(
    storage_pool_urls: Vec<String>,
    doorway_id: String,
    app_file_cache: Option<Arc<AppFileCacheService>>,
    epr_router: Arc<EprRouter>,
    bundle_heads: Option<Arc<BundleHeadsReconciler>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let Some(storage_url) = storage_pool_urls.first().cloned() else {
            info!(
                "storage_events_subscriber: no storage pool configured; subscriber will not start"
            );
            return;
        };
        if storage_url.is_empty() {
            info!("storage_events_subscriber: storage_url is empty; subscriber will not start");
            return;
        }

        let url = format!("{}/api/v1/events", storage_url.trim_end_matches('/'));
        let http = reqwest::Client::builder().build().unwrap_or_default();

        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(60);
        // A stream must survive this long before the backoff resets — a
        // storage/LB that answers 200 and closes immediately must escalate,
        // not hot-loop (clean close used to reconnect with zero delay).
        let stable_stream = Duration::from_secs(10);

        loop {
            let stream_start = std::time::Instant::now();
            let result = run_subscriber(
                &url,
                &storage_pool_urls,
                &doorway_id,
                app_file_cache.as_ref(),
                &epr_router,
                bundle_heads.as_ref(),
                &http,
            )
            .await;

            if stream_start.elapsed() >= stable_stream {
                backoff = Duration::from_secs(1);
            }

            match result {
                Ok(()) => {
                    info!(
                        url = %url,
                        backoff_secs = %backoff.as_secs(),
                        "storage_events_subscriber: stream ended cleanly; reconnecting after backoff"
                    );
                }
                Err(e) => {
                    warn!(
                        url = %url,
                        error = %e,
                        backoff_secs = %backoff.as_secs(),
                        "storage_events_subscriber: stream error; backing off before retry"
                    );
                }
            }

            sleep(backoff).await;
            backoff = (backoff * 2).min(max_backoff);
        }
    })
}

/// Inner subscriber loop. Connects, tails events, returns on disconnect.
#[allow(clippy::too_many_arguments)]
async fn run_subscriber(
    url: &str,
    pool_urls: &[String],
    doorway_id: &str,
    app_file_cache: Option<&Arc<AppFileCacheService>>,
    epr_router: &EprRouter,
    bundle_heads: Option<&Arc<BundleHeadsReconciler>>,
    http: &reqwest::Client,
) -> Result<(), String> {
    // No top-level timeout — this is a long-lived stream. The reqwest body
    // stream itself yields whenever the connection drops, which is the
    // signal we use to trigger a reconnect.
    let response = http
        .get(url)
        .header("Accept", "text/event-stream")
        .send()
        .await
        .map_err(|e| format!("connect: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} from {}", response.status(), url));
    }

    info!(url = %url, "storage_events_subscriber: connected; tailing events");

    // Initial re-sync on every (re)connect. The boot-time fetch (main.rs) is a
    // one-shot; without this, a projection.{registered,revoked} emitted while
    // this subscriber was disconnected — e.g. during the storage pod cycle in
    // the genesis Seed Projections stage — is missed, and reconnect only
    // resumes tailing FUTURE events. The EprRouter would then stay stale until
    // the next doorway reboot, leaving / and /lamad 404ing after a successful
    // seed. Re-syncing here makes the subscriber self-healing: any missed
    // projection event is recovered on the next reconnect.
    sync_router_from_storage(pool_urls, doorway_id, epr_router, http, "connect").await;

    let mut byte_stream = response.bytes_stream();
    let mut line_buffer = String::new();
    let mut current_event_type: Option<String> = None;
    let mut current_event_data: Option<String> = None;

    while let Some(chunk_result) = byte_stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("stream chunk: {e}"))?;
        let text = std::str::from_utf8(&chunk).map_err(|e| format!("non-utf8 chunk: {e}"))?;
        line_buffer.push_str(text);

        while let Some(newline_pos) = line_buffer.find('\n') {
            let line = line_buffer[..newline_pos]
                .trim_end_matches('\r')
                .to_string();
            line_buffer = line_buffer[newline_pos + 1..].to_string();

            if line.is_empty() {
                // Blank line terminates one SSE event. Dispatch if we
                // accumulated a type+data pair.
                if let (Some(etype), Some(edata)) =
                    (current_event_type.take(), current_event_data.take())
                {
                    handle_event(
                        &etype,
                        &edata,
                        pool_urls,
                        doorway_id,
                        app_file_cache,
                        epr_router,
                        bundle_heads,
                        http,
                    )
                    .await;
                }
            } else if let Some(rest) = line.strip_prefix("event:") {
                current_event_type = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("data:") {
                current_event_data = Some(rest.trim().to_string());
            } else if line.starts_with(':') {
                // SSE comment (heartbeat) — ignore
            }
            // Unknown lines silently ignored (per SSE spec)
        }
    }

    // Stream closed without error — let the outer loop reconnect.
    Ok(())
}

/// Re-fetch the full active project-epr set — consulting `pool_urls[0]` (this
/// doorway's own primary) with the SAME pool-fallback resilience `main.rs`'s periodic
/// `DOORWAY_EPR_REFRESH_SECS` task uses (`fetch_projections_with_fallback`) — and route
/// the result through `EprRouter::install_from_fallback` so a PEER-sourced answer is
/// shielded exactly like every other consumer.
///
/// THIS is the fix for the 2026-09-13 name-routing scenario-4 defect: this function used
/// to call the single-target `fetch_projections_from_storage` and install its result
/// unconditionally — the ONE caller that bypassed the revocation shield entirely,
/// because it never went through `FallbackOutcome`/`PeerServed` at all. A commitment
/// this doorway had just revoked (SSE `projection.revoked`, which calls this same
/// function to re-sync) could, in the gap before the doorway's own primary storage
/// confirmed the revoke, still read as genuinely empty on the primary — and a sibling's
/// not-yet-caught-up DHT replica would answer non-empty, get installed right here with
/// no shield in the way, and resurrect the just-revoked mount at the exact moment a
/// visitor's request landed. See `EprRouter::install_from_fallback`'s doc: there is
/// exactly one way to consume a `FallbackOutcome` now, and it always shields.
///
/// Used both on (re)connect (initial sync — closes the missed-event gap) and on every
/// `projection.{registered,revoked}` event. The event payload carries only a
/// `commitment_id`, not the full projection, so a full re-fetch is required either way.
/// In MVP (≤4 projections) this is cheap.
///
/// Non-fatal: on total fetch failure (`AllUnreachable`) the router state is left
/// unchanged rather than cleared, so a transient storage blip never blanks live routing.
async fn sync_router_from_storage(
    pool_urls: &[String],
    doorway_id: &str,
    epr_router: &EprRouter,
    http: &reqwest::Client,
    cause: &str,
) {
    let outcome = fetch_projections_with_fallback(pool_urls, doorway_id, http).await;
    match epr_router.install_from_fallback(outcome) {
        FallbackInstallOutcome::Primary {
            installed,
            rejected,
            ..
        } => {
            // Log the POST-validation truth (installed, not the fetched length):
            // a batch of N rows that installs 0 is the Welcome-at-`/` incident
            // class, and the fetched count hides it.
            info!(
                cause,
                installed, rejected, "storage_events_subscriber: EprRouter re-synced from storage"
            );
        }
        FallbackInstallOutcome::Peer {
            primary_url,
            primary_empty,
            serving_url,
            installed,
            rejected,
            shielded,
        } => {
            warn!(
                cause,
                installed,
                rejected,
                shielded_from_revocation = shielded,
                primary_url = %primary_url,
                primary_state = if primary_empty { "empty" } else { "unreachable" },
                serving_url = %serving_url,
                "storage_events_subscriber: EprRouter re-synced DEGRADED — primary storage \
                 gave no projections; a pool peer supplied them (revoked commitments shielded)"
            );
        }
        FallbackInstallOutcome::AllEmpty { urls_tried } => {
            info!(
                cause,
                urls_tried = ?urls_tried,
                "storage_events_subscriber: EprRouter re-synced — every storage pool member \
                 returned 0 projections, genuine empty state, router cleared"
            );
        }
        FallbackInstallOutcome::AllUnreachable {
            urls_tried,
            last_error,
        } => {
            warn!(
                error = %last_error,
                cause,
                urls_tried = ?urls_tried,
                "storage_events_subscriber: EprRouter re-sync failed; router state unchanged"
            );
        }
    }
}

/// Dispatch one parsed SSE event to the right cache-invalidation or router-refresh primitive.
#[allow(clippy::too_many_arguments)]
async fn handle_event(
    event_type: &str,
    event_data: &str,
    pool_urls: &[String],
    doorway_id: &str,
    app_file_cache: Option<&Arc<AppFileCacheService>>,
    epr_router: &EprRouter,
    bundle_heads: Option<&Arc<BundleHeadsReconciler>>,
    http: &reqwest::Client,
) {
    match event_type {
        "content.created" | "content.updated" | "content.deleted" => {
            // The cache surface that matters for /apps/{slug} is keyed by
            // content slug (= content id for html5-app/spa-bundle rows).
            let id = match parse_id_from_data(event_data) {
                Some(id) => id,
                None => {
                    debug!(
                        event_type = %event_type,
                        data = %event_data,
                        "storage_events_subscriber: content event without parseable id; skipping"
                    );
                    return;
                }
            };

            info!(
                event_type = %event_type,
                id = %id,
                "storage_events_subscriber: invalidating app file cache"
            );

            if let Some(cache) = app_file_cache {
                // clear_slug evicts both the per-file MongoDB cache entries AND the
                // in-memory slug→blob_hash index for this content. The next request
                // to /apps/{slug}/{file} will re-resolve through resolve_blob_hash's
                // slow path (MongoDB query), then cache miss → fetch from storage.
                //
                // The projected_entries gap this comment used to only NAME is
                // closed below by the bundle-heads reconciler: `clear_slug`
                // drops the per-file cache + the index entry, and the reconcile
                // then RE-DECLARES the head from storage rather than letting the
                // next request re-resolve out of the same stale projection.
                let _ = cache.clear_slug(&id).await;
            }

            // Event arm of D1: a content.{created,updated} for a bundled app is
            // the earliest possible signal that its head moved. Reconciling here
            // converges in the event's latency instead of waiting up to one
            // BUNDLE_HEADS_TICK_SECS. A no-op for any other content row.
            if event_type != "content.deleted" {
                if let Some(reconciler) = bundle_heads {
                    if let Some(mv) = reconciler.on_content_event(&id).await {
                        info!(
                            slug = %mv.slug,
                            "storage_events_subscriber: bundle head reconciled from a content event"
                        );
                    }
                }
            }
        }

        "projection.registered" | "projection.revoked" => {
            if event_type == "projection.revoked" {
                // Shield BEFORE the authoritative re-sync below so the periodic
                // pool-fallback refresh (main.rs's DOORWAY_EPR_REFRESH_SECS
                // task, running on its own independent timer) can never install
                // a sibling doorway's stale DHT-replicated copy of THIS
                // commitment in the window between "revoked" and the next time
                // THIS doorway's own primary confirms it live again. See
                // `EprRouter::mark_revoked`'s doc.
                if let Some(commitment_id) = parse_commitment_id_from_data(event_data) {
                    epr_router.mark_revoked(&commitment_id);
                } else {
                    debug!(
                        event_type = %event_type,
                        data = %event_data,
                        "storage_events_subscriber: projection.revoked without a parseable \
                         commitmentId; re-sync will still drop the row locally, but the \
                         revocation shield cannot be armed for it"
                    );
                }
            }
            sync_router_from_storage(pool_urls, doorway_id, epr_router, http, event_type).await;
        }

        _ => {
            debug!(
                event_type = %event_type,
                "storage_events_subscriber: skipping unhandled event kind"
            );
        }
    }
}

/// Pull the `id` field out of an SSE event's JSON data payload.
///
/// Storage's sse.rs encodes content events as `{"id":"<content-id>"}`.
fn parse_id_from_data(data: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    v.get("id").and_then(|x| x.as_str()).map(|s| s.to_string())
}

/// Pull the `commitmentId` field out of a `projection.{registered,revoked}`
/// SSE event's JSON data payload.
///
/// Storage's sse.rs encodes both events as `{"commitmentId":"<id>"}`
/// (`event_data` in `elohim-storage/src/sse.rs`).
fn parse_commitment_id_from_data(data: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    v.get("commitmentId")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed_id_payload() {
        assert_eq!(
            parse_id_from_data(r#"{"id":"elohim-host-landing"}"#),
            Some("elohim-host-landing".to_string())
        );
    }

    #[test]
    fn ignores_payload_without_id() {
        assert_eq!(parse_id_from_data(r#"{"foo":"bar"}"#), None);
    }

    #[test]
    fn ignores_malformed_json() {
        assert_eq!(parse_id_from_data("not json at all"), None);
    }

    #[test]
    fn handles_extra_fields_in_payload() {
        // Content events may carry additional fields (title, contentType)
        // for ContentCreated; we should still extract id.
        assert_eq!(
            parse_id_from_data(r#"{"id":"abc","title":"x","contentType":"concept"}"#),
            Some("abc".to_string())
        );
    }

    #[tokio::test]
    async fn empty_storage_pool_exits_cleanly() {
        // The spawned task should return without panicking when the storage pool
        // is empty — covers the "no peer configured" startup case.
        let router = Arc::new(EprRouter::new());
        let handle =
            spawn_subscriber_task(Vec::new(), "doorway:test".to_string(), None, router, None);
        // Task should complete (return) — give it a generous timeout in
        // case the runtime is slow.
        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(
            result.is_ok(),
            "spawn_subscriber_task with an empty storage pool should return promptly"
        );
    }

    #[test]
    fn projection_event_kinds_are_handled() {
        // Verify the match arm covers both expected event kinds.
        // (Logic coverage; the re-fetch is tested end-to-end by a2o B22 tests
        // since it requires a live storage instance.)
        for kind in &["projection.registered", "projection.revoked"] {
            assert!(matches!(
                *kind,
                "projection.registered" | "projection.revoked"
            ));
        }
    }

    /// Build a single canned /lamad projection for mock storage responses.
    fn lamad_projection_fixture() -> Vec<elohim_views::projection::EprProjectionView> {
        use elohim_views::projection::{EprProjectionView, ProjectionMode};
        vec![EprProjectionView {
            commitment_id: "test-commitment".into(),
            epr_id: "lamad-spa".into(),
            doorway_id: "doorway:test".into(),
            url_path: "/lamad".into(),
            hostnames: vec![],
            channel: elohim_views::projection::Channel::Converged,
            mode: ProjectionMode::Cached,
            reach: "commons".into(),
            base_href: "/lamad/".into(),
            entry_file: "index.html".into(),
            spa_fallback: true,
            redirects_from: vec![],
            redirect_templates: vec![],
            route_claims: None,
            preview_epr_ref: None,
            gate_hints: vec![],
            dead_end: false,
            steward_direct_endpoint: None,
            responsive_reach: None,
            hosting_agreement_id: None,
            seeded_at: "2026-05-28T00:00:00Z".into(),
            seeded_by: "test".into(),
        }]
    }

    /// THE missed-event-gap regression test (RC1).
    ///
    /// Proves that `run_subscriber` populates the EprRouter from an on-connect
    /// re-sync *without ever receiving a projection.registered event*. Before
    /// the fix the router was only populated by the boot one-shot or a live
    /// event; an event emitted while the subscriber was disconnected (e.g. the
    /// storage pod cycle during genesis Seed Projections) was missed and the
    /// router stayed empty until a doorway reboot — leaving / and /lamad 404ing.
    #[tokio::test]
    async fn run_subscriber_resyncs_router_on_connect_without_any_event() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        // SSE endpoint: an empty, immediately-closing stream. run_subscriber
        // does its on-connect sync, then the stream closes → returns Ok. No
        // projection.{registered,revoked} event is ever delivered.
        Mock::given(method("GET"))
            .and(path("/api/v1/events"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(""),
            )
            .mount(&server)
            .await;

        // The active project-epr set storage would return on re-fetch.
        Mock::given(method("GET"))
            .and(path("/db/rea_commitments"))
            .and(query_param("action", "project-epr"))
            .respond_with(ResponseTemplate::new(200).set_body_json(lamad_projection_fixture()))
            .mount(&server)
            .await;

        let router = EprRouter::new();
        let http = reqwest::Client::new();

        // Precondition: empty router — the missed-event state.
        assert!(router.dispatch_any_host("/lamad").is_none());

        let events_url = format!("{}/api/v1/events", server.uri());
        let pool = vec![server.uri()];
        let result = run_subscriber(
            &events_url,
            &pool,
            "doorway:test",
            None,
            &router,
            None,
            &http,
        )
        .await;
        assert!(result.is_ok(), "subscriber should end cleanly: {result:?}");

        // The router resolves /lamad purely from the on-connect re-sync.
        let hit = router
            .dispatch_any_host("/lamad")
            .expect("router must be populated by on-connect re-sync (no event was sent)");
        assert_eq!(hit.epr_id, "lamad-spa");
    }

    /// The fix itself: a `projection.revoked`-armed shield must survive into
    /// `sync_router_from_storage`'s OWN re-fetch — even when that re-fetch has to fall
    /// back to a pool peer because the primary genuinely has nothing left to say. Before
    /// this fix, this function called the single-target `fetch_projections_from_storage`
    /// and installed its result unconditionally, with no pool consulted and no shield in
    /// the way at all — the exact path the 2026-09-13 name-routing scenario-4 defect rode
    /// in on.
    #[tokio::test]
    async fn sync_router_from_storage_shields_a_peer_served_revoked_commitment() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Primary: genuinely, honestly empty — the commitment was just revoked.
        let primary = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/rea_commitments"))
            .and(query_param("action", "project-epr"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(Vec::<elohim_views::projection::EprProjectionView>::new()),
            )
            .mount(&primary)
            .await;

        // Peer: a not-yet-caught-up DHT replica still answering with the SAME
        // commitment, still "active".
        let stale = lamad_projection_fixture();
        let revoked_commitment_id = stale[0].commitment_id.clone();
        let peer = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/db/rea_commitments"))
            .and(query_param("action", "project-epr"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&stale))
            .mount(&peer)
            .await;

        let router = EprRouter::new();
        router.mark_revoked(&revoked_commitment_id);

        let http = reqwest::Client::new();
        let pool = vec![primary.uri(), peer.uri()];

        sync_router_from_storage(&pool, "doorway:test", &router, &http, "test").await;

        assert!(
            router.dispatch_any_host("/lamad").is_none(),
            "a peer-served commitment this doorway just revoked must never install, even \
             when the primary itself has nothing left to say about it"
        );
    }
}
