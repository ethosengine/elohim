//! Demand-driven auto-pin (self-healing opportunity map row 15).
//!
//! # Why this exists
//!
//! The runtime provide-loop (`main.rs`, the 60s provide-author tick) is
//! input-starved fleet-wide: its desired set is derived from
//! `acquisition_pins` rows (`kind='item'`, `status='active'`) via
//! [`crate::services::provide_reconcile::ProvideReconciler::derive_desired`],
//! and the ONLY writer of those rows was `POST /api/v1/pins`. Nothing pins at
//! runtime, so no pod ever authors a `replicates-commons` Commitment on its own
//! — pins were the last missing input to the closed self-healing loop.
//!
//! # What it does (and deliberately does NOT do)
//!
//! On a **confirmed local content read-miss** — a client asked this node for
//! content by id and the local projection does not have it — that IS a demand
//! signal ("want=DevicePin C → REA Commitment A", opportunity-map row 15). This
//! controller authors an `item` DevicePin (`head_ref == content id`) via the
//! existing [`crate::db::acquisition_pins::upsert_pin`] path — the SAME row
//! shape `POST /api/v1/pins` writes (agent `local-device`, kind `item`, born
//! `active`). The acquisition loop then fetches the bytes and the provide loop
//! authors the notarized commitment.
//!
//! This is **demand-driven**, not blanket backfill. It never "pins everything
//! present" and never fires on boot — those are consent-adjacent and explicitly
//! out of scope. A pin is only ever born from a real read someone performed.
//!
//! # Reach-awareness
//!
//! We do **not** reach-gate at pin time. On a local miss we do not have the
//! content (or its reach) at all, so there is nothing to classify here. The
//! DOWNSTREAM provide-eligibility gate
//! (`provide_reconcile::ClassifierEligibility`) is the single source of truth
//! for "may this node provide this?": `derive_desired` only emits a desired
//! provide for a pin whose content is locally present AND provide-eligible. A
//! read-miss for non-providable content therefore costs exactly one inert local
//! row and never produces a commitment.
//!
//! # Off the hot path
//!
//! `upsert_pin` is idempotent, but writing it on every repeated miss for the
//! same id is wasteful. An in-memory throttle window short-circuits BEFORE the
//! DB write, so a burst of misses for one id costs one write per window.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use diesel::sqlite::SqliteConnection;
use diesel::QueryResult;

use crate::db::acquisition_pins;
use crate::db::models::{AcquisitionPin, NewAcquisitionPin};

/// Agent-key placeholder for auto-authored pins — identical to the
/// `POST /api/v1/pins` HTTP route (single-device identity; slice-2 wires the
/// real agent key). Kept in one place so the two writers stay byte-identical.
const AUTOPIN_AGENT: &str = "local-device";
/// Pin kind — item pins resolve trivially (`head_ref` IS the content id).
const AUTOPIN_KIND: &str = "item";
/// Default demand-pin priority (matches the HTTP route's default).
const AUTOPIN_PRIORITY: i32 = 1;

/// Global admission budget: maximum DISTINCT never-seen content ids admitted
/// per throttle window. The per-id throttle alone does not bound an anonymous
/// scanner sweeping DISTINCT junk ids (each would insert a permanent pin row
/// AND fan out into P2P fetch attempts to every connected peer — an
/// unauthenticated amplification surface, joint-verification finding
/// 2026-07-17). With the self-pruning window map, this caps new admissions at
/// a constant per window regardless of scan rate.
const MAX_NEW_ADMISSIONS_PER_WINDOW: usize = 256;

/// Hard ceiling on total ACTIVE pin rows before auto-pinning pauses. The
/// window budget bounds the RATE; this bounds the ACCUMULATION (pins have no
/// TTL — without this, a sustained scan still grows the working set scanned by
/// both 60s reconcile loops monotonically). Generous vs. any legitimate
/// single-node pin count; auto-pinning resumes as pins complete/are removed.
const MAX_ACTIVE_PIN_ROWS: i64 = 2048;

/// Demand auto-pin controller. Holds the enable flag, the throttle window, and
/// an in-memory recently-pinned set. Cheap to `Arc`-share across the HTTP
/// server; the throttle map self-prunes so it stays bounded by the number of
/// distinct content ids demanded within one window.
#[derive(Debug)]
pub struct DemandAutoPin {
    enabled: bool,
    throttle: Duration,
    /// Max distinct never-seen ids admitted per window (DoS budget).
    max_new_per_window: usize,
    /// Max total active pin rows before auto-pinning pauses (DoS ceiling).
    max_active_rows: i64,
    /// content id → last time we upserted a pin for it. Guards the DB write.
    seen: Mutex<HashMap<String, Instant>>,
}

impl DemandAutoPin {
    /// Construct with an explicit enable flag and throttle window (default
    /// DoS budgets — see [`MAX_NEW_ADMISSIONS_PER_WINDOW`] / [`MAX_ACTIVE_PIN_ROWS`]).
    pub fn new(enabled: bool, throttle: Duration) -> Self {
        Self::with_limits(
            enabled,
            throttle,
            MAX_NEW_ADMISSIONS_PER_WINDOW,
            MAX_ACTIVE_PIN_ROWS,
        )
    }

    /// Construct with explicit admission budgets (tests; ops tuning if ever
    /// needed lands here rather than new consts).
    pub fn with_limits(
        enabled: bool,
        throttle: Duration,
        max_new_per_window: usize,
        max_active_rows: i64,
    ) -> Self {
        Self {
            enabled,
            throttle,
            max_new_per_window,
            max_active_rows,
            seen: Mutex::new(HashMap::new()),
        }
    }

    /// A disabled controller — the safe default for constructors that have not
    /// yet been threaded the operator config (e.g. `HttpServer::new`, test
    /// harnesses). `record_demand` is always a no-op until `with_fetch_config`
    /// replaces it with a config-derived instance.
    pub fn disabled() -> Self {
        Self::new(false, Duration::from_secs(0))
    }

    /// Whether auto-pinning is active (used by the wiring site to skip work).
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Return `true` if a pin should be (re)written for `content_id` now,
    /// recording the current instant. Returns `false` when disabled or when the
    /// id was pinned within the throttle window. Prunes expired entries while
    /// holding the lock so the map stays bounded.
    fn admit(&self, content_id: &str) -> bool {
        if !self.enabled {
            return false;
        }
        let now = Instant::now();
        let mut seen = match self.seen.lock() {
            Ok(g) => g,
            // A poisoned mutex here would only affect throttling, never
            // correctness (upsert is idempotent). Fail toward NOT hammering the
            // DB: treat as recently-seen.
            Err(_) => return false,
        };
        // Prune expired entries (bounds memory under wide id churn).
        seen.retain(|_, last| now.duration_since(*last) < self.throttle);
        match seen.get(content_id) {
            Some(last) if now.duration_since(*last) < self.throttle => false,
            _ => {
                // Never-seen id: enforce the per-window admission budget so a
                // distinct-id scan cannot mint unbounded pins (each admitted id
                // costs a permanent row + P2P fetch fan-out). Fail toward
                // not-pinning, mirroring the poisoned-mutex branch.
                if seen.len() >= self.max_new_per_window {
                    return false;
                }
                seen.insert(content_id.to_string(), now);
                true
            }
        }
    }

    /// Record a demand signal for `content_id` (a confirmed local read-miss).
    ///
    /// Authors an `item` DevicePin via the shared idempotent upsert when the
    /// controller is enabled and the id is not throttled; otherwise a no-op.
    ///
    /// Returns `Ok(Some(pin))` when a pin was written, `Ok(None)` when skipped
    /// (disabled or throttled). Only DB errors surface as `Err`; the caller
    /// treats those as non-fatal (the read path continues regardless).
    pub fn record_demand(
        &self,
        conn: &mut SqliteConnection,
        content_id: &str,
    ) -> QueryResult<Option<AcquisitionPin>> {
        if !self.admit(content_id) {
            return Ok(None);
        }
        // Accumulation ceiling: the window budget bounds the RATE of new pins;
        // this bounds the TOTAL active set (pins have no TTL, and both 60s
        // reconcile loops scan every active row each tick). One cheap COUNT on
        // the ≤once-per-id-per-window admitted path only.
        let active_rows = acquisition_pins::count_active_pins(conn)?;
        if active_rows >= self.max_active_rows {
            tracing::warn!(
                active_rows,
                ceiling = self.max_active_rows,
                content_id,
                "demand auto-pin paused: active pin rows at ceiling (DoS guard; \
                 resumes as pins complete or are removed)"
            );
            return Ok(None);
        }
        let pin = acquisition_pins::upsert_pin(
            conn,
            NewAcquisitionPin {
                agent_pub_key: AUTOPIN_AGENT.to_string(),
                head_ref: content_id.to_string(),
                kind: AUTOPIN_KIND.to_string(),
                closure_rule_json: None,
                priority: AUTOPIN_PRIORITY,
            },
        )?;
        Ok(Some(pin))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    use crate::services::provide_reconcile::ProvideReconciler;
    use crate::test_util::test_pool;

    // (i) A demand signal creates an active item pin with the canonical shape.
    #[test]
    fn demand_signal_creates_active_item_pin() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let autopin = DemandAutoPin::new(true, Duration::from_secs(300));

        let pin = autopin
            .record_demand(&mut conn, "epr:demanded-1")
            .expect("upsert must not error")
            .expect("enabled + first-seen must create a pin");

        // Same row shape as the POST /api/v1/pins route.
        assert_eq!(pin.agent_pub_key, "local-device");
        assert_eq!(pin.head_ref, "epr:demanded-1");
        assert_eq!(pin.kind, "item");
        assert_eq!(pin.status, "active");
        assert_eq!(pin.priority, 1);
        assert!(pin.closure_rule_json.is_none());
        assert!(pin.commitment_cid.is_none());

        // It is visible to the provide loop's active-pin loader.
        let active = acquisition_pins::list_active_pins(&mut conn).expect("list_active");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].head_ref, "epr:demanded-1");
    }

    // (ii) Dedupe/throttle: a repeat miss inside the window skips the DB write;
    // once the window elapses (window=0), the same id writes again.
    #[test]
    fn repeated_demand_is_throttled_within_window() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let autopin = DemandAutoPin::new(true, Duration::from_secs(300));

        let first = autopin
            .record_demand(&mut conn, "epr:hot")
            .expect("first upsert");
        assert!(first.is_some(), "first miss must author a pin");

        // Second miss for the same id inside the window is throttled → no write.
        let second = autopin
            .record_demand(&mut conn, "epr:hot")
            .expect("second call");
        assert!(
            second.is_none(),
            "repeat miss within window must be throttled"
        );

        // A different id is not throttled.
        let other = autopin
            .record_demand(&mut conn, "epr:cold")
            .expect("distinct id");
        assert!(other.is_some(), "a distinct id must not be throttled");

        // Still exactly the two distinct pins (the throttled call wrote nothing).
        let active = acquisition_pins::list_active_pins(&mut conn).expect("list_active");
        assert_eq!(active.len(), 2);

        // With a zero-length window the same id is admitted again (window elapsed).
        let no_throttle = DemandAutoPin::new(true, Duration::from_secs(0));
        assert!(no_throttle
            .record_demand(&mut conn, "epr:hot")
            .unwrap()
            .is_some());
        assert!(no_throttle
            .record_demand(&mut conn, "epr:hot")
            .unwrap()
            .is_some());
    }

    // (iii) Knob off → no pin ever authored.
    #[test]
    fn disabled_controller_never_pins() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        for ctor in [
            DemandAutoPin::disabled(),
            DemandAutoPin::new(false, Duration::from_secs(300)),
        ] {
            let out = ctor
                .record_demand(&mut conn, "epr:should-not-pin")
                .expect("call must not error");
            assert!(out.is_none(), "disabled controller must never author a pin");
        }
        let active = acquisition_pins::list_active_pins(&mut conn).expect("list_active");
        assert!(active.is_empty(), "no pins may exist when the knob is off");
    }

    // (iv) The auto-created pin flows into the provide loop's desired set:
    // once its content is present (caught-up proxy) AND provide-eligible,
    // derive_desired emits it — closing want=DevicePin → REA Commitment.
    #[test]
    fn autopinned_head_ref_reaches_provide_desired_set() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let autopin = DemandAutoPin::new(true, Duration::from_secs(300));

        autopin
            .record_demand(&mut conn, "epr:demanded-2")
            .expect("upsert")
            .expect("pin created");

        // Load the active pins exactly as the provide-author tick does.
        let pins = acquisition_pins::list_active_pins(&mut conn).expect("list_active");

        // Simulate the two downstream sets the provide loop computes: content is
        // now locally present (caught-up proxy) and provide-eligible (reach gate).
        let caught_up: HashSet<String> = ["epr:demanded-2".to_string()].into_iter().collect();
        let eligible: HashSet<String> = ["epr:demanded-2".to_string()].into_iter().collect();
        let reach_by_head_ref: HashMap<String, String> = HashMap::new();

        let desired =
            ProvideReconciler::derive_desired(&pins, &caught_up, &eligible, &reach_by_head_ref);
        assert_eq!(
            desired.len(),
            1,
            "the auto-created pin must be desired-to-provide"
        );
        assert_eq!(desired[0].head_ref, "epr:demanded-2");
        assert_eq!(desired[0].reach, "commons"); // missing map → commons back-compat

        // And when NOT eligible, the same pin yields no desired provide (proves
        // the downstream reach gate — not this controller — governs providing).
        let none_eligible: HashSet<String> = HashSet::new();
        let desired_gated = ProvideReconciler::derive_desired(
            &pins,
            &caught_up,
            &none_eligible,
            &reach_by_head_ref,
        );
        assert!(
            desired_gated.is_empty(),
            "ineligible content must not be provided"
        );
    }

    // (v) DoS guard, rate leg: distinct-id scans are capped by the per-window
    // admission budget — the (N+1)th never-seen id inside one window is refused.
    #[test]
    fn distinct_id_scan_is_capped_by_window_budget() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let autopin = DemandAutoPin::with_limits(true, Duration::from_secs(300), 2, 2048);

        assert!(autopin
            .record_demand(&mut conn, "epr:scan-1")
            .unwrap()
            .is_some());
        assert!(autopin
            .record_demand(&mut conn, "epr:scan-2")
            .unwrap()
            .is_some());
        // Budget of 2 exhausted within the window: a third DISTINCT id is refused.
        assert!(
            autopin
                .record_demand(&mut conn, "epr:scan-3")
                .unwrap()
                .is_none(),
            "never-seen id beyond the window budget must be refused"
        );
        // Repeat of an already-admitted id is governed by the throttle, not the
        // budget (still within window → throttled, no extra row either way).
        assert!(autopin
            .record_demand(&mut conn, "epr:scan-1")
            .unwrap()
            .is_none());

        let active = acquisition_pins::list_active_pins(&mut conn).expect("list_active");
        assert_eq!(active.len(), 2, "only budgeted admissions may write rows");
    }

    // (v) DoS guard, accumulation leg: the total-active-rows ceiling pauses
    // auto-pinning even for admitted ids (pins have no TTL; the ceiling bounds
    // the set both reconcile loops scan every tick).
    #[test]
    fn active_row_ceiling_pauses_autopin() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        // Window budget generous; ceiling of 1 active row.
        let autopin = DemandAutoPin::with_limits(true, Duration::from_secs(0), 100, 1);

        assert!(
            autopin
                .record_demand(&mut conn, "epr:first")
                .unwrap()
                .is_some(),
            "below ceiling: pin authored"
        );
        assert!(
            autopin
                .record_demand(&mut conn, "epr:second")
                .unwrap()
                .is_none(),
            "at ceiling: auto-pin must pause"
        );
        let active = acquisition_pins::list_active_pins(&mut conn).expect("list_active");
        assert_eq!(active.len(), 1);
    }
}
