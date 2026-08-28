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
    Encoder, GaugeVec, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec, Opts, Registry, TextEncoder,
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

    /// Active bindings that were SELF-ASSERTED at the moment an economic
    /// attribution join asked for them (habit `identity-cross-signed`).
    ///
    /// This is the measure that makes the habit's red honest. An
    /// `AgentPeerBinding` claims "agent X owns transport endpoint Y"; today that
    /// claim carries `STAGE1_SIGNATURE_SENTINEL`, so a gossiped spoof
    /// (agent_cid = attacker, transport_id = victim) would credit the attacker
    /// for the victim's work. label: posture = "observe" (counted and served —
    /// the default, so nothing on a fleet changes) | "enforce" (counted and
    /// REFUSED). The flip to green is this counter reaching zero under
    /// `posture="enforce"` — i.e. every binding an attribution join touches is
    /// cross-signed, not merely that nobody looked.
    pub static ref ATTRIBUTION_UNVERIFIED_BINDINGS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_attribution_unverified_bindings_total",
            "Self-asserted (non-cross-signed) peer identity bindings reaching an economic attribution join.",
        ),
        &["posture"],
    )
    .unwrap();

    /// Attribution joins performed, and bindings they examined — the DENOMINATOR
    /// under `ATTRIBUTION_UNVERIFIED_BINDINGS` (habit `identity-cross-signed`).
    ///
    /// The unverified counter alone cannot carry the habit, because a zero is
    /// ambiguous between two opposite fleet states: every binding examined was
    /// cross-signed (the green), or no attribution join examined a binding at all
    /// (silence). Its own doc names that hazard — "not merely that nobody looked"
    /// — but nothing in the exposition could express it, so the flip-to-green
    /// measure was unfalsifiable. MEASURED 2026-08-20: the numerator has read
    /// zero across all 56 alpha pod instances for 7 days, with no
    /// `posture="enforce"` series at all — while the standing analysis predicts
    /// a NON-zero count (minting is libp2p-only; alpha has run dual since
    /// 2026-08-05, and an iroh-kind row classifies unverified permanently).
    /// A prediction of non-zero and 7 days of zero cannot both be right, and
    /// the numerator alone cannot say which gave. Flipping the posture on it
    /// would satisfy the habit having verified nothing.
    ///
    /// Read all three together. The three states are distinguishable only as a
    /// conjunction:
    ///
    ///   joins == 0                          → nobody looked; the zero is silence
    ///   joins > 0, examined == 0            → looked, reached no binding
    ///   examined > 0, unverified == 0       → the habit's actual green
    ///
    /// Both are incremented UNCONDITIONALLY, including when a join finds nothing
    /// — a counter that only fires on the interesting path is how the numerator
    /// got into this state.
    pub static ref ATTRIBUTION_JOINS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_attribution_joins_total",
            "Economic attribution joins that consulted the peer-identity binding cut.",
        ),
        &["posture"],
    )
    .unwrap();

    /// Bindings examined by an attribution join, admitted or refused. Counted
    /// before the posture filter, so `Enforce` cannot shrink the denominator it
    /// is being judged against. See [`ATTRIBUTION_JOINS`].
    pub static ref ATTRIBUTION_BINDINGS_EXAMINED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_attribution_bindings_examined_total",
            "Peer identity bindings examined by an economic attribution join (denominator).",
        ),
        &["posture"],
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

    /// `ContentHeadRecord` answers served WITHOUT their Record bytes — the
    /// honest-absence degrade (the head hash is still served; only the carried
    /// bytes are missing). label: cause = [`HeadRecordDegraded`]
    /// ("no_record" | "conductor_error" | "budget_elapsed").
    ///
    /// `no_record` was the ONLY uncounted collapse until 2026-08-03 — and it is
    /// the STRUCTURAL one: the conductor answered cleanly and holds no record at
    /// all, which no amount of budget or capacity relief can change. Leaving it
    /// silent made the phantom class (ids whose bytes exist nowhere) and the
    /// saturation class arithmetically inseparable from meters alone, so the
    /// remedy an operator reached for was a coin flip. See
    /// `genesis/data/timeline/backlog/adopt-before-author-evidence-starvation.md`.
    ///
    /// `budget_elapsed` is the direct live signal for the responder bound
    /// (`HEAD_RECORD_CONDUCTOR_TIMEOUT`): it counts the asks a saturated
    /// conductor could not answer inside the responder's own budget. Before the
    /// bound existed those same asks were invisible here and surfaced instead as
    /// `elohim_view_federation_outbound_total{result="timeout"}` at the
    /// REQUESTER — i.e. as a transport fault indistinguishable from an offline
    /// peer, which is precisely why fleet-wide adoption failure went undiagnosed.
    /// A rise here alongside a FALL in requester-side timeouts is the cure
    /// working; a rise in BOTH means the budget is too tight.
    pub static ref CONTENT_HEAD_RECORD_DEGRADED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_head_record_degraded_total",
            "ContentHeadRecord answers served hash-only (no carried Record) by cause.",
        ),
        &["cause"],
    )
    .unwrap();

    /// Hold/Contest decisions downgraded to Author by the ghost-declaration
    /// decay arm (`head_adoption::ghost_decay_authorizes_author`) — each one is
    /// a phantom declaration (dead conductor incarnation) positively falsified
    /// and released to the witness author path. Flag-gated
    /// (`ELOHIM_GHOST_DECLARATION_DECAY`); a nonzero series on a pod IS the
    /// evidence the flag is doing work there.
    pub static ref GHOST_DECLARATION_DECAY_AUTHOR: IntCounter = IntCounter::new(
        "elohim_content_ghost_decay_author_total",
        "Pre-flight Hold/Contest decisions released to Author by ghost-declaration decay.",
    )
    .unwrap();

    /// The NEGATIVE twin of [`GHOST_DECLARATION_DECAY_AUTHOR`]: Hold/Contest
    /// decisions the decay arm considered and did NOT release, by the leg that
    /// blocked (`head_adoption::ghost_decay_blocked_leg`).
    ///
    /// A silent `_author_total` is ambiguous on its own — a pod draining no
    /// phantoms looks identical whether the flag never reached it, its rows are
    /// settled, no peer advertises them this sweep, or the advertiser evidence
    /// has not yet stood the dwell. Those are four different operator actions
    /// (redeploy · nothing · supply · wait), so the refusal reason is counted
    /// rather than inferred. `disabled` counting up IS a pod running without the
    /// flag; `evidence_not_stood` climbing while `author_total` is flat is the
    /// dwell doing its job, not a stall.
    pub static ref CONTENT_GHOST_DECAY_BLOCKED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_ghost_decay_blocked_total",
            "Hold/Contest decisions the ghost-declaration decay arm did not release, by blocking leg.",
        ),
        &["leg"],
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

    /// Own-conductor head answers by ELECTION TIER, counted on every
    /// `resolve_content_head_local` reply the heal loop reads.
    ///
    /// THE MISSING METER (2026-08-02). `ContentHeadWire.canonical` was consumed
    /// to pick a stamp mode and then discarded — never counted — so "is the
    /// canonical-head anchor empty for this class?" could not be answered from
    /// telemetry at all. A fleet stuck in two-way declared divergence and a fleet
    /// whose election merely hasn't gossiped in look IDENTICAL on every other
    /// gauge; this is the one that separates them.
    ///
    /// - `none`   — the root-author FALLBACK election: no canonical link resolved
    ///   (either none was ever declared, or the winner's target has not gossiped
    ///   in). Sustained ~100% `none` against a non-zero `divergent_refused` means
    ///   NO ELECTION EXISTS to arbitrate — the supply side is the blocker, not
    ///   the selector.
    /// - `staging` / `earned` — a canonical winner resolved, at that tier. The
    ///   cure signal is `none` falling as these rise.
    pub static ref CONTENT_CANONICAL_ANSWERS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_canonical_answers_total",
            "Own-conductor content HEAD answers by canonical-election tier.",
        ),
        &["tier"],
    )
    .unwrap();

    /// WHY a `HealCanonical` stamp refused to move an already-declared row.
    ///
    /// A sibling of `elohim_projection_heal_outcomes_total{outcome="refused_stale"}`
    /// rather than a label on it: adding `reason` to that family would multiply
    /// the cardinality of every other outcome for one arm's benefit. The totals
    /// reconcile — this family's sum equals that series.
    ///
    /// - `stored_null` — the row carries NO election ordering to compare against
    ///   (its declaration came from a channel that NULLs it: the deploy PATCH's
    ///   `HeadElection::Declare`, or a `ContentHeadDeclared` signal). Under the
    ///   three-tier rule this is now a MOVE, so a nonzero rate here after the
    ///   cure means an answer arrived carrying no election either.
    /// - `not_newer` — both sides carry an election and the incoming one is not
    ///   strictly newer. The honest, converged steady state.
    /// - `tier` — incoming is STAGING and the row already holds an EARNED
    ///   declaration. Correct refusal; earned is not displaced by scaffold.
    pub static ref PROJECTION_REFUSED_STALE_REASONS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_projection_heal_refused_stale_total",
            "HealCanonical stamps that kept the adopted head, by refusal reason.",
        ),
        &["reason"],
    )
    .unwrap();

    /// Canonical-head LINKS this node minted on the DHT, by the channel that
    /// caused it — the SUPPLY side of the election.
    ///
    /// Reads against `elohim_content_canonical_answers_total{tier="none"}`: links
    /// minted is the input, canonical answers is the output. Supply flat at zero
    /// while `none` sits at 100% is the two-way-declared deadlock — every peer
    /// holds a declaration, so the only automated minter (`adopt_peer`, which
    /// requires an UNDECLARED local row) never fires and no election is ever
    /// created for the arbiter to run on.
    ///
    /// Sources: `adopt_peer` (the adopt-before-author pre-flight), `http`
    /// (`POST /db/content/{id}/canonical-head`, i.e. the deploy's stage-spa-blob
    /// declare), and the two CONTEST shapes —
    /// - `contest_peer_head` — nominated the peer's head, carried over
    ///   view-federation and proven in wasm by `validate_carried_record`;
    /// - `contest_self_head` — nominated THIS node's own declared head because
    ///   the peer's was not retrievable here (symmetric candidacy: the peer
    ///   nominates its own from its side and the DHT election picks).
    ///
    /// Watching the two contest labels against each other is how we learn WHICH
    /// shape actually converges the fleet.
    ///
    /// ADDITIVE (operator-reserved, default OFF): `adopt_before_author` — minted
    /// by a node that holds NO local chain for the id, from a carried record the
    /// zome proved in wasm. It is the ONLY label that can be non-zero for the
    /// both-sides-missing residual, so it doubles as the flag's own live probe:
    /// zero means the flag is off (or its precondition — validated carried bytes
    /// for a chainless id — never occurred), never that the arm silently failed.
    /// A failed attempt is counted separately as
    /// `elohim_content_contest_failed_total{class="adopt_refused"}`.
    pub static ref CONTENT_CANONICAL_LINKS_MINTED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_canonical_links_minted_total",
            "Canonical-head declaration links minted through this node, by source channel.",
        ),
        &["source"],
    )
    .unwrap();

    /// Contest attempts that minted NOTHING, by failure class — the decisive
    /// discrimination for the two-way-declared class.
    ///
    /// The predecessor of this metric was a single log line that merged two
    /// completely different failures into one sentence, which is precisely why a
    /// full observation window could not tell them apart:
    ///
    /// - `no_local_chain` — this conductor holds no chain for the id AT ALL.
    ///   `declare_canonical_head_inner`'s first gate is target-independent, so
    ///   NO candidate shape can pass it. Dominance here means contest is starved
    ///   upstream (the conductor needs the id witnessed before any election can
    ///   be supplied) and is the signal that a coordinator-zome gate decision is
    ///   required — it is NOT fixable in storage.
    /// - `not_retrievable` — the chain exists but the peer's head action could
    ///   not be resolved, AND we had carried bytes. Self-candidacy handles this;
    ///   a count here means even that failed.
    /// - `fetch_none` — same wall, but we had NO record to carry (the fetch
    ///   returned nothing). Distinguished from `not_retrievable` because the
    ///   remedy is different: this points at the head-record transport, that one
    ///   at the conductor's DHT view.
    /// - `declare_error` — anything else the conductor refused. Always paired
    ///   with a WARN carrying the verbatim message.
    ///
    /// Success is deliberately NOT counted here — it lives on
    /// `elohim_content_canonical_links_minted_total`, so the two series can be
    /// read as attempted-vs-landed without double-counting.
    pub static ref CONTENT_CONTEST_FAILED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_contest_failed_total",
            "Contest attempts that minted no canonical-head candidate, by failure class.",
        ),
        &["class"],
    )
    .unwrap();

    /// Contest attempts DECLINED before they were made, because the id is
    /// serving a backoff for a PREDICTABLE repeat failure
    /// (`services::contest_backoff`). Read beside
    /// `elohim_content_contest_failed_total`: failures are budget SPENT proving
    /// something already known; skips are that budget RECLAIMED.
    ///
    /// - `no_local_chain_backoff` — a previous contest hit the zome's
    ///   target-independent no-chain gate. Cleared early by
    ///   `contest_backoff::note_local_chain_arrived` when an author path lands a
    ///   chain, and in every case by window expiry.
    /// - `self_candidacy_backoff` — the self-head fallback was refused too.
    ///
    /// A rising skip series with a FALLING failure series is the F-B lever
    /// working. A rising skip series with a flat `canonical_links_minted_total`
    /// means the backoff is holding back ids that would have succeeded — shorten
    /// the window (`CONTEST_BACKOFF_SECONDS`) or set it to 0 to disable.
    pub static ref CONTENT_CONTEST_SKIPPED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_contest_skipped_total",
            "Contest attempts skipped without a conductor round-trip, by backoff reason.",
        ),
        &["reason"],
    )
    .unwrap();

    /// How each adopt-before-author sweep ENDED. The fan-out lever's honesty
    /// meter: `budget_elapsed` means the 120s wall clock cut the sweep short, so
    /// the per-tick cap was NOT the binding constraint and raising
    /// `ADOPT_CONTEST_FANOUT` cannot help until the conductor answers faster.
    /// `completed` means the sweep drained its candidate slice within budget.
    pub static ref CONTENT_ADOPT_SWEEP: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_adopt_sweep_total",
            "Adopt-before-author sweeps, by how the sweep ended.",
        ),
        &["outcome"],
    )
    .unwrap();

    /// Head-plane adoption decisions the trust gradient PRICED, by the
    /// verification depth it chose (Q7 — `trust::adoption_pricing`).
    ///
    /// This is the probe that says whether ceremony compression is actually
    /// biting. `accept_with_provenance` moving means a declared Simulacra
    /// grant is removing per-id evidence round-trips (the election probe and
    /// the peer `ContentHeadRecord` fetch); a fleet sitting entirely on
    /// `full_chain` is verifying exactly as it did before the seam existed.
    /// `delta_verify` is a measured zero on this consumer — the head plane has
    /// no delta path to route to, and the series exists so that stays visible
    /// rather than absent.
    pub static ref TRUST_PRICED_ADOPTIONS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_trust_priced_adoptions_total",
            "Head-plane adoption decisions priced by the trust gradient, by verification depth.",
        ),
        &["depth"],
    )
    .unwrap();

    /// Rows whose head MOVED to obey a DHT election this node could SEE but
    /// could not resolve locally — the last-mile of convergence for the
    /// conductor-missing class.
    ///
    /// `path="carried"`: the election was read from the own conductor
    /// (`resolve_canonical_election`), the winner's bytes were fetched from a
    /// peer, the ZOME proved them against the elected action, and the stamp then
    /// moved the row under the election's own ordering.
    ///
    /// This is the series that answers "is the fleet actually converging?" for
    /// the ~24.9k/2h conductor-missing class. Contest supplies the election;
    /// this obeys it. Both must be non-zero for a two-way divergence to close on
    /// a pod that holds no chain.
    pub static ref CONTENT_ELECTION_OBEYED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_election_obeyed_total",
            "Rows moved to the DHT-elected canonical head, by obey path.",
        ),
        &["path"],
    )
    .unwrap();

    /// Election-obey attempts that did NOT move the row, by failure class.
    ///
    /// - `fetch` — no peer served bytes for the elected action (no record came
    ///   back, or the peer served a DIFFERENT head than the one elected).
    /// - `validate` — bytes came back but the ZOME refused them: hash mismatch,
    ///   bad author signature, entry↔action mismatch, or the carried Content
    ///   belongs to another id. Any count here is worth reading the WARN for —
    ///   it means a peer served bytes that do not prove what they claim.
    /// - `stamp_refused` — proven bytes, but `canonical_move_verdict` declined
    ///   the move (the row already obeys an equal-or-newer election, or an
    ///   earned one). A correct refusal, not an error.
    ///
    /// Success is NOT counted here — see `elohim_content_election_obeyed_total`.
    pub static ref CONTENT_ELECTION_OBEY_FAILED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_election_obey_failed_total",
            "Election-obey attempts that did not move the row, by failure class.",
        ),
        &["class"],
    )
    .unwrap();

    /// Election-obey PROBES, by how each probe ended — the denominator, plus the
    /// three EARLY exits that [`CONTENT_ELECTION_OBEY_FAILED`] deliberately does
    /// not cover.
    ///
    /// **The two meters answer different questions and must never be folded.**
    /// `obey_failed` is scoped, by its own doc, to "we HAD an election and the
    /// row still did not move" (fetch / validate / stamp_refused). This one is
    /// scoped to "did the probe ever GET an election, and a courier, to act on?"
    /// Folding them destroys both: a run of `no_election` would read as
    /// obey-failures, and `fetch`/`validate`/`stamp_refused` would stop meaning
    /// what they say.
    ///
    /// Labels (`outcome`):
    ///
    /// - `attempted` — the DENOMINATOR: one per probe entering
    ///   `try_obey_visible_election`. NOT disjoint from the three exits; each of
    ///   them is a subset of it. `attempted - (no_election + resolve_error +
    ///   no_courier)` is exactly the population that reached the fetch arm, and
    ///   that population is fully accounted for by
    ///   `elohim_content_election_obeyed_total` +
    ///   `elohim_content_election_obey_failed_total` — so the outcome set closes
    ///   by arithmetic with no silent remainder.
    /// - `no_election` — `resolve_canonical_election` answered `Ok(None)`: this
    ///   conductor sees no election for the id. Dominant ⇒ an
    ///   **election-visibility wall** (canonical-head links have not gossiped
    ///   in) — a gossip/link-layer question, not an obey-arm one.
    /// - `resolve_error` — the call itself failed. Dominant ⇒ a
    ///   **coordinator-swap wall**: the shipped `resolve_canonical_election`
    ///   extern is not on the running conductor. An ops question (the DNA hash
    ///   is blind to coordinator zomes, so a shipped fix can sit unlanded), not
    ///   a code one.
    /// - `no_courier` — an election IS visible, but no peer hint / no head-record
    ///   fetcher can supply the winner's bytes. Dominant ⇒ a **hint-supply
    ///   wall**.
    ///
    /// **Forcing incident (2026-08-03 live diagnosis).** The obey arm ran ~900
    /// probes/hr and died 100% at `resolve_canonical_election` through two exits
    /// that incremented nothing and logged at `debug!` — a level this deployment
    /// drops. Two shifts read the resulting flat-zero
    /// `elohim_content_election_obeyed_total` as "the arm is idle" when it was
    /// in fact "the arm is failing on every single row."
    ///
    /// All four label combinations are PRE-TOUCHED at registration (see
    /// [`register_all`]). A structurally-absent series is what hid this: a
    /// zero-valued series reads *measured zero*, an absent one reads *never
    /// measured*, and only one of those is a fact. Pre-touching does not make
    /// any label structurally constant (the C8 hazard) — every one of the four
    /// is incremented by a real branch of
    /// `services::head_adoption::try_obey_visible_election`.
    pub static ref CONTENT_ELECTION_OBEY_PROBE: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_election_obey_probe_total",
            "Election-obey probes by outcome (attempted denominator + the three early exits).",
        ),
        &["outcome"],
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

    /// Custody pledges ROTATED: a successor `custody-blob` commitment naming the
    /// content's CURRENT blob was notarized AND its stale predecessor was marked
    /// `superseded`. The complete-ceremony success series — a rotation that
    /// authored but failed to supersede counts
    /// [`CustodyRotationSkip::SupersedeFailed`] instead, so this counter never
    /// over-reports a half-applied rotation.
    ///
    /// Denominator partner: [`CUSTODY_ROTATION_SKIPPED`] (every examined pledge
    /// that did not rotate, by typed reason), so a rotation rate is readable
    /// rather than inferred from a bare success count.
    pub static ref CUSTODY_ROTATION_AUTHORED: IntCounter = IntCounter::new(
        "elohim_custody_rotation_authored_total",
        "Custody pledges rotated onto the content's current blob (successor notarized + predecessor superseded).",
    )
    .unwrap();

    /// Examined custody pledges that did NOT rotate, by
    /// [`CustodyRotationSkip`]. Every label combination is pre-touched at
    /// registration (see `register_all`) so each series reads as a measured zero
    /// from boot rather than an absent series.
    pub static ref CUSTODY_ROTATION_SKIPPED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_custody_rotation_skipped_total",
            "Custody pledges examined but not rotated, by reason (no_content_id | content_blob_absent | not_divergent | successor_exists | bytes_absent | author_failed | supersede_failed).",
        ),
        &["reason"],
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
    ///         | "missing_deferred" | "failed" | "refreshed" | "refused_declared"
    ///         | "refused_stale" | "no_row" | "deferred_to_adopt" | "call_failed"
    ///         | "unattempted".
    ///
    /// `call_failed` (a BATCH conductor call answered nothing for a whole chunk)
    /// and `unattempted` (the batch resolver never started an id, a per-chunk
    /// budget/ceiling stop) are the two content-arm paths B1/A4 made legible:
    /// before, a call-level failure broke the WHOLE heal leg via an unconditional
    /// `break` and neither series existed, so a struggling conductor read as
    /// silence rather than as a countable, retried-next-chunk event.
    ///
    /// The last four (before these two) are CONTENT-only and were previously
    /// uncounted "so the cure
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

    /// Last-sweep count of locally-ANCHORED rows held at a reach outside
    /// `DISTRIBUTION_SAFE_REACH` while a peer advertises the same id
    /// (`ContentGap::ReachScoped`). label: stream = "content".
    ///
    /// Reach is earned and per-node, so this is a NORMAL steady-state population,
    /// not a defect — it is published because it used to be invisible: these rows
    /// were classified as un-anchored gaps and re-enqueued for a heal that could
    /// never resolve them (alpha, 2026-08-19: ~2584 of matthew's 2585 "gaps").
    /// A number here that tracks the old `gaps` value is the cure landing, not a
    /// regression.
    pub static ref PROJECTION_RECONCILE_REACH_SCOPED: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_reach_scoped",
            "Last-sweep anchored-but-scoped-reach rows a peer advertises, by stream.",
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

    /// Whether the last reconcile sweep actually OBSERVED `stream`'s state (1)
    /// or short-circuited on a DB/query error or zero peers answered (0). Set
    /// UNCONDITIONALLY every sweep, unlike the value gauges above (`gaps`,
    /// `local_total`, `exhausted`, `divergent`, `divergent_refused`), which are
    /// gated on this bit: an unmeasured arm returns all-zero counts that read
    /// identically to a healthy in-sync arm, so publishing them anyway would
    /// write a FALSE zero over the last real sample. Watch this beside a
    /// plateaued value gauge — a flat `gaps` reading could mean "converged" or
    /// "the measurement stopped landing", and only this series tells them
    /// apart.
    pub static ref PROJECTION_RECONCILE_MEASURED: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_measured",
            "Whether the last reconcile sweep observed this stream's state (1) or was unmeasured (0).",
        ),
        &["stream"],
    )
    .unwrap();

    /// Persistent, cross-sweep known-gap count per reconcile stream — hosted on
    /// the `MissLedger` pod state (`MissLedger::tracked` minus
    /// [`Self::PROJECTION_RECONCILE_KNOWN_DIVERGENT`]'s per-stream count), NOT
    /// the per-sweep discovery tracker `elohim_projection_reconcile_gaps`
    /// samples.
    ///
    /// The distinction matters: `..._gaps` is a ROTATING-PAGE sample (window =
    /// 2000-row page, ~3 sweeps/rotation) — it tells you what THIS page looked
    /// like, not whether the backlog is actually draining. This gauge reflects
    /// the ledger's memory across sweeps, so it is drain-readable: a real cure
    /// makes it fall over time; a page-rotation artifact does not move it.
    ///
    /// Honesty bound: the known set is complete only after one full window
    /// rotation (~3 sweeps ≈ 15 min at the default sweep interval); an id's
    /// removal (via [`MissLedger::resolved`]) lags by up to one rotation too,
    /// since a healed id is only re-observed once its page comes back around.
    pub static ref PROJECTION_RECONCILE_KNOWN_GAPS: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_known_gaps",
            "Persistent cross-sweep known-gap count by reconcile stream (MissLedger pod state; \
             complete only after one full window rotation, ~3 sweeps).",
        ),
        &["stream"],
    )
    .unwrap();

    /// Persistent, cross-sweep known-ACTIONABLE-DIVERGENT count per reconcile
    /// stream — the divergent partition of [`PROJECTION_RECONCILE_KNOWN_GAPS`]'s
    /// ledger (`MissLedger::divergent_tracked`), but ONLY the share heal is
    /// still permitted to move.
    ///
    /// EXCLUDES the refused-declared partition (the content arm's
    /// `declared_ids`, checked at the `admit()` call site) — a refused-declared
    /// row already carries a DIFFERENT declared head, so heal is FORBIDDEN to
    /// move it until a canonical channel fires; it is PERMANENT, not a drain
    /// target. Admitting it here would make this gauge read ≈ the whole
    /// refused population (a live peer's dominant class — matthew: ~6071) and
    /// never fall, the same total-shaped trap
    /// [`PROJECTION_RECONCILE_DIVERGENT_REFUSED`]/`divergent_actionable` exist
    /// to avoid on the per-sweep gauge. Mirror that split here, on the
    /// persistent one.
    ///
    /// Same honesty bound as [`PROJECTION_RECONCILE_KNOWN_GAPS`]: complete
    /// only after one full window rotation, and removal lags by one rotation.
    pub static ref PROJECTION_RECONCILE_KNOWN_DIVERGENT: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_known_divergent",
            "Persistent cross-sweep known ACTIONABLE-divergent count by reconcile stream \
             (MissLedger pod state, excludes refused-declared; complete only after one full \
             window rotation, ~3 sweeps).",
        ),
        &["stream"],
    )
    .unwrap();

    /// THIS sweep's requested inventory-window offset, by reconcile stream —
    /// the numerator every other reconcile gauge is missing a denominator for.
    /// Beside [`PROJECTION_RECONCILE_WINDOW_TOTAL`], a sample reads as "1052
    /// actionable at offset 2000 of 4440" instead of a bare count with no
    /// sense of where in the rotating window it was taken.
    pub static ref PROJECTION_RECONCILE_WINDOW_OFFSET: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_window_offset",
            "This sweep's requested rotating-window offset by reconcile stream.",
        ),
        &["stream"],
    )
    .unwrap();

    /// THIS sweep's largest peer-reported corpus size, by reconcile stream,
    /// clamped to `MAX_INVENTORY_WINDOW_TOTAL` (the same clamp
    /// `InventoryWindow::advance` applies before its wrap test) — a single
    /// inflated peer `total` can never make this gauge lie about the window's
    /// real span.
    pub static ref PROJECTION_RECONCILE_WINDOW_TOTAL: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_window_total",
            "This sweep's clamped peer-reported corpus size by reconcile stream.",
        ),
        &["stream"],
    )
    .unwrap();

    /// RC-4: content-arm heal patches whose `reach` field was DROPPED because
    /// the incoming conductor answer would have NARROWED a distribution-safe
    /// local row into a scoped tier. labels: `from` = the local row's
    /// (distribution-safe) reach, `to` = the incoming (scoped) reach that was
    /// refused. See `p2p::projection_reconcile::reach_patch_would_narrow` —
    /// heal converges the HEAD, it must never relitigate REACH; a narrowing
    /// write from heal would permanently evict the row from
    /// `list_content_anchor_inventory` (which filters on distribution-safe
    /// reach) while `content_ids_present` still sees it locally, producing
    /// permanent AnchorGap churn. A nonzero rate here is diagnostic, not
    /// alarming by itself — it means the guard is doing its job.
    pub static ref PROJECTION_RECONCILE_REACH_NARROWED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_projection_reconcile_reach_narrowed_total",
            "Content-arm heal patches whose reach field was dropped to avoid narrowing a \
             distribution-safe local row, by from/to reach tier.",
        ),
        &["from", "to"],
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

    /// 1 when this peer holds what its peers advertised, 0 otherwise. The exact
    /// predicate lives in [`converged_gauge_value`]; in words, ALL of:
    ///
    /// - the sweep was **measured** (peers were asked, and no arm short-circuited
    ///   on a DB/query error) — "could not measure" is never published as
    ///   converged;
    /// - no arm left work **pending**;
    /// - no attempted gap ended the sweep **failed** (the conductor answered
    ///   `None`/`Err` for a gap this sweep actually tried);
    /// - **unadjudicated** divergence is zero (`divergent - divergent_refused`).
    ///
    /// The HELP string previously read "pending+exhausted+divergent all zero",
    /// which misdescribed it twice: divergence is gated on the ACTIONABLE
    /// remainder (see [`PROJECTION_RECONCILE_DIVERGENT_REFUSED`]), and the
    /// `exhausted` term is vacuous on these arms (the reconcile trackers are
    /// rebuilt per sweep — see `reconcile_rails::GapCounts::exhausted`). An
    /// operator reading the old string would look for a backlog that cannot be
    /// the cause.
    ///
    /// When this reads 0, `elohim_projection_reconcile_converged_blocked_by`
    /// names WHICH term is holding it there — do not deduce it.
    ///
    /// The field an SLO may ride. Deliberately NOT the same thing as the
    /// `caughtUp` published on `/p2p/status`, which goes true when a sweep ENDS
    /// — including a sweep that healed nothing because every gap spent its retry
    /// budget (the live 22-sweep / `healedTotal: 0` shape). Cumulative heals are
    /// already derivable from `elohim_projection_heal_outcomes_total`, so no
    /// healed-total counter is added here.
    pub static ref PROJECTION_RECONCILE_CONVERGED: IntGauge = IntGauge::new(
        "elohim_projection_reconcile_converged",
        "1 when the peer holds what its peers advertised: sweep measured, nothing pending, \
         nothing failed, and no unadjudicated divergence.",
    )
    .unwrap();

    /// Which term is holding `elohim_projection_reconcile_converged` at 0.
    /// label: term = "pending" | "failed" | "divergent_actionable" | "unmeasured".
    ///
    /// Every term is published EVERY sweep (0 or 1), so a term reading 0 is
    /// evidence, not silence. Several can be 1 at once — they are independent
    /// blockers, not a priority list.
    ///
    /// Why it exists: `converged` is one bit, so a fleet stuck at 0 forced a
    /// code-read to guess which half of the predicate was responsible — and the
    /// guess was WRONG (the `exhausted`/`MissLedger` bucket was blamed for
    /// months while the real terms were `pending` and a small unadjudicated
    /// divergence residue; live alpha A published `exhausted: 0` beside
    /// `pending: 2894`). The fleet should TELL us which term pins the gauge.
    ///
    /// The invariant, asserted in tests: `converged == 1` iff every term is 0.
    pub static ref PROJECTION_RECONCILE_CONVERGED_BLOCKED_BY: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_projection_reconcile_converged_blocked_by",
            "1 per term currently preventing convergence (pending | failed | \
             divergent_actionable | unmeasured); all zero iff converged is 1.",
        ),
        &["term"],
    )
    .unwrap();

    /// The atomic actionable-divergence COUNT behind the binary
    /// `blocked_by{term="divergent_actionable"}` flag — set in
    /// [`record_reconcile_sweep`] from the SAME fold, the SAME moment, so an
    /// external reader (the CI fleet-quiesce gate's tolerance predicate) can
    /// bound "how much in-flight divergence" without subtracting the
    /// per-stream `divergent`/`divergent_refused` gauges, which are published
    /// at DIFFERENT moments mid-sweep (discovery vs heal-leg re-publish) and
    /// therefore over-read under cross-gauge subtraction — live 2026-08-08:
    /// skewed sum 7-11 while the fold's own count read 1-2. Invariant, pinned
    /// in tests: this gauge > 0 iff the blocked_by term flag is 1.
    pub static ref PROJECTION_RECONCILE_DIVERGENT_ACTIONABLE: IntGauge = IntGauge::new(
        "elohim_projection_reconcile_divergent_actionable",
        "Atomic unadjudicated-divergence count from the converged fold (rea+content), \
         published with blocked_by from the same sweep numbers.",
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

    // -----------------------------------------------------------------------
    // iroh plane (Track-2 substrate, dual/iroh modes). These are the counters
    // that answer "did iroh carry anything" — every one of them read ZERO on
    // every dual-mode node before 2026-08-23 because the plane had no peer
    // discovery (see `p2p_iroh::peer_book`). A proof of the iroh leg is these
    // moving, not the endpoint logging "started".
    // -----------------------------------------------------------------------

    /// Peers this node can currently dial over iroh (size of the
    /// `IrohPeerBook`, self excluded).
    pub static ref IROH_PEERS_KNOWN: IntGauge = IntGauge::new(
        "elohim_iroh_peers_known",
        "iroh peers with a dialable NodeAddr in the peer book (self excluded).",
    )
    .unwrap();

    /// Transport-manifest announcements received. label: result = "accepted"
    /// (signature verified, book updated) | "stale" (older than held) |
    /// "bad_signature" | "decode_failed" | "self".
    pub static ref IROH_MANIFEST_ANNOUNCEMENTS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_iroh_manifest_announcements_total",
            "elohim/transport/manifest announcements received, by result.",
        ),
        &["result"],
    )
    .unwrap();

    /// iroh-gossip membership transitions on inbound topics. label:
    /// direction = "up" | "down". `up` staying at zero = the plane has no
    /// neighbors and every iroh publish is a shout into a void.
    pub static ref IROH_GOSSIP_NEIGHBOR_EVENTS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_iroh_gossip_neighbor_events_total",
            "iroh-gossip NeighborUp/NeighborDown events on inbound topics.",
        ),
        &["direction"],
    )
    .unwrap();

    /// Gossip messages RECEIVED over the iroh plane, by topic. The libp2p
    /// twin is implicit in the per-topic handlers; this one exists so a
    /// dual-mode node can show which leg actually delivered.
    pub static ref IROH_GOSSIP_RECEIVED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_iroh_gossip_received_total",
            "Gossip messages received over iroh, by topic.",
        ),
        &["topic"],
    )
    .unwrap();

    /// iroh sync rounds initiated by the iroh round driver (twin of
    /// `elohim_sync_rounds_total`, which is libp2p-only).
    pub static ref IROH_SYNC_ROUNDS: IntCounter = IntCounter::new(
        "elohim_iroh_sync_rounds_total",
        "Sync rounds initiated over the iroh plane.",
    )
    .unwrap();

    /// Outbound iroh sync requests. label: kind = "list_documents" |
    /// "sync_changes" | "announce_change" | "announce_pull"; result = "ok" |
    /// "dial_failed" | "request_failed" | "error_response".
    ///
    /// The two announce kinds are the doorbell's legs (`p2p_iroh::announce_change`
    /// sends, `p2p_iroh::sync_backend` pulls back when a doorbell carried no
    /// usable bytes) — they are OUTBOUND on both nodes, so a converging pair
    /// shows `announce_change` on the author and `announce_pull` on the peer
    /// that had to fetch the dependencies.
    pub static ref IROH_SYNC_REQUESTS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_iroh_sync_requests_total",
            "Outbound sync requests over iroh by verb and result.",
        ),
        &["kind", "result"],
    )
    .unwrap();

    /// Automerge changes APPLIED from an iroh sync answer — the byte-level
    /// proof that content moved over iroh.
    pub static ref IROH_SYNC_CHANGES_APPLIED: IntCounter = IntCounter::new(
        "elohim_iroh_sync_changes_applied_total",
        "Automerge changes applied from iroh sync responses.",
    )
    .unwrap();

    /// Blob fetches attempted over iroh-blobs. label: result = "ok" |
    /// "dial_failed" | "not_found" | "verify_failed" | "error".
    pub static ref IROH_BLOB_FETCHES: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_iroh_blob_fetches_total",
            "Blob fetches over iroh-blobs by result.",
        ),
        &["result"],
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

    /// Which plane each acquisition `GetContent` was dispatched on. Read beside
    /// [`ACQUISITION_OUTCOMES`] and `elohim_iroh_blob_fetches_total`: a dual mesh
    /// where `{transport="iroh"}` stays flat has iroh wired but idle on the pull
    /// leg (the 2026-08-28 fleet shape); labels are the two planes, never a peer.
    pub static ref ACQUISITION_DISPATCH: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_acquisition_dispatch_total",
            "Acquisition (pull-leg) GetContent dispatches by transport plane.",
        ),
        &["transport"],
    )
    .unwrap();

    /// Acquisition reconcile tick outcomes. This is deliberately separate from
    /// [`ACQUISITION_OUTCOMES`]: fetch outcomes answer whether requested bytes
    /// arrived, while this counter answers whether the local desired-set pass
    /// ran at all. Every scheduled tick lands in exactly one typed outcome.
    pub static ref ACQUISITION_RECONCILE_OUTCOMES: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_acquisition_reconcile_outcomes_total",
            "Acquisition reconcile ticks by completed or early-return outcome.",
        ),
        &["outcome"],
    )
    .unwrap();

    /// Whether this process has completed at least one acquisition reconcile.
    /// Read beside ACTIVE_PINS: `initialized=1, active_pins=0` is an observed
    /// empty desired set; `initialized=0` means no honest pull sample exists.
    pub static ref ACQUISITION_RECONCILE_INITIALIZED: IntGauge = IntGauge::new(
        "elohim_acquisition_reconcile_initialized",
        "1 after the first completed acquisition reconcile in this process; otherwise 0.",
    )
    .unwrap();

    /// Active acquisition pins observed by the latest completed reconcile.
    /// Meaningful only when [`ACQUISITION_RECONCILE_INITIALIZED`] is 1.
    pub static ref ACQUISITION_ACTIVE_PINS: IntGauge = IntGauge::new(
        "elohim_acquisition_active_pins",
        "Active acquisition pins observed by the latest completed reconcile.",
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

    /// Outbound sync request outcomes. Deliberately NEVER labelled by raw peer
    /// id: that is an unbounded-cardinality bomb on a real mesh. Peer identity
    /// stays in the log line at the same site. label: result = "ok" |
    /// "timeout" | "connection_closed" | "dial_failure" | "unsupported_protocols" |
    /// "io" — the same closed vocabulary as `elohim_view_federation_outbound_total`,
    /// so the two planes read the same way.
    ///
    /// Second label: peer_class — a BOUNDED substitute for peer identity, so
    /// trust can be priced without paying the cardinality cost ("you cannot
    /// price a relationship you cannot observe"). Derived from
    /// `PeerTrustCache::peer_class_for`: the cached `VerifiedTrustContext`'s
    /// `reach_ceiling`, clamped to the closed
    /// [`crate::generated_enums::CORE_REACH_LEVELS`] vocabulary (an
    /// unrecognised string lands on `"other"`, never mints its own label —
    /// the same clamp `inc_projection_reconcile_reach_narrowed` applies to
    /// wire-derived reach) — or `"unverified"` when no trust context is
    /// cached for the peer. The label set stays bounded by
    /// `|CORE_REACH_LEVELS| + "other" + "unverified"` regardless of mesh size.
    pub static ref SYNC_REQUEST_OUTCOMES: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_sync_request_outcomes_total",
            "Outbound sync request outcomes by result (ok vs failure variant) and peer_class \
             (bounded reach-tier classification, never a raw peer id).",
        ),
        &["result", "peer_class"],
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

// A SEPARATE `lazy_static!` block, purely to stay under the macro's recursion
// limit — the single block above was already at the edge of it, and one more
// entry tips `__lazy_static_internal!` over. No other reason for the split;
// these statics register into the SAME [`REGISTRY`] as everything above.
lazy_static! {
    /// Durable Prometheus mirror of `signals.rs`'s per-family
    /// `AtomicU64`/`Ordering::Relaxed` decode-miss counters (a REAL
    /// mirror-variant tag whose typed decode still failed). label: family =
    /// "infra" | "mishpat" | "elohim_content". Additive only — the atomics stay
    /// as the in-process status-surface source of truth; this is the durable,
    /// graphable twin, same idiom as `IDENTITY_NAMESPACE_VIOLATIONS`.
    pub static ref SIGNAL_DECODE_MISS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_signal_decode_miss_total",
            "Real mirror-variant signal decode misses, by signal family.",
        ),
        &["family"],
    )
    .unwrap();

    /// Requester-side [`p2p::head_record_client::PeerHeadRecordFetcher::fetch`]
    /// outcomes, by [`seam_contracts::AnswerState`] — `present` | `absent` |
    /// `unreachable`. Counted once inside `fetch` itself (every one of its exit
    /// points), so every caller of the head-record client is covered without
    /// needing its own inc site.
    pub static ref CONTENT_HEAD_RECORD_FETCH_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_head_record_fetch_total",
            "Requester-side head-record fetch outcomes, by AnswerState.",
        ),
        &["state"],
    )
    .unwrap();

    /// Requester-side EVIDENCE resolutions in the adopt/contest arm, by
    /// [`AdoptEvidence`] — `carried` | `no_record` | `budget_elapsed` |
    /// `conductor_error` | `unknown`. One increment per fetch the arm performs,
    /// counted at the single `resolve_peer_evidence` site so the sum is exactly
    /// "how many times did we go looking for bytes".
    ///
    /// This is the meter the 2026-08-03 diagnosis had to reconstruct by hand from
    /// a `carried=` INFO line: `contest_failed{no_local_chain}` counted the
    /// refusals, but nothing said WHY the bytes were missing, so the phantom
    /// class (permanently no bytes anywhere) and the C11-saturation class (bytes
    /// exist, the advertiser's conductor is wedged) were indistinguishable — and
    /// they point at OPPOSITE remedies (data hygiene vs conductor capacity).
    /// `no_record`-dominated says supply is structurally absent;
    /// `budget_elapsed`-dominated says the supply is there and starved.
    pub static ref CONTENT_ADOPT_EVIDENCE: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_adopt_evidence_total",
            "Adopt-arm head-record evidence resolutions, by evidence state.",
        ),
        &["state"],
    )
    .unwrap();

    /// The contest-backoff ledger hit [`crate::services::contest_backoff::CONTEST_BACKOFF_CAP`]
    /// and fail-open CLEARED (every id returns to contested-every-sweep until it
    /// refills). Unlabeled: this is a single bounded-memory guard, not a
    /// per-reason vocabulary.
    pub static ref CONTENT_CONTEST_BACKOFF_CLEARED: IntCounter = IntCounter::new(
        "elohim_content_contest_backoff_cleared_total",
        "Contest-backoff ledger cap-overflow fail-open clears.",
    )
    .unwrap();

    /// ADVERTISER DIVERSITY (2026-08-03). One ROUTE-AROUND decision on the
    /// adopt-evidence path, by [`AdoptEvidenceFallback`] — `attempted` |
    /// `carried` | `degraded` | `no_alternative`.
    ///
    /// Strictly SUPPLEMENTARY to [`CONTENT_ADOPT_EVIDENCE`], which keeps
    /// counting exactly one FINAL resolution per evidence ask. This meter says
    /// how often the primary advertiser was too weak to serve and whether
    /// routing around it worked: `attempted = carried + degraded`, and
    /// `no_alternative` is the population that WANTED a second courier and had
    /// none (the signal that hint plurality, not peer health, is the wall).
    ///
    /// Why it exists: the hinted advertiser is chosen by first-advertiser-wins,
    /// which on the live fleet was usually a 100% CFS-throttled conductor —
    /// `adopt_evidence{budget_elapsed}` dominated while idle peers holding the
    /// same rows were never asked. The protocol adapts; the meter says whether
    /// the adaptation is finding anyone.
    pub static ref CONTENT_ADOPT_EVIDENCE_FALLBACK: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_adopt_evidence_fallback_total",
            "Adopt-arm advertiser-diversity fallback decisions, by outcome.",
        ),
        &["outcome"],
    )
    .unwrap();

    /// Contest-backoff ledger PERSISTENCE outcomes, by [`BackoffSnapshot`] —
    /// `saved` | `save_failed` | `loaded` | `load_failed`.
    ///
    /// The ledger is process-local operational state (never a truth store), but
    /// losing it on every deploy re-churns the whole corpus from zero — hours of
    /// re-learning which ids are evidence-absent. A snapshot sidecar closes that
    /// without a migration or a table; this meter says whether the sidecar is
    /// actually round-tripping rather than silently failing to write.
    pub static ref CONTENT_CONTEST_BACKOFF_SNAPSHOT: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_content_contest_backoff_snapshot_total",
            "Contest-backoff ledger snapshot persistence outcomes.",
        ),
        &["outcome"],
    )
    .unwrap();
}

// Keep incremental metric families outside the original registry block. That
// block is already close to lazy_static's default macro-recursion ceiling; a
// separate expansion avoids raising the recursion limit for the entire crate.
lazy_static! {
    /// Most recently observed conductor read-pool saturation ratio. This is an
    /// event sample projected from the conductor log, not a continuously sampled
    /// utilization value. labels: kind = dht|authored|cache|conductor|wasm|
    /// peer_meta_store|unknown.
    pub static ref NODE_DB_READ_POOL_LAST_SATURATION_RATIO: GaugeVec = GaugeVec::new(
        Opts::new(
            "elohim_node_db_read_pool_last_saturation_ratio",
            "Most recently observed conductor read-pool saturation ratio (1.0 = 100%; 0 = no saturation event observed since process start).",
        ),
        &["kind"],
    )
    .unwrap();

    /// Count of conductor read-pool saturation events. Paired with the last
    /// ratio gauge so alerts can require both recent activity and magnitude.
    pub static ref NODE_DB_READ_POOL_SATURATION_EVENTS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_node_db_read_pool_saturation_events_total",
            "Conductor read-pool saturation events observed in the child log.",
        ),
        &["kind"],
    )
    .unwrap();

    // ── C8: head-plane BATCH externs (L1, head-plane trust-gradient program) ──
    //
    // The five series below make the round-trip collapse CHECKABLE rather than
    // asserted. Read them together:
    //
    //   ids_total / calls_total  = the actual batch yield (the whole claim).
    //   unattempted_total        = ids the coordinator never started. Rising with
    //                              a `budget_exhausted` stop reason means the
    //                              in-wasm deadline is binding; rising with
    //                              `shared_failure` means the conductor is
    //                              backpressured and the AIMD controller should
    //                              already be shrinking `batch_size`.
    //   queue_wait_ms            = observed RTT MINUS the extern's in-wasm time.
    //                              THE probe that decides whether the conductor
    //                              -fork call-deadline patch stays warranted: if
    //                              this stays small while quiesce falls, the
    //                              scheduling floor is not the binding cost.
    //                              DIAGNOSTIC ONLY — it no longer feeds the AIMD
    //                              controller, because it is not monotonic in
    //                              batch size: it subtracts exactly the interval
    //                              a stalled extern spends waiting on a read
    //                              permit, so a saturated conductor reads here as
    //                              headroom. The control input is
    //                              elohim_conductor_admission_wait_ms.
    //   batch_size               = the AIMD controller's current ask. adam (WAN)
    //                              converging smaller than matthew (LAN) under
    //                              the SAME constants is the heterogeneity
    //                              preservation the design promised.

    /// One batch conductor round-trip. label: extern_name, outcome.
    pub static ref HEAD_BATCH_CALLS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_head_batch_calls_total",
            "Batched head-plane conductor round-trips, by extern and outcome.",
        ),
        &["extern_name", "outcome"],
    )
    .unwrap();

    /// Ids carried by those round-trips. Divided by `calls_total` this IS the
    /// yield the whole L1 lever exists to raise. label: extern_name.
    pub static ref HEAD_BATCH_IDS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_head_batch_ids_total",
            "Ids carried by batched head-plane calls, by extern.",
        ),
        &["extern_name"],
    )
    .unwrap();

    /// Ids the coordinator NEVER STARTED. Not failures — every one of these went
    /// back to `pending`. label: extern_name, stop_reason.
    pub static ref HEAD_BATCH_UNATTEMPTED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_head_batch_unattempted_total",
            "Ids a batch call never started (ceiling, deadline, or a shared failure's stop \
             point). NOT failures — these are returned to pending.",
        ),
        &["extern_name", "stop_reason"],
    )
    .unwrap();

    /// Observed round-trip MINUS the extern's own in-wasm `elapsed_ms`: the time
    /// a batch call spent queued for a conductor DB read permit rather than doing
    /// work. Buckets span the sub-millisecond (healthy LAN) to the 10s
    /// `ACQUIRE_TIMEOUT_MS` ceiling (a fully saturated read pool).
    pub static ref HEAD_BATCH_QUEUE_WAIT_MS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "elohim_head_batch_queue_wait_ms",
            "Conductor queue-wait per batch call in milliseconds (observed RTT minus in-wasm \
             elapsed_ms).",
        )
        .buckets(vec![
            1.0, 5.0, 25.0, 100.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 15_000.0,
        ]),
        &["extern_name"],
    )
    .unwrap();

    /// The AIMD controller's CURRENT batch size. label: extern_name.
    pub static ref HEAD_BATCH_SIZE: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "elohim_head_batch_size",
            "Current adaptive batch size the head-plane sweep asks for (AIMD).",
        ),
        &["extern_name"],
    )
    .unwrap();

    /// Single-id fallbacks taken because a conductor cannot serve the batch
    /// externs. label: why = "unknown_function" | "schema_version_mismatch" |
    /// "undecodable_output". Expected NON-ZERO before the coordinator hot-swap
    /// lands, and expected to STOP once it has — a series that keeps ticking
    /// after the swap means some pod never got it.
    pub static ref HEAD_BATCH_FALLBACK: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_head_batch_fallback_total",
            "Single-id fallbacks taken because the conductor cannot serve the batch externs.",
        ),
        &["why"],
    )
    .unwrap();

    // ── Q11: ATOMIC CONVERGENCE-PATH TIMING BUDGET (operator directive 2026-08-16,
    // minutes-quiesce plan §5/§8) ───────────────────────────────────────────────
    //
    // ONE histogram family for the ~8 atoms the quiesce prediction
    // `quiesce ≈ Σ count×cost÷parallelism` is built from: `put_record`,
    // `inventory_page`, `head_batch_per_id`, `head_record_verify`,
    // `adopt_declare`, `shard_fetch`, `manifest_persist`, `digest_fold`. Buckets
    // span 0.1ms (a pure fold) to 30s (a saturated conductor round-trip) on a
    // roughly log scale, so both ends of the atom roster land inside a bucket
    // rather than pinning the tail.
    //
    // `head_batch_per_id` is DERIVED, not double-instrumented: the batched head
    // externs already carry `elapsed_ms` (`HeadBatchResolver`,
    // `elohim_head_batch_queue_wait_ms`) — this series records that same
    // in-wasm time divided by the ids it covered, which is what the budget
    // table's per-atom "cost" column needs (a per-call number, not a per-batch
    // one) without a second `Instant` wrapping the same round-trip.
    pub static ref ATOM_DURATION_MS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "elohim_atom_duration_ms",
            "Wall-clock cost of one convergence-path atom (Q11 atomic timing budget), by atom.",
        )
        .buckets(vec![
            0.1, 0.5, 1.0, 5.0, 25.0, 100.0, 500.0, 1_000.0, 5_000.0, 15_000.0, 30_000.0,
        ]),
        &["atom"],
    )
    .unwrap();

    // ── CONDUCTOR ADMISSION — the capacity contract's own instruments ─────────
    //
    // Read these together with `elohim_node_db_max_readers`. Before this gate
    // existed we exported CAPACITY and ARRIVAL and never OCCUPANCY or HOLD-TIME,
    // so Little's Law (L = λW) had no L: nothing we emitted could say "the pool
    // is full", and every diagnosis had to infer it. These four close that.
    //
    //   in_flight            = L. The number. Pinned at `capacity` with demand
    //                          behind it IS the evidence for raising the
    //                          conductor's db_max_readers; anything less means
    //                          we were oversubscribing, not undersized.
    //   hold_ms              = W. How long one call owns capacity.
    //   wait_ms              = queueing delay AT the gate. Unlike
    //                          head_batch_queue_wait_ms (RTT − in-wasm elapsed)
    //                          this is monotonic in offered load, which is what
    //                          makes it safe to close a control loop over.
    //   shed_total           = calls refused BEFORE dispatch. Never a conductor
    //                          failure — the conductor never saw these.

    /// Permits in the gate. Mirrors the conductor's admissible read-pool share.
    pub static ref CONDUCTOR_ADMISSION_CAPACITY: IntGauge = IntGauge::new(
        "elohim_conductor_admission_capacity",
        "Conductor admission permits (db_max_readers minus the conductor's own reserve).",
    )
    .unwrap();

    /// **L** — permits currently held. THE occupancy signal.
    pub static ref CONDUCTOR_ADMISSION_IN_FLIGHT: IntGauge = IntGauge::new(
        "elohim_conductor_admission_in_flight",
        "Conductor calls holding an admission permit right now (pool occupancy).",
    )
    .unwrap();

    /// Time spent queued at the gate, by caller class.
    pub static ref CONDUCTOR_ADMISSION_WAIT_MS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "elohim_conductor_admission_wait_ms",
            "Milliseconds a conductor call waited for an admission permit, by class.",
        )
        .buckets(vec![
            1.0, 5.0, 25.0, 100.0, 250.0, 500.0, 1_000.0, 2_000.0, 5_000.0,
        ]),
        &["class"],
    )
    .unwrap();

    /// **W** — how long a call held capacity, by zome.
    pub static ref CONDUCTOR_ADMISSION_HOLD_MS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "elohim_conductor_admission_hold_ms",
            "Milliseconds a conductor call held its admission permit (service time), by zome.",
        )
        .buckets(vec![
            5.0, 25.0, 100.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 15_000.0, 30_000.0,
        ]),
        &["zome"],
    )
    .unwrap();

    /// Permits granted, and how many arrived to a already-full gate. A rising
    /// `saturated` share is the pool telling us it is the binding constraint.
    pub static ref CONDUCTOR_ADMISSION_ACQUIRED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_conductor_admission_acquired_total",
            "Admission permits granted, by class and whether the gate was full on arrival.",
        ),
        &["class", "arrival"],
    )
    .unwrap();

    /// Calls refused at the gate. NOT conductor failures — nothing was
    /// dispatched, so these cost the conductor nothing and establish nothing
    /// about the work.
    pub static ref CONDUCTOR_ADMISSION_SHED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_conductor_admission_shed_total",
            "Conductor calls shed at the admission gate before dispatch, by class and zome.",
        ),
        &["class", "zome"],
    )
    .unwrap();
}

// Q3/Q4 (minutes-quiesce W1.2/W1.3): blob-swarm shard-manifest propagation +
// swarm-spread reassembly. A separate block for the same recursion-limit
// reason as the block above.
lazy_static! {
    /// A blob-fetch response answered with a durable `ShardManifest` instead
    /// of bytes (Q3) — the requester had no direct-byte hit but the peer
    /// could still name the shard set. Flat-zero after Q3 lands means no
    /// composite hash ever misses a peer that actually holds a manifest for
    /// it (either every composite is servable directly, or manifests never
    /// propagate — cross-check against `elohim_blob_swarm_composite_completed_total`).
    pub static ref BLOB_SWARM_MANIFESTS_RECEIVED: IntCounter = IntCounter::new(
        "elohim_blob_swarm_manifests_received_total",
        "Blob-fetch responses that returned a shard manifest instead of bytes (Q3).",
    )
    .unwrap();

    /// Per-shard fetch outcomes during a Q4 swarm-spread composite
    /// reassembly. label: outcome = "hit" | "miss" | "no_candidates" |
    /// "parity_skipped".
    pub static ref BLOB_SWARM_SHARDS_FETCHED: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "elohim_blob_swarm_shards_fetched_total",
            "Per-shard fetch outcomes during a swarm-spread composite reassembly, by outcome.",
        ),
        &["outcome"],
    )
    .unwrap();

    /// Composite blobs fully reassembled from swarm-fetched shards (Q4) — the
    /// denominator the exponential-curve measure (W4.3, time-to-N-replicas)
    /// reads alongside the per-completion `distinct_source_peers` log field.
    pub static ref BLOB_SWARM_COMPOSITE_COMPLETED: IntCounter = IntCounter::new(
        "elohim_blob_swarm_composite_completed_total",
        "Composite blobs fully reassembled from swarm-fetched shards.",
    )
    .unwrap();
}

// ── HTTP dispatch wrapper — the request-duration histogram this file was
// missing entirely until 2026-08-21: the doorway's 2-failure breaker opened on
// a ~60s storage stall under suite load while storage answered in 1.6ms a
// minute later, and there was no series here to say which request stalled,
// for how long, or whether it ever produced a status code at all. See
// genesis/data/timeline/backlog/doorway-breaker-trial-theft-fleet-verification.md
// "Mesh evidence 2026-08-21 19:38". A separate `lazy_static!` block for the
// same macro-recursion reason noted above the Q3/Q4 block.
lazy_static! {
    /// Wall-clock cost of one HTTP request through `http.rs::handle_request`,
    /// by `route_class` (a small closed set derived from the path prefix —
    /// see [`classify_http_route`], NEVER the raw path or an id, which would
    /// make this series' cardinality unbounded) and `status_class`
    /// (`"2xx"|"4xx"|"5xx"`). Observed on every exit from the dispatch
    /// wrapper — the normal success/error tail, an early return (admission
    /// shed, an SSE/stream upgrade), or a request that is dropped before it
    /// ever reports a status (client disconnect, cancellation, a panic) via
    /// the [`RequestMetricsGuard`] `Drop` fallback — so a request that burns
    /// the full client timeout is IN this distribution, not silently omitted.
    /// That silent omission (only counting requests that finish fast) is the
    /// coordinated-omission trap this instrument exists to close: it is
    /// exactly what hid the stall behind a healthy-looking `/metrics` reading
    /// a minute later.
    ///
    /// Buckets span 1ms (a cheap `/health`) to 12s (past the doorway's own
    /// forwarding timeout), roughly log-spaced, so both ends of the route
    /// roster land inside a bucket instead of pinning the tail.
    pub static ref HTTP_REQUEST_DURATION_MS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "elohim_http_request_duration_ms",
            "Wall-clock cost of one HTTP request through the dispatch wrapper, by route_class and status_class.",
        )
        .buckets(vec![
            1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1_000.0, 2_000.0, 5_000.0,
            12_000.0,
        ]),
        &["route_class", "status_class"],
    )
    .unwrap();

    /// Requests currently inside `handle_request` — the occupancy signal a
    /// stall shows up on directly (rising and staying up) independent of
    /// whether any individual request has finished long enough yet to land in
    /// the duration histogram.
    pub static ref HTTP_REQUESTS_IN_FLIGHT: IntGauge = IntGauge::new(
        "elohim_http_requests_in_flight",
        "HTTP requests currently being handled by the dispatch wrapper.",
    )
    .unwrap();

    /// Documents the bounded sync-fetch window re-queued after the transport
    /// REFUSED to carry their `SyncChanges` request (libp2p
    /// `OutboundFailure::Io` — the measured `max sub-streams reached` shape).
    ///
    /// One counter, both planes (`p2p::sync_round::FetchWindow` is the shared
    /// scheduler). Read it against
    /// `elohim_sync_request_outcomes_total{result="io"}`: a non-zero `io` with a
    /// zero re-queue count means the window is not settling failures at all,
    /// and a re-queue count that keeps pace with `io` means the window is still
    /// too wide for what the multiplexer will grant. On a healthy mesh both
    /// trend to zero — before this window existed, the recovering peer logged
    /// 196 -> 55 -> 0 io failures per round with nothing retrying any of them.
    pub static ref SYNC_FETCH_WINDOW_REQUEUED: IntCounter = IntCounter::new(
        "elohim_sync_fetch_window_requeued_total",
        "SyncChanges requests re-queued by the bounded fetch window after a transport io refusal.",
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
        let _ = REGISTRY.register(Box::new(NODE_DB_READ_POOL_LAST_SATURATION_RATIO.clone()));
        let _ = REGISTRY.register(Box::new(NODE_DB_READ_POOL_SATURATION_EVENTS.clone()));
        for kind in [
            "dht",
            "authored",
            "cache",
            "conductor",
            "wasm",
            "peer_meta_store",
            "unknown",
        ] {
            NODE_DB_READ_POOL_LAST_SATURATION_RATIO
                .with_label_values(&[kind])
                .set(0.0);
            NODE_DB_READ_POOL_SATURATION_EVENTS.with_label_values(&[kind]);
        }
        let _ = REGISTRY.register(Box::new(NODE_CPU_QUOTA_MILLICORES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CONDUCTOR_SMAPS_ANON_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CONDUCTOR_ANON_MAPPING_COUNT.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CONDUCTOR_LARGEST_ANON_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CONDUCTOR_ANON_BUCKET_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CONDUCTOR_ANON_BUCKET_COUNT.clone()));
        let _ = REGISTRY.register(Box::new(NODE_CORPUS_DOCS.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_NAMESPACE_VIOLATIONS.clone()));
        let _ = REGISTRY.register(Box::new(ATTRIBUTION_UNVERIFIED_BINDINGS.clone()));
        let _ = REGISTRY.register(Box::new(ATTRIBUTION_JOINS.clone()));
        let _ = REGISTRY.register(Box::new(ATTRIBUTION_BINDINGS_EXAMINED.clone()));
        // Pre-touch the posture this deployment actually runs, so "no
        // self-asserted binding reached an attribution join" reads as a measured
        // 0 rather than an absent series. Only the live posture is pre-touched:
        // a 0 on the *other* label would read as "enforce is on and refusing
        // nothing", which is the exact false-green this counter exists to deny.
        ATTRIBUTION_UNVERIFIED_BINDINGS
            .with_label_values(&[
                match crate::db::peer_identity_bindings::attribution_posture() {
                    crate::db::peer_identity_bindings::AttributionPosture::Enforce => "enforce",
                    crate::db::peer_identity_bindings::AttributionPosture::Observe => "observe",
                },
            ])
            .inc_by(0);
        let _ = REGISTRY.register(Box::new(SIGNAL_DECODE_MISS_TOTAL.clone()));
        // Pre-touch the three signal families so zero-misses reads as a
        // measured 0, never as an absent series (the exact ambiguity the
        // obey-probe pre-touch exists to kill; this counter shipped without
        // it and was flagged the same night).
        for family in ["infra", "mishpat", "elohim_content"] {
            SIGNAL_DECODE_MISS_TOTAL
                .with_label_values(&[family])
                .inc_by(0);
        }
        let _ = REGISTRY.register(Box::new(ELOHIM_PLACEMENT_GAP_COUNT.clone()));
        let _ = REGISTRY.register(Box::new(ELOHIM_RS_COVERAGE_MILLI.clone()));
        let _ = REGISTRY.register(Box::new(ELOHIM_CUSTODIAN_FREE_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(ELOHIM_CUSTODIAN_USED_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(ELOHIM_CUSTODIAN_STEWARDED_BYTES.clone()));
        let _ = REGISTRY.register(Box::new(ELOHIM_CUSTODY_CLASS_COUNT.clone()));
        let _ = REGISTRY.register(Box::new(VIEW_FEDERATION_OUTBOUND.clone()));
        let _ = REGISTRY.register(Box::new(VIEW_FEDERATION_INBOUND_SERVED.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_HEAD_RECORD_DEGRADED.clone()));
        // Pre-touch every degrade cause. `no_record` shipped UNCOUNTED until
        // 2026-08-03 and was the structural one; a series that only appears once
        // it fires cannot answer "is this class silent, or is it not measured?"
        // — the exact question the evidence-starvation diagnosis had to answer
        // from log lines instead of meters.
        {
            use seam_contracts::ReasonLabel as _;
            for cause in HeadRecordDegraded::ALL {
                CONTENT_HEAD_RECORD_DEGRADED
                    .with_label_values(&[cause.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(CONTENT_HEAD_RECORD_FETCH_TOTAL.clone()));
        // Pre-touch every AnswerState so /metrics shows the full state set from
        // boot, same zero-with-a-series discipline as the obey-probe below.
        {
            use seam_contracts::ReasonLabel as _;
            for state in seam_contracts::AnswerState::ALL {
                CONTENT_HEAD_RECORD_FETCH_TOTAL
                    .with_label_values(&[state.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(GHOST_DECLARATION_DECAY_AUTHOR.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_GHOST_DECAY_BLOCKED.clone()));
        {
            use seam_contracts::ReasonLabel as _;
            for leg in GhostDecayBlockedLeg::ALL {
                CONTENT_GHOST_DECAY_BLOCKED
                    .with_label_values(&[leg.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(CONTENT_WITNESS_AUTHORED.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_HEAD_ADOPTED.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_CANONICAL_ANSWERS.clone()));
        // Pre-touch every canonical-election tier so `/metrics` can distinguish
        // "this tier never wins" from "this tier was never asked about" from
        // boot — same zero-with-a-series discipline as the obey-probe below.
        for tier in ["earned", "staging", "none"] {
            CONTENT_CANONICAL_ANSWERS
                .with_label_values(&[tier])
                .inc_by(0);
        }
        let _ = REGISTRY.register(Box::new(PROJECTION_REFUSED_STALE_REASONS.clone()));
        for reason in ["stored_null", "not_newer", "tier"] {
            PROJECTION_REFUSED_STALE_REASONS
                .with_label_values(&[reason])
                .inc_by(0);
        }
        let _ = REGISTRY.register(Box::new(CONTENT_CANONICAL_LINKS_MINTED.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_CONTEST_FAILED.clone()));
        {
            use seam_contracts::ReasonLabel as _;
            for class in ContestFailure::ALL {
                CONTENT_CONTEST_FAILED
                    .with_label_values(&[class.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(CONTENT_CONTEST_SKIPPED.clone()));
        {
            use seam_contracts::ReasonLabel as _;
            for reason in ContestSkip::ALL {
                CONTENT_CONTEST_SKIPPED
                    .with_label_values(&[reason.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(CONTENT_ADOPT_EVIDENCE.clone()));
        // Pre-touch every evidence state. This meter's whole job is to let an
        // operator read WHICH supply wall dominates, and a state that only
        // materialises on first occurrence cannot say "this class is measured
        // and zero" — the absent-series trap, a repeat offender on this seam.
        {
            use seam_contracts::ReasonLabel as _;
            for state in AdoptEvidence::ALL {
                CONTENT_ADOPT_EVIDENCE
                    .with_label_values(&[state.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(CONTENT_CONTEST_BACKOFF_CLEARED.clone()));
        let _ = REGISTRY.register(Box::new(CONTENT_ADOPT_EVIDENCE_FALLBACK.clone()));
        // Pre-touch every fallback outcome, same discipline as the evidence
        // states above: `no_alternative` at zero is a MEANINGFUL reading (the
        // hint plurality is there), and a series that only materialises on first
        // occurrence cannot say that.
        {
            use seam_contracts::ReasonLabel as _;
            for outcome in AdoptEvidenceFallback::ALL {
                CONTENT_ADOPT_EVIDENCE_FALLBACK
                    .with_label_values(&[outcome.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(CONTENT_CONTEST_BACKOFF_SNAPSHOT.clone()));
        {
            use seam_contracts::ReasonLabel as _;
            for outcome in BackoffSnapshot::ALL {
                CONTENT_CONTEST_BACKOFF_SNAPSHOT
                    .with_label_values(&[outcome.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(CONTENT_ADOPT_SWEEP.clone()));
        {
            use seam_contracts::ReasonLabel as _;
            for outcome in AdoptSweepOutcome::ALL {
                CONTENT_ADOPT_SWEEP
                    .with_label_values(&[outcome.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(TRUST_PRICED_ADOPTIONS.clone()));
        {
            use seam_contracts::ReasonLabel as _;
            // Pre-touch all three depths so a fleet that has never cheapened
            // anything reads as a MEASURED zero on `accept_with_provenance`,
            // not as a missing series (the difference between "the grant is
            // not biting" and "the seam was never deployed here").
            for depth in TrustPricedDepth::ALL {
                TRUST_PRICED_ADOPTIONS
                    .with_label_values(&[depth.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(CONTENT_ELECTION_OBEYED.clone()));
        CONTENT_ELECTION_OBEYED
            .with_label_values(&["carried"])
            .inc_by(0);
        let _ = REGISTRY.register(Box::new(CONTENT_ELECTION_OBEY_FAILED.clone()));
        for class in ["fetch", "validate", "stamp_refused"] {
            CONTENT_ELECTION_OBEY_FAILED
                .with_label_values(&[class])
                .inc_by(0);
        }
        let _ = REGISTRY.register(Box::new(CONTENT_ELECTION_OBEY_PROBE.clone()));
        // Pre-touch EVERY obey-probe outcome, for the same reason as the
        // reauthor classes below — and with a sharper forcing incident. The obey
        // arm failed on 100% of ~900 probes/hr for two shifts while its exits
        // materialised no series at all, so `/metrics` could not distinguish
        // "this arm is idle" from "this arm never once got past its first gate."
        // Zero-with-a-series is a measured zero; no series is an unasked
        // question. Iterating `ALL` (rather than listing literals) means a new
        // variant cannot be added without also being pre-touched.
        {
            use seam_contracts::ReasonLabel as _;
            for outcome in ElectionObeyProbe::ALL {
                CONTENT_ELECTION_OBEY_PROBE
                    .with_label_values(&[outcome.label()])
                    .inc_by(0);
            }
        }
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
        let _ = REGISTRY.register(Box::new(CUSTODY_ROTATION_AUTHORED.clone()));
        let _ = REGISTRY.register(Box::new(CUSTODY_ROTATION_SKIPPED.clone()));
        // Pre-touch every reason so all seven series exist in `/metrics` from
        // boot — an outcome that has never fired reads as a measured zero, not
        // an absent series.
        {
            use seam_contracts::ReasonLabel as _;
            for reason in CustodyRotationSkip::ALL {
                CUSTODY_ROTATION_SKIPPED
                    .with_label_values(&[reason.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(IDENTITY_KEY_SUPERSEDE.clone()));
        let _ = REGISTRY.register(Box::new(SHARD_PUSH_PEER_UNRESOLVED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_HEAL_OUTCOMES.clone()));
        // Pre-touch every (stream, outcome) combination — 3 streams x 13
        // `p2p::projection_reconcile::HealOutcomeKind` variants = 39 series —
        // so an outcome that has literally never fired for a stream
        // still reads as a measured zero, not an absent series. Labels below
        // are the same vocabulary already used at the `inc_projection_heal_outcome`
        // / `inc_projection_heal_outcome_by` call sites in
        // `p2p/projection_reconcile.rs`. Keep this list in exact sync with
        // `HealOutcomeKind::label`'s match arms — a variant missing here still
        // increments correctly, it just starts absent instead of a measured
        // zero (this list itself was missing `missing_deferred` before F8).
        for stream in ["rea", "content", "collectives"] {
            for outcome in [
                "healed",
                "timeout_retried",
                "timeout_exhausted",
                "missing",
                "missing_deferred",
                "failed",
                "refused_declared",
                "refused_stale",
                "no_row",
                "refreshed",
                "deferred_to_adopt",
                "call_failed",
                "unattempted",
            ] {
                PROJECTION_HEAL_OUTCOMES
                    .with_label_values(&[stream, outcome])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_GAPS.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_LOCAL_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_REACH_SCOPED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_EXHAUSTED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_DIVERGENT.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_DIVERGENT_REFUSED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_MEASURED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_KNOWN_GAPS.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_KNOWN_DIVERGENT.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_WINDOW_OFFSET.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_WINDOW_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_REACH_NARROWED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_SWEEPS.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_CONVERGED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_CONVERGED_BLOCKED_BY.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_RECONCILE_DIVERGENT_ACTIONABLE.clone()));
        let _ = REGISTRY.register(Box::new(SHARD_REDISTRIBUTE_BYTES_MISSING.clone()));
        let _ = REGISTRY.register(Box::new(PEER_STATUS_FANOUT_MISSED.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_FILL_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_FILL_LAST_WRITES.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_FILL_DISCOVERED_CIDS.clone()));
        let _ = REGISTRY.register(Box::new(IDENTITY_FILL_COLLECTIVE_CID_STAMPED.clone()));
        let _ = REGISTRY.register(Box::new(CUSTODY_ANNOUNCE_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(IROH_PEERS_KNOWN.clone()));
        let _ = REGISTRY.register(Box::new(IROH_MANIFEST_ANNOUNCEMENTS.clone()));
        let _ = REGISTRY.register(Box::new(IROH_GOSSIP_NEIGHBOR_EVENTS.clone()));
        let _ = REGISTRY.register(Box::new(IROH_GOSSIP_RECEIVED.clone()));
        let _ = REGISTRY.register(Box::new(IROH_SYNC_ROUNDS.clone()));
        let _ = REGISTRY.register(Box::new(IROH_SYNC_REQUESTS.clone()));
        let _ = REGISTRY.register(Box::new(IROH_SYNC_CHANGES_APPLIED.clone()));
        let _ = REGISTRY.register(Box::new(IROH_BLOB_FETCHES.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_ROUNDS.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_REQUESTS.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_DOCS_ENUMERATED.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_REQUEST_OUTCOMES.clone()));
        // Pre-touch the always-live combination so the series renders before
        // the first real sync round completes — an operator dashboarding
        // "any successful sync request from an unverified peer" must find the
        // series, not infer absence from a metric that has never fired.
        SYNC_REQUEST_OUTCOMES
            .with_label_values(&["ok", "unverified"])
            .inc_by(0);
        let _ = REGISTRY.register(Box::new(SYNC_FETCH_WINDOW_REQUEUED.clone()));
        let _ = REGISTRY.register(Box::new(ACQUISITION_OUTCOMES.clone()));
        let _ = REGISTRY.register(Box::new(ACQUISITION_DISPATCH.clone()));
        let _ = REGISTRY.register(Box::new(ACQUISITION_RECONCILE_OUTCOMES.clone()));
        {
            use seam_contracts::ReasonLabel as _;
            for outcome in AcquisitionReconcileOutcome::ALL {
                ACQUISITION_RECONCILE_OUTCOMES
                    .with_label_values(&[outcome.label()])
                    .inc_by(0);
            }
        }
        let _ = REGISTRY.register(Box::new(ACQUISITION_RECONCILE_INITIALIZED.clone()));
        let _ = REGISTRY.register(Box::new(ACQUISITION_ACTIVE_PINS.clone()));
        let _ = REGISTRY.register(Box::new(ACQUISITION_PINS_RETIRED.clone()));
        let _ = REGISTRY.register(Box::new(ACQUISITION_PIN_RETIREMENTS.clone()));
        let _ = REGISTRY.register(Box::new(SYNC_IN_SYNC_TOTAL.clone()));
        let _ = REGISTRY.register(Box::new(HEAD_BATCH_CALLS.clone()));
        let _ = REGISTRY.register(Box::new(HEAD_BATCH_IDS.clone()));
        let _ = REGISTRY.register(Box::new(HEAD_BATCH_UNATTEMPTED.clone()));
        let _ = REGISTRY.register(Box::new(HEAD_BATCH_QUEUE_WAIT_MS.clone()));
        let _ = REGISTRY.register(Box::new(CONDUCTOR_ADMISSION_CAPACITY.clone()));
        let _ = REGISTRY.register(Box::new(CONDUCTOR_ADMISSION_IN_FLIGHT.clone()));
        let _ = REGISTRY.register(Box::new(CONDUCTOR_ADMISSION_WAIT_MS.clone()));
        let _ = REGISTRY.register(Box::new(CONDUCTOR_ADMISSION_HOLD_MS.clone()));
        let _ = REGISTRY.register(Box::new(CONDUCTOR_ADMISSION_ACQUIRED.clone()));
        let _ = REGISTRY.register(Box::new(CONDUCTOR_ADMISSION_SHED.clone()));
        let _ = REGISTRY.register(Box::new(HEAD_BATCH_SIZE.clone()));
        let _ = REGISTRY.register(Box::new(HEAD_BATCH_FALLBACK.clone()));
        let _ = REGISTRY.register(Box::new(BLOB_SWARM_MANIFESTS_RECEIVED.clone()));
        let _ = REGISTRY.register(Box::new(BLOB_SWARM_SHARDS_FETCHED.clone()));
        let _ = REGISTRY.register(Box::new(BLOB_SWARM_COMPOSITE_COMPLETED.clone()));
        let _ = REGISTRY.register(Box::new(ATOM_DURATION_MS.clone()));
        // Pre-touch all 8 atoms so a run that has never hit one (e.g. no
        // `shard_fetch` on a mesh with no >16MB blob) reads as a MEASURED zero,
        // not an absent series — the same "measured vs never-deployed"
        // distinction `TRUST_PRICED_ADOPTIONS` pre-touches for.
        {
            use seam_contracts::ReasonLabel as _;
            for atom in ConvergenceAtom::ALL {
                ATOM_DURATION_MS
                    .with_label_values(&[atom.label()])
                    .observe(0.0);
            }
        }
        let _ = REGISTRY.register(Box::new(HTTP_REQUEST_DURATION_MS.clone()));
        let _ = REGISTRY.register(Box::new(HTTP_REQUESTS_IN_FLIGHT.clone()));
    });
}

// ── Head-plane batch typed setters (callers never import `prometheus`) ───────

/// Count ONE batch conductor round-trip and the ids it carried, and record the
/// queue-wait it observed.
///
/// **Concerns:** C8 — every call is counted, including the ones that failed, so
/// the yield ratio (`ids_total / calls_total`) has an honest denominator.
///
/// `outcome` is a closed vocabulary supplied by the caller:
/// `"answered"` | `"call_failed"` | `"fallback"`.
pub fn observe_head_batch_call(
    extern_name: &'static str,
    outcome: &str,
    ids: usize,
    queue_wait: std::time::Duration,
) {
    HEAD_BATCH_CALLS
        .with_label_values(&[extern_name, outcome])
        .inc();
    HEAD_BATCH_IDS
        .with_label_values(&[extern_name])
        .inc_by(ids as u64);
    HEAD_BATCH_QUEUE_WAIT_MS
        .with_label_values(&[extern_name])
        .observe(queue_wait.as_secs_f64() * 1_000.0);
}

/// Count ids a batch call never started, tagged by WHY admission stopped.
/// These are returned to `pending`; none of them is a failure.
pub fn add_head_batch_unattempted(extern_name: &'static str, stop_reason: &str, ids: usize) {
    if ids == 0 {
        return;
    }
    HEAD_BATCH_UNATTEMPTED
        .with_label_values(&[extern_name, stop_reason])
        .inc_by(ids as u64);
}

/// Publish the AIMD controller's current ask.
pub fn set_head_batch_size(extern_name: &'static str, size: usize) {
    HEAD_BATCH_SIZE
        .with_label_values(&[extern_name])
        .set(size as i64);
}

/// Count a single-id fallback (the conductor cannot serve the batch externs).
pub fn inc_head_batch_fallback(why: &str) {
    HEAD_BATCH_FALLBACK.with_label_values(&[why]).inc();
}

// ── Q11: convergence-path atoms — the label vocabulary of [`ATOM_DURATION_MS`] ─

/// The convergence-path atoms Q11's budget table sums over. Closed vocabulary,
/// kept separate from any call-site type for the same reason every other label
/// enum in this file is: `metrics` owns the wire string `/metrics` shows, so a
/// rename is a deliberate decision here, never a silent series rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvergenceAtom {
    /// One `kademlia.put_record` on the drain ticker
    /// (`p2p::mod::ElohimP2P::drain_publish_queue`).
    PutRecord,
    /// One `ProjectionInventory` page fetch — per peer, per table
    /// (`projection_reconcile`'s discovery legs).
    InventoryPage,
    /// One id's share of a batched head-plane extern round-trip
    /// (`elapsed_ms / ids`) — DERIVED from the batch resolver's own timing, not
    /// a second `Instant` around the same call.
    HeadBatchPerId,
    /// One `ContentHeadRecord` verify RPC (`head_record_client::PeerHeadRecordFetcher`).
    HeadRecordVerify,
    /// One `try_adopt_canonical_head` per-candidate decision — the atom where
    /// the chain-head collision cost shows under fan-out.
    AdoptDeclare,
    /// One shard fetch race (`blob_swarm::fetch_shards_via_swarm`'s per-shard
    /// `race_fetch`).
    ShardFetch,
    /// One `shard_manifests::upsert_manifest` durable write + read-back.
    ManifestPersist,
    /// One `digest_of_entry_lines` fold — pure, expected sub-millisecond.
    DigestFold,
    /// One inventory page SERVED to a peer (`shard_service::handle_list_content`)
    /// — the local `holochain_sqlite` read cost of answering a `ListContent`.
    ///
    /// Added 2026-08-20 because this was the single highest-rate loop in the
    /// system with NO timer on it. Measured that day on matthew:
    /// `Serving content inventory count=1000 total=4495` firing MULTIPLE TIMES
    /// PER SECOND while `holochain_sqlite::db::access` reported
    /// "Database read connection is saturated. Util 1387.50%" — read starvation
    /// that opened the doorway breaker and surfaced as `503 catching-up`.
    /// Finding that required Loki archaeology precisely because this atom did
    /// not exist. Note the handler runs TWO queries per page — `list_content`
    /// AND `count_content`, both `MinTrust::Invisible` — so the cost is not
    /// one scan but two.
    InventoryServe,
    /// One `ListContent` page ROUND TRIP as seen by the requester — send to
    /// matching `ContentList` (or to transport failure), `p2p::mod`'s
    /// replication cycle.
    ///
    /// The requester half of the pair whose responder half is [`Self::InventoryServe`].
    /// Together they give this hop's composition residual:
    /// `inventory_request - remote inventory_serve = wire + queue`. Neither
    /// alone can separate "the peer is slow to answer" from "the link is slow",
    /// and that ambiguity is exactly what let a local read-pool saturation read
    /// as a network problem on 2026-08-20.
    ///
    /// Timed per PAGE, not per chain — a 5-page walk is five round trips, and
    /// attributing the chain's total to its last page would misreport both.
    /// Failed round trips are recorded at full elapsed (see the transport-failure
    /// arm) so a timeout storm cannot hide by omitting its own samples.
    InventoryRequest,
}

impl seam_contracts::ReasonLabel for ConvergenceAtom {
    const ALL: &'static [Self] = &[
        ConvergenceAtom::PutRecord,
        ConvergenceAtom::InventoryPage,
        ConvergenceAtom::HeadBatchPerId,
        ConvergenceAtom::HeadRecordVerify,
        ConvergenceAtom::AdoptDeclare,
        ConvergenceAtom::ShardFetch,
        ConvergenceAtom::ManifestPersist,
        ConvergenceAtom::DigestFold,
        ConvergenceAtom::InventoryServe,
        ConvergenceAtom::InventoryRequest,
    ];

    fn label(&self) -> &'static str {
        match self {
            ConvergenceAtom::PutRecord => "put_record",
            ConvergenceAtom::InventoryPage => "inventory_page",
            ConvergenceAtom::HeadBatchPerId => "head_batch_per_id",
            ConvergenceAtom::HeadRecordVerify => "head_record_verify",
            ConvergenceAtom::AdoptDeclare => "adopt_declare",
            ConvergenceAtom::ShardFetch => "shard_fetch",
            ConvergenceAtom::ManifestPersist => "manifest_persist",
            ConvergenceAtom::DigestFold => "digest_fold",
            ConvergenceAtom::InventoryServe => "inventory_serve",
            ConvergenceAtom::InventoryRequest => "inventory_request",
        }
    }
}

/// Record one convergence-path atom's wall-clock cost. The only function that
/// touches [`ATOM_DURATION_MS`], so a new call site never imports `prometheus`
/// directly — pass the already-elapsed [`std::time::Duration`].
pub fn observe_atom_duration(atom: ConvergenceAtom, elapsed: std::time::Duration) {
    use seam_contracts::ReasonLabel as _;
    ATOM_DURATION_MS
        .with_label_values(&[atom.label()])
        .observe(elapsed.as_secs_f64() * 1_000.0);
}

/// Same, taking an already-computed millisecond value — for `head_batch_per_id`,
/// which is a DERIVED share (`elapsed_ms / ids`) rather than a `Duration`
/// measured fresh at the call site.
pub fn observe_atom_duration_ms(atom: ConvergenceAtom, ms: f64) {
    use seam_contracts::ReasonLabel as _;
    ATOM_DURATION_MS
        .with_label_values(&[atom.label()])
        .observe(ms);
}

// ── Conductor-admission typed setters ────────────────────────────────────────

/// Publish the gate's permit count (once, at boot).
pub fn set_admission_capacity(capacity: u32) {
    CONDUCTOR_ADMISSION_CAPACITY.set(i64::from(capacity));
}

/// Record one granted permit: how long it queued, and whether it arrived to an
/// already-full gate (the two are different questions — a call can wait because
/// the gate was full on arrival, or because it filled while the call queued).
///
/// Occupancy moves as a DELTA here and in [`observe_admission_release`], never
/// as an absolute `set`: a concurrent acquire and release would otherwise race
/// a snapshot into the gauge and leave **L** permanently off by one.
pub fn observe_admission_acquire(class: &str, wait: std::time::Duration, arrived_saturated: bool) {
    CONDUCTOR_ADMISSION_IN_FLIGHT.inc();
    CONDUCTOR_ADMISSION_WAIT_MS
        .with_label_values(&[class])
        .observe(wait.as_secs_f64() * 1_000.0);
    CONDUCTOR_ADMISSION_ACQUIRED
        .with_label_values(&[
            class,
            if arrived_saturated {
                "saturated"
            } else {
                "free"
            },
        ])
        .inc();
}

/// Record **W** — a permit released after holding capacity for `held`.
pub fn observe_admission_release(zome: &str, held: std::time::Duration) {
    CONDUCTOR_ADMISSION_HOLD_MS
        .with_label_values(&[zome])
        .observe(held.as_secs_f64() * 1_000.0);
    // Occupancy falls by exactly one; read as a delta so a release cannot race a
    // concurrent acquire's absolute `set` into a stale value.
    CONDUCTOR_ADMISSION_IN_FLIGHT.dec();
}

/// Count one call shed at the gate. NOT a conductor failure — nothing was
/// dispatched, so this must never be aggregated with call-level errors.
pub fn inc_admission_shed(class: &str, zome: &str) {
    CONDUCTOR_ADMISSION_SHED
        .with_label_values(&[class, zome])
        .inc();
}

// ── Blob swarm (Q3/Q4, minutes-quiesce W1.2/W1.3) ────────────────────────────

/// Record one blob-fetch response that answered with a shard manifest
/// instead of bytes (Q3). Called from `p2p::blob_fetch::race_fetch` — the
/// single choke point every manifest-bearing reply passes through,
/// regardless of which caller issued the fetch.
pub fn inc_blob_swarm_manifest_received() {
    BLOB_SWARM_MANIFESTS_RECEIVED.inc();
}

/// Record one per-shard fetch outcome during a Q4 swarm-spread reassembly.
/// `outcome`: "hit" | "miss" | "no_candidates" | "parity_skipped".
pub fn inc_blob_swarm_shard_fetched(outcome: &str) {
    BLOB_SWARM_SHARDS_FETCHED
        .with_label_values(&[outcome])
        .inc();
}

/// Record one composite blob fully reassembled from swarm-fetched shards (Q4).
pub fn inc_blob_swarm_composite_completed() {
    BLOB_SWARM_COMPOSITE_COMPLETED.inc();
}

// ── HTTP dispatch wrapper (route_class × status_class) ───────────────────────

/// Bucket a request path into a CLOSED route class for the
/// `elohim_http_request_duration_ms` / in-flight labels. Prefix-matched, most
/// specific first, so cardinality stays fixed at these 9 values regardless of
/// what a client sends — an unrecognized (or hostile) path lands on
/// `"other"`, it never mints a new series. Label-only: this classifies a
/// measurement and must never be read to decide how a request is served.
pub fn classify_http_route(path: &str) -> &'static str {
    if path.starts_with("/blob/")
        || path.starts_with("/shard/")
        || path.starts_with("/manifest")
        || path.starts_with("/dag/")
        || path.starts_with("/ipfs/")
    {
        "blob"
    } else if path.starts_with("/apps/")
        || path.starts_with("/chrome/")
        || path.starts_with("/spa/")
    {
        "apps"
    } else if path.starts_with("/sync/") {
        "sync"
    } else if path.starts_with("/admin") {
        "admin"
    } else if path.starts_with("/api/") {
        "api"
    } else if path.starts_with("/p2p/") {
        "p2p"
    } else if path.starts_with("/db/")
        || path.starts_with("/epr-head/")
        || path.starts_with("/import/")
    {
        "content"
    } else if path.starts_with("/health")
        || path.starts_with("/version")
        || path.starts_with("/metrics")
    {
        "health"
    } else {
        "other"
    }
}

/// Bucket an HTTP status into the closed `status_class` vocabulary. Any status
/// outside 2xx/4xx — including the `599` sentinel [`RequestMetricsGuard`] uses
/// when a request exits without ever reporting one (cancellation, a panic, an
/// early return nobody instrumented) — lands on `"5xx"`: the conservative
/// choice, since an unreported outcome is exactly what an operator watching
/// this series needs flagged as a possible failure, never silently folded
/// into success.
fn status_class(status: u16) -> &'static str {
    match status {
        200..=299 => "2xx",
        400..=499 => "4xx",
        _ => "5xx",
    }
}

/// RAII guard for `http.rs::handle_request` — the single dispatch point every
/// request passes through. Guarantees ONE `elohim_http_request_duration_ms`
/// observation per request no matter which of the wrapper's several exit
/// points is taken (the normal tail, an early `return` for admission-shed or
/// a streaming upgrade, an escaped-error arm, or the future simply being
/// dropped mid-await because the client disconnected) — because `Drop` runs
/// on every one of those, not just fall-through. That is the fix for the
/// coordinated-omission gap this instrument exists to close: a request that
/// stalls for the full client timeout and is then abandoned still gets
/// counted, at whatever elapsed time it reached.
pub struct RequestMetricsGuard {
    start: std::time::Instant,
    route_class: &'static str,
    status: Option<u16>,
}

impl RequestMetricsGuard {
    /// Start tracking one request: bumps the in-flight gauge immediately. The
    /// paired decrement lives in `Drop`, so every exit path balances the
    /// gauge automatically, including ones added later without touching this
    /// file.
    pub fn start(path: &str) -> Self {
        HTTP_REQUESTS_IN_FLIGHT.inc();
        Self {
            start: std::time::Instant::now(),
            route_class: classify_http_route(path),
            status: None,
        }
    }

    /// Report the outcome status at a known exit point. Consumes the guard so
    /// a call site cannot double-report; `Drop` fires immediately after,
    /// observing with this status. Call sites that never reach a status
    /// (cancellation, panic) simply drop the guard without calling this —
    /// `Drop`'s `599` sentinel covers them.
    pub fn finish(mut self, status: u16) {
        self.status = Some(status);
    }
}

impl Drop for RequestMetricsGuard {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let status = self.status.unwrap_or(599);
        HTTP_REQUEST_DURATION_MS
            .with_label_values(&[self.route_class, status_class(status)])
            .observe(elapsed.as_secs_f64() * 1_000.0);
        HTTP_REQUESTS_IN_FLIGHT.dec();
    }
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

/// Project one conductor read-pool saturation log event into Prometheus.
///
/// `utilization_ratio` uses 1.0 for 100%. The caller normalizes `kind` to the
/// fixed label vocabulary registered above, preventing agent/DNA identifiers
/// from becoming unbounded metric labels.
pub(crate) fn observe_db_read_pool_saturation(kind: &str, utilization_ratio: f64) {
    if !utilization_ratio.is_finite() || utilization_ratio < 0.0 {
        return;
    }
    NODE_DB_READ_POOL_LAST_SATURATION_RATIO
        .with_label_values(&[kind])
        .set(utilization_ratio);
    NODE_DB_READ_POOL_SATURATION_EVENTS
        .with_label_values(&[kind])
        .inc();
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

/// Increment the durable signal decode-miss mirror (paired with the
/// per-family `AtomicU64` in `signals.rs`, additive only). `family` is
/// "infra" | "mishpat" | "elohim_content".
pub fn inc_signal_decode_miss(family: &str) {
    SIGNAL_DECODE_MISS_TOTAL.with_label_values(&[family]).inc();
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

/// Why a `ContentHeadRecord` answer was served hash-only — the label vocabulary
/// of [`CONTENT_HEAD_RECORD_DEGRADED`], as a closed type.
///
/// **Concerns:** C8 (typed reason, closed vocabulary — this counter took a raw
/// `&str` until 2026-08-03, and the branch that never called it was the one that
/// mattered most).
///
/// The three variants are the RESPONDER's three collapse sites, and they are not
/// interchangeable: `no_record` is structural (the conductor answered and holds
/// nothing — no capacity relief changes it), while `conductor_error` and
/// `budget_elapsed` are load-shaped (the bytes plausibly exist and the responder
/// could not get at them in time). Reading a saturation wall as a structural one
/// sends an operator to delete content that was merely starved.
///
/// **Contract test:**
/// [`crate::p2p::view_federation::tests::head_record_degrade_labels_are_stable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadRecordDegraded {
    /// The conductor answered cleanly and holds NO record for this head. The
    /// structural absence — the only one that says "these bytes are nowhere",
    /// and the only branch that shipped uncounted.
    NoRecord,
    /// The conductor answered with an error.
    ConductorError,
    /// The responder's own budget (`HEAD_RECORD_CONDUCTOR_TIMEOUT`) fired first.
    BudgetElapsed,
}

impl seam_contracts::ReasonLabel for HeadRecordDegraded {
    const ALL: &'static [Self] = &[
        HeadRecordDegraded::NoRecord,
        HeadRecordDegraded::ConductorError,
        HeadRecordDegraded::BudgetElapsed,
    ];

    fn label(&self) -> &'static str {
        match self {
            HeadRecordDegraded::NoRecord => "no_record",
            HeadRecordDegraded::ConductorError => "conductor_error",
            HeadRecordDegraded::BudgetElapsed => "budget_elapsed",
        }
    }
}

/// Record one `ContentHeadRecord` answer served hash-only, by cause.
///
/// The `conductor_error` and `budget_elapsed` label STRINGS are byte-identical
/// to the raw literals this function took before it was typed, so no existing
/// dashboard series moves.
pub fn inc_content_head_record_degraded(cause: HeadRecordDegraded) {
    use seam_contracts::ReasonLabel as _;
    CONTENT_HEAD_RECORD_DEGRADED
        .with_label_values(&[cause.label()])
        .inc();
}

// ---------------------------------------------------------------------------
// iroh plane helpers
// ---------------------------------------------------------------------------

/// Set the number of dialable iroh peers (self excluded).
pub fn set_iroh_peers_known(n: usize) {
    IROH_PEERS_KNOWN.set(n as i64);
}

/// Record one transport-manifest announcement by result ("accepted" |
/// "stale" | "bad_signature" | "decode_failed" | "self").
pub fn inc_iroh_manifest_announcement(result: &str) {
    IROH_MANIFEST_ANNOUNCEMENTS
        .with_label_values(&[result])
        .inc();
}

/// Record an iroh-gossip neighbor transition ("up" | "down").
pub fn inc_iroh_gossip_neighbor(direction: &str) {
    IROH_GOSSIP_NEIGHBOR_EVENTS
        .with_label_values(&[direction])
        .inc();
}

/// Record one gossip message received over iroh on `topic`.
pub fn inc_iroh_gossip_received(topic: &str) {
    IROH_GOSSIP_RECEIVED.with_label_values(&[topic]).inc();
}

/// Record one iroh sync round.
pub fn inc_iroh_sync_round() {
    IROH_SYNC_ROUNDS.inc();
}

/// Record one outbound iroh sync request ("list_documents" | "sync_changes" |
/// "announce_change" | "announce_pull") with its result ("ok" | "dial_failed" |
/// "request_failed" | "error_response").
pub fn inc_iroh_sync_request(kind: &str, result: &str) {
    IROH_SYNC_REQUESTS.with_label_values(&[kind, result]).inc();
}

/// Record Automerge changes applied from an iroh sync answer.
pub fn add_iroh_sync_changes_applied(n: u64) {
    IROH_SYNC_CHANGES_APPLIED.inc_by(n);
}

/// Record one iroh-blobs fetch by result.
pub fn inc_iroh_blob_fetch(result: &str) {
    IROH_BLOB_FETCHES.with_label_values(&[result]).inc();
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
/// "connection_closed" | "dial_failure" | "unsupported_protocols" | "io"),
/// classified by `peer_class` — see [`SYNC_REQUEST_OUTCOMES`] on why that is a
/// bounded reach-tier classification (from `PeerTrustCache::peer_class_for`)
/// rather than a raw peer id.
pub fn inc_sync_request_outcome(result: &str, peer_class: &str) {
    SYNC_REQUEST_OUTCOMES
        .with_label_values(&[result, peer_class])
        .inc();
}

/// Record one document re-queued by the bounded sync-fetch window after a
/// transport io refusal. See [`SYNC_FETCH_WINDOW_REQUEUED`].
pub fn inc_sync_fetch_window_requeued() {
    SYNC_FETCH_WINDOW_REQUEUED.inc();
}

/// Counts sync rounds that short-circuited on a matching corpus digest —
/// the converged steady state where the opener costs O(1) instead of O(corpus).
/// A flat-zero value after the digest opener lands means the shortcut never
/// fires and the optimisation is inert.
pub fn inc_sync_in_sync() {
    SYNC_IN_SYNC_TOTAL.inc();
}

/// Why one scheduled acquisition reconcile did or did not complete.
///
/// This is a closed operational vocabulary (C8/C14): every tick is counted,
/// including success, and every early return has its own stable label. That
/// makes `active_pins=0` distinguishable from a pass that never reached the pin
/// query. These are Category C observations only; they confer no protocol
/// authority and are reconstructed after restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AcquisitionReconcileOutcome {
    Completed,
    SyncPaused,
    DbPoolMissing,
    DbPoolUnavailable,
    PinLoadFailed,
    PresenceQueryFailed,
}

impl seam_contracts::ReasonLabel for AcquisitionReconcileOutcome {
    const ALL: &'static [Self] = &[
        Self::Completed,
        Self::SyncPaused,
        Self::DbPoolMissing,
        Self::DbPoolUnavailable,
        Self::PinLoadFailed,
        Self::PresenceQueryFailed,
    ];

    fn label(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::SyncPaused => "sync_paused",
            Self::DbPoolMissing => "db_pool_missing",
            Self::DbPoolUnavailable => "db_pool_unavailable",
            Self::PinLoadFailed => "pin_load_failed",
            Self::PresenceQueryFailed => "presence_query_failed",
        }
    }
}

/// Count one scheduled acquisition-reconcile outcome.
pub fn inc_acquisition_reconcile_outcome(outcome: AcquisitionReconcileOutcome) {
    use seam_contracts::ReasonLabel as _;
    ACQUISITION_RECONCILE_OUTCOMES
        .with_label_values(&[outcome.label()])
        .inc();
}

/// Publish the first/latest completed acquisition sample.
///
/// `active_pins=0` is intentionally materialized beside `initialized=1`: it is
/// the evidence that an empty desired set was observed, rather than assumed.
pub fn set_acquisition_reconcile_completed(active_pins: usize) {
    ACQUISITION_ACTIVE_PINS.set(active_pins as i64);
    ACQUISITION_RECONCILE_INITIALIZED.set(1);
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

/// Count one acquisition dispatch on `transport` (`libp2p` | `iroh`).
pub fn inc_acquisition_dispatch(transport: &str) {
    ACQUISITION_DISPATCH.with_label_values(&[transport]).inc();
}

/// Record `n` content heads freshly authored by the witness-bootstrap sweep.
pub fn add_content_witness_authored(n: u64) {
    CONTENT_WITNESS_AUTHORED.inc_by(n);
}

/// Record one Hold/Contest decision released to Author by ghost-declaration
/// decay (see `head_adoption::ghost_decay_authorizes_author`).
pub fn inc_ghost_declaration_decay_author() {
    GHOST_DECLARATION_DECAY_AUTHOR.inc();
}

/// Why the ghost-declaration decay arm did NOT release a Hold/Contest — the
/// label vocabulary of [`CONTENT_GHOST_DECAY_BLOCKED`], as a closed type.
///
/// One label per leg of `head_adoption::ghost_decay_authorizes_author`, in the
/// predicate's own evaluation order — which is also the operator's triage order:
/// a pod that never got the flag is a different problem from a corpus whose
/// advertisers have gone quiet.
///
/// **Concerns:** C8 (observability-per-decision — the refusal reason is a typed
/// label, never a raw string, and the vocabulary enumerates itself so every
/// series is pre-touched and a typo cannot mint a sixth).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhostDecayBlockedLeg {
    /// `ELOHIM_GHOST_DECLARATION_DECAY` is off on this pod — the arm is dormant
    /// by operator decision. Behaviour-identical to the pre-decay build.
    Disabled,
    /// The own conductor did not ANSWER empty for this id this sweep (it
    /// answered with a head, or was unreachable/unprobed — neither is the
    /// positive local absence C4 requires).
    LocalNotObservedAbsent,
    /// The row obeys a local canonical election: it is settled, never decayed.
    LocalElectionStands,
    /// No peer advertised a declared head for this id in this sweep's hint
    /// window — there is no live declaration to falsify, and stale ledger
    /// hearsay alone must never fire the arm.
    NoLiveAdvertiser,
    /// A live advertiser states no record, but that verdict has not yet STOOD
    /// the decay dwell (`config::ghost_decay_min_dwell`). The expected steady
    /// state for a corpus still inside its dwell — waiting, not stuck.
    EvidenceNotStood,
}

impl seam_contracts::ReasonLabel for GhostDecayBlockedLeg {
    const ALL: &'static [Self] = &[
        GhostDecayBlockedLeg::Disabled,
        GhostDecayBlockedLeg::LocalNotObservedAbsent,
        GhostDecayBlockedLeg::LocalElectionStands,
        GhostDecayBlockedLeg::NoLiveAdvertiser,
        GhostDecayBlockedLeg::EvidenceNotStood,
    ];

    fn label(&self) -> &'static str {
        match self {
            GhostDecayBlockedLeg::Disabled => "disabled",
            GhostDecayBlockedLeg::LocalNotObservedAbsent => "local_not_observed_absent",
            GhostDecayBlockedLeg::LocalElectionStands => "local_election_stands",
            GhostDecayBlockedLeg::NoLiveAdvertiser => "no_live_advertiser",
            GhostDecayBlockedLeg::EvidenceNotStood => "evidence_not_stood",
        }
    }
}

/// Record one Hold/Contest decision the decay arm considered and did not
/// release, by the leg that blocked it.
///
/// **Concerns:** C8 — the negative path is as visible as the positive one, so a
/// pod draining no phantoms says WHY rather than going silent.
pub fn inc_ghost_decay_blocked(leg: GhostDecayBlockedLeg) {
    use seam_contracts::ReasonLabel as _;
    CONTENT_GHOST_DECAY_BLOCKED
        .with_label_values(&[leg.label()])
        .inc();
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
/// Count an own-conductor head answer by election tier: `"earned"`, `"staging"`,
/// or `"none"` (root-author fallback — no canonical winner resolved).
pub fn inc_content_canonical_answer(tier: &str) {
    CONTENT_CANONICAL_ANSWERS.with_label_values(&[tier]).inc();
}

/// Count a `HealCanonical` refusal by reason (`StaleReason::label`).
pub fn inc_projection_refused_stale(reason: &str) {
    PROJECTION_REFUSED_STALE_REASONS
        .with_label_values(&[reason])
        .inc();
}

/// `source` label for a link minted by the ADOPT-BEFORE-AUTHOR arm — a node with
/// NO local chain for the id declaring from a wasm-proven carried record.
///
/// A named const rather than a literal at the call sites because this label IS
/// the flag's live probe (see [`CONTENT_CANONICAL_LINKS_MINTED`]): a typo would
/// mint a permanently-zero series and read as "the operator flip did nothing".
pub const MINTED_SOURCE_ADOPT_BEFORE_AUTHOR: &str = "adopt_before_author";

/// `source` label for a link minted by the SPIN-discharge arm — an UNDECLARED
/// row nominating its OWN head against a peer-advertised divergent anchor
/// (`services::head_adoption::contest_divergent`). Named const for the same
/// reason as [`MINTED_SOURCE_ADOPT_BEFORE_AUTHOR`]: this label is the arm's
/// live probe.
pub const MINTED_SOURCE_CONTEST_DIVERGENT_SELF: &str = "contest_divergent_self";

/// Count a canonical-head declaration LINK minted through this node, by the
/// channel that caused it (`"adopt_peer"`, `"contest_peer_head"`,
/// `"contest_self_head"`, [`MINTED_SOURCE_ADOPT_BEFORE_AUTHOR`],
/// [`MINTED_SOURCE_CONTEST_DIVERGENT_SELF`], `"http"`).
pub fn inc_content_canonical_link_minted(source: &str) {
    CONTENT_CANONICAL_LINKS_MINTED
        .with_label_values(&[source])
        .inc();
}

/// Why a contest attempt minted no canonical-head candidate — the label
/// vocabulary of [`CONTENT_CONTEST_FAILED`], as a closed type.
///
/// **Concerns:** C8 (observability-per-decision — a typed reason, never a raw
/// string), C4 (the `fetch_none` / `not_retrievable` split is an honest-absence
/// distinction: "we got no bytes" is not "the bytes were rejected").
///
/// **Forcing incident:** these four classes were raw `&str` literals at four
/// call sites in `services::head_adoption` (two inline, two through a
/// `unresolvable_class` variable) with no test anywhere asserting the set stayed
/// distinct or stable — so a typo would have minted a fifth, permanently-zero
/// series and nothing would have gone red. Registered as a known gap in
/// `elohim/elohim-storage/seam-registry.yaml`; closed by plan task P1.3.
///
/// **Contract test:** [`tests::contest_failure_labels_are_stable`] — the label
/// strings are pinned BYTE-IDENTICAL to the pre-P1.3 literals, because
/// `elohim_content_contest_failed_total{class}` panels and alerts are keyed on
/// them. Renaming one silently zeroes every panel that reads it.
///
/// Variant order matches the failure-class narrative in
/// [`CONTENT_CONTEST_FAILED`]'s doc: starved upstream → walled with bytes →
/// walled without bytes → refused for any other reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContestFailure {
    /// This conductor holds no chain for the id at all — the target-independent
    /// first gate. Contest is starved upstream; not fixable in storage.
    NoLocalChain,
    /// The chain exists but the peer's head action could not be resolved, AND we
    /// had carried bytes to offer.
    NotRetrievable,
    /// Same wall, but the fetch returned nothing to carry — points at the
    /// head-record transport rather than the conductor's DHT view.
    FetchNone,
    /// Anything else the conductor refused; always paired with a WARN carrying
    /// the verbatim message.
    DeclareError,
    /// ADOPT-BEFORE-AUTHOR (operator-reserved, default OFF): the flag was on,
    /// validated-shape carried bytes were in hand, and the conductor STILL
    /// refused the chainless declaration.
    ///
    /// A genuinely NEW refusal shape, which is why it is a new variant rather
    /// than folded into `declare_error`: it can only be produced by the evidence
    /// branch, so it isolates "the bypass ran and the zome rejected the
    /// evidence" from "the classic path failed". Structurally zero while the
    /// flag is off — the arm that increments it is inside the flag's `if`.
    AdoptRefused,
}

impl seam_contracts::ReasonLabel for ContestFailure {
    const ALL: &'static [Self] = &[
        ContestFailure::NoLocalChain,
        ContestFailure::NotRetrievable,
        ContestFailure::FetchNone,
        ContestFailure::DeclareError,
        // ADDITIVE. Appended, never inserted: the four labels above are pinned
        // byte-identical to the pre-P1.3 literals because dashboards key on
        // them, and appending is the only shape that cannot renumber a series.
        ContestFailure::AdoptRefused,
    ];

    fn label(&self) -> &'static str {
        match self {
            ContestFailure::NoLocalChain => "no_local_chain",
            ContestFailure::NotRetrievable => "not_retrievable",
            ContestFailure::FetchNone => "fetch_none",
            ContestFailure::DeclareError => "declare_error",
            ContestFailure::AdoptRefused => "adopt_refused",
        }
    }
}

/// Count a contest attempt that minted nothing, by failure class.
///
/// **Concerns:** C8 — the class is a [`ContestFailure`], so the counter cannot
/// grow an unannounced label and a dashboard can enumerate what it may see.
pub fn inc_contest_failed(class: ContestFailure) {
    use seam_contracts::ReasonLabel as _;
    CONTENT_CONTEST_FAILED
        .with_label_values(&[class.label()])
        .inc();
}

/// Why a contest attempt was DECLINED before it was made — the label vocabulary
/// of [`CONTENT_CONTEST_SKIPPED`], as a closed type.
///
/// Deliberately a SEPARATE type from [`ContestFailure`] rather than two more
/// variants on it: a skip is not a failure. Folding them would make
/// `contest_failed_total` count work that was never done, and the whole point of
/// the F-B lever is to watch failures fall while skips rise. Additive by
/// construction — a new backoff shape adds a variant here; no existing label is
/// ever renamed (a renamed label silently zeroes a dashboard series).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContestSkip {
    /// A previous contest for this id hit the zome's target-independent
    /// no-chain gate. Nothing about a fresh attempt can pass it until this
    /// conductor holds a chain for the id.
    NoLocalChainBackoff,
    /// A previous SELF-head candidacy for this id was refused by the conductor.
    /// The chain exists; this row's own declared head is what cannot be
    /// resolved, and that does not change between sweeps either.
    SelfCandidacyBackoff,
    /// A previous peer-head declaration failed for a conductor reason outside
    /// the expected not-retrievable refusal. Retrying it every sweep spends the
    /// same scarce contest budget on the same id, so it uses the ordinary
    /// finite contest window while retaining its own observable cause.
    DeclareErrorBackoff,
    /// A previous contest hit the no-chain gate AND the advertising peer's
    /// responder stated that its conductor holds NO record for the head it
    /// advertises ([`HeadRecordDegraded::NoRecord`]). The evidence this arm needs
    /// does not exist on the only peer offering it, so the id is held for a
    /// LONGER window than the ordinary no-chain backoff — but held, never
    /// excluded: bytes can appear later (a genesis story gets witnessed), and the
    /// window expiry re-admits the id with no intervention.
    EvidenceAbsentBackoff,
}

impl seam_contracts::ReasonLabel for ContestSkip {
    const ALL: &'static [Self] = &[
        ContestSkip::NoLocalChainBackoff,
        ContestSkip::SelfCandidacyBackoff,
        ContestSkip::DeclareErrorBackoff,
        ContestSkip::EvidenceAbsentBackoff,
    ];

    fn label(&self) -> &'static str {
        match self {
            ContestSkip::NoLocalChainBackoff => "no_local_chain_backoff",
            ContestSkip::SelfCandidacyBackoff => "self_candidacy_backoff",
            ContestSkip::DeclareErrorBackoff => "declare_error_backoff",
            ContestSkip::EvidenceAbsentBackoff => "evidence_absent_backoff",
        }
    }
}

/// What the adopt/contest arm learned when it went looking for a peer's head
/// `Record` — the label vocabulary of [`CONTENT_ADOPT_EVIDENCE`], as a closed
/// type.
///
/// **Concerns:** C8 (typed reason, closed vocabulary). C14 (the residual is
/// witnessed: a fetch that yields no bytes now names WHY, instead of vanishing
/// into a single `carried=false` bit).
///
/// Deliberately separate from [`HeadRecordDegraded`] even though three variants
/// share their label strings: this vocabulary is the REQUESTER's, and it has a
/// success member (`Carried`) plus an `Unknown` the responder-side type must
/// never have. Folding them would give the responder a `carried` label it can
/// never emit and the requester a closed set it cannot honour across a
/// mixed-version fleet.
///
/// **Contract test:**
/// [`crate::services::head_adoption::tests::adopt_evidence_labels_are_stable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdoptEvidence {
    /// The peer served the head AND its `Record` bytes. The only state the
    /// adopt-before-author exit can act on.
    Carried,
    /// Hash-only, and the responder said its conductor holds NO record. The
    /// STRUCTURAL absence: re-asking this peer cannot help.
    NoRecord,
    /// Hash-only, and the responder's own budget elapsed. The bytes plausibly
    /// exist; the advertiser's conductor is saturated (the C11 class).
    BudgetElapsed,
    /// Hash-only, and the responder's conductor answered with an error.
    ConductorError,
    /// No cause is available: a hash-only answer from a peer too old to state
    /// one, an unrecognised future reason, or an `Absent`/`Unreachable` answer
    /// that established nothing about why bytes are missing.
    ///
    /// This is the mixed-version bucket, and it must stay a DISTINCT state:
    /// silently folding it into `no_record` would apply the long evidence-absent
    /// backoff on no evidence at all.
    Unknown,
}

impl seam_contracts::ReasonLabel for AdoptEvidence {
    const ALL: &'static [Self] = &[
        AdoptEvidence::Carried,
        AdoptEvidence::NoRecord,
        AdoptEvidence::BudgetElapsed,
        AdoptEvidence::ConductorError,
        AdoptEvidence::Unknown,
    ];

    fn label(&self) -> &'static str {
        match self {
            AdoptEvidence::Carried => "carried",
            AdoptEvidence::NoRecord => "no_record",
            AdoptEvidence::BudgetElapsed => "budget_elapsed",
            AdoptEvidence::ConductorError => "conductor_error",
            AdoptEvidence::Unknown => "unknown",
        }
    }
}

/// Count ONE adopt-arm evidence resolution.
///
/// **Concerns:** C8 — called from the single `resolve_peer_evidence` site in
/// `services::head_adoption`, so the sum over states is exactly the number of
/// head-record fetches the arm performed and every state has the same
/// denominator.
pub fn inc_adopt_evidence(state: AdoptEvidence) {
    use seam_contracts::ReasonLabel as _;
    CONTENT_ADOPT_EVIDENCE
        .with_label_values(&[state.label()])
        .inc();
}

/// What the advertiser-diversity fallback decided on ONE adopt-evidence
/// resolution — the label vocabulary of [`CONTENT_ADOPT_EVIDENCE_FALLBACK`], as
/// a closed type.
///
/// **Concerns:** C8 (typed reason, closed vocabulary). C6a (the vocabulary
/// itself encodes the amplification bound: there is exactly ONE `attempted` per
/// resolution, so `attempted` can never exceed `adopt_evidence` total).
///
/// **Contract test:**
/// [`crate::services::head_adoption::tests::fallback_labels_are_stable`].
///
/// Deliberately separate from [`AdoptEvidence`]: this vocabulary answers "did we
/// route around a weak advertiser, and did it help?", which is a different
/// question from "what did the evidence say?". Folding them would double-count
/// the evidence meter, whose whole value is one increment per ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdoptEvidenceFallback {
    /// An alternative advertiser was selected and asked — counted PER FETCH.
    ///
    /// Was exactly one per fallback until the source-widening lever (2026-08-07)
    /// let one resolution walk up to
    /// [`crate::config::evidence_fallback_max_alternates`] couriers. It is
    /// therefore the round-trip denominator (`carried / attempted` = the
    /// per-ask hit rate), while `carried` + `degraded` + `no_alternative`
    /// remain exactly one per RESOLUTION. `attempted / (carried + degraded)` is
    /// the mean ladder depth, and its rise above 1 is the lever engaging.
    Attempted,
    /// An alternative advertiser served the `Record` bytes the primary could
    /// not. The whole point of the lever. One per resolution — the ladder stops
    /// at the first courier that carries.
    Carried,
    /// EVERY alternative answered too, and all without bytes. The primary's
    /// evidence stands as the resolution. One per resolution.
    Degraded,
    /// A fallback was WARRANTED (the primary answered degraded) and no distinct
    /// candidate existed. Not a failure of peer health — a failure of hint
    /// plurality, which is a different remedy (more advertisers per id).
    NoAlternative,
}

impl seam_contracts::ReasonLabel for AdoptEvidenceFallback {
    const ALL: &'static [Self] = &[
        AdoptEvidenceFallback::Attempted,
        AdoptEvidenceFallback::Carried,
        AdoptEvidenceFallback::Degraded,
        AdoptEvidenceFallback::NoAlternative,
    ];

    fn label(&self) -> &'static str {
        match self {
            AdoptEvidenceFallback::Attempted => "attempted",
            AdoptEvidenceFallback::Carried => "carried",
            AdoptEvidenceFallback::Degraded => "degraded",
            AdoptEvidenceFallback::NoAlternative => "no_alternative",
        }
    }
}

/// Count ONE advertiser-diversity fallback decision.
pub fn inc_adopt_evidence_fallback(outcome: AdoptEvidenceFallback) {
    use seam_contracts::ReasonLabel as _;
    CONTENT_ADOPT_EVIDENCE_FALLBACK
        .with_label_values(&[outcome.label()])
        .inc();
}

/// Contest-backoff ledger snapshot persistence outcomes — the label vocabulary
/// of [`CONTENT_CONTEST_BACKOFF_SNAPSHOT`], as a closed type.
///
/// Every member is NON-FATAL by construction: a failed save leaves the in-memory
/// ledger authoritative, and a failed load starts empty — which is exactly the
/// behaviour before the sidecar existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackoffSnapshot {
    /// One periodic snapshot written (temp + rename).
    Saved,
    /// A snapshot write failed. The ledger stays dirty and the next tick retries.
    SaveFailed,
    /// A snapshot was restored at boot (once per process, at most).
    Loaded,
    /// A snapshot existed but was unreadable/undecodable/wrong-version. Start
    /// empty — never crash on operational state.
    LoadFailed,
}

impl seam_contracts::ReasonLabel for BackoffSnapshot {
    const ALL: &'static [Self] = &[
        BackoffSnapshot::Saved,
        BackoffSnapshot::SaveFailed,
        BackoffSnapshot::Loaded,
        BackoffSnapshot::LoadFailed,
    ];

    fn label(&self) -> &'static str {
        match self {
            BackoffSnapshot::Saved => "saved",
            BackoffSnapshot::SaveFailed => "save_failed",
            BackoffSnapshot::Loaded => "loaded",
            BackoffSnapshot::LoadFailed => "load_failed",
        }
    }
}

/// Count ONE contest-backoff snapshot persistence outcome.
pub fn inc_contest_backoff_snapshot(outcome: BackoffSnapshot) {
    use seam_contracts::ReasonLabel as _;
    CONTENT_CONTEST_BACKOFF_SNAPSHOT
        .with_label_values(&[outcome.label()])
        .inc();
}

/// Count a contest attempt skipped by the backoff ledger, by reason.
///
/// **Concerns:** C8 — typed reason, closed vocabulary. C3 — every reason here
/// names a state with an automated exit (see `services::contest_backoff`), so a
/// non-zero series can never mean "these ids were abandoned".
pub fn inc_contest_skipped(reason: ContestSkip) {
    use seam_contracts::ReasonLabel as _;
    CONTENT_CONTEST_SKIPPED
        .with_label_values(&[reason.label()])
        .inc();
}

/// Count one contest-backoff ledger cap-overflow fail-open clear.
pub fn inc_contest_backoff_cleared() {
    CONTENT_CONTEST_BACKOFF_CLEARED.inc();
}

/// How one adopt-before-author sweep ended — the label vocabulary of
/// [`CONTENT_ADOPT_SWEEP`], as a closed type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdoptSweepOutcome {
    /// The sweep drained its candidate slice inside the wall-clock budget.
    Completed,
    /// The wall-clock budget elapsed first; the remainder resumes next sweep.
    BudgetElapsed,
}

impl seam_contracts::ReasonLabel for AdoptSweepOutcome {
    const ALL: &'static [Self] = &[
        AdoptSweepOutcome::Completed,
        AdoptSweepOutcome::BudgetElapsed,
    ];

    fn label(&self) -> &'static str {
        match self {
            AdoptSweepOutcome::Completed => "completed",
            AdoptSweepOutcome::BudgetElapsed => "budget_elapsed",
        }
    }
}

/// The verification depth a trust-priced adoption decision chose — the label
/// vocabulary of [`TRUST_PRICED_ADOPTIONS`], as a closed type.
///
/// A mirror of `trust::pricer::VerificationDepth`, kept SEPARATE for the same
/// reason every other label vocabulary in this file is: `metrics` owns the wire
/// strings a dashboard queries, and a rename in the domain enum must be a
/// deliberate decision here rather than a silent series rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustPricedDepth {
    /// The declared grant licensed accepting carried provenance — the per-id
    /// evidence round-trips were skipped.
    AcceptWithProvenance,
    /// Verify only what changed. No head-plane consumer routes here today.
    DeltaVerify,
    /// Re-derive and verify everything — today's behavior, and the answer at
    /// every stage that is not an explicit Simulacra declaration.
    FullChain,
}

impl seam_contracts::ReasonLabel for TrustPricedDepth {
    const ALL: &'static [Self] = &[
        TrustPricedDepth::AcceptWithProvenance,
        TrustPricedDepth::DeltaVerify,
        TrustPricedDepth::FullChain,
    ];

    fn label(&self) -> &'static str {
        match self {
            TrustPricedDepth::AcceptWithProvenance => "accept_with_provenance",
            TrustPricedDepth::DeltaVerify => "delta_verify",
            TrustPricedDepth::FullChain => "full_chain",
        }
    }
}

/// Count ONE priced adoption decision at the depth the pricer chose.
///
/// **Concerns:** C8 — every priced decision is counted, including the
/// full-chain ones, so `accept_with_provenance / (sum of all)` reads directly
/// as the share of the sweep the declared grant actually compressed.
pub fn inc_trust_priced_adoption(depth: TrustPricedDepth) {
    use seam_contracts::ReasonLabel as _;
    TRUST_PRICED_ADOPTIONS
        .with_label_values(&[depth.label()])
        .inc();
}

/// Count one completed adopt sweep by how it ended.
///
/// **Concerns:** C8 — both outcomes are counted, so a sweep that runs out of
/// budget is as visible as one that finishes. A budget-elapsed-dominated series
/// is the signal that the fan-out is already saturating the conductor.
pub fn inc_adopt_sweep(outcome: AdoptSweepOutcome) {
    use seam_contracts::ReasonLabel as _;
    CONTENT_ADOPT_SWEEP
        .with_label_values(&[outcome.label()])
        .inc();
}

/// How ONE election-obey probe ended — the label vocabulary of
/// [`CONTENT_ELECTION_OBEY_PROBE`], as a closed type.
///
/// **Concerns:** C8 (observability-per-decision — a typed reason, never a raw
/// string; the vocabulary enumerates itself, so a dashboard can list what it may
/// see and a typo cannot mint a fifth permanently-zero series). C14 (witnessed
/// residual, PARTIAL — see the registry row: the three previously-silent exits
/// are now counted and the `resolve_error` echo meets the `warn!` floor, but no
/// `ResidualWitness` capsule reaches the findings ledger yet).
///
/// Deliberately a SEPARATE type from [`ContestFailure`] and from the
/// `class` vocabulary of [`CONTENT_ELECTION_OBEY_FAILED`], for the same reason
/// [`ContestSkip`] is separate from [`ContestFailure`]: **a probe that never got
/// an election is not an obey failure.** `CONTENT_ELECTION_OBEY_FAILED`'s doc
/// scopes it to had-an-election-didn't-move; folding these four in would make it
/// count work that was never attempted and would strip `fetch` / `validate` /
/// `stamp_refused` of their meaning at the same time.
///
/// **Forcing incident:** 2026-08-03 live diagnosis — the obey arm ran ~900
/// probes/hr and died 100% at `resolve_canonical_election` through two exits
/// that incremented nothing and whose only signal was a `debug!` this deployment
/// drops. Same shape as `486982bb8` (2026-07-25), the incident
/// [`seam_contracts::residual`] itself is written against: *a leg that fails
/// 100% and says nothing*.
///
/// **Contract tests:**
/// [`crate::services::head_adoption::tests::election_obey_probe_labels_are_stable`]
/// (the label strings, pinned as a dashboard contract from their first deploy)
/// and [`tests::election_obey_probe_outcomes_are_pretouched_at_boot`] (all four
/// series exist in the scrape from boot, which is the half that was missing).
///
/// Variant order is the denominator first, then the three exits in the order the
/// probe hits them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionObeyProbe {
    /// A probe entered `try_obey_visible_election`. The DENOMINATOR — every
    /// other variant is a subset of this one, never disjoint from it.
    Attempted,
    /// `resolve_canonical_election` answered `Ok(None)`: no election is visible
    /// to this conductor for this id.
    NoElection,
    /// `resolve_canonical_election` returned `Err`: the conductor would not
    /// answer the election read at all.
    ResolveError,
    /// An election IS visible, but there is no peer hint and/or no head-record
    /// fetcher — nobody to hand us the winner's bytes.
    NoCourier,
}

impl seam_contracts::ReasonLabel for ElectionObeyProbe {
    const ALL: &'static [Self] = &[
        ElectionObeyProbe::Attempted,
        ElectionObeyProbe::NoElection,
        ElectionObeyProbe::ResolveError,
        ElectionObeyProbe::NoCourier,
    ];

    fn label(&self) -> &'static str {
        match self {
            ElectionObeyProbe::Attempted => "attempted",
            ElectionObeyProbe::NoElection => "no_election",
            ElectionObeyProbe::ResolveError => "resolve_error",
            ElectionObeyProbe::NoCourier => "no_courier",
        }
    }
}

/// Count one election-obey probe outcome.
///
/// **Concerns:** C8 — typed reason, closed vocabulary, and the `Attempted`
/// denominator is counted BESIDE the exits so a failure rate is computable
/// rather than inferred from a flat success series.
pub fn inc_election_obey_probe(outcome: ElectionObeyProbe) {
    use seam_contracts::ReasonLabel as _;
    CONTENT_ELECTION_OBEY_PROBE
        .with_label_values(&[outcome.label()])
        .inc();
}

/// Why a custody-rotation pass did NOT author a successor pledge for one
/// examined `custody-blob` commitment.
///
/// A closed vocabulary so every examined row lands in exactly one bucket beside
/// the [`CUSTODY_ROTATION_AUTHORED`] success series — a rotation rate is
/// readable, not inferred. Variant order is the order
/// [`crate::services::custody_rotation`] evaluates them: the detection
/// decisions first, then the acting ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustodyRotationSkip {
    /// The pledge's `metadata_json` resolves no `contentId` — absent,
    /// unparseable, or carrying no such key. We never guess which content a
    /// pledge meant (C4).
    NoContentId,
    /// The bound content row is missing in this scope, or its blob column for
    /// the pledge's artifact role is NULL/empty.
    ContentBlobAbsent,
    /// The pledge already names the content's current blob — the healthy
    /// steady state, and the denominator's dominant series.
    NotDivergent,
    /// A successor for `(provider, receiver, current blob)` already exists;
    /// replaying the pass mints nothing (C6b).
    SuccessorExists,
    /// The local pantry does not hold the current bytes — a node never pledges
    /// custody of bytes it cannot serve.
    BytesAbsent,
    /// The notarized successor create (or its activation) failed; retried next
    /// tick, predecessor left standing.
    AuthorFailed,
    /// The successor was authored but the predecessor's supersession mark
    /// failed.
    SupersedeFailed,
}

impl seam_contracts::ReasonLabel for CustodyRotationSkip {
    const ALL: &'static [Self] = &[
        CustodyRotationSkip::NoContentId,
        CustodyRotationSkip::ContentBlobAbsent,
        CustodyRotationSkip::NotDivergent,
        CustodyRotationSkip::SuccessorExists,
        CustodyRotationSkip::BytesAbsent,
        CustodyRotationSkip::AuthorFailed,
        CustodyRotationSkip::SupersedeFailed,
    ];

    fn label(&self) -> &'static str {
        match self {
            CustodyRotationSkip::NoContentId => "no_content_id",
            CustodyRotationSkip::ContentBlobAbsent => "content_blob_absent",
            CustodyRotationSkip::NotDivergent => "not_divergent",
            CustodyRotationSkip::SuccessorExists => "successor_exists",
            CustodyRotationSkip::BytesAbsent => "bytes_absent",
            CustodyRotationSkip::AuthorFailed => "author_failed",
            CustodyRotationSkip::SupersedeFailed => "supersede_failed",
        }
    }
}

/// Count one custody-rotation successor authored AND its predecessor superseded
/// (the complete ceremony — a partial rotation counts a
/// [`CustodyRotationSkip`], never this).
pub fn inc_custody_rotation_authored() {
    CUSTODY_ROTATION_AUTHORED.inc();
}

/// Count one examined custody pledge that did NOT rotate, by typed reason.
pub fn inc_custody_rotation_skipped(reason: CustodyRotationSkip) {
    use seam_contracts::ReasonLabel as _;
    CUSTODY_ROTATION_SKIPPED
        .with_label_values(&[reason.label()])
        .inc();
}

/// Current value of one [`CUSTODY_ROTATION_SKIPPED`] series. The counter is
/// process-wide, so callers compare a before/after delta rather than an
/// absolute.
pub fn custody_rotation_skipped_count(reason: CustodyRotationSkip) -> u64 {
    use seam_contracts::ReasonLabel as _;
    CUSTODY_ROTATION_SKIPPED
        .with_label_values(&[reason.label()])
        .get()
}

/// Count one requester-side head-record fetch outcome, by
/// [`seam_contracts::AnswerState`].
///
/// **Concerns:** C8 — called from inside
/// [`crate::p2p::head_record_client::PeerHeadRecordFetcher::fetch`] itself, so
/// every caller of the head-record client is covered by one inc site.
pub fn inc_head_record_fetch(state: seam_contracts::AnswerState) {
    use seam_contracts::ReasonLabel as _;
    CONTENT_HEAD_RECORD_FETCH_TOTAL
        .with_label_values(&[state.label()])
        .inc();
}

/// Count a row moved to the DHT-elected head (`path` = `"carried"`).
pub fn inc_election_obeyed(path: &str) {
    CONTENT_ELECTION_OBEYED.with_label_values(&[path]).inc();
}

/// Count an election-obey attempt that did not move the row
/// (`fetch` | `validate` | `stamp_refused`).
pub fn inc_election_obey_failed(class: &str) {
    CONTENT_ELECTION_OBEY_FAILED
        .with_label_values(&[class])
        .inc();
}

pub fn inc_projection_heal_outcome(stream: &str, outcome: &str) {
    PROJECTION_HEAL_OUTCOMES
        .with_label_values(&[stream, outcome])
        .inc();
}

/// Bulk form of [`inc_projection_heal_outcome`] — one call for a whole batch
/// (a call-level failure or an unattempted set), rather than looping per id.
/// A no-op for `count == 0` (mirrors [`add_head_batch_unattempted`]'s guard).
pub fn inc_projection_heal_outcome_by(stream: &str, outcome: &str, count: usize) {
    if count == 0 {
        return;
    }
    PROJECTION_HEAL_OUTCOMES
        .with_label_values(&[stream, outcome])
        .inc_by(count as u64);
}

/// Publish a reconcile stream's last-sweep gauges: discovered `gaps` (pending after
/// discovery) and `local_total` (local projection rows). `stream` is "rea" |
/// "content" | "collectives".
///
/// `divergent` is the TOTAL; `divergent_refused` is its adjudicated share (see
/// [`PROJECTION_RECONCILE_DIVERGENT_REFUSED`]) — `divergent_refused <= divergent`
/// always, and the difference is what gates convergence.
/// Publish the anchored-but-scoped-reach population for `stream`.
///
/// Separate from [`set_projection_reconcile_gauges`] because it is computed at
/// classification time inside the content arm (like the window gauges), and
/// because it is NOT a convergence input — it is a normal population that must
/// stay visible, never a gap.
pub fn set_projection_reconcile_reach_scoped(stream: &str, reach_scoped: u64) {
    PROJECTION_RECONCILE_REACH_SCOPED
        .with_label_values(&[stream])
        .set(reach_scoped as i64);
}

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

/// Update ONLY the adjudicated-divergence gauge for a stream, leaving the rest of
/// the stream's last-sweep gauges untouched.
///
/// The REA arm adjudicates in TWO places: retry-exhaustion is known at discovery
/// (published with the rest via [`set_projection_reconcile_gauges`]), but the
/// dominant class — the own conductor answering with the anchor the local row
/// already holds — is only knowable from a conductor answer, i.e. after the heal
/// leg has run. This setter lets the heal leg complete the number without
/// re-publishing discovery-time gauges the heal has since invalidated.
///
/// No new series: this writes the SAME
/// `elohim_projection_reconcile_divergent_refused{stream=…}` gauge.
pub fn set_projection_reconcile_divergent_refused(stream: &str, divergent_refused: u64) {
    PROJECTION_RECONCILE_DIVERGENT_REFUSED
        .with_label_values(&[stream])
        .set(divergent_refused as i64);
}

/// Publish whether the last sweep MEASURED `stream` — see
/// [`PROJECTION_RECONCILE_MEASURED`]. Set unconditionally, every sweep,
/// regardless of whether [`set_projection_reconcile_gauges`] also fires this
/// sweep (it is gated on the same bit the caller passes here).
pub fn set_projection_reconcile_measured(stream: &str, measured: bool) {
    PROJECTION_RECONCILE_MEASURED
        .with_label_values(&[stream])
        .set(i64::from(measured));
}

/// Publish the persistent, cross-sweep known-gap / known-divergent gauges for
/// `stream` — see [`PROJECTION_RECONCILE_KNOWN_GAPS`] /
/// [`PROJECTION_RECONCILE_KNOWN_DIVERGENT`]. `known_gaps` excludes
/// `known_divergent` (the two partition the ledger's tracked set; an id is
/// never double-counted).
pub fn set_projection_reconcile_known_gauges(stream: &str, known_gaps: u64, known_divergent: u64) {
    PROJECTION_RECONCILE_KNOWN_GAPS
        .with_label_values(&[stream])
        .set(known_gaps as i64);
    PROJECTION_RECONCILE_KNOWN_DIVERGENT
        .with_label_values(&[stream])
        .set(known_divergent as i64);
}

/// Publish this sweep's rotating-window progress for `stream` — see
/// [`PROJECTION_RECONCILE_WINDOW_OFFSET`] / [`PROJECTION_RECONCILE_WINDOW_TOTAL`].
pub fn set_projection_reconcile_window_gauges(stream: &str, offset: u64, total: u64) {
    PROJECTION_RECONCILE_WINDOW_OFFSET
        .with_label_values(&[stream])
        .set(offset as i64);
    PROJECTION_RECONCILE_WINDOW_TOTAL
        .with_label_values(&[stream])
        .set(total as i64);
}

/// Count ONE content-arm heal patch whose `reach` field was dropped by the
/// RC-4 non-narrowing guard — see [`PROJECTION_RECONCILE_REACH_NARROWED`].
///
/// `to` is WIRE-DERIVED — the incoming conductor answer's `reach`, sourced
/// from a peer's `ContentHeadWire` (2026-08-09, R1) — so it is clamped to the
/// known [`crate::generated_enums::CORE_REACH_LEVELS`] vocabulary plus
/// `"other"` before it ever becomes a label value. Without the clamp, one peer
/// running a future build (or simply misbehaving) could mint an unbounded
/// number of label combinations on a `IntCounterVec`, which is a Prometheus
/// cardinality footgun the label's own definition already warned about (this
/// counter has no such clamp on `from`, which is read from the LOCAL row —
/// this node's own conductor already validated it against the same
/// vocabulary at the point it was written).
pub fn inc_projection_reconcile_reach_narrowed(from: &str, to: &str) {
    let to = if crate::generated_enums::CORE_REACH_LEVELS.contains(&to) {
        to
    } else {
        "other"
    };
    PROJECTION_RECONCILE_REACH_NARROWED
        .with_label_values(&[from, to])
        .inc();
}

/// The terms of the convergence predicate, in publication order. Every one is
/// published every sweep on [`PROJECTION_RECONCILE_CONVERGED_BLOCKED_BY`].
pub const CONVERGED_BLOCKED_BY_TERMS: [&str; 4] =
    ["pending", "failed", "divergent_actionable", "unmeasured"];

/// Which terms currently prevent convergence. Pure, so the honesty floor is
/// testable without a registry, and so the gauge set and the `converged` bit are
/// computed from ONE expression that cannot drift apart.
///
/// - **`unmeasured`** — this sweep could not observe the thing it reports on: no
///   peer was asked, or an arm short-circuited on a DB/query error and returned
///   an empty discovery. A sweep that measured nothing must not publish
///   convergence; that is a false green, not a conservative one.
/// - **`pending`** — an arm ended the sweep with gaps still outstanding (the
///   heal leg yielded on its wall-clock budget or opened its circuit). This term
///   also carries the `GapCounts::exhausted` bit, which is vacuous on the
///   reconcile arms but real on the cross-cycle trackers.
/// - **`failed`** — a gap this sweep ATTEMPTED ended failed (conductor answered
///   `None`/`Err`). Steady state stays reachable: an id the conductor genuinely
///   cannot see stops being attempted once the `MissLedger` exhausts it after
///   `MAX_RETRIES`, so it no longer produces `failed`. The ledger's re-admit wave
///   every `MISS_READMIT_SWEEPS` dormant sweeps will honestly flap this term for
///   a few sweeps per cycle — that flap is the truth, not noise.
/// - **`divergent_actionable`** — the UNADJUDICATED divergence: the total MINUS
///   the share heal is forbidden to move (a canonical channel owns the row's
///   declared head) or has spent its retry budget against. Passing the TOTAL is
///   the bug this parameter name guards: on a live peer the refused class is
///   dominant and permanent until a canonical channel fires, so a total-gated
///   gauge reads 0 forever no matter how well the heal leg works. Divergence
///   nobody has adjudicated still defeats convergence — that half is honest and
///   is kept.
pub fn converged_blockers(
    counts: &crate::p2p::reconcile_rails::GapCounts,
    divergent_actionable: usize,
    measured: bool,
) -> [(&'static str, bool); 4] {
    [
        ("pending", !counts.converged),
        ("failed", counts.failed > 0),
        ("divergent_actionable", divergent_actionable > 0),
        ("unmeasured", !measured),
    ]
}

/// 1 when the peer holds what its peers advertised, 0 otherwise — true exactly
/// when [`converged_blockers`] reports no blocking term. Pure so the rule is
/// testable without a registry.
pub fn converged_gauge_value(
    counts: &crate::p2p::reconcile_rails::GapCounts,
    divergent_actionable: usize,
    measured: bool,
) -> i64 {
    i64::from(
        !converged_blockers(counts, divergent_actionable, measured)
            .iter()
            .any(|(_, blocked)| *blocked),
    )
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
    measured: bool,
) {
    PROJECTION_RECONCILE_SWEEPS.inc();
    PROJECTION_RECONCILE_CONVERGED.set(converged_gauge_value(
        counts,
        divergent_actionable,
        measured,
    ));
    // The atomic count behind the blocked_by term flag — same numbers, same
    // moment (see the gauge's doc for why cross-gauge subtraction over-reads).
    PROJECTION_RECONCILE_DIVERGENT_ACTIONABLE.set(divergent_actionable as i64);
    // Every term every sweep, so a 0 is evidence rather than an absent series.
    for (term, blocked) in converged_blockers(counts, divergent_actionable, measured) {
        PROJECTION_RECONCILE_CONVERGED_BLOCKED_BY
            .with_label_values(&[term])
            .set(i64::from(blocked));
    }
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

    /// Serializes every test that calls [`record_reconcile_sweep`]: the
    /// prometheus statics are process-global and `cargo test --lib` runs
    /// multithreaded, so a set-then-assert on the actionable count gauge can
    /// race another test's sweep publish without this guard.
    static SWEEP_GAUGE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A fully-healed, fully-measured sweep — the only shape that converges.
    fn healed_sweep() -> GapCounts {
        GapCounts {
            pending: 0,
            completed: 5,
            failed: 0,
            caught_up: true,
            exhausted: 0,
            converged: true,
        }
    }

    #[test]
    fn converged_gauge_needs_all_three_conditions_not_just_an_empty_queue() {
        // measured AND pending==0 AND failed==0 AND divergent_actionable==0.
        // Any one of them defeats it — that is the whole point of the field.
        let healed = healed_sweep();
        assert_eq!(converged_gauge_value(&healed, 0, true), 1);

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
        assert_eq!(converged_gauge_value(&abandoned, 0, true), 0);

        // Clean gap ledger, but rows sit locally under an anchor no peer
        // advertises. The sweep resolved nothing about them.
        assert_eq!(converged_gauge_value(&healed, 1860, true), 0);
    }

    #[test]
    fn an_unmeasured_sweep_never_publishes_convergence() {
        // The N2 false-green: every discovery arm short-circuits to an `empty()`
        // discovery on a DB/query error, whose all-zero counts are BYTE-
        // IDENTICAL to a perfectly healed sweep — and `run_heal` publishes them
        // regardless. Without the `measured` term a database outage is the
        // easiest way to make this gauge read 1.
        let healed = healed_sweep();
        assert_eq!(
            converged_gauge_value(&healed, 0, true),
            1,
            "the same counts, measured, DO converge"
        );
        assert_eq!(
            converged_gauge_value(&healed, 0, false),
            0,
            "a sweep that observed nothing must never claim convergence"
        );
        assert!(
            converged_blockers(&healed, 0, false)
                .iter()
                .any(|(term, blocked)| *term == "unmeasured" && *blocked),
            "and it must say WHY — unmeasured is distinguishable from an honest false"
        );
    }

    #[test]
    fn an_attempted_gap_that_failed_defeats_convergence() {
        // The honesty floor: the conductor answered None/Err for a gap this
        // sweep actually TRIED. `mark_failed` drains it from `pending`, so
        // without this term the sweep reports caught_up + pending 0 and claims
        // convergence having resolved nothing (the live shape: failed 47).
        let mut all_failed = healed_sweep();
        all_failed.completed = 0;
        all_failed.failed = 47;
        assert_eq!(
            converged_gauge_value(&all_failed, 0, true),
            0,
            "a sweep that could not resolve what it attempted has not converged"
        );
        assert!(converged_blockers(&all_failed, 0, true)
            .iter()
            .any(|(term, blocked)| *term == "failed" && *blocked));
    }

    #[test]
    fn steady_state_with_ledger_exhausted_ids_still_converges() {
        // Reachability guard — the floor must not become another pinned gauge.
        // Ids the conductor genuinely cannot see stop being ATTEMPTED once the
        // MissLedger exhausts them, so they produce no `failed`; their divergent
        // share is already folded into `divergent_refused` and subtracted. The
        // steady state is therefore a sweep with a large adjudicated backlog and
        // a clean attempt record — and it converges.
        let steady = GapCounts {
            pending: 0,
            completed: 3,
            failed: 0,
            caught_up: true,
            exhausted: 0,
            converged: true,
        };
        let divergent_actionable = 1951usize.saturating_sub(1951);
        assert_eq!(
            converged_gauge_value(&steady, divergent_actionable, true),
            1,
            "398 ledger-exhausted ids and 1951 adjudicated divergent rows must not veto \
             a sweep that attempted everything it admitted and failed none of it"
        );
        assert!(
            converged_blockers(&steady, divergent_actionable, true)
                .iter()
                .all(|(_, blocked)| !*blocked),
            "no term may block the steady state"
        );
    }

    #[test]
    fn converged_is_one_exactly_when_no_term_blocks() {
        // The invariant that keeps the diagnostic gauge honest: the blocked_by
        // set is not a parallel description of the predicate, it IS the
        // predicate. Every combination of the four terms is checked.
        for pending in [0usize, 2894] {
            for failed in [0usize, 47] {
                for actionable in [0usize, 3] {
                    for measured in [true, false] {
                        let counts = GapCounts {
                            pending,
                            completed: 1,
                            failed,
                            caught_up: pending == 0,
                            exhausted: 0,
                            converged: pending == 0,
                        };
                        let blockers = converged_blockers(&counts, actionable, measured);
                        let any_blocked = blockers.iter().any(|(_, b)| *b);
                        assert_eq!(
                            converged_gauge_value(&counts, actionable, measured) == 1,
                            !any_blocked,
                            "converged must be 1 iff every term is 0 \
                             (pending={pending} failed={failed} actionable={actionable} \
                              measured={measured})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_actionable_count_gauge_and_term_flag_publish_from_one_fold() {
        let _guard = SWEEP_GAUGE_LOCK.lock().unwrap();
        // The CI quiesce gate's tolerance predicate reads the COUNT gauge; the
        // strict predicate reads the term FLAG. Both must come from the same
        // record_reconcile_sweep numbers or the gate's two modes can disagree
        // about the same sweep (the 2026-08-08 cross-gauge-subtraction lesson:
        // per-stream divergent/refused publish at different moments, so a
        // derived sum read 7-11 while the fold's own count read 1-2).
        for actionable in [0usize, 1, 2, 11] {
            record_reconcile_sweep(&healed_sweep(), actionable, true);
            assert_eq!(
                PROJECTION_RECONCILE_DIVERGENT_ACTIONABLE.get(),
                actionable as i64,
                "count gauge must equal the fold's actionable value"
            );
            let flag = PROJECTION_RECONCILE_CONVERGED_BLOCKED_BY
                .with_label_values(&["divergent_actionable"])
                .get();
            assert_eq!(
                flag > 0,
                actionable > 0,
                "term flag must be 1 iff the count is nonzero (actionable={actionable})"
            );
        }
    }

    #[test]
    fn every_predicate_term_is_published_every_sweep() {
        // A term that only appears when it fires is indistinguishable from an
        // emitter that never ran (the inc_identity_fill lesson). All four series
        // must be materialised on every sweep so a 0 is EVIDENCE.
        let published: Vec<&str> = converged_blockers(&healed_sweep(), 0, true)
            .iter()
            .map(|(term, _)| *term)
            .collect();
        assert_eq!(
            published,
            CONVERGED_BLOCKED_BY_TERMS.to_vec(),
            "the published label set must be exactly the declared term set"
        );
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
            converged_gauge_value(&healed, actionable, true),
            1,
            "divergence heal is FORBIDDEN to move must not defeat convergence"
        );

        // The honest half is kept: ONE row nobody has adjudicated still defeats
        // it. This is the guard against the split becoming a whitewash.
        let actionable = 6071usize.saturating_sub(6070);
        assert_eq!(
            converged_gauge_value(&healed, actionable, true),
            0,
            "unadjudicated divergence must still defeat convergence"
        );
    }

    #[test]
    fn register_all_idempotent_and_gathers_all_metrics() {
        let _guard = SWEEP_GAUGE_LOCK.lock().unwrap();
        register_all();
        register_all(); // Once-guarded — must not panic or double-register.

        set_proc_rss("holochain", 5_000_000_000, 130_000_000, 42);
        set_cgroup_mem(5_000_000_000, 130_000_000, 50_000_000, Some(0));
        set_boot_tunables(8, Some(4.0));
        observe_db_read_pool_saturation("dht", 226.62);
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
        // The anchored-but-scoped population. Primed here so the render
        // assertion below can prove the series actually REACHES /metrics — it is
        // the confirm/falsify instrument for the 2026-08-19 classifier cure, and
        // an instrument that never renders cannot settle anything.
        set_projection_reconcile_reach_scoped("content", 1562);
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
            true,
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
        // The bounded fetch window's own signal (2026-08-24): the io refusals
        // it re-queues. Primed so the series is proven to REACH /metrics — an
        // instrument that never renders cannot confirm or falsify the cure.
        inc_sync_fetch_window_requeued();
        for result in [
            "ok",
            "timeout",
            "connection_closed",
            "dial_failure",
            "unsupported_protocols",
            "io",
        ] {
            // peer_class exercised across the bounded vocabulary: a reach
            // tier, the "other" clamp bucket, and "unverified" (no cached
            // trust context) — never a raw peer id.
            for peer_class in ["trusted", "other", "unverified"] {
                inc_sync_request_outcome(result, peer_class);
            }
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

        // Conductor-admission family. Touched here on purpose: a *Vec collector
        // with no label values registers but renders NOTHING, so "I registered
        // it" and "it reaches /metrics" are different claims and only the second
        // one matters to an operator reading a dashboard.
        set_admission_capacity(13);
        observe_admission_acquire("interactive", std::time::Duration::from_millis(4), false);
        observe_admission_acquire("background", std::time::Duration::from_millis(900), true);
        observe_admission_release("content_store", std::time::Duration::from_millis(120));
        inc_admission_shed("background", "content_store");

        let text = gather_text();
        assert!(
            text.contains("elohim_node_proc_rss_bytes"),
            "proc rss gauge missing:\n{text}"
        );
        assert!(text.contains("elohim_node_cgroup_mem_bytes"));
        assert!(text.contains("elohim_node_db_max_readers"));
        assert!(text.contains("elohim_node_db_read_pool_last_saturation_ratio"));
        assert!(text.contains("elohim_node_db_read_pool_saturation_events_total"));
        assert!(
            text.contains("elohim_node_db_read_pool_last_saturation_ratio{kind=\"dht\"} 226.62")
        );
        assert!(text.contains("elohim_identity_namespace_violation_total"));
        // Capacity contract: all six series reach the exposition, and the two
        // that answer "is the pool the binding constraint?" render their labels.
        assert!(
            text.contains("elohim_conductor_admission_capacity 13"),
            "{text}"
        );
        assert!(
            text.contains("elohim_conductor_admission_in_flight"),
            "{text}"
        );
        assert!(
            text.contains("elohim_conductor_admission_wait_ms_bucket"),
            "{text}"
        );
        assert!(
            text.contains("elohim_conductor_admission_hold_ms_bucket"),
            "{text}"
        );
        assert!(
            text.contains("elohim_conductor_admission_acquired_total"),
            "{text}"
        );
        assert!(
            text.contains("elohim_conductor_admission_shed_total"),
            "{text}"
        );
        assert!(text.contains("arrival=\"saturated\""), "{text}");
        assert!(text.contains("zome=\"content_store\""), "{text}");
        // L moved as a DELTA: two acquires, one release ⇒ 1, not a stale snapshot.
        assert!(
            text.contains("elohim_conductor_admission_in_flight 1"),
            "{text}"
        );
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
        for term in CONVERGED_BLOCKED_BY_TERMS {
            assert!(
                text.contains(&format!(
                    "elohim_projection_reconcile_converged_blocked_by{{term=\"{term}\"}}"
                )),
                "blocked-by term {term:?} missing — a fleet stuck at converged=0 must \
                 REPORT which term pins it, not force a code read:\n{text}"
            );
        }
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
        // peer_class: the bounded reach-tier classification that makes the
        // sync edge priceable without an unbounded peer-id label.
        assert!(
            text.contains(
                "elohim_sync_request_outcomes_total{peer_class=\"unverified\",result=\"ok\"}"
            ),
            "sync request-outcome must carry peer_class=unverified for an uncached peer:\n{text}"
        );
        assert!(
            text.contains(
                "elohim_sync_request_outcomes_total{peer_class=\"trusted\",result=\"ok\"}"
            ),
            "sync request-outcome must carry the reach-tier peer_class for a cached peer:\n{text}"
        );
        assert!(
            text.contains("elohim_sync_request_outcomes_total{peer_class=\"other\",result=\"ok\"}"),
            "an out-of-vocabulary reach_ceiling must clamp to peer_class=other:\n{text}"
        );
        assert!(
            text.contains("elohim_sync_fetch_window_requeued_total"),
            "fetch-window re-queue counter missing — without it an io-refusing \
             transport looks the same before and after the bounded window:\n{text}"
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
            text.contains("elohim_projection_reconcile_reach_scoped{stream=\"content\"} 1562"),
            "reach-scoped gauge missing — without it the anchored-but-scoped population \
             is invisible again and the classifier cure cannot be confirmed OR falsified \
             on the fleet:\n{text}"
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

    /// The cure for the two-shift blind spot, asserted as a property of the
    /// SCRAPE rather than of the code that writes it.
    ///
    /// All four `outcome` series must exist from registration alone — before any
    /// probe has run. `attempted` is the one that matters most: without it a
    /// flat-zero `elohim_content_election_obeyed_total` is ambiguous between
    /// "the arm is idle" and "the arm fails on every row," which is exactly the
    /// reading that cost 2026-08-01→03. Presence is asserted, never a value:
    /// `cargo test` runs this module's tests in parallel threads of ONE process
    /// against a process-global registry, so any sibling test may legitimately
    /// have incremented these counters first.
    #[test]
    fn election_obey_probe_outcomes_are_pretouched_at_boot() {
        register_all();

        let text = gather_text();
        assert!(
            text.contains("elohim_content_election_obey_probe_total"),
            "obey-probe counter missing:\n{text}"
        );
        for outcome in ["attempted", "no_election", "resolve_error", "no_courier"] {
            assert!(
                text.contains(&format!("outcome=\"{outcome}\"")),
                "obey-probe outcome {outcome:?} not pre-touched at registration — an absent \
                 series reads 'never measured', which is the ambiguity this metric exists to \
                 remove:\n{text}"
            );
        }
    }

    #[test]
    fn election_obey_probe_outcomes_increment() {
        use seam_contracts::ReasonLabel as _;
        register_all();

        // Every variant is reachable from a real branch — the C8 guard against a
        // structurally-constant label surviving because only one arm is wired.
        for outcome in ElectionObeyProbe::ALL {
            inc_election_obey_probe(*outcome);
        }

        let text = gather_text();
        assert!(text.contains("elohim_content_election_obey_probe_total"));
        assert!(text.contains("outcome=\"attempted\""), "{text}");
        assert!(text.contains("outcome=\"no_election\""), "{text}");
        assert!(text.contains("outcome=\"resolve_error\""), "{text}");
        assert!(text.contains("outcome=\"no_courier\""), "{text}");
    }

    // ── Q11: atomic convergence-path timing budget ───────────────────────────

    /// All 8 atoms must exist in the scrape from registration alone, same
    /// discipline as `TRUST_PRICED_ADOPTIONS` above: a run that never hits one
    /// atom (e.g. `shard_fetch` on a mesh with no >16MB blob) must read as a
    /// MEASURED zero, not an absent series — the distinction the budget table's
    /// `count × cost` terms depend on (zero cost from zero occurrences is a
    /// real reading; a missing series is an unasked question). Iterating
    /// `ConvergenceAtom::ALL` (rather than a literal list) means a 9th atom
    /// cannot be added to the enum without also being pre-touched here.
    #[test]
    fn atom_duration_all_eight_atoms_are_pretouched_at_boot() {
        use seam_contracts::ReasonLabel as _;
        register_all();

        let text = gather_text();
        assert!(
            text.contains("elohim_atom_duration_ms_bucket"),
            "Q11 atom-duration histogram missing:\n{text}"
        );
        for atom in ConvergenceAtom::ALL {
            assert!(
                text.contains(&format!("atom=\"{}\"", atom.label())),
                "Q11 atom {:?} not pre-touched at registration — an absent series is \
                 indistinguishable from an atom nobody ever wired up:\n{text}",
                atom.label()
            );
        }
        // Exactly 10 — the roster stays CLOSED, not open-ended. Was 8 until
        // 2026-08-20, when `inventory_serve` was added: the highest-rate loop in
        // the system had no atom, so a read-pool saturation at 1387% util had to
        // be found by log archaeology instead of read off this histogram.
        // Bumping this number is the deliberate act that admits a 9th atom.
        assert_eq!(ConvergenceAtom::ALL.len(), 10);
    }

    /// Both recording paths land in the SAME series: a `Duration` measured at
    /// a call site, and a derived millisecond share (`head_batch_per_id`).
    #[test]
    fn observe_atom_duration_and_ms_share_the_same_series() {
        register_all();
        observe_atom_duration(
            ConvergenceAtom::PutRecord,
            std::time::Duration::from_millis(7),
        );
        observe_atom_duration_ms(ConvergenceAtom::HeadBatchPerId, 3.5);

        let text = gather_text();
        assert!(text.contains("atom=\"put_record\""), "{text}");
        assert!(text.contains("atom=\"head_batch_per_id\""), "{text}");
    }

    /// The digest-fold smoke test: `digest_of_entry_lines` (pure, expected
    /// sub-millisecond) must actually record a sample when invoked — the
    /// atom this sprint expects to stay near-zero, proven rather than assumed.
    #[test]
    fn digest_fold_atom_records_a_timing_sample() {
        use seam_contracts::ReasonLabel as _;
        register_all();

        let before = digest_fold_sample_count(&gather_text());
        let _ = crate::p2p::reconcile_rails::digest_of_entry_lines(vec![
            "a=1".to_string(),
            "b=2".to_string(),
        ]);
        let after = digest_fold_sample_count(&gather_text());

        assert!(
            after > before,
            "digest_of_entry_lines must record a {:?} sample: before={before} after={after}",
            ConvergenceAtom::DigestFold.label()
        );
    }

    /// Parse the `elohim_atom_duration_ms_count{atom="digest_fold"}` sample
    /// count out of a `/metrics` scrape. Tests run in parallel threads of ONE
    /// process against a process-global registry, so this reads the counter
    /// rather than asserting an absolute value.
    fn digest_fold_sample_count(text: &str) -> u64 {
        text.lines()
            .find(|l| {
                l.starts_with("elohim_atom_duration_ms_count") && l.contains("atom=\"digest_fold\"")
            })
            .and_then(|l| l.rsplit(' ').next())
            .and_then(|v| v.parse::<f64>().ok())
            .map(|v| v as u64)
            .unwrap_or(0)
    }

    #[test]
    fn acquisition_reconcile_outcomes_are_stable_pretouched_and_incrementable() {
        use seam_contracts::ReasonLabel as _;

        let labels: Vec<_> = AcquisitionReconcileOutcome::ALL
            .iter()
            .map(|outcome| outcome.label())
            .collect();
        assert_eq!(
            labels,
            [
                "completed",
                "sync_paused",
                "db_pool_missing",
                "db_pool_unavailable",
                "pin_load_failed",
                "presence_query_failed",
            ]
        );

        register_all();
        for outcome in AcquisitionReconcileOutcome::ALL {
            inc_acquisition_reconcile_outcome(*outcome);
        }
        set_acquisition_reconcile_completed(0);

        let text = gather_text();
        assert!(
            text.contains("elohim_acquisition_reconcile_outcomes_total"),
            "{text}"
        );
        for label in labels {
            assert!(
                text.contains(&format!("outcome=\"{label}\"")),
                "reconcile outcome {label:?} was not materialized:\n{text}"
            );
        }
        assert!(
            text.contains("elohim_acquisition_reconcile_initialized 1"),
            "{text}"
        );
        assert!(
            text.contains("elohim_acquisition_active_pins 0"),
            "zero active pins must be an observable completed census:\n{text}"
        );
    }

    #[test]
    fn head_batch_series_register_and_increment() {
        register_all();
        observe_head_batch_call(
            "resolve_content_heads_local",
            "answered",
            64,
            std::time::Duration::from_millis(1_500),
        );
        observe_head_batch_call(
            "resolve_canonical_elections",
            "call_failed",
            0,
            std::time::Duration::ZERO,
        );
        add_head_batch_unattempted("resolve_content_heads_local", "budget_exhausted", 12);
        set_head_batch_size("resolve_content_heads_local", 64);
        inc_head_batch_fallback("unknown_function");

        let text = gather_text();
        assert!(text.contains("elohim_head_batch_calls_total"), "{text}");
        assert!(text.contains("outcome=\"answered\""), "{text}");
        assert!(text.contains("outcome=\"call_failed\""), "{text}");
        assert!(text.contains("elohim_head_batch_ids_total"), "{text}");
        assert!(
            text.contains("elohim_head_batch_unattempted_total"),
            "{text}"
        );
        assert!(text.contains("stop_reason=\"budget_exhausted\""), "{text}");
        // The histogram is THE probe that decides whether the conductor-fork
        // call-deadline patch stays warranted — a missing bucket series would
        // make that question unanswerable.
        assert!(
            text.contains("elohim_head_batch_queue_wait_ms_bucket"),
            "{text}"
        );
        assert!(text.contains("elohim_head_batch_size"), "{text}");
        assert!(text.contains("elohim_head_batch_fallback_total"), "{text}");
        assert!(text.contains("why=\"unknown_function\""), "{text}");
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

    /// A1 (2026-08-09, R1): the measured-gate pattern every `run_heal` arm
    /// follows (`set_projection_reconcile_measured` fires UNCONDITIONALLY;
    /// `set_projection_reconcile_gauges` is gated on that same bit), pinned at
    /// the metrics layer so the honesty floor is provable without spinning up
    /// a sweep. An unmeasured sweep must publish ONLY `measured=0` and leave
    /// every value gauge (here: `divergent`, standing in for the whole
    /// gauge set `set_projection_reconcile_gauges` writes together) exactly as
    /// the last REAL sample left it — publishing a false zero over it would be
    /// the N2 false-green this gate exists to prevent. A measured sweep writes
    /// both.
    #[test]
    fn an_unmeasured_sweep_leaves_the_divergent_gauge_untouched_but_a_measured_sweep_writes_both() {
        let stream = "test-a1-measured-gate";

        // Establish a non-zero baseline via a MEASURED sweep, so "untouched" on
        // the next (unmeasured) sweep is actually checkable rather than
        // vacuously true against a default-zero gauge.
        set_projection_reconcile_measured(stream, true);
        set_projection_reconcile_gauges(stream, 7, 3, 1, 5, 2);
        assert_eq!(
            PROJECTION_RECONCILE_MEASURED
                .with_label_values(&[stream])
                .get(),
            1
        );
        assert_eq!(
            PROJECTION_RECONCILE_DIVERGENT
                .with_label_values(&[stream])
                .get(),
            5
        );

        // UNMEASURED: only the bit moves.
        set_projection_reconcile_measured(stream, false);
        assert_eq!(
            PROJECTION_RECONCILE_MEASURED
                .with_label_values(&[stream])
                .get(),
            0,
            "measured must flip to 0 on an unmeasured sweep"
        );
        assert_eq!(
            PROJECTION_RECONCILE_DIVERGENT
                .with_label_values(&[stream])
                .get(),
            5,
            "the divergent gauge must be UNTOUCHED by an unmeasured sweep — the prior real \
             sample stands, never overwritten by a false zero"
        );

        // MEASURED: both the bit and the value gauges move.
        set_projection_reconcile_measured(stream, true);
        set_projection_reconcile_gauges(stream, 0, 3, 0, 0, 0);
        assert_eq!(
            PROJECTION_RECONCILE_MEASURED
                .with_label_values(&[stream])
                .get(),
            1
        );
        assert_eq!(
            PROJECTION_RECONCILE_DIVERGENT
                .with_label_values(&[stream])
                .get(),
            0,
            "a measured sweep must overwrite the value gauges with the fresh sample"
        );
    }

    /// B4 (2026-08-09, R1): a `to` label OUTSIDE the known reach vocabulary
    /// must land on the `"other"` bucket, never mint its own label series — the
    /// cardinality footgun a peer running a future build (or a bug) could
    /// otherwise trip on a wire-derived value.
    #[test]
    fn reach_narrowed_clamps_an_unknown_to_label_to_other() {
        let before_other = PROJECTION_RECONCILE_REACH_NARROWED
            .with_label_values(&["public", "other"])
            .get();
        let before_raw = PROJECTION_RECONCILE_REACH_NARROWED
            .with_label_values(&["public", "some-future-reach-tier"])
            .get();

        inc_projection_reconcile_reach_narrowed("public", "some-future-reach-tier");

        assert_eq!(
            PROJECTION_RECONCILE_REACH_NARROWED
                .with_label_values(&["public", "other"])
                .get(),
            before_other + 1,
            "an unrecognised wire reach must be counted under the bounded \"other\" bucket"
        );
        assert_eq!(
            PROJECTION_RECONCILE_REACH_NARROWED
                .with_label_values(&["public", "some-future-reach-tier"])
                .get(),
            before_raw,
            "the raw wire-derived string must never mint its own label series"
        );
    }

    /// Every value in the known reach vocabulary must pass through UNCLAMPED —
    /// the clamp targets cardinality risk from wire-derived novelty, not the
    /// protocol's own vocabulary.
    #[test]
    fn reach_narrowed_passes_known_reach_values_through_unclamped() {
        let before = PROJECTION_RECONCILE_REACH_NARROWED
            .with_label_values(&["public", "commons"])
            .get();

        inc_projection_reconcile_reach_narrowed("public", "commons");

        assert_eq!(
            PROJECTION_RECONCILE_REACH_NARROWED
                .with_label_values(&["public", "commons"])
                .get(),
            before + 1,
            "a known reach value is a real label, not a clamp target"
        );
    }

    // ── HTTP dispatch wrapper (elohim_http_request_duration_ms) ──────────────

    const HTTP_ROUTE_CLASS_SET: [&str; 9] = [
        "content", "blob", "apps", "sync", "api", "admin", "health", "p2p", "other",
    ];

    /// Representative paths pulled from the live `http.rs` match block (as of
    /// the addition of this instrument) must land on the class an operator
    /// would expect, and every classification — representative or not — must
    /// stay inside the 9-value closed set.
    #[test]
    fn classify_http_route_maps_representative_paths_and_stays_bounded() {
        let cases: &[(&str, &str)] = &[
            ("/blob/sha256-abc", "blob"),
            ("/shard/sha256-abc", "blob"),
            ("/manifest/sha256-abc", "blob"),
            ("/manifest", "blob"),
            ("/dag/bafyreiabc/links", "blob"),
            ("/ipfs/bafkreiabc", "blob"),
            ("/apps/lamad", "apps"),
            ("/chrome/omnibar.js", "apps"),
            ("/spa/index.html", "apps"),
            ("/sync/v1/round", "sync"),
            ("/admin/write-through", "admin"),
            ("/admin", "admin"),
            ("/api/v1/pins", "api"),
            ("/api/v1/events", "api"),
            ("/p2p/status", "p2p"),
            ("/p2p/peers", "p2p"),
            ("/db/content/foo", "content"),
            ("/epr-head/abc-123", "content"),
            ("/import/progress", "content"),
            ("/health", "health"),
            ("/health/serving", "health"),
            ("/version", "health"),
            ("/metrics", "health"),
            ("/", "other"),
            ("/session/exchange", "other"),
            ("/auth/me", "other"),
            ("/_capability", "other"),
        ];
        for (path, expected) in cases {
            let got = classify_http_route(path);
            assert_eq!(
                got, *expected,
                "path {path} classified as {got}, expected {expected}"
            );
            assert!(
                HTTP_ROUTE_CLASS_SET.contains(&got),
                "classify_http_route({path}) returned {got}, outside the 9-value closed set"
            );
        }
    }

    /// No matter what a client sends — an unknown route, a path-traversal
    /// attempt, an id-bearing path a naive classifier might echo back — the
    /// classifier must return one of exactly 9 fixed strings. Never the raw
    /// path: that would make this label's cardinality unbounded and turn the
    /// instrument itself into an outage vector.
    #[test]
    fn classify_http_route_never_yields_an_unbounded_label() {
        let hostile_paths = [
            "",
            "/",
            "/blob/",
            "/blob/../../../../../../etc/shadow",
            "/api/v1/content/00000000-0000-0000-0000-000000000000",
            "/db/content/some-very-long-random-id-that-should-never-become-a-label-9f8e7d6c",
            "/nonexistent/route/that/does/not/exist/anywhere",
            "/%00%00",
            "/blob/deadbeef?../../escape",
        ];
        for path in hostile_paths {
            let got = classify_http_route(path);
            assert!(
                HTTP_ROUTE_CLASS_SET.contains(&got),
                "path {path:?} produced label {got:?}, outside the closed 9-value set"
            );
            assert_ne!(
                got, path,
                "the classifier must never echo the raw path back as a label"
            );
        }
    }

    #[test]
    fn status_class_buckets_into_the_closed_three_value_vocabulary() {
        for status in [
            0u16, 100, 200, 201, 204, 299, 300, 302, 400, 404, 429, 499, 500, 503, 599, 65535,
        ] {
            let class = status_class(status);
            assert!(
                class == "2xx" || class == "4xx" || class == "5xx",
                "status {status} classified as {class}, outside the closed 2xx|4xx|5xx vocabulary"
            );
        }
        assert_eq!(status_class(200), "2xx");
        assert_eq!(status_class(404), "4xx");
        assert_eq!(status_class(503), "5xx");
        assert_eq!(
            status_class(599),
            "5xx",
            "the guard's unreported-outcome sentinel must land on 5xx, never silently pass as success"
        );
    }

    /// The whole point of the guard: a request dropped WITHOUT ever calling
    /// `finish()` — the shape of a cancelled/timed-out request, where the
    /// client walks away and hyper drops the future mid-handler — must still
    /// land in the histogram and release the in-flight gauge. This is the
    /// coordinated-omission gap the doorway-breaker incident hit: storage
    /// exported nothing for the stalled request because nothing observed it
    /// before it was gone.
    #[test]
    fn request_metrics_guard_observes_a_dropped_unfinished_request() {
        let route_class = "other";
        let path = "/__test_request_metrics_guard_dropped__";
        assert_eq!(classify_http_route(path), route_class);

        let before_in_flight = HTTP_REQUESTS_IN_FLIGHT.get();
        let before_count = HTTP_REQUEST_DURATION_MS
            .with_label_values(&[route_class, "5xx"])
            .get_sample_count();

        {
            let guard = RequestMetricsGuard::start(path);
            assert_eq!(
                HTTP_REQUESTS_IN_FLIGHT.get(),
                before_in_flight + 1,
                "starting a guard must bump in-flight immediately"
            );
            drop(guard); // never called .finish() — simulates cancellation
        }

        assert_eq!(
            HTTP_REQUESTS_IN_FLIGHT.get(),
            before_in_flight,
            "Drop must release the in-flight permit even without finish()"
        );
        assert_eq!(
            HTTP_REQUEST_DURATION_MS
                .with_label_values(&[route_class, "5xx"])
                .get_sample_count(),
            before_count + 1,
            "an unfinished/cancelled request must still be observed exactly once, under the 5xx sentinel"
        );
    }

    /// An explicit error status reported via `finish()` must land under its
    /// own `status_class`, and an errored request must be observed just like
    /// a successful one — never dropped from the distribution because it
    /// failed.
    #[test]
    fn request_metrics_guard_observes_an_explicit_error_finish() {
        let route_class = "admin";
        let path = "/admin/some-op";
        assert_eq!(classify_http_route(path), route_class);

        let before_count = HTTP_REQUEST_DURATION_MS
            .with_label_values(&[route_class, "5xx"])
            .get_sample_count();
        let before_in_flight = HTTP_REQUESTS_IN_FLIGHT.get();

        let guard = RequestMetricsGuard::start(path);
        guard.finish(503);

        assert_eq!(
            HTTP_REQUEST_DURATION_MS
                .with_label_values(&[route_class, "5xx"])
                .get_sample_count(),
            before_count + 1,
            "an explicit error status must be observed under its own status_class"
        );
        assert_eq!(
            HTTP_REQUESTS_IN_FLIGHT.get(),
            before_in_flight,
            "finish() must release the in-flight permit via Drop, same as any other exit"
        );
    }

    /// A successful request must be observed under 2xx, distinguishing it
    /// from the error path in the same histogram.
    #[test]
    fn request_metrics_guard_observes_a_success_finish() {
        let route_class = "blob";
        let path = "/blob/sha256-abcdef";
        assert_eq!(classify_http_route(path), route_class);

        let before_count = HTTP_REQUEST_DURATION_MS
            .with_label_values(&[route_class, "2xx"])
            .get_sample_count();

        let guard = RequestMetricsGuard::start(path);
        guard.finish(200);

        assert_eq!(
            HTTP_REQUEST_DURATION_MS
                .with_label_values(&[route_class, "2xx"])
                .get_sample_count(),
            before_count + 1,
            "a successful request must be observed under 2xx"
        );
    }
}
