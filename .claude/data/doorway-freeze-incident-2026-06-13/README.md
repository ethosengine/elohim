# Doorway-alpha freeze — incident handoff (2026-06-13)

**For:** dev on `feat/frontend-eyes-sprint`
**From:** ops (cluster-side investigation). Raw thread dump: `./thread-dump.txt`
**Severity:** alpha gateway (`doorway-alpha.elohim.host`, and `alpha.elohim.host` SSR) intermittently
unavailable — the pod deadlocks under load and stops serving entirely.

---

## TL;DR

The `doorway` pod is **not crashing on its own** — it **deadlocks**. After ~30–60s of serving it
goes completely silent: 0 log lines, every HTTP request (incl. `/health`) hangs 8–60s, nginx logs the
upstream as `499/504`. Mongo + NATS are healthy, cross-node networking is fine, and the bootstrap
flood is 0 at the moment of freeze — so this is **purely an application-level hang**, not infra.

The thread dump shows **all 10 OS threads parked in `futex_wait_queue`**: the single `tokio-runtime-w`
worker, the `angular-renderer` thread, and 8 `V8 DefaultWorker` threads. The doorway container has a
**CPU limit of `1`**, so tokio runs a **single worker thread** — one blocking call on it freezes the
entire gateway.

**Most likely root cause:** an Angular SSR render whose `fetch` (→ `DataFetcher` → doorway resolver →
conductor/zome call) **hangs with no enforced timeout**, wedging the single sequential SSR isolate,
which back-pressures onto the lone tokio worker. The last log line before every freeze is a **burst of
`PUT /bootstrap` requests** — load is the trigger; the single-worker + sequential-isolate design is why
it deadlocks instead of just slowing down.

---

## Evidence

| Signal | Observation |
|---|---|
| Logs at freeze | **0 lines in 60s** (healthy doorway logs continuously: EPR refresh every 30s, bootstrap, signals) |
| nginx upstream | `10.1.58.30:8080` (the doorway pod) returning **499 after 7.96s / 4.87s / 59.85s** |
| `/health` | times out at 6s **even direct, same-node, bypassing ingress**; readiness only squeaks through within its 15s timeout occasionally |
| Threads | **10/10 in `futex_wait_queue`** (state S): `doorway` main, **1×** `tokio-runtime-w`, `angular-renderer`, 8× `V8 DefaultWorker` |
| Deps | MongoDB `ping ok`; NATS connected; bootstrap flood **0/15s** at freeze |
| Trigger | last log before freeze = **burst of `PUT /bootstrap/...`** (dozens in one ms), then silence |
| CPU limit | container `resources.limits.cpu: 1` → tokio = **1 worker thread** |

Full per-thread `comm`/`state`/`wchan` table is in `./thread-dump.txt`. Kernel **symbol** stacks were
not capturable (non-privileged container — `/proc/1/task/*/stack` → EPERM). To get Rust symbol
backtraces: catch a frozen pod and, on its node, run
`sudo rust-gdb -p <pid> -batch -ex 'thread apply all bt'`.

---

## Why it deadlocks (code path)

The SSR engine in `elohim/elohim-render/src/angular.rs` (~line 121) is deliberately **sequential**:

```rust
// Bounded channel — capacity 1 for sequential MVP isolate.
let (tx, rx) = mpsc::sync_channel::<StringWorkItem>(1);
// ...one "angular-renderer" thread owns the V8 isolate, running its own
//    tokio::runtime::Builder::new_current_thread()...
```

…and the code's own comment warns:

> *Without fetch, Angular SSR bootstrap **hangs forever** waiting on HttpClient calls (services like
> ConfigService, AuthService, ContentService).*

The chain:

1. A page request triggers an Angular SSR render.
2. Angular bootstrap issues `HttpClient` calls → `elohim_render::DataFetcher` (impl in
   `doorway/doorway-service/src/ssr.rs`) → doorway resolver → **conductor/zome call**.
3. That conductor call can hang — we observed `Failed to connect to conductor: ... Name or service
   not known` (intermittent cluster NXDOMAIN) and `record_heartbeat`-style zome errors in the live
   logs (`services/zome_caller.rs`). If the SSR `fetch` future has **no hard timeout**, the render
   never completes.
4. The `sync_channel(1)` + single `angular-renderer` thread means **one stuck render blocks all
   subsequent renders** (head-of-line).
5. The request handler awaiting that render — on the **single** tokio worker (cpu=1) — blocks too.
   Add the concurrent `/bootstrap` flood also competing for that one worker → **total wedge**.

## Suspect files

- `elohim/elohim-render/src/angular.rs` — sequential `sync_channel(1)` isolate; `new_current_thread`.
- `elohim/elohim-render/src/runtime.rs` — `DenoJsRuntime` / V8 isolate boot.
- `doorway/doorway-service/src/ssr.rs` — `DataFetcher` impl; note the existing "soft-deadline / stall"
  machinery (~ssr.rs:196) — **verify it actually bounds the fetch/conductor future**, not just the
  outer render.
- `doorway/doorway-service/src/services/zome_caller.rs` — conductor zome calls (the ones hanging).
- `doorway/doorway-service/src/worker/{pool,processor,conductor}.rs` — worker pool that awaits renders.
- `doorway/doorway-service/src/main.rs:30` — `#[tokio::main]` (inherits cpu-limited worker count).

---

## Recommended fixes (sprint-sized → durable)

1. **Hard timeout on every SSR `fetch` / conductor call** (`tokio::time::timeout`). A single
   unresolvable conductor lookup must *not* be able to hang a render forever. This alone likely stops
   the deadlock.
2. **Don't let SSR head-of-line block** — pool more than one isolate, or bound the render queue with a
   timeout + fast fallback so a stuck render is abandoned, not infinitely queued.
3. **Decouple `/health` from the request path** — serve it on a separate listener/runtime (or make it
   a cheap liveness flag) so the probe reflects "process alive," not "request path drained." Right now
   a wedged request path drags `/health` down with it, which defeats kubelet's restart-on-hang.
4. **Give tokio more than one worker** — the cpu=1 limit forces a single async worker; even one
   `spawn_blocking`/blocking await freezes everything. Bump the limit and/or set `worker_threads`
   explicitly, and run blocking SSR via `spawn_blocking`.
5. **Rate-limit / shed `k2` bootstrap PUTs** — the flood is the trigger; a bounded concurrency or
   shed-load guard keeps a burst from saturating the worker.

---

## Ops context (what changed on the cluster, so you don't chase ghosts)

- **MongoDB for alpha was migrated** `thinkc-p0h` (hostpath) → **`intel-nuc`** earlier tonight, with a
  verified cold data copy (3652 projected_entries, 531 users, both `doorway-alpha`/`-b` DBs). The
  doorway connects to it fine — **mongo is not implicated** in this freeze.
- `thinkc-p0h` was rebooted (kernel update) and uncordoned; healthy.
- A temporary probe-tolerance patch + intel-nuc pin were applied to stop a *separate* 137 liveness-kill
  crashloop, then **reverted** — the deployment is back to **baseline** (startup ft=24, liveness ft=5,
  readiness ft=5, no nodeSelector). So under load the baseline pod will liveness-kill/restart roughly
  every couple minutes until the deadlock is fixed.
- Image at time of capture: `harbor.ethosengine.com/ethosengine/elohim-doorway:1.0.0-dev-3d04d24f`
  (DEVELOPMENT mode).
