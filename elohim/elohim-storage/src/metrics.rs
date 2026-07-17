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

/// Record `n` content heads freshly authored by the witness-bootstrap sweep.
pub fn add_content_witness_authored(n: u64) {
    CONTENT_WITNESS_AUTHORED.inc_by(n);
}

/// Record one provide-loop author skipped because no `agent_cid` provider was
/// resolvable (the CID-hardening guard refusing to write a transport id).
pub fn inc_provide_provider_unresolved() {
    PROVIDE_PROVIDER_UNRESOLVED.inc();
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
