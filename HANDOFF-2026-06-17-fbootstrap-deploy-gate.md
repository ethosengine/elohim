# HANDOFF 2026-06-17 — F-BOOTSTRAP fix is READY but the deploy path restarts the genesis pair (operator gate)

> ## ⛔ LEAK-RCA CORRECTED — 2026-06-19 (the F-BOOTSTRAP fix below STILL STANDS)
> The "REAL leak fix" roadmap (§ lines ~118, 125–131) attributes the OOM to off-heap tx5/go-pion CGo buffer
> retention curable only by an upstream transport pin. REFUTED: go-pion exonerated (Go heap flat ~52MB); the
> leak was glibc-malloc arena retention (Rust/C allocations). CURED locally by glibc→jemalloc — no upstream
> tx5 pin, no config-topology mitigation needed. Item 2 ("reduce transport work via arc factor") is also
> falsified (arc=0 leaks the same shape). EVERYTHING ELSE here is correct and stands: F-BOOTSTRAP is a real
> bootstrap-coherence fix, the islanding-not-the-leak refutation holds, the heap-flat observation was right —
> only the *go-pion attribution* of the off-heap growth was wrong.
> Truth: .claude/data/conductor-leak-jemalloc-cure-verdict-2026-06-19.md · conductor-leak-rca-native-heap-reframe-2026-06-18.md


**For:** the operator (Matthew) / integrator landing the bootstrap-islanding fix.
**TL;DR:** The fix is implemented, reviewed, execution-validated, committed on `claude/fbootstrap-shakeout` (= `feat/frontend-eyes-sprint` HEAD `81ce20c2e`). **I did NOT push it.** The reason is a safety gate I could not clear autonomously: the standard edge deploy **rolling-restarts every conductor StatefulSet, including the matthew/adam genesis pair** — which your standing constraint says I must never do. The fix itself only needs the *doorway* to restart. **Your call to trigger; recommended safe path below.**

---

## 1. What's ready (no further work needed)

The kitsune2 bootstrap-islanding fix (F-BOOTSTRAP plan, 8 tasks) — per-pod in-memory `DashMap` → shared mongo-backed table so matthew (doorway-A) and adam (doorway-B) PUT/GET the same rows and can finally discover each other.

- **Code:** `doorway/doorway-service/src/bootstrap/k2_mongo.rs` (`MongoK2Store`), `k2.rs` (extracted `K2Store` async trait + shared `validate_put`), `store.rs` (env-selects backend), `db/schemas/bootstrap_entry.rs` (TTL + unique `(space,agent)` index), `GET /admin/bootstrap-coherence` read-model.
- **Safe-by-default:** `BootstrapStore::new` selects `MongoK2Store` **only** when BOTH a mongo client is present AND `BOOTSTRAP_MONGODB_DB` is set. Env unset → `MemK2Store`, byte-identical to today. One-step rollback = unset the env.
- **Wiring:** `BOOTSTRAP_MONGODB_DB=elohim-bootstrap` added to `genesis/orchestrator/manifests/doorway/alpha.yaml` + `alpha-b.yaml` (commit `2f5702e01`).
- **Validated:** cross-pod visibility proven against local podman mongo (`put_on_a_is_visible_on_b_across_distinct_db_names` PASSED). GET is byte-identical to the mem backend (verbatim `raw_body`, comma-joined, no serde round-trip → preserves `AgentInfoSigned::decode_list`). No mongo error panics (GET→`[]`, stats→`(0,0)`, PUT→`Err`).
- **Caveat:** clippy did not run on the final tree locally (disk hard-ceiling); CI is the backstop. Style-only risk.

Commits `0bccadb8a … 81ce20c2e` (+ diagnosis docs `1a4cb7177`).

---

## 2. The gate — why I stopped (the load-bearing finding)

`claude/fbootstrap-shakeout` is a dev-class branch (orchestrator `Jenkinsfile:873` — `claude\/.+` matches `isDevBranch`), so a push dispatches `elohim-edge` with `UpstreamCause` → the **Deploy Edge Node - Alpha** stage runs.

That stage (`elohim/holochain/Jenkinsfile:1830`) calls `deployHumansInParallel` **unconditionally**, which calls `deployHumanManifest` → **`Jenkinsfile:712 kubectl rollout restart statefulset/<human>`** for every human (`matthew, adam, jessica, james, pete` — default `HUMAN_ASSIGNMENTS`). `kubectl rollout restart` bounces the pod **even though my change leaves the edgenode image unchanged** (it patches the pod-template `restartedAt` annotation). The doorway restart that actually activates my fix is a *separate* call right after (`Jenkinsfile:763 rollout restart deployment/elohim-doorway-alpha`).

**There is no push-reachable param/changeset that runs the doorway deploy while skipping the human-conductor restart.** `DEPLOY_ONLY` only skips *build* stages; it still runs `deployHumansInParallel`. My whole fix lives under `doorway/**`, so the Deploy stage's `when` always matches.

**Why this is the dangerous half, not a formality:** bootstrap is a *runtime* protocol, not startup config. The doorway restart alone makes `MongoK2Store` live; the **already-running** matthew/adam conductors then populate and read the shared store on their next heartbeat (the ~3–30 min PUT/GET cadence) and rediscover each other — **no conductor restart required**. Conversely, forcing matthew+adam to restart makes them re-bootstrap *through the very fix being shaken out, unproven against the live cluster's mongo, overnight, unwatched.* If the fix has a live-env bug (env not reaching the pod, mongo connectivity, serialization), a doorway-only deploy fails safe (conductors keep their existing peer state); the full deploy fails into the exact DHT partition the genesis-pair constraint exists to prevent. The restart is what arms the risk, and it's avoidable.

(No re-key risk in either case: doorway-only change, DNA hash unchanged → the install stale-check reads "not stale," so even with `ALLOW_DNA_REINSTALL=true` on alpha no reinstall fires. The line-712 concern is a *rolling restart*, not a reinstall.)

---

## 3. Recommended safe path (operator-owned)

**Goal: get the new doorway image + `BOOTSTRAP_MONGODB_DB` live on the doorways WITHOUT restarting the matthew/adam conductor StatefulSets.**

1. **Build the doorway image from the branch** (build stages restart nothing). Push/merge `claude/fbootstrap-shakeout` (or cherry-pick the 8 commits) so the edge pipeline builds + pushes the `elohim-doorway` image to Harbor. If you let the *full* edge Deploy stage run here, it WILL restart the genesis pair — so either accept that (your call; a plain rolling restart rejoins the same DHT, same key — routine), OR stop after the build and do step 2 by hand.
2. **Doorway-only activation** (the truly-safe path, satisfies both constraints):
   ```
   kubectl apply -f genesis/orchestrator/manifests/doorway/alpha.yaml -n elohim-alpha
   kubectl apply -f genesis/orchestrator/manifests/doorway/alpha-b.yaml -n elohim-alpha
   kubectl rollout restart deployment/elohim-doorway-alpha   -n elohim-alpha
   kubectl rollout restart deployment/elohim-doorway-alpha-b -n elohim-alpha
   ```
   (apply carries the new image tag + `BOOTSTRAP_MONGODB_DB`; the two doorway rollouts pick them up; the conductors are never touched.)
3. **Rollback if needed:** unset `BOOTSTRAP_MONGODB_DB` in both manifests, re-apply, rollout the doorways → instantly back to `MemK2Store`.

If you'd rather just take the routine full deploy (you own the genesis-pair risk decision and a rolling restart is benign here), a normal `claude/*`/dev deploy lands it — the restart is the standard rollout, not a reinstall.

---

## 4. Read-out — what "it worked" looks like (for the post-deploy shakeout)

- ✅ **Fix working:** `GET /admin/bootstrap-coherence` shows both genesis agents present across both doorways; Loki rate of `validation_receipt_workflow … could not find url for peer` on the leecher conductors **falls**; `rate(elohim_node_conductor_smaps_anon_bytes{class="other"})` flattens on jessica/james.
- ⚠️ **Escalate (pin holochain #5719 / file upstream):** the receipt error disappears but a transport-level send error replaces it AND `other_anon` keeps climbing — that means resolution-success didn't stop the re-drive loop; the leak is the per-send buffer retention independent of the bootstrap miss.

Pre-fix baseline for the before/after is captured in §5 (gathered tonight, non-destructively).

---

## 5. Pre-fix baseline (gathered this session, read-only — Prometheus `elohim_node_*` now LIVE)

The operator's PodMonitor + NetworkPolicy landed this session — the `elohim_node_*` gauges are **scraped in Prometheus now** (no more Loki JSON-parsing). Baseline captured at cluster time ≈ `2026-06-17T11:30Z` over a ~7h scrape window (scraping began when the PodMonitor was applied). **Clock note:** the dev container's `date -u` runs ~6h BEHIND cluster time — use Prometheus/Loki **relative** windows (`now-Xh`), never absolute RFC3339 from the dev box.

**Deploy status at capture: NOT deployed.** doorway `/health` = old image (uptime ~1.7h, pool 14/14 healthy, p2p peerCount 13, caughtUp); the fix's new `GET /admin/bootstrap-coherence` route returns **404** ("Use WebSocket connection to /admin" = the old WS-only `/admin`). So this is a clean pre-fix window; no RCA branch was triggered.

**Topology grew to 14 edgenode pods** (not the historical 6). `conductor.smaps` `heap`/`stack` are flat & tiny (~16–32 MB / 128 KB) on every pod — the leak is entirely in `class="other"` (discrete >128 KB anon mmaps), confirming the H3 verdict.

### Per-pod `other_anon` (restart-aware — slope taken only over clean monotonic climbs; large drops = OOM/reclaim)

| pod | role | other_anon now | climb rate (clean) | receipt err/h | reading |
|---|---|---|---|---|---|
| **matthew** | genesis (doorway-A) | 7.83 GB | **~2.0 GB/h**, sawtooth peak 7.8 / trough ~2.8, cycle ~3h | **95,179** (~26/s) | floor + heavy amplifier |
| **james** | leecher | 7.45 GB | ~1.35 GB/h, peak 7.65 / trough ~4.7 | 49,840 (~14/s) | floor + amplifier |
| **jessica** | leecher (arc=0) | 3.42 GB | ~1.5 GB/h, lower amplitude | 36,353 (~10/s) | floor + amplifier |
| **adam** | genesis (doorway-B) | 5.23 GB | ~1.0 GB/h\* (\*incl. post-restart refill — drop to 1.19 then steady climb) | **~0** | floor (+ refill) |
| **terrance** | household | 3.93 GB | **0.21 GB/h** — clean monotonic, no drops in window | **0** | **floor only — reference rate** |
| eve / pete / daniel | household | 1.3–3.0 GB (bursty) | ~0.2–0.5 GB/h net, frequent partial reclaims | 0 | floor only (noisy) |

Aggregate "could not find url for peer" = **181,474/h** across elohim-alpha — but emitted by **only 3 of 14 pods** (matthew, james, jessica); the other 11 emit **zero**.

### Mechanism decomposition (NEW — measured, not inferred from a single pod)

The leak splits into two additive components:

1. **FLOOR (~0.2 GB/h, ALL 14 pods, zero receipt errors).** `terrance` is the clean witness: steady 0.21 GB/h with no peer-resolution errors at all. This is the **universal per-send buffer retention (H3)** — it happens on every conductor regardless of peer-resolution success. **F-BOOTSTRAP does NOT touch this.** It is the holochain `#5718/#5719` / upstream-pin territory (KNOWN-OPEN). At 0.2 GB/h the OOM cycle is ~30h+ — slow, survivable with periodic restart.
2. **AMPLIFIER (+~1–1.8 GB/h, only matthew/james/jessica), scaling with receipt-error rate.** matthew's ~2.0 GB/h ≈ 0.2 floor + ~1.8 amplifier, driven by 95k "could not find url for peer"/h → DB-backed re-drive loop → high failed-send volume → per-send retention amplified. **This is F-BOOTSTRAP's target** — *if* the unresolvable peers are the bootstrap-islanded ones (matthew on doorway-A's store cannot resolve peers registered only in doorway-B's store). The shared `MongoK2Store` makes both stores one, so the resolution should succeed and the re-drive loop subsides.

### Read-out (sharpened by the decomposition)

- ✅ **Fix working:** matthew's receipt-error rate → ~0; matthew's `other_anon` slope drops from ~2.0 GB/h toward terrance's ~0.2 GB/h **floor** (≈10× slower leak; OOM cycle 3h→30h+); `GET /admin/bootstrap-coherence` returns 200 with both genesis agents present on both doorways. Same expected for james/jessica.
- ⚠️ **Escalate (pin holochain #5719 / upstream):** (a) receipt errors **persist** post-fix → the unresolvable peers were NOT the islanded ones (transport/NAT, not bootstrap) → amplifier stays; or (b) the residual **floor** (~0.2 GB/h on all 14 pods) is judged too fast → the real cure is the upstream per-send-buffer fix, which F-BOOTSTRAP was never going to deliver.

**Caveat held honestly:** the amplifier→bootstrap-islanding link is the *leading* hypothesis (strong circumstantial support: matthew the doorway-A anchor has by far the most errors; the fix targets exactly this resolution path), but it is not proven from logs alone — **the doorway-only deploy is the definitive test.** Even in the worst case (amplifier unaffected) F-BOOTSTRAP still fixes genesis-pair *discovery*, which is a correctness win independent of the leak.

## 6. POST-FIX RESULT — islanding-amplifier theory REFUTED; F-BOOTSTRAP is a correctness + doorway-stability win, NOT the leak cure

Landed on **dev** (`0b9e5308c`, surgical cherry-pick of the 9 F-BOOTSTRAP commits — the 3 doorway-metrics commits were already on dev via the integrator), orchestrator #1270 → edge #1090 → alpha. Deploy landed ~`12:52Z` cluster time: `GET /admin/bootstrap-coherence` → `{"backend":"mongo","agents":1215→1705,"spaces":11}` — `MongoK2Store` confirmed live and accumulating. Conductors restarted (T0 leaks reset).

**The decisive measurement — the clean test is the steady-state receipt-error rate** (unconfounded by the T0 restart), sampled 15–25 min past the cold-bootstrap phase with the shared store live and full (1705 agents, both genesis agents unified):

| pod | receipt errors/h (was → now, settled) | note |
|---|---|---|
| matthew | 95k → **~95k (ZERO CHANGE)** | the clean refutation signal |
| james | 50k → ~38k | within noise |
| jessica | 36k → ~13k | depressed by a mid-window OOM-restart |
| terrance (floor witness) | 0 → 0 | unchanged |

**DECISION GATE → REFUTED.** Unifying the bootstrap store should have improved peer-URL resolution; matthew's "could not find url for peer" rate **did not move at all**. Bootstrap islanding was **not** the driver.

> ⚠️ Do NOT cite the `other_anon` slope as proof — it is **confounded by the T0 restart** (every conductor restarted on deploy, so post-fix is cold-refill, not a steady climb comparable to the pre-fix ~2.0 GB/h). matthew ~3.6 / terrance ~0.37 GB/h post-deploy only re-confirms matthew≠terrance (always true); it is not a clean before/after. **The receipt rate carries the verdict, not the slope.**

**Mechanism (why a full shared store changed nothing):** "could not find url for peer" is a **local peer-store / transport resolution miss driven by ongoing churn** — peers OOM-cycling leave stale/dead tx5 URLs in every peer's local store. Bootstrap unification fixes *initial discovery*, not churn-driven staleness, so a complete shared store has zero effect on the steady-state failure rate.

**This confirms the audit critic's mechanism correction:** the leak is **off-heap tx5/go-pion (CGo) transport buffer retention** (Rust `heap` is dead-flat — matthew 32,313,344 B for 8 samples — while `other` anon grows GBs; a Rust receipt queue would grow the heap, it doesn't), **one mechanism at different volumes** (terrance leaks the same monotonic shape at ~0.12–0.37 GB/h; there is no separate "amplifier vs incurable floor" — it's send/reconnect-volume scaling), driven by P2P transport work. The settled-memory sub-claim "158 MB largest_anon = fixed wasmer, byte-identical" is **also false** (77 MB on terrance/nancy, not constant).

**What F-BOOTSTRAP DID buy (keep it — it's correct and net-positive):**
- **Bootstrap coherence** — genesis pair now shares one table (real islanding fix; islanding *was* real, just not the leak's cause). backend=mongo, 1705 agents.
- **Doorway: held 11 min post-deploy — cause UNATTRIBUTED, watching.** The alpha doorway logged ~65 restarts (~2-min uptimes) and is now at 674s+ uptime. Do **not** credit F-BOOTSTRAP: 11 min isn't conclusive against a minutes-long crashloop, and the "DashMap→mongo relieved OOM" mechanism doesn't hold quantitatively (1705 agent-infos in a DashMap is single-digit MB, not an OOM driver). More likely mundane — many of the "restarts" were rollout churn during the deploy attempts, and it's simply stable now because the deploy finished. **Verify** with the doorway's actual restart-count + OOM-reason before claiming any win.
- Safe-by-default, one-step rollback (unset `BOOTSTRAP_MONGODB_DB`).

**The REAL leak fix (corrected roadmap) — note the leaking path is in the holochain/tx5 binary we do NOT compile (edgenode base image), so "fix it in our code" is mostly not a lever we hold:**
1. **Upstream transport pin (holochain/tx5 #5718/#5719)** — the leak lives in go-pion below our reach; the only thing that flattens the sawtooth. P0 for the conductor leak, and the only true *cure*.
2. **Reduce transport work via conductor CONFIG/topology** (the lever we DO hold) — less send/reconnect volume → lower leak rate: arc factor / `target_arc`, peer count, topology. This is config, not a code change to the receipt/tx5 path (which we don't compile). NB: the audit's "bound the receipt queue in our code" is wrong twice over — no queue is growing (heap flat), and the path isn't ours.
3. **Self-healing is wrong-axis** — admission shed / circuit breakers / memory bound all defend inbound-HTTP + upstream-storage; none wrap the outbound transport path. Accept they only convert crash→sawtooth; treat OOM-restart count as the pressure gauge, keep the memory bound as a crash-floor not a strategy.
4. **Keep F-BOOTSTRAP** — it is a real bootstrap-coherence fix (genesis pair unified; islanding *was* real, just not the leak's cause), safe-by-default, one-step rollback. It is not the leak cure, and that's fine.

Cheap discriminating test still worth running to fully nail the mechanism: correlate anon-growth with reconnect/connection-churn rate vs receipt-send count (transport-churn vs receipt-path).
