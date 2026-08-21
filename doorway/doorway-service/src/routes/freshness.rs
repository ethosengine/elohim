//! Freshness graded by declared stakes — the doorway's serving-side wiring.
//!
//! The PREDICATE lives in `seam_contracts::freshness` (leaf crate, zero
//! first-party deps, shared with the storage peer that will consume the same
//! matrix for its projector-lag shed). This module is the doorway's half: the
//! declared-stage resolution, the bounded last-good DATA pantry that produces
//! `Available::Amber`, and the advertise surface that must equal what the
//! predicate would decide.
//!
//! ## What changed, and why
//!
//! The prior contract was uniform honest-shed: whenever the upstream circuit
//! opened, EVERY data read returned 503 `catching-up`. On 2026-08-20 that meant
//! elohim.host answered 503 on every `/db/content/*` for hours while holding
//! true, content-addressed bytes it could have served honestly. Being behind is
//! now an amber trust signal on the wire, not a blocker — except where the
//! stakes forbid it (see `ReadClass::is_floor`).
//!
//! ## The pantry is Cat C / Ephemeral (p2p-design-gate)
//!
//! No DHT entry type, no table, no coordinator fn, no route of its own. Every
//! entry is RECONSTRUCTABLE by simply re-fetching the same path from the
//! upstream once it answers again; nothing here is anyone's source of truth,
//! and a restart that empties it costs one upstream round trip per path. It
//! passes the swap test — a sibling doorway serves its OWN last-good bytes for
//! the same path and the two never need to agree, because each is only ever
//! reporting what IT last witnessed.
//!
//! What makes an entry honest rather than merely cached is `served_head`: a
//! CIDv1 (raw codec `0x55`, sha2-256) minted over the exact body bytes, so a
//! consumer can re-derive the address and check that the amber answer is the
//! content it claims to be. That is the C5 leg the predicate itself cannot
//! carry.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use bytes::Bytes;
use cid::Cid;
use hyper::Method;
use multihash_codetable::{Code, MultihashDigest};
use serde::Serialize;
use tracing::{debug, warn};

use seam_contracts::freshness::{requirement, NetworkStage, ReadClass, Requirement};

/// Operator config key for the declared network stage. Read ONCE at AppState
/// construction — never on a request path.
pub const NETWORK_STAKES_ENV: &str = "ELOHIM_NETWORK_STAKES";

/// Per-entry ceiling: a single last-good body larger than this is served
/// through but never stocked. Mirrors the intent of
/// `storage_proxy::BLOB_PANTRY_MAX_BYTES_DEFAULT` at the much smaller scale a
/// JSON projection lives at.
pub const FRESHNESS_PANTRY_MAX_ENTRY_BYTES_DEFAULT: u64 = 1024 * 1024; // 1 MiB

/// Total ceiling across all entries. The pantry is a bounded budget (C6a), not
/// a cache that grows with traffic.
pub const FRESHNESS_PANTRY_MAX_TOTAL_BYTES_DEFAULT: u64 = 64 * 1024 * 1024; // 64 MiB

/// Hard entry-count guard, independent of the byte budget: a flood of tiny
/// bodies must not be able to grow the map without bound either.
pub const FRESHNESS_PANTRY_MAX_ENTRIES: usize = 8_192;

/// Env override key for the per-entry ceiling.
pub const MAX_ENTRY_BYTES_ENV: &str = "FRESHNESS_PANTRY_MAX_ENTRY_BYTES";
/// Env override key for the total ceiling.
pub const MAX_TOTAL_BYTES_ENV: &str = "FRESHNESS_PANTRY_MAX_TOTAL_BYTES";

/// Where the declared [`NetworkStage`] came from.
///
/// Deliberately only two variants at the doorway. elohim-storage's
/// `trust::stage::StakesProvenance` has a third — `Manifest { cid }` — and the
/// doorway CANNOT produce it: reading a standing-policy manifest requires the
/// registry that lives on the storage side. That gap is declared, not faked
/// (C10); see the `catching_up::read_class` registry row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageProvenance {
    /// An explicit, well-formed `ELOHIM_NETWORK_STAKES` declaration.
    OperatorConfig,
    /// Nothing declared, or a declaration that did not parse — the fail-closed
    /// [`NetworkStage::default`] was applied.
    BootstrapDefault,
}

impl StageProvenance {
    /// The wire token the status surface advertises.
    pub fn label(self) -> &'static str {
        match self {
            StageProvenance::OperatorConfig => "operator-config",
            StageProvenance::BootstrapDefault => "bootstrap-default",
        }
    }
}

/// The PURE core of stage resolution: a declaration (or its absence) in, a
/// stage plus its provenance out.
///
/// Split from [`resolve_stage_from_env`] on purpose. An env read on any code
/// path a test can reach invites the `set_var`-leaks-across-parallel-tests
/// flake class (measured on this crate's own `BLOB_PANTRY_MAX_BYTES`), so the
/// decision is tested here and the env is read exactly once, at boot.
///
/// Fail-closed on BOTH absent and unparseable: an unrecognized declaration is
/// never a best-effort guess, and it never yields `Simulacra` — the cheapest
/// stage is reachable only by an exact, positive declaration.
pub fn resolve_stage(declared: Option<&str>) -> (NetworkStage, StageProvenance) {
    match declared {
        None => (NetworkStage::default(), StageProvenance::BootstrapDefault),
        Some(raw) => match raw.parse::<NetworkStage>() {
            Ok(stage) => (stage, StageProvenance::OperatorConfig),
            Err(()) => {
                warn!(
                    declared = %raw,
                    env = NETWORK_STAKES_ENV,
                    "unrecognized network-stakes declaration — falling back to the \
                     fail-closed Bootstrap stage (exact lowercase: simulacra|bootstrap|\
                     coordinated|enforced)"
                );
                (NetworkStage::default(), StageProvenance::BootstrapDefault)
            }
        },
    }
}

/// Read `ELOHIM_NETWORK_STAKES` ONCE. Call from AppState construction only.
pub fn resolve_stage_from_env() -> (NetworkStage, StageProvenance) {
    let declared = std::env::var(NETWORK_STAKES_ENV).ok();
    let (stage, provenance) = resolve_stage(declared.as_deref());
    debug!(
        stage = stage.label(),
        provenance = provenance.label(),
        "resolved declared network stage"
    );
    (stage, provenance)
}

/// One stocked last-good answer.
#[derive(Debug, Clone)]
pub struct PantryEntry {
    /// The exact body bytes the upstream returned.
    pub body: Bytes,
    /// The upstream's `Content-Type`, replayed verbatim.
    pub content_type: String,
    /// Monotonic stamp the age is computed from. `Instant`, not `SystemTime`:
    /// a wall-clock step (NTP, container migration) must not be able to make a
    /// stocked entry look fresher — or older — than it is.
    pub stocked_at: Instant,
    /// The same moment in wall-clock terms, for the `x-elohim-stocked-at`
    /// header a human or a client reads. Advisory; never the age source.
    pub stocked_at_rfc3339: String,
    /// CIDv1, raw codec (`0x55`), sha2-256 over [`Self::body`] — the address a
    /// consumer re-derives to check that amber bytes are the content they claim.
    pub served_head: String,
}

impl PantryEntry {
    /// How long ago this entry was stocked, measured monotonically.
    pub fn age(&self) -> Duration {
        self.stocked_at.elapsed()
    }
}

/// Mint the `served_head` address for a body: CIDv1, raw codec, sha2-256.
///
/// NOT a bare sha256 hex: `/blob/<addr>` already accepts a CID and the doorway
/// already parses one (`routes::blob::parse_content_address`), so the amber
/// answer advertises an address in the same vocabulary the substrate uses.
pub fn mint_served_head(body: &[u8]) -> String {
    let mh = Code::Sha2_256.digest(body);
    Cid::new_v1(0x55, mh).to_string()
}

/// The pantry key: path plus query, because two queries against one path are
/// two different answers.
pub fn pantry_key(path: &str, query: Option<&str>) -> String {
    match query {
        Some(q) if !q.is_empty() => format!("{path}?{q}"),
        _ => path.to_string(),
    }
}

/// Should this response be stocked as last-good?
///
/// PURE, and deliberately conservative on every axis — a wrong `true` here is a
/// consent breach (C12), not a performance regression:
///
/// - **GET only.** A HEAD carries no body to stock; a write is not an answer.
/// - **200 only.** A 206 is partial, a 404 is a real absence, a 5xx is not an
///   answer at all.
/// - **Knowledge / ValueCoupled only.** A floor class can never be answered
///   amber, so stocking it would only ever grow the budget for bytes that can
///   never be served.
/// - **No `Authorization` header.** An authorized response was shaped FOR that
///   bearer. Stocking it under a bearer-blind (path+query) key is exactly how a
///   cache serves one person's answer to another.
/// - **Reach absent / `public` / `commons` only.** The doorway already knows
///   how to read a response's reach (`cache::reach_aware_serving::
///   extract_reach_from_response`); anything narrower than commons is never
///   stocked, so no reach-gated body can be replayed to an unauthenticated
///   caller during a shed.
/// - **Within the per-entry ceiling.**
///
/// **Contract tests:** `stock_predicate_refuses_authorized_responses`,
/// `stock_predicate_refuses_reach_gated_bodies`,
/// `stock_predicate_refuses_floor_classes`, `stock_predicate_accepts_the_
/// public_knowledge_case`.
#[allow(clippy::too_many_arguments)]
pub fn should_stock(
    method: &Method,
    status: u16,
    class: ReadClass,
    has_authorization: bool,
    reach: Option<&str>,
    body_len: usize,
    per_entry_max: u64,
) -> bool {
    if *method != Method::GET {
        return false;
    }
    if status != 200 {
        return false;
    }
    if class.is_floor() {
        return false;
    }
    if has_authorization {
        return false;
    }
    if !reach_is_stockable(reach) {
        return false;
    }
    if body_len == 0 {
        return false;
    }
    body_len as u64 <= per_entry_max
}

/// Absent, `public`, or `commons` — nothing narrower. An UNRECOGNIZED reach
/// token is refused (fail-closed): a reach vocabulary that grows must not
/// silently widen what this pantry will replay.
pub fn reach_is_stockable(reach: Option<&str>) -> bool {
    match reach {
        None => true,
        Some(r) => matches!(r, "public" | "commons"),
    }
}

/// The bounded last-good DATA pantry.
///
/// LRU by insertion/access order with a BYTE budget on top of an entry-count
/// guard: `put` evicts least-recently-used entries until the new body fits, so
/// the store's memory is bounded by declaration rather than by traffic (C6a).
/// Both ceilings are read from the environment ONCE, at construction — not per
/// call. That is a deliberate departure from `blob_pantry_max_bytes()`'s
/// read-per-call style: an env read on a hot path plus a `set_var` in a test is
/// the documented parallel-test flake class in this very crate.
pub struct FreshnessPantry {
    inner: Mutex<lru::LruCache<String, PantryEntry>>,
    bytes: AtomicU64,
    per_entry_max: u64,
    total_max: u64,
}

impl std::fmt::Debug for FreshnessPantry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FreshnessPantry")
            .field("entries", &self.entries())
            .field("bytes", &self.bytes())
            .field("per_entry_max", &self.per_entry_max)
            .field("total_max", &self.total_max)
            .finish()
    }
}

impl Default for FreshnessPantry {
    fn default() -> Self {
        Self::with_limits(
            FRESHNESS_PANTRY_MAX_ENTRY_BYTES_DEFAULT,
            FRESHNESS_PANTRY_MAX_TOTAL_BYTES_DEFAULT,
        )
    }
}

impl FreshnessPantry {
    /// Construct with explicit ceilings (the form tests use).
    pub fn with_limits(per_entry_max: u64, total_max: u64) -> Self {
        let cap = std::num::NonZeroUsize::new(FRESHNESS_PANTRY_MAX_ENTRIES)
            .expect("FRESHNESS_PANTRY_MAX_ENTRIES is a non-zero constant");
        Self {
            inner: Mutex::new(lru::LruCache::new(cap)),
            bytes: AtomicU64::new(0),
            per_entry_max,
            total_max,
        }
    }

    /// Construct from the environment. Call ONCE, at AppState construction.
    pub fn from_env() -> Self {
        fn read(key: &str, default: u64) -> u64 {
            std::env::var(key)
                .ok()
                .and_then(|s| s.parse().ok())
                .filter(|v| *v > 0)
                .unwrap_or(default)
        }
        Self::with_limits(
            read(
                MAX_ENTRY_BYTES_ENV,
                FRESHNESS_PANTRY_MAX_ENTRY_BYTES_DEFAULT,
            ),
            read(
                MAX_TOTAL_BYTES_ENV,
                FRESHNESS_PANTRY_MAX_TOTAL_BYTES_DEFAULT,
            ),
        )
    }

    /// The declared per-entry ceiling (feeds [`should_stock`]).
    pub fn per_entry_max(&self) -> u64 {
        self.per_entry_max
    }

    /// The declared total ceiling.
    pub fn total_max(&self) -> u64 {
        self.total_max
    }

    /// Current stocked entry count.
    pub fn entries(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Current stocked byte total.
    pub fn bytes(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }

    /// Draw the last-good answer for `key`, refreshing its recency.
    pub fn draw(&self, key: &str) -> Option<PantryEntry> {
        let mut guard = self.inner.lock().ok()?;
        guard.get(key).cloned()
    }

    /// Stock a last-good answer, evicting LRU entries until the byte budget
    /// admits it. A body larger than the per-entry ceiling is refused outright
    /// rather than evicting the whole pantry to make room for it.
    pub fn stock(&self, key: String, entry: PantryEntry) {
        let len = entry.body.len() as u64;
        if len > self.per_entry_max || len > self.total_max {
            return;
        }
        let Ok(mut guard) = self.inner.lock() else {
            return;
        };
        // Replacing an existing key reclaims its bytes first.
        if let Some(old) = guard.pop(&key) {
            self.bytes
                .fetch_sub(old.body.len() as u64, Ordering::Relaxed);
        }
        while self.bytes.load(Ordering::Relaxed) + len > self.total_max {
            match guard.pop_lru() {
                Some((_, evicted)) => {
                    self.bytes
                        .fetch_sub(evicted.body.len() as u64, Ordering::Relaxed);
                }
                // Empty and still over budget cannot happen (the per-entry
                // ceiling is <= the total), but never spin on it.
                None => break,
            }
        }
        // The count guard can also evict; reclaim those bytes too.
        if guard.len() == FRESHNESS_PANTRY_MAX_ENTRIES {
            if let Some((_, evicted)) = guard.pop_lru() {
                self.bytes
                    .fetch_sub(evicted.body.len() as u64, Ordering::Relaxed);
            }
        }
        guard.put(key, entry);
        self.bytes.fetch_add(len, Ordering::Relaxed);
    }
}

// ── The advertise surface (C7) ───────────────────────────────────────────────

/// One row of the live policy, as `/status.json` and `/health/serving` render it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessPolicyRow {
    /// `knowledge` | `value` | `authority` | `counter_evidence`.
    pub class: &'static str,
    /// `amber-ok` | `amber-warn` | `green-only`.
    pub requirement: &'static str,
    /// `null` for `green-only` — a floor row has no amber window to state.
    ///
    /// Serialized as an explicit `null` rather than omitted: a consumer
    /// checking `typeof row.maxAgeSecs === 'number' || row.maxAgeSecs === null`
    /// reads an absent key as `undefined` and fails. An advertised contract
    /// that a reader cannot check the same way twice is not a contract.
    pub max_age_secs: Option<u64>,
}

/// The pantry's own bounded-budget report.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessPantryStatus {
    /// Stocked entries right now.
    pub entries: usize,
    /// Stocked bytes right now.
    pub bytes: u64,
    /// The declared total ceiling those bytes are counted against.
    pub max_bytes: u64,
}

/// The `freshness` block on `/status.json` and `/health/serving`.
///
/// **C7:** what this advertises is DERIVED from `seam_contracts::freshness::
/// requirement` — the same function the serving path decides with — rather
/// than restated beside it. `freshness_status_block_matches_the_predicate`
/// pins that equality for every class at every stage, so the surface cannot
/// drift into advertising a policy the doorway does not serve (the exact
/// asymmetry `snapshot()` carried for HalfOpen circuits until 2026-08-21).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessStatus {
    /// The declared stage, as its wire token.
    pub stage: &'static str,
    /// `operator-config` | `bootstrap-default`.
    pub stage_provenance: &'static str,
    /// One row per read class, at the CURRENT stage.
    pub policy: Vec<FreshnessPolicyRow>,
    /// The last-good pantry's occupancy against its declared ceiling.
    pub pantry: FreshnessPantryStatus,
}

/// Render one class's requirement at `stage` as an advertised row.
fn policy_row(class: ReadClass, stage: NetworkStage) -> FreshnessPolicyRow {
    let (requirement_label, max_age_secs) = match requirement(class, stage) {
        Requirement::AmberOk { max_age } => ("amber-ok", Some(max_age.as_secs())),
        Requirement::AmberWarn { max_age } => ("amber-warn", Some(max_age.as_secs())),
        Requirement::GreenOnly => ("green-only", None),
    };
    FreshnessPolicyRow {
        class: class.label(),
        requirement: requirement_label,
        max_age_secs,
    }
}

/// The classes this doorway can actually classify a read into today, and
/// therefore the only ones it may advertise a policy for.
///
/// `ReadClass::CounterEvidence` is deliberately ABSENT. `catching_up::read_class`
/// is route-derived, and no URL path can tell you a read carries
/// counter-evidence — so advertising a requirement for a class this surface can
/// never apply would be the same advertise/serve asymmetry C7 exists to stop,
/// just pointed the other way (claiming a policy rather than hiding one). The
/// class stays named in the predicate, where the EPR-envelope rung will reach
/// it; the day `read_class` can return it, it joins this list and
/// `advertised_policy_covers_every_classifiable_read` starts requiring it.
pub const ADVERTISED_CLASSES: &[ReadClass] = &[
    ReadClass::Knowledge,
    ReadClass::ValueCoupled,
    ReadClass::Authority,
];

/// Build the advertised block from live state.
pub fn status_block(
    stage: NetworkStage,
    provenance: StageProvenance,
    pantry: &FreshnessPantry,
) -> FreshnessStatus {
    FreshnessStatus {
        stage: stage.label(),
        stage_provenance: provenance.label(),
        policy: ADVERTISED_CLASSES
            .iter()
            .map(|&class| policy_row(class, stage))
            .collect(),
        pantry: FreshnessPantryStatus {
            entries: pantry.entries(),
            bytes: pantry.bytes(),
            max_bytes: pantry.total_max(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seam_contracts::freshness::{verdict, Available, FreshnessVerdict};

    fn entry(body: &[u8]) -> PantryEntry {
        PantryEntry {
            body: Bytes::copy_from_slice(body),
            content_type: "application/json".to_string(),
            stocked_at: Instant::now(),
            stocked_at_rfc3339: "2026-08-21T00:00:00Z".to_string(),
            served_head: mint_served_head(body),
        }
    }

    // ── stage resolution ────────────────────────────────────────────────────

    #[test]
    fn absent_declaration_is_the_fail_closed_bootstrap_default() {
        assert_eq!(
            resolve_stage(None),
            (NetworkStage::Bootstrap, StageProvenance::BootstrapDefault)
        );
    }

    #[test]
    fn an_exact_declaration_is_operator_config() {
        for (raw, expected) in [
            ("simulacra", NetworkStage::Simulacra),
            ("bootstrap", NetworkStage::Bootstrap),
            ("coordinated", NetworkStage::Coordinated),
            ("enforced", NetworkStage::Enforced),
        ] {
            assert_eq!(
                resolve_stage(Some(raw)),
                (expected, StageProvenance::OperatorConfig),
                "{raw} must resolve as an operator declaration"
            );
        }
    }

    /// An unparseable declaration must NEVER reach the cheapest stage, and must
    /// not claim operator provenance it does not have.
    #[test]
    fn an_unparseable_declaration_falls_back_fail_closed() {
        for raw in ["Simulacra", "SIMULACRA", "dev", "prod", "", " enforced"] {
            let (stage, provenance) = resolve_stage(Some(raw));
            assert_eq!(
                stage,
                NetworkStage::Bootstrap,
                "{raw:?} must not be honored"
            );
            assert_eq!(
                provenance,
                StageProvenance::BootstrapDefault,
                "{raw:?} must not claim operator-config provenance"
            );
        }
    }

    // ── the stock predicate (C12) ───────────────────────────────────────────

    #[test]
    fn stock_predicate_accepts_the_public_knowledge_case() {
        assert!(should_stock(
            &Method::GET,
            200,
            ReadClass::Knowledge,
            false,
            None,
            1_000,
            FRESHNESS_PANTRY_MAX_ENTRY_BYTES_DEFAULT
        ));
        assert!(should_stock(
            &Method::GET,
            200,
            ReadClass::ValueCoupled,
            false,
            Some("commons"),
            1_000,
            FRESHNESS_PANTRY_MAX_ENTRY_BYTES_DEFAULT
        ));
    }

    /// The consent invariant: a response shaped for a bearer is never stocked
    /// under a bearer-blind key.
    #[test]
    fn stock_predicate_refuses_authorized_responses() {
        assert!(!should_stock(
            &Method::GET,
            200,
            ReadClass::Knowledge,
            true,
            None,
            1_000,
            FRESHNESS_PANTRY_MAX_ENTRY_BYTES_DEFAULT
        ));
    }

    /// Anything narrower than commons — and anything UNRECOGNIZED — is refused.
    #[test]
    fn stock_predicate_refuses_reach_gated_bodies() {
        for reach in [
            "private",
            "local",
            "relationship",
            "household",
            "collective",
            "some-future-reach",
            "",
            "Public",
        ] {
            assert!(
                !should_stock(
                    &Method::GET,
                    200,
                    ReadClass::Knowledge,
                    false,
                    Some(reach),
                    1_000,
                    FRESHNESS_PANTRY_MAX_ENTRY_BYTES_DEFAULT
                ),
                "reach {reach:?} must never be stocked"
            );
        }
        assert!(reach_is_stockable(None));
        assert!(reach_is_stockable(Some("public")));
        assert!(reach_is_stockable(Some("commons")));
    }

    /// A floor class can never be answered amber, so it is never stocked
    /// either — the budget is not spent on bytes that can never be served.
    #[test]
    fn stock_predicate_refuses_floor_classes() {
        for class in [ReadClass::Authority, ReadClass::CounterEvidence] {
            assert!(!should_stock(
                &Method::GET,
                200,
                class,
                false,
                None,
                1_000,
                FRESHNESS_PANTRY_MAX_ENTRY_BYTES_DEFAULT
            ));
        }
    }

    #[test]
    fn stock_predicate_refuses_non_get_non_200_and_oversize() {
        assert!(!should_stock(
            &Method::POST,
            200,
            ReadClass::Knowledge,
            false,
            None,
            10,
            1_000
        ));
        assert!(!should_stock(
            &Method::HEAD,
            200,
            ReadClass::Knowledge,
            false,
            None,
            10,
            1_000
        ));
        for status in [201u16, 204, 206, 301, 304, 404, 429, 500, 503] {
            assert!(!should_stock(
                &Method::GET,
                status,
                ReadClass::Knowledge,
                false,
                None,
                10,
                1_000
            ));
        }
        assert!(!should_stock(
            &Method::GET,
            200,
            ReadClass::Knowledge,
            false,
            None,
            1_001,
            1_000
        ));
        // Empty bodies carry nothing worth replaying.
        assert!(!should_stock(
            &Method::GET,
            200,
            ReadClass::Knowledge,
            false,
            None,
            0,
            1_000
        ));
    }

    // ── the bounded store (C6a) ─────────────────────────────────────────────

    #[test]
    fn stocked_bytes_are_drawable_and_addressed() {
        let p = FreshnessPantry::default();
        p.stock("/db/content/x".to_string(), entry(b"hello"));
        let drawn = p.draw("/db/content/x").expect("stocked entry must draw");
        assert_eq!(drawn.body.as_ref(), b"hello");
        // The address is a CIDv1 raw/sha2-256, re-derivable from the bytes.
        assert_eq!(drawn.served_head, mint_served_head(b"hello"));
        assert!(
            drawn.served_head.starts_with("bafkrei"),
            "served_head must be a CIDv1 raw + sha2-256 address, got {}",
            drawn.served_head
        );
        assert!(p.draw("/db/content/never-stocked").is_none());
    }

    #[test]
    fn pantry_key_separates_queries_on_one_path() {
        assert_eq!(pantry_key("/db/content", None), "/db/content");
        assert_eq!(pantry_key("/db/content", Some("")), "/db/content");
        assert_eq!(
            pantry_key("/db/content", Some("reach=commons")),
            "/db/content?reach=commons"
        );
        assert_ne!(
            pantry_key("/db/content", Some("a=1")),
            pantry_key("/db/content", Some("a=2"))
        );
    }

    /// The declared budget is a real ceiling: stocking past it evicts, and the
    /// byte total never exceeds the declaration.
    #[test]
    fn total_byte_budget_is_never_exceeded() {
        let p = FreshnessPantry::with_limits(100, 250);
        for i in 0..20 {
            p.stock(format!("/k/{i}"), entry(&[b'x'; 100]));
            assert!(
                p.bytes() <= 250,
                "byte total {} exceeded the declared ceiling after {i} stocks",
                p.bytes()
            );
        }
        assert!(
            p.entries() <= 3,
            "entries {} should be LRU-bounded",
            p.entries()
        );
        // The most recent stock survives; the oldest is gone.
        assert!(p.draw("/k/19").is_some());
        assert!(p.draw("/k/0").is_none());
    }

    #[test]
    fn an_oversize_body_is_refused_without_evicting_the_pantry() {
        let p = FreshnessPantry::with_limits(100, 250);
        p.stock("/keep".to_string(), entry(b"small"));
        let before = p.bytes();
        p.stock("/huge".to_string(), entry(&vec![b'x'; 5_000]));
        assert_eq!(p.bytes(), before, "an oversize body must change nothing");
        assert!(
            p.draw("/keep").is_some(),
            "an oversize stock must not evict"
        );
        assert!(p.draw("/huge").is_none());
    }

    #[test]
    fn restocking_a_key_replaces_rather_than_double_counts() {
        let p = FreshnessPantry::with_limits(1_000, 10_000);
        p.stock("/k".to_string(), entry(&vec![b'a'; 300]));
        assert_eq!(p.bytes(), 300);
        p.stock("/k".to_string(), entry(&[b'b'; 100]));
        assert_eq!(
            p.bytes(),
            100,
            "a replace must reclaim the old body's bytes"
        );
        assert_eq!(p.entries(), 1);
    }

    // ── advertise == serve (C7) ─────────────────────────────────────────────

    /// What `/status.json` and `/health/serving` advertise must equal what the
    /// predicate would decide — for EVERY class at EVERY stage, not a sample.
    #[test]
    fn freshness_status_block_matches_the_predicate() {
        let pantry = FreshnessPantry::default();
        let mut checked = 0u32;
        for &stage in &NetworkStage::ALL {
            let block = status_block(stage, StageProvenance::OperatorConfig, &pantry);
            assert_eq!(block.stage, stage.label());
            assert_eq!(block.policy.len(), ADVERTISED_CLASSES.len());
            for &class in ADVERTISED_CLASSES {
                let row = block
                    .policy
                    .iter()
                    .find(|r| r.class == class.label())
                    .unwrap_or_else(|| panic!("no advertised row for {class:?}"));
                // Independently re-derive the expectation from the predicate.
                match requirement(class, stage) {
                    Requirement::AmberOk { max_age } => {
                        assert_eq!(row.requirement, "amber-ok");
                        assert_eq!(row.max_age_secs, Some(max_age.as_secs()));
                        // And the advertised window is the one actually served.
                        assert_eq!(
                            verdict(class, stage, Available::Amber { age: max_age }),
                            FreshnessVerdict::ServeAmber
                        );
                    }
                    Requirement::AmberWarn { max_age } => {
                        assert_eq!(row.requirement, "amber-warn");
                        assert_eq!(row.max_age_secs, Some(max_age.as_secs()));
                        assert_eq!(
                            verdict(class, stage, Available::Amber { age: max_age }),
                            FreshnessVerdict::ServeAmberWarn
                        );
                    }
                    Requirement::GreenOnly => {
                        assert_eq!(row.requirement, "green-only");
                        assert_eq!(
                            row.max_age_secs, None,
                            "a floor row must advertise no amber window"
                        );
                        assert_eq!(
                            verdict(
                                class,
                                stage,
                                Available::Amber {
                                    age: Duration::ZERO
                                }
                            ),
                            FreshnessVerdict::Shed,
                            "advertised green-only must SHED even brand-new amber"
                        );
                    }
                }
                checked += 1;
            }
        }
        assert_eq!(
            checked,
            (ADVERTISED_CLASSES.len() * NetworkStage::ALL.len()) as u32,
            "full advertised-class x stage product, not a sample"
        );
    }

    /// Every class `read_class` can actually return must be advertised — the
    /// other half of C7. A read the doorway classifies but whose policy it never
    /// states would be a silent rule.
    #[test]
    fn advertised_policy_covers_every_classifiable_read() {
        use hyper::Method;
        let probes = [
            ("/db/content/x", ReadClass::Knowledge),
            ("/db/reciprocity", ReadClass::ValueCoupled),
            ("/db/content/x/head-record", ReadClass::Authority),
            ("/unknown-path", ReadClass::Authority),
        ];
        for (path, expected) in probes {
            let class = crate::routes::catching_up::read_class(&Method::GET, path)
                .unwrap_or_else(|| panic!("{path} must classify"));
            assert_eq!(class, expected, "{path}");
            assert!(
                ADVERTISED_CLASSES.contains(&class),
                "{path} classifies as {class:?}, which the status block never advertises"
            );
        }
    }

    /// The exact wire tokens a consumer matches on. Locked as literals — a
    /// rename here breaks an a2o scenario and a dashboard at the same time.
    #[test]
    fn advertised_wire_vocabulary_is_exact() {
        let pantry = FreshnessPantry::default();
        for &stage in &NetworkStage::ALL {
            let block = status_block(stage, StageProvenance::OperatorConfig, &pantry);
            assert!(
                ["simulacra", "bootstrap", "coordinated", "enforced"].contains(&block.stage),
                "stage token {:?} is outside the declared vocabulary",
                block.stage
            );
            let json = serde_json::to_value(&block).expect("status block must serialize");
            let rows = json["policy"].as_array().expect("policy is an array");
            assert_eq!(rows.len(), 3);
            for row in rows {
                let class = row["class"].as_str().expect("class is a string");
                assert!(
                    ["knowledge", "value", "authority"].contains(&class),
                    "class token {class:?} is outside the declared vocabulary"
                );
                let req = row["requirement"]
                    .as_str()
                    .expect("requirement is a string");
                assert!(
                    ["amber-ok", "amber-warn", "green-only"].contains(&req),
                    "requirement token {req:?} is outside the declared vocabulary"
                );
                // Present ALWAYS — a number or an explicit null, never absent.
                let max_age = row
                    .get("maxAgeSecs")
                    .expect("maxAgeSecs must be present, as a number or null");
                assert!(
                    max_age.is_u64() || max_age.is_null(),
                    "maxAgeSecs must be a number or null, got {max_age}"
                );
                if req == "green-only" {
                    assert!(max_age.is_null(), "a floor row advertises a null window");
                } else {
                    assert!(max_age.is_u64(), "an amber row advertises its window");
                }
            }
        }
    }

    #[test]
    fn status_block_reports_the_live_pantry_occupancy() {
        let p = FreshnessPantry::with_limits(1_000, 10_000);
        let empty = status_block(
            NetworkStage::Bootstrap,
            StageProvenance::BootstrapDefault,
            &p,
        );
        assert_eq!(empty.pantry.entries, 0);
        assert_eq!(empty.pantry.bytes, 0);
        assert_eq!(empty.pantry.max_bytes, 10_000);
        assert_eq!(empty.stage_provenance, "bootstrap-default");

        p.stock("/k".to_string(), entry(&[b'a'; 42]));
        let filled = status_block(NetworkStage::Enforced, StageProvenance::OperatorConfig, &p);
        assert_eq!(filled.pantry.entries, 1);
        assert_eq!(filled.pantry.bytes, 42);
        assert_eq!(filled.stage, "enforced");
        assert_eq!(filled.stage_provenance, "operator-config");
    }
}
