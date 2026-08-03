//! Configuration for elohim-storage

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Selects the P2P transport stack used at runtime.
///
/// `Libp2p` (default) keeps the existing libp2p path. `Iroh` selects the
/// parallel iroh-based stack — see `crate::p2p_iroh` and the staged-cutover
/// plan at `genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md`.
/// `Dual` runs BOTH stacks co-resident in one process (dual-stack is the
/// design per the complementarity spec, not a transitional state).
///
/// Runtime selection:
/// - `Libp2p` — only the libp2p node is built (default; fleet continuity).
/// - `Iroh` — only the iroh node is built.
/// - `Dual` — both nodes are built. They share one on-disk `sync.sled`
///   DocStore (Automerge doc state is one truth — the iroh sync path reuses
///   the libp2p node's `SyncManager` Arc rather than opening a second sled
///   lock) and one in-process `DedupLru` (so a message arriving on both
///   planes is deduped once). Disk seams are otherwise disjoint by
///   construction: `identity.key` vs `iroh.key`, `blobs/` vs `blobs_iroh/`,
///   distinct sockets.
///
/// Identity in `Dual` mode: the canonical join key is the Holochain
/// `agent_cid`. Legacy/status fields keep reporting the libp2p `PeerId`
/// (fleet continuity — `self_cid` is derived from the libp2p identity, see
/// the `NodeTransport` seam in `main.rs`); the iroh `NodeId` is INFO-logged
/// at boot and rides `/p2p/status` as an ADDITIONAL optional field once the
/// schema-governed view adds it (never as a replacement).
///
/// Both compile in only if both feature flags (`p2p` + `p2p-iroh`) are
/// enabled; `Dual` degrades to whichever single stack is compiled in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TransportBackend {
    #[default]
    Libp2p,
    Iroh,
    Dual,
}

impl std::str::FromStr for TransportBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "libp2p" => Ok(Self::Libp2p),
            "iroh" => Ok(Self::Iroh),
            "dual" => Ok(Self::Dual),
            other => Err(format!(
                "invalid transport backend '{other}' (expected 'libp2p', 'iroh', or 'dual')"
            )),
        }
    }
}

impl std::fmt::Display for TransportBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Libp2p => f.write_str("libp2p"),
            Self::Iroh => f.write_str("iroh"),
            Self::Dual => f.write_str("dual"),
        }
    }
}

/// Default storage directory
pub fn default_storage_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("elohim-storage")
}

/// Process-wide mirror of [`Config::contest_two_way_declared`], published once
/// by `main` after the config is fully assembled.
///
/// The reconcile sweep (`p2p::projection_reconcile::run_heal`) is reached
/// through several layers that carry no `Config`, and threading one down for a
/// single boolean would touch every caller. A `OnceLock` set at startup is the
/// sanctioned alternative to reading `std::env::var` on the hot path — an env
/// read there makes parallel tests flaky (a `set_var` in one test leaks into
/// another) and re-reads a value that cannot change after boot anyway.
static CONTEST_TWO_WAY_DECLARED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Publish the contest switch for the reconcile sweep. Idempotent: the first
/// call wins, later ones are ignored (`OnceLock::set` semantics), so a test that
/// sets it cannot be clobbered by a second initialisation.
pub fn set_contest_two_way_declared(enabled: bool) {
    let _ = CONTEST_TWO_WAY_DECLARED.set(enabled);
}

/// Is the adopt-before-author CONTEST arm enabled? Defaults to `true` when
/// `main` never published a value (tests, embedded uses) — the same default as
/// the config field, so behaviour cannot diverge between the two homes.
pub fn contest_two_way_declared_enabled() -> bool {
    *CONTEST_TWO_WAY_DECLARED.get().unwrap_or(&true)
}

/// Process-wide mirror of [`Config::adopt_before_author`] — the OPERATOR-RESERVED
/// switch for the both-sides-missing residual. Same `OnceLock` rationale as
/// [`CONTEST_TWO_WAY_DECLARED`].
///
/// Defaults **false** when `main` never published a value, matching the config
/// field, so the two homes cannot diverge and an embedded/test use is dormant
/// exactly like a shipped pod.
static ADOPT_BEFORE_AUTHOR: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Publish the adopt-before-author switch for the reconcile sweep. Idempotent
/// (`OnceLock::set` semantics — the first call wins).
pub fn set_adopt_before_author(enabled: bool) {
    let _ = ADOPT_BEFORE_AUTHOR.set(enabled);
}

/// May this node ask the conductor to declare a canonical head for an id it holds
/// NO local chain for, on validated carried evidence?
///
/// **Default FALSE.** This is shipped DORMANT: the capability is built, tested,
/// and wired, and turning it on is an env-var flip (`ELOHIM_ADOPT_BEFORE_AUTHOR`)
/// rather than a build cycle. Off, every call site is byte-for-byte the prior
/// behaviour — see `services::head_adoption::adopt_before_author_param`.
pub fn adopt_before_author_enabled() -> bool {
    *ADOPT_BEFORE_AUTHOR.get().unwrap_or(&false)
}

/// Process-wide mirror of [`Config::adopt_contest_fanout`]. Same rationale as
/// [`CONTEST_TWO_WAY_DECLARED`]: the reconcile sweep carries no `Config`, and an
/// env read on the hot path is the parallel-test-flake anti-pattern.
static ADOPT_CONTEST_FANOUT: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

/// Process-wide mirror of [`Config::contest_backoff_seconds`].
static CONTEST_BACKOFF_SECONDS: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

/// Publish the adopt-sweep fan-out for the reconcile sweep. Idempotent
/// (`OnceLock::set` semantics).
pub fn set_adopt_contest_fanout(fanout: usize) {
    let _ = ADOPT_CONTEST_FANOUT.set(fanout);
}

/// How many adopt-before-author candidates the sweep may have in flight at once.
///
/// Clamped to at least 1: a zero would make the sweep a silent no-op, which is
/// the one value a throughput knob must never be allowed to mean. `1` is exactly
/// the pre-F-B sequential behaviour, so this is also the OFF switch.
pub fn adopt_contest_fanout() -> usize {
    (*ADOPT_CONTEST_FANOUT
        .get()
        .unwrap_or(&DEFAULT_ADOPT_CONTEST_FANOUT))
    .max(1)
}

/// Publish the contest-backoff window for the reconcile sweep. Idempotent.
pub fn set_contest_backoff_seconds(seconds: u64) {
    let _ = CONTEST_BACKOFF_SECONDS.set(seconds);
}

/// How long a predictable contest failure holds an id back
/// (`services::contest_backoff`). `Duration::ZERO` DISABLES the backoff, which
/// restores the pre-F-B behaviour exactly.
pub fn contest_backoff_window() -> std::time::Duration {
    std::time::Duration::from_secs(
        *CONTEST_BACKOFF_SECONDS
            .get()
            .unwrap_or(&DEFAULT_CONTEST_BACKOFF_SECONDS),
    )
}

/// Conservative default fan-out for the adopt sweep.
///
/// 8, not "as many as fit": every in-flight candidate is a conductor round-trip,
/// and the heal-pacing machinery in `projection_reconcile` exists because a
/// saturated conductor (adam, at its read-pool ceiling) is the live constraint.
/// 8 lifts the per-sweep contest supply roughly 8x while keeping the conductor's
/// instantaneous load hard-bounded and far below its pool.
pub const DEFAULT_ADOPT_CONTEST_FANOUT: usize = 8;

/// Conservative default backoff window: 3600s.
///
/// Deliberately the SAME ~1h dormancy as `projection_reconcile`'s
/// `MISS_READMIT_SWEEPS` (12 sweeps × the 300s reconcile cadence) — one
/// re-attempt horizon at this seam, not a second one with a different shape.
pub const DEFAULT_CONTEST_BACKOFF_SECONDS: u64 = 3600;

/// Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Storage directory for blobs
    #[serde(default = "default_storage_dir")]
    pub storage_dir: PathBuf,

    /// Holochain admin websocket URL
    #[serde(default = "default_admin_url")]
    pub holochain_admin_url: String,

    /// App ID to connect to
    #[serde(default = "default_app_id")]
    pub app_id: String,

    /// DNA role name
    #[serde(default = "default_role_name")]
    pub role_name: String,

    /// Zome name for blob operations
    #[serde(default = "default_zome_name")]
    pub zome_name: String,

    /// Maximum storage size in bytes (0 = unlimited)
    #[serde(default)]
    pub max_storage_bytes: u64,

    /// Enable LRU eviction when max storage reached
    #[serde(default = "default_true")]
    pub enable_eviction: bool,

    /// Minimum replicas before considering eviction
    #[serde(default = "default_min_replicas")]
    pub min_replicas_for_eviction: u32,

    /// Sync interval in seconds (register new blobs in DNA)
    #[serde(default = "default_sync_interval")]
    pub sync_interval_secs: u64,

    /// P2P port for direct blob transfers
    #[serde(default = "default_p2p_port")]
    pub p2p_port: u16,

    /// HTTP API port for shard storage
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// P2P bootstrap nodes for content discovery
    /// Format: /ip4/1.2.3.4/tcp/9876/p2p/12D3KooW...
    #[serde(default)]
    pub p2p_bootstrap_nodes: Vec<String>,

    /// Enable mDNS for local network discovery
    #[serde(default = "default_true")]
    pub enable_mdns: bool,

    /// Extraction cache for HTML5 apps and rendered content
    #[serde(default)]
    pub extraction_cache: elohim_cache_core::extraction::ExtractionCacheConfig,

    /// Path to peer-stewarded availability policy TOML file
    /// (heartbeat cadence, conductor forwarder settings, etc.)
    #[serde(default = "default_peer_policy_path")]
    pub peer_policy_path: PathBuf,

    /// Device archetype id for this node (e.g. "home-nuc", "laptop",
    /// "chromebook-edu"). Loaded from env `DEVICE_ARCHETYPE` at boot.
    /// When set, triggers the node-shape self-registration at startup.
    #[serde(default)]
    pub device_archetype: Option<String>,

    /// Household collective id this node belongs to (e.g. "household-matthew").
    /// Loaded from env `HOUSEHOLD_ID` at boot. Projects into `stewarded_nodes.household_id`.
    #[serde(default)]
    pub household_id: Option<String>,

    /// Node role within the household fabric. Loaded from env `NODE_ROLE`.
    /// One of: edge | archival | inference | doorway.
    #[serde(default)]
    pub node_role: Option<String>,

    /// Geographic region label. Loaded from env `REGION`.
    #[serde(default)]
    pub region: Option<String>,

    /// Cadence for inventory snapshot broadcasts on `elohim/inventory/blob`.
    /// Defaults are archetype-driven (see `inventory_broadcast_seconds_default`).
    /// Operator preset; 4-layer override pattern (archetype → policy.toml → env/CLI → admin trigger).
    #[serde(default)]
    pub inventory_broadcast_seconds: Option<u64>,

    /// Periodic full reconcile-pass cadence for the custody controller.
    #[serde(default = "default_custody_sweep_seconds")]
    pub custody_sweep_seconds: u64,

    /// How long a custody commitment can be unhonored before placement-gap fires.
    #[serde(default = "default_placement_grace_seconds")]
    pub placement_grace_seconds: u64,

    /// Minimum time between repeated placement-gap events for the same commitment.
    #[serde(default = "default_placement_gap_cooldown_seconds")]
    pub placement_gap_cooldown_seconds: u64,

    /// Rate limit on reconciliation-driven fetches per peer.
    #[serde(default = "default_kick_fetch_per_peer_per_minute")]
    pub kick_fetch_per_peer_per_minute: u32,

    /// TTL for peer_blob_inventory entries before they're considered stale.
    #[serde(default = "default_inventory_freshness_seconds")]
    pub inventory_freshness_seconds: u64,

    /// Per-peer timeout for race-fetch blob retrieval (seconds).
    /// Controls how long the GET-time fallback waits for each peer before
    /// marking it as a miss and trying the next candidate in the batch.
    #[serde(default = "default_fetch_blob_timeout_seconds")]
    pub fetch_blob_timeout_seconds: u64,

    /// Maximum number of peer fetch attempts to run in parallel per batch
    /// during race-fetch blob retrieval. First verified reply wins.
    #[serde(default = "default_fetch_blob_parallelism")]
    pub fetch_blob_parallelism: usize,

    /// In-request time budget (milliseconds) for the peer-heal leg of
    /// `get_blob_or_heal`. On a local blob miss the request waits at most this
    /// long for the cross-peer race-fetch + finalize before degrading to a
    /// syncing-status 503 (Retry-After); the fetch itself continues detached in
    /// the background so heal still converges. Bounds the >30s blocking serve
    /// observed live when a healed content pointer references bytes that have
    /// not yet replicated locally. Loaded from env `HEAL_ON_READ_BUDGET_MS`.
    #[serde(default = "default_heal_on_read_budget_ms")]
    pub heal_on_read_budget_ms: u64,

    /// CID of this peer's steward (its agent's content-addressed identity).
    /// Used as `receiver` field in serve-blob REA events emitted on successful
    /// GET-time race-fetch. Loaded from env `SELF_CID` at boot.
    #[serde(default)]
    pub self_cid: Option<String>,

    /// Selects the P2P transport stack at runtime — see [`TransportBackend`].
    /// Loaded from `ELOHIM_TRANSPORT_BACKEND` env or `transport_backend` TOML
    /// key. Defaults to `Libp2p` so existing deployments are unaffected.
    #[serde(default)]
    pub transport_backend: TransportBackend,

    /// GENESIS BOOTSTRAP STOPGAP — gate for the self-heal-identity act.
    /// When true (env `GENESIS_SELF_HEAL_IDENTITY=1`), at boot the node fills
    /// its own configured human's NULL `humans.agent_pub_key` (and
    /// `household_id`) from its OWN conductor cell key, and clears the no-session
    /// 401 gate so the provide-rows seeder's `/auth/me` succeeds. Default
    /// **false** — this is an operator-authorized act, never on by default.
    /// Superseded by the cross-signed `coherent-transport-identity-resolver`.
    #[serde(default)]
    pub genesis_self_heal_identity: bool,

    /// GENESIS BOOTSTRAP STOPGAP — the stable `humans.id` (slug) this pod
    /// self-heals when `genesis_self_heal_identity` is set. Loaded from env
    /// `SELF_HUMAN_ID` (e.g. `human-matthew-manager`). Only this one row is
    /// touched; a missing row is a logged skip, not an error.
    #[serde(default)]
    pub self_human_id: Option<String>,

    /// GENESIS BOOTSTRAP STOPGAP — the household collective id healed into the
    /// configured human's NULL `humans.household_id`. Loaded from env
    /// `SELF_HOUSEHOLD_ID`; falls back to `household_id` (env `HOUSEHOLD_ID`)
    /// when unset. Load-bearing: the resilience snapshot's collective join
    /// filters out rows with a NULL `household_id`, so without it the card
    /// stays dark even after the agent key is healed
    /// (`services/household_resilience.rs:194-195`).
    #[serde(default)]
    pub self_household_id: Option<String>,

    /// P3 salvage: opt-in consent gate for Good-Samaritan salvage custody.
    /// Default **false** — a node is NEVER conscripted (the imago-dei floor);
    /// salvage is an enhancement a node offers, never a participation
    /// precondition (the hub-optional floor). When true AND the node is an
    /// always-on archetype (`node`/`steward`), it advertises spare capacity and
    /// the salvage pass may author custody-blob commitments naming self.
    /// Loaded from env `SALVAGE_CAPACITY_ENABLED`.
    #[serde(default)]
    pub salvage_capacity_enabled: bool,

    /// P3 salvage: desired distinct-holder count per blob. Default = the
    /// `min_replicas_for_eviction` value (2). The salvage pass treats a blob
    /// with fewer fresh honored providers than this as under-replicated.
    #[serde(default = "default_salvage_target_replicas")]
    pub salvage_target_replicas: usize,

    /// P3 salvage: cadence (seconds) for the salvage-capacity advertisement
    /// broadcast and the salvage recheck sweep. Default 300s (5 min).
    #[serde(default = "default_salvage_recheck_seconds")]
    pub salvage_recheck_seconds: u64,

    /// P3-8 salvage: select the diversity-aware placement strategy
    /// (`DiversityAwarePlacementStrategy`) over the MVP `XorDistanceStrategy`
    /// when picking salvage re-placement holders. Default **true** — the
    /// strategy degrades EXACTLY to XOR when household data is absent (slice 1a),
    /// so it is never worse than XOR and strictly better once households are
    /// known; the knob lets ops roll back to plain XOR deliberately.
    /// Loaded from env `SALVAGE_DIVERSITY_PLACEMENT`.
    #[serde(default = "default_true")]
    pub salvage_diversity_placement: bool,

    /// Adopt-before-author CONTEST arm: when this row and a peer BOTH declare a
    /// head, and this row's declaration has NO canonical election behind it,
    /// mint a canonical-head declaration link naming the peer's head so the
    /// DHT's own arbiter (`select_canonical_winner`) has something to elect on.
    ///
    /// Default **true** — this is the supply side of the election, and without
    /// it the two-way-declared class is structurally stuck (every peer holds a
    /// declaration, so the `AdoptPeer` arm — which requires an UNDECLARED local
    /// row — never fires and no election is ever created). The knob exists as a
    /// per-pod OFF switch: turning it off returns EXACTLY the prior `Hold`
    /// behaviour, which is safe but non-converging.
    ///
    /// Contesting never stamps a row and never widens `Declare`. It only creates
    /// the DHT evidence; the winner is chosen on the DHT and obeyed by the
    /// existing `HealCanonical` path.
    /// Loaded from env `CONTEST_TWO_WAY_DECLARED`.
    #[serde(default = "default_true")]
    pub contest_two_way_declared: bool,

    /// ADOPT-BEFORE-AUTHOR (operator-reserved): may this node ask the conductor
    /// to declare a canonical head for an id it holds **no local chain** for,
    /// backed by a carried record the zome proves in wasm?
    ///
    /// Default **false**, and deliberately so. `contest_two_way_declared` above
    /// is default-ON because a node with a chain contesting its own view is a
    /// safe supply act. This is a different question: it lets a node that has
    /// never held an id enter the election for it. The mechanism is
    /// evidence-bound and re-derived (`content_store::classify_chain_gate` opens
    /// only with BOTH the opt-in and a record that passes every carried-record
    /// clause plus the target-id gate), but WHETHER the fleet should participate
    /// that way is an operator decision, not a default.
    ///
    /// ## The residual it exists for
    ///
    /// The zome's no-chain gate is target-INDEPENDENT, so when both sides of a
    /// divergent pair lack a chain, neither can mint a candidate and the DHT
    /// arbiter never runs. `contest_failed{no_local_chain}` is that class, and it
    /// dominates non-progress. This flag is its only automated exit; with it off
    /// the residual stays open (asserted in `liveness_contract`).
    ///
    /// ## Off is behaviour-identical
    ///
    /// Every call site passes the zome param as `flag && carried_bytes_in_hand`,
    /// so `false` yields the exact pre-flag conductor payload — and the zome
    /// itself refuses identically on `adopt_before_author: false`.
    /// Loaded from env `ELOHIM_ADOPT_BEFORE_AUTHOR`.
    #[serde(default)]
    pub adopt_before_author: bool,

    /// F-B THROUGHPUT LEVER, half 1: how many adopt-before-author candidates the
    /// reconcile sweep processes CONCURRENTLY.
    ///
    /// The sweep's budget is fixed (200 candidates/tick, 120s wall clock, ~300s
    /// cadence) and the binding constraint is the wall clock, not the cap: 200
    /// sequential conductor round-trips do not fit in 120s. Fan-out converts the
    /// budget into ~N× the contest supply per sweep.
    ///
    /// `1` restores the exact pre-F-B sequential sweep (the OFF switch); the
    /// per-item 25ms spacing is retained inside each concurrent task so a
    /// fan-out of 1 is byte-for-byte the old behaviour. Raise with care: each
    /// in-flight candidate is a conductor round-trip and a saturated conductor
    /// is the live constraint this whole pacing layer exists for.
    /// Loaded from env `ADOPT_CONTEST_FANOUT`.
    #[serde(default = "default_adopt_contest_fanout")]
    pub adopt_contest_fanout: usize,

    /// F-B THROUGHPUT LEVER, half 2: how long (seconds) a PREDICTABLE contest
    /// failure holds an id back from re-contesting
    /// (`services::contest_backoff`).
    ///
    /// A contest that failed the zome's target-independent no-chain gate cannot
    /// succeed on a later sweep either — until this conductor acquires a chain
    /// for the id — so re-attempting it every sweep burns budget that productive
    /// contests need. The backoff is never permanent: it expires on this window
    /// AND is cleared immediately when an author path lands a local chain.
    ///
    /// `0` DISABLES the backoff (every candidate contested every sweep — the
    /// pre-F-B behaviour). Default 3600 = 12 sweeps at the 300s cadence, the
    /// same ~1h horizon as `MISS_READMIT_SWEEPS`.
    /// Loaded from env `CONTEST_BACKOFF_SECONDS`.
    #[serde(default = "default_contest_backoff_seconds")]
    pub contest_backoff_seconds: u64,

    /// Demand-driven auto-pin: when a local content read MISSES (a client asked
    /// this node for content it does not have), author an `item` DevicePin for
    /// that content so the acquisition loop fetches it and the provide loop
    /// authors a `replicates-commons` Commitment. This closes the runtime gap
    /// where NOTHING pinned at runtime, leaving every pod's provide-loop
    /// input-starved (self-healing opportunity map row 15).
    ///
    /// Default **true**, chosen deliberately. This is *demand-driven*, not
    /// blanket backfill: a pin is only ever born from a real read someone
    /// performed (never on boot, never "pin everything present" — those are
    /// consent-adjacent and out of scope). The imago-dei consent floor that
    /// makes `salvage_capacity_enabled` default OFF does not apply here — that
    /// knob conscripts a node to hold OTHER peers' content as a Good-Samaritan;
    /// this one only reacts to THIS node's own demand. Reach-safety is preserved
    /// downstream, not here: the provide-eligibility gate
    /// (`provide_reconcile::ClassifierEligibility`) still decides whether any
    /// commitment is ever authored, so a non-providable read-miss costs exactly
    /// one inert local row. Shipping ON is what actually closes the loop — a
    /// dark default-OFF feature that no operator flips stays inert.
    ///
    /// To disable: set env `DEMAND_AUTOPIN_ENABLED` to `0`/`false`/`off` (or
    /// `demand_autopin_enabled = false` in config.toml). Loaded from env
    /// `DEMAND_AUTOPIN_ENABLED`.
    #[serde(default = "default_true")]
    pub demand_autopin_enabled: bool,

    /// Demand auto-pin throttle window (seconds): repeated read-misses for the
    /// same content id within this window skip the idempotent DB upsert (the pin
    /// already exists; rewriting it per request is wasteful). Keeps the pin
    /// write off the read hot path. Default 300s (5 min). Loaded from env
    /// `DEMAND_AUTOPIN_THROTTLE_SECONDS`.
    #[serde(default = "default_demand_autopin_throttle_seconds")]
    pub demand_autopin_throttle_seconds: u64,

    /// Custody manifest + self-held evidence backfill
    /// (`reconcile::custody::manifest_backfill_pass`): enumerates the local
    /// blob store directly and, for blobs missing a `shard_manifests` row
    /// (legacy blobs that predate the manifest producer, commit 123fd4bd5),
    /// derives + persists the manifest and records self-held
    /// `shard_locations` evidence for shards this node verifiably holds — no
    /// fresh PUT or `distribute_shards` round required.
    ///
    /// Default **true**, deliberately, and NOT tied to
    /// `salvage_capacity_enabled`: unlike salvage (which volunteers a node for
    /// NEW custody it does not yet hold — the imago-dei consent floor keeps
    /// that opt-in), this pass only projects bytes the node ALREADY holds.
    /// That is reconciliation, not conscription, so it ships on by default
    /// like `demand_autopin_enabled`.
    ///
    /// To disable: set env `MANIFEST_BACKFILL_ENABLED` to `0`/`false`/`off`
    /// (or `manifest_backfill_enabled = false` in config.toml). Loaded from
    /// env `MANIFEST_BACKFILL_ENABLED`.
    #[serde(default = "default_true")]
    pub manifest_backfill_enabled: bool,
}

fn default_peer_policy_path() -> PathBuf {
    PathBuf::from("./config/peer-policy.toml")
}

fn default_http_port() -> u16 {
    8090
}

fn default_admin_url() -> String {
    "ws://localhost:4444".to_string()
}

fn default_app_id() -> String {
    "elohim".to_string()
}

fn default_role_name() -> String {
    "elohim".to_string()
}

fn default_zome_name() -> String {
    "content_store".to_string()
}

fn default_true() -> bool {
    true
}

fn default_min_replicas() -> u32 {
    2
}

fn default_sync_interval() -> u64 {
    60
}

fn default_p2p_port() -> u16 {
    9876
}

fn default_custody_sweep_seconds() -> u64 {
    120
}

fn default_placement_grace_seconds() -> u64 {
    300
}

fn default_placement_gap_cooldown_seconds() -> u64 {
    1800
}

fn default_kick_fetch_per_peer_per_minute() -> u32 {
    10
}

fn default_inventory_freshness_seconds() -> u64 {
    600
}

fn default_fetch_blob_timeout_seconds() -> u64 {
    5
}

fn default_fetch_blob_parallelism() -> usize {
    3
}

fn default_adopt_contest_fanout() -> usize {
    DEFAULT_ADOPT_CONTEST_FANOUT
}

fn default_contest_backoff_seconds() -> u64 {
    DEFAULT_CONTEST_BACKOFF_SECONDS
}

fn default_heal_on_read_budget_ms() -> u64 {
    5000
}

fn default_salvage_target_replicas() -> usize {
    // Mirrors `min_replicas_for_eviction` (2) — the desired distinct-holder
    // count per blob the salvage pass restores toward.
    2
}

fn default_salvage_recheck_seconds() -> u64 {
    300
}

fn default_demand_autopin_throttle_seconds() -> u64 {
    // 5-minute window: a burst of read-misses for one content id costs one pin
    // upsert, not one-per-request.
    300
}

impl Default for Config {
    fn default() -> Self {
        Self {
            storage_dir: default_storage_dir(),
            holochain_admin_url: default_admin_url(),
            app_id: default_app_id(),
            role_name: default_role_name(),
            zome_name: default_zome_name(),
            max_storage_bytes: 0,
            enable_eviction: true,
            min_replicas_for_eviction: 2,
            sync_interval_secs: 60,
            p2p_port: 9876,
            http_port: 8090,
            p2p_bootstrap_nodes: Vec::new(),
            enable_mdns: true,
            extraction_cache: elohim_cache_core::extraction::ExtractionCacheConfig::default(),
            peer_policy_path: default_peer_policy_path(),
            device_archetype: None,
            household_id: None,
            node_role: None,
            region: None,
            inventory_broadcast_seconds: None,
            custody_sweep_seconds: default_custody_sweep_seconds(),
            placement_grace_seconds: default_placement_grace_seconds(),
            placement_gap_cooldown_seconds: default_placement_gap_cooldown_seconds(),
            kick_fetch_per_peer_per_minute: default_kick_fetch_per_peer_per_minute(),
            inventory_freshness_seconds: default_inventory_freshness_seconds(),
            fetch_blob_timeout_seconds: default_fetch_blob_timeout_seconds(),
            fetch_blob_parallelism: default_fetch_blob_parallelism(),
            heal_on_read_budget_ms: default_heal_on_read_budget_ms(),
            self_cid: None,
            transport_backend: TransportBackend::default(),
            genesis_self_heal_identity: false,
            self_human_id: None,
            self_household_id: None,
            salvage_capacity_enabled: false,
            salvage_target_replicas: default_salvage_target_replicas(),
            salvage_recheck_seconds: default_salvage_recheck_seconds(),
            salvage_diversity_placement: default_true(),
            contest_two_way_declared: default_true(),
            adopt_before_author: false,
            adopt_contest_fanout: default_adopt_contest_fanout(),
            contest_backoff_seconds: default_contest_backoff_seconds(),
            demand_autopin_enabled: default_true(),
            demand_autopin_throttle_seconds: default_demand_autopin_throttle_seconds(),
            manifest_backfill_enabled: default_true(),
        }
    }
}

/// Default snapshot broadcast cadence per archetype.
/// `None` means broadcasting is disabled by default for this archetype.
///
/// T22 review fix #4: unknown archetype strings now emit a `tracing::warn!`
/// before falling back to the conservative `node` default. This surfaces
/// typos like `DEVICE_ARCHETYPE=nod` (missing 'e'), which previously
/// silently enabled the most aggressive cadence.
pub fn inventory_broadcast_seconds_default(archetype: Option<&str>) -> Option<u64> {
    match archetype {
        Some("node") | Some("steward") => Some(60),
        Some("desktop") => Some(300),
        Some("mobile") => None,
        // unset archetype → conservative node default (no warn — a missing
        // value is a normal config state, not a misconfiguration).
        None => Some(60),
        Some(other) => {
            tracing::warn!(
                target: "elohim_storage::inventory",
                archetype = %other,
                "unknown device archetype; defaulting to 60s inventory broadcast cadence (node)"
            );
            Some(60)
        }
    }
}

impl Config {
    /// Load config from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save config to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)
    }

    /// Get blobs directory
    pub fn blobs_dir(&self) -> PathBuf {
        self.storage_dir.join("blobs")
    }

    /// Get metadata database path
    pub fn metadata_db_path(&self) -> PathBuf {
        self.storage_dir.join("metadata.sled")
    }

    /// Get config file path
    pub fn config_path(&self) -> PathBuf {
        self.storage_dir.join("config.toml")
    }

    /// Get extraction cache directory (defaults to {storage_dir}/cache/extractions)
    pub fn extraction_cache_dir(&self) -> PathBuf {
        if self.extraction_cache.cache_dir.as_os_str().is_empty() {
            self.storage_dir.join("cache").join("extractions")
        } else {
            self.extraction_cache.cache_dir.clone()
        }
    }
}

#[cfg(test)]
mod transport_backend_tests {
    use super::TransportBackend;
    use std::str::FromStr;

    #[test]
    fn defaults_to_libp2p() {
        let cfg = super::Config::default();
        assert_eq!(cfg.transport_backend, TransportBackend::Libp2p);
    }

    #[test]
    fn salvage_diversity_placement_defaults_on() {
        // P3-8: the diversity strategy degrades exactly to XOR without household
        // data, so the knob ships ON; a flipped default must not pass silently.
        assert!(super::Config::default().salvage_diversity_placement);
    }

    #[test]
    fn demand_autopin_defaults_on_with_5min_throttle() {
        // Demand-driven (not blanket) auto-pin ships ON so the runtime provide
        // loop actually gets an input; a dark default-OFF stays inert. Consent
        // floor is preserved by the downstream provide-eligibility gate, not by
        // disabling this. A flipped default must not pass silently.
        let cfg = super::Config::default();
        assert!(cfg.demand_autopin_enabled);
        assert_eq!(cfg.demand_autopin_throttle_seconds, 300);
    }

    #[test]
    fn manifest_backfill_defaults_on() {
        // Reconciliation of bytes already held, never conscription for new
        // custody — unlike `salvage_capacity_enabled` this ships ON. A flipped
        // default must not pass silently.
        assert!(super::Config::default().manifest_backfill_enabled);
    }

    #[test]
    fn parses_known_values_case_insensitive() {
        assert_eq!(
            TransportBackend::from_str("libp2p").unwrap(),
            TransportBackend::Libp2p
        );
        assert_eq!(
            TransportBackend::from_str("LIBP2P").unwrap(),
            TransportBackend::Libp2p
        );
        assert_eq!(
            TransportBackend::from_str("iroh").unwrap(),
            TransportBackend::Iroh
        );
        assert_eq!(
            TransportBackend::from_str("Iroh").unwrap(),
            TransportBackend::Iroh
        );
    }

    #[test]
    fn rejects_unknown() {
        let err = TransportBackend::from_str("quic").unwrap_err();
        assert!(err.contains("invalid transport backend"));
    }

    #[test]
    fn round_trips_through_display() {
        for backend in [TransportBackend::Libp2p, TransportBackend::Iroh] {
            let s = backend.to_string();
            let parsed = TransportBackend::from_str(&s).unwrap();
            assert_eq!(parsed, backend);
        }
    }
}
