# HANDOFF — Doorway-service own /metrics surface (design-decision toolkit P2 remainder)

**Date:** 2026-06-17
**Branch:** commit-only on the shift branch — the operator pushes (never `git push` here).
**Predecessor:** storage P0/P1 landed on `dev` @ `1907249a4` (durable Prometheus `/metrics` on elohim-storage). This handoff is the **P2 doorway leg** — doorway-service's OWN `/metrics`.
**Plan:** `genesis/docs/superpowers/plans/2026-06-17-design-decision-toolkit-plan.md` (P2 line 64).
**RCA:** `genesis/docs/content/elohim-protocol/history/2026-06-15-matthew-edge-resiliency-rca-fanout-synthesis.md` (§4.3 item 3 = line 65 is the instrument spec; §3 tunables; §4.3 = the live-flap proof).
**Verdict the toolkit serves:** `genesis/data/timeline/backlog/conductor-memory-attribution-verdict.md` — the alpha conductor OOM is an **anonymous-heap leak in the holochain conductor child**, NOT doorway (doorway working set is 75–94 MB; its restart is a **liveness-watchdog self-kill**, a *different* failure wearing the same alarm).

---

## 0. The one-paragraph why

Storage P0/P1 proved the pattern: an in-codebase instrument turned a RAM guess into an anon-heap-leak *verdict* — and it exposed the fragility (the only signal home was `tracing`→Loki, and Loki 502-stormed mid-decision). The matthew "flap" is **two loosely-coupled failures**: (A) the conductor OOM (storage's metrics already attribute it), and (B) the **doorway restart**, which is doorway's own liveness watchdog (`:8079`, stale>15s ⇒ wedged) firing because doorway's main tokio runtime parks while co-located on `ethosengine` next to two memory-climbing conductors. The A/B control (doorway-alpha-b on `shem`) reconnect-storms *just as hard* with **0 restarts** — co-location, not config, is the differentiator. **Doorway's metrics must explain (B): why the watchdog wedged, why the conductor sessions churn, and how much load doorway sheds — none of which storage can see, because doorway sees the conductor only over the wire, never its memory.**

---

## 1. DISCOVER FIRST — current state (verified in-tree 2026-06-17)

| Question | Answer (file:line) |
|---|---|
| Prometheus/metrics dep today? | **NONE.** `doorway/doorway-service/Cargo.toml` has neither `prometheus` nor `lazy_static`. Both must be added. |
| A `/metrics` route today? | **NONE.** `server/http.rs` dispatch has `/version` (2392), `/status` (2395), health probes — no `/metrics`. The only thing named "metrics" is `routes/admin.rs` `ClusterMetrics`/`handle_cluster_metrics` (462) — **peer social/resource aggregation, not a Prometheus surface.** Honest current state: *no /metrics, no prometheus dep.* |
| Watchdog / self-kill path | `server/http.rs:978–1086`. `HEALTH_STALE_MS_DEFAULT = 15_000` (988), env `DOORWAY_HEALTH_STALE_MS` (991). `main_runtime_wedged(age, threshold)` (1000) is the pure verdict. `watchdog_liveness_response` (1028) builds the **503 wedge branch** at 1035–1040. The watchdog runs on its **own OS-thread runtime** (`spawn_health_listener` 1139) bound to `DOORWAY_HEALTH_PORT` (1144) = **8079** (`health-wd` container port). `handle_watchdog_probe` (1063) serves ONLY `/health`,`/ready`,`/health/startup` — **everything else 404s.** |
| Conductor reconnect + WS close-frame + session-duration | `worker/conductor.rs`. Backoff consts: `BASE=100ms` (42), `MAX=30s` (45), `STABLE_SESSION_THRESHOLD=10s` (50). `connection_loop` (206) drives reconnect; `run_session` (294) is the convergence site — `session_start` (306), `session_end = handle_messages(...)` (308), **`session_len` computed at 311 and THROWN AWAY**. Close frame logged at `worker/conductor.rs:532` (`Ok(Message::Close(frame)) => info!("Conductor closed connection: {:?}", frame)`); WS error at 536. `SessionEnd` enum (84) carries only `ChannelClosed`/`ConnectionClosed` — **no reason/close-code.** |
| Subscriber close path (the bug to fix) | `projection/subscriber.rs:635` — `Some(Ok(Message::Close(_))) => info!("Conductor closed connection")` **DROPS the frame** (`_`). The worker path keeps it; the subscriber path discards it. RCA §4.3 calls this out: "the **subscriber** path drops the frame — fix it." |
| Proxy + projection cache (hit/miss + 503/Retry-After) | `routes/storage_proxy.rs`: `forward_to_storage` (130); circuit-open shed → 503+Retry-After (222–233, logs `counter = "doorway_upstream_breaker_open_total"`); upstream 429/503 honored (243–261, logs `counter = "doorway_upstream_backpressure_honored_total"`); `catching_up_proxy_response` (71). Blob pantry: `forward_blob_to_storage` — pantry **hit** (361 `"Blob drawn from pantry (cache hit)"`), **miss** (400), **stock-on-200** (465–475). Tier resolver: `cache/resolution.rs` `DoorwayResolver::resolve_with_identity` (197) — `ResolutionStats` already tallies `projection_hits` (71), conductor_fallbacks (73), external_fallbacks (74) at 342–344, with a `cached: bool` flag on `ResolutionResult<T>` (62) and "Projection hit" (254). |
| Admission shed (503) | `server/http.rs:2264` `admission_exempt(&path, is_upgrade)`; shed at 2267–2283 logs `counter = "doorway_admission_shed_total"` (2275) → `catching_up_response` (2280). `admission_exempt` body at **3881**: `/health /healthz /health/startup /ready /readyz /version` (+ any upgrade). |
| Per-conductor session fan-out (`DOORWAY_PER_CONDUCTOR_WORKERS`) | **NOT yet an env.** The fan-out is a **hardcoded `worker_count: 2`** at `main.rs:418` ("Per-conductor pools are smaller than the main pool"), created once per conductor in `CONDUCTOR_URLS`. RCA proposes wiring `DOORWAY_PER_CONDUCTOR_WORKERS` here (§4.3 tunables, line 50). Total app-ws sessions ≈ `app(main WORKER_COUNT=4) + admin + 2·len(CONDUCTOR_URLS) + 1(subscriber)`. The main pool `WORKER_COUNT` env is `config.rs:86` (default 4). |
| Doorway pod labels (for the PodMonitor) | `genesis/orchestrator/manifests/doorway/alpha.yaml`: Deployment `elohim-doorway-alpha` (49), pod labels `app: elohim-doorway` (69), `environment: alpha` (70), `app.kubernetes.io/name: doorway` (71), `app.kubernetes.io/component: gateway` (74). Container ports: **`gateway-ws` = 8080** (the request listener, 113), **`health-wd` = 8079** (watchdog, 118). |

---

## 2. THE THIN FOUNDATION (mirror storage; add nothing heavier)

Doorway is the **porch** — a gateway (bootstrap + signal + conductor proxy/cache). Stay thin: mirror `elohim/elohim-storage/src/metrics.rs` exactly — `lazy_static!` statics + one process-wide `Registry::new()` + `TextEncoder` for the body — and **nothing more**. No new runtime, no sampler thread (see §5), no aggregation.

### 2a. Deps — `doorway/doorway-service/Cargo.toml`
```toml
prometheus = "0.13"   # same as storage; do NOT pull 0.14+
lazy_static = "1"     # doorway has NEITHER today — add both
```

### 2b. New module `src/metrics.rs` (mirror storage's idiom)
```rust
use lazy_static::lazy_static;
use prometheus::{Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec,
                 IntGauge, Opts, Registry, TextEncoder};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    // … the COMPLEMENT set from §3, registered into REGISTRY …
}

/// text/plain; version=0.0.4 exposition body. Compute scrape-time-derived
/// gauges (heartbeat_age) HERE from the watchdog atomics — no extra task.
pub fn gather_text() -> String { /* TextEncoder over REGISTRY.gather() */ }
```
`src/lib.rs` (or `main.rs` module tree): `pub mod metrics;`. Use typed setters/inc helpers so call sites never `use prometheus` (storage convention).

### 2c. The route — MAIN listener, admission-exempt (BLOCKING, get this right)
- Add `(Method::GET, "/metrics") => to_boxed(routes::metrics(...))` to the **main** dispatch in `server/http.rs` **next to `/version` (2392)** — `text/plain; version=0.0.4`, no auth (in-cluster scrape, same posture as storage's `/metrics` and `/p2p/status`).
- **Add `"/metrics"` to `admission_exempt` (3881).** A scrape endpoint that gets 503'd under admission pressure is dead exactly when you need it. (The wisdom gate at 2252 is a no-op for a GET — only admission needs the exemption.)
- **It must NOT go on the watchdog runtime (8079).** `handle_watchdog_probe` (1063) 404s everything but the three probes; the watchdog runtime is intentionally minimal. `/metrics` lives on **8080 / `gateway-ws`**, exactly like storage's `/metrics` lives on its main 8090 listener — not a side port.
- **Consequence (a feature, not a gap):** during a *real* main-runtime wedge, `/metrics` on 8080 cannot answer → the Prometheus target goes `up == 0`. **That `up == 0` IS the wedge-time signal** (alert on it). A main-listener `heartbeat_age` gauge can never capture a fatal wedge — don't pretend it does. (v1 mirrors storage. Future-only option, explicitly out of scope: a tiny lock-free `/metrics` arm added to the watchdog runtime would make `heartbeat_age` readable mid-wedge.)

### 2d. PodMonitor — `genesis/orchestrator/manifests/infra/alpha-doorway-podmonitor.yaml`
Clone `infra/alpha-edgenode-podmonitor.yaml` (same dir → same pipeline reconcile) including its **operator-confirm header** (the cluster is operator-owned; repo is the surface, never `kubectl`). Differences:
```yaml
metadata:
  name: elohim-doorway
  namespace: elohim-alpha
  labels:
    app: elohim-doorway
    release: kube-prom-stack        # kube-prometheus-stack default selector
spec:
  namespaceSelector: { matchNames: [elohim-alpha] }
  selector:
    matchLabels:
      app: elohim-doorway           # NOT elohim-edgenode
  podMetricsEndpoints:
    - port: gateway-ws              # containerPort 8080 — the MAIN listener (/metrics lives here, NOT health-wd/8079)
      path: /metrics
      interval: 30s
      scrapeTimeout: 10s
      honorLabels: false
```
Carry the same three operator-confirm flags from the storage manifest (podMonitorSelector admits `release: kube-prom-stack`; namespaceSelector admits `elohim-alpha`; a pipeline applies `infra/*.yaml`).

---

## 3. THE COMPLEMENT SET — ONLY what storage CANNOT see

Cardinal rule: **complement, never duplicate.** Doorway sees the conductor only over the wire. Every metric below is a *doorway-local* signal storage has no access to. Each cites its exact code site and the RCA §4.3 reasoning.

### M1 — `doorway_watchdog_wedged_total` (counter) + `heartbeat_age` (gauge) — *the 503-flap proximate trigger*
- **Site:** `server/http.rs:1035–1040` (the 503 branch in `watchdog_liveness_response`). Increment the counter when `wedged == true`. Keep/add `warn!(heartbeat_age_ms, threshold_ms, "watchdog WEDGED")` at the same branch (RCA §4.3) so the fatal case still leaves a **Loki trace** even when `/metrics` can't answer.
- **heartbeat_age:** compute **at scrape time in `gather_text()`** from the same `Arc<AtomicU64>` + `Instant` the watchdog already reads (`spawn_liveness_heartbeat` 1009; `watchdog_liveness_response` 1033–1034). **No new task.** Pass the atomic+start handle into the metrics module at boot.
- **Why un-gettable from storage:** the watchdog is doorway's own liveness; storage has no idea doorway's runtime parked. RCA Theory 3: "WEDGED precedes each restart ⇒ confirms the watchdog-self-kill path." This is the proximate cause of (B).

### M2 — `doorway_conductor_reconnect_total{reason}` (counter-vec) + WS close code — *classifies T3 vs T8*
- **Sites (one coordinated change, NOT a pure add):** `SessionEnd` (`worker/conductor.rs:84`) carries only Channel/Connection — **enrich it to carry `reason` + optional close code**. The Close-vs-Err distinction lives inside `handle_messages` (Close at 532, Err at 536); the connect-refused case is the `Err(e)` arm of `connect_to_conductor` in `connection_loop` (270). Increment the counter at the reconnect decision in `run_session` / `connection_loop`.
- **Labels (REVISED from the RCA's three to avoid axis-conflation):** the RCA's `tcp_refused|accept_then_drop|close_frame` mixes *why it closed* with *how long it lived*. Use **`reason ∈ {connect_refused, close_frame, ws_error, channel_closed}`** (the close *cause*) and let `accept_then_drop` **fall out of the duration histogram** (M3: a session < `STABLE_SESSION_THRESHOLD` = 10s is the accept-then-drop / auth-reject signature). Non-overlapping axes.
- **Fix the subscriber drop as part of THIS change:** `projection/subscriber.rs:635` `Message::Close(_)` → capture the frame and emit the same `{reason="close_frame", code}` so both conductor-wire paths report. (RCA §4.3 explicitly: "the subscriber path drops the frame — fix it.")
- **Why un-gettable from storage:** these are doorway↔conductor *transport sessions*. Storage doesn't observe doorway's WS lifecycle.

### M3 — `doorway_conductor_session_duration_seconds` (Histogram) — *the auth-reject vs idle-reap discriminator*
- **Site:** `worker/conductor.rs:311` — `session_len` is **already computed and thrown away.** Observe it. `run_session` (294–325) is the convergence site for M2+M3 + the session-count gauge.
- **Buckets must straddle sub-second (auth-reject) AND 10s (idle/stable):** `0.05, 0.1, 0.5, 1, 5, 10, 30, 60, 300`. A pile in the <1s bucket = auth-reject churn (T8); a pile at the long end = healthy long-lived sessions reaped for other reasons.
- **NOTE — this is the FIRST Histogram in the toolkit tree** (storage P0/P1 used only gauges + counters). It's still thin; just flag it so the implementer pulls `Histogram`/`HistogramOpts` from `prometheus`.
- **Why un-gettable from storage:** session lifetime is a doorway-side fact.

### M4 — proxy / cache hit-miss + 503/Retry-After rate — *how much doorway sheds, and from where*
- **Cache tier outcomes:** `cache/resolution.rs:342–344` — `ResolutionStats` already counts `projection_hits` / conductor_fallbacks / external_fallbacks. Mirror onto a counter-vec `doorway_resolve_total{tier="projection|conductor|external"}` at that tally site (`cached: bool` on `ResolutionResult<T>` at 62 distinguishes hit vs fallthrough).
- **Blob pantry:** `routes/storage_proxy.rs` `forward_blob_to_storage` — pantry **hit** (361), **miss** (400), **stock** (465–475). Counter `doorway_blob_pantry_total{outcome="hit|miss|stocked|skipped"}` (the `second_request_served_from_pantry` / `blob_200_stocks_pantry` tests mark these branches).
- **503/backpressure:** the tracing `counter=` fields **already exist** — reuse the exact names so logs and metrics share vocabulary: `doorway_upstream_breaker_open_total` (storage_proxy.rs:224), `doorway_upstream_backpressure_honored_total` (252), `doorway_admission_shed_total` (http.rs:2275). Register a real counter per name; `inc()` inline at those existing sites.
- **Why un-gettable from storage:** the projection cache, the blob pantry, the per-upstream breaker, and inbound admission are **doorway-resident operational state** (the "doorway-local Operational state" the trust model explicitly permits — `doorway/CLAUDE.md`). Storage never sees a doorway cache hit (the request never reaches it) or a doorway shed (it's refused before forwarding).

### M5 — `doorway_conductor_sessions` (gauge) — *the fan-out the RCA wants bounded*
- **Site:** `run_session` (`worker/conductor.rs`) — inc at 304 (`*connected = true`), dec at 310 (`*connected = false`). Total live app-ws sessions per doorway.
- **Why un-gettable from storage:** this is doorway's session multiplication (`main.rs:418` hardcoded `worker_count: 2` × `len(CONDUCTOR_URLS)` + app + admin + subscriber). RCA §4.3 names the shape dim `doorwayConductorSessionsTotal`; this gauge IS it. (If you wire `DOORWAY_PER_CONDUCTOR_WORKERS` at 418 while here, that's the RCA's diagnostic lever — but it's a falsified *differentiator*, so it's optional, not the point of this handoff.)

---

## 4. DO NOT ADD / DO NOT DUPLICATE

**Storage already exposes these — re-exposing any of them on doorway is the cardinal sin.** Doorway sees the conductor ONLY over the wire; it cannot measure the conductor's memory, threads, or corpus. If you find yourself reaching for a memory/corpus/thread number, you are about to duplicate storage.

| Do NOT add to doorway | Already owned by | Why doorway can't/shouldn't |
|---|---|---|
| `elohim_node_proc_rss_bytes`, `elohim_node_cgroup_mem_bytes{anon\|file\|slab}`, `elohim_node_cgroup_swap_bytes` | **storage** (P1) | The conductor's heap is in a process doorway never introspects. Doorway is a separate pod with a tiny (75–94 MB) footprint — measuring its own RSS adds nothing the verdict needs. |
| `elohim_node_conductor_smaps_anon_bytes`, `_anon_mapping_count`, `_largest_anon_bytes` | **storage** (P1) | smaps of the conductor child — storage is the co-located sidecar that can read `/proc`; doorway cannot. |
| `elohim_node_proc_threads`, `elohim_node_db_max_readers`, `elohim_node_cpu_quota_millicores`, `elohim_node_corpus_docs` | **storage** (P1) | conductor/DHT internals + corpus shape — all conductor-side. |
| `elohim_identity_namespace_violation_total` | **storage** | a storage write-path validation; doorway only proxies. |
| **A doorway restart counter** | **kube-state-metrics** | `kube_pod_container_status_restarts_total` already owns the *restart fact*. Doorway's unique value is the heartbeat_age / reconnect-reason / session-duration that explain **why** it restarted — not the count, which k8s has. |
| **A wedge-time `heartbeat_age` on the watchdog port** | (no one — but it's a trap) | During a fatal wedge the main listener can't answer; the wedge-time signal is the scrape target's **`up == 0`** (k8s/Prometheus), plus the `warn!("watchdog WEDGED")` Loki line. Don't build a side-port gauge for v1. |

**Naming discipline:** doorway metrics are prefixed `doorway_*` (matching the existing tracing `counter=` fields), NOT `elohim_node_*` (storage's per-node prefix). The prefixes themselves keep the two surfaces from ever colliding in one Prometheus.

---

## 5. THIN WINS — assert these so nobody over-builds

1. **No 60s sampler (unlike storage's `main.rs:640`).** Doorway's signals are **event-driven**: increment counters inline at the sites in §3; observe the histogram where `session_len` is already computed; derive `heartbeat_age` at scrape time in `gather_text()` from the watchdog atomic. No background task, no `tokio::spawn`. (Storage needed a sampler because cgroup/smaps gauges are *poll*-shaped; doorway's are *event*-shaped.)
2. **Reuse the existing tracing `counter=` names verbatim** (`doorway_upstream_breaker_open_total`, `doorway_upstream_backpressure_honored_total`, `doorway_admission_shed_total`) so a Loki log line and a Prometheus series carry the same identifier — operators pivot between them without a translation table.
3. **No new routes beyond `/metrics`.** This is a porch. The §3 metrics ride existing code paths.

---

## 6. SEAM WITH P-DIAGNOSTIC (`routes/self_healing.rs`) — complement, co-deliver, don't fork

`GET /admin/self-healing` (`routes/self_healing.rs`) already composes a `SelfHealingView` — the **point-in-time JSON** twin of these metrics (the historical/alertable surface). They are the same data at two cadences; **do not fork the read model.**

The crucial co-delivery: `self_healing.rs` has two fields **blocked on accessors that the §3 metrics need anyway** (its own FOLLOW-ON notes, lines 37–45):
- `AdmissionView` (40, 53–60) needs `inbound_semaphore` **max** + a **shed atomic** — exactly what M4's `doorway_admission_shed_total` counter needs.
- `UpstreamView` (45, 62–72) needs an `UpstreamBreakers` **snapshot accessor** — exactly what M4's breaker counters surface.

**Land those two accessors once → light up BOTH the `/metrics` series AND the currently-null `self_healing` fields.** That is "complement, don't fork" as a concrete co-delivery, not a slogan. The metrics module reads the same atomics/snapshots the view composer reads; neither owns the other.

(Storage's analog: P-DIAGNOSTIC's `/p2p/status` is the JSON twin of storage's `/metrics`; the plan §"Seam with P-DIAGNOSTIC" makes the same additive-not-fork point. Doorway inherits the pattern.)

---

## 7. BUILD / TEST / DISCIPLINE

**Doorway is NATIVE Rust — override RUSTFLAGS** (the system sets the WASM `getrandom` flag, which breaks native link):
```bash
RUSTFLAGS="" cargo build --release
RUSTFLAGS="" cargo test --lib --bins                     # unit tests (331+); add metrics tests here
RUSTFLAGS="" cargo test --lib --bins metrics             # the new module's tests only
RUSTFLAGS="" cargo clippy -- -D warnings
cargo fmt --check
```
Container quirks (per memory): `RUSTC_WRAPPER=""`, prefer `/tmp` target dirs, **plain cargo** (no nextest here), gate with `--lib`/`--bins` NOT `--all-targets` (pre-existing reds elsewhere in the tree). The `.husky/pre-push` `sweettest-check` does not touch this native crate.

**Suggested unit tests** (pure, no runtime — mirror storage's metric tests):
- `gather_text()` returns non-empty `text/plain` containing each registered metric name.
- `watchdog_wedged_total` increments only when `main_runtime_wedged(age, threshold)` is true (reuse the existing `watchdog_tests` at http.rs:1089).
- `SessionEnd` reason labeling: a Close-frame session yields `reason="close_frame"`; a sub-`STABLE_SESSION_THRESHOLD` session lands in the histogram's <1s buckets.
- `admission_exempt("/metrics", false) == true` (extend the existing exempt test at 4639).
- session gauge inc/dec balance across a session.

**Discipline:** commit-only on the shift branch with the `Co-Authored-By: Claude Opus 4.8` trailer; **the operator pushes/merges.** The cluster is operator-owned — the PodMonitor in `genesis/orchestrator/manifests/infra/` is the *repo* surface; the pipeline reconciles it. Never `kubectl`. Until the operator confirms the three PodMonitor preconditions, `/metrics` is still reachable by port-forward/manual scrape — the manifest only wires the *automatic* scrape.

---

## 8. Sequencing for the implementer

1. **Foundation first** (§2): deps → `metrics.rs` (`Registry` + `gather_text` + helpers) → `/metrics` route on the main listener → add to `admission_exempt`. Verify `curl :8080/metrics` returns an (empty-ish) exposition body. This is the seam everything else registers into.
2. **M1 + M4-reuse** (cheapest, highest-signal): watchdog counter at the 503 branch + scrape-time `heartbeat_age`; wire the three already-named `counter=` tracing sites to real counters. Immediate flap visibility.
3. **M2 + M3 + subscriber fix** as ONE unit (they all converge on `SessionEnd`/`run_session`): enrich `SessionEnd`, label the reconnect counter, observe the session-duration histogram, fix `subscriber.rs:635`.
4. **M4 cache tiers + M5 session gauge**: resolver/pantry counters + the live-sessions gauge.
5. **PodMonitor** (§2d): clone storage's, flip selector/port/name, carry the operator-confirm header.
6. **P-DIAGNOSTIC co-delivery** (§6): land the `inbound_semaphore`-max+shed-atomic and `UpstreamBreakers`-snapshot accessors once; they feed both `/metrics` and the null `self_healing` fields.

The live-flap proof (RCA §4.3, "@requires:observability") runs on alpha after this lands — out of scope for the build, named here so the next session knows the verification target.
