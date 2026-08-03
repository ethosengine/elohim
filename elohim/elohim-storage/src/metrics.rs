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
        let _ = REGISTRY.register(Box::new(CONTENT_ADOPT_SWEEP.clone()));
        {
            use seam_contracts::ReasonLabel as _;
            for outcome in AdoptSweepOutcome::ALL {
                CONTENT_ADOPT_SWEEP
                    .with_label_values(&[outcome.label()])
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
        let _ = REGISTRY.register(Box::new(IDENTITY_KEY_SUPERSEDE.clone()));
        let _ = REGISTRY.register(Box::new(SHARD_PUSH_PEER_UNRESOLVED.clone()));
        let _ = REGISTRY.register(Box::new(PROJECTION_HEAL_OUTCOMES.clone()));
        // Pre-touch every (stream, outcome) combination — 3 streams x 10
        // `p2p::projection_reconcile::HealOutcomeKind` outcomes = 30 series —
        // so an outcome that has literally never fired for a stream still
        // reads as a measured zero, not an absent series. Labels below are the
        // same vocabulary already used at the `inc_projection_heal_outcome`
        // call sites in `p2p/projection_reconcile.rs`.
        for stream in ["rea", "content", "collectives"] {
            for outcome in [
                "healed",
                "timeout_retried",
                "timeout_exhausted",
                "missing",
                "failed",
                "refused_declared",
                "refused_stale",
                "no_row",
                "refreshed",
                "deferred_to_adopt",
            ] {
                PROJECTION_HEAL_OUTCOMES
                    .with_label_values(&[stream, outcome])
                    .inc_by(0);
            }
        }
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

/// Count a canonical-head declaration LINK minted through this node, by the
/// channel that caused it (`"adopt_peer"`, `"contest_peer_head"`,
/// `"contest_self_head"`, [`MINTED_SOURCE_ADOPT_BEFORE_AUTHOR`], `"http"`).
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
        ContestSkip::EvidenceAbsentBackoff,
    ];

    fn label(&self) -> &'static str {
        match self {
            ContestSkip::NoLocalChainBackoff => "no_local_chain_backoff",
            ContestSkip::SelfCandidacyBackoff => "self_candidacy_backoff",
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
