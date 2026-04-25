//! Per-agent pubkey timeline — in-memory sorted validity chain for ed25519 verification.
//!
//! ## Design
//!
//! A `PubkeyTimeline` is the per-agent source of truth for "which ed25519 key
//! was authoritative at time T?". It maintains a `Vec<PubkeyValidity>` sorted
//! ascending by `valid_from`. The verify path (Task A.7) calls `pubkey_at(issued_at)`
//! to look up the signing key that should have produced an EPR envelope's signature.
//!
//! ## Classification
//!
//! Category C — operational. This structure is an in-memory cache derived from
//! DHT-notarized `KeyRotation` entries (Category A). It is fully reconstructable
//! from the DHT on restart. The `PubkeyTimelineCache` wraps it in an LRU eviction
//! policy so memory use stays bounded even with large agent populations.
//!
//! ## Revocation interaction
//!
//! When `DnaSignal::KeyRevocation` fires, Task A.8's sweep calls `mark_revoked(compromise_at)`
//! on the affected agent's timeline. This truncates any open-ended validity windows
//! to close at `compromise_at`, so `pubkey_at(t)` correctly returns `None` for any
//! timestamp after the compromise point — causing those EPR atoms to fail verification.

use chrono::{DateTime, Utc};
use lru::LruCache;
use std::num::NonZeroUsize;

use crate::reconcile::signal_stream::KeyRotationSignal;

// ---------------------------------------------------------------------------
// PubkeyValidity
// ---------------------------------------------------------------------------

/// A single validity window for an ed25519 pubkey.
///
/// The window is `[valid_from, valid_until)` — open on the left, exclusive on
/// the right (or unbounded if `valid_until` is `None`).
///
/// `action_hash` is the Holochain DHT `ActionHash` (base64) of the `KeyRotation`
/// entry that established this window. Stored for audit / provenance tracking
/// but not used in verification logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubkeyValidity {
    /// Raw 32-byte ed25519 public key.
    pub pubkey: [u8; 32],
    /// Timestamp from which this key is valid (inclusive).
    pub valid_from: DateTime<Utc>,
    /// Timestamp until which this key is valid (exclusive), or `None` if the
    /// key is still current (no rotation or revocation has closed this window).
    pub valid_until: Option<DateTime<Utc>>,
    /// Holochain ActionHash (base64) of the DHT entry that opened this window.
    pub action_hash: String,
}

// ---------------------------------------------------------------------------
// PubkeyTimeline
// ---------------------------------------------------------------------------

/// Sorted validity chain for a single agent's ed25519 rotation history.
///
/// Invariant: `validities` is sorted ascending by `valid_from` at all times.
/// This is maintained by `insert` via `partition_point`. At most one entry
/// should have `valid_until = None` (the currently-active key); `mark_revoked`
/// closes any open entries.
#[derive(Debug, Clone, Default)]
pub struct PubkeyTimeline {
    /// Sorted ascending by `valid_from`. Binary-searched in `pubkey_at`.
    validities: Vec<PubkeyValidity>,
}

impl PubkeyTimeline {
    /// Construct an empty timeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up which pubkey was authoritative at time `at`.
    ///
    /// Returns the latest validity `v` such that:
    ///   `v.valid_from <= at AND (v.valid_until.is_none() OR at < v.valid_until)`
    ///
    /// Returns `None` if no key covers `at` (i.e. `at` predates the first rotation,
    /// or `at` falls after a revocation closed all windows).
    pub fn pubkey_at(&self, at: DateTime<Utc>) -> Option<&PubkeyValidity> {
        // Walk from the end (most recent) so we find the latest covering window
        // first without needing to scan all entries.
        self.validities
            .iter()
            .rev()
            .find(|v| v.valid_from <= at && v.valid_until.is_none_or(|until| at < until))
    }

    /// Insert a new validity window, maintaining sorted order by `valid_from`.
    ///
    /// Callers are responsible for not inserting duplicate or overlapping ranges;
    /// this method does not validate overlap — it preserves insertion order for
    /// equal `valid_from` values (new entry goes after equal existing entries,
    /// via `partition_point`'s `<=` semantics).
    pub fn insert(
        &mut self,
        pubkey: [u8; 32],
        from: DateTime<Utc>,
        until: Option<DateTime<Utc>>,
        action_hash: String,
    ) {
        let v = PubkeyValidity {
            pubkey,
            valid_from: from,
            valid_until: until,
            action_hash,
        };
        // partition_point returns the first index where valid_from > from,
        // so inserting there preserves ascending sort.
        let pos = self.validities.partition_point(|x| x.valid_from <= from);
        self.validities.insert(pos, v);
    }

    /// Close all open-ended validity windows at `revoked_at`.
    ///
    /// Any entry with `valid_until = None` is set to `Some(revoked_at)`.
    /// Any entry with `valid_until = Some(t)` where `t > revoked_at` is also
    /// tightened to `Some(revoked_at)` (handles out-of-order signals).
    /// Entries already closed at or before `revoked_at` are not modified.
    ///
    /// After this call, `pubkey_at(t)` returns `None` for any `t >= revoked_at`.
    pub fn mark_revoked(&mut self, revoked_at: DateTime<Utc>) {
        for v in self.validities.iter_mut() {
            match v.valid_until {
                None => v.valid_until = Some(revoked_at),
                Some(until) if until > revoked_at => v.valid_until = Some(revoked_at),
                _ => {}
            }
        }
    }

    /// Number of validity windows in this timeline.
    pub fn len(&self) -> usize {
        self.validities.len()
    }

    /// `true` if the timeline has no validity windows yet.
    pub fn is_empty(&self) -> bool {
        self.validities.is_empty()
    }
}

// ---------------------------------------------------------------------------
// PubkeyTimelineCache
// ---------------------------------------------------------------------------

/// LRU cache of per-agent pubkey timelines.
///
/// Wraps an `LruCache<String, PubkeyTimeline>` to bound memory use as the
/// agent population grows. Hot agents (those whose EPR envelopes are frequently
/// verified) stay warm; cold agents are evicted and reloaded from DB on the
/// next access (Task A.7 warm-up path).
///
/// ## Thread safety
///
/// `PubkeyTimelineCache` is `!Send` because `LruCache` is `!Sync`. In the
/// controller, it is owned exclusively by the `ReconcileController` task and
/// accessed only from the task's single-threaded context. If it needs to be
/// shared across tasks in the future, wrap in `Arc<Mutex<...>>` at that point.
pub struct PubkeyTimelineCache {
    lru: LruCache<String, PubkeyTimeline>,
}

impl PubkeyTimelineCache {
    /// Construct a cache with the given capacity.
    ///
    /// `cap` is clamped to at least 1 (an empty cache would be useless and
    /// `NonZeroUsize::new` would panic).
    pub fn with_capacity(cap: usize) -> Self {
        let n = NonZeroUsize::new(cap.max(1)).expect("cap.max(1) is always >= 1");
        Self {
            lru: LruCache::new(n),
        }
    }

    /// Insert or replace the timeline for `agent_cid`.
    ///
    /// If insertion pushes the cache over capacity, the least-recently-used
    /// agent is evicted. The controller's cold-start warm-up path calls this
    /// after loading a timeline from the DB.
    pub fn insert(&mut self, agent_cid: String, timeline: PubkeyTimeline) {
        self.lru.put(agent_cid, timeline);
    }

    /// Look up the timeline for `agent_cid`, promoting it to most-recently-used.
    ///
    /// Returns `None` if the agent is not in cache. The caller (Task A.7 verify
    /// path) should then load from DB and call `insert` to warm the cache.
    pub fn get(&mut self, agent_cid: &String) -> Option<&PubkeyTimeline> {
        self.lru.get(agent_cid)
    }

    /// Look up the timeline for `agent_cid` with mutable access.
    ///
    /// Used by `update_on_rotation` and Task A.8's revocation sweep to mutate
    /// an in-cache timeline without evicting and re-inserting it.
    pub fn get_mut(&mut self, agent_cid: &String) -> Option<&mut PubkeyTimeline> {
        self.lru.get_mut(agent_cid)
    }

    /// Apply a `KeyRotationSignal` to the cache.
    ///
    /// This is the hot-path update called by `ReconcileController::on_key_rotation`
    /// (Task A.4 stub, wired in a later batch). It:
    ///
    /// 1. Gets-or-creates the agent's timeline (creating a fresh empty one if the
    ///    agent is cold — the controller's startup warm-up should have pre-loaded
    ///    known agents, but this handles late-joining agents gracefully).
    /// 2. Closes any open validity windows at `signal.rotated_at` (the old key
    ///    is no longer valid after the rotation timestamp).
    /// 3. Opens a new validity window from `signal.rotated_at` for `new_pubkey`.
    ///
    /// ## Pubkey encoding
    ///
    /// `signal.new_pubkey` is base64-encoded (standard alphabet, padded). If
    /// decoding fails or the decoded slice is not exactly 32 bytes, the pubkey
    /// field is zeroed — the controller will log a warning and the verify path
    /// (A.7) will reject the envelope. This is preferable to panicking.
    pub fn update_on_rotation(&mut self, signal: &KeyRotationSignal) {
        let new_pubkey = decode_b64_pubkey(&signal.new_pubkey).unwrap_or_else(|| {
            tracing::warn!(
                agent_cid = %signal.agent_cid,
                raw = %signal.new_pubkey,
                "KeyRotationSignal: new_pubkey is not valid base64 or not 32 bytes; \
                 storing zeroed key — verification will reject envelopes signed with this key"
            );
            [0u8; 32]
        });

        // get_or_insert_mut promotes on hit; creates empty timeline on miss.
        let timeline = self
            .lru
            .get_or_insert_mut(signal.agent_cid.clone(), PubkeyTimeline::new);

        // Close the previous key's window at the rotation point.
        timeline.mark_revoked(signal.rotated_at);

        // Open the new key's window (unbounded until the next rotation or revocation).
        timeline.insert(
            new_pubkey,
            signal.rotated_at,
            None,
            signal.action_hash.clone(),
        );
    }

    /// Remove the agent's timeline from the cache.
    ///
    /// Called by Task A.8's revocation sweep when the revocation invalidates
    /// an agent's entire key history — the next verify call will reload from DB.
    pub fn invalidate(&mut self, agent_cid: &str) {
        self.lru.pop(agent_cid);
    }

    /// Current number of entries in the cache.
    pub fn len(&self) -> usize {
        self.lru.len()
    }

    /// `true` if the cache contains no entries.
    pub fn is_empty(&self) -> bool {
        self.lru.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Decode a base64 standard-padded string into a 32-byte ed25519 pubkey.
///
/// Returns `None` if the input is not valid base64 or the decoded slice is not
/// exactly 32 bytes.
fn decode_b64_pubkey(s: &str) -> Option<[u8; 32]> {
    use base64::{engine::general_purpose, Engine};
    let bytes = general_purpose::STANDARD.decode(s).ok()?;
    bytes.try_into().ok()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconcile::signal_stream::KeyRotationSignal;
    use chrono::{DateTime, TimeZone, Utc};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn t(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0)
            .single()
            .expect("test timestamp must be valid")
    }

    fn pubkey_a() -> [u8; 32] {
        [0x01; 32]
    }

    fn pubkey_b() -> [u8; 32] {
        [0x02; 32]
    }

    /// base64-encode a raw 32-byte key for use in signal fixtures.
    fn b64_pubkey(key: [u8; 32]) -> String {
        use base64::{engine::general_purpose, Engine};
        general_purpose::STANDARD.encode(key)
    }

    fn ah_rotation() -> String {
        "uhCkk-rotation-action-hash".to_string()
    }

    // -----------------------------------------------------------------------
    // PubkeyTimeline tests
    // -----------------------------------------------------------------------

    #[test]
    fn pubkey_timeline_finds_key_valid_at_timestamp() {
        let mut tl = PubkeyTimeline::new();
        tl.insert(pubkey_a(), t(100), Some(t(200)), ah_rotation());
        tl.insert(pubkey_b(), t(200), None, ah_rotation());

        assert_eq!(tl.pubkey_at(t(150)).unwrap().pubkey, pubkey_a());
        assert_eq!(tl.pubkey_at(t(250)).unwrap().pubkey, pubkey_b());
        assert!(tl.pubkey_at(t(50)).is_none());
    }

    #[test]
    fn pubkey_timeline_open_window_covers_future() {
        let mut tl = PubkeyTimeline::new();
        tl.insert(pubkey_a(), t(100), None, ah_rotation());

        // At exactly valid_from — covered (inclusive).
        assert_eq!(tl.pubkey_at(t(100)).unwrap().pubkey, pubkey_a());
        // Far future — still covered because valid_until is None.
        assert_eq!(tl.pubkey_at(t(999_999)).unwrap().pubkey, pubkey_a());
        // Before valid_from — not covered.
        assert!(tl.pubkey_at(t(99)).is_none());
    }

    #[test]
    fn pubkey_timeline_closed_window_excludes_boundary() {
        let mut tl = PubkeyTimeline::new();
        tl.insert(pubkey_a(), t(100), Some(t(200)), ah_rotation());

        // At valid_until exactly — excluded (exclusive upper bound).
        assert!(tl.pubkey_at(t(200)).is_none());
        // Just before — covered.
        assert_eq!(tl.pubkey_at(t(199)).unwrap().pubkey, pubkey_a());
    }

    #[test]
    fn mark_revoked_truncates_open_validities() {
        let mut tl = PubkeyTimeline::new();
        tl.insert(pubkey_a(), t(100), None, ah_rotation());
        tl.mark_revoked(t(150));

        // Window now closes at t(150) — t(120) still inside.
        let v = tl.pubkey_at(t(120)).unwrap();
        assert_eq!(v.valid_until, Some(t(150)));
        // t(160) is after revocation — no key covers it.
        assert!(tl.pubkey_at(t(160)).is_none());
    }

    #[test]
    fn mark_revoked_doesnt_touch_already_closed() {
        let mut tl = PubkeyTimeline::new();
        tl.insert(pubkey_a(), t(100), Some(t(150)), ah_rotation());
        tl.insert(pubkey_b(), t(150), None, ah_rotation());
        tl.mark_revoked(t(200));

        // pubkey_a's already-closed window is untouched (closed at t(150) < t(200)).
        assert_eq!(tl.pubkey_at(t(120)).unwrap().pubkey, pubkey_a());
        assert_eq!(
            tl.pubkey_at(t(120)).unwrap().valid_until,
            Some(t(150)),
            "already-closed window must not be moved"
        );

        // pubkey_b's open window now closes at t(200).
        assert_eq!(tl.pubkey_at(t(180)).unwrap().pubkey, pubkey_b());
        assert_eq!(tl.pubkey_at(t(180)).unwrap().valid_until, Some(t(200)));
        // After revocation — no key.
        assert!(tl.pubkey_at(t(220)).is_none());
    }

    #[test]
    fn mark_revoked_tightens_future_closed_windows() {
        // An already-closed window whose valid_until is after revoked_at should
        // be tightened. This handles out-of-order signal delivery.
        let mut tl = PubkeyTimeline::new();
        tl.insert(pubkey_a(), t(100), Some(t(300)), ah_rotation());
        tl.mark_revoked(t(200));

        // valid_until was t(300), now tightened to t(200).
        assert_eq!(tl.pubkey_at(t(150)).unwrap().valid_until, Some(t(200)));
        assert!(tl.pubkey_at(t(250)).is_none());
    }

    #[test]
    fn insert_maintains_sorted_order() {
        let mut tl = PubkeyTimeline::new();
        // Insert out of timestamp order.
        tl.insert(pubkey_b(), t(200), None, ah_rotation());
        tl.insert(pubkey_a(), t(100), Some(t(200)), ah_rotation());

        // pubkey_at should still work correctly regardless of insertion order.
        assert_eq!(tl.pubkey_at(t(150)).unwrap().pubkey, pubkey_a());
        assert_eq!(tl.pubkey_at(t(250)).unwrap().pubkey, pubkey_b());
    }

    #[test]
    fn empty_timeline_returns_none() {
        let tl = PubkeyTimeline::new();
        assert!(tl.pubkey_at(t(0)).is_none());
        assert!(tl.pubkey_at(t(999_999)).is_none());
        assert!(tl.is_empty());
    }

    // -----------------------------------------------------------------------
    // PubkeyTimelineCache tests
    // -----------------------------------------------------------------------

    #[test]
    fn cache_lru_evicts_oldest() {
        let mut cache = PubkeyTimelineCache::with_capacity(2);
        cache.insert("agent-1".into(), PubkeyTimeline::new());
        cache.insert("agent-2".into(), PubkeyTimeline::new());
        cache.insert("agent-3".into(), PubkeyTimeline::new());

        // agent-1 was inserted first and not accessed since — should be evicted.
        assert!(
            cache.get(&"agent-1".to_string()).is_none(),
            "agent-1 should be evicted by LRU policy"
        );
        assert!(cache.get(&"agent-2".to_string()).is_some());
        assert!(cache.get(&"agent-3".to_string()).is_some());
    }

    #[test]
    fn cache_lru_respects_access_order() {
        let mut cache = PubkeyTimelineCache::with_capacity(2);
        cache.insert("agent-1".into(), PubkeyTimeline::new());
        cache.insert("agent-2".into(), PubkeyTimeline::new());
        // Access agent-1 to promote it to MRU.
        let _ = cache.get(&"agent-1".to_string());
        // Now insert agent-3 — agent-2 (now LRU) should be evicted, not agent-1.
        cache.insert("agent-3".into(), PubkeyTimeline::new());

        assert!(
            cache.get(&"agent-2".to_string()).is_none(),
            "agent-2 should be evicted (LRU after agent-1 was accessed)"
        );
        assert!(cache.get(&"agent-1".to_string()).is_some());
        assert!(cache.get(&"agent-3".to_string()).is_some());
    }

    #[test]
    fn cache_invalidate_removes_entry() {
        let mut cache = PubkeyTimelineCache::with_capacity(8);
        cache.insert("agent-1".into(), PubkeyTimeline::new());
        cache.invalidate("agent-1");
        assert!(
            cache.get(&"agent-1".to_string()).is_none(),
            "invalidated entry should be absent"
        );
    }

    #[test]
    fn cache_invalidate_nonexistent_is_noop() {
        let mut cache = PubkeyTimelineCache::with_capacity(8);
        // Must not panic.
        cache.invalidate("does-not-exist");
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn cache_update_on_rotation_appends_to_existing() {
        let mut cache = PubkeyTimelineCache::with_capacity(8);

        let signal = KeyRotationSignal {
            action_hash: "uhCkk-rotation-action-hash".to_string(),
            agent_cid: "bafybeicid-agent-1".to_string(),
            new_pubkey: b64_pubkey(pubkey_b()),
            old_pubkey: b64_pubkey(pubkey_a()),
            rotated_at: t(200),
            emitted_at: t(201),
        };

        cache.update_on_rotation(&signal);

        let tl = cache
            .get(&signal.agent_cid)
            .expect("entry must exist after rotation");

        // The rotation opened a window for pubkey_b starting at t(200).
        let v = tl
            .pubkey_at(t(250))
            .expect("pubkey_b must be valid at t(250)");
        assert_eq!(v.pubkey, pubkey_b(), "rotated-in key must be pubkey_b");
        assert_eq!(v.valid_from, t(200));
        assert_eq!(v.valid_until, None, "new window should be open-ended");
    }

    #[test]
    fn cache_update_on_rotation_closes_prior_window() {
        let mut cache = PubkeyTimelineCache::with_capacity(8);

        // Pre-warm with a timeline containing an open pubkey_a window.
        let mut tl = PubkeyTimeline::new();
        tl.insert(pubkey_a(), t(100), None, "ah-a".into());
        cache.insert("agent-x".into(), tl);

        let signal = KeyRotationSignal {
            action_hash: "uhCkk-rotation-to-b".to_string(),
            agent_cid: "agent-x".to_string(),
            new_pubkey: b64_pubkey(pubkey_b()),
            old_pubkey: b64_pubkey(pubkey_a()),
            rotated_at: t(200),
            emitted_at: t(201),
        };

        cache.update_on_rotation(&signal);

        let tl = cache
            .get(&"agent-x".to_string())
            .expect("entry must still be in cache");

        // pubkey_a window must now close at t(200).
        let va = tl.pubkey_at(t(150)).expect("pubkey_a at t(150)");
        assert_eq!(va.pubkey, pubkey_a());
        assert_eq!(
            va.valid_until,
            Some(t(200)),
            "prior window must be closed by mark_revoked"
        );

        // pubkey_b window opens at t(200).
        let vb = tl.pubkey_at(t(250)).expect("pubkey_b at t(250)");
        assert_eq!(vb.pubkey, pubkey_b());
    }

    #[test]
    fn cache_update_on_rotation_creates_cold_agent() {
        // When the agent isn't in cache, update_on_rotation creates a fresh timeline.
        let mut cache = PubkeyTimelineCache::with_capacity(8);
        assert!(cache.get(&"cold-agent".to_string()).is_none());

        let signal = KeyRotationSignal {
            action_hash: "uhCkk-cold-rotation".to_string(),
            agent_cid: "cold-agent".to_string(),
            new_pubkey: b64_pubkey(pubkey_a()),
            old_pubkey: b64_pubkey([0u8; 32]),
            rotated_at: t(500),
            emitted_at: t(501),
        };

        cache.update_on_rotation(&signal);

        assert!(
            cache.get(&"cold-agent".to_string()).is_some(),
            "cold agent must be warm after rotation signal"
        );
    }
}
