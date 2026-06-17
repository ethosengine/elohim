---
title: Design-Decision Toolkit — durable instruments so design calls are evidenced, not scrambled
id: design-decision-toolkit-plan
status: Draft
cites:
  - .claude/data/matthew-edge-resiliency-rca-fanout-2026-06-15.md
  - genesis/data/timeline/backlog/conductor-memory-attribution-verdict.md
  - conductor-memory-attribution-instrument-plan | Conductor Memory Attribution Instrument | sha256:9b86bc94e115c866 | path: genesis/docs/superpowers/plans/2026-06-16-conductor-memory-attribution-instrument-plan.md
  - genesis/docs/superpowers/plans/2026-06-14-dataplane-diagnostic-plan.md
domain: dataplane / observability
sprint: design-decision-toolkit
# Mixed plan. P0/P1/P3 unit-testable + observable on household-nodes/observability.
# P2 doorway/heap legs and the live-graph proof want alpha — tagged inline.
---

# Design-Decision Toolkit (composition-ordered)

> For agentic workers: REQUIRED SUB-SKILL: superpowers:test-driven-development + subagent-driven-development. Checkbox (- [ ]) tasks.

## Why
This session proved the pattern: an in-codebase instrument (the memory-attribution sampler) turned a RAM-bump *guess* into an evidence-based *verdict* (anon-heap leak, not cache, arc-independent). It also exposed the fragility we're fixing: **elohim-storage has NO durable app-metrics surface** — every runtime signal is a `tracing` log line grepped from Loki, and **Loki 502-stormed mid-decision today**, so the verdict had to come from cadvisor by luck. The goal: durable instruments so design decisions are *evidenced and don't get re-litigated*.

## Composition order (the registry is the single integration seam)
```
P0 Foundation (metrics registry + /metrics + scrape)   ← everything registers into this
   └─> P1 Promote existing signals (sampler gauges, violation counter)   ← cheap, proves the surface
   └─> P2 New instruments (fan-out, all emit into P0)
   │      ├─ heap-leak attribution (jemalloc; allocator-level, P0-independent)
   │      └─ RCA instrument suite (doorway watchdog + storage saturation/db_read_timeout/corpus↔RSS)
   └─> P3 Decision-record discipline (records what P1/P2 measure)
```
**Order rationale:** P0's `Registry` is the one API every instrument depends on — build it first with a clean `register_all()` + `gather_text()`. P1 lifts already-existing signals (logs/atomics) onto it (immediate value, validates the API). P2 fans out (independent instruments) once the seam exists. P3 is last (it records measured effects) but its template can be authored anytime. Heap-profiling is allocator-level so it's P0-independent, but its *output* belongs in P3.

## Compose, don't reinvent
- **`prometheus = "0.13"`** is already in the workspace (`elohim-bitswap/src/stats.rs` + `behaviour.rs::register_metrics`): `lazy_static!` metric statics → `registry.register(Box::new(M.clone()))` → `TextEncoder`. Mirror that idiom exactly in elohim-storage.
- **Prometheus Operator is deployed** (`kube-prom-stack`) but **no PodMonitor exists** → the scrape config is a net-new in-repo manifest (pipeline-reconciled; never `kubectl`).
- `/p2p/status` (`http.rs:932`, `P2PStatusInfo` in `p2p/mod.rs`) is the existing JSON introspection surface — P-DIAGNOSTIC's pattern. Metrics are the *historical/alertable* twin of that point-in-time JSON.

---

## P0 — FOUNDATION: durable metrics surface  (build first; this turn)

**Files:** `elohim/elohim-storage/Cargo.toml` (+`prometheus = "0.13"`), `src/metrics.rs` (new), `src/lib.rs` (+`pub mod metrics`), `src/http.rs` (`/metrics` route), `genesis/orchestrator/manifests/` (PodMonitor).

- [ ] `metrics.rs`: process-global `Registry` (OnceLock) + `lazy_static!` statics + `pub fn register_all()` (idempotent, called once at boot) + `pub fn gather_text() -> String` (TextEncoder over the registry). Unit-test: `register_all()` twice is ok; `gather_text()` contains a registered metric name.
- [ ] `(Method::GET, "/metrics") => handle_metrics()` in `http.rs` → `text/plain; version=0.0.4` body from `gather_text()`. (No auth — same posture as `/p2p/status`; scrape is in-cluster.)
- [ ] Call `metrics::register_all()` once at storage boot (`main.rs`, near the sampler spawn).
- [ ] **PodMonitor** manifest (`genesis/orchestrator/manifests/.../elohim-node-podmonitor.yaml`) selecting the elohim-node pods, port/path `/metrics`. Pipeline-reconciled. (Operator confirms the Prometheus Operator's `podMonitorSelector` admits it — flag in hand-off.)

## P1 — PROMOTE existing signals onto the foundation  (this turn; proves the surface)

**Files:** `src/metrics.rs` (declare the metrics), `src/main.rs` (sampler sets gauges), `src/identity_namespace.rs` (violation counter).

- [ ] Memory-attribution gauges: `elohim_node_proc_rss_bytes{proc,kind="anon"|"file"}`, `elohim_node_proc_threads{proc}`, `elohim_node_cgroup_mem_bytes{kind="anon"|"file"|"slab"}`, `elohim_node_cgroup_swap_bytes`. The sampler (main.rs) sets them each tick *in addition to* the Loki log line — so the leak verdict becomes a durable graph (fixes today's Loki-died-mid-decision). Boot gauge: `elohim_node_db_max_readers`, `elohim_node_cpu_quota_cores`.
- [ ] Identity-namespace violation: add `elohim_identity_namespace_violation_total{column,expected,got}` IntCounterVec; `observe_agent_cid_write` increments it alongside the existing AtomicU64 + WARN. (Keeps the log line; adds the scrapeable counter — which is what I went looking for and found missing earlier today.)
- [ ] Verify `gather_text()` now carries these; add a smoke test.

## P2 — NEW INSTRUMENTS (fan-out after P0; next phase)

**Heap-leak attribution** (allocator-level; the immediate next step for the live verdict) · @requires:observability for the live proof
- [ ] `tikv-jemallocator` as `#[global_allocator]` (feature-gated `heap-profiling`, default off to avoid changing prod allocator without intent) + jemalloc prof; a guarded `/debug/heap-profile` dump (admin-only) or a SIGUSR-triggered dump. Narrows the confirmed anon leak from a *process* (P1 sampler) to a *call site*.

**RCA instrument suite** (the RCA §4/§5 named instruments; each registers into P0)
- [ ] doorway: `doorway_watchdog_wedged_total`, `doorway_conductor_reconnect{reason}`, `doorway_conductor_session_duration_seconds` histogram (targets in `doorway/.../main.rs`/`routes/health.rs`/`orchestrator/heartbeat.rs`; RCA §4.3). @requires:observability for the live doorway-flap proof.
- [ ] storage: `hc_db_read_timeout_total` (alert on the existing DatabaseError::Timeout line), DHT-read saturation gauge, `conductor_corpus_docs` × `conductor_rss_peak_bytes`, `dht_authority_set_entry_count` (RCA §4.1/§4.5/§4.6).

## P3 — DECISION-RECORD DISCIPLINE (records what P1/P2 measure; ties it together)

- [ ] A decision-record template + convention: `tunable → measured-effect → verdict` (the RCA §3 "tune→document→report" + shape dims). Home: `genesis/data/timeline/backlog/` (the verdict docs are the first instances: arc-falsified, heap-leak). Optional small script to scaffold a record from a metric query + a one-line verdict.
- [ ] Backfill the two decisions already made as records: `arc-shrink-ineffective-memory-soak.md` (tunable=`target_arc_factor`, effect=none, verdict=falsified) and `conductor-memory-attribution-verdict.md` (tunable=RAM/cache, effect=anon-dominant, verdict=heap-leak).

---

## Build/test (elohim-storage = WASM-flagged; verified idiom)
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/cargo-elohim-storage RUSTC_WRAPPER="" cargo test --lib metrics 2>&1 | tail -40
# whole-lib + clippy --lib + fmt --check as the gate (NOT --all-targets: pre-existing reds, see verdict/handoff)
```

## Dispatch / seams
- **Foundation (P0) built by one hand** (single registry API); P2 fans out (Workflow candidate) once P0 lands.
- **Commit-only on the shift branch; operator pushes** the deploy (the live-graph proof). P0/P1 are unit-gated locally.
- **Seam with P-DIAGNOSTIC** (`P2PStatusInfo`): metrics are the historical twin of `/p2p/status`; the memory-attribution gauges could also feed `P2PStatusInfo` later (the P-DIAGNOSTIC anchor pattern). Don't fork — additive.
- **PodMonitor is operator-reconciled** (repo manifest, not `kubectl`); flag the `podMonitorSelector` confirm.
