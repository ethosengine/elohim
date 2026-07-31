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

    /// Current holder observations bucketed by custody class — the gauge
    /// projection of `CustodyClassCounts` (the typed custody fold in
    /// `elohim-facings`), set by `services::custody_facing`.
    ///
    /// label `class` ∈ `none` | `shelved` | `stocked` | `stocked_warm` |
    /// `unknown` | `observed_lost`. The vocabulary is deliberately six-valued and
    /// NOT summable into a single "holders" number:
    /// - `none` = a COMPLETE commitment lookup measured no active promise
    ///   (measured absence), whereas
    /// - `unknown` = missing/expired observation or an incomplete lookup
    ///   (honest unknown — never a fabricated zero), and
    /// - `observed_lost` = FRESH negative evidence, a distinct fact from both.
    ///
    /// All six series are materialised on every publish, so a bucket sitting at
    /// 0 is distinguishable from an emitter that never ran (series absent).
    pub static ref ELOHIM_CUSTODY_CLASS_COUNT: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_custody_class_count",
            "Current holder observations by custody class (none=measured absence, \
             unknown=honest unknown, observed_lost=fresh negative evidence).",
        ),
        &["class"],
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

    /// Canonical heads ADOPTED rather than re-authored, by the adopt-before-author
    /// pre-flight (`services::head_adoption`) — either resolved canonical from this
    /// node's own conductor, or fetched from an advertising peer and declared
    /// through the own conductor (declare-carries-Record).
    ///
    /// Reads AGAINST `elohim_content_witness_authored_total`: on a peer that keeps
    /// re-minting its own root for a cross-root id, authored climbs every restart
    /// and adopted stays flat. The cure signal is adopted climbing ONCE per id and
    /// then both going quiet — a converged corpus adopts nothing because there is
    /// nothing left to adopt.
    pub static ref CONTENT_HEAD_ADOPTED: IntCounter = IntCounter::new(
        "elohim_content_head_adopted_total",
        "Canonical content heads adopted from the substrate instead of re-authored locally.",
    )
    .unwrap();

    /// Ghost-witness re-author call FAILURES, by class — the two per-row
    /// failure modes minted as saga-06-heads-converge stations (2026-07-26
    /// story-harvest of live Loki evidence, elohim-alpha namespace): previously
    /// visible only by tailing logs, never counted. labels: class =
    ///   "chain_head_moved" — the zome call failed with "Source chain error:
    ///     source chain head has moved" (adam-alpha, seq 7096->7476 in ~6 min):
    ///     a chronically busy own-chain writer races the re-author call.
    ///     Non-fatal + retried next sweep, but can livelock a row forever on a
    ///     chain that never quiets.
    ///   "already_exists" — the create collided with content that already has
    ///     a local entry ("Content with id '…' already exists. Use
    ///     update_content to modify existing entries", jessica-alpha): the
    ///     stale-anchor heal path's create assumed the row never already has a
    ///     local entry; it does.
    /// Both label combos are pre-touched at registration (see `register_all`)
    /// so the series exist in `/metrics` from boot, before either failure mode
    /// has ever fired.
    pub static ref CONTENT_WITNESS_REAUTHOR_FAILED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_witness_reauthor_failed_total",
            "Ghost-witness re-author call failures by class (chain_head_moved | already_exists).",
        ),
        &["class"],
    )
    .unwrap();

    /// Ghost-witness sweeps ABANDONED because the whole sweep exceeded its
    /// wall-clock budget (`WITNESS_SWEEP_BUDGET`) — the third saga-06-heads-
    /// converge station ("sweep exceeded wall-clock budget — abandoned,
    /// resumes next sweep", shem-node conductors). A saturated/slow conductor
    /// drops the WHOLE sweep's progress for that tick, not just one row —
    /// distinct from (and additive to) the per-row `CONTENT_WITNESS_REAUTHOR_FAILED`
    /// class above.
    pub static ref CONTENT_WITNESS_SWEEP_ABANDONED: IntCounter = IntCounter::new(
        "elohim_content_witness_sweep_abandoned_total",
        "Ghost-witness sweeps abandoned after exceeding their wall-clock budget.",
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
    ///         | "failed" | "refreshed" | "refused_declared" | "refused_stale"
    ///         | "no_row".
    ///
    /// The last four are CONTENT-only and were previously uncounted "so the cure
    /// signal is not inflated". That reasoning inverted: giving each its own label
    /// keeps `healed` clean (label-filtered) while making the two questions an
    /// operator actually asks answerable from Prometheus alone —
    ///
    /// - **Is the heal starved, erroring, or refusing?** `refused_declared` /
    ///   `refused_stale` are correct refusals (heal fills-never-moves); a backlog
    ///   made of these is HONEST and permanent until a canonical channel fires.
    ///   `timeout_*` / `failed` / `missing` are the not-refusing classes.
    /// - **Is the heal converging or SPINNING?** `healed` means the declared HEAD
    ///   actually moved; `refreshed` means the own conductor answered the head the
    ///   row already held, so nothing converged. A high `refreshed` rate against a
    ///   non-zero `divergent_anchor` means the peers hold different roots and heal
    ///   can never close it (live 2026-07-26: 1578 heal events in 12h on matthew,
    ///   `elohim-host-landing`'s head never moved).
    ///
    /// Still NOT total-row-accounting: `Refreshed` rows also re-enqueue next sweep,
    /// so dashboard math must not assume outcomes partition attempts exactly.
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

    /// The ADJUDICATED share of `elohim_projection_reconcile_divergent` — the
    /// divergence this sweep is not permitted (or no longer able) to resolve.
    /// label: stream = "rea" | "content" | "collectives".
    ///
    /// Additive on purpose: the existing `..._divergent` series KEEPS meaning the
    /// TOTAL (no class label was added to it, so every existing dashboard and
    /// alert stays valid). Subtract to get the number convergence is actually
    /// gated on:
    ///
    /// ```promql
    /// elohim_projection_reconcile_divergent - elohim_projection_reconcile_divergent_refused
    /// ```
    ///
    /// Two things land in this bucket, both of them the substrate working as
    /// designed rather than a gap:
    ///
    /// - **refused-declared** — the local row already carries a DIFFERENT declared
    ///   head, so heal is FORBIDDEN to move it (`StampOutcome::SkippedDeclared`;
    ///   canonical channels own that move). Permanent until a canonical channel
    ///   fires, and the DOMINANT class on a live peer.
    /// - **retry-exhausted** — the cross-sweep miss ledger has spent the id's
    ///   budget against unchanged peer evidence.
    ///
    /// Why it exists: `elohim_projection_reconcile_converged` was gated on the
    /// TOTAL, so ONE correctly-refused row pinned it at 0 for the process
    /// lifetime (matthew, 12h: 6071 `refused_declared` heal outcomes against 8
    /// `healed`). A gauge that cannot move is not a conservative gauge; it is an
    /// unreadable one. This series is what makes the split legible from
    /// Prometheus alone.
    pub static ref PROJECTION_RECONCILE_DIVERGENT_REFUSED: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_divergent_refused",
            "Last-sweep ADJUDICATED anchor-divergent rows by reconcile stream \
             (heal forbidden to move, or retry budget spent).",
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

    /// Household collective cids discovered by the LAST identity-fill sweep's
    /// `discover_household_pairs` union (local `collectives` projection ∪ the
    /// pod's own source-chain read). A sustained `0` here — previously invisible
    /// at DEBUG — is the direct explanation for a fill sweep that silently does
    /// nothing: no memberships were found on the DHT or in the local projection,
    /// so there was nothing to fill.
    pub static ref IDENTITY_FILL_DISCOVERED_CIDS: IntGauge = IntGauge::new(
        "elohim_identity_fill_discovered_cids",
        "Household collective cids discovered by the last identity-fill sweep.",
    )
    .unwrap();

    /// Local `collectives` rows whose NULL `collective_cid` was stamped from a
    /// discovered household cid (`db::collectives::gap_fill_household_collective_cid_from_discovery`,
    /// called from `services::identity_fill`'s discovery sweep). Closes the
    /// other half of the collectives-arm bootstrap gap
    /// (`backlog-collectives-arm-bootstrap-gap-no-stamped-cid-anywhere`): this
    /// climbing off 0 is the direct cure signal that the peer's
    /// `ProjectionInventory` now has a stamped row to advertise, unblocking the
    /// cross-peer collectives reconcile arm's inventory discovery fleet-wide.
    pub static ref IDENTITY_FILL_COLLECTIVE_CID_STAMPED: IntCounter = IntCounter::new(
        "elohim_identity_fill_collective_cid_stamped_total",
        "Local collectives rows whose NULL collective_cid was stamped from discovered household truth.",
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

    /// Acquisition pins currently RETIRED — pins whose bytes no connected peer
    /// could supply, held back so they stop pinning `pull.caughtUp` false forever.
    ///
    /// The gauge that makes retirement legible instead of magical. `pull.caughtUp`
    /// reaching true is only trustworthy read TOGETHER with this: caught-up with
    /// this at 0 means everything wanted arrived; caught-up with this non-zero
    /// means the queue drained because unsatisfiable wants were set aside, and the
    /// pins are still on the books awaiting re-admission.
    pub static ref ACQUISITION_PINS_RETIRED: IntGauge = IntGauge::new(
        "elohim_acquisition_pins_retired",
        "Acquisition pins currently retired (no connected peer can supply their bytes).",
    )
    .unwrap();

    /// Acquisition pin retirement TRANSITIONS. label: reason = "exhausted"
    /// (retired: every want spent its retry budget against every connected peer)
    /// | "readmitted" (revived: a peer advertised the content again, or the
    /// cooldown elapsed).
    ///
    /// A counter beside the gauge on purpose: a steady gauge can mean "nothing is
    /// happening" or "pins are retiring and reviving at the same rate", and only
    /// the transition rate tells them apart — the same flapping question
    /// `elohim_projection_reconcile_sweeps_total` answers for the reconcile plane.
    pub static ref ACQUISITION_PIN_RETIREMENTS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_acquisition_pin_retirements_total",
            "Acquisition pin retirement transitions by reason.",
        ),
        &["reason"],
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

    /// Sync rounds that short-circuited on a matching corpus digest — the
    /// converged steady state where the opener costs O(1) instead of O(corpus).
    /// A flat-zero value after the digest opener lands means the shortcut never
    /// fires and the optimisation is inert.
    pub static ref SYNC_IN_SYNC_TOTAL: IntCounter = IntCounter::new(
        "elohim_sync_in_sync_total",
        "Sync rounds where a peer answered InSync (digest match, nothing enumerated).",
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
        let _ = REGISTRY.register(Box::new(ELOHIM_CUSTODY_CLASS_COUNT.clone()));
        let _ = REGISTRY.register(Box::new(VIEW_FEDERATION_OUTBOUND.clone()));
        let _ = REGISTRY.register(Box::new(VIEW_FEDERATION_INBOUND_SERVED.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_WITNESS_AUTHORED.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_HEAD_ADOPTED.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_WITNESS_REAUTHOR_FAILED.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_WITNESS_SWEEP_ABANDONED.clone()));
        // Pre-touch both known `class` combos so both series exist in
        // `/metrics` from boot. An `IntCounterVec` label combination only
        // materialises after first touch — the saga-06-heads-converge stations
        // assert the metric is PRESENT in the scrape (`>= 0`), which would
        // otherwise stay unsatisfiable until the failure mode it counts had
        // actually fired at least once.
        CONTENT_WITNESS_REAUTHOR_FAILED
            .with_label_values(&["chain_head_moved"])
            .inc_by(0);
        CONTENT_WITNESS_REAUTHOR_FAILED
            .with_label_values(&["already_exists"])
            .inc_by(0);
        let _ = REGISTRY.register(Box::new(PROVIDE_PROVIDER_UNRESOLVED.clone()));
        let _ = REGISTRY.register(Box::new(SALVAGE_PROVIDER_UNRESOLVED.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_KEY_SUPERSEDE.clone()));
        let _ = REGISTRY.register(Box::new(SHARD_PUSH_PEER_UNRESOLVED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_HEAL_OUTCOMES.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_GAPS.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_LOCAL_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_EXHAUSTED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_DIVERGENT.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_DIVERGENT_REFUSED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_SWEEPS.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_CONVERGED.clone()));
        let _ = REGISTRY.register(Box::new(SHARD_REDISTRIBUTE_BYTES_MISSING.clone()));
        let _ = REGISTRY.register(Box::new(PEER_STATUS_FANOUT_MISSED.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_FILL_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_FILL_LAST_WRITES.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_FILL_DISCOVERED_CIDS.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_FILL_COLLECTIVE_CID_STAMPED.clone()));
        let _ = REGISTRY.register(Box::new(CUSTODY_ANNOUNCE_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_ROUNDS.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_REQUESTS.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_DOCS_ENUMERATED.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_REQUEST_OUTCOMES.clone()));
        let _ = REGISTRY.register(Box::new(ACQUISITION_OUTCOMES.clone()));
        let _ = REGISTRY.register(Box::new(ACQUISITION_PINS_RETIRED.clone()));
        let _ = REGISTRY.register(Box::new(ACQUISITION_PIN_RETIREMENTS.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_IN_SYNC_TOTAL.clone()));
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

/// Counts sync rounds that short-circuited on a matching corpus digest —
/// the converged steady state where the opener costs O(1) instead of O(corpus).
/// A flat-zero value after the digest opener lands means the shortcut never
/// fires and the optimisation is inert.
pub fn inc_sync_in_sync() {
    SYNC_IN_SYNC_TOTAL.inc();
}

/// Record one acquisition (pull-leg) outcome ("fetched" | "fetch_error" |
/// "transport_failure" | "blob_unavailable" | "store_failed" | "no_db_pool" |
/// "no_db_conn" | "unexpected_response").
///
/// `unexpected_response` is the peer answering with a variant the fetch path does
/// not model — in practice `ShardResponse::Error(_)` from the responder's own DB
/// layer. Before it existed that case fell through a bare `debug!` and leaked the
/// in-flight slot outright (see the catch-all arm in `p2p::mod`).
pub fn inc_acquisition_outcome(outcome: &str) {
    ACQUISITION_OUTCOMES.with_label_values(&[outcome]).inc();
}

/// Record `n` content heads freshly authored by the witness-bootstrap sweep.
pub fn add_content_witness_authored(n: u64) {
    CONTENT_WITNESS_AUTHORED.inc_by(n);
}

/// Record one canonical head ADOPTED (own-conductor resolve, or a peer's head
/// declared through the own conductor) instead of re-authored locally.
pub fn inc_content_head_adopted() {
    CONTENT_HEAD_ADOPTED.inc();
}

/// Record one ghost-witness re-author call failure of `class` — one of
/// "chain_head_moved" | "already_exists" (see
/// [`CONTENT_WITNESS_REAUTHOR_FAILED`]). Both label combos are already
/// pre-touched at registration, so calling this only ever increments an
/// existing series.
pub fn inc_content_witness_reauthor_failed(class: &str) {
    CONTENT_WITNESS_REAUTHOR_FAILED
        .with_label_values(&[class])
        .inc();
}

/// Record one ghost-witness sweep abandoned after exceeding its wall-clock
/// budget (see [`CONTENT_WITNESS_SWEEP_ABANDONED`]).
pub fn inc_content_witness_sweep_abandoned() {
    CONTENT_WITNESS_SWEEP_ABANDONED.inc();
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
/// discovery) and `local_total` (local projection rows). `stream` is "rea" |
/// "content" | "collectives".
///
/// `divergent` is the TOTAL; `divergent_refused` is its adjudicated share (see
/// [`PROJECTION_RECONCILE_DIVERGENT_REFUSED`]) — `divergent_refused <= divergent`
/// always, and the difference is what gates convergence.
pub fn set_projection_reconcile_gauges(
    stream: &str,
    gaps: u64,
    local_total: u64,
    exhausted: u64,
    divergent: u64,
    divergent_refused: u64,
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
    PROJECTION_RECONCILE_DIVERGENT_REFUSED
        .with_label_values(&[stream])
        .set(divergent_refused as i64);
}

/// 1 when the peer holds what its peers advertised, 0 otherwise. Pure so the
/// three-condition rule is testable without a registry — and so it cannot drift
/// from `GapCounts::converged`, which computes the same rule minus divergence.
///
/// `divergent_actionable` is the UNADJUDICATED divergence — the total MINUS the
/// share heal is forbidden to move (a canonical channel owns the row's declared
/// head) or has exhausted its retry budget against. Passing the TOTAL here is the
/// bug this parameter name now guards: on a live peer the refused class is the
/// dominant one and permanent until a canonical channel fires, so a total-gated
/// gauge reads 0 forever no matter how well the heal leg works. Divergence
/// nobody has adjudicated still defeats convergence — that half is the honest
/// half and is kept.
pub fn converged_gauge_value(
    counts: &crate::p2p::reconcile_rails::GapCounts,
    divergent_actionable: usize,
) -> i64 {
    i64::from(counts.converged && divergent_actionable == 0)
}

/// Publish one completed reconcile sweep: advance the sweep denominator and
/// republish whether this peer actually converged.
///
/// Called from `ProjectionReconcileState::publish_sweep`, so the metric and the
/// `/p2p/status` field are written from the same place and cannot disagree —
/// the failure mode that let `/health` and `/p2p/status` drift 12x apart.
pub fn record_reconcile_sweep(
    counts: &crate::p2p::reconcile_rails::GapCounts,
    divergent_actionable: usize,
) {
    PROJECTION_RECONCILE_SWEEPS.inc();
    PROJECTION_RECONCILE_CONVERGED.set(converged_gauge_value(counts, divergent_actionable));
}

/// Record one Part-B redistribution attempt skipped because the candidate's blob
/// bytes were not local (the closed placement-gap enum cannot carry this cause, so
/// the precise signal lives here — see [`SHARD_REDISTRIBUTE_BYTES_MISSING`]).
pub fn inc_shard_redistribute_bytes_missing() {
    SHARD_REDISTRIBUTE_BYTES_MISSING.inc();
}

/// Publish the count of currently-retired acquisition pins.
pub fn set_acquisition_pins_retired(count: u64) {
    ACQUISITION_PINS_RETIRED.set(count as i64);
}

/// Record `n` acquisition-pin retirement transitions of `reason` ("exhausted" |
/// "readmitted").
///
/// Both label series are materialised on the first call of each (an `inc_by(0)`
/// registers the series without moving it), so "no pin has ever retired" stays
/// distinguishable from "the emitter never ran" — the same diagnosis-ambiguity
/// lesson as [`inc_identity_fill`].
pub fn inc_acquisition_pin_retirements(reason: &str, n: u64) {
    ACQUISITION_PIN_RETIREMENTS
        .with_label_values(&[reason])
        .inc_by(n);
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

/// Publish the household collective cid count discovered by the last
/// identity-fill sweep's `discover_household_pairs` union.
pub fn set_identity_fill_discovered_cids(count: u64) {
    IDENTITY_FILL_DISCOVERED_CIDS.set(count as i64);
}

/// Record one local `collectives` row stamped from discovered household truth
/// (`db::collectives::HouseholdCidGapFillOutcome::Stamped`).
pub fn inc_identity_fill_collective_cid_stamped() {
    IDENTITY_FILL_COLLECTIVE_CID_STAMPED.inc();
}

/// Publish the six custody-class buckets from one fold of the custody-observation
/// relation (`services::custody_facing`).
///
/// ALL six series are set on every call — including the zeros — so "this bucket
/// is empty" stays distinguishable from "the emitter never ran" (the same
/// diagnosis-ambiguity lesson as [`inc_identity_fill`]). The buckets are not
/// summable into an unlabeled holder count: `none` is a measured absence,
/// `unknown` an honest unknown, `observed_lost` fresh negative evidence.
pub fn set_custody_class_counts(
    counts: &elohim_facings::folds::operational_weave::CustodyClassCounts,
) {
    for (class, value) in [
        ("none", counts.none),
        ("shelved", counts.shelved),
        ("stocked", counts.stocked),
        ("stocked_warm", counts.stocked_warm),
        ("unknown", counts.unknown),
        ("observed_lost", counts.observed_lost),
    ] {
        ELOHIM_CUSTODY_CLASS_COUNT
            .with_label_values(&[class])
            .set(value as i64);
    }
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
    fn divergence_heal_is_forbidden_to_move_does_not_defeat_convergence_but_unadjudicated_does() {
        // The live shape this split cures (matthew, 12h): 6071 divergent rows,
        // EVERY one of them a row whose local declared head differs — heal is
        // forbidden to move those (canonical channels own them), so it healed 8.
        // Gating the gauge on the TOTAL made `converged` structurally
        // unreachable: the refusals are correct and permanent until a canonical
        // channel fires, so no amount of healing could ever flip it.
        let healed = GapCounts {
            pending: 0,
            completed: 5,
            failed: 0,
            caught_up: true,
            exhausted: 0,
            converged: true,
        };

        // 6071 divergent, all 6071 adjudicated → actionable 0 → CONVERGED.
        // The total is still published on `..._divergent`; the adjudicated share
        // on `..._divergent_refused`. Nothing is hidden, only re-classified.
        let actionable = 6071usize.saturating_sub(6071);
        assert_eq!(
            converged_gauge_value(&healed, actionable),
            1,
            "divergence heal is FORBIDDEN to move must not defeat convergence"
        );

        // The honest half is kept: ONE row nobody has adjudicated still defeats
        // it. This is the guard against the split becoming a whitewash.
        let actionable = 6071usize.saturating_sub(6070);
        assert_eq!(
            converged_gauge_value(&healed, actionable),
            0,
            "unadjudicated divergence must still defeat convergence"
        );
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
        set_projection_reconcile_gauges("rea", 62, 0, 61, 0, 0);
        // 1860 divergent of which 1855 are adjudicated (heal forbidden to move)
        // — the split the `..._divergent_refused` series exists to publish.
        set_projection_reconcile_gauges("content", 1956, 4158, 0, 1860, 1855);
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
            "unexpected_response",
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
            text.contains("elohim_projection_reconcile_divergent_refused"),
            "adjudicated-divergence gauge missing — without it the total cannot \
             be read against what convergence is actually gated on:\n{text}"
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

    // ── Ghost-witness sweep failure legibility (saga-06-heads-converge stations) ──

    #[test]
    fn content_witness_reauthor_failed_classes_are_pretouched_at_boot() {
        register_all();

        // CRITICAL: both `class` label combos must be present in the scrape
        // from registration alone — before either failure mode has ever
        // fired. An IntCounterVec label combination only materialises after
        // first touch, and the saga-06-heads-converge scenarios assert the
        // metric is PRESENT (`>= 0`), not merely that it eventually appears
        // once a sweep hits that failure class.
        let text = gather_text();
        assert!(
            text.contains("elohim_content_witness_reauthor_failed_total"),
            "reauthor-failed counter missing:\n{text}"
        );
        assert!(
            text.contains("class=\"chain_head_moved\""),
            "chain_head_moved class not pre-touched at registration:\n{text}"
        );
        assert!(
            text.contains("class=\"already_exists\""),
            "already_exists class not pre-touched at registration:\n{text}"
        );
        // Plain IntCounter: registration alone suffices (no pre-touch needed —
        // it has no labels to materialise).
        assert!(
            text.contains("elohim_content_witness_sweep_abandoned_total"),
            "sweep-abandoned counter missing:\n{text}"
        );
    }

    #[test]
    fn content_witness_reauthor_failed_and_sweep_abandoned_increment() {
        register_all();
        inc_content_witness_reauthor_failed("chain_head_moved");
        inc_content_witness_reauthor_failed("already_exists");
        inc_content_witness_sweep_abandoned();

        let text = gather_text();
        assert!(text.contains("elohim_content_witness_reauthor_failed_total"));
        assert!(text.contains("class=\"chain_head_moved\""), "{text}");
        assert!(text.contains("class=\"already_exists\""), "{text}");
        assert!(text.contains("elohim_content_witness_sweep_abandoned_total"));
    }
}
