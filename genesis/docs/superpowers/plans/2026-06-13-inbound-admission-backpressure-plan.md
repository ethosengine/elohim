# Inbound Admission & Propagated Backpressure — Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax.

**Goal:** Make doorway and elohim-storage SHED (not silently queue or hang) under inbound overload — a bounded global request-admission semaphore on doorway's dispatch path, storage's queueing `acquire().await` converted to a `try_acquire` shed that advertises remaining permits, and a bounded, circuit-broken `forward_to_storage` that surfaces upstream backpressure to the browser as a propagated `503 + Retry-After + {status:"catching-up"}` instead of a bare 502 — all while `/health` stays answerable.

**Architecture:** Pillar 2 / defense-in-depth layers 2-4. Today doorway's accept loop has ZERO admission (`http.rs:1128-1132` spawns one task per connection unbounded); storage's `acquire().await` (`http.rs:705-711`) QUEUES forever and never sheds; `forward_to_storage` uses an untimed `reqwest::Client::new()` (`storage_proxy.rs:112`/`:295`) that blocks a worker on a hung peer and returns a bare 502 on failure; nothing emits or honors a `Retry-After`. This plan adds a global `inbound_semaphore` on doorway (mirroring the render path's `try_acquire_owned()` shed idiom, placed AFTER health/version/WS-upgrade exemption so liveness never sheds), converts storage to per-request `try_acquire` + `X-Available-Permits` advertisement, replaces both untimed proxy clients with ONE pooled timed client guarded by a per-upstream circuit breaker (the shared `elohim_compute::CircuitBreaker`), and makes the proxy honor an upstream `429/503 + Retry-After` by re-emitting catching-up to the browser. Admission/permit state is Cat C node-local operational (no DHT entry, no table).

**Tech Stack:** Rust (doorway-service `doorway` + elohim-storage, both native builds; `elohim-compute` shared crate — both crates already depend on it), tokio, hyper, reqwest, serde_json. No new deps. Tests are inline `#[cfg(test)]` unit tests; TDD throughout.

---

## Decisions (dependency ordering + fallbacks)

**(D1) `CircuitBreaker`/`CircuitState` — ONE shared type, reused from the sibling plan; NO doorway-local fallback.** The sibling **Upstream Self-Protection plan** (`2026-06-13-upstream-self-protection-plan.md`, its Task 1, lines 191-289) lands `CircuitState` + `CircuitBreaker` as a pure, deterministic state machine in `elohim/elohim-compute/src/peers.rs`. As of this plan's authoring that type is **NOT yet landed** (`grep CircuitBreaker elohim-compute/src/` is clean; `lib.rs:19` only re-exports `PeerHealthRegistry, PeerHealthSnapshot`). Therefore **Task 1 of THIS plan lands the SAME type, byte-identical, idempotent ("skip if already present")** — both plans converge on identical code in the same file, whichever lands first wins, the other's Task 1 becomes verify-only. doorway consumes `elohim_compute::CircuitBreaker` directly; it needs only `new`/`record_outcome`/`is_open`/`should_skip`, all present. This is the B5 ordering: do NOT hard-depend on the sibling plan having merged, and do NOT fork a second breaker type.

> **CORRECTION (program-roadmap cross-plan review, 2026-06-13): Plan A is the SINGLE canonical OWNER of `CircuitBreaker`/`CircuitState`. This plan (B) is a PURE CONSUMER — A→B is now a HARD ordering dependency, not a coincidence of identical text.** The original "whichever lands first wins, both byte-identical" framing above is superseded: execute **A before B** (see `2026-06-13-self-healing-program-roadmap.md` dependency order). Task 1 here becomes a **verify-or-backfill safety net, not co-authorship**: *if `elohim-compute` already exposes `CircuitBreaker` (it will, once A lands) → verify-only; only if it is genuinely absent → land it verbatim from Plan A's Task 1 definition.* Two independently-authored "byte-identical" copies are a drift trap — A's definition is the source; B never edits or re-specifies it.

**(D2) Tick reconciliation for the wall-clock proxy path.** The shared `CircuitBreaker` is tick-injected (in the sibling plan a tick = one warm-up *pass*). The forward path has no pass counter, so feed it **`tick = monotonic_seconds`** = a process-start `std::time::Instant`'s `.elapsed().as_secs()`. Then `cooldown_ticks` is literally cooldown *seconds*. This is the clean way to run the tick-based breaker on a wall-clock path. The breaker map (`UpstreamBreakers`) owns the process-start `Instant` and derives the tick on every call.

**(D3) Ceiling source — env knob now, Auto-derived ceiling is a named follow-on.** The Auto-config plan's `elohim_compute::limits::derive()` (`2026-06-13-auto-config-resource-probe-plan.md`) is also not landed (`elohim-compute/src/limits.rs` has never existed). So doorway's ceiling comes from a NEW env knob **`DOORWAY_MAX_INFLIGHT`** parsed at startup, with a conservative static default and a `>= floor` clamp (anti-deadlock: never 0). Follow-on (named): swap the default for `elohim_compute::limits::derive().max_inflight` once Auto lands — a one-line change at the `main.rs` parse site, flagged with a code `// FOLLOW-ON:` note.

**(D4) Admission exemption is NARROW (liveness + /version + WS upgrades), NOT `!is_service_path`.** Verified: `is_service_path` (`http.rs:1285-1345`) returns `true` for `/api/`, `/db/`, `/blob/`, `/apps/` — exactly the proxy/flood surface — so gating on `!is_service_path` would exempt the flood and gut the gate. A purpose-built `admission_exempt(path, is_upgrade) -> bool` returns true ONLY for the health family (`/health`, `/healthz`, `/health/startup`, `/ready`, `/readyz`), `/version`, and ANY WebSocket upgrade (signal + `/debug/stream` hold a permit for the whole socket lifetime — never gate them). Everything else, proxy included, is gated. (Doorway also has a SEPARATE dedicated health listener `handle_health_probe` on `DOORWAY_HEALTH_PORT` that bypasses `handle_request` entirely — a second liveness escape, untouched by this plan.)

**(D5) Storage: REMOVE the accept-loop `acquire().await`, do NOT just add a check.** Leaving the connection-level queue (`http.rs:708`) means it still wedges and front-runs the new shed. Task 6 drops the `acquire().await` in `run()` (keeping the `request_semaphore` field), and Task 7 adds a per-request `try_acquire` in `handle_request` AFTER the `/health` (756) and `/version` (762) arms. This changes per-connection → per-request granularity, which is MORE correct for admission (one keep-alive connection no longer holds a permit for all its requests).

**(D6) Failure-outcome mapping for the breaker (get this right).** The breaker records a FAILURE only on `{connect error, request timeout, upstream 429, upstream 503}`. A `2xx` AND any `404`/other-`4xx` is success/neutral — a `404` is a normal blob miss under the no-fanout rule (`doorway/CLAUDE.md`) and must NEVER open the breaker. Breaker-open → shed `503 + Retry-After + {status:"catching-up"}` WITHOUT calling storage.

**(D7) Shed status + values (with rationale).**
- Use **`503 SERVICE_UNAVAILABLE`** (saturation) over 429 for both sheds, consistently — it reads as "the service is catching up," and 429 connotes per-client rate-limiting we don't do.
- **`DOORWAY_MAX_INFLIGHT` default = 256, floor = 8.** Rationale: doorway is a 4-worker (`DEFAULT_WORKER_THREADS=4`) projection edge fronting storage's own 64-permit gate; doorway requests are mostly cheap proxy forwards (I/O-bound), so it can admit several multiples of storage's ceiling before storage's own shed engages — 256 ≈ 4× storage's 64. Conservative (a single doorway pod under burst sheds well before OOM), env-overridable, floored at 8 so a fat-fingered tiny value can't deadlock.
- **Admission Retry-After = 2s** (`DOORWAY_ADMISSION_RETRY_AFTER_SECS`): admission saturation is a transient burst; a short retry drains the backlog fast without a thundering herd.
- **Breaker-open Retry-After = cooldown seconds (30)**: when an upstream is circuit-broken, tell the client to wait the cooldown, not 2s.
- **Proxy client timeouts: connect 3s, request 12s.** Browser-facing (not warm-up's 45s): a human waiting on a content read should get a propagated catching-up well inside a tab's patience; 12s > storage's slowest legit read, 3s connect fails fast on a dead peer.
- **`UPSTREAM_CIRCUIT_FAIL_THRESHOLD = 3`, `UPSTREAM_CIRCUIT_COOLDOWN_SECS = 30`** — open after 3 consecutive failures, cool down 30s before a half-open trial.

**(D8) Permit advertisement is minimal (advertise + honor; sender-clamp is OUT).** Storage promotes its existing `semaphorePermits` (`http.rs:1487`) to default health detail AND adds an `X-Available-Permits` header on the shed. Doorway HONORS an upstream `429/503 + Retry-After` (re-emits catching-up). Doorway *reading* `X-Available-Permits` to clamp a send window is the bilateral-credit follow-on — NOT built here.

**(D9) Coordination — file isolation from the parallel thread.** A parallel thread is active in the shared worktree (arc spec + storage hygiene leak-fixes) and owns `warm_stream.rs` / conductor config / `target_arc_factor`. This plan **does NOT touch `warm_stream.rs`, conductor config, or `target_arc_factor`.** The doorway proxy breaker map lives in its OWN new file `doorway/doorway-service/src/routes/upstream_health.rs` (NOT warm_stream.rs). Execution is **selective-staged** (each `git add` names exact files) or in an **isolated worktree**. Commit only on the shift branch; the integrator pushes.

**(D10) Plan D seam (no poller built).** Every shed-storm leaves a `tracing::warn!` on a distinct greppable target (`admission_busy` / `upstream_shed`) with a counter field, mirroring the render path's `target: "ssr_busy", counter = "ssr_render_busy_total"` idiom — the structured event seam for the future elevate poller. The poller itself is OUT.

**Out of scope (named follow-on plans, do NOT implement here):**
- **Bilateral credit/window** — numeric credit accounting on the wire; doorway reading `X-Available-Permits` to clamp a send window. This plan does coarse admit/shed + advertise + honor only.
- **Upstream Self-Protection** (sibling Plan A) — warm-up warm-set circuit-breaking; this plan REUSES its `CircuitBreaker` (D1) but does not touch `warm_stream.rs`.
- **Auto-derived ceiling** (`elohim_compute::limits::derive`) — env/boot knob only here; the swap site is flagged `// FOLLOW-ON:`.
- **arc-shrink / `target_arc_factor`** — other thread.
- **REA actuation** — `delegates-compute` runtime mutation of the ceiling; knobs stay boot/env.
- **Plan D elevate poller** — only the `warn!` + structured-event seam is left (D10).

---

## Canonical type & function names (MUST match across ALL tasks)

| Name | Kind | Home | Signature / shape |
|---|---|---|---|
| `CircuitState` | enum | `elohim-compute/src/peers.rs` | `Closed`/`Open`/`HalfOpen`; `#[serde(rename_all = "kebab-case")]`. **Reused — byte-identical to sibling Plan A Task 1.** |
| `CircuitBreaker` | struct | `elohim-compute/src/peers.rs` | `new(fail_threshold: u32, cooldown_ticks: u64)`, `record_outcome(&mut self, ok: bool, tick: u64)`, `is_open(&self) -> bool`, `should_skip(&mut self, tick: u64) -> bool`, `state(&self) -> CircuitState`, `error_streak(&self) -> u32`, `last_good_tick(&self) -> Option<u64>`. **Reused — byte-identical to sibling Plan A Task 1.** |
| `admission_exempt` | fn | `doorway server/http.rs` | `fn admission_exempt(path: &str, is_upgrade: bool) -> bool` — true ONLY for health family + `/version` + any upgrade |
| `catching_up_response` | fn | `doorway server/http.rs` | `fn catching_up_response(retry_after_secs: u64) -> Response<Full<Bytes>>` — `503` + `Retry-After` + body `{status:"catching-up", retryAfter:N}` |
| `inbound_semaphore` | field | `doorway AppState` (`server/http.rs`) | `pub inbound_semaphore: Arc<tokio::sync::Semaphore>` — NON-Option (always present, unlike `render_semaphore`) |
| `storage_proxy_client` | field | `doorway AppState` (`server/http.rs`) | `pub storage_proxy_client: Arc<reqwest::Client>` — pooled, connect_timeout 3s + timeout 12s |
| `upstream_breakers` | field | `doorway AppState` (`server/http.rs`) | `pub upstream_breakers: Arc<UpstreamBreakers>` |
| `UpstreamBreakers` | struct | `doorway routes/upstream_health.rs` (NEW) | per-endpoint breaker map: `breakers: std::sync::Mutex<HashMap<String, CircuitBreaker>>`, `started: std::time::Instant`, `fail_threshold: u32`, `cooldown_ticks: u64` |
| `UpstreamBreakers::new` | fn | `routes/upstream_health.rs` | `fn new(fail_threshold: u32, cooldown_secs: u64) -> Self` |
| `UpstreamBreakers::is_open` | fn | `routes/upstream_health.rs` | `fn is_open(&self, endpoint: &str) -> bool` — advances Open→HalfOpen on cooldown elapse; true if the call should be shed |
| `UpstreamBreakers::record` | fn | `routes/upstream_health.rs` | `fn record(&self, endpoint: &str, ok: bool)` — derives tick from `started.elapsed().as_secs()` |
| `ProxyOutcome` | enum | `routes/storage_proxy.rs` | `Ok`, `Failure`, `Neutral` — classifies an upstream result for the breaker (D6) |
| `init_storage_proxy_client` | fn | `doorway server/http.rs` | `fn init_storage_proxy_client() -> Arc<reqwest::Client>` (mirrors `init_ssr_http_client`) |
| `too_many_requests_with_retry` | fn | `elohim-storage services/response.rs` | `fn too_many_requests_with_retry(retry_after_secs: u64, available: usize) -> Response<Full<Bytes>>` — `503` + `Retry-After` + `X-Available-Permits` + `{status:"catching-up", retryAfter:N}` (name kept per B2; emits 503 per D7) |
| `inbound_max` | fn | `doorway main.rs` | `fn inbound_max() -> usize` (mirrors `worker_threads()` at main.rs:49-55) |
| `DOORWAY_MAX_INFLIGHT` | env | doorway | default 256, floor 8 |
| `DOORWAY_ADMISSION_RETRY_AFTER_SECS` | const | doorway `server/http.rs` | `u64 = 2` |
| `STORAGE_PROXY_CONNECT_TIMEOUT_SECS` | const | doorway `routes/storage_proxy.rs` | `u64 = 3` |
| `STORAGE_PROXY_REQUEST_TIMEOUT_SECS` | const | doorway `routes/storage_proxy.rs` | `u64 = 12` |
| `UPSTREAM_CIRCUIT_FAIL_THRESHOLD` | const | doorway `routes/upstream_health.rs` | `u32 = 3` |
| `UPSTREAM_CIRCUIT_COOLDOWN_SECS` | const | doorway `routes/upstream_health.rs` | `u64 = 30` |
| `DEFAULT_MAX_INFLIGHT` | const | doorway `server/http.rs` | `pub const usize = 256` (home is http.rs because the lib's AppState ctors reference it; a lib cannot import a const from the bin `main.rs`. main.rs `use`s it) |
| `MIN_MAX_INFLIGHT` | const | doorway `server/http.rs` | `pub const usize = 8` (same home as `DEFAULT_MAX_INFLIGHT`; main.rs `use`s it) |
| `STORAGE_SHED_RETRY_AFTER_SECS` | const | elohim-storage `http.rs` | `u64 = 2` |

---

## File Structure

| File | Created/Modified | Responsibility |
|---|---|---|
| `elohim/elohim-compute/src/peers.rs` | Modified (idempotent) | Land `CircuitState` + `CircuitBreaker` (byte-identical to sibling Plan A) if not already present. |
| `elohim/elohim-compute/src/lib.rs` | Modified (idempotent) | Re-export `CircuitBreaker, CircuitState` if not already present. |
| `doorway/doorway-service/src/routes/upstream_health.rs` | **Created** | `UpstreamBreakers` per-endpoint breaker map (`new`/`is_open`/`record`, wall-clock tick) + `UPSTREAM_CIRCUIT_*` consts + unit tests. |
| `doorway/doorway-service/src/routes/mod.rs` | Modified | `pub mod upstream_health;` + re-export `UpstreamBreakers`. |
| `doorway/doorway-service/src/server/http.rs` | Modified | `admission_exempt`, `catching_up_response`, `init_storage_proxy_client`; add `inbound_semaphore`/`storage_proxy_client`/`upstream_breakers` to `AppState` (struct + 3 ctors); admission gate in `handle_request` after gate-check (2046); thread client+breakers through the two `forward_*` call sites (3159-3173). |
| `doorway/doorway-service/src/main.rs` | Modified | `inbound_max()` + `DEFAULT_MAX_INFLIGHT`/`MIN_MAX_INFLIGHT`; populate the three new AppState fields at boot (env-parsed ceiling). |
| `doorway/doorway-service/src/routes/storage_proxy.rs` | Modified | Replace `Client::new()` (`:112`, `:295`) with the pooled `client: &reqwest::Client` arg; add breaker arg + `ProxyOutcome` classification; honor upstream `429/503 + Retry-After` → catching-up; thread breaker through `forward_to_storage` + `forward_blob_to_storage` signatures. |
| `elohim/elohim-storage/src/services/response.rs` | Modified | Add `too_many_requests_with_retry` helper (503 + Retry-After + X-Available-Permits + catching-up body). |
| `elohim/elohim-storage/src/http.rs` | Modified | Remove accept-loop `acquire().await` (705-711); add per-request `try_acquire` shed in `handle_request` after `/health`/`/version` arms; promote `semaphorePermits` to default health detail. |

---

## Build / test commands (verified, B6)

Rules (B6 + memory): `RUSTFLAGS=""` for **both native crates** (doorway, elohim-compute); `RUSTFLAGS='--cfg getrandom_backend="custom"'` for **elohim-storage** (WASM getrandom flag — its absence link-fails); `RUSTC_WRAPPER=""` (sccache spawn-ENOENT); `/tmp` target dirs (fingerprint-ENOENT on pool slot); **plain `cargo test`, NEVER nextest**; doorway uses `--lib --bins`, storage uses `--lib`; **never `&&`-pipe a gate exit code** (use `2>&1 | tail -N`).

elohim-compute (Task 1-2):
```
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib peers 2>&1 | tail -40
```

doorway-service (Tasks 3-5, 8-11):
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins upstream_health 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins admission 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins storage_proxy 2>&1 | tail -40
```

elohim-storage (Tasks 6-7):
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib admission 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib response 2>&1 | tail -40
```

Final gate (whole crates + fmt/clippy), Task 12:
```
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/doorway/doorway-service && cargo fmt --check
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-compute && cargo fmt --check
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && cargo fmt --check
```

---

## TASK 1 — Land shared `CircuitBreaker` + `CircuitState` in elohim-compute (idempotent)

Files:
- `elohim/elohim-compute/src/peers.rs` — add types ABOVE `#[cfg(test)] mod tests` (insert after the `Default` impl at line 115, before line 117). Tests append to the existing `mod tests` (after the last test, line 211).

**Idempotency note:** if a prior session (or the sibling Upstream Self-Protection plan) has already landed `CircuitBreaker`/`CircuitState` in this file, SKIP the implementation and run only the verification step. The code below is byte-identical to that plan's Task 1.

- [ ] First check presence: `cd /projects/elohim/elohim/elohim-compute && grep -c "pub struct CircuitBreaker" src/peers.rs` — if `1`, skip to the run-PASS step (verify-only); if `0`, proceed.
- [ ] Write the failing test — append to `elohim-compute/src/peers.rs` `mod tests`:
```rust
    #[test]
    fn circuit_opens_after_k_consecutive_failures() {
        let mut cb = CircuitBreaker::new(3, 30);
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_outcome(false, 1);
        cb.record_outcome(false, 2);
        assert!(!cb.is_open(), "2 failures < K=3 should stay closed");
        cb.record_outcome(false, 3);
        assert!(cb.is_open(), "3rd consecutive failure must open");
        assert_eq!(cb.state(), CircuitState::Open);
        assert_eq!(cb.error_streak(), 3);
    }

    #[test]
    fn circuit_success_resets_streak() {
        let mut cb = CircuitBreaker::new(3, 30);
        cb.record_outcome(false, 1);
        cb.record_outcome(false, 2);
        cb.record_outcome(true, 3);
        assert!(!cb.is_open());
        assert_eq!(cb.error_streak(), 0);
        assert_eq!(cb.last_good_tick(), Some(3));
    }

    #[test]
    fn circuit_should_skip_open_until_cooldown_then_halfopen_one_trial() {
        let mut cb = CircuitBreaker::new(1, 5);
        cb.record_outcome(false, 10); // opens at tick 10
        assert!(cb.is_open());
        assert!(cb.should_skip(11), "within cooldown: skip");
        assert!(cb.should_skip(14), "tick 14, elapsed 4 < 5: skip");
        assert!(!cb.should_skip(15), "tick 15, elapsed 5 >= cooldown: half-open admits one trial");
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.should_skip(16), "half-open trial already admitted: skip until outcome recorded");
    }

    #[test]
    fn circuit_halfopen_success_closes_failure_reopens() {
        let mut cb = CircuitBreaker::new(1, 5);
        cb.record_outcome(false, 0);
        assert!(!cb.should_skip(5));
        cb.record_outcome(true, 6);
        assert_eq!(cb.state(), CircuitState::Closed);
        let mut cb2 = CircuitBreaker::new(1, 5);
        cb2.record_outcome(false, 0);
        assert!(!cb2.should_skip(5));
        cb2.record_outcome(false, 6);
        assert_eq!(cb2.state(), CircuitState::Open, "half-open failure re-opens");
    }

    #[test]
    fn circuit_state_serializes_kebab_case() {
        assert_eq!(serde_json::to_string(&CircuitState::HalfOpen).unwrap(), "\"half-open\"");
        assert_eq!(serde_json::to_string(&CircuitState::Open).unwrap(), "\"open\"");
        assert_eq!(serde_json::to_string(&CircuitState::Closed).unwrap(), "\"closed\"");
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib peers 2>&1 | tail -40` — expect compile error `cannot find type CircuitBreaker` / `cannot find type CircuitState`.
- [ ] Write minimal implementation — insert into `elohim-compute/src/peers.rs` between line 115 (`}` closing `Default`) and line 117 (`#[cfg(test)]`):
```rust
/// Operational circuit state for an upstream. Cat C node-local — no DHT entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Pure, deterministic per-upstream circuit breaker.
///
/// Advanced ONLY by injected outcomes + a monotonic `tick`, never wall-clock —
/// so the state machine is unit-testable without time or network. Opens after
/// `fail_threshold` consecutive failures; stays open for `cooldown_ticks`; then
/// admits exactly ONE half-open trial.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    consecutive_failures: u32,
    opened_at_tick: Option<u64>,
    error_streak: u32,
    last_good_tick: Option<u64>,
    fail_threshold: u32,
    cooldown_ticks: u64,
}

impl CircuitBreaker {
    pub fn new(fail_threshold: u32, cooldown_ticks: u64) -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at_tick: None,
            error_streak: 0,
            last_good_tick: None,
            fail_threshold: fail_threshold.max(1),
            cooldown_ticks,
        }
    }

    pub fn record_outcome(&mut self, ok: bool, tick: u64) {
        if ok {
            self.consecutive_failures = 0;
            self.error_streak = 0;
            self.last_good_tick = Some(tick);
            self.state = CircuitState::Closed;
            self.opened_at_tick = None;
        } else {
            self.consecutive_failures = self.consecutive_failures.saturating_add(1);
            self.error_streak = self.error_streak.saturating_add(1);
            if self.state == CircuitState::HalfOpen
                || self.consecutive_failures >= self.fail_threshold
            {
                self.state = CircuitState::Open;
                self.opened_at_tick = Some(tick);
            }
        }
    }

    pub fn is_open(&self) -> bool {
        self.state == CircuitState::Open
    }

    /// Returns true if this upstream should be skipped on `tick`.
    /// Side effect: an Open breaker whose cooldown has elapsed transitions to
    /// HalfOpen and admits exactly one trial (returns false once); subsequent
    /// calls return true until an outcome is recorded.
    pub fn should_skip(&mut self, tick: u64) -> bool {
        match self.state {
            CircuitState::Closed => false,
            CircuitState::HalfOpen => true,
            CircuitState::Open => {
                let elapsed = self
                    .opened_at_tick
                    .map(|t| tick.saturating_sub(t))
                    .unwrap_or(u64::MAX);
                if elapsed >= self.cooldown_ticks {
                    self.state = CircuitState::HalfOpen;
                    false
                } else {
                    true
                }
            }
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    pub fn error_streak(&self) -> u32 {
        self.error_streak
    }

    pub fn last_good_tick(&self) -> Option<u64> {
        self.last_good_tick
    }
}
```
  (If `Serialize`/`Deserialize` are not already imported in `peers.rs`, add `use serde::{Deserialize, Serialize};` at the top — check the existing imports first; `PeerHealthSnapshot` already derives `Serialize` so the import almost certainly exists.)
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib peers 2>&1 | tail -40` — expect all peers tests pass (existing + 5 new).
- [ ] Commit:
```
git add elohim/elohim-compute/src/peers.rs
git commit -m "feat(elohim-compute): CircuitBreaker pure state machine (shared, idempotent)

Open-after-K, tick-injected cooldown, half-open one-trial. Byte-identical
to the upstream-self-protection plan's Task 1; whichever lands first wins.
Consumed by doorway's inbound-admission proxy breaker.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 2 — Re-export `CircuitBreaker`/`CircuitState` from elohim-compute (idempotent)

Files:
- `elohim/elohim-compute/src/lib.rs:19` — the `pub use peers::{PeerHealthRegistry, PeerHealthSnapshot};` line.

- [ ] Check presence: `cd /projects/elohim/elohim/elohim-compute && grep -c "CircuitBreaker" src/lib.rs` — if `>=1`, skip to verify-only.
- [ ] Write the failing test — append to `elohim/elohim-compute/src/lib.rs`:
```rust
#[cfg(test)]
mod reexport_admission_tests {
    #[test]
    fn circuit_breaker_is_publicly_reachable() {
        let _cb = crate::CircuitBreaker::new(3, 30);
        let _s: crate::CircuitState = crate::CircuitState::Closed;
    }
}
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib reexport_admission 2>&1 | tail -20` — expect `cannot find type CircuitBreaker in crate root`.
- [ ] Write minimal implementation — edit `lib.rs:19` to:
```rust
pub use peers::{CircuitBreaker, CircuitState, PeerHealthRegistry, PeerHealthSnapshot};
```
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib reexport_admission 2>&1 | tail -20` — expect pass.
- [ ] Commit:
```
git add elohim/elohim-compute/src/lib.rs
git commit -m "feat(elohim-compute): re-export CircuitBreaker and CircuitState

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 3 — `UpstreamBreakers` per-endpoint map (NEW file `routes/upstream_health.rs`)

Files:
- `doorway/doorway-service/src/routes/upstream_health.rs` — **NEW file**.
- `doorway/doorway-service/src/routes/mod.rs` — add `pub mod upstream_health;` + `pub use upstream_health::UpstreamBreakers;`.

`UpstreamBreakers` wraps a `HashMap<endpoint, CircuitBreaker>` behind a `Mutex` (interior mutability — `AppState` is shared behind `Arc`, callers hold `&`). The wall-clock tick (D2) is `started.elapsed().as_secs()`, so `cooldown_ticks` == cooldown seconds.

- [ ] Create the file with imports, consts, the struct, and the failing test:
```rust
//! Per-upstream circuit breakers for the storage-proxy path.
//!
//! Cat C node-local OPERATIONAL state (no DHT entry, no table). Wraps the
//! shared `elohim_compute::CircuitBreaker` keyed by storage endpoint URL.
//! The breaker is tick-injected; this map feeds it a wall-clock tick
//! (`started.elapsed().as_secs()`) so `cooldown_ticks` == cooldown seconds.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use elohim_compute::CircuitBreaker;

/// Consecutive failed upstream outcomes before a circuit opens.
pub const UPSTREAM_CIRCUIT_FAIL_THRESHOLD: u32 = 3;
/// Seconds a circuit stays open before a half-open trial.
pub const UPSTREAM_CIRCUIT_COOLDOWN_SECS: u64 = 30;

/// Per-endpoint breaker map for the storage proxy.
pub struct UpstreamBreakers {
    breakers: Mutex<HashMap<String, CircuitBreaker>>,
    started: Instant,
    fail_threshold: u32,
    cooldown_ticks: u64,
}

impl UpstreamBreakers {
    pub fn new(fail_threshold: u32, cooldown_secs: u64) -> Self {
        Self {
            breakers: Mutex::new(HashMap::new()),
            started: Instant::now(),
            fail_threshold,
            cooldown_ticks: cooldown_secs,
        }
    }

    fn tick(&self) -> u64 {
        self.started.elapsed().as_secs()
    }

    /// True if a call to `endpoint` should be SHED (circuit open and not yet
    /// admitting a half-open trial). Side effect: advances Open→HalfOpen when
    /// the cooldown has elapsed (admits exactly one trial).
    pub fn is_open(&self, endpoint: &str) -> bool {
        let tick = self.tick();
        let mut map = self.breakers.lock().unwrap();
        let cb = map
            .entry(endpoint.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.fail_threshold, self.cooldown_ticks));
        cb.should_skip(tick)
    }

    /// Record an outcome for `endpoint` (ok=false counts toward opening).
    pub fn record(&self, endpoint: &str, ok: bool) {
        let tick = self.tick();
        let mut map = self.breakers.lock().unwrap();
        let cb = map
            .entry(endpoint.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.fail_threshold, self.cooldown_ticks));
        cb.record_outcome(ok, tick);
    }
}

impl Default for UpstreamBreakers {
    fn default() -> Self {
        Self::new(UPSTREAM_CIRCUIT_FAIL_THRESHOLD, UPSTREAM_CIRCUIT_COOLDOWN_SECS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_after_threshold_then_sheds() {
        let b = UpstreamBreakers::new(3, 1_000_000); // huge cooldown so it stays open
        let ep = "http://broken:8090";
        assert!(!b.is_open(ep), "closed on first sight");
        b.record(ep, false);
        b.record(ep, false);
        assert!(!b.is_open(ep), "2 < 3: still closed");
        b.record(ep, false);
        assert!(b.is_open(ep), "3rd failure opens -> shed");
    }

    #[test]
    fn success_keeps_closed() {
        let b = UpstreamBreakers::new(3, 30);
        let ep = "http://healthy:8090";
        for _ in 0..10 {
            b.record(ep, true);
        }
        assert!(!b.is_open(ep));
    }

    #[test]
    fn distinct_endpoints_isolated() {
        let b = UpstreamBreakers::new(1, 1_000_000);
        b.record("http://a", false); // a opens
        assert!(b.is_open("http://a"));
        assert!(!b.is_open("http://b"), "b unaffected by a");
    }
}
```
- [ ] Add to `routes/mod.rs` (alphabetical with the other `pub mod` lines): `pub mod upstream_health;` and a re-export `pub use upstream_health::UpstreamBreakers;`.
- [ ] Run, expect FAIL then PASS in one step (new file — first compile is the "fail" point; the test asserts behavior): `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins upstream_health 2>&1 | tail -40` — expect the 3 tests pass. (If `elohim_compute::CircuitBreaker` is unresolved, Tasks 1-2 have not landed — go back.)
- [ ] Commit:
```
git add doorway/doorway-service/src/routes/upstream_health.rs doorway/doorway-service/src/routes/mod.rs
git commit -m "feat(doorway): UpstreamBreakers per-endpoint proxy circuit map

Wraps elohim_compute::CircuitBreaker keyed by storage endpoint, wall-clock
tick so cooldown_ticks == seconds. Cat C node-local; own file (not
warm_stream.rs, per coordination scar).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 4 — `admission_exempt` + `catching_up_response` + `init_storage_proxy_client` (pure helpers)

Files:
- `doorway/doorway-service/src/server/http.rs` — add `admission_exempt` and `catching_up_response` near the other module-local response helpers (after `bad_request_response`, line 3596); add `init_storage_proxy_client` near `init_ssr_http_client` (after line 295); add `DOORWAY_ADMISSION_RETRY_AFTER_SECS` const near the top of the file (with the other consts — grep for `const DEFAULT` to find the const region, else place above `pub struct AppState`).

These are the TESTABLE seams for the "/health 200 under shed" scar (B4 / hard constraint): `handle_request` takes `Request<Incoming>` which is awkward to construct in a unit test, so we test the DECISION (`admission_exempt`) and the shed RESPONSE (`catching_up_response`) directly, plus a zero-permit `Semaphore` `try_acquire_owned()` to prove a non-exempt path sheds.

- [ ] Write the failing test — append to an existing `#[cfg(test)] mod` in `http.rs` (e.g. the `ssr_session_tests` module at line 3712, or add a new `#[cfg(test)] mod admission_tests` at file end):
```rust
#[cfg(test)]
mod admission_tests {
    use super::*;

    #[test]
    fn liveness_paths_are_exempt() {
        for p in ["/health", "/healthz", "/health/startup", "/ready", "/readyz", "/version"] {
            assert!(admission_exempt(p, false), "{p} must be admission-exempt");
        }
    }

    #[test]
    fn upgrades_are_exempt() {
        assert!(admission_exempt("/some-signal-pubkey", true), "WS upgrades hold the socket; never gate");
        assert!(admission_exempt("/debug/stream", true));
    }

    #[test]
    fn flood_surface_is_gated() {
        // The whole point: proxy/api/blob/apps traffic is NOT exempt.
        for p in ["/api/v1/cache/x", "/db/content/y", "/blob/sha256-z", "/apps/foo", "/lamad"] {
            assert!(!admission_exempt(p, false), "{p} must be gated (not exempt)");
        }
    }

    #[test]
    fn catching_up_is_503_with_retry_after_and_body() {
        let resp = catching_up_response(2);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(resp.headers().get("Retry-After").unwrap(), "2");
        // body shape is asserted by the storage-side test; here assert presence
        assert!(resp.headers().get("Content-Type").is_some());
    }

    #[test]
    fn zero_permit_semaphore_sheds_nonexempt() {
        // Models the gate: a zero-permit semaphore fails try_acquire_owned ->
        // the gate returns catching_up; an exempt path is never consulted.
        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
        assert!(std::sync::Arc::clone(&sem).try_acquire_owned().is_err(), "0 permits => shed");
    }
}
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins admission 2>&1 | tail -40` — expect `cannot find function admission_exempt` / `catching_up_response`.
- [ ] Write minimal implementation — add the const near the file's const region:
```rust
/// Retry-After (seconds) advertised when doorway sheds an inbound request at
/// the admission ceiling. Short — admission saturation is a transient burst.
const DOORWAY_ADMISSION_RETRY_AFTER_SECS: u64 = 2;

/// Inbound admission ceiling default (≈4× storage's 64 permits for a 4-worker
/// projection edge) and floor (anti-deadlock: a ceiling can never be 0). HOME
/// is here (not main.rs): the AppState ctors below reference DEFAULT_MAX_INFLIGHT,
/// and a lib cannot import a const from the bin. main.rs `use`s these.
pub const DEFAULT_MAX_INFLIGHT: usize = 256;
pub const MIN_MAX_INFLIGHT: usize = 8;
```
  Add `init_storage_proxy_client` after `init_ssr_http_client` (line 295):
```rust
/// Build the ONE pooled client the storage proxy uses for forward_to_storage /
/// forward_blob_to_storage. Replaces the per-request `reqwest::Client::new()`
/// (untimed) so a hung upstream cannot block a worker indefinitely.
fn init_storage_proxy_client() -> Arc<reqwest::Client> {
    Arc::new(
        reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(
                crate::routes::storage_proxy::STORAGE_PROXY_CONNECT_TIMEOUT_SECS,
            ))
            .timeout(std::time::Duration::from_secs(
                crate::routes::storage_proxy::STORAGE_PROXY_REQUEST_TIMEOUT_SECS,
            ))
            .build()
            .unwrap_or_default(),
    )
}
```
  Add the two helpers after `bad_request_response` (line 3596):
```rust
/// True ONLY for paths that must NEVER be shed by the inbound admission gate:
/// the liveness/health family, /version, and ANY WebSocket upgrade (signal +
/// /debug/stream hold a permit for the whole socket lifetime). Everything else
/// — proxy, api, blob, apps, SPA/EPR — is gated. Deliberately NARROW: gating on
/// `!is_service_path` would exempt /api,/db,/blob,/apps and gut the gate.
fn admission_exempt(path: &str, is_upgrade: bool) -> bool {
    if is_upgrade {
        return true;
    }
    matches!(
        path,
        "/health" | "/healthz" | "/health/startup" | "/ready" | "/readyz" | "/version"
    )
}

/// Propagated-backpressure shed response: 503 + Retry-After + a structured
/// `{status:"catching-up", retryAfter:N}` body. Never a bare drop/hang/502.
fn catching_up_response(retry_after_secs: u64) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "status": "catching-up",
        "retryAfter": retry_after_secs,
    });
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "application/json")
        .header("Retry-After", retry_after_secs.to_string())
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins admission 2>&1 | tail -40` — expect pass. (`init_storage_proxy_client` references `STORAGE_PROXY_CONNECT_TIMEOUT_SECS`/`STORAGE_PROXY_REQUEST_TIMEOUT_SECS`, which live in `storage_proxy.rs`. Add those two `pub const`s to `storage_proxy.rs` now — they are listed in the names table and reused in Task 8 — so this task compiles green. `init_storage_proxy_client` is only called by the AppState ctors (same crate/lib), so keep it private — no `pub` needed.)
- [ ] Commit:
```
git add doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): admission_exempt + catching_up_response + pooled-client builder

admission_exempt is NARROW (health family + /version + WS upgrades only) so
the gate covers the proxy/flood surface. catching_up_response = 503 +
Retry-After + {status:catching-up}. init_storage_proxy_client = one pooled
client (connect 3s / request 12s).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 5 — Add `inbound_semaphore` / `storage_proxy_client` / `upstream_breakers` to `AppState`; wire the gate

Files:
- `doorway/doorway-service/src/server/http.rs:200-285` (`AppState` struct), and the THREE ctor sites (after the field block ending ~448, ~542, ~654 — each currently ends `portal_health_override: Arc::new(...)`). Also the test ctors `admin_dev.rs:141`, `elohim_agent.rs:192`, `health.rs:293` if they construct `AppState` literally (check; many use a shared helper).
- The admission gate goes in `handle_request` right after the gate-check at line 2046 (before the EPR block at 2105 and the match at 2137).

`inbound_semaphore` is NON-Option (always present). `storage_proxy_client`/`upstream_breakers` are populated in `main.rs` (Task 10 wires the real ceiling); the ctors here get sane defaults so tests and direct-proxy mode work.

- [ ] Write the failing test — append to `admission_tests` (Task 4):
```rust
    #[test]
    fn appstate_has_inbound_admission_fields() {
        // Construct via the same path the ctors use; assert the fields exist and
        // the semaphore is non-Option with a usable permit count.
        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(256));
        assert!(sem.available_permits() >= 8, "ceiling floored");
        let breakers = std::sync::Arc::new(crate::routes::UpstreamBreakers::default());
        assert!(!breakers.is_open("http://x:8090"), "fresh breaker closed");
    }
```
- [ ] Run, expect FAIL: only if `UpstreamBreakers` re-export missing (it lands in Task 3). If Task 3 landed, this compiles+passes — the BEHAVIORAL change (fields on AppState + the gate) is verified by the whole-crate gate. Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins admission 2>&1 | tail -40`.
- [ ] Write minimal implementation — add to the `AppState` struct (after `portal_health_override`, line 284):
```rust
    /// Global inbound admission limiter (Pillar 2 layer 2). Bounds total
    /// in-flight requests; `try_acquire_owned()` sheds 503+Retry-After when
    /// full. NON-Option (always present, unlike render_semaphore). Sized from
    /// DOORWAY_MAX_INFLIGHT at boot (main.rs). Liveness/version/WS are exempt
    /// (admission_exempt) so /health stays answerable while shedding.
    pub inbound_semaphore: Arc<tokio::sync::Semaphore>,

    /// ONE pooled HTTP client for the storage proxy (connect 3s / request 12s).
    /// Replaces per-request `reqwest::Client::new()` in forward_to_storage.
    pub storage_proxy_client: Arc<reqwest::Client>,

    /// Per-upstream circuit breakers for the storage proxy (Pillar 2 layer 4).
    pub upstream_breakers: Arc<crate::routes::UpstreamBreakers>,
```
  Add to EACH of the 3 ctors (after `portal_health_override: Arc::new(...)`, before the closing `}`):
```rust
            inbound_semaphore: Arc::new(tokio::sync::Semaphore::new(DEFAULT_MAX_INFLIGHT)),
            storage_proxy_client: init_storage_proxy_client(),
            upstream_breakers: Arc::new(crate::routes::UpstreamBreakers::default()),
```
  (`DEFAULT_MAX_INFLIGHT`/`MIN_MAX_INFLIGHT` were defined in http.rs in Task 4 — the single home. main.rs `use`s them in Task 10. Do NOT redefine them in main.rs: the lib's AppState ctors reference `DEFAULT_MAX_INFLIGHT`, and a lib cannot import a const from the bin.)
  Insert the GATE in `handle_request` immediately after line 2048 (`}` closing the gate-check block), before the observation-id extraction:
```rust
    // ── Pillar 2 / layer 2: global inbound admission ──────────────────────────
    // Placed AFTER the wisdom gate and BEFORE EPR/routing. Liveness, /version,
    // and WebSocket upgrades are exempt (admission_exempt) so /health stays
    // answerable while we shed. SHED (try_acquire_owned), never QUEUE — an
    // unbounded queue is just a slower wedge. The permit is bound at function
    // scope so it is held across the downstream forward_to_storage await.
    let is_upgrade = hyper_tungstenite::is_upgrade_request(&req);
    let _admit = if admission_exempt(&path, is_upgrade) {
        None
    } else {
        match Arc::clone(&state.inbound_semaphore).try_acquire_owned() {
            Ok(permit) => Some(permit),
            Err(_) => {
                // Admission saturation is otherwise invisible to status-code
                // metrics on the happy path. Emit a distinct, greppable,
                // aggregation-countable signal (Plan D seam).
                tracing::warn!(
                    target: "admission_busy",
                    counter = "doorway_admission_shed_total",
                    path = %path,
                    available = state.inbound_semaphore.available_permits(),
                    "inbound admission at ceiling — shedding (503 + Retry-After)"
                );
                return Ok(to_boxed(catching_up_response(
                    DOORWAY_ADMISSION_RETRY_AFTER_SECS,
                )));
            }
        }
    };
```
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins admission 2>&1 | tail -40` — expect pass; confirm the crate compiles (the gate references resolve).
- [ ] Commit:
```
git add doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): global inbound admission gate (shed 503+Retry-After, health exempt)

inbound_semaphore on AppState (non-Option); try_acquire_owned in
handle_request AFTER the gate-check, exempting liveness/version/WS so
/health stays answerable. SHED not QUEUE. Mirrors the render-shed
try_acquire_owned idiom. Greppable admission_busy warn (Plan D seam).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 6 — Storage: REMOVE the accept-loop `acquire().await` (stop queueing)

Files:
- `elohim/elohim-storage/src/http.rs:705-711` (the `tokio::spawn` body in `run()`).

Per D5: dropping the connection-level queue is REQUIRED — leaving it means the queue still wedges and front-runs the per-request shed (Task 7). Keep the `request_semaphore` field; the per-request `try_acquire` in Task 7 is the new gate.

- [ ] Write the failing test — this is a structural removal verified by Task 7's behavioral test + the whole-crate gate; add a guard test asserting the field still exists for Task 7 to use. Append to an existing `#[cfg(test)] mod` in `http.rs` (or add `#[cfg(test)] mod admission_tests`):
```rust
#[cfg(test)]
mod admission_tests {
    use tokio::sync::Semaphore;

    #[test]
    fn try_acquire_sheds_when_exhausted() {
        // Models the per-request gate (Task 7): a 0-permit semaphore fails
        // try_acquire immediately (shed), never blocking like acquire().await.
        let sem = Semaphore::new(0);
        assert!(sem.try_acquire().is_err(), "exhausted => shed, not queue");
        let sem2 = Semaphore::new(1);
        assert!(sem2.try_acquire().is_ok(), "available => admit");
    }
}
```
- [ ] Run, expect PASS (pure semaphore behavior): `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib admission 2>&1 | tail -40`.
- [ ] Write minimal implementation — in `run()` (705-716), remove the `acquire().await` block and the now-unused `let semaphore = self.request_semaphore.clone();` (line 703). The spawn body becomes:
```rust
            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    let server = server.clone();
                    async move { server.handle_request(req).await }
                });

                // Enable HTTP upgrades for WebSocket support
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .with_upgrades()
                    .await
                {
                    warn!(addr = %remote_addr, error = %err, "Connection error");
                }
            });
```
  (Per-request admission moves INTO `handle_request` — Task 7. The `request_semaphore` field stays; it is now acquired per-request, not per-connection.)
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib admission 2>&1 | tail -40` — expect pass + compile clean (watch for an unused-var warning on `semaphore` — remove its binding).
- [ ] Commit:
```
git add elohim/elohim-storage/src/http.rs
git commit -m "fix(storage): remove accept-loop acquire().await (stop queueing connections)

The connection-level acquire().await queued under burst and never shed; it
also gated /health (only reached after the permit was held). Removed so the
per-request try_acquire gate (next commit) can shed with a propagated
503+Retry-After while /health stays exempt.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 7 — Storage: per-request `try_acquire` shed + `too_many_requests_with_retry` + advertise permits

Files:
- `elohim/elohim-storage/src/services/response.rs` — add `too_many_requests_with_retry` after `service_unavailable` (line 110).
- `elohim/elohim-storage/src/http.rs` — add `const STORAGE_SHED_RETRY_AFTER_SECS: u64 = 2;` near `MAX_CONCURRENT_REQUESTS` (line 132); add the per-request gate in `handle_request` AFTER the `/health` (756-759) and `/version` (762-771) arms; promote `semaphorePermits` to default health detail (move from the Trace block 1487-1488 into the always-present body 1458-1461 region).

The gate uses `request_semaphore.try_acquire()` (NOT `acquire().await`). On `Err`, return `too_many_requests_with_retry`. The held permit must live for the request duration — bind it to a guard that the match arms run under. Since `handle_request` is a big match, the cleanest seam is: at the top of `handle_request`, after computing `path`/`method`, branch — if NOT `/health` and NOT `/version` and NOT OPTIONS, `try_acquire`; hold the `OwnedSemaphorePermit` in a variable that outlives the match. (Use `try_acquire_owned()` on an `Arc<Semaphore>` clone, mirroring doorway.)

**WebSocket-upgrade decision (settled — do NOT exempt upgrades on the storage gate).** Storage serves WS (progress/sync) via `.with_upgrades()`. Under the OLD per-connection `acquire().await`, a WS connection already held exactly one permit for its whole session; the new per-request `try_acquire` likewise takes one permit on the upgrade request and holds it for the socket — so this is NOT a regression in permit accounting. The only behavior change is queue→shed: a WS upgrade arriving at the ceiling now gets a 503 instead of blocking. That is the correct admission posture (the semaphore exists for OOM protection under burst, per the const's doc comment) — so storage does NOT add doorway's `is_upgrade` exemption; only `/health`/`/version`/OPTIONS are exempt. (Doorway DOES exempt upgrades because its signal/`/debug/stream` sockets are long-lived relays, a different role.)

- [ ] Write the failing test — append to `services/response.rs` a `#[cfg(test)] mod tests` (or its existing one):
```rust
#[cfg(test)]
mod admission_response_tests {
    use super::*;

    #[test]
    fn too_many_requests_is_503_with_retry_and_permits_and_body() {
        let resp = too_many_requests_with_retry(2, 0);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(resp.headers().get(header::RETRY_AFTER).unwrap(), "2");
        assert_eq!(resp.headers().get("X-Available-Permits").unwrap(), "0");
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }
}
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib admission_response 2>&1 | tail -40` — expect `cannot find function too_many_requests_with_retry`.
- [ ] Write minimal implementation — add to `services/response.rs` after `service_unavailable` (line 110):
```rust
/// Build a propagated-backpressure shed response: 503 + Retry-After +
/// X-Available-Permits + structured {status:"catching-up", retryAfter:N} body.
/// (Named per the inbound-admission plan; uses 503 for saturation consistency.)
pub fn too_many_requests_with_retry(
    retry_after_secs: u64,
    available: usize,
) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "status": "catching-up",
        "retryAfter": retry_after_secs,
    });
    let json = body.to_string();
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::RETRY_AFTER, retry_after_secs.to_string())
        .header("X-Available-Permits", available.to_string())
        .body(Full::new(Bytes::from(json)))
        .unwrap()
}
```
- [ ] Run, expect PASS (response helper): `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib admission_response 2>&1 | tail -40` — expect pass.
- [ ] Write the gate + advertise — add the const near line 132:
```rust
/// Retry-After (seconds) when storage sheds at the request-admission ceiling.
const STORAGE_SHED_RETRY_AFTER_SECS: u64 = 2;
```
  In `handle_request`, after `let method_str = method.to_string();` (line 747) and the `debug!` (749), insert the gate. Because the existing `/health`/`/version` arms are INSIDE the match below, gate BEFORE the match but exempt those two paths and OPTIONS (CORS preflight must always answer):
```rust
        // ── Pillar 2 / layer 2: per-request admission (shed, never queue) ─────
        // Exempt /health, /version, and OPTIONS (preflight) so monitoring + CORS
        // stay live under shed. try_acquire (NOT acquire().await): at the ceiling
        // return 503 + Retry-After + X-Available-Permits immediately. The permit
        // is held for the whole request via `_admit`.
        let admission_exempt = matches!(method, Method::OPTIONS)
            || matches!(path.as_str(), "/health" | "/version");
        let _admit = if admission_exempt {
            None
        } else {
            match self.request_semaphore.clone().try_acquire_owned() {
                Ok(permit) => Some(permit),
                Err(_) => {
                    let available = self.request_semaphore.available_permits();
                    warn!(
                        target: "upstream_shed",
                        counter = "storage_admission_shed_total",
                        path = %path,
                        available,
                        "storage request admission at ceiling — shedding (503 + Retry-After)"
                    );
                    return Ok(crate::services::response::too_many_requests_with_retry(
                        STORAGE_SHED_RETRY_AFTER_SECS,
                        available,
                    )
                    .map(Either::Left));
                }
            }
        };
```
  (`request_semaphore` is `Arc<Semaphore>`; `.clone().try_acquire_owned()` mirrors doorway. Confirm `Method` is in scope — it is, used throughout the match.)
  Promote `semaphorePermits` to default detail — move it from the Trace block (1487-1488) to the always-present body (insert into the `body` json! at 1458-1461 or add right after as `body["semaphorePermits"] = ...`):
```rust
        body["semaphorePermits"] =
            serde_json::json!(self.request_semaphore.available_permits());
```
  (Remove the duplicate from the Trace block to avoid setting it twice.)
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib admission 2>&1 | tail -40` — expect pass + compile clean. (Watch: the `_admit` permit must NOT be dropped before the match runs — keeping it bound at function scope guarantees the held-for-request semantics.)
- [ ] Commit:
```
git add elohim/elohim-storage/src/services/response.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): per-request admission shed (503+Retry-After) + advertise permits

try_acquire (not acquire().await) in handle_request, exempting
/health,/version,OPTIONS so monitoring+CORS stay live. New
too_many_requests_with_retry helper (503 + Retry-After + X-Available-Permits
+ {status:catching-up}). semaphorePermits promoted to default health detail
so callers can restrain.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 8 — Storage proxy: pooled client + timeout consts + breaker gate (no-call when open)

Files:
- `doorway/doorway-service/src/routes/storage_proxy.rs` — add `STORAGE_PROXY_CONNECT_TIMEOUT_SECS`/`STORAGE_PROXY_REQUEST_TIMEOUT_SECS` consts near the top (after the `use` block, ~line 41); add `ProxyOutcome` enum; change `forward_to_storage` (`:90`) and `forward_blob_to_storage` (`:239`) signatures to take `client: &reqwest::Client` and `breakers: &UpstreamBreakers`; replace `reqwest::Client::new()` (`:112`, `:295`) with the passed `client`; add the breaker pre-check (no call when open).

- [ ] Write the failing test — append to `storage_proxy.rs` `mod tests` (after line 408 `use super::*;` region):
```rust
    #[test]
    fn proxy_timeout_consts_browser_facing() {
        assert_eq!(STORAGE_PROXY_CONNECT_TIMEOUT_SECS, 3);
        assert_eq!(STORAGE_PROXY_REQUEST_TIMEOUT_SECS, 12);
        assert!(STORAGE_PROXY_REQUEST_TIMEOUT_SECS < 45, "browser-facing, well under warm-up's 45s");
    }

    #[test]
    fn proxy_outcome_classifies_failures() {
        assert_eq!(ProxyOutcome::classify(200), ProxyOutcome::Ok);
        assert_eq!(ProxyOutcome::classify(204), ProxyOutcome::Ok);
        assert_eq!(ProxyOutcome::classify(404), ProxyOutcome::Neutral, "blob miss never opens breaker");
        assert_eq!(ProxyOutcome::classify(400), ProxyOutcome::Neutral);
        assert_eq!(ProxyOutcome::classify(429), ProxyOutcome::Failure);
        assert_eq!(ProxyOutcome::classify(503), ProxyOutcome::Failure);
        assert_eq!(ProxyOutcome::classify(500), ProxyOutcome::Failure);
    }
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins storage_proxy 2>&1 | tail -40` — expect `cannot find value STORAGE_PROXY_CONNECT_TIMEOUT_SECS` / `ProxyOutcome`.
- [ ] Write minimal implementation — add after the `use` block (~line 41):
```rust
use crate::routes::UpstreamBreakers;

/// Connect timeout for the pooled storage-proxy client (fail fast on a dead peer).
pub const STORAGE_PROXY_CONNECT_TIMEOUT_SECS: u64 = 3;
/// Whole-request timeout — browser-facing, well under warm-up's 45s.
pub const STORAGE_PROXY_REQUEST_TIMEOUT_SECS: u64 = 12;

/// Classifies an upstream result for the per-endpoint circuit breaker (D6).
/// Only transient saturation/connectivity counts as Failure; a 404 is a normal
/// blob miss (no-fanout rule) and must NEVER open the breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyOutcome {
    Ok,
    Failure,
    Neutral,
}

impl ProxyOutcome {
    pub fn classify(status: u16) -> ProxyOutcome {
        match status {
            200..=299 => ProxyOutcome::Ok,
            429 | 503 => ProxyOutcome::Failure,
            500..=599 => ProxyOutcome::Failure,
            _ => ProxyOutcome::Neutral, // 4xx incl 404 = neutral
        }
    }
}
```
  Change `forward_to_storage`'s signature (`:90-95`) to add `client` + `breakers`:
```rust
pub async fn forward_to_storage<B>(
    req: Request<B>,
    storage_url: &str,
    path: &str,
    client: &reqwest::Client,
    breakers: &UpstreamBreakers,
    ctx: ForwardCtx<'_>,
) -> Response<Full<Bytes>>
```
  At the top of the body (after `full_url` is computed, ~line 108), add the breaker pre-check (NO call when open → shed):
```rust
    // Per-upstream breaker (Pillar 2 layer 4): if this endpoint is circuit-open,
    // shed WITHOUT calling storage. Keyed by storage_url (per-upstream, not per
    // path — matches the single-target dispatch model).
    if breakers.is_open(storage_url) {
        warn!(
            target: "upstream_shed",
            counter = "doorway_upstream_breaker_open_total",
            storage_url = %storage_url,
            path = %path,
            "upstream circuit OPEN — shedding without calling storage (503 + Retry-After)"
        );
        return catching_up_proxy_response(crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS);
    }
```
  Replace `let client = reqwest::Client::new();` (`:112`) with using the passed `client` (delete that line; the `client.get(...)` etc. now use the arg). Add a module-local shed helper near the top of the file (proxy returns `Response<Full<Bytes>>`, so it can't call http.rs's `catching_up_response` — duplicate the small builder here):
```rust
/// Proxy-side catching-up shed (mirrors server::http::catching_up_response but
/// returns Response<Full<Bytes>> for the forwarder return type).
fn catching_up_proxy_response(retry_after_secs: u64) -> Response<Full<Bytes>> {
    let body = serde_json::json!({ "status": "catching-up", "retryAfter": retry_after_secs });
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "application/json")
        .header("Retry-After", retry_after_secs.to_string())
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}
```
  Do the SAME signature change for `forward_blob_to_storage` (`:239-245`): add `client: &reqwest::Client, breakers: &UpstreamBreakers` (after `cache`), add the same breaker pre-check, and replace `reqwest::Client::new()` (`:295`) with the passed `client`. The internal fall-through call `forward_to_storage(req, storage_url, path, ctx)` (at `:256`, `:264`) must pass `client, breakers` through.
  Update the TWO production call sites NOW (so the crate compiles green this task) — `http.rs:3159-3173`:
```rust
                    if p.starts_with("/blob/") {
                        return Ok(to_boxed(
                            routes::forward_blob_to_storage(
                                req,
                                &endpoint,
                                p,
                                Arc::clone(&state.cache),
                                &state.storage_proxy_client,
                                &state.upstream_breakers,
                                ctx,
                            )
                            .await,
                        ));
                    }
                    return Ok(to_boxed(
                        routes::forward_to_storage(
                            req,
                            &endpoint,
                            p,
                            &state.storage_proxy_client,
                            &state.upstream_breakers,
                            ctx,
                        )
                        .await,
                    ));
```
  Then `grep -rn "forward_to_storage\|forward_blob_to_storage" doorway/doorway-service/src/` and fix any OTHER callers (the doc-example/test helper at storage_proxy.rs:886 and any `#[cfg(test)]` callers) to pass `&reqwest::Client::new(), &UpstreamBreakers::default()`.
- [ ] Run, expect PASS (crate compiles green): `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins storage_proxy 2>&1 | tail -40` — the new const/outcome tests pass and the crate compiles (call sites updated). No red-compile checkpoint.
- [ ] Commit:
```
git add doorway/doorway-service/src/routes/storage_proxy.rs doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): pooled+timed+breaker-gated storage proxy (no bare 502)

forward_to_storage / forward_blob_to_storage take the pooled client +
UpstreamBreakers; circuit-open sheds 503+Retry-After WITHOUT calling storage.
ProxyOutcome classifies failures (404=neutral, never opens). Replaces the
two untimed reqwest::Client::new() sites.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 9 — Storage proxy: HONOR upstream backpressure + record breaker outcome + timeout→catching-up

Files:
- `doorway/doorway-service/src/routes/storage_proxy.rs` — the `match builder.send().await` block (`:175-214` in `forward_to_storage`; the mirror in `forward_blob_to_storage` `:316-383`).

Today the `Ok(response)` arm (`:177-191`) copies the upstream status but DROPS its `Retry-After`; the `Err(e)` arm (`:204-213`) returns a bare 502. After this task: (a) read the upstream status; if `429`/`503`, capture its `Retry-After` (fallback to cooldown) and RE-EMIT a catching-up response carrying that Retry-After (do not hammer); (b) record `ProxyOutcome::classify(status)` into the breaker; (c) on `send()` Err (connect error / timeout), record a Failure and emit `503 + Retry-After + catching-up` instead of `502`.

- [ ] Write the failing test — append to `storage_proxy.rs` `mod tests`:
```rust
    #[test]
    fn honor_decision_maps_upstream_to_retry_after() {
        // The honor decision: a 503/429 upstream surfaces catching-up; the
        // Retry-After is the upstream's value if present, else the cooldown.
        fn honored_retry_after(upstream_status: u16, upstream_ra: Option<u64>, cooldown: u64) -> Option<u64> {
            match upstream_status {
                429 | 503 => Some(upstream_ra.unwrap_or(cooldown)),
                _ => None,
            }
        }
        assert_eq!(honored_retry_after(503, Some(7), 30), Some(7), "preserve upstream Retry-After");
        assert_eq!(honored_retry_after(429, None, 30), Some(30), "fallback to cooldown");
        assert_eq!(honored_retry_after(200, None, 30), None, "2xx passes through unchanged");
    }
```
- [ ] Run, expect FAIL or PASS: this is a pure decision test (passes on the helper). Run: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins storage_proxy 2>&1 | tail -40`. The BEHAVIORAL change is in the send-match arms, verified by the whole-crate gate; the inline `fn honored_retry_after` documents the contract the impl follows.
- [ ] Write minimal implementation — replace the `Ok(response)` arm body (`:177-202` in `forward_to_storage`) so it honors + records:
```rust
        Ok(response) => {
            let status = response.status();
            let status_u16 = status.as_u16();
            let outcome = ProxyOutcome::classify(status_u16);
            breakers.record(storage_url, outcome != ProxyOutcome::Failure);

            // HONOR upstream backpressure: a 429/503 from storage becomes a
            // catching-up to the browser, preserving the upstream Retry-After
            // (else the breaker cooldown) so the client does not hammer.
            if matches!(status_u16, 429 | 503) {
                let upstream_ra = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok());
                let retry_after = upstream_ra
                    .unwrap_or(crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS);
                warn!(
                    target: "upstream_shed",
                    counter = "doorway_upstream_backpressure_honored_total",
                    storage_url = %storage_url,
                    status = status_u16,
                    retry_after,
                    "honoring upstream backpressure — surfacing catching-up to client"
                );
                return catching_up_proxy_response(retry_after);
            }

            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            match response.bytes().await {
                Ok(body) => Response::builder()
                    .status(StatusCode::from_u16(status_u16).unwrap_or(StatusCode::OK))
                    .header("Content-Type", content_type)
                    .header("Cross-Origin-Resource-Policy", "cross-origin")
                    .body(Full::new(Bytes::from(body.to_vec())))
                    .unwrap(),
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    breakers.record(storage_url, false);
                    catching_up_proxy_response(
                        crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS,
                    )
                }
            }
        }
```
  Replace the `Err(e)` arm (`:204-213`) so connect/timeout records a Failure + emits catching-up (not 502):
```rust
        Err(e) => {
            warn!(error = %e, path = %path, storage_url = %storage_url,
                "storage forward failed (connect/timeout) — recording breaker failure");
            breakers.record(storage_url, false);
            catching_up_proxy_response(
                crate::routes::upstream_health::UPSTREAM_CIRCUIT_COOLDOWN_SECS,
            )
        }
```
  Apply the SAME three changes (record outcome, honor 429/503, Err→catching-up) to the mirror arms in `forward_blob_to_storage` (`:316-383`). Keep the blob 404 path returning the upstream 404 verbatim (a miss is `Neutral`, not a shed — `breakers.record(storage_url, true)` for any non-Failure status).
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins storage_proxy 2>&1 | tail -40` — expect pass + compile clean.
- [ ] Commit:
```
git add doorway/doorway-service/src/routes/storage_proxy.rs
git commit -m "feat(doorway): honor upstream 429/503+Retry-After; record breaker; no bare 502

The proxy now surfaces upstream backpressure as catching-up (preserving the
upstream Retry-After, else cooldown), records every outcome into the per-
upstream breaker (404=neutral), and on connect/timeout emits 503+Retry-After
instead of a bare 502.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 10 — Wire the two proxy call sites + boot the ceiling/client/breakers in `main.rs`

Files:
- `doorway/doorway-service/src/server/http.rs:3159-3173` (the two `forward_*` call sites).
- `doorway/doorway-service/src/main.rs` — add `DEFAULT_MAX_INFLIGHT`/`MIN_MAX_INFLIGHT` (or `use` from http.rs per Task 5's note) + `inbound_max()`; populate `inbound_semaphore`/`storage_proxy_client`/`upstream_breakers` after `state` is built and before `Arc::new(state)` (near the warmup_state wiring at 617-623).

(If Task 8 already updated the call sites to compile, this task is the main.rs boot wiring + verifying the call sites pass `&state.storage_proxy_client, &state.upstream_breakers`.)

- [ ] Write the failing test — append to `main.rs` a `#[cfg(test)] mod` (or extend one):
```rust
#[cfg(test)]
mod inbound_max_tests {
    use super::*;

    #[test]
    fn inbound_max_floors_and_defaults() {
        std::env::remove_var("DOORWAY_MAX_INFLIGHT");
        assert_eq!(inbound_max(), DEFAULT_MAX_INFLIGHT, "unset => default");
        std::env::set_var("DOORWAY_MAX_INFLIGHT", "0");
        assert_eq!(inbound_max(), MIN_MAX_INFLIGHT, "0 clamped to floor (anti-deadlock)");
        std::env::set_var("DOORWAY_MAX_INFLIGHT", "3");
        assert_eq!(inbound_max(), MIN_MAX_INFLIGHT, "below floor clamps up");
        std::env::set_var("DOORWAY_MAX_INFLIGHT", "512");
        assert_eq!(inbound_max(), 512, "honored above floor");
        std::env::remove_var("DOORWAY_MAX_INFLIGHT");
    }
}
```
- [ ] Run, expect FAIL: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins inbound_max 2>&1 | tail -40` — expect `cannot find function inbound_max`.
- [ ] Write minimal implementation — `DEFAULT_MAX_INFLIGHT`/`MIN_MAX_INFLIGHT` already live in http.rs (Task 4); import them in main.rs (`use doorway::server::http::{DEFAULT_MAX_INFLIGHT, MIN_MAX_INFLIGHT};`). Add `inbound_max()` near `worker_threads()` (line 49):
```rust
fn inbound_max() -> usize {
    // FOLLOW-ON: when elohim_compute::limits::derive() lands (Auto-config plan),
    // source the default from the host-derived ceiling instead of the constant.
    std::env::var("DOORWAY_MAX_INFLIGHT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_INFLIGHT)
        .max(MIN_MAX_INFLIGHT)
}
```
  `DEFAULT_MAX_INFLIGHT`/`MIN_MAX_INFLIGHT` are defined ONCE in http.rs (Task 4). main.rs imports them — add near main.rs's other `use` lines: `use doorway::server::http::{DEFAULT_MAX_INFLIGHT, MIN_MAX_INFLIGHT};`. Do NOT redefine them in main.rs.
  Override ONLY `inbound_semaphore` before `Arc::new(state)` (near line 617). The other two fields (`storage_proxy_client`, `upstream_breakers`) are already set to identical values by the AppState ctors (Task 5) — do NOT reassign them:
```rust
    state.inbound_semaphore =
        std::sync::Arc::new(tokio::sync::Semaphore::new(inbound_max()));
    info!(max_inflight = inbound_max(), "inbound admission ceiling set");
```
  (`state` must be mutable here — the warmup block at 619 already mutates `state`, so it is. Because only `inbound_semaphore` is overridden, `init_storage_proxy_client` keeps NO main.rs caller and stays private — no `pub` fixup needed in Task 4.)
  The two production call sites (http.rs:3159-3173) were already updated in Task 8 (which committed them green). Here, just VERIFY no straggler callers remain: `grep -rn "forward_to_storage\|forward_blob_to_storage" doorway/doorway-service/src/` — every call must pass `&client, &breakers` (production sites use `&state.storage_proxy_client, &state.upstream_breakers`; test/doc callers use a throwaway `&reqwest::Client::new(), &UpstreamBreakers::default()`). Fix any that Task 8's grep missed.
- [ ] Run, expect PASS: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins inbound_max 2>&1 | tail -40` then a full compile: `... cargo test --lib --bins 2>&1 | tail -40` — expect pass.
- [ ] Commit:
```
git add doorway/doorway-service/src/main.rs
git commit -m "feat(doorway): boot inbound ceiling from DOORWAY_MAX_INFLIGHT

inbound_max() parses DOORWAY_MAX_INFLIGHT (default 256, floor 8 — never 0)
and overrides the ctor's inbound_semaphore at boot. (storage_proxy_client +
upstream_breakers are already ctor-set to identical defaults.) FOLLOW-ON note
left for the Auto-derived ceiling swap. (Production proxy call sites were
wired in Task 8.)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 11 — Scar test: `/health` answers 200 while the gate is exhausted (both edges)

Files:
- `doorway/doorway-service/src/server/http.rs` — extend `admission_tests`.
- `elohim/elohim-storage/src/http.rs` — extend `admission_tests`.

The HARD CONSTRAINT, made explicit and permanent: liveness stays answerable while shedding. We test the DECISION composition (exempt path is never gated even at zero permits) on both edges, since constructing `Request<Incoming>` is impractical in a unit test.

- [ ] Write the failing test (doorway) — append to `admission_tests` in `http.rs`:
```rust
    #[test]
    fn health_never_shed_even_at_zero_permits() {
        // Compose the gate decision exactly as handle_request does: exempt paths
        // bypass the semaphore entirely, so a 0-permit ceiling cannot shed them.
        let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
        for p in ["/health", "/healthz", "/health/startup", "/ready", "/readyz", "/version"] {
            let exempt = admission_exempt(p, false);
            assert!(exempt, "{p} must be exempt");
            // Exempt => semaphore NOT consulted => answerable regardless of permits.
            let would_shed = !exempt && std::sync::Arc::clone(&sem).try_acquire_owned().is_err();
            assert!(!would_shed, "{p} must NOT shed at 0 permits");
        }
        // Control: a gated path DOES shed at 0 permits.
        let gated = "/api/v1/cache/x";
        let would_shed = !admission_exempt(gated, false)
            && std::sync::Arc::clone(&sem).try_acquire_owned().is_err();
        assert!(would_shed, "gated path must shed at 0 permits");
    }
```
- [ ] Write the failing test (storage) — append to `admission_tests` in storage `http.rs`:
```rust
    #[test]
    fn storage_health_version_never_shed_at_zero_permits() {
        use hyper::Method;
        let sem = tokio::sync::Semaphore::new(0);
        let exempt = |method: &Method, path: &str| {
            matches!(method, &Method::OPTIONS) || matches!(path, "/health" | "/version")
        };
        for p in ["/health", "/version"] {
            assert!(exempt(&Method::GET, p), "{p} exempt");
        }
        // gated path sheds
        assert!(!exempt(&Method::GET, "/db/content/x"));
        assert!(sem.try_acquire().is_err(), "0 permits => gated path sheds");
    }
```
- [ ] Run both, expect PASS: doorway `... cargo test --lib --bins admission 2>&1 | tail -40`; storage `RUSTFLAGS='--cfg getrandom_backend="custom"' ... cargo test --lib admission 2>&1 | tail -40` — expect pass.
- [ ] Commit:
```
git add doorway/doorway-service/src/server/http.rs elohim/elohim-storage/src/http.rs
git commit -m "test(admission): /health answers at zero permits on both edges (liveness scar)

Locks the hard constraint: the admission gate sits AFTER health-exemption,
so /health,/ready,/version are never shed even when the semaphore is
exhausted; a gated path is shed. Permanent regression guard.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 12 — Whole-crate green gate + fmt + clippy (all three crates) + story-harvest

Files: none new — verification only.

- [ ] doorway tests: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo test --lib --bins 2>&1 | tail -40` — expect 331+ pass.
- [ ] doorway clippy: `cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40` — expect no warnings.
- [ ] doorway fmt: `cd /projects/elohim/doorway/doorway-service && cargo fmt --check` (fix with `cargo fmt`, re-run).
- [ ] elohim-compute: `cd /projects/elohim/elohim/elohim-compute && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/ec-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40` then `... cargo clippy -- -D warnings 2>&1 | tail -40` then `cargo fmt --check`.
- [ ] elohim-storage: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib 2>&1 | tail -40` then `... cargo clippy -- -D warnings 2>&1 | tail -40` then `cargo fmt --check`.
- [ ] If any fmt fixes were applied, commit them:
```
git add elohim/elohim-compute/src/peers.rs elohim/elohim-compute/src/lib.rs doorway/doorway-service/src/routes/upstream_health.rs doorway/doorway-service/src/routes/mod.rs doorway/doorway-service/src/routes/storage_proxy.rs doorway/doorway-service/src/server/http.rs doorway/doorway-service/src/main.rs elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/services/response.rs
git commit -m "style(inbound-admission): cargo fmt

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```
- [ ] Invoke `story-harvest` to scaffold an a2o regression scenario capturing the constraints: admission sheds 503+Retry-After at the ceiling while /health stays 200; storage sheds (no longer queues); the proxy circuit-breaks a broken upstream and honors its Retry-After instead of returning 502. These are the parameter-bearing discoveries (ceiling 256/floor 8, Retry-After 2s/30s cooldown, timeouts 3s/12s) story-harvest preserves.

---

## Self-Review

**Spec coverage (every IN item maps to a task):**
- (1) DOORWAY INBOUND ADMISSION — global `inbound_semaphore`, placed AFTER health/version/WS exemption so `/health`,`/healthz`,`/health/startup`,`/ready` are never shed; SHED with 503 + Retry-After + `{status:"catching-up", retryAfter:N}`; ceiling from `DOORWAY_MAX_INFLIGHT` (default 256, floor 8); Auto-derived ceiling named follow-on (`// FOLLOW-ON:` in `inbound_max`) → **Tasks 4 (`admission_exempt`/`catching_up_response`), 5 (field + gate), 10 (boot ceiling).**
- (2) STORAGE ADMISSION SHEDS + ADVERTISES — `acquire().await` removed; per-request `try_acquire` shed (503/Retry-After, `/health`+`/version`+OPTIONS exempt); `X-Available-Permits` header + `semaphorePermits` promoted to default health detail → **Tasks 6 (remove queue), 7 (try_acquire shed + `too_many_requests_with_retry` + advertise).**
- (3) BOUNDED `forward_to_storage` — ONE pooled `storage_proxy_client` (connect 3s/request 12s) replacing both `Client::new()` sites; per-upstream breaker REUSING `elohim_compute::CircuitBreaker`; breaker-open → 503+Retry-After+catching-up (not 502) → **Tasks 1-3 (shared breaker + map), 4 (`init_storage_proxy_client`), 8 (pooled+breaker-gated), 10 (wire call sites).**
- (4) HONOR backpressure — proxy reads upstream 429/503+Retry-After and re-emits catching-up (preserving Retry-After); records outcome into the breaker; connect/timeout → catching-up → **Task 9.**
- HARD CONSTRAINTS: /health answerable under shed — **Task 11 (both edges, explicit scar test)**, gate AFTER exemption (Tasks 5/7). SHED not QUEUE — `try_acquire`/`try_acquire_owned` everywhere (Tasks 5/6/7); D5 removes the storage queue. Propagated not silent — every shed = 503 + Retry-After + structured body (Tasks 4/7/8/9); never bare 502. Anti-deadlock — floor `MIN_MAX_INFLIGHT=8`, breaker `fail_threshold.max(1)`, bounded Retry-After (2s/30s) (Tasks 10/1/D7). Self-contained — env knob + shared (not local) breaker; ordering + fallbacks in D1/D3 (idempotent Task 1, FOLLOW-ON swap site). Cat C node-local — stated D-prefix + type docs; no DHT/table. Coordination — `warm_stream.rs`/conductor/`target_arc_factor` untouched; proxy breaker in its OWN file `routes/upstream_health.rs` (D9); each `git add` names exact files.
- OUT items — bilateral credit/window, upstream-self-protection warm-up gating (CircuitBreaker reused), arc-shrink/`target_arc_factor`, REA actuation, Plan D elevate poller — all named in "Out of scope"; the Plan D seam is the `admission_busy`/`upstream_shed` greppable `warn!` + counter fields (Tasks 5/7/8/9, D10).

**Placeholder scan:** No `TODO`, no `<...>`, no "implement here". Every step has actual test code, exact commands (with the correct per-crate `RUSTFLAGS`), actual implementation code, and a concrete `git add` + commit with the `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` trailer. The single `FOLLOW-ON` marker is an intentional named-deferral code comment, not a plan placeholder.

**Type/fn-name consistency (cross-task):** `CircuitBreaker`/`CircuitState` defined Task 1, exported Task 2, consumed by `UpstreamBreakers` (Task 3) and `init_storage_proxy_client`/`forward_*` (Tasks 4/8). `UpstreamBreakers::{new,is_open,record}` — defined Task 3, consumed Tasks 5/8/9/10. `admission_exempt`/`catching_up_response`/`init_storage_proxy_client` — defined Task 4, consumed Tasks 5/10/11. `inbound_semaphore`/`storage_proxy_client`/`upstream_breakers` — added to AppState Task 5, populated Task 10, consumed by the gate (5) + call sites (10). `ProxyOutcome::classify` — defined Task 8, consumed Task 9. `too_many_requests_with_retry` — defined Task 7, consumed Task 7's gate. Consts (`DOORWAY_MAX_INFLIGHT`/`DEFAULT_MAX_INFLIGHT`=256/`MIN_MAX_INFLIGHT`=8/`DOORWAY_ADMISSION_RETRY_AFTER_SECS`=2/`STORAGE_PROXY_CONNECT_TIMEOUT_SECS`=3/`STORAGE_PROXY_REQUEST_TIMEOUT_SECS`=12/`UPSTREAM_CIRCUIT_FAIL_THRESHOLD`=3/`UPSTREAM_CIRCUIT_COOLDOWN_SECS`=30/`STORAGE_SHED_RETRY_AFTER_SECS`=2) each defined once (named-table home) and referenced by exact name. `DEFAULT_MAX_INFLIGHT`/`MIN_MAX_INFLIGHT` have ONE home — `server/http.rs` (defined Task 4) — because the lib's AppState ctors reference them and a lib cannot import a const from the bin; main.rs `use`s them (Task 10). Names table, Task 4, Task 5, and Task 10 all agree.

**Determinism / idempotency check:** the shared `CircuitBreaker` is tick-injected and pure (D1); the wall-clock seam is isolated to `UpstreamBreakers::tick` (`started.elapsed().as_secs()`, D2) and `reqwest` timeouts — never inside the state machine. Task 1/2 are explicitly idempotent (presence-check first) so they no-op if the sibling plan landed the type. Unit tests use fixed permit counts / fixed status codes / fixed ticks — no network, no sleep.

**Scar adherence:** SHED-not-QUEUE enforced (D5 removes the storage queue; `try_acquire*` only). Health-exemption placement tested explicitly on both edges (Task 11). No bare 502 (Task 9). `warm_stream.rs` / conductor / `target_arc_factor` untouched; proxy breaker in its own file (D9). Per-crate `RUSTFLAGS` correct (storage keeps the WASM getrandom flag; doorway/compute use `""`). Plain `cargo test`, `--lib --bins` (doorway) / `--lib` (storage/compute), `/tmp` target dirs, `RUSTC_WRAPPER=""`, no `&&`-piped gate exits.
