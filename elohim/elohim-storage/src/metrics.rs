//! Durable Prometheus app-metrics surface — the foundation (P0) of the
//! design-decision toolkit.
//!
//! elohim-storage previously had NO app-metrics surface: every runtime signal
//! was a `tracing` log line scraped from Loki, which 502-storms (it died
//! mid-decision on 2026-06-17, forcing the heap-leak verdict to come from
//! cadvisor by luck). This module is the durable, graphable, alertable,
//! historical twin of those log lines: one process-wide [`Registry`] exposed at
//! `GET /metrics`, scraped by the Prometheus Operator via a PodMonitor.
//!
//! Idiom mirrors `elohim-bitswap`: `lazy_static!` metric statics registered into
//! one `Registry`, `TextEncoder` for exposition. Callers use the typed setters
//! below (`set_proc_rss`, `set_cgroup_mem`, …) so they never import `prometheus`.
//!
//! Plan: `genesis/docs/superpowers/plans/2026-06-17-design-decision-toolkit-plan.md`

use lazy_static::lazy_static;
use prometheus::{
    Encoder, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder,
};
use std::sync::Once;

lazy_static! {
    /// The single process-wide registry the `/metrics` endpoint exposes.
    pub static ref REGISTRY: Registry = Registry::new();

    // ── Memory attribution (P1: the leak-vs-cache verdict + per-process split) ──

    /// Per-process resident memory, split anon (heap → the leak suspect) vs file
    /// (mapped → page cache). labels: proc = "holochain"|"elohim-storage",
    /// kind = "anon"|"file". This is the ATTRIBUTION the fused cgroup hides.
    pub static ref NODE_PROC_RSS_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_node_proc_rss_bytes",
            "Per-process resident set size in bytes, split anon (heap) vs file (mapped).",
        ),
        &["proc", "kind"],
    )
    .unwrap();

    /// Per-process thread count (tracks-RSS ⇒ blocking-pool exhaustion). label: proc.
    pub static ref NODE_PROC_THREADS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("elohim_node_proc_threads", "Per-process thread count."),
        &["proc"],
    )
    .unwrap();

    /// cgroup `memory.stat` breakdown — the leak-vs-cache VERDICT at the container
    /// level. label: kind = "anon" (heap/leak) | "file" (page cache) | "slab".
    pub static ref NODE_CGROUP_MEM_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_node_cgroup_mem_bytes",
            "cgroup memory.stat breakdown in bytes (anon=heap/leak, file=page-cache, slab=kernel).",
        ),
        &["kind"],
    )
    .unwrap();

    /// Swap charged to this cgroup (0 when swap off → anon is unreclaimable → OOM
    /// leans anon).
    pub static ref NODE_CGROUP_SWAP_BYTES: IntGauge = IntGauge::new(
        "elohim_node_cgroup_swap_bytes",
        "Swap charged to this cgroup in bytes (0 when swap off).",
    )
    .unwrap();

    /// Boot-time effective DHT read pool size = max(2*cpus, 8) (host-vs-cgroup proof).
    pub static ref NODE_DB_MAX_READERS: IntGauge = IntGauge::new(
        "elohim_node_db_max_readers",
        "Effective conductor db_max_readers = max(2*cpus, 8).",
    )
    .unwrap();

    /// Open shard-placement gaps (under-replicated shards) across the cluster weave.
    /// Set by the operational-weave facing adapter; the fold is pure (never touches this gauge).
    pub static ref ELOHIM_PLACEMENT_GAP_COUNT: IntGauge = IntGauge::new(
        "elohim_placement_gap_count",
        "Open shard-placement gaps (under-replicated shards) across the cluster weave",
    )
    .expect("valid gauge");

    /// Mean RS contract-coverage across open gaps, ×1000 (1000 = fully covered).
    /// Set by the operational-weave facing adapter; the fold is pure (never touches this gauge).
    pub static ref ELOHIM_RS_COVERAGE_MILLI: IntGauge = IntGauge::new(
        "elohim_rs_coverage_milli",
        "Mean RS contract-coverage across open gaps, ×1000 (1000 = fully covered)",
    )
    .expect("valid gauge");

    /// Cluster-aggregate FREE custodian capacity in bytes (Σ over reporting nodes;
    /// 0 when no node reported). The gauge mirror of `WeaveView.cluster_capacity.free`,
    /// set by the operational-weave facing adapter from the SAME `aggregate_capacity`
    /// fold (one fold, two projections). Cluster-scalar by design — per-custodian
    /// breakdown would be an unbounded-cardinality `IntGaugeVec` and is NOT emitted here.
    pub static ref ELOHIM_CUSTODIAN_FREE_BYTES: IntGauge = IntGauge::new(
        "elohim_custodian_free_bytes",
        "Cluster-aggregate free custodian capacity in bytes (0 = none reported)",
    )
    .expect("valid gauge");

    /// Cluster-aggregate USED custodian capacity in bytes (Σ over reporting nodes).
    pub static ref ELOHIM_CUSTODIAN_USED_BYTES: IntGauge = IntGauge::new(
        "elohim_custodian_used_bytes",
        "Cluster-aggregate used custodian capacity in bytes (0 = none reported)",
    )
    .expect("valid gauge");

    /// Cluster-aggregate STEWARDED bytes (Σ of custody-blob commitment quantities).
    pub static ref ELOHIM_CUSTODIAN_STEWARDED_BYTES: IntGauge = IntGauge::new(
        "elohim_custodian_stewarded_bytes",
        "Cluster-aggregate stewarded bytes from custody-blob commitments (0 = none)",
    )
    .expect("valid gauge");

    /// Boot-time cgroup CPU quota in millicores (0 = unbounded/unknown).
    pub static ref NODE_CPU_QUOTA_MILLICORES: IntGauge = IntGauge::new(
        "elohim_node_cpu_quota_millicores",
        "cgroup CPU quota in millicores (0 = unbounded/unknown).",
    )
    .unwrap();

    // ── Conductor heap-leak attribution (P2: smaps anon breakdown of the child) ──

    /// Conductor (child) anonymous memory by smaps mapping class — localizes the
    /// confirmed heap leak. label: class = "heap" | "stack" | "other".
    pub static ref NODE_CONDUCTOR_SMAPS_ANON_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_node_conductor_smaps_anon_bytes",
            "Conductor child anonymous memory by smaps mapping class (heap/stack/other).",
        ),
        &["class"],
    )
    .unwrap();

    /// Conductor (child) count of anon-bearing mappings (arena-proliferation signal).
    pub static ref NODE_CONDUCTOR_ANON_MAPPING_COUNT: IntGauge = IntGauge::new(
        "elohim_node_conductor_anon_mapping_count",
        "Conductor child count of anon-bearing mappings (arena-proliferation signal).",
    )
    .unwrap();

    /// Conductor (child) largest single mapping's anon bytes (the leak locus).
    pub static ref NODE_CONDUCTOR_LARGEST_ANON_BYTES: IntGauge = IntGauge::new(
        "elohim_node_conductor_largest_anon_bytes",
        "Conductor child largest single mapping's anon bytes (leak locus).",
    )
    .unwrap();

    /// Conductor (child) anon resident bytes by per-mapping SIZE bucket — the SHAPE
    /// discriminator. At flat mapping-count / flat-largest (the 2026-06-18 leak
    /// signature) a scalar `other` can't say WHERE the growth lands; a shift in this
    /// histogram localizes it (Go-arena band vs thread-stack band vs small-buffer
    /// band). label: bucket (see `system_metrics::ANON_SIZE_BUCKETS`).
    pub static ref NODE_CONDUCTOR_ANON_BUCKET_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_node_conductor_anon_bucket_bytes",
            "Conductor child anon resident bytes by per-mapping size bucket.",
        ),
        &["bucket"],
    )
    .unwrap();

    /// Conductor (child) count of anon mappings by per-mapping SIZE bucket (pairs
    /// with `_anon_bucket_bytes`: count-flat + bytes-up in a band = arena fill).
    pub static ref NODE_CONDUCTOR_ANON_BUCKET_COUNT: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_node_conductor_anon_bucket_count",
            "Conductor child count of anon mappings by per-mapping size bucket.",
        ),
        &["bucket"],
    )
    .unwrap();

    /// Node corpus size — content rows held, by app scope. label: app =
    /// "lamad" | "elohim". Pairs with the conductor RSS gauges to confirm
    /// (durably) that the heap leak is NOT corpus-proportional (RCA §4.5).
    pub static ref NODE_CORPUS_DOCS: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_node_corpus_docs",
            "Content rows held by this node, by app scope (corpus size).",
        ),
        &["app"],
    )
    .unwrap();

    // ── Identity-namespace coherence (P1: promote the atomic+log counter) ──

    /// A non-`agent_cid` value written to a column the joins treat as `agent_cid`
    /// (the all-zeros resilience-card root). labels: column, expected, got.
    pub static ref IDENTITY_NAMESPACE_VIOLATIONS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_identity_namespace_violation_total",
            "Non-agent_cid values written to agent_cid join-key columns (LOGGED, not rejected).",
        ),
        &["column", "expected", "got"],
    )
    .unwrap();

    // ── View-federation outcome attribution (the notary-authority spine) ──

    /// Outbound view-federation request outcomes at the resolution site. Before
    /// this counter the ONLY per-request signal was a Loki WARN (F-T19), which
    /// 502-storms — the timeout-vs-transport split (the whole failure diagnosis)
    /// was unobservable in Prometheus. label: result = "ok" | "timeout" |
    /// "connection_closed" | "dial_failure" | "unsupported_protocols" | "io".
    pub static ref VIEW_FEDERATION_OUTBOUND: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_view_federation_outbound_total",
            "View-federation outbound request outcomes by result (ok vs failure variant).",
        ),
        &["result"],
    )
    .unwrap();

    /// Inbound view-federation requests answered with a signed slice (the
    /// responder-side served count — pairs with the outbound counter to read the
    /// ask/serve balance across the fleet).
    pub static ref VIEW_FEDERATION_INBOUND_SERVED: IntCounter = IntCounter::new(
        "elohim_view_federation_inbound_served_total",
        "View-federation inbound requests answered with a signed slice (ResponseSent).",
    )
    .unwrap();

    /// Content heads freshly authored through the conductor by the sweep-driven
    /// witness-bootstrap step — bulk-seeded rows born un-witnessed
    /// (`dht_anchor_hash` NULL) that now carry a notarized head. Counts fresh
    /// authorings only (already-committed rows recovered via the idempotent
    /// already-exists path are NOT re-authored and are not counted here).
    pub static ref CONTENT_WITNESS_AUTHORED: IntCounter = IntCounter::new(
        "elohim_content_witness_authored_total",
        "Content heads authored through the conductor by the witness-bootstrap sweep.",
    )
    .unwrap();

    /// Provide-loop author calls SKIPPED because no candidate (active local
    /// session key nor the pod's own conductor cell key) yielded an `agent_cid`
    /// (`uhCAk…`) provider. The CID-hardening guard (rung 3) refuses to write a
    /// transport-id provider (`12D3Koo…`) that could never join the resilience
    /// card's `humans.agent_pub_key = rea_commitments.provider` join — a junk row
    /// that can never join is worse than absence, so the author skips this tick.
    pub static ref PROVIDE_PROVIDER_UNRESOLVED: IntCounter = IntCounter::new(
        "elohim_provide_provider_unresolved_total",
        "Provide-loop author calls skipped because no agent_cid provider was resolvable.",
    )
    .unwrap();

    /// Salvage-pass authoring ticks skipped because the node could not resolve a
    /// truthful `agent_cid` (`uhCAk…`) for itself (neither the active local
    /// session key nor the pod's own conductor cell key). The salvage author
    /// refuses to write a transport-id (`12D3Koo…` / iroh NodeId) `provider` into
    /// `rea_commitments.provider` — the same custody/resilience-card join key the
    /// provide-loop guards — so it skips this tick rather than mint a junk row.
    /// Sibling to [`PROVIDE_PROVIDER_UNRESOLVED`] but scoped to the salvage writer.
    pub static ref SALVAGE_PROVIDER_UNRESOLVED: IntCounter = IntCounter::new(
        "elohim_salvage_provider_unresolved_total",
        "Salvage author ticks skipped because no agent_cid self-provider was resolvable.",
    )
    .unwrap();

    /// Membership-truth key supersede + rekey cascade outcomes. A boot-time pass
    /// (`services::membership_identity_reconcile`) converges a SET-but-stale
    /// `humans.agent_pub_key` (a NON-self row a peer re-key fossilised) to the live
    /// membership key and cascades the holder rows so the resilience stewarding
    /// join re-aligns. labels: kind = "supersede" (human rows moved) |
    /// "shard_locations" (holder rows re-attributed) | "rea_commitments"
    /// (commitments re-attributed) | "ambiguous_skip" (a household whose
    /// fossil↔membership pairing was not 1:1 — never guessed) | "non_agent_cid_skip"
    /// (a membership member_cid that was not agent_cid-shaped) | "incomplete_read_skip"
    /// (a household whose membership read dropped/failed a record — abstained so a
    /// missing member cannot force a false 1:1).
    pub static ref IDENTITY_KEY_SUPERSEDE: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_identity_key_supersede_total",
            "Membership-truth human agent-key supersedes + rekey-cascade re-attributions, by kind.",
        ),
        &["kind"],
    )
    .unwrap();

    /// Shard-push targets SKIPPED because the selected `agent_cid` had no known
    /// libp2p transport binding (`peer_transport_manifest` + `peer_identity_bindings`
    /// both missed). Before the push-side resolver, distribution errored on EVERY
    /// peer (an `agent_cid` never parses as a libp2p PeerId); this counter surfaces
    /// the remaining, honest gap — a peer we selected but cannot dial yet.
    pub static ref SHARD_PUSH_PEER_UNRESOLVED: IntCounter = IntCounter::new(
        "elohim_shard_push_peer_unresolved_total",
        "Shard-push targets skipped because the selected agent_cid had no libp2p transport binding.",
    )
    .unwrap();

    // ── Projection-reconcile heal legibility (the saturated-conductor cure) ──

    /// Projection-reconcile HEAL outcomes, by stream and outcome. The durable twin
    /// of the per-row heal WARN/DEBUG lines (which 502-storm out of Loki): on a
    /// saturated conductor (adam's ~1/min steady WS timeouts) the split between
    /// `timeout_retried` (recovered by the bounded in-leg retry) and
    /// `timeout_exhausted` (still wedged after retries) IS the diagnosis, and it was
    /// previously invisible in Prometheus. labels: stream = "rea" | "content";
    /// outcome = "healed" | "timeout_retried" | "timeout_exhausted" | "missing"
    ///         | "failed".
    ///
    /// NOT a total-row-accounting metric: benign content resolutions
    /// (SkippedDeclared / SkippedStale / NoRow) are deliberately uncounted so
    /// the cure signal is not inflated — sum-of-outcomes < rows-attempted on
    /// the content stream by design. Dashboard math must not assume equality.
    pub static ref PROJECTION_HEAL_OUTCOMES: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_projection_heal_outcomes_total",
            "Projection-reconcile heal row outcomes by stream and outcome.",
        ),
        &["stream", "outcome"],
    )
    .unwrap();

    /// Last-sweep DISCOVERED gaps per reconcile stream (pending after discovery,
    /// before heal). label: stream = "rea" | "content". Watch this fall toward 0
    /// as heal lands rows without tailing Loki.
    pub static ref PROJECTION_RECONCILE_GAPS: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_gaps",
            "Last-sweep discovered projection-reconcile gaps by stream.",
        ),
        &["stream"],
    )
    .unwrap();

    /// Last-sweep LOCAL projection row count per reconcile stream (the convergence
    /// target). label: stream = "rea" | "content". `rea` climbing off 0 is the
    /// direct cure signal for the starved-heal incident (rea_local_total stuck at 0
    /// for 3h on adam while matthew healed the same backlog).
    pub static ref PROJECTION_RECONCILE_LOCAL_TOTAL: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_local_total",
            "Last-sweep local projection row count by reconcile stream.",
        ),
        &["stream"],
    )
    .unwrap();

    /// Last-sweep gaps ABANDONED at `max_retries` per reconcile stream. label:
    /// stream = "rea" | "content".
    ///
    /// This is the counterpart `gaps` cannot show. `GapTracker::mark_failed`
    /// removes an id from `pending` without re-queueing it, and
    /// `enqueue_missing` refuses to re-queue anything past `max_retries` — so an
    /// abandoned gap leaves BOTH `pending` and the `gaps` gauge, and the sweep
    /// reports `caughtUp: true` having healed nothing. A `gaps` gauge falling to
    /// zero is therefore ambiguous: it means "healed" or "gave up", and only
    /// this series tells them apart.
    pub static ref PROJECTION_RECONCILE_EXHAUSTED: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_exhausted",
            "Last-sweep gaps abandoned at max_retries by reconcile stream.",
        ),
        &["stream"],
    )
    .unwrap();

    /// Last-sweep ANCHOR-DIVERGENT rows per reconcile stream — present locally
    /// but under a different anchor than a peer advertised. label: stream =
    /// "rea" | "content". Per-arm on purpose: the status surface folds both arms
    /// into one `divergentAnchor` number, which cannot answer "which stream is
    /// diverging?" — the first question asked when a peer will not converge.
    pub static ref PROJECTION_RECONCILE_DIVERGENT: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_divergent",
            "Last-sweep anchor-divergent rows by reconcile stream.",
        ),
        &["stream"],
    )
    .unwrap();

    /// Reconcile sweeps completed. The DENOMINATOR for every other reconcile
    /// series — "gaps per sweep" and "heals per sweep" are only readable against
    /// it, exactly as `elohim_sync_rounds_total` serves the sync plane. Without
    /// it, `healedTotal: 0` cannot be distinguished from "no sweep has run yet".
    pub static ref PROJECTION_RECONCILE_SWEEPS: IntCounter = IntCounter::new(
        "elohim_projection_reconcile_sweeps_total",
        "Reconcile sweeps completed.",
    )
    .unwrap();

    /// 1 when this peer holds what its peers advertised, 0 otherwise:
    /// `pending == 0 && exhausted == 0 && divergentAnchor == 0`.
    ///
    /// The field an SLO may ride. Deliberately NOT the same thing as the
    /// `caughtUp` published on `/p2p/status`, which goes true when a sweep ENDS
    /// — including a sweep that healed nothing because every gap spent its retry
    /// budget (the live 22-sweep / `healedTotal: 0` shape). Cumulative heals are
    /// already derivable from `elohim_projection_heal_outcomes_total`, so no
    /// healed-total counter is added here.
    pub static ref PROJECTION_RECONCILE_CONVERGED: IntGauge = IntGauge::new(
        "elohim_projection_reconcile_converged",
        "1 when the peer holds what its peers advertised (pending+exhausted+divergent all zero).",
    )
    .unwrap();

    /// Part-B redistribution attempts SKIPPED because the candidate's blob bytes
    /// were not local — measured-but-dark content this node holds a manifest for but
    /// cannot source the bytes of, so it cannot be the distributor. Before this the
    /// skip was a silent `return` (no log, no gap row, no metric) that hid
    /// elohim-host-landing's non-distribution for a day. Pairs with the
    /// `peers-unavailable` placement-gap row written on the same path: the
    /// placement-gap `gap_kind` is a CLOSED enum at the wire schema
    /// (`placement-gap-view.schema.json`), so the precise "bytes-not-local" cause
    /// lives HERE, in this metric, not in the gap row's kind.
    pub static ref SHARD_REDISTRIBUTE_BYTES_MISSING: IntCounter = IntCounter::new(
        "elohim_shard_redistribute_bytes_missing_total",
        "Redistribution attempts skipped because the candidate's blob bytes were not local.",
    )
    .unwrap();

    /// Last peer_status fan-in sweep's `missed` count — per-agent reads that
    /// failed or were skipped (`services::peer_status_fanout::FanoutStats.missed`).
    /// Folded onto the durable surface so a chronically lossy fan-in is visible
    /// without tailing Loki.
    pub static ref PEER_STATUS_FANOUT_MISSED: IntGauge = IntGauge::new(
        "elohim_peer_status_fanout_missed",
        "Missed per-agent reads in the last peer_status fan-in sweep.",
    )
    .unwrap();

    /// Identity-fill outcomes by action — the periodic pass that fills NULL
    /// `humans.agent_pub_key` and creates missing member rows from DHT membership
    /// truth (`services::identity_fill`). labels: action = "created" | "filled"
    /// | "skipped_present" | "skipped_non_agent_cid". `created` + `filled`
    /// climbing off 0 on a dark pod (adam `commitmentBacked 0`) is the direct
    /// cure signal that the resilience stewarding join now has keyed rows to join.
    pub static ref IDENTITY_FILL_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_identity_fill_total",
            "Periodic membership-truth identity fills, by action.",
        ),
        &["action"],
    )
    .unwrap();

    /// Rows written (created + filled) in the LAST identity-fill sweep. Falls to
    /// 0 once the pod's membership is fully projected (steady state); a nonzero
    /// value means a member row was just laid this tick.
    pub static ref IDENTITY_FILL_LAST_WRITES: IntGauge = IntGauge::new(
        "elohim_identity_fill_last_writes",
        "Rows created+filled in the last identity-fill sweep.",
    )
    .unwrap();

    /// Custody-announcement flow (the cross-pod `shard_locations` convergence).
    /// `shard_locations` is written only locally (self-held recording + push-ack),
    /// so custody claims never left the node and each doorway read a different
    /// network footprint. A [`CustodyAnnouncement`](crate::p2p::custody_announce)
    /// gossips a claim on change; peers project it as a `peer-announced` row that
    /// never overwrites a locally-witnessed one. labels: direction =
    ///   "sent"          — a claim this node broadcast,
    ///   "received"      — a claim decoded off the wire (before the apply verdict),
    ///   "applied"       — landed a write (absence filled or refreshed),
    ///   "dropped_weaker"— an existing local/peer row was stronger-or-fresher,
    ///   "dropped_self"  — a peer claiming THIS node holds a shard (weaker than
    ///                     our own records).
    /// `received == applied + dropped_weaker + dropped_self` per receive path.
    pub static ref CUSTODY_ANNOUNCE_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_custody_announce_total",
            "Custody-announcement flow for cross-pod shard_locations convergence, by direction.",
        ),
        &["direction"],
    )
    .unwrap();

    // ── Sync plane cost + outcomes (spine node `sync-scale-honesty`) ──

    /// Sync rounds initiated by this node's poll tick. The denominator for every
    /// other sync counter — "requests per round" and "docs enumerated per round"
    /// are only readable against it.
    pub static ref SYNC_ROUNDS: IntCounter = IntCounter::new(
        "elohim_sync_rounds_total",
        "Sync rounds initiated by the poll tick.",
    )
    .unwrap();

    /// Outbound sync requests by protocol verb. label: kind = "list_documents"
    /// (round opener + page follow-ups) | "sync_changes" (per diverged doc) |
    /// "announce_change" (push on local change — NO send site exists yet, so this
    /// series staying at zero IS the standing red, visible rather than asserted).
    pub static ref SYNC_REQUESTS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_sync_requests_total",
            "Outbound sync requests by protocol verb.",
        ),
        &["kind"],
    )
    .unwrap();

    /// Document entries received in `DocumentList` answers — the MAGNITUDE of the
    /// round's enumeration. This is the counter that makes a corpus-proportional
    /// round visible: on a converged mesh it should trend to zero, and today it
    /// tracks corpus size x peers forever. Rate over `elohim_sync_rounds_total`
    /// is the scaling read.
    pub static ref SYNC_DOCS_ENUMERATED: IntCounter = IntCounter::new(
        "elohim_sync_docs_enumerated_total",
        "Document entries received in DocumentList answers (round enumeration cost).",
    )
    .unwrap();

    /// Acquisition (`pull` leg) outcomes — the leg that fetches content this peer
    /// is missing. label: outcome = "fetched" | "transport_failure" | "no_db_pool" |
    /// "no_db_conn" | "blob_unavailable" | "store_failed" | "fetch_error".
    ///
    /// Measured on alpha 2026-07-25: `pull` reported total=29 fetched=0 failed=29
    /// (beta: 5/0/5) while the ONLY code signal was a `debug!` the deployed level
    /// drops — 1.13M log lines over 6h contained zero occurrences. A leg that fails
    /// every single attempt looked exactly like a healthy idle leg. `fetched` is
    /// counted alongside the failures on purpose: a ratio is readable, a failure
    /// count alone is not.
    pub static ref ACQUISITION_OUTCOMES: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_acquisition_outcomes_total",
            "Acquisition (pull-leg) fetch outcomes by result.",
        ),
        &["outcome"],
    )
    .unwrap();

    /// Outbound sync request outcomes. Deliberately labelled by RESULT ONLY, never
    /// by peer: a peer-id label is an unbounded-cardinality bomb on a real mesh.
    /// Peer identity stays in the log line at the same site. label: result = "ok" |
    /// "timeout" | "connection_closed" | "dial_failure" | "unsupported_protocols" |
    /// "io" — the same closed vocabulary as `elohim_view_federation_outbound_total`,
    /// so the two planes read the same way.
    pub static ref SYNC_REQUEST_OUTCOMES: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_sync_request_outcomes_total",
            "Outbound sync request outcomes by result (ok vs failure variant).",
        ),
        &["result"],
    )
    .unwrap();
}

/// Register every toolkit collector into [`REGISTRY`]. Idempotent (guarded by a
/// `Once`), so calling it more than once at boot is safe. Call exactly once early
/// in storage startup; `/metrics` reads the registry thereafter.
pub fn register_all() {
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| {
        // register() errors only on duplicate/invalid collectors; the Once guard
        // already prevents duplicates, so a stray Err is non-fatal.
        let _ = REGISTRY.register(Box::new(NODE_PROC_RSS_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_PROC_THREADS.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CGROUP_MEM_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CGROUP_SWAP_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_DB_MAX_READERS.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CPU_QUOTA_MILLICORES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CONDUCTOR_SMAPS_ANON_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CONDUCTOR_ANON_MAPPING_COUNT.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CONDUCTOR_LARGEST_ANON_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CONDUCTOR_ANON_BUCKET_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CONDUCTOR_ANON_BUCKET_COUNT.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CORPUS_DOCS.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_NAMESPACE_VIOLATIONS.clone()));
        let _ = REGISTRY.register(Box::new(ELOHIM_PLACEMENT_GAP_COUNT.clone()));
        let _ = REGISTRY.register(Box::new(ELOHIM_RS_COVERAGE_MILLI.clone()));
        let _ = REGISTRY.register(Box::new(ELOHIM_CUSTODIAN_FREE_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(ELOHIM_CUSTODIAN_USED_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(ELOHIM_CUSTODIAN_STEWARDED_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(VIEW_FEDERATION_OUTBOUND.clone()));
        let _ = REGISTRY.register(Box::new(VIEW_FEDERATION_INBOUND_SERVED.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_WITNESS_AUTHORED.clone()));
        let _ = REGISTRY.register(Box::new(PROVIDE_PROVIDER_UNRESOLVED.clone()));
        let _ = REGISTRY.register(Box::new(SALVAGE_PROVIDER_UNRESOLVED.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_KEY_SUPERSEDE.clone()));
        let _ = REGISTRY.register(Box::new(SHARD_PUSH_PEER_UNRESOLVED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_HEAL_OUTCOMES.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_GAPS.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_LOCAL_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_EXHAUSTED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_DIVERGENT.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_SWEEPS.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_CONVERGED.clone()));
        let _ = REGISTRY.register(Box::new(SHARD_REDISTRIBUTE_BYTES_MISSING.clone()));
        let _ = REGISTRY.register(Box::new(PEER_STATUS_FANOUT_MISSED.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_FILL_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_FILL_LAST_WRITES.clone()));
        let _ = REGISTRY.register(Box::new(CUSTODY_ANNOUNCE_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_ROUNDS.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_REQUESTS.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_DOCS_ENUMERATED.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_REQUEST_OUTCOMES.clone()));
        let _ = REGISTRY.register(Box::new(ACQUISITION_OUTCOMES.clone()));
    });
}

/// Render the registry in Prometheus text exposition format (the `/metrics` body).
pub fn gather_text() -> String {
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    let families = REGISTRY.gather();
    if encoder.encode(&families, &mut buf).is_err() {
        return String::new();
    }
    String::from_utf8(buf).unwrap_or_default()
}

// ── Typed setters (callers never import `prometheus`) ──

/// Set a process's RSS split + thread count (the memory-attribution sampler).
pub fn set_proc_rss(proc_name: &str, rss_anon: u64, rss_file: u64, threads: u64) {
    NODE_PROC_RSS_BYTES
        .with_label_values(&[proc_name, "anon"])
        .set(rss_anon as i64);
    NODE_PROC_RSS_BYTES
        .with_label_values(&[proc_name, "file"])
        .set(rss_file as i64);
    NODE_PROC_THREADS
        .with_label_values(&[proc_name])
        .set(threads as i64);
}

/// Set the cgroup memory breakdown (the leak-vs-cache verdict gauges).
pub fn set_cgroup_mem(anon: u64, file: u64, slab: u64, swap: Option<u64>) {
    NODE_CGROUP_MEM_BYTES
        .with_label_values(&["anon"])
        .set(anon as i64);
    NODE_CGROUP_MEM_BYTES
        .with_label_values(&["file"])
        .set(file as i64);
    NODE_CGROUP_MEM_BYTES
        .with_label_values(&["slab"])
        .set(slab as i64);
    if let Some(s) = swap {
        NODE_CGROUP_SWAP_BYTES.set(s as i64);
    }
}

/// Set boot-time effective tunables (logged once at startup).
pub fn set_boot_tunables(db_max_readers: u32, cpu_quota_cores: Option<f64>) {
    NODE_DB_MAX_READERS.set(db_max_readers as i64);
    if let Some(c) = cpu_quota_cores {
        NODE_CPU_QUOTA_MILLICORES.set((c * 1000.0) as i64);
    }
}

/// Set the conductor (child) smaps anon breakdown (the heap-leak locus gauges).
pub fn set_conductor_smaps(heap: u64, stack: u64, other: u64, count: u64, largest: u64) {
    NODE_CONDUCTOR_SMAPS_ANON_BYTES
        .with_label_values(&["heap"])
        .set(heap as i64);
    NODE_CONDUCTOR_SMAPS_ANON_BYTES
        .with_label_values(&["stack"])
        .set(stack as i64);
    NODE_CONDUCTOR_SMAPS_ANON_BYTES
        .with_label_values(&["other"])
        .set(other as i64);
    NODE_CONDUCTOR_ANON_MAPPING_COUNT.set(count as i64);
    NODE_CONDUCTOR_LARGEST_ANON_BYTES.set(largest as i64);
}

/// Set the conductor (child) anon size-bucket histogram. `buckets` is
/// `(label, count, bytes)` per band, as produced by
/// `system_metrics::anon_size_histogram`.
pub fn set_conductor_anon_buckets(buckets: &[(&str, u64, u64)]) {
    for (label, count, bytes) in buckets {
        NODE_CONDUCTOR_ANON_BUCKET_COUNT
            .with_label_values(&[label])
            .set(*count as i64);
        NODE_CONDUCTOR_ANON_BUCKET_BYTES
            .with_label_values(&[label])
            .set(*bytes as i64);
    }
}

/// Set the node corpus size for an app scope (content rows held).
pub fn set_corpus_docs(app: &str, docs: u64) {
    NODE_CORPUS_DOCS.with_label_values(&[app]).set(docs as i64);
}

/// Increment the identity-namespace violation counter (paired with the WARN +
/// atomic in `identity_namespace::observe_agent_cid_write`).
pub fn inc_identity_namespace_violation(column: &str, got: &str) {
    IDENTITY_NAMESPACE_VIOLATIONS
        .with_label_values(&[column, "agent_cid", got])
        .inc();
}

/// Record one outbound view-federation outcome. `result` is the outcome label
/// ("ok" | "timeout" | "connection_closed" | "dial_failure" |
/// "unsupported_protocols" | "io"), produced at the resolution site.
pub fn inc_view_federation_outbound(result: &str) {
    VIEW_FEDERATION_OUTBOUND.with_label_values(&[result]).inc();
}

/// Record one inbound view-federation request answered with a signed slice.
pub fn inc_view_federation_inbound_served() {
    VIEW_FEDERATION_INBOUND_SERVED.inc();
}

/// Record one sync round initiated by the poll tick.
pub fn inc_sync_round() {
    SYNC_ROUNDS.inc();
}

/// Record one outbound sync request by protocol verb ("list_documents" |
/// "sync_changes" | "announce_change").
pub fn inc_sync_request(kind: &str) {
    SYNC_REQUESTS.with_label_values(&[kind]).inc();
}

/// Record document entries received in one `DocumentList` answer — the round's
/// enumeration cost.
pub fn add_sync_docs_enumerated(n: u64) {
    SYNC_DOCS_ENUMERATED.inc_by(n);
}

/// Record one outbound sync request outcome ("ok" | "timeout" |
/// "connection_closed" | "dial_failure" | "unsupported_protocols" | "io").
/// Result-only by design — see [`SYNC_REQUEST_OUTCOMES`] on why there is no peer
/// label.
pub fn inc_sync_request_outcome(result: &str) {
    SYNC_REQUEST_OUTCOMES.with_label_values(&[result]).inc();
}

/// Record one acquisition (pull-leg) outcome ("fetched" | "transport_failure" |
/// "no_db_pool" | "no_db_conn").
pub fn inc_acquisition_outcome(outcome: &str) {
    ACQUISITION_OUTCOMES.with_label_values(&[outcome]).inc();
}

/// Record `n` content heads freshly authored by the witness-bootstrap sweep.
pub fn add_content_witness_authored(n: u64) {
    CONTENT_WITNESS_AUTHORED.inc_by(n);
}

/// Record one provide-loop author skipped because no `agent_cid` provider was
/// resolvable (the CID-hardening guard refusing to write a transport id).
pub fn inc_provide_provider_unresolved() {
    PROVIDE_PROVIDER_UNRESOLVED.inc();
}

/// Record one salvage author tick skipped because no `agent_cid` self-provider
/// was resolvable (the salvage writer refusing to write a transport id).
pub fn inc_salvage_provider_unresolved() {
    SALVAGE_PROVIDER_UNRESOLVED.inc();
}

/// Record one membership-truth human key supersede and its rekey cascade. `kind`
/// is one of the labels documented on [`IDENTITY_KEY_SUPERSEDE`]. Row-count kinds
/// (`shard_locations`, `rea_commitments`) are incremented by their re-attributed
/// row counts; single-event kinds (`supersede`, `ambiguous_skip`,
/// `non_agent_cid_skip`) by 1.
pub fn inc_identity_key_supersede(kind: &str, n: u64) {
    if n == 0 {
        return;
    }
    IDENTITY_KEY_SUPERSEDE.with_label_values(&[kind]).inc_by(n);
}

/// Record one shard-push target skipped because its `agent_cid` had no resolvable
/// libp2p transport binding (`services::transport_resolve` returned `None`).
pub fn inc_shard_push_peer_unresolved() {
    SHARD_PUSH_PEER_UNRESOLVED.inc();
}

/// Record one custody-announcement flow event. `direction` is one of the labels
/// documented on [`CUSTODY_ANNOUNCE_TOTAL`]: `sent` | `received` | `applied` |
/// `dropped_weaker` | `dropped_self`.
pub fn inc_custody_announce(direction: &str) {
    CUSTODY_ANNOUNCE_TOTAL.with_label_values(&[direction]).inc();
}

/// Record one projection-reconcile heal row outcome. `stream` is "rea" | "content";
/// `outcome` is one of the labels documented on [`PROJECTION_HEAL_OUTCOMES`].
pub fn inc_projection_heal_outcome(stream: &str, outcome: &str) {
    PROJECTION_HEAL_OUTCOMES
        .with_label_values(&[stream, outcome])
        .inc();
}

/// Publish a reconcile stream's last-sweep gauges: discovered `gaps` (pending after
/// discovery) and `local_total` (local projection rows). `stream` is "rea" | "content".
pub fn set_projection_reconcile_gauges(
    stream: &str,
    gaps: u64,
    local_total: u64,
    exhausted: u64,
    divergent: u64,
) {
    PROJECTION_RECONCILE_GAPS
        .with_label_values(&[stream])
        .set(gaps as i64);
    PROJECTION_RECONCILE_LOCAL_TOTAL
        .with_label_values(&[stream])
        .set(local_total as i64);
    PROJECTION_RECONCILE_EXHAUSTED
        .with_label_values(&[stream])
        .set(exhausted as i64);
    PROJECTION_RECONCILE_DIVERGENT
        .with_label_values(&[stream])
        .set(divergent as i64);
}

/// 1 when the peer holds what its peers advertised, 0 otherwise. Pure so the
/// three-condition rule is testable without a registry — and so it cannot drift
/// from `GapCounts::converged`, which computes the same rule minus divergence.
pub fn converged_gauge_value(
    counts: &crate::p2p::reconcile_rails::GapCounts,
    divergent: usize,
) -> i64 {
    i64::from(counts.converged && divergent == 0)
}

/// Publish one completed reconcile sweep: advance the sweep denominator and
/// republish whether this peer actually converged.
///
/// Called from `ProjectionReconcileState::publish_sweep`, so the metric and the
/// `/p2p/status` field are written from the same place and cannot disagree —
/// the failure mode that let `/health` and `/p2p/status` drift 12x apart.
pub fn record_reconcile_sweep(
    counts: &crate::p2p::reconcile_rails::GapCounts,
    divergent_anchor: usize,
) {
    PROJECTION_RECONCILE_SWEEPS.inc();
    PROJECTION_RECONCILE_CONVERGED.set(converged_gauge_value(counts, divergent_anchor));
}

/// Record one Part-B redistribution attempt skipped because the candidate's blob
/// bytes were not local (the closed placement-gap enum cannot carry this cause, so
/// the precise signal lives here — see [`SHARD_REDISTRIBUTE_BYTES_MISSING`]).
pub fn inc_shard_redistribute_bytes_missing() {
    SHARD_REDISTRIBUTE_BYTES_MISSING.inc();
}

/// Publish the last peer_status fan-in sweep's `missed` count.
pub fn set_peer_status_fanout_missed(missed: u64) {
    PEER_STATUS_FANOUT_MISSED.set(missed as i64);
}

/// Record `n` identity-fill outcomes of `action` (one of the labels documented
/// on [`IDENTITY_FILL_TOTAL`]).
///
/// The label series is ALWAYS materialised (via `with_label_values`), even when
/// `n == 0` — `inc_by(0)` is a no-op on the value but registers the series. The
/// fill loop calls this for all four labels every sweep, so after the first
/// sweep all four series exist (at whatever value), and a completed no-op sweep
/// (four series present at 0) is DISTINGUISHABLE in Prometheus from a
/// never-ran loop (series absent). The earlier early-return-before-touch made a
/// zero-`n` sweep indistinguishable from a loop that never ran — a diagnosis
/// ambiguity that cost an hour on the adam `commitmentBacked 0` incident.
pub fn inc_identity_fill(action: &str, n: u64) {
    // Materialise the series unconditionally; increment only when non-zero.
    IDENTITY_FILL_TOTAL.with_label_values(&[action]).inc_by(n);
}

/// Publish the last identity-fill sweep's rows-written (created + filled) count.
pub fn set_identity_fill_last_writes(writes: u64) {
    IDENTITY_FILL_LAST_WRITES.set(writes as i64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::reconcile_rails::GapCounts;

    #[test]
    fn converged_gauge_needs_all_three_conditions_not_just_an_empty_queue() {
        // pending==0 AND exhausted==0 AND divergentAnchor==0. Any one of the
        // three defeats it — that is the whole point of the field.
        let healed = GapCounts {
            pending: 0,
            completed: 5,
            failed: 0,
            caught_up: true,
            exhausted: 0,
            converged: true,
        };
        assert_eq!(converged_gauge_value(&healed, 0), 1);

        // The live beta shape: 22 sweeps, healedTotal 0 — every gap abandoned
        // at max_retries. `caught_up` is true here; convergence must not be.
        let abandoned = GapCounts {
            pending: 0,
            completed: 0,
            failed: 61,
            caught_up: true,
            exhausted: 61,
            converged: false,
        };
        assert_eq!(converged_gauge_value(&abandoned, 0), 0);

        // Clean gap ledger, but rows sit locally under an anchor no peer
        // advertises. The sweep resolved nothing about them.
        assert_eq!(converged_gauge_value(&healed, 1860), 0);
    }

    #[test]
    fn register_all_idempotent_and_gathers_all_metrics() {
        register_all();
        register_all(); // Once-guarded — must not panic or double-register.

        set_proc_rss("holochain", 5_000_000_000, 130_000_000, 42);
        set_cgroup_mem(5_000_000_000, 130_000_000, 50_000_000, Some(0));
        set_boot_tunables(8, Some(4.0));
        set_conductor_smaps(4_096, 132_096, 6_000_000_000, 5_600, 158_265_344);
        set_conductor_anon_buckets(&[
            ("1m-8m", 4_200, 5_800_000_000),
            ("64m-256m", 1, 158_265_344),
        ]);
        inc_identity_namespace_violation("rea_commitments.provider", "libp2p");
        // Exercise every outbound outcome label + the inbound served counter.
        for result in [
            "ok",
            "timeout",
            "connection_closed",
            "dial_failure",
            "unsupported_protocols",
            "io",
        ] {
            inc_view_federation_outbound(result);
        }
        inc_view_federation_inbound_served();
        add_content_witness_authored(3);
        // Projection-reconcile heal legibility: exercise every outcome label on
        // both streams, the per-stream gauges, and the redistribution-bytes-missing
        // + fan-in-missed surfaces.
        for stream in ["rea", "content"] {
            for outcome in [
                "healed",
                "timeout_retried",
                "timeout_exhausted",
                "missing",
                "failed",
            ] {
                inc_projection_heal_outcome(stream, outcome);
            }
        }
        // The live 2026-07-25 shape, per arm: rea holds ZERO local rows while
        // 62 gaps sit discovered, and content carries 1860 rows under an anchor
        // no peer advertises. `caughtUp` reported true over both.
        set_projection_reconcile_gauges("rea", 62, 0, 61, 0);
        set_projection_reconcile_gauges("content", 1956, 4158, 0, 1860);
        record_reconcile_sweep(
            &GapCounts {
                pending: 0,
                completed: 0,
                failed: 61,
                caught_up: true,
                exhausted: 61,
                converged: false,
            },
            1860,
        );
        inc_shard_redistribute_bytes_missing();
        set_peer_status_fanout_missed(2);
        // Sync plane (spine node sync-scale-honesty): the round's cost and its
        // per-request outcomes had NO metric at all — the plane was not just
        // poll-only, it was unmeasured, so a corpus-proportional round and a
        // peer whose sync requests always time out looked identical to healthy.
        inc_sync_round();
        for kind in ["list_documents", "sync_changes", "announce_change"] {
            inc_sync_request(kind);
        }
        add_sync_docs_enumerated(1956);
        for result in [
            "ok",
            "timeout",
            "connection_closed",
            "dial_failure",
            "unsupported_protocols",
            "io",
        ] {
            inc_sync_request_outcome(result);
        }
        // Acquisition (the `pull` leg). Measured 2026-07-25: alpha 29/29 failed,
        // beta 5/5 failed, fetched=0 on both — a 100% dead leg whose ONLY signal
        // was a debug! that the deployed log level drops (1.13M log lines in 6h,
        // zero mentions). A silent total failure is indistinguishable from health.
        for outcome in [
            "fetched",
            "transport_failure",
            "no_db_pool",
            "no_db_conn",
            "blob_unavailable",
            "store_failed",
            "fetch_error",
        ] {
            inc_acquisition_outcome(outcome);
        }

        let text = gather_text();
        assert!(
            text.contains("elohim_node_proc_rss_bytes"),
            "proc rss gauge missing:\n{text}"
        );
        assert!(text.contains("elohim_node_cgroup_mem_bytes"));
        assert!(text.contains("elohim_node_db_max_readers"));
        assert!(text.contains("elohim_identity_namespace_violation_total"));
        assert!(text.contains("elohim_node_conductor_anon_bucket_bytes"));
        assert!(text.contains("elohim_node_conductor_anon_bucket_count"));
        // Label rendering sanity (the attribution dimension).
        assert!(text.contains("proc=\"holochain\""), "{text}");
        assert!(text.contains("kind=\"anon\""));
        assert!(text.contains("bucket=\"1m-8m\""), "{text}");
        // View-federation outcome attribution — every failure variant + ok +
        // the inbound served counter must render.
        assert!(
            text.contains("elohim_view_federation_outbound_total"),
            "view-federation outbound counter missing:\n{text}"
        );
        assert!(text.contains("result=\"timeout\""), "{text}");
        assert!(text.contains("result=\"ok\""), "{text}");
        assert!(text.contains("result=\"connection_closed\""), "{text}");
        assert!(text.contains("result=\"io\""), "{text}");
        assert!(
            text.contains("elohim_view_federation_inbound_served_total"),
            "view-federation inbound served counter missing:\n{text}"
        );
        assert!(
            text.contains("elohim_content_witness_authored_total"),
            "content witness-authored counter missing:\n{text}"
        );
        // Reconcile honesty. `gaps`/`local_total`/`heal_outcomes` already
        // existed; what was missing is the distinction between "this sweep
        // ended" and "this peer holds what its peers hold" — and the sweep
        // denominator that makes the other reconcile series a rate.
        assert!(
            text.contains("elohim_projection_reconcile_exhausted"),
            "exhausted gauge missing — abandoned gaps stay invisible:\n{text}"
        );
        assert!(
            text.contains("elohim_projection_reconcile_divergent"),
            "per-stream divergent gauge missing:\n{text}"
        );
        assert!(
            text.contains("elohim_projection_reconcile_sweeps_total"),
            "sweep denominator missing:\n{text}"
        );
        assert!(
            text.contains("elohim_projection_reconcile_converged"),
            "converged gauge missing — the field an SLO rides:\n{text}"
        );
        // Sync plane cost + per-request outcome attribution.
        assert!(
            text.contains("elohim_sync_rounds_total"),
            "sync rounds counter missing:\n{text}"
        );
        assert!(
            text.contains("elohim_sync_requests_total"),
            "sync requests counter missing:\n{text}"
        );
        assert!(
            text.contains("elohim_sync_docs_enumerated_total"),
            "sync docs-enumerated counter missing — this is the one that makes a \
             corpus-proportional round visible:\n{text}"
        );
        assert!(
            text.contains("elohim_sync_request_outcomes_total"),
            "sync request-outcome counter missing:\n{text}"
        );
        assert!(text.contains("kind=\"list_documents\""), "{text}");
        assert!(text.contains("kind=\"announce_change\""), "{text}");
        assert!(
            text.contains("elohim_acquisition_outcomes_total"),
            "acquisition outcome counter missing — the pull leg's 100% failure was \
             invisible without it:\n{text}"
        );
        assert!(text.contains("outcome=\"transport_failure\""), "{text}");
        // Projection-reconcile heal legibility surfaces render with their labels.
        assert!(
            text.contains("elohim_projection_heal_outcomes_total"),
            "projection heal outcomes counter missing:\n{text}"
        );
        assert!(text.contains("outcome=\"timeout_retried\""), "{text}");
        assert!(text.contains("outcome=\"timeout_exhausted\""), "{text}");
        assert!(text.contains("stream=\"rea\""), "{text}");
        assert!(
            text.contains("elohim_projection_reconcile_gaps"),
            "reconcile gaps gauge missing:\n{text}"
        );
        assert!(
            text.contains("elohim_projection_reconcile_local_total"),
            "reconcile local-total gauge missing:\n{text}"
        );
        assert!(
            text.contains("elohim_shard_redistribute_bytes_missing_total"),
            "redistribute bytes-missing counter missing:\n{text}"
        );
        assert!(
            text.contains("elohim_peer_status_fanout_missed"),
            "peer_status fan-in missed gauge missing:\n{text}"
        );
    }
}
