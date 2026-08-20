---
title: The Latency Valueflow Chain — per-round-trip measurement, composition spikes, two-tier validation
id: latency-valueflow-chain
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: first cut landed (doorway hop histograms + PodMonitor selector fix + scoreboard reader) AND a printed local-vs-deployed delta for at least one hop
created: 2026-08-20
topic: [latency, dataplane, measurement, composition-spike, peer-diversity, reactive-controllers, two-tier-validation, observability, habits]
cites:
  - genesis/docs/superpowers/plans/2026-08-20-three-lane-dataplane-performance-contract.md
  - genesis/docs/superpowers/plans/2026-08-16-minutes-quiesce-fixture-trust-swarm-plan.md
  - genesis/manifests/habits.yaml
  - elohim/elohim-storage/src/metrics.rs
  - doorway/doorway-service/src/metrics.rs
  - genesis/orchestrator/manifests/infra/alpha-doorway-podmonitor.yaml
  - genesis/data/timeline/backlog/alpha-a-projector-chronic-catchup-flap.md
---

<!-- Authored 2026-08-20 from an operator directive: "measure timing integration on every
possible round trip in the chain, optimizing on each p2p dataplane, and seeing the spikes
when we compose the layers"; constrained by four further directives — optimize in micro with
re-runnable tests, scale under load, design for peer diversity with reactive dynamic scaling
that does not destabilize, and prove/validate LOCALLY at small scale before proving in the
pipeline at deployed scale. Grounded by four parallel readers over the hop inventory, the
existing instrumentation, the epr-flow valueflow machinery and the declared budgets; every
load-bearing citation re-verified against disk in section 0. -->

# THE LATENCY VALUEFLOW CHAIN — design for approval

## 0. VERIFICATION OF THE FOUR READERS

I checked every load-bearing citation against disk. Most survived. Five did not, and two of the failures change the design.

**Survived, verified verbatim.** The `ConvergenceAtom` closed vocabulary (8 members, `elohim/elohim-storage/src/metrics.rs:2193-2239`), its single-choke-point recorders (`:2247`, `:2257`), its `HistogramVec` (`:1643-1653`), its boot-time pre-touch (`:2120-2128`) and the completeness test that iterates `ALL` rather than a literal list (`:4318-4339`) — this is real, and it is the best instrument in the tree. 14 call sites across 7 files (measured). `put_record` genuinely measures a **local enqueue, not a round trip** — the SAFETY comment at `elohim/elohim-storage/src/p2p/mod.rs:4296-4300` says so explicitly and the timer at `:4305-4313` brackets only the synchronous call. `doorway/doorway-service/src/routes/storage_proxy.rs` contains **zero** `Instant::now()` (measured: 0). `doorway/doorway-service/src/cache/resolution.rs:203-218` times the whole resolve and folds it into a lifetime cumulative mean at `:363-365`; `get_stats()` is called from exactly one place outside its own definition — `#[test] fn test_stats_initial` at `:451-456`. Doorway has exactly **one** histogram (`CONDUCTOR_SESSION_DURATION_SECONDS`, `doorway/doorway-service/src/metrics.rs:113`) and it times session *lifetime*. `warm_shell.rs` has zero `Instant::now()`. Neither crate depends on `tower-http`. `sweet_consistency.rs:82` is a fixed `sleep(500ms)`. The three-lane lane table and SLOs are verbatim (`genesis/docs/superpowers/plans/2026-08-20-three-lane-dataplane-performance-contract.md:129-140`), including its own admission that T4's SLO is "50× the one real observation that exists anywhere in the tree" and its self-correction that T1's micro-legs *are* timed. `AdaptiveBatchBudget` is exactly as described: AIMD, floor 8 / ceiling 128 / threshold 2s / +16 / ×0.5, `next_size` a pure total function, starts at floor fail-closed (`elohim/elohim-storage/src/p2p/reconcile_rails.rs:289-420`). The bench suite is 10 files with `bench-all` at `elohim/elohim-storage/justfile:92`, and it is **not CI-wired** (grep across all five Jenkinsfiles, `scripts/ci/`, `genesis/orchestrator/`: zero hits). `habits.yaml` reads 7 green / 5 red / 0 unwired and already conventionalizes a `best_observed:` ratchet at lines 55-60. `MESH_RATE_TARGET` default 344 (`scripts/ci/run-mesh-quiesce-stage.sh:93`).

**Reader 3's strongest claim is the most thoroughly confirmed of all.** `.eprfs/status/flows.jsonl` is 5,425 records. Every `Magnitude::Count` on disk carries `"value":1.0` — 619 occurrences, **no other value has ever been minted**, across four units (`artifact` 478, `run-note` 57, `red-run` 55, `green-run` 29). The numeric slot exists (`elohim/epr/src/witness.rs:141-152`) and has never once held a measurement.

**Did not survive — and it matters.**

1. **Reader 1's kitsune2 gossip citations are against code that is not in the binary, and worse than he flagged.** `kitsune2_gossip` resolves to `registry+crates.io` v**0.4.1** (`elohim/holochain-conductor/Cargo.lock:4718-4720`); every `path = "../kitsune2/..."` patch is commented out (`Cargo.toml:89-98`). The conductor's own Cargo.toml comment settles it: *"elohim/kitsune2 is pinned at v0.3.2, which predates the transport_iroh crate entirely"* (`Cargo.toml:126-128`). So `elohim/kitsune2/crates/gossip/src/gossip.rs:336` is not a place instrumentation can be added — it is a museum copy. Hop W12 is not merely uninstrumented; it is **un-instrumentable in place**. The design must route around it, and does (§2, spike level 4).

2. **Reader 4 quoted a stale controller signal from the plan doc, and the code has repudiated it.** The plan says AIMD feeds on `queue_wait = RTT − in-wasm elapsed_ms`. `AdaptiveBatchBudget::observe`'s doc explicitly refuses that: *"The caller supplies the ADMISSION wait… not `RTT − in-wasm elapsed_ms`. The latter is not monotonic in batch size — it subtracts exactly the interval a stalled extern spends waiting on a conductor read permit — so feeding it here made the loop grow the batch precisely when the conductor was struggling"* (`reconcile_rails.rs:~404-412`). The controller's input is `elohim_conductor_admission_wait_ms` (`metrics.rs:1689-1699`), **not** `elohim_head_batch_queue_wait_ms` (`metrics.rs:1589-1600`). This is a controller that has already resonated once, for exactly the reason §10 is asking about, and the plan document still carries the version that caused it. Any convergence analysis reading the wrong series will misdiagnose.

3. **Readers 1 and 2 disagreed on where `adopt_declare` is recorded; both were partially wrong.** There are three sites: `projection_reconcile.rs:2460`, `:4969`, and `services/reanchor_backfill.rs:302`. Immaterial to the design (the label vocabulary is the contract), but it means neither reader enumerated the call sites.

4. **Reader 4's "no PodMonitor" implication and reader 1's silence on scrape coverage both missed the cheapest finding in the tree.** `genesis/orchestrator/manifests/infra/alpha-doorway-podmonitor.yaml` selects `matchLabels: {app: elohim-doorway}`. Doorway-B's pods are labelled `app: elohim-doorway-b` (`genesis/orchestrator/manifests/doorway/alpha-b.yaml:85,90`). **Doorway B is not scraped.** Confirmed on disk. Half of lane C is dark for a five-line manifest reason.

5. **Reader 1's "H7 put_record is mislabeled" is right but understated.** It is not a taxonomy nit — it is a live falsity in an existing series that a dashboard reader would take as "cost of DHT publish." It is a *renaming* item in the first cut, not a footnote.

---

## 1. THE CHAIN

The dominant misrouting risk is treating this as one linear chain. It is **two chains that meet at exactly one seam**, and conflating them is precisely how the fast lane ends up gated on the slow lane.

**Chain R — the read journey.** Synchronous, browser-initiated, lane C/T4 dominant, budget in *milliseconds*. This is the fast lane. Nothing in it may ever block on Chain W.

**Chain W — the converge journey.** Asynchronous, publish-initiated, lanes A+B, budget in *seconds to minutes*. Allowed to lag, per contract.

**Seam S1 — the freshness delta.** The one hop that legitimately spans both: time from a fact becoming true on peer P to the first Chain-R read on doorway D that returns it. It is **reported, never gated**.

### Containment tree

```
CHAIN R  (fast lane — ms budgets, gates)
R0  browser_rtt          client fetch → response       storage-client-ts/src/client.ts:84
 └─ R1  doorway_serve    handle_request → written      doorway/.../server/http.rs:4324
     │                    (R0 − R1 = network + client; measured only from a2o/look)
     ├─ R2  resolve_tiered  DoorwayResolver             doorway/.../cache/resolution.rs:203-218
     │   ├─ R2a  tier=projection    local, ~0 network
     │   └─ R2b  tier=conductor  ─┐  CONTAINS R4
     ├─ R3  proxy_storage     ────┼─ builder.send()     doorway/.../routes/storage_proxy.rs:327
     │   └─ R3s storage_serve     │  storage handler    elohim/elohim-storage/src/http.rs
     │       └─ R3z storage_zome  │  call_zome          elohim/.../conductor/session.rs:173-217
     │           └─ R5 ───────────┤
     ├─ R3b proxy_blob            │  pantry-HIT short-circuits: NO child
     │                            │                     storage_proxy.rs:464-480, :537
     ├─ R6  ssr_render            │  may CONTAIN R3     doorway/.../render/warm_shell.rs:305,442
     └─ R4  doorway_zome_call ────┘  call_on_ws         doorway/.../services/zome_caller.rs:934-943
         └─ R5  conductor_exec       in-wasm elapsed_ms — SELF-REPORTED IN-BAND, no fork edit

R2, R3, R6 are ALTERNATIVES selected per route, not siblings that all run.
R2 sits ABOVE R3/R4 — for cached routes the tiered resolve is the parent and the
proxy hop is one of its two branches. The naive "browser→doorway→storage" line is wrong.

CHAIN W  (slow lane — s/min budgets, amber only)
W0  publish_to_present   author commits → peer can serve
 ├─ W1  kad_enqueue      LOCAL, NOT an RTT (mislabelled today)  p2p/mod.rs:4305-4313
 ├─ W1b kad_confirm      NEW — the real Kademlia RTT, by QueryId  [uninstrumented]
 ├─ W2  inventory_page   view-federation                projection_reconcile.rs (8 sites)
 ├─ W3  list_content     NEW — 1000-item page           p2p/mod.rs:8009 send → :4805 recv
 ├─ W4  get_content      NEW — per-gap fetch            p2p/mod.rs:8081
 ├─ W5  shard_fetch      existing atom                  p2p/blob_swarm.rs:184-196
 └─ W6  manifest_persist existing atom (local disk)     db/shard_manifests.rs:97-111

W7  head_notarize_lag    declared → notarized + visible
 ├─ W8  head_batch_per_id   derived                     metrics.rs:1626-1631
 │   ├─ W8a admission_wait  existing histogram          metrics.rs:1689-1699
 │   └─ W8b admission_hold  existing histogram          metrics.rs:1702-1712
 ├─ W9  head_record_verify  existing atom               p2p/head_record_client.rs:112-119
 ├─ W10 adopt_declare       existing atom, COMPOSITE — contains W9 (sometimes skipped)
 │                          plus an unmeasured local declare write
 ├─ W11 digest_fold         existing atom (pure compute)
 └─ W12 gossip_round        ★ BLACK BOX — crates.io 0.4.1, un-instrumentable in place
     └─ W12a relay_connect  timed TODAY as a log field only
                            patches/kitsune2_transport_iroh/src/lib.rs:849, :876

SEAM
S1  freshness_delta      W-completion → first R that sees it.  Measured by ONE prober's
                         clock (write and read both from the prober) — never by
                         differencing two peers' clocks.
```

Three structural claims this tree makes and the design depends on: R2 is a **parent** of R3/R4, not a peer; W10 is the **only composite already measured today**, so `W10 − W9` is a spike computable this afternoon with zero new code; and W12 is architecturally decoupled from W7's children because `create_entry` returns locally and the publish sweep runs on its own timer (`publish_dht_ops_workflow.rs:38`) — which is the code-level reason lane B is *allowed* to lag, not a concession.

---

## 2. THE COMPOSITION SPIKE — the exact arithmetic

**Percentiles do not sum.** `p95(parent) ≠ Σ p95(children)`. Two arithmetics are defined and must never be mixed.

**Spike-T (per-trace residual)** — where a request identity exists (all of Chain R). `residual_i = parent_i − Σ children_i` computed per request, then the **distribution** of residuals is reported. Exact, and the primary measure for the fast lane.

**Spike-B (budget residual)** — where there is no request identity (all of Chain W; sweeps, not requests). `Unexplained(P) = E[P] − Σ_j (count_j × E[child_j] ÷ parallelism_j)`. This is the Q11 formula (`genesis/docs/superpowers/plans/2026-08-16-minutes-quiesce-fixture-trust-swarm-plan.md:201`) already committed to in a plan and never built. **It is valid on means only, never on percentiles.** Counts come from each histogram's `_count`, sweep counts from `elohim_projection_reconcile_sweeps_total` (`metrics.rs:1063-1067`), parallelism from `HEAD_BATCH_FANOUT = 2` (`projection_reconcile.rs:492-500`).

| # | Residual | Numbers come from | A spike here means |
|---|---|---|---|
| 1 | `R1 − (branch that ran: R2 ∣ R3 ∣ R6)` | R1 new timer at `http.rs:4324`; R2 existing timer at `resolution.rs:215` (today thrown into a dead mean); R3 new timer at `storage_proxy.rs:327` | Doorway-internal overhead — the per-request `authenticated` splice (three-lane §5 item 4), header work, admission gating. Flat children + fat parent ⇒ **none of the ranked couplings is the cure; the doorway is.** |
| 2 | `R3 − R3s` | R3s self-reported by storage in a response header | Wire + connect + TLS + LB. First-ever comparison against the chosen `STORAGE_PROXY_CONNECT_TIMEOUT_SECS = 3` (`storage_proxy.rs:44`), a deadline nobody has measured. |
| 3 | `R3z − R5` | R5 already returned in-band by the batch extern (`metrics.rs:1585-1587`) | Admission/queue. **Free consistency test**: this must equal `W8a + W8b`'s complement. If the two disagree, one instrument is lying. |
| 4 | **`W7 − (W8 + W9 + W10 + W11)`** | Spike-B, all four already live series | ★ **This residual IS the gossip session plus publish-workflow delay.** It surfaces the black box as a number *without instrumenting kitsune2 at all*. The single highest-value line in this design. |
| 5 | `W10 − W9`, conditioned on whether verify ran | Both live atoms today, zero new code (SKIP-POINT 2, `projection_reconcile.rs:~4963`) | Cost of the local head-declare write leg. **Computable this afternoon.** |
| 6 | `W0 − (W1b + W3 + W4 + W5 + W6)` | Spike-B | Replication *scheduling*: the unconditional 60s tick and the 1000-item page (`p2p/mod.rs:175`). Confirms or refutes ranked couplings #1/#2 with a number instead of an estimate. |
| 7 | `S1 − max(W0, W7)` | Prober vs the two chain parents | The projection/signal path (`doorway/.../projection/subscriber.rs:591-623`, unmeasured). Large ⇒ bytes present, head present, **doorway just hasn't noticed** — a cache-invalidation bug wearing a dataplane costume. This is the misrouting disambiguator. |

Admissibility rule, enforced by the scoreboard: a residual is printed only if parent and children were measured over the **same window, same `peer_class`, same n-basis**. Otherwise the row prints `INCOMMENSURABLE` — the vocabulary the repo already owns (`elohim/epr/src/measure.rs:160-162`).

---

## 3. THE INSTRUMENT

**Histograms everywhere. Zero tracing spans. Cross-process residuals via in-band self-reported elapsed, never timestamp differencing.**

Neither service uses `tracing` spans today (0 `#[instrument]`, 0 `span!` in both trees) and neither depends on `tower-http`. Adding a span/context-propagation stack to get parent-child residuals would be the largest new machinery in this design for the smallest gain. Instead, generalize a pattern the repo already ships: **the batch extern already returns its own in-wasm `elapsed_ms` in the response payload** (`metrics.rs:1585-1587`), and `HEAD_BATCH_QUEUE_WAIT_MS` is computed by subtracting it outside the conductor. That is a self-reported-elapsed ladder, and it is already load-bearing. Extend it:

- Storage stamps `x-elohim-elapsed-ms` on its HTTP responses; doorway subtracts to get residual #2. No trace context, no propagation library, no clock comparison.
- Doorway stamps `x-elohim-hop-serve-ms` (joining the existing `x-ssr-*` header convention) so the a2o prober computes residual #1 client-side.
- **Only ever subtract self-reported durations, never timestamps.** Container clocks here skew by hours; a timestamp-differencing residual is poison by construction.

**Vocabulary.** Extend `ConvergenceAtom` in elohim-storage with `KadConfirm`, `ListContentPage`, `GetContent`, `StorageServe`, `StorageZome`. Create a structurally identical `DoorwayHop` enum in `doorway/doorway-service/src/metrics.rs` — same `ReasonLabel` impl, same `ALL`, same boot pre-touch, same completeness test mirroring `metrics.rs:4318-4339`. This is not a second instrument; Prometheus registries are per-process, so it is *the same instrument in the second process*, which is mandatory.

**Labels (closed sets only, no exceptions):** `hop`, `peer_class`, `route_class`, `outcome`, `env`. Never `content_id`, `peer_id`, or a route path.

**Resolution, per hop, and why.** The rule is **instrument resolution ≤ 1/10 of the hop's budget**, because the sweettest failure is what happens at 1/9: a 500 ms poll against a ~4.5 s phenomenon is 11 % quantization, which is exactly what turned five clean runs into "7 vs 9 ticks."

| Hop class | Budget | Required resolution | Bucket set |
|---|---|---|---|
| R1/R2/R3/R3b/R6 | 200 ms hit, 800 ms cold | ≤ 20 ms | 1, 2, 5, 10, 20, 50, 100, 200, 400, 800, 1600, 5000 ms |
| R3s/R3z/R4/R5 | ~500 ms | ≤ 50 ms | same |
| W1b/W2/W3/W4/W5/W6 | ≤ 60 s | ≤ 6 s | existing 0.1 ms–30 s set (`metrics.rs:1643-1653`) — adequate |
| W7/W0/S1 | p95 ≤ 30 min, p99 ≤ 4 h | ≤ 3 min | **NEW slow set**: 1, 5, 15, 60, 300, 900, 1800, 3600, 14400 s. The existing set tops out at 30 s, so W7 would land entirely in `+Inf` and be unreadable — this is a real blocker, not a preference. |
| W12/W12a (local micro) | ~4.5 s | ≤ 50 ms | — |

**Escaping the observer-dominates trap in sweettest without a polling change.** `await_op_integration` selects `DhtOp.when_integrated` out of the database (`sweet_consistency.rs:~85`). **The integration timestamp is already in the row.** Keep the 500 ms poll for *loop control* — it only decides when to stop — and compute the reported latency from `when_integrated` minus the publish instant. That removes the observer from the *measurement* entirely while leaving it in the *control*, at sub-millisecond resolution, and it costs one line in a test file rather than a change to sweettest's timing.

---

## 4. LITERAL OR NOT

**Not literal on the measurement plane. Literal on the commitment plane. No new register.**

Against epr-flow: `flow project` mints one node per file matched by a glob, dated by `git log` on that path (`elohim/eprfs/epr-cli/src/flow/project.rs:3-4`). A round trip has no file and no commit. Forcing it through would mean minting a JSON report and calling `flow fulfill` per trace, inverting the purpose of a governance ceremony into a telemetry sink. And the decisive measurement: across 5,425 sidecar records, **every single `Magnitude::Count.value` is `1.0`** — the numeric slot has never carried a measurement in this repo's history. The stage0b latency finding is *already in* `flows.jsonl` as free prose inside a `classified_as` tag with `quantity = 1.0, unit = "run-note"` — proving both that the channel is reached for and that it cannot hold the number. No fold, walk, or stock can read `0.233` back out of that record.

What *is* literal, and earns itself because it adds nothing: **each hop's budget is a `Commitment` carrying a `Bound { limit, unit, sense: Ceiling }`** — machinery that already exists and is already exercised (`elohim/epr-rea/src/model.rs:113-133`), satisfied by an `Intent` ("drive hop X under budget"), fulfilled by `epr flow fulfill` of the scoreboard's JSON, and folded by the existing `epr flow stocks --check` rate-vs-rate equilibrium. The budget table *is* the set of `Bound`s. Nothing new.

**New artifacts, total: one script and zero registers.** `.claude/scripts/latency-scoreboard.py` is a **reader**, not a register. The documented instrument-proliferation problem is about registers with no reader; this is a reader with no register. Raw numbers live in Prometheus (existing). Budgets live as `Bound` on Commitments (existing). Status lives in `habits.yaml` (existing). Local runs land in `elohim/elohim-storage/reports/latency/*.json` alongside the existing `genesis/a2o/reports/` convention. If anyone proposes `genesis/manifests/latency-budgets.yaml`, refuse it.

---

## 5. THE SCOREBOARD

`.claude/scripts/latency-scoreboard.py`, two sources, one output format.

`--deployed` runs PromQL (`histogram_quantile(0.95, sum by (le,hop,peer_class,route_class) (rate(doorway_hop_duration_ms_bucket[15m])))` and siblings) against Prometheus via the observability MCP. `--local` reads the harness JSON. Both emit the same row:

```
hop  class  env  n  p50  p90  p95  p99  max  MODALITY  budget  best  Δbest  Δlocal↔deployed  residual
```

Competitive-coding discipline: **`best_observed` is a ratchet.** `habits.yaml:55-60` already conventionalizes the field, citing Meadows' drift-to-low-performance trap and the repo's own documented eroding-goal channel (PVC-deferral making "green" mean "deferred, not passed"). The scoreboard **refuses to print green** for a run weaker than `best_observed` unless invoked with `--regression-accepted "<reason>"`, which is written into the evidence line. A later weaker green must read as visibly weaker, never as equivalent.

**Re-runnability is the anti-one-off rail:** every row prints, beside it, the exact command or PromQL and the window that produced it. Tomorrow's agent re-derives by copy-paste or the number is not admissible.

**Habits binding.** `habits.yaml` currently holds 12 (7 green + 5 red), and covenant rule 1 caps it at 12 — a candidate must displace one or wait. It waits. Latency files as a **new `checks:` line and evidence delta on the existing `dataplane-convergence` habit** (`genesis/manifests/habits.yaml:70`), because a bounded-latency convergence *is* that habit's invariant plus a bound. Per three-lane §7, the check must be a **conjunction**: speed AND a trust invariant, never speed alone — a fast stale head is worse than a slow correct one.

The a2o binding is `@concern:latency-budget`, added to the existing `genesis/a2o/features/delivery/transport-perf.feature` (which already carries pinned numeric baselines and a `@requires:bench-suite` tag), not a new feature file.

Evidence discipline, unchanged: a flip needs a build number (edge #NNNN Dataplane Validation, `elohim/holochain/Jenkinsfile:2203`), a live probe (a `--deployed` scoreboard run with its PromQL and timestamp), or a test run (`just bench-all`, `cargo test --test iroh_stage0`).

**Fast lane never gated on slow lane, concretely:** R and W are separate metric families with separate bucket sets; no R alarm may reference a W series; a W-budget miss prints **amber** and files as an evidence delta, never as a red CI gate; S1 is reported and never gated.

---

## 6. FIRST CUT — a real number tomorrow

**Doorway only. One process, no fork, no DNA, no submodule.** Lane C has literally zero request-duration series and the weakest-anchored SLO in the table (three-lane `:136-140`: "50× the one real observation that exists anywhere in the tree").

Files:

1. `doorway/doorway-service/src/metrics.rs` — add `DoorwayHop` enum + `DOORWAY_HOP_DURATION_MS: HistogramVec{hop, peer_class, route_class, outcome}` + registration + boot pre-touch + a completeness test mirroring `elohim/elohim-storage/src/metrics.rs:4318-4339`. Cache label-resolved handles at construction; do not do `with_label_values` per request.
2. `doorway/doorway-service/src/server/http.rs:4324` — R1 timer around `handle_request`; stamp `x-elohim-hop-serve-ms`.
3. `doorway/doorway-service/src/cache/resolution.rs:203-218, 363-365` — feed the already-computed `duration_ms` into the histogram labelled by tier; **delete `avg_resolution_ms`**. It is the tree's one measurement that looks quantitative and answers nothing, and leaving it is worse than never having built it.
4. `doorway/doorway-service/src/routes/storage_proxy.rs:327, 537` — R3/R3b timers around `builder.send().await`, labelled `pantry_hit|miss`.
5. `genesis/orchestrator/manifests/infra/alpha-doorway-podmonitor.yaml:47-49` — selector to `matchExpressions: [{key: app, operator: In, values: [elohim-doorway, elohim-doorway-b]}]`. **Five lines that double lane-C observability for zero code.**
6. `.claude/scripts/latency-scoreboard.py` — new reader.
7. `genesis/manifests/habits.yaml:70` — one-line delta on `dataplane-convergence`.

**What it tells us tomorrow:** the first honest doorway p95 in this repo's history, split cache-hit / cold / blob, against SLOs whose only anchor is a multiplication. Plus residual #1 — doorway's own overhead. Fat parent with flat children means none of the five ranked couplings is the cure and the doorway is; near-zero residual means the SLO must be renegotiated against storage.

**Free, same day, zero code:** residual #5 (`W10 − W9`, conditioned on the skip-point) is queryable *right now* from series already flowing.

**Second cut** (storage, no fork): W3/W4 instrumentation at `p2p/mod.rs:8009, 8081` + `W1 → kad_enqueue` rename + the slow bucket set + `x-elohim-elapsed-ms` on storage responses. **Third cut** (the one fork commit, batched): promote `relay_connect` from log field to histogram at `patches/kitsune2_transport_iroh/src/lib.rs:849-876`, and switch `iroh_stage0.rs` to `when_integrated`-derived deltas. One SHA, one image rebuild, done once.

---

## 7. STABILITY AS A MEASURAND

**Never store a summary. Store the bucket vector.** This is structural: a Prometheus histogram's `_bucket` series preserves the full distribution, so modality is recoverable post-hoc, forever, from data already scraped. A summary or a computed p50 destroys it irreversibly. Every hop reports `n, p50, p90, p95, p99, max`, **plus the bucket vector**, plus:

**Modality — the gap test.** Scan the bucket vector for the longest run of consecutive buckets holding < 1 % of n, bounded on both sides by buckets holding ≥ 10 % of n. If such a gap spans ≥ 2 bucket edges → `BIMODAL`. Printed as a first-class column: `BIMODAL 3.52s|4.52s (Δ1.00s, 40/60)`. Against the stage0b evidence — 3.516 / 3.518 / 4.520 / 4.521 / 4.523 — this fires cleanly on the empty tick at 4.0 s, which is exactly what a p50 of 4.520 erases.

**Oscillation.** Per-window p95 series: sign-change rate of Δp95 over 20 windows > 0.6 ⇒ oscillating; corroborate with lag-1 autocorrelation and a periodogram peak.

**Alarm conditions, per hop:** (i) p95 > class budget; (ii) `BIMODAL` for ≥ 3 consecutive windows; (iii) modal separation ratio > 1.5; (iv) sign-change rate > 0.6; (v) `max / p50 > 20` (tail blowout); (vi) **bucket-entropy collapse** — a distribution collapsing into one bucket is as suspicious as splitting into two, and usually means a timeout is clipping or the instrument saturated.

**Small-n rule, from the stage0b evidence directly:** for local micro runs with **n < 30, print every single run**, never only a summary. The bimodality was visible only because every run was printed. This is a design rule, not a nicety.

---

## 8. OBSERVER COST AT SCALE

**Budget: ≤ 0.5 % of each hop's own p50, and ≤ 1 % of process CPU for the whole instrument.**

Honest costs: `Instant::now()` ≈ 20–30 ns (vDSO); `observe()` ≈ 50–150 ns, dominated by **label lookup**, which is why handles are cached at construction rather than resolved per call (the existing `with_label_values(...).observe()` at `metrics.rs:2249` is fine at sweep frequency and would *not* be fine in doorway's request path). The real cost is **cardinality, not the timer**: `hop(≤20) × peer_class(≤6) × route_class(≤8) × outcome(≤4)` = ≤ 3,840 series × ~12 buckets ≈ 46 k samples per process. Acceptable. **Hard rule: no unbounded label**, enforced by the same closed-enum `ReasonLabel` discipline that already guards `ConvergenceAtom`.

**Histograms are always-on and never sampled** — they are O(1) and bounded, and sampling them would destroy the modality signal §7 depends on. Sampling applies *only* to the expensive things:

- **Exemplars**: reservoir, target ≤ 10 per hop per minute, rate adjusted by **the AIMD law already in `AdaptiveBatchBudget`** — reuse `next_size`'s pure function, do not write a second controller.
- **Per-trace residual ladder (Spike-T)**: head-based at 1 % of requests at the doorway edge (`x-elohim-trace: 1` turns on the elapsed-header ladder), plus **retro-triggered tail sampling** — any request exceeding its route budget forces the ladder on for the next N requests in that route class, so slow paths are over-sampled without a trace backend.

**How the overhead itself is measured**, because an unmeasured budget is a wish, two ways: (a) `doorway_hop_duration_ms{hop="instrument_self"}` — the instrument times a no-op observe once per 1,000 observations; (b) the definitive one — an **`--instrument=off` A/B in the local micro harness**: the same 200 iterations with the feature flag on and off, the delta *is* the overhead, printed as its own scoreboard row with its own ≤ 0.5 % budget. **If the A/B delta exceeds budget, the instrument fails its own gate and the cut is reverted.** Re-runnable on demand, same as every other row.

---

## 9. PEER DIVERSITY AS A DIMENSION

`peer_class` is a **required label on every measure**. A p50 across incommensurable classes is arithmetic on nonsense and hides the slow tier behind the fast one.

**How a peer declares its class: from observed capability, at startup — not from a manifest.** Today's ground truth is `nodeTypes` in `genesis/orchestrator/data/deployments.json` (measured: only two shapes in the whole file, `["remote"]` ×11 and `["operations","edge","performance"]` ×3) crossed with `provides_node_types` in `genesis/manifests/cluster-state.yaml:18,23`. That is a **k8s/compute vocabulary, and k8s is not the architecture** — it must not become the protocol's peer-class vocabulary. So:

`PeerClass::derive(caps) -> {watch, phone, laptop, household_node, rack, hosted_remote, unknown}` — a **pure total function** over (cores, total RAM, storage headroom, holds-full-arc, has-public-relay), testable without a cluster, deliberately the same shape as `AdaptiveBatchBudget::next_size` being pure and total (`reconcile_rails.rs:314-320`). Exported as a constant label on every series from that process, plus `elohim_peer_class{class} = 1` so it joins in PromQL.

**Budgets are per class.** R1 cache-hit p95: rack ≤ 120 ms, household-node ≤ 200 ms, laptop ≤ 300 ms, phone ≤ 600 ms, watch ≤ 1500 ms. W7 head lag: hosted-remote and household-node ≤ 30 min; **watch: hop not applicable** — a watch is not an authority, and a budget for a hop it never runs is a lie.

**A fleet-wide unlabelled aggregate is banned.** The scoreboard refuses to print one. Where a single number is demanded, it prints the **worst class**, never a mean.

**Unknown class**: `PeerClass::Unknown` is legal, is **never dropped**, gets the most permissive budget (never gates), and is **counted and alarmed** — `elohim_peer_class{class="unknown"} > 0` is itself a red, because an unknown class means the derivation function is stale relative to the fleet. This is the `unwired` discipline `habits.yaml:36-38` already names as "the most valuable state in this file": the chain does not break on an unknown class, it degrades to an honest, visible unknown.

---

## 10. REACTIVE CONTROLLERS — SETTLE OR OSCILLATE, MEASURED

Prior art read in full: `AdaptiveBatchBudget`, `elohim/elohim-storage/src/p2p/reconcile_rails.rs:289-420`. AIMD, floor 8 / ceiling 128 / threshold 2 s / +16 / ×0.5 (`:332-342`). `next_size` is a **pure total function — no clock, no state, no I/O** — "so the convergence properties are testable without a conductor" (`:314-317`). Starts at floor, fail-closed (`:355-358`). Two deliberately unmerged axes: `DispatchBudget` bounds concurrency, this bounds batch size (`:289-297`), because the write-guard constraint forbids raising either the per-tick cap or the concurrency — "only the per-round-trip YIELD may rise."

**It has already resonated once**, and the cure is in the code while the stale version is still in the plan (§0 finding 2): feeding `RTT − in-wasm elapsed_ms` "made the loop grow the batch precisely when the conductor was struggling." A second, sibling defect is documented at `observe_batch_outcome`: a call admitted instantly but refused by the coordinator's in-wasm deadline fed *headroom* into the controller, so "the controller GREW on the strongest evidence it should shrink."

**Measured convergence, not assumed.** Output signal: `elohim_head_batch_size{extern_name}` (`metrics.rs:1603-1607`, set at `:2175`). Input signal: `elohim_conductor_admission_wait_ms` (`metrics.rs:1689-1699`) — **not** `elohim_head_batch_queue_wait_ms`. Third signal: the ratio of decrease-by-refusal (`unattempted > 0`) to decrease-by-wait.

Window: sample the gauge at 15 s; analyse over 30 samples (~7.5 min) for control dynamics and over 4 h for slow drift.

**SETTLED** ⇔ coefficient of variation < 0.15 **and** sign-change rate of Δsize < 0.3 **and** the mean lies strictly inside `(floor, ceiling)`. A gauge pinned at 8 or at 128 is **SATURATED, not settled**, and the two must never print the same: floor-pinned means permanent backpressure, ceiling-pinned means the controller has no headroom signal left and is flying blind.

**What resonance looks like in the data.** A sawtooth with lag-1 sign-change ≈ 1.0 is *normal AIMD* and is fine. The pathologies are: **(a) cross-peer phase-lock** — two peers' `head_batch_size` series with cross-correlation > 0.7 at lag 0, meaning they are synchronizing on a shared bottleneck (the conductor write guard) and will beat together, amplifying instead of averaging; **(b) amplitude growth** — peak-to-trough range increasing across windows; **(c) positive cross-correlation between size(t) and admission_wait(t+1)** — correct control is *negative*; positive means the controller is chasing its own output. Alarm on (a) sustained 3 windows; the classic AIMD desynchronizer is jitter — randomize the increment ±25 %.

**Local proof is nearly free and is the highest value-per-minute item in this design.** Because `next_size` is pure, add a **trajectory test** beside the existing unit tests at `reconcile_rails.rs:485-583`: drive 1,000 steps against a simulated conductor whose admission wait is a monotone function of batch size; assert settling within N steps with CoV < 0.15; and — using the two-budget skeleton already present at `:621-622` (`let mut adam = …; let mut matthew = …`) — assert two independent budgets fed correlated noise **do not phase-lock**. Controller stability proven with zero conductors, in milliseconds, re-runnable by `cargo test -p elohim-storage reconcile_rails`.

---

## 11. TWO-TIER VALIDATION — LOCAL MICRO, THEN DEPLOYED SCALE

**The commensurability rule is the spine of this design: local and deployed emit into the SAME histogram — same name, same buckets, same labels — differing only in `env={local,alpha}`.** Mechanism: both services already export `/metrics`; the local harness runs the same binaries via `just dev start` and the scoreboard scrapes `localhost:8090/metrics` and `localhost:8888/metrics`. **A local harness that computes numbers inside the test process instead of reading the process's own `/metrics` is a failed harness** — it would be a different instrument and the delta would mean nothing.

| Hop | (a) Local harness — cost | (b) Deployed probe | (c) Commensurable because |
|---|---|---|---|
| R1/R2/R3/R3b/R6 | `just dev start` + 200-request fixed-rate loop; ~3 min incl. start, one dev container | PodMonitor scrape of **both** doorways (needs the selector fix) → PromQL p95 by hop/class/route | Same `/metrics` body, same buckets, `env` label only |
| R3s/R3z/R4/R5 | same trio, same loop | same scrape | same |
| W1/W1b/W2/W5/W6 | `just bench-all` (`elohim/elohim-storage/justfile:92`, 10 bench files, loopback, minutes) — **but the benches must be changed to drive the real code paths so the process histograms fill, then dump `/metrics`, instead of printing p50/p95 to stdout**, and must be CI-wired (verified today: zero hits in any Jenkinsfile) | `alpha-edgenode-podmonitor.yaml` (already wired, `app: elohim-edgenode`, port `storage-http`) | same |
| W3/W4 | same bench harness, two in-process nodes | same scrape | same |
| W8/W8a/W8b/W9/W10/W11 | `scripts/ci/run-mesh-quiesce-stage.sh` local mode (generates hc sandboxes, `:330-360`; 3 conductors + 2 doorways on one host), minutes | same scrape, plus the Dataplane Validation stage (`elohim/holochain/Jenkinsfile:2203`) | same |
| W7 | mesh-quiesce local mode + the new slow bucket set | same | same |
| W12a | `cargo test -p holochain --test iroh_stage0` (2 conductors, 2 relays, ~6.14 s wall) — **fork edit required** to promote the log field to a histogram, and the test dumps `prometheus::gather()` to JSON with `env=local` | Loki log-field scrape until the fork commit lands; a real series after | Same metric name and buckets registered in the conductor; local goes via `gather()` → JSON, deployed via scrape — identical values |
| S1 | ~0 locally and therefore near-meaningless | a2o scenario `@concern:latency-budget` in Dataplane Validation: write on peer A, poll doorway B, report delta, one prober's clock | delta is expected to be enormous; that *is* the output |
| Controller convergence | pure unit test, milliseconds | gauge + admission-wait analysis over 30 samples | same signals, different n |

**The local↔deployed delta is a first-class scoreboard column**, not a derived curiosity. We already own the flagship instance: stage0b locally converged cross-relay in 6.14 s wall / 0.233 s preflight; the same chain on the 7-peer fleet produces a 4-plane cascade and doorway A at 69.3 % availability. Same code, same chain, opposite verdicts. Making that a printed number per hop is the cheapest early warning available.

**(d) Hops that CANNOT be measured locally — where we fly blind before deploy.**

1. **W12 gossip at fleet fan-out.** Local is 2 conductors, 1 round, no concurrency; the fleet is 7 peers × N rounds and the round-timeout cascade is an N² phenomenon. Structurally not reproducible at N = 2.
2. **Conductor write-guard contention (`PTxnGuard`).** Needs a real corpus of thousands of divergent anchors and real simultaneity. The 2.5–3 h fleet-stall class has no local shape.
3. **Cross-relay WAN RTT and NAT paths.** The adam-vs-matthew order-of-magnitude gap is named in the controller's own doc as "a real fixture property, not noise to be laundered into one constant" (`reconcile_rails.rs:301-304`) — and locally both relays are fast.
4. **The ~20-minute restart addressing churn.** Requires a real fleet roll.
5. **Peer-class diversity itself.** A dev container is exactly one class. **Every non-`household_node` budget is unvalidated locally by construction.**
6. **The doorway breaker cascade.** Needs a real upstream that fails slowly; a mock that fails fast exercises the wrong branch.

Those six are precisely where the delta is the only warning and where it should be *expected* to be large. The design's job is to make it a printed number rather than a surprise.

---

## 12. WHAT THIS WILL NOT CATCH

**Correctness.** Every hop can be fast and wrong; serving a stale head quickly is worse than serving a correct one slowly. This is why the habit check must be a conjunction of speed AND a trust invariant, and it is the failure mode a latency program is most likely to induce.

**The gossip interior.** Residual #4 will tell us *that* W12 is the spike and how large it is; it will never tell us *where inside* — Initiate vs Accept vs diff-exchange vs apply. Closing that needs `kitsune2_gossip` 0.4.1 vendored into `patches/` on the `transport_iroh` precedent (`Cargo.toml:112-128`), and it should earn itself only if residual #4 proves dominant.

**Coordinated omission** — the worst statistical trap in latency work and the one most likely to bite here. When the system stalls, a closed-loop generator stops issuing, so the stall under-represents itself and the breaker cascade reads as "fewer, faster requests." Both harnesses must issue on a **fixed schedule** and count timeouts **at their timeout value**, not as missing samples. This is mitigated by design, not eliminated.

**Anything the fleet doesn't do.** A histogram sees only traffic that happens. The ≤ 30 min cold-peer full-corpus backfill SLO will never be exercised on a warm fleet, so it stays unvalidated until a peer is deliberately wiped.

**Sub-bucket structure.** Two modes inside one bucket are invisible to the gap test. Partly mitigated by exemplars and by the print-every-run rule at n < 30; not eliminated.

**Everything upstream of `handle_request`.** Kernel accept queue, k8s LB, TLS termination. Deployed, that is exactly where a shed hides. R0 − R1 catches some of it but only for probed traffic.

**Clock-skew-poisoned residuals** are avoided only where the self-reported-elapsed discipline is followed. S1 is inherently two-clock and is safe only because one prober owns both ends; anyone who later computes it by differencing two peers' timestamps will get hours of noise.

**Second-order instrument cost.** Cardinality growth over months and Prometheus scrape cost are budgeted (≤ 3,840 series) but not self-alarming; the scoreboard needs a series-count check or the budget decays into a wish, which is the same failure this design accuses `avg_resolution_ms` of.

**And the honest meta-blind-spot:** this design measures the composed system as it is currently *shaped*. The 4-plane 503 cascade was five stacked individually-invisible defects, and per-hop histograms would have made four of the five legible — but the fifth, the latched half-open breaker that never records an outcome, is a *state* bug, not a *duration* bug. No amount of latency instrumentation finds it. Timing tells you where the seconds went; it never tells you that a state machine stopped.
