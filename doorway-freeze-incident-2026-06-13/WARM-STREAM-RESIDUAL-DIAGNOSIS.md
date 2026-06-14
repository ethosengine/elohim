# Doorway residual under-load wedge — root-cause diagnosis (warm_stream / Mongo)

**Date:** 2026-06-14 · **Method:** systematic-debugging Phase 1–3 (evidence + hypothesis), primary-source-verified against the **current** post-fix-commit code. Phase 4 (fix) is **deploy-gated** — see §2.
**Sibling docs:** `README.md` (ops incident handoff), `CASCADE-DEADLOCK-AND-FIX.md` (the CI deadlock that blocked the deploy).

## TL;DR

The symptom is the discriminator: **all 10 OS threads in `futex_wait`, zero error logs, ~2.5min under sustained load (on the pre-fix `cpu:1` image)**. A futex-parked thread is *suspended-pending*, not computing — that fits **task-level starvation** (every task pending on an external await that never completes), not CPU-saturation and not a lock cycle (refuted on every surface).

**Genuine residual = H3: a continuous re-projection firehose (worst at cold start) parks every worker on an unbounded Mongo await.** It is *orthogonal to `worker_threads=4`* (4 workers park on the same await exactly as 1 does), so the committed fix does not cover it. **Do not apply a fix yet** — the captured dump is from the pre-fix image and is a mechanism *clue*, not proof the residual recurs. Run the §2 observation against a `worker_threads=4` pod first.

> **Verified by direct read (2026-06-14, not inherited from the sub-agents):** `find_one` (`store.rs:190-193`) and `replace_one` (`store.rs:263-267`) are both **unwrapped** (no `tokio::time::timeout`); the URI (`mongo.rs:42-44`) carries only `serverSelectionTimeoutMS=3000&connectTimeoutMS=3000`; the idempotency guard (`store.rs:216-220`) keys off `hot_cache` with `unwrap_or(false)`. **Cache-cap check (tightens H3):** `max_hot_cache_entries=10_000` > the ~3.6k corpus (`store.rs:54`) — so the firehose is NOT capacity-thrash; the full-cost path is the cold-start empty cache and/or re-projections that aren't byte-equivalent (so `is_equivalent_projection` never short-circuits the guard). **Timing nuance:** warm_stream's 75s budget (`warm_stream.rs:40`) ends ~85s in, but the wedge is ~150s — so the sustained firehose past cold start is **signal-driven re-projection** (every DHT signal re-`set`s), not only the first warm pass. The §2 Loki last-log discriminator settles which empirically.

---

## Hypothesis set (grounded in current code, ranked by likelihood of being a *genuine* residual)

### H3 — Cold-start projection firehose parks all workers on an unbounded Mongo await — **RANK 1, genuine residual**

**Mechanism:** During warm_stream's first pass over a large corpus, every read `get()` and write `set()` awaits a Mongo op with *no per-operation timeout*; if Mongo is slow-but-alive or the pool is exhausted, each worker parks on that await, draining runnable tasks until all threads sit in `futex_wait` with zero error logs.

**Evidence FOR (quoted):**
- Firehose is real + in-source acknowledged — `store.rs:206-211`: *"warm-stream re-streams the entire corpus on every (re)connect and every DHT signal re-projects … ~14k upserts/2min over a ~3.6k-doc corpus on alpha — a re-projection firehose that saturated the gateway."*
- Idempotency guard keys off the **hot cache** (`store.rs:216-220`); on an empty cache (settled pod's first warm pass — when the freeze was captured) `already_current` is always `false`, so every doc takes the full `upsert_to_mongo(...).await` (`store.rs:229-231`).
- Mongo awaits are **not per-operation bounded** (code-grounded, the load-bearing leg): `find_one(...).await` unwrapped (`store.rs:190-193`); `replace_one(...).await` unwrapped (`store.rs:263-267`). URI sets only `serverSelectionTimeoutMS=3000&connectTimeoutMS=3000` (`db/mongo.rs:42-44`) — no `socketTimeoutMS`/`timeoutMS`/`maxTimeMS`. A **slow-but-alive Mongo** therefore parks the op with no bound. (A second, *possible* leg — pool-checkout exhaustion at the driver-default `maxPoolSize` with no client `ClientOptions` override in `mongo.rs` — is driver-version-specific and asserted from recall, not code; the slow-Mongo leg carries the conclusion on its own.)
- Timing fits: warm_stream spawns at +10s (`main.rs:955`), runs to a 75s peer budget (`warm_stream.rs:40`) — wedge onset aligns with the firehose window, not steady state. Write path is serial-await per entry (`warm_stream.rs:340`), so a slow Mongo turns the firehose into a stall.

**Evidence AGAINST / scope:**
- **Dead/unreachable Mongo is REFUTED by the symptom:** `serverSelectionTimeoutMS=3000` fails server-selection every 3s and emits `error!` (`store.rs:172`). Zero error logs ⟹ the survivor is *slow-but-alive Mongo* or *pool-checkout exhaustion* (both park past 3s with no error). State this precondition; do not assert H3 broadly.
- "Zero logs" weight is asymmetric: zero **ERROR** logs is load-bearing; absence of `debug!` throughput lines is weaker (likely level-suppressed in prod).
- **Lock-across-await REFUTED on every surface:** idempotency read guard dropped before any await (`store.rs:213-215`); `evict_if_needed` fully sync (`store.rs:558-591`); `WarmStreamHealth` Mutex released with explicit `drop(map)` (`warm_stream.rs:207`); broadcast channel bounded drop-oldest (`store.rs:87`).

**PLAUSIBILITY vs the committed fix: NOT covered.** `worker_threads=4` fixes *one blocking await on the sole worker*; H3 is *all workers parking on the same external await* — orthogonal to thread count. `DOORWAY_ZOME_CALL_TIMEOUT_MS` bounds only the conductor WebSocket path, not the HTTP→Mongo projection path. **Genuine residual, pending §2.**

### H2 — SSR head-of-line on the single V8 isolate — **RANK 2, real residual but WRONG symptom**

`sync_channel(1)` + one `angular-renderer` thread, sequential `eval_string().await` (`angular.rs:112-157`); deferred `TerminateExecution` (`angular.rs:179-181`) → a pure-JS hang occupies the isolate until restart. **But:** the isolate is on its **own `std::thread`** (`angular.rs:121-126`) — it parks *that* thread, not a gateway worker; the inner fetch **is** time-bounded + cancelled (`traced_fetcher.rs:99-101`, `tokio::time::timeout` ~1.2s); the `try_send`/Busy→CSR shed bounds isolate-parked workers to ≤2 (`angular.rs:243-257`, `http.rs:3227-3236`). So H2 **cannot** produce all-10-threads-futex. Its blast radius is "SSR degrades to CSR-shell, `/health` stays fast" — a *different* symptom, so H2 is a **discriminator** (§2), not the wedge.

### H1 — `cpu:1` single-tokio-worker blocking wedge — **RANK 3, ADDRESSED**

Runtime now builds the pool explicitly, CPU-decoupled: `DOORWAY_WORKER_THREADS` else `DEFAULT_WORKER_THREADS=4` (`main.rs:51`), `Builder::new_multi_thread().worker_threads(workers)` (`main.rs:84`); in-source incident note `main.rs:34-51`. The blocking-single-worker freeze is **fixed — do not re-fix.** A residual CPU-saturation sub-claim survives structurally but **mis-fits** the futex+zero-logs symptom (a throttled compute workload shows compute stacks, not futex parks).

### H4 — k2 bootstrap PUT flood — **RANK 4, REFUTED**

k2 store is a private `DashMap` shared with nothing on the SSR/projection path (`k2.rs:50-53`); `put_at` fully synchronous, sub-ms, bounded `MAX_ENTRIES_PER_SPACE=128` (`k2.rs:39`); bootstrap PUT is admission-gated (`http.rs:2120`, 503+Retry-After at `DOORWAY_MAX_INFLIGHT`). Collapses into H1-CPU; not a distinct mechanism. The "PUT burst interleaved with warm_stream" in the dump is co-occurring load, not contention.

---

## UPDATE 2026-06-14 — H3 CONFIRMED from the cluster (§2 observation resolved)

Operator cluster-side investigation answered the §2 distinguishing observation directly: doorway-alpha crashlooped 109×/15h on `aa6debe6` (which HAS `worker_threads=4`) — a **load-driven liveness-probe kill** (exit 137, not crash/OOM; doorway-alpha-b on the identical image at adam stayed 3/3). Live evidence: ~8.6k log lines/60s, only ~14/s bootstrap — the rest `projection::store`/`warm_stream`; **avg ~0.25 core but bursty, zero CFS throttling**. So it is **NOT** the cpu:1 single-worker phantom (worker_threads=4 was live) — it is the **genuine warm_stream residual**: the projection replay grabs the Tokio workers in tight bursts and starves the latency-sensitive `/health` task off the runtime on a 1-core quota → /health misses the 15s probe → SIGKILL.

**Refinement to the ranked hypotheses:** the dominant kill mechanism is **burst-starvation of /health** (+ debug-log overhead: 2 lines per projected doc), more than the all-threads-Mongo-await-park H3 originally emphasized — but the ROOT is identical: **warm_stream has no backpressure/yield (open-loop pacing).** The unbounded Mongo awaits (Fix 1) remain a real contributing latency amplifier; the burst-starvation is the proximate killer.

**Stopgap applied (band-aid, operator + shift):** cpu limit 1→2, request 100m→500m on the doorway container (live patch + durable in `genesis/orchestrator/manifests/doorway/alpha.yaml`, commit `ece274734`). Survives the kill window cleanly. **This hides the pacing gap; it does not fix it.** Durable fix = §3 below (warm_stream pacing/backpressure + `/health` isolation onto a dedicated runtime). **Operator-gated:** debug logging kept ON to keep observing; the warm_stream fix waits until the operator has resolution on the pacing question.

## The single distinguishing observation (run the moment the deploy lands)

**Top-line arbiter — splits "already fixed" from "genuine residual":** deploy the `worker_threads=4` image, apply sustained load to ONE pod. **Does `/health` still hang gateway-wide at ~2.5min?**
- **No** → the captured freeze was H1 (already fixed); the all-10-futex dump was a pre-fix artifact. **Ship nothing further. STOP.**
- **Yes** → a genuine residual survives; discriminate below. (Guardrail: the captured dump is pre-fix — best mechanism clue, not proof it presents identically.)

**Discriminators, cheapest first:**
1. **`/health` behavior:** stays fast + only page loads degrade to CSR-shell ⟹ **H2**; `/health` itself hangs ⟹ gateway-wide (H3 / CPU-branch).
2. **`/admin/render-stats` queue depth:** pegged render queue + fast `/health` ⟹ **H2**; near-empty queue + gateway-wide hang ⟹ **H3**.
3. **Mongo metrics at freeze (cheap H3 corroborator, before gdb):** server-side op latency + **connection-pool checkout-wait**. Healthy/fast Mongo + non-exhausted pool ⟹ **H3-park refuted** (fall to CPU-branch); op-latency spike or checkout-wait pegged (pool exhausted at default 10) ⟹ **H3 confirmed.**
4. **Loki last-log slice:** last lines = `Projected streamed entry` then silence ⟹ **H3** (parked mid-firehose); render dispatch ⟹ **H2**; a budget `warn!` (`warm_stream.rs:536`) ⟹ warm_stream completed cleanly, *not* the wedge.
5. **`rust-gdb -p <pid> -batch -ex 'thread apply all bt'` on the node (the decider):** threads in the `mongodb` driver / socket-read / pool-checkout future ⟹ **H3**; a worker on `reply_rx` (render oneshot) ⟹ **H2**; threads on the admission semaphore ⟹ upstream backpressure (not a wedge); a compute stack ⟹ CPU/JS-hang branch.

---

## Staged fix per surviving hypothesis (NOT to be applied until §2 confirms)

### For H3 (apply only if the live pod wedges gateway-wide AND gdb shows Mongo-park / pool-checkout)

- **Fix 1 — lead, surgical: bound the Mongo awaits per-operation.** In `store.rs`, wrap `find_one` (`190-193`) and `replace_one` (`263-267`) in `tokio::time::timeout` (~1.5–2s, env `DOORWAY_MONGO_OP_TIMEOUT_MS`), mirroring the SSR soft-budget pattern (`traced_fetcher.rs:101`); on elapse return the existing `DoorwayError::Database` (which `get` already maps to `None`+`error!`). This converts a *silent unbounded park* into a **sheddable error + log storm** — degrades gracefully AND breaks the zero-logs invisibility that hid this class. Prefer per-call over a global URI `socketTimeoutMS`/`timeoutMS` (the global knob also bounds `ensure_indexes`, startup `ping`, and cursor scans — blast radius).
- **Fix 2 — firehose backpressure (closed-loop).** warm_stream paces by wall-time + per-64 yield + error-streak breaker — **none sense downstream writer saturation.** Expose a pool/inflight-upsert signal from `ProjectionStore` and back off (double inter-entry delay) when upsert latency / checkout-wait climbs, recover (halve) when it drains — the proven `drain_publish_queue` pattern (`[project_closed_loop_ingest_drain_prior_art]`).
- **Fix 3 — root, flag don't require.** Close the idempotency cold-start gap (`store.rs:216-220`): warm the hot cache from Mongo before accepting the firehose, so re-projections hit `already_current` from entry one. Larger change (ordering/consistency); Fix 1 is the bleed-stopper and lands first.

### For H2 (apply only if `/health` fast + render queue pegged + gdb shows `reply_rx`-park / render-thread compute)

- **V8 execution watchdog:** call `terminate_execution` on the isolate after `ctx.limits.wall_time_ms` so a runaway script is force-cancelled, not occupied-until-restart. Requires threading the isolate handle to the watchdog — scope only if live evidence points at H2.

---

## What the committed-but-undeployed fix already covers (do NOT re-build)

- **H1** — explicit CPU-decoupled `worker_threads=4` (`main.rs:51,84`); one blocked await wedges ≤1 of 4 and leaves admission-exempt `/health` (`http.rs:3738-3741`) responsive.
- **SSR back-pressure shed** — blocking `send` replaced by `try_send`→Busy→CSR-shell (`angular.rs:243-257`); ≤2 workers parked on the isolate.
- **Inbound admission gate** — non-exempt routes capped at `DOORWAY_MAX_INFLIGHT`, shed 503 (`http.rs:2120`).
- **Conductor/zome timeouts** — `pool.rs:178`, `conductor.rs:187`, `DOORWAY_ZOME_CALL_TIMEOUT_MS` (`zome_caller.rs:40`).

**The gap none of these close = H3:** projection/Mongo read+write awaits remain unbounded per-operation, and warm_stream pacing stays open-loop. That is the residual the §2 observation confirms or refutes before any §3 fix is applied.
