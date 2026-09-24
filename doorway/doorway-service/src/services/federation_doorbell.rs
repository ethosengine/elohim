//! Doorbell — story 4.2 slice 1 (`genesis/a2o/reports/recovery/serving-edge-20260919/story-4.2-design.md`).
//!
//! A sibling doorway learns THIS doorway's projection-index change within
//! seconds, without waiting on the existing 60s federation discovery poll
//! (`services::federation::spawn_peer_discovery_task`). Two halves live here:
//!
//! - [`spawn_doorbell_ringer`] — the SENDER: wakes on `EprRouter::changed()`,
//!   debounces, and POSTs a headless `CoherenceDoorbell {doorwayId, digest}`
//!   to every registered peer. Bounded: one pending signal (the router's
//!   `Notify`), no retry, 2s per ring, at most
//!   [`MAX_RING_PEERS`] peers, and a per-peer MUTE on backpressure (503) or an
//!   older sibling (404/405) so a busy or stale peer is not hammered every
//!   round.
//! - [`receive_doorbell`] — the RECEIVER: called from
//!   `routes::coherence::handle_doorbell` after the deaf-fixture gate and body
//!   parse. Resolves the sender through `peer_cache`, checks the digest
//!   against `NameRouteTable::held_digest` (C6b idempotent), and spawns AT
//!   MOST ONE pull-and-install per holder, coalescing a doorbell that arrives
//!   mid-pull into one dirty-bit follow-up ([`DoorbellReceiver`], C6a). The
//!   sender is answered before any network I/O to the holder happens (C5
//!   evidence≠authority — the doorbell itself installs nothing; only the
//!   pulled `GET /api/v1/federation/coherence` at the URL this doorway already
//!   holds can).
//!
//! The 60s poll remains the backstop and the membership authority (§4 "Waits
//! for later slices" — never dropped in this campaign). §3.4's reach filter
//! is explicitly OUT of this slice (operator decision).

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashSet;
use tracing::{debug, info, warn};

use crate::routes::coherence::CoherenceDoorbell;
use crate::server::AppState;
use crate::services::federation::{self, PullOutcome};
use crate::services::name_routing::{
    parse_retry_after_secs, SHED_WINDOW_DEFAULT_SECS, SHED_WINDOW_MAX_SECS, SHED_WINDOW_MIN_SECS,
};

/// At most this many siblings are rung per digest change (C6a bounded work).
pub const MAX_RING_PEERS: usize = 32;
/// Per-peer ring timeout — no retry (design §3.1 step 4).
pub const RING_TIMEOUT: Duration = Duration::from_secs(2);
/// Coalescing debounce after an `EprRouter::changed()` wake, before minting
/// the self-manifest and ringing. A burst of `replace_all` calls inside this
/// window rings once, for the LAST digest.
pub const RING_DEBOUNCE: Duration = Duration::from_millis(250);
/// How long an older sibling that answered 404/405 (this route not yet
/// shipped) is muted from the ring for. Chosen to outlast a rolling deploy
/// without going stale against the discovery poll's own knowledge horizon.
pub const OLD_SIBLING_MUTE: Duration = Duration::from_secs(600);

// ── Metrics vocabulary (C8) ──────────────────────────────────────────────────

/// The sender side of `doorway_federation_doorbell_total{side=...}`.
pub const SIDE_RING: &str = "ring";
/// The receiver side of `doorway_federation_doorbell_total{side=...}`.
pub const SIDE_RECEIVE: &str = "receive";

/// Ring outcome: the peer answered 2xx.
pub const OUTCOME_ACCEPTED: &str = "accepted";
/// Ring outcome: the peer answered 503 — muted for the window it named.
pub const OUTCOME_MUTED_BACKPRESSURE: &str = "muted_backpressure";
/// Ring outcome: the peer answered 404/405 (an older sibling) — muted.
pub const OUTCOME_MUTED_OLD_SIBLING: &str = "muted_old_sibling";
/// Ring outcome: the peer answered something else non-2xx.
pub const OUTCOME_REFUSED: &str = "refused";
/// Both sides: transport error / unreachable.
pub const OUTCOME_UNREACHABLE: &str = "unreachable";
/// Ring outcome: the peer is still muted from an earlier round — never dialed
/// this round (C14 witnessed residual).
pub const OUTCOME_SKIPPED_MUTED: &str = "skipped_muted";
/// Receive outcome: the pull installed a fresh holder snapshot.
pub const OUTCOME_INSTALLED: &str = "installed";
/// Receive outcome: the digest already matched the held one (C6b).
pub const OUTCOME_NOOP: &str = "noop";
/// Receive outcome: the household "deaf" fixture is standing.
pub const OUTCOME_DEAF: &str = "deaf";
/// Receive outcome: the `doorwayId` is not in `peer_cache` (C4).
pub const OUTCOME_UNKNOWN_DOORWAY: &str = "unknown_doorway";
/// Receive outcome: the pulled manifest's self-reported id differed from the
/// doorbell's (C1 anti-self-election).
pub const OUTCOME_ID_MISMATCH: &str = "id_mismatch";
/// Receive outcome: the pull got 200 with an undeserializable body.
pub const OUTCOME_GARBAGE_BODY: &str = "garbage_body";
/// Receive outcome: a pull was already running for this holder — queued
/// behind it via the dirty bit (C6a).
pub const OUTCOME_QUEUED: &str = "queued";

/// The full closed outcome vocabulary, for metric pre-touch. Some values only
/// ever occur on one `side`; pre-touching both is harmless (a handful of
/// always-zero series) and keeps the pre-touch loop trivial.
pub const ALL_OUTCOMES: &[&str] = &[
    OUTCOME_ACCEPTED,
    OUTCOME_MUTED_BACKPRESSURE,
    OUTCOME_MUTED_OLD_SIBLING,
    OUTCOME_REFUSED,
    OUTCOME_UNREACHABLE,
    OUTCOME_SKIPPED_MUTED,
    OUTCOME_INSTALLED,
    OUTCOME_NOOP,
    OUTCOME_DEAF,
    OUTCOME_UNKNOWN_DOORWAY,
    OUTCOME_ID_MISMATCH,
    OUTCOME_GARBAGE_BODY,
    OUTCOME_QUEUED,
];

// ── Receiver-side pull coordination (C6a) ────────────────────────────────────

/// At most one fetch-then-install in flight per holder, plus one dirty bit
/// for a bump that arrived while a pull was already running. A burst of
/// doorbells for the same holder collapses to "pull once more after the one
/// in flight finishes" — never a queue, never unbounded concurrent pulls.
#[derive(Debug, Default)]
pub struct DoorbellReceiver {
    in_flight: DashSet<String>,
    dirty: DashSet<String>,
}

impl DoorbellReceiver {
    pub fn new() -> Self {
        Self::default()
    }

    /// True iff the caller should spawn a pull now. `false` means a pull for
    /// this holder is already running — the dirty bit is set instead, and the
    /// running pull's [`Self::finish_or_repeat`] will pull once more before
    /// releasing the holder.
    fn try_begin(&self, doorway_id: &str) -> bool {
        if self.in_flight.insert(doorway_id.to_string()) {
            true
        } else {
            self.dirty.insert(doorway_id.to_string());
            false
        }
    }

    /// Called after a pull completes. Returns `true` iff a follow-up pull is
    /// owed (the dirty bit was set while this pull ran) — the caller loops.
    /// Returns `false` and releases the holder from `in_flight` otherwise.
    fn finish_or_repeat(&self, doorway_id: &str) -> bool {
        if self.dirty.remove(doorway_id).is_some() {
            true
        } else {
            self.in_flight.remove(doorway_id);
            false
        }
    }
}

// ── Receiver: POST /api/v1/federation/doorbell ───────────────────────────────

/// The pure "what should the caller do" decision from a doorbell, after the
/// deaf gate and body parse. See `routes::coherence::handle_doorbell` for the
/// HTTP mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoorbellReceipt {
    /// The `doorwayId` is not in `peer_cache` (C4 honest absence).
    UnknownDoorway,
    /// The digest already matches the held one — nothing to do (C6b).
    NoOp,
    /// A pull was spawned now.
    Pulling,
    /// A pull is already running for this holder — queued via the dirty bit.
    Queued,
}

/// Household-fixture-only: true while `PUT /admin/dev/federation-deaf` has a
/// standing window (self-clearing). Delegates to
/// `routes::admin_dev::check_and_clear_federation_deaf`, which owns the state
/// and the gate — this module only reads the verdict.
pub async fn is_deaf(state: &Arc<AppState>) -> bool {
    crate::routes::admin_dev::check_and_clear_federation_deaf(state).await
}

/// Resolve, idempotency-check, and (if warranted) spawn ONE background pull
/// for the holder named in `doorbell`. Never awaits the pull itself — the
/// sender is answered before any network I/O to the holder happens (C5).
pub async fn receive_doorbell(
    state: &Arc<AppState>,
    doorbell: CoherenceDoorbell,
) -> DoorbellReceipt {
    let doorway_id = doorbell.doorway_id.clone();

    let peer_url = federation::get_cached_peers(&state.peer_cache)
        .await
        .into_iter()
        .find(|p| p.id == doorway_id)
        .map(|p| p.url);
    let Some(peer_url) = peer_url else {
        crate::metrics::record_doorbell(SIDE_RECEIVE, OUTCOME_UNKNOWN_DOORWAY);
        return DoorbellReceipt::UnknownDoorway;
    };

    if state.name_routes.held_digest(&doorway_id).as_deref() == Some(doorbell.digest.as_str()) {
        crate::metrics::record_doorbell(SIDE_RECEIVE, OUTCOME_NOOP);
        return DoorbellReceipt::NoOp;
    }

    if !state.doorbell_receiver.try_begin(&doorway_id) {
        crate::metrics::record_doorbell(SIDE_RECEIVE, OUTCOME_QUEUED);
        debug!(doorway_id = %doorway_id, "doorbell: pull already in flight — dirty bit set");
        return DoorbellReceipt::Queued;
    }

    let table = Arc::clone(&state.name_routes);
    let client = Arc::clone(&state.doorbell_client);
    let receiver = Arc::clone(&state.doorbell_receiver);
    let spawned_id = doorway_id.clone();
    let doorbell_digest = doorbell.digest.clone();
    tokio::spawn(async move {
        loop {
            let outcome =
                federation::pull_and_install_holder(&table, &spawned_id, &peer_url, &client).await;
            let (metric, label) = match &outcome {
                PullOutcome::Installed => (OUTCOME_INSTALLED, "installed"),
                PullOutcome::IdMismatch {
                    manifest_doorway_id,
                } => {
                    warn!(
                        expected = %spawned_id,
                        manifest_doorway_id = %manifest_doorway_id,
                        "doorbell pull: manifest id differs from the doorbell's — skipped \
                         (the next discovery poll will catch this peer)"
                    );
                    (OUTCOME_ID_MISMATCH, "id-mismatch")
                }
                PullOutcome::Unreachable => (OUTCOME_UNREACHABLE, "unreachable"),
                PullOutcome::GarbageBody => (OUTCOME_GARBAGE_BODY, "garbage-body"),
            };
            crate::metrics::record_doorbell(SIDE_RECEIVE, metric);
            info!(
                doorway_id = %spawned_id,
                digest = %doorbell_digest,
                outcome = label,
                "doorbell pull"
            );
            if !receiver.finish_or_repeat(&spawned_id) {
                break;
            }
        }
    });

    DoorbellReceipt::Pulling
}

// ── Sender: ring on EprRouter digest change ─────────────────────────────────

/// Per-peer mute memory for the ring loop (C10 contract evolution, C11
/// external backpressure): a peer to skip ringing until `mute_until` has
/// passed. Two distinct reasons open a mute — a 503 naming its own window
/// (clamped) and a 404/405 from an OLDER sibling that has not yet shipped
/// this route — but the memory itself carries only the deadline; both cases
/// resolve the same way, by waiting.
#[derive(Default)]
struct RingMutes(dashmap::DashMap<String, Instant>);

impl RingMutes {
    fn is_muted(&self, doorway_id: &str, now: Instant) -> bool {
        self.0.get(doorway_id).is_some_and(|until| now < *until)
    }

    fn mute_for(&self, doorway_id: &str, window: Duration, now: Instant) {
        self.0.insert(doorway_id.to_string(), now + window);
    }
}

/// Clamp a peer's declared `Retry-After` the same way the name-route shed
/// memory does — reusing its bounds rather than inventing a second set for
/// the same shape of number. A present, parseable value is clamped to
/// [`SHED_WINDOW_MIN_SECS`]..=[`SHED_WINDOW_MAX_SECS`] (design §3.1: "clamped
/// 5-300s"); an absent or unparseable one still means "this peer is busy",
/// so it gets [`SHED_WINDOW_DEFAULT_SECS`] — the SAME `ShedReason::NoWindowNamed`
/// convention `ShedMemory::note` already uses for a 503 whose header didn't
/// parse as delta-seconds.
fn clamp_retry_after(raw: Option<&str>) -> Duration {
    let secs = match raw.and_then(parse_retry_after_secs) {
        Some(declared) => declared.clamp(SHED_WINDOW_MIN_SECS, SHED_WINDOW_MAX_SECS),
        None => SHED_WINDOW_DEFAULT_SECS,
    };
    Duration::from_secs(secs)
}

/// Background task: ring every registered sibling within seconds of a local
/// `EprRouter` digest change. See the module doc for the full bounded shape.
/// Started only when `self_doorway_id` is a real id (main.rs mirrors the
/// F-COHERENCE `coherence_self_id` gate) — a doorway with no identity has
/// nothing to ring as.
pub fn spawn_doorbell_ringer(
    epr_router: Arc<crate::projection::EprRouter>,
    peer_cache: federation::PeerCache,
    self_doorway_id: String,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(RING_TIMEOUT)
            .build()
            .unwrap_or_default();
        let mutes = RingMutes::default();
        let mut last_rung: Option<String> = None;

        loop {
            epr_router.changed().notified().await;
            tokio::time::sleep(RING_DEBOUNCE).await;

            let manifest = crate::routes::coherence::router_fingerprint(
                epr_router.as_ref(),
                &self_doorway_id,
                None,
            );
            // A no-op refetch bumps `generation` without changing the digest
            // (design §3.1 step 3) — never ring on that.
            if last_rung.as_deref() == Some(manifest.digest.as_str()) {
                continue;
            }
            let doorbell = CoherenceDoorbell {
                doorway_id: self_doorway_id.clone(),
                digest: manifest.digest.clone(),
            };

            let peers = federation::get_cached_peers(&peer_cache).await;
            let now = Instant::now();
            let mut accepted = 0usize;
            let rung_this_round = peers.len().min(MAX_RING_PEERS);

            for peer in peers.iter().take(MAX_RING_PEERS) {
                if mutes.is_muted(&peer.id, now) {
                    crate::metrics::record_doorbell(SIDE_RING, OUTCOME_SKIPPED_MUTED);
                    continue;
                }
                let url = format!(
                    "{}/api/v1/federation/doorbell",
                    peer.url.trim_end_matches('/')
                );
                match client.post(&url).json(&doorbell).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        accepted += 1;
                        crate::metrics::record_doorbell(SIDE_RING, OUTCOME_ACCEPTED);
                    }
                    Ok(resp) if resp.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE => {
                        let window = clamp_retry_after(
                            resp.headers()
                                .get(reqwest::header::RETRY_AFTER)
                                .and_then(|v| v.to_str().ok()),
                        );
                        mutes.mute_for(&peer.id, window, now);
                        crate::metrics::record_doorbell(SIDE_RING, OUTCOME_MUTED_BACKPRESSURE);
                    }
                    Ok(resp)
                        if resp.status() == reqwest::StatusCode::NOT_FOUND
                            || resp.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED =>
                    {
                        // An OLDER sibling that has not shipped this route yet
                        // (C10 contract evolution). Mute rather than treat as
                        // a genuine 404 — the coherence poll still reaches it.
                        mutes.mute_for(&peer.id, OLD_SIBLING_MUTE, now);
                        crate::metrics::record_doorbell(SIDE_RING, OUTCOME_MUTED_OLD_SIBLING);
                    }
                    Ok(_) => {
                        crate::metrics::record_doorbell(SIDE_RING, OUTCOME_REFUSED);
                    }
                    Err(_) => {
                        crate::metrics::record_doorbell(SIDE_RING, OUTCOME_UNREACHABLE);
                    }
                }
            }

            info!(
                peers = rung_this_round,
                accepted,
                digest = %manifest.digest,
                "doorbell: ring round complete"
            );
            last_rung = Some(manifest.digest);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── DoorbellReceiver: C6a bounded work ────────────────────────────────

    #[test]
    fn first_doorbell_for_a_holder_begins_a_pull() {
        let receiver = DoorbellReceiver::new();
        assert!(receiver.try_begin("alpha-elohim-host"));
    }

    #[test]
    fn a_second_doorbell_while_the_first_pull_is_running_sets_the_dirty_bit_instead() {
        let receiver = DoorbellReceiver::new();
        assert!(receiver.try_begin("alpha-elohim-host"));
        // A pull is already running for this holder — the second doorbell must NOT
        // spawn a second pull; it only marks the holder dirty.
        assert!(!receiver.try_begin("alpha-elohim-host"));
    }

    #[test]
    fn a_burst_of_doorbells_for_the_same_holder_still_collapses_to_one_follow_up_pull() {
        let receiver = DoorbellReceiver::new();
        assert!(receiver.try_begin("alpha-elohim-host"));
        assert!(!receiver.try_begin("alpha-elohim-host"));
        assert!(!receiver.try_begin("alpha-elohim-host"));
        assert!(!receiver.try_begin("alpha-elohim-host"));
        // Exactly ONE follow-up pull is owed, no matter how many doorbells arrived
        // mid-flight — never a queue, never one follow-up per doorbell.
        assert!(receiver.finish_or_repeat("alpha-elohim-host"));
        assert!(!receiver.finish_or_repeat("alpha-elohim-host"));
    }

    #[test]
    fn finishing_with_no_dirty_bit_releases_the_holder_so_a_fresh_doorbell_begins_again() {
        let receiver = DoorbellReceiver::new();
        assert!(receiver.try_begin("alpha-elohim-host"));
        assert!(!receiver.finish_or_repeat("alpha-elohim-host"));
        // The holder was released — a NEW doorbell must be free to begin a fresh pull.
        assert!(receiver.try_begin("alpha-elohim-host"));
    }

    #[test]
    fn two_different_holders_never_interfere() {
        let receiver = DoorbellReceiver::new();
        assert!(receiver.try_begin("alpha-elohim-host"));
        assert!(receiver.try_begin("gamma-elohim-host"));
        assert!(!receiver.try_begin("alpha-elohim-host"));
        assert!(!receiver.try_begin("gamma-elohim-host"));
    }

    // ── RingMutes + clamp_retry_after: C10/C11 ────────────────────────────

    #[test]
    fn an_unmuted_peer_is_never_muted() {
        let mutes = RingMutes::default();
        assert!(!mutes.is_muted("apex-elohim-host", Instant::now()));
    }

    #[test]
    fn a_muted_peer_is_muted_until_the_window_named_elapses() {
        let mutes = RingMutes::default();
        let now = Instant::now();
        mutes.mute_for("apex-elohim-host", Duration::from_secs(30), now);
        assert!(mutes.is_muted("apex-elohim-host", now + Duration::from_secs(29)));
        assert!(!mutes.is_muted("apex-elohim-host", now + Duration::from_secs(31)));
    }

    #[test]
    fn a_fresh_mute_replaces_the_standing_one_rather_than_extending_it() {
        let mutes = RingMutes::default();
        let now = Instant::now();
        mutes.mute_for("apex-elohim-host", Duration::from_secs(300), now);
        mutes.mute_for("apex-elohim-host", Duration::from_secs(10), now);
        assert!(!mutes.is_muted("apex-elohim-host", now + Duration::from_secs(11)));
    }

    #[test]
    fn clamp_retry_after_honours_a_declared_window_within_bounds() {
        assert_eq!(clamp_retry_after(Some("60")), Duration::from_secs(60));
    }

    #[test]
    fn clamp_retry_after_floors_a_window_below_the_minimum() {
        assert_eq!(
            clamp_retry_after(Some("1")),
            Duration::from_secs(SHED_WINDOW_MIN_SECS)
        );
    }

    #[test]
    fn clamp_retry_after_caps_a_window_above_the_maximum() {
        assert_eq!(
            clamp_retry_after(Some("86400")),
            Duration::from_secs(SHED_WINDOW_MAX_SECS)
        );
    }

    #[test]
    fn clamp_retry_after_defaults_an_absent_header_to_the_no_window_named_default() {
        assert_eq!(
            clamp_retry_after(None),
            Duration::from_secs(SHED_WINDOW_DEFAULT_SECS)
        );
    }

    #[test]
    fn clamp_retry_after_defaults_an_unparseable_header_the_same_way() {
        assert_eq!(
            clamp_retry_after(Some("Wed, 21 Oct 2026 07:28:00 GMT")),
            Duration::from_secs(SHED_WINDOW_DEFAULT_SECS)
        );
    }

    // ── ALL_OUTCOMES: C8 observability — the pre-touch vocabulary is closed
    //    and has no accidental duplicate label ────────────────────────────

    #[test]
    fn all_outcomes_has_no_duplicate_label() {
        let mut seen = std::collections::HashSet::new();
        for outcome in ALL_OUTCOMES {
            assert!(seen.insert(*outcome), "duplicate outcome label: {outcome}");
        }
    }
}
